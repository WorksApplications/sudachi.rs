/*
 *  Copyright (c) 2023 Works Applications Co., Ltd.
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

use crate::dictionary::PyDicData;
use crate::morpheme::PyProjector;
use pyo3::types::PyString;
use pyo3::{PyResult, Python};
use std::convert::TryFrom;
use std::ops::Deref;
use std::sync::Arc;
use sudachi::analysis::stateless_tokenizer::DictionaryAccess;
use sudachi::config::SurfaceProjection;
use sudachi::pos::PosMatcher;
use sudachi::prelude::Morpheme;

pub(crate) trait MorphemeProjection {
    fn project<'py>(&self, m: &Morpheme<Arc<PyDicData>>, py: Python<'py>) -> &'py PyString;
}

struct Surface {}

impl MorphemeProjection for Surface {
    fn project<'py>(&self, m: &Morpheme<Arc<PyDicData>>, py: Python<'py>) -> &'py PyString {
        PyString::new(py, m.surface().deref())
    }
}

struct Mapped<F: for<'a> Fn(&'a Morpheme<'a, Arc<PyDicData>>) -> &'a str> {
    func: F,
}

impl<F: for<'a> Fn(&'a Morpheme<'a, Arc<PyDicData>>) -> &'a str> MorphemeProjection for Mapped<F> {
    fn project<'py>(&self, m: &Morpheme<Arc<PyDicData>>, py: Python<'py>) -> &'py PyString {
        PyString::new(py, (self.func)(m))
    }
}

struct DictionaryAndSurface {
    matcher: PosMatcher,
}

impl DictionaryAndSurface {
    fn new<D: DictionaryAccess>(dic: &D) -> Self {
        let matcher = conjugating_matcher(dic);
        DictionaryAndSurface { matcher }
    }
}

impl MorphemeProjection for DictionaryAndSurface {
    fn project<'py>(&self, m: &Morpheme<Arc<PyDicData>>, py: Python<'py>) -> &'py PyString {
        if self.matcher.matches_id(m.part_of_speech_id()) {
            PyString::new(py, m.surface().deref())
        } else {
            PyString::new(py, m.dictionary_form())
        }
    }
}

struct NormalizedAndSurface {
    matcher: PosMatcher,
}

impl NormalizedAndSurface {
    fn new<D: DictionaryAccess>(dic: &D) -> Self {
        let matcher = conjugating_matcher(dic);
        NormalizedAndSurface { matcher }
    }
}

impl MorphemeProjection for NormalizedAndSurface {
    fn project<'py>(&self, m: &Morpheme<Arc<PyDicData>>, py: Python<'py>) -> &'py PyString {
        if self.matcher.matches_id(m.part_of_speech_id()) {
            PyString::new(py, m.surface().deref())
        } else {
            PyString::new(py, m.normalized_form())
        }
    }
}

struct NormalizedNouns {
    matcher: PosMatcher,
}

impl NormalizedNouns {
    fn new<D: DictionaryAccess>(dic: &D) -> Self {
        let matcher = make_matcher(dic, |p| p[5] == "*");
        Self { matcher }
    }
}

impl MorphemeProjection for NormalizedNouns {
    fn project<'py>(&self, m: &Morpheme<Arc<PyDicData>>, py: Python<'py>) -> &'py PyString {
        if self.matcher.matches_id(m.part_of_speech_id()) {
            PyString::new(py, m.normalized_form())
        } else {
            PyString::new(py, m.surface().deref())
        }
    }
}

fn conjugating_matcher<D: DictionaryAccess>(dic: &D) -> PosMatcher {
    make_matcher(dic, |pos| match pos[0].deref() {
        "動詞" | "形容詞" | "助動詞" => true,
        _ => false,
    })
}

pub(crate) fn morpheme_projection<D: DictionaryAccess>(
    projection: SurfaceProjection,
    dict: &D,
) -> Arc<dyn MorphemeProjection + Send + Sync> {
    match projection {
        // implement for surface to make this function full
        SurfaceProjection::Surface => Arc::new(Surface {}),
        SurfaceProjection::Normalized => Arc::new(Mapped {
            func: |m| m.normalized_form(),
        }),
        SurfaceProjection::Reading => Arc::new(Mapped {
            func: |m| m.reading_form(),
        }),
        SurfaceProjection::Dictionary => Arc::new(Mapped {
            func: |m| m.dictionary_form(),
        }),
        SurfaceProjection::DictionaryAndSurface => Arc::new(DictionaryAndSurface::new(dict)),
        SurfaceProjection::NormalizedAndSurface => Arc::new(NormalizedAndSurface::new(dict)),
        SurfaceProjection::NormalizedNouns => Arc::new(NormalizedNouns::new(dict)),
    }
}

fn make_matcher<D: DictionaryAccess, F: FnMut(&Vec<String>) -> bool>(
    dic: &D,
    mut f: F,
) -> PosMatcher {
    let ids = dic.grammar().pos_list.iter().enumerate().filter_map(|p| {
        let (id, pos) = p;
        if f(pos) {
            Some(id as u16)
        } else {
            None
        }
    });
    PosMatcher::new(ids)
}

pub(crate) fn resolve_projection(base: PyProjector, fallback: &PyProjector) -> PyProjector {
    match (base, fallback) {
        (None, None) => None,
        (Some(p), _) => Some(p),
        (_, Some(p)) => Some(p.clone()),
    }
}

pub(crate) fn parse_projection<D: DictionaryAccess>(
    value: &PyString,
    dict: &D,
) -> PyResult<(PyProjector, SurfaceProjection)> {
    value.to_str().and_then(|s| parse_projection_raw(s, dict))
}

pub(crate) fn parse_projection_raw<D: DictionaryAccess>(
    value: &str,
    dict: &D,
) -> PyResult<(PyProjector, SurfaceProjection)> {
    match SurfaceProjection::try_from(value) {
        Ok(v) => {
            if v == SurfaceProjection::Surface {
                Ok((None, SurfaceProjection::Surface))
            } else {
                Ok((Some(morpheme_projection(v, dict)), v))
            }
        }
        Err(e) => Err(crate::errors::SudachiError::new_err(format!(
            "invalid surface projection: {e:?}"
        ))),
    }
}

pub(crate) fn parse_projection_opt<D: DictionaryAccess>(
    value: Option<&PyString>,
    dict: &D,
) -> PyResult<(PyProjector, SurfaceProjection)> {
    match value {
        None => Ok((None, SurfaceProjection::Surface)),
        Some(v) => parse_projection(v, dict),
    }
}
