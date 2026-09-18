/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use std::fmt::Write;
use std::ops::Deref;
use std::sync::Arc;

use pyo3::exceptions::PyIndexError;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyString, PyTuple, PyType};

use sudachi::prelude::{
    Morpheme, MorphemeList, MorphemeRef as RustMorphemeRef, MorphemeView, SingleMorpheme,
};

use crate::dictionary::{extract_mode, PyDicData, PyDictionary};
use crate::errors;
use crate::projection::{MorphemeProjection, PyProjector};

pub(crate) type PyMorphemeList = MorphemeList<Arc<PyDicData>>;

enum PyMorphemeListBacking {
    List(PyMorphemeList),
    Singles(Vec<SingleMorpheme<Arc<PyDicData>>>),
}

/// A list of morphemes.
///
/// An object can not be instantiated manually.
/// Use Tokenizer.tokenize("") to create an empty morpheme list.
#[pyclass(module = "sudachipy.morphemelist", name = "MorphemeList")]
pub struct PyMorphemeListWrapper {
    inner: PyMorphemeListBacking,
    projection: PyProjector,
}

// PyMorphemeListWrapper is used only when GIL is active,
// all associated functions take GIL token as a parameter
unsafe impl Sync for PyMorphemeListWrapper {}
unsafe impl Send for PyMorphemeListWrapper {}

impl PyMorphemeListWrapper {
    pub(crate) fn new(dict: Arc<PyDicData>) -> Self {
        let proj = dict.projection.clone();
        Self {
            inner: PyMorphemeListBacking::List(PyMorphemeList::empty(dict)),
            projection: proj,
        }
    }

    pub(crate) fn from_components(list: PyMorphemeList, projection: PyProjector) -> Self {
        Self {
            inner: PyMorphemeListBacking::List(list),
            projection,
        }
    }

    pub(crate) fn from_singles(
        singles: Vec<SingleMorpheme<Arc<PyDicData>>>,
        projection: PyProjector,
    ) -> Self {
        Self {
            inner: PyMorphemeListBacking::Singles(singles),
            projection,
        }
    }

    pub(crate) fn projection(&self) -> Option<&dyn MorphemeProjection> {
        match &self.projection {
            None => None,
            Some(p) => Some(p.as_ref()),
        }
    }

    pub(crate) fn as_list(&self) -> PyResult<&PyMorphemeList> {
        match &self.inner {
            PyMorphemeListBacking::List(list) => Ok(list),
            PyMorphemeListBacking::Singles(_) => errors::wrap(Err::<&PyMorphemeList, _>(
                "expected analyzed MorphemeList, got standalone morphemes",
            )),
        }
    }

    pub(crate) fn replace_with_empty_list(
        &mut self,
        dict: Arc<PyDicData>,
        projection: PyProjector,
    ) -> PyResult<&mut PyMorphemeList> {
        self.projection = projection;
        self.inner = PyMorphemeListBacking::List(PyMorphemeList::empty(dict));

        match &mut self.inner {
            PyMorphemeListBacking::List(list) => Ok(list),
            PyMorphemeListBacking::Singles(_) => errors::wrap(Err::<&mut PyMorphemeList, _>(
                "failed to install analyzed MorphemeList backing",
            )),
        }
    }

    pub(crate) fn replace_with_singles(
        &mut self,
        singles: Vec<SingleMorpheme<Arc<PyDicData>>>,
        projection: PyProjector,
    ) {
        self.projection = projection;
        self.inner = PyMorphemeListBacking::Singles(singles);
    }

    fn len_internal(&self) -> usize {
        match &self.inner {
            PyMorphemeListBacking::List(list) => list.len(),
            PyMorphemeListBacking::Singles(items) => items.len(),
        }
    }

    fn is_empty_internal(&self) -> bool {
        self.len_internal() == 0
    }
}

