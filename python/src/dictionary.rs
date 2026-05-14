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

use std::convert::TryFrom;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::{PySet, PyString, PyTuple};

use sudachi::analysis::Mode;
use sudachi::config::{Config, ConfigBuilder, SurfaceProjection};
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::dic::grammar::Grammar;
use sudachi::dic::lexicon_set::LexiconSet;
use sudachi::dic::subset::InfoSubset;
use sudachi::dic::{DictionaryAccess, LexiconAccess};
use sudachi::plugin::input_text::InputTextPlugin;
use sudachi::plugin::oov::OovProviderPlugin;
use sudachi::plugin::path_rewrite::PathRewritePlugin;

use crate::errors;
use crate::morpheme::PyMorphemeListWrapper;
use crate::pos_matcher::PyPosMatcher;
use crate::pretokenizer::PyPretokenizer;
use crate::projection::{pyprojection, PyProjector};
use crate::tokenizer::{PySplitMode, PyTokenizer};

pub(crate) struct PyDicData {
    pub(crate) dictionary: JapaneseDictionary,
    pub(crate) pos: Vec<Py<PyTuple>>,
    /// Compute default string representation for a morpheme using vtable dispatch.
    /// None by default (if outputting surface as it is)
    /// This is default per-dictionary value, can be overridden when creating tokenizers and pre-tokenizers
    pub(crate) projection: PyProjector,
}

impl DictionaryAccess for PyDicData {
    fn grammar(&self) -> &Grammar<'_> {
        self.dictionary.grammar()
    }

    fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin + Sync + Send>] {
        self.dictionary.input_text_plugins()
    }

    fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin + Sync + Send>] {
        self.dictionary.oov_provider_plugins()
    }

    fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin + Sync + Send>] {
        self.dictionary.path_rewrite_plugins()
    }
}

impl LexiconAccess for PyDicData {
    fn lexicon(&self) -> &LexiconSet<'_> {
        self.dictionary.lexicon()
    }
}

impl PyDicData {
    pub fn pos_of(&self, pos_id: u16) -> &Py<PyTuple> {
        &self.pos[pos_id as usize]
    }
}

/// A sudachi dictionary.
///
/// If both config.systemDict and dict are not given, `sudachidict_core` is used.
/// If both config.systemDict and dict are given, dict is used.
/// If dict is an absolute path to a file, it is used as a dictionary.
///
/// :param config_path: path to the configuration JSON file, config json as a string, or a [sudachipy.Config] object.
/// :param config: alias to config_path, only one of them can be specified at the same time.
/// :param resource_dir: path to the resource directory folder.
/// :param dict: type of pre-packaged dictionary, referring to sudachidict_<dict> packages on PyPI: https://pypi.org/search/?q=sudachidict.
///     Also, can be an _absolute_ path to a compiled dictionary file.
/// :param dict_type: deprecated alias to dict.
///
/// :type config_path: Config | pathlib.Path | str | None
/// :type config: Config | pathlib.Path | str | None
/// :type resource_dir: pathlib.Path | str | None
/// :type dict: pathlib.Path | str | None
/// :type dict_type: pathlib.Path | str | None
#[pyclass(module = "sudachipy.dictionary", name = "Dictionary")]
#[derive(Clone)]
pub struct PyDictionary {
    pub(super) dictionary: Option<Arc<PyDicData>>,
    pub config: Config,
}