#[pymethods]
impl PyMorphemeListWrapper {
    /// Returns an empty morpheme list with dictionary.
    ///
    /// .. deprecated:: 0.6.0
    ///     Use Tokenizer.tokenize("") if you need.
    #[classmethod]
    #[pyo3(text_signature = "(dict: Dictionary) -> MorphemeList")]
    fn empty(_cls: &Bound<PyType>, py: Python, dict: &PyDictionary) -> PyResult<Self> {
        errors::warn_deprecation(
            py,
            c_str!("Use Tokenizer.tokenize(\"\") if you need an empty MorphemeList."),
        )?;

        let cloned = dict.dictionary.as_ref().unwrap().clone();
        let proj = cloned.projection.clone();
        Ok(Self {
            inner: PyMorphemeListBacking::List(PyMorphemeList::empty(cloned)),
            projection: proj,
        })
    }

    /// Returns the total cost of the path.
    #[pyo3(text_signature = "(self, /) -> int")]
    fn get_internal_cost(&self, _py: Python) -> PyResult<i32> {
        match &self.inner {
            PyMorphemeListBacking::List(list) => Ok(list.get_internal_cost()),
            PyMorphemeListBacking::Singles(_) => errors::wrap(Err(
                "standalone morpheme lists do not have a lattice path cost",
            )),
        }
    }

    /// Returns the number of morpheme in this list.
    #[pyo3(text_signature = "(self, /) -> int")]
    fn size(&self, _py: Python) -> usize {
        self.len_internal()
    }

    fn __len__(&self, py: Python) -> usize {
        self.size(py)
    }

    fn __getitem__(slf: Bound<PyMorphemeListWrapper>, mut idx: isize) -> PyResult<PyMorpheme> {
        enum Item {
            List(usize),
            Single(Box<SingleMorpheme<Arc<PyDicData>>>, PyProjector),
        }

        let item = {
            let list = slf.borrow();
            let len = list.len_internal() as isize;

            if idx < 0 {
                // negative indexing
                idx += len;
            }

            if idx < 0 || len <= idx {
                return Err(PyIndexError::new_err(format!(
                    "MorphemeList index out of range: the len is {} but the index is {}",
                    len, idx
                )));
            }

            let idx = idx as usize;
            match &list.inner {
                PyMorphemeListBacking::List(_) => Item::List(idx),
                PyMorphemeListBacking::Singles(items) => {
                    Item::Single(Box::new(items[idx].clone()), list.projection.clone())
                }
            }
        };

        match item {
            Item::List(idx) => {
                let py_list: Py<PyMorphemeListWrapper> = slf.into();
                Ok(PyMorpheme::list_backed(py_list, idx))
            }
            Item::Single(morpheme, projection) => {
                Ok(PyMorpheme::single_backed_boxed(morpheme, projection))
            }
        }
    }

    fn __str__<'py>(&'py self, py: Python<'py>) -> Bound<'py, PyString> {
        let mut result = String::new();
        for i in 0..self.len_internal() {
            if i > 0 {
                result.push(' ');
            }
            match &self.inner {
                PyMorphemeListBacking::List(list) => {
                    result.push_str(list.get(i).surface().deref());
                }
                PyMorphemeListBacking::Singles(items) => {
                    result.push_str(items[i].surface());
                }
            }
        }
        PyString::new(py, result.as_str())
    }

    fn __repr__(slf: Py<PyMorphemeListWrapper>, py: Python) -> PyResult<Bound<PyString>> {
        let self_ref = slf.borrow(py);
        let mut result = String::new();
        result.push_str("<MorphemeList[\n");
        let nmorphs = self_ref.len_internal();
        for i in 0..nmorphs {
            result.push_str("  ");
            let pymorph = match &self_ref.inner {
                PyMorphemeListBacking::List(_) => PyMorpheme::list_backed(slf.clone_ref(py), i),
                PyMorphemeListBacking::Singles(items) => {
                    PyMorpheme::single_backed(items[i].clone(), self_ref.projection.clone())
                }
            };
            pymorph.write_repr(py, &mut result)?;
            result.push_str(",\n");
        }
        result.push_str("]>");
        Ok(PyString::new(py, result.as_str()))
    }

    fn __iter__(slf: Py<Self>) -> PyMorphemeIter {
        PyMorphemeIter {
            list: slf,
            index: 0,
        }
    }

    fn __bool__(&self, _py: Python) -> bool {
        !self.is_empty_internal()
    }
}

/// An iterator over the MorphemeList.
#[pyclass(module = "sudachipy.morphemelist", name = "MorphemeIter")]
pub struct PyMorphemeIter {
    list: Py<PyMorphemeListWrapper>,
    index: usize,
}

#[pymethods]
impl PyMorphemeIter {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self, py: Python) -> Option<PyMorpheme> {
        enum Item {
            List(usize),
            Single(Box<SingleMorpheme<Arc<PyDicData>>>, PyProjector),
        }

        let item = {
            let list = self.list.borrow(py);
            if self.index >= list.len_internal() {
                return None;
            }

            match &list.inner {
                PyMorphemeListBacking::List(_) => Item::List(self.index),
                PyMorphemeListBacking::Singles(items) => {
                    Item::Single(Box::new(items[self.index].clone()), list.projection.clone())
                }
            }
        };

        self.index += 1;
        match item {
            Item::List(index) => Some(PyMorpheme::list_backed(self.list.clone_ref(py), index)),
            Item::Single(morpheme, projection) => {
                Some(PyMorpheme::single_backed_boxed(morpheme, projection))
            }
        }
    }
}

/// It is a syntax sugar for accessing Morpheme reference
/// Without it binding implementations become much less readable
struct MorphemeBorrow<'py> {
    #[allow(unused)] // need to keep this around for correct reference count
    list: PyRef<'py, PyMorphemeListWrapper>,
    morph: Morpheme<'py, Arc<PyDicData>>,
}

impl<'py> Deref for MorphemeBorrow<'py> {
    type Target = Morpheme<'py, Arc<PyDicData>>;

    fn deref(&self) -> &Self::Target {
        &self.morph
    }
}

enum PyMorphemeBacking {
    List {
        list: Py<PyMorphemeListWrapper>,
        index: usize,
    },
    Single {
        morpheme: Box<SingleMorpheme<Arc<PyDicData>>>,
        projection: PyProjector,
    },
}

/// A morpheme (basic semantic unit of language).
#[pyclass(module = "sudachipy.morpheme", name = "Morpheme", frozen)]
pub struct PyMorpheme {
    backing: PyMorphemeBacking,
}

#[derive(Clone, Copy)]
enum FormMorphemeKind {
    Dictionary,
    Normalized,
}

impl PyMorpheme {
    fn list_backed(list: Py<PyMorphemeListWrapper>, index: usize) -> Self {
        Self {
            backing: PyMorphemeBacking::List { list, index },
        }
    }

    pub(crate) fn single_backed(
        morpheme: SingleMorpheme<Arc<PyDicData>>,
        projection: PyProjector,
    ) -> Self {
        Self::single_backed_boxed(Box::new(morpheme), projection)
    }

    pub(crate) fn single_backed_boxed(
        morpheme: Box<SingleMorpheme<Arc<PyDicData>>>,
        projection: PyProjector,
    ) -> Self {
        Self {
            backing: PyMorphemeBacking::Single {
                morpheme,
                projection,
            },
        }
    }