#[pymethods]
impl PyDictionary {
    /// Creates a sudachi dictionary.
    ///
    /// If both config.systemDict and dict are not given, `sudachidict_core` is used.
    /// If both config.systemDict and dict are given, dict is used.
    /// If dict is an absolute path to a file, it is used as a dictionary.
    ///
    /// :param config_path: path to the configuration JSON file, config json as a string, or a [sudachipy.Config] object.
    /// :param config: alias to config_path, only one of them can be specified at the same time.
    /// :param resource_dir: path to the resource directory folder.
    /// :param dict: type of pre-packaged dictionary, referring to sudachidict_<dict> packages on PyPI: https://pypi.org/search/?q=sudachidict.
    ///     Also, can be an _absolute_ path to a compiled dictionary file.
    /// :param dict_type: deprecated alias to dict.
    ///
    /// :type config_path: Config | pathlib.Path | str | None
    /// :type config: Config | pathlib.Path | str | None
    /// :type resource_dir: pathlib.Path | str | None
    /// :type dict: pathlib.Path | str | None
    /// :type dict_type: pathlib.Path | str | None
    #[new]
    #[pyo3(
        text_signature="(config_path=None, resource_dir=None, dict=None, dict_type=None, *, config=None) -> Dictionary",
        signature=(config_path=None, resource_dir=None, dict=None, dict_type=None, *, config=None)
    )]
    fn new(
        py: Python,
        config_path: Option<&Bound<PyAny>>,
        resource_dir: Option<PathBuf>,
        dict: Option<&str>,
        dict_type: Option<&str>,
        config: Option<&Bound<PyAny>>,
    ) -> PyResult<Self> {
        if config.is_some() && config_path.is_some() {
            return errors::wrap(Err("Both config and config_path options were specified at the same time, use one of them"));
        }

        let default_config = read_default_config(py)?;

        let config_builder = match config.or(config_path) {
            None => default_config,
            Some(v) => read_config(v)?.fallback(&default_config),
        };

        let resource_dir = match resource_dir {
            None => Some(get_default_resource_dir(py)?),
            Some(v) => Some(v),
        };

        let dict_path = match dict.or(dict_type) {
            None => None,
            Some(dt) => Some(locate_system_dict(py, Path::new(dt))?),
        };

        if dict_type.is_some() {
            errors::warn_deprecation(
                py,
                c_str!("Parameter dict_type of Dictionary() is deprecated, use dict instead"),
            )?
        }

        let config_builder = match resource_dir {
            Some(p) => config_builder.resource_path(p),
            None => config_builder,
        };

        let config_builder = match dict_path {
            Some(p) => config_builder.system_dict(p),
            None => config_builder,
        };

        let mut config = config_builder.build();

        // Load a dictionary from `sudachidict_core` as the default one.
        // For this behavior, the value of `systemDict` key in the default setting file must be
        // empty (or no `systemDict` key), different from rust's one.
        if config.system_dict.is_none() || config.system_dict.as_ref().unwrap().is_dir() {
            let system_dict = find_dict_path(py, "core")?;
            assert!(
                system_dict.exists(),
                "system dictionary {} did not exist",
                system_dict.display()
            );
            config.system_dict = Some(system_dict);
        } else {
            // resolve system dictionary alias to full path
            let system_dict = config.system_dict.as_deref().unwrap();
            if let Some(kind @ ("small" | "core" | "full")) = system_dict.to_str() {
                let system_dict = find_dict_path(py, kind)?;
                assert!(
                    system_dict.exists(),
                    "system dictionary {} did not exist",
                    system_dict.display()
                );
                config.system_dict = Some(system_dict)
            }
        }

        let jdic = errors::wrap_ctx(
            JapaneseDictionary::from_cfg(&config),
            "Error while constructing dictionary",
        )?;

        let pos_data = jdic
            .grammar()
            .pos_list
            .iter()
            .map(|pos| {
                let tuple: Py<PyTuple> = PyTuple::new(py, pos)
                    .expect("failed to convert POS tuple")
                    .unbind();
                tuple
            })
            .collect();

        let projection = pyprojection(config.projection, &jdic);

        let dic_data = PyDicData {
            dictionary: jdic,
            pos: pos_data,
            projection,
        };

        let dictionary = Arc::new(dic_data);

        Ok(Self {
            config,
            dictionary: Some(dictionary),
        })
    }

    /// Creates a sudachi tokenizer.
    ///
    /// :param mode: sets the analysis mode for this Tokenizer
    /// :param fields: load only a subset of fields.
    ///     See https://worksapplications.github.io/sudachi.rs/python/topics/subsetting.html.
    /// :param projection: Projection override for created Tokenizer. See Config.projection for values.
    ///
    /// :type mode: SplitMode | str | None
    /// :type fields: set[str] | None
    /// :type projection: str | None
    #[pyo3(
        text_signature="(self, /, mode=SplitMode.C, fields=None, *, projection=None) -> Tokenizer",
        signature=(mode=None, fields=None, *, projection=None)
    )]
    fn create<'py>(
        &'py self,
        mode: Option<&Bound<'py, PyAny>>,
        fields: Option<&Bound<'py, PySet>>,
        projection: Option<&Bound<'py, PyString>>,
    ) -> PyResult<PyTokenizer> {
        let mode = match mode {
            Some(m) => extract_mode(m)?,
            None => Mode::C,
        };
        let fields = parse_field_subset(fields)?;
        let dict = self.dictionary.as_ref().unwrap().clone();

        let (projection, required_fields) = if let Some(s) = projection {
            let projection = errors::wrap(SurfaceProjection::try_from(s.to_str()?))?;
            (
                pyprojection(projection, &dict),
                projection.required_subset(),
            )
        } else {
            (
                dict.projection.clone(),
                self.config.projection.required_subset(),
            )
        };

        let tok = PyTokenizer::new(dict, mode, fields | required_fields, projection);
        Ok(tok)
    }

    /// Creates a POS matcher object
    ///
    /// If target is a function, then it must return whether a POS should match or not.
    /// If target is a list, it should contain partially specified POS.
    /// By partially specified it means that it is possible to omit POS fields or use None as a sentinel value that matches any POS.
    ///
    /// For example, ('名詞',) will match any noun and
    /// (None, None, None, None, None, '終止形‐一般') will match any word in 終止形‐一般 conjugation form.
    ///
    /// :param target: can be either a list of POS partial tuples or a callable which maps POS to bool.
    ///
    /// :type target: Iterable[PartialPOS] | Callable[[POS], bool]
    fn pos_matcher<'py>(&'py self, target: &Bound<'py, PyAny>) -> PyResult<PyPosMatcher> {
        PyPosMatcher::create(self.dictionary.as_ref().unwrap(), target)
    }

    /// Creates HuggingFace Tokenizers-compatible PreTokenizer.
    /// Requires package `tokenizers` to be installed.     
    ///
    /// :param mode: Use this split mode (C by default)
    /// :param fields: ask Sudachi to load only a subset of fields.
    ///     See https://worksapplications.github.io/sudachi.rs/python/topics/subsetting.html.
    ///     Only used when `handler` is set.
    /// :param handler: a custom callable to transform MorphemeList into list of tokens. If None, simply use surface as token representations.
    ///     Overrides `projection`.
    ///     It should be a `function(index: int, original: NormalizedString, morphemes: MorphemeList) -> List[NormalizedString]`.
    ///     See https://github.com/huggingface/tokenizers/blob/master/bindings/python/examples/custom_components.py.
    ///     If nothing was passed, simply use surface as token representations.
    /// :param projection: Projection override for created Tokenizer. See Config.projection for supported values.
    ///
    /// :type mode: SplitMode | str | None
    /// :type fields: set[str] | None
    /// :type handler: Callable[[int, NormalizedString, MorphemeList], list[NormalizedString]] | None
    /// :type projection: str | None
    #[pyo3(
        text_signature="(self, /, mode=None, fields=None, handler=None, *, projection=None) -> tokenizers.PreTokenizer",
        signature=(mode=None, fields=None, handler=None, *, projection=None)
    )]
    fn pre_tokenizer<'py>(
        &'py self,
        py: Python<'py>,
        mode: Option<&Bound<'py, PyAny>>,
        fields: Option<&Bound<'py, PySet>>,
        handler: Option<Py<PyAny>>,
        projection: Option<&Bound<'py, PyString>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mode = match mode {
            Some(m) => extract_mode(m)?,
            None => Mode::C,
        };

        if let Some(h) = handler.as_ref() {
            if !h.bind(py).is_callable() {
                return errors::wrap(Err("handler must be callable"));
            }
        }

        let dict = self.dictionary.as_ref().unwrap().clone();

        // morphemes will be consumed inside pretokenizer therefore we only need fields used by handler or projection
        let (projection, required_fields) = if handler.is_some() {
            // pretokenizer won't use projection when handler is set.
            (
                None,
                self.config.projection.required_subset() | parse_field_subset(fields)?,
            )
        } else if let Some(s) = projection {
            let projection = errors::wrap(SurfaceProjection::try_from(s.to_str()?))?;
            // use default projection if "surface" is specified (see #259)
            if projection == SurfaceProjection::Surface {
                (
                    dict.projection.clone(),
                    self.config.projection.required_subset(),
                )
            } else {
                (
                    pyprojection(projection, &dict),
                    projection.required_subset(),
                )
            }
        } else {
            (
                dict.projection.clone(),
                self.config.projection.required_subset(),
            )
        };

        let pretokenizer = PyPretokenizer::new(dict, mode, required_fields, handler, projection);
        let module = py.import("tokenizers.pre_tokenizers")?;
        module
            .getattr("PreTokenizer")?
            .getattr("custom")?
            .call1((pretokenizer,))
    }

    /// Look up morphemes in the binary dictionary without performing the analysis.
    ///
    /// All morphemes from the dictionary with the given surface string are returned,
    /// with the last user dictionary searched first and the system dictionary searched last.
    /// Inside a dictionary, morphemes are outputted in-binary-dictionary order.
    /// Morphemes which are not indexed are not returned.
    ///
    /// :param surface: find all morphemes with the given surface
    /// :param out: if passed, reuse the given morpheme list instead of creating a new one.
    ///     See https://worksapplications.github.io/sudachi.rs/python/topics/out_param.html for details.
    ///
    /// :type surface: str
    /// :type out: MorphemeList | None
    #[pyo3(
        signature = (surface, out=None),
        text_signature = "(self, /, surface, out=None) -> MorphemeList",
    )]
    fn lookup<'py>(
        &'py self,
        py: Python<'py>,
        surface: &'py str,
        out: Option<Bound<'py, PyMorphemeListWrapper>>,
    ) -> PyResult<Bound<'py, PyMorphemeListWrapper>> {
        let l = match out {
            Some(l) => l,
            None => {
                let list = PyMorphemeListWrapper::new(self.dictionary.clone().unwrap());
                Bound::new(py, list)?
            }
        };

        let dict = self.dictionary.clone().unwrap();
        let projection = dict.projection.clone();
        // this needs to be a variable
        let mut borrow = l.try_borrow_mut();
        let out_list = match borrow {
            Ok(ref mut ms) => ms.replace_with_empty_list(dict, projection)?,
            Err(_) => return errors::wrap(Err("out was used twice at the same time")),
        };

        out_list.clear();
        errors::wrap_ctx(out_list.lookup(surface, InfoSubset::all()), surface)?;
        Ok(l)
    }

    /// Close this dictionary.
    #[pyo3(text_signature = "(self, /) -> ()")]
    fn close(&mut self) {
        self.dictionary = None;
    }

    /// Returns POS with the given id.
    ///
    /// :param pos_id: POS id
    /// :return: POS tuple with the given id or None for non existing id.
    ///
    /// :type pos_id: int
    #[pyo3(text_signature = "(self, /, pos_id: int) -> tuple[str, str, str, str, str, str] | None")]
    fn pos_of<'py>(&'py self, py: Python<'py>, pos_id: usize) -> Option<&'py Bound<'py, PyTuple>> {
        let dic = self.dictionary.as_ref().unwrap();
        dic.pos.get(pos_id).map(|x| x.bind(py))
    }

    fn __repr__(&self) -> PyResult<String> {
        errors::wrap(config_repr(&self.config))
    }
}