    fn borrow_morpheme<'py>(
        list: &'py Py<PyMorphemeListWrapper>,
        py: Python<'py>,
        index: usize,
    ) -> PyResult<MorphemeBorrow<'py>> {
        let list = list.borrow(py);
        // workaround for self-referential structs
        let morph = unsafe {
            std::mem::transmute::<Morpheme<'_, Arc<PyDicData>>, Morpheme<'_, Arc<PyDicData>>>(
                list.as_list()?.get(index),
            )
        };
        Ok(MorphemeBorrow { list, morph })
    }

    fn write_repr<'py, W: Write>(&'py self, py: Python<'py>, out: &mut W) -> PyResult<()> {
        // per https://github.com/WorksApplications/SudachiPy/pull/166#issuecomment-932043063
        match &self.backing {
            PyMorphemeBacking::List { list, index } => {
                let mrp = Self::borrow_morpheme(list, py, *index)?;
                let surf = mrp.surface();
                errors::wrap_ctx(
                    write!(
                        out,
                        "<Morpheme({}, {}:{}, {})>",
                        surf.deref(),
                        mrp.begin_c(),
                        mrp.end_c(),
                        mrp.word_id()
                    ),
                    "format failed",
                )
            }
            PyMorphemeBacking::Single { morpheme, .. } => errors::wrap_ctx(
                write!(
                    out,
                    "<Morpheme({}, {}:{}, {})>",
                    morpheme.surface(),
                    morpheme.begin_c(),
                    morpheme.end_c(),
                    morpheme.word_id()
                ),
                "format failed",
            ),
        }
    }

    fn self_equivalent(&self, py: Python<'_>) -> PyMorpheme {
        match &self.backing {
            PyMorphemeBacking::List { list, index } => {
                PyMorpheme::list_backed(list.clone_ref(py), *index)
            }
            PyMorphemeBacking::Single {
                morpheme,
                projection,
            } => PyMorpheme::single_backed_boxed((*morpheme).clone(), projection.clone()),
        }
    }

    fn with_morpheme<'py, R>(
        &'py self,
        py: Python<'py>,
        f: impl for<'m> FnOnce(&'m dyn MorphemeView<Dictionary = Arc<PyDicData>>) -> R,
    ) -> PyResult<R> {
        match &self.backing {
            PyMorphemeBacking::List { list, index } => {
                let mrp = Self::borrow_morpheme(list, py, *index)?;
                Ok(f(mrp.deref()))
            }
            PyMorphemeBacking::Single { morpheme, .. } => Ok(f(morpheme.as_ref())),
        }
    }

    fn form_morpheme<'py>(
        &'py self,
        py: Python<'py>,
        kind: FormMorphemeKind,
        context: &str,
    ) -> PyResult<PyMorpheme> {
        match &self.backing {
            PyMorphemeBacking::List { list, index } => {
                let morph = Self::borrow_morpheme(list, py, *index)?;
                let form_morpheme = match kind {
                    FormMorphemeKind::Dictionary => morph.dictionary_form_morpheme(),
                    FormMorphemeKind::Normalized => morph.normalized_form_morpheme(),
                };

                match errors::wrap_ctx(form_morpheme, context)? {
                    RustMorphemeRef::ListItem(_) => Ok(self.self_equivalent(py)),
                    RustMorphemeRef::Single(morpheme) => Ok(PyMorpheme::single_backed(
                        *morpheme,
                        morph.list.projection.clone(),
                    )),
                    _ => errors::wrap(Err("unsupported Rust morpheme reference variant; \
                         sudachipy bindings are out of sync with sudachi core")),
                }
            }
            PyMorphemeBacking::Single {
                morpheme,
                projection,
            } => {
                let form_morpheme = match kind {
                    FormMorphemeKind::Dictionary => morpheme.dictionary_form_morpheme(),
                    FormMorphemeKind::Normalized => morpheme.normalized_form_morpheme(),
                };

                match errors::wrap_ctx(form_morpheme, context)? {
                    RustMorphemeRef::Single(morpheme) => {
                        Ok(PyMorpheme::single_backed(*morpheme, projection.clone()))
                    }
                    RustMorphemeRef::ListItem(_) => Ok(self.self_equivalent(py)),
                    _ => errors::wrap(Err("unsupported Rust morpheme reference variant; \
                         sudachipy bindings are out of sync with sudachi core")),
                }
            }
        }
    }

    fn split_single_backed<'py>(
        &'py self,
        py: Python<'py>,
        mode: &Bound<'py, PyAny>,
        out: Option<Bound<'py, PyMorphemeListWrapper>>,
        add_single: Option<bool>,
    ) -> PyResult<Bound<'py, PyMorphemeListWrapper>> {
        let mode = extract_mode(mode)?;

        let (splits, projection) = match &self.backing {
            PyMorphemeBacking::Single {
                morpheme,
                projection,
            } => {
                let splits =
                    errors::wrap_ctx(morpheme.split(mode), "Error while splitting morpheme")?;
                let no_split = splits.len() == 1 && splits[0].word_id() == morpheme.word_id();
                let splits = if add_single.unwrap_or(true) || !no_split {
                    splits
                } else {
                    Vec::new()
                };
                (splits, projection.clone())
            }
            PyMorphemeBacking::List { .. } => {
                return errors::wrap(Err("split_single_backed called for list-backed morpheme"));
            }
        };

        match out {
            None => Bound::new(py, PyMorphemeListWrapper::from_singles(splits, projection)),
            Some(cell) => {
                {
                    let mut wrapper = match cell.try_borrow_mut() {
                        Ok(wrapper) => wrapper,
                        Err(_) => return errors::wrap(Err("out was used twice at the same time")),
                    };
                    wrapper.replace_with_singles(splits, projection);
                }
                Ok(cell)
            }
        }
    }

    fn split_list_backed<'py>(
        &'py self,
        py: Python<'py>,
        mode: &Bound<'py, PyAny>,
        out: Option<Bound<'py, PyMorphemeListWrapper>>,
        add_single: Option<bool>,
    ) -> PyResult<Bound<'py, PyMorphemeListWrapper>> {
        let mode = extract_mode(mode)?;
        let (list_py, index) = match &self.backing {
            PyMorphemeBacking::List { list, index } => (list.clone_ref(py), *index),
            PyMorphemeBacking::Single { .. } => {
                return errors::wrap(Err("split_list_backed called for single-backed morpheme"));
            }
        };

        let list_ref = list_py.borrow(py);
        let source_list = list_ref.as_list()?;
        let dict = source_list.dict().clone();
        let projection = list_ref.projection.clone();

        let out_cell = match out {
            None => Bound::new(
                py,
                PyMorphemeListWrapper::from_components(
                    MorphemeList::empty(dict.clone()),
                    projection.clone(),
                ),
            )?,
            Some(cell) => cell,
        };

        let mut borrow = out_cell.try_borrow_mut();
        let out_ref = match borrow {
            Ok(ref mut v) => v.replace_with_empty_list(dict, projection)?,
            Err(_) => return errors::wrap(Err("out was used twice at the same time")),
        };

        out_ref.clear();
        let splitted = errors::wrap_ctx(
            source_list.split_into(mode, index, out_ref),
            "Error while splitting morpheme",
        )?;

        if add_single.unwrap_or(true) && !splitted {
            source_list.copy_slice(index, index + 1, out_ref);
        }

        Ok(out_cell)
    }

    fn dict_pos<'py>(&'py self, py: Python<'py>, pos_id: u16) -> PyResult<Py<PyTuple>> {
        match &self.backing {
            PyMorphemeBacking::List { list, .. } => {
                let list = list.borrow(py);
                Ok(list.as_list()?.dict().pos_of(pos_id).clone_ref(py))
            }
            PyMorphemeBacking::Single { morpheme, .. } => Ok(MorphemeView::dict(morpheme.as_ref())
                .pos_of(pos_id)
                .clone_ref(py)),
        }
    }
}