fn config_repr(cfg: &Config) -> Result<String, std::fmt::Error> {
    let mut result = String::from("<SudachiDictionary(");
    match cfg.resolved_system_dict() {
        Ok(path) => write!(result, "system={}", path.display()),
        Err(e) => write!(result, "system=<err:{}>", e),
    }?;
    write!(result, ", user=[")?;
    match cfg.resolved_user_dicts() {
        Ok(dicts) => {
            for (i, dic) in dicts.iter().enumerate() {
                if i != 0 {
                    write!(result, ", ")?;
                }
                write!(result, "{}", dic.display())?;
            }
        }
        Err(e) => {
            write!(result, "<err:{:?}>", e)?;
        }
    }
    write!(result, "])>")?;
    Ok(result)
}

pub(crate) fn extract_mode(mode: &Bound<'_, PyAny>) -> PyResult<Mode> {
    if mode.is_instance_of::<PyString>() {
        errors::wrap(Mode::from_str(mode.str()?.to_str()?))
    } else if mode.is_instance_of::<PySplitMode>() {
        let mode = mode.extract::<PySplitMode>()?;
        Ok(Mode::from(mode))
    } else {
        errors::wrap(Err(format!(
            "mode should be sudachipy.SplitMode or str, was {}: {}",
            mode,
            mode.get_type()
        )))
    }
}

fn read_config_from_fs(path: Option<&Path>) -> PyResult<ConfigBuilder> {
    errors::wrap(ConfigBuilder::from_opt_file(path))
}

fn read_config(config_opt: &Bound<PyAny>) -> PyResult<ConfigBuilder> {
    if config_opt.is_instance_of::<PyString>() {
        let config_pystr = config_opt.str()?;
        let config_str = config_pystr.to_str()?.trim();
        // looks like json
        if config_str.starts_with('{') && config_str.ends_with('}') {
            let result = ConfigBuilder::from_bytes(config_str.as_bytes());
            return errors::wrap(result);
        }
        let p = Path::new(config_str);
        if p.exists() && p.is_file() {
            return read_config_from_fs(Some(p));
        }
        return errors::wrap(Err(format!(
            "config file [{}] do not exist or is not a file",
            p.display()
        )));
    }
    let py = config_opt.py();
    let cfg_type = py.import("sudachipy.config")?.getattr("Config")?;
    if config_opt.is_instance(&cfg_type)? {
        let cfg_as_str = config_opt.call_method0("as_jsons")?;
        return read_config(&cfg_as_str);
    }
    errors::wrap(Err(format!(
        "config should be sudachipy.Config or str which represents a file path or json obj, was {}: {}",
        config_opt,
        config_opt.get_type()
    )))
}