#[pymethods]
impl PyMorpheme {
    /// Returns the begin index of this in the input text.
    #[pyo3(text_signature = "(self, /) -> int")]
    fn begin(&self, py: Python) -> PyResult<usize> {
        // call codepoint version
        self.with_morpheme(py, |m| m.begin_c())
    }

    /// Returns the end index of this in the input text.
    #[pyo3(text_signature = "(self, /) -> int")]
    fn end(&self, py: Python) -> PyResult<usize> {
        // call codepoint version
        self.with_morpheme(py, |m| m.end_c())
    }

    /// Returns the substring of input text corresponding to the morpheme, or a projection if one is configured.
    ///
    /// See `Config.projection`.
    #[pyo3(text_signature = "(self, /) -> str")]
    fn surface<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        match &self.backing {
            PyMorphemeBacking::List { list, index } => {
                let morph = Self::borrow_morpheme(list, py, *index)?;
                match morph.list.projection() {
                    None => {
                        let surface = morph.surface();
                        let result = PyString::new(py, surface.deref());
                        Ok(result)
                    }
                    Some(proj) => Ok(proj.project(morph.deref(), py)),
                }
            }
            PyMorphemeBacking::Single {
                morpheme,
                projection,
            } => match projection {
                None => Ok(PyString::new(py, morpheme.surface())),
                Some(proj) => Ok(proj.project(morpheme.as_ref(), py)),
            },
        }
    }

    /// Returns the substring of input text corresponding to the morpheme regardless the configured projection.
    ///
    /// See `Config.projection`.
    #[pyo3(text_signature = "(self, /) -> str")]
    fn raw_surface<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        match &self.backing {
            PyMorphemeBacking::List { list, index } => {
                let morph = Self::borrow_morpheme(list, py, *index)?;
                let surface = morph.surface();
                let result = PyString::new(py, surface.deref());
                Ok(result)
            }
            PyMorphemeBacking::Single { morpheme, .. } => Ok(PyString::new(py, morpheme.surface())),
        }
    }

    /// Returns the part of speech as a six-element tuple.
    /// Tuple elements are four POS levels, conjugation type and conjugation form.
    #[pyo3(text_signature = "(self, /) -> tuple[str, str, str, str, str, str]")]
    fn part_of_speech<'py>(&'py self, py: Python<'py>) -> PyResult<Py<PyTuple>> {
        let pos_id = self.part_of_speech_id(py)?;
        self.dict_pos(py, pos_id)
    }

    /// Returns the id of the part of speech in the dictionary.
    #[pyo3(text_signature = "(self, /) -> int")]
    pub fn part_of_speech_id(&self, py: Python) -> PyResult<u16> {
        self.with_morpheme(py, |m| m.part_of_speech_id())
    }

    /// Returns the dictionary form.
    #[pyo3(text_signature = "(self, /) -> str")]
    fn dictionary_form<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        self.with_morpheme(py, |m| PyString::new(py, m.dictionary_form()))
    }

    /// Returns the morpheme corresponding to this morpheme's dictionary form.
    ///
    /// For out-of-vocabulary morphemes, invalid references, and references to
    /// the same dictionary entry, returns a morpheme equivalent to ``self``.
    /// For a distinct referenced entry, returns a standalone morpheme whose
    /// begin/end offsets are ``0..len(surface)``.
    #[pyo3(text_signature = "(self, /) -> Morpheme")]
    fn dictionary_form_morpheme<'py>(&'py self, py: Python<'py>) -> PyResult<PyMorpheme> {
        self.form_morpheme(
            py,
            FormMorphemeKind::Dictionary,
            "Failed to load dictionary form morpheme",
        )
    }

    /// Returns the normalized form.
    #[pyo3(text_signature = "(self, /) -> str")]
    fn normalized_form<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        self.with_morpheme(py, |m| PyString::new(py, m.normalized_form()))
    }

    /// Returns the morpheme corresponding to this morpheme's normalized form.
    ///
    /// For out-of-vocabulary morphemes, invalid references, and references to
    /// the same dictionary entry, returns a morpheme equivalent to ``self``.
    /// For a distinct referenced entry, returns a standalone morpheme whose
    /// begin/end offsets are ``0..len(surface)``.
    #[pyo3(text_signature = "(self, /) -> Morpheme")]
    fn normalized_form_morpheme<'py>(&'py self, py: Python<'py>) -> PyResult<PyMorpheme> {
        self.form_morpheme(
            py,
            FormMorphemeKind::Normalized,
            "Failed to load normalized form morpheme",
        )
    }

    /// Returns the reading form.
    #[pyo3(text_signature = "(self, /) -> str")]
    fn reading_form<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        self.with_morpheme(py, |m| PyString::new(py, m.reading_form()))
    }

    /// Returns sub-morphemes in the provided split mode.
    ///
    /// :param mode: mode of new split.
    /// :param out: write results to this MorphemeList instead of creating new one.
    ///     See https://worksapplications.github.io/sudachi.rs/python/topics/out_param.html for
    ///     more information on output parameters.
    ///     Returned MorphemeList will be invalidated if this MorphemeList is used as an output parameter.
    /// :param add_single: return lists with the current morpheme if the split hasn't produced any elements.
    ///     When False is passed, empty lists are returned instead.
    ///
    /// :type mode: SplitMode | None
    /// :type out: MorphemeList | None
    /// :type add_single: bool
    #[pyo3(
        signature = (mode, out=None, add_single=false),
        text_signature = "(self, /, mode, out=None, add_single=False) -> MorphemeList",
    )]
    fn split<'py>(
        &'py self,
        py: Python<'py>,
        mode: &Bound<'py, PyAny>,
        out: Option<Bound<'py, PyMorphemeListWrapper>>,
        add_single: Option<bool>,
    ) -> PyResult<Bound<'py, PyMorphemeListWrapper>> {
        match &self.backing {
            PyMorphemeBacking::List { .. } => self.split_list_backed(py, mode, out, add_single),
            PyMorphemeBacking::Single { .. } => self.split_single_backed(py, mode, out, add_single),
        }
    }

    /// Returns whether if this is out of vocabulary word.
    #[pyo3(text_signature = "(self, /) -> bool")]
    fn is_oov(&self, py: Python) -> PyResult<bool> {
        self.with_morpheme(py, |m| m.is_oov())
    }

    /// Returns word id of this word in the dictionary.
    #[pyo3(text_signature = "(self, /) -> int")]
    fn word_id(&self, py: Python) -> PyResult<u32> {
        self.with_morpheme(py, |m| m.word_id().as_raw())
    }

    /// Returns the dictionary id which this word belongs.
    #[pyo3(text_signature = "(self, /) -> int")]
    fn dictionary_id(&self, py: Python) -> PyResult<i32> {
        self.with_morpheme(py, |m| m.dictionary_id())
    }

    /// Returns the list of synonym group ids.
    #[pyo3(text_signature = "(self, /) -> List[int]")]
    fn synonym_group_ids<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        match &self.backing {
            PyMorphemeBacking::List { list, index } => {
                let mref = Self::borrow_morpheme(list, py, *index)?;
                PyList::new(py, mref.get_word_info().synonym_group_ids())
            }
            PyMorphemeBacking::Single { morpheme, .. } => {
                PyList::new(py, morpheme.get_word_info().synonym_group_ids())
            }
        }
    }

    /// Returns user-defined data associated with this morpheme.
    #[pyo3(text_signature = "(self, /) -> str")]
    fn user_data<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        self.with_morpheme(py, |m| PyString::new(py, m.user_data()))
    }

    /// Returns morpheme length in codepoints.
    pub fn __len__(&self, py: Python) -> PyResult<usize> {
        self.with_morpheme(py, |m| m.end_c() - m.begin_c())
    }

    pub fn __str__<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        self.surface(py)
    }

    pub fn __repr__<'py>(&'py self, py: Python<'py>) -> PyResult<String> {
        let mut result = String::new();
        self.write_repr(py, &mut result)?;
        Ok(result)
    }
}