pub(crate) fn read_default_config(py: Python) -> PyResult<ConfigBuilder> {
    let path = py.import("sudachipy")?.getattr("_DEFAULT_SETTINGFILE")?;
    let path = path.cast::<PyString>()?.to_str()?;
    let path = PathBuf::from(path);
    errors::wrap_ctx(ConfigBuilder::from_opt_file(Some(&path)), &path)
}

pub(crate) fn get_default_resource_dir(py: Python) -> PyResult<PathBuf> {
    let path = py.import("sudachipy")?.getattr("_DEFAULT_RESOURCEDIR")?;
    let path = path.cast::<PyString>()?.to_str()?;
    Ok(PathBuf::from(path))
}

fn find_dict_path(py: Python, dict_type: &str) -> PyResult<PathBuf> {
    let pyfunc = py.import("sudachipy")?.getattr("_find_dict_path")?;
    let path = pyfunc.call1((dict_type,))?;
    let path = path.cast::<PyString>()?.to_str()?;
    Ok(PathBuf::from(path))
}

fn locate_system_dict(py: Python, path: &Path) -> PyResult<PathBuf> {
    if path.exists() && path.is_file() {
        return Ok(path.to_owned());
    }
    match path.to_str() {
        Some(name @ ("small" | "core" | "full")) => find_dict_path(py, name),
        _ => errors::wrap(Err(format!("invalid dictionary path {:?}", path))),
    }
}

fn parse_field_subset(data: Option<&Bound<PySet>>) -> PyResult<InfoSubset> {
    if data.is_none() {
        return Ok(InfoSubset::all());
    }

    let mut subset = InfoSubset::empty();
    for elem in data.unwrap().iter() {
        subset |= match elem.str()?.to_str()? {
            "surface" => InfoSubset::HEADWORD,
            "pos" | "pos_id" => InfoSubset::POS_ID,
            "normalized_form" => InfoSubset::NORMALIZED_FORM,
            "dictionary_form" => InfoSubset::DICTIONARY_FORM,
            "reading_form" => InfoSubset::READING_FORM,
            "word_structure" => InfoSubset::WORD_STRUCTURE,
            "split_a" => InfoSubset::SPLIT_A,
            "split_b" => InfoSubset::SPLIT_B,
            "synonym_group_id" => InfoSubset::SYNONYM_GROUP_IDS,
            x => return errors::wrap(Err(format!("Invalid WordInfo field name {}", x))),
        };
    }
    Ok(subset)
}
