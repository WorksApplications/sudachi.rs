use std::sync::Arc;

use pyo3::prelude::*;

use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::dic::lexicon::word_infos::WordInfo;
use sudachi::prelude::*;

#[pyclass]
pub struct PyMorpheme {
    surface: String,
    word_info: WordInfo,
    is_oov: bool,
    dictionary_id: i32,
    dict: Arc<JapaneseDictionary>,
}

impl PyMorpheme {
    pub fn new(m: Morpheme, dict: Arc<JapaneseDictionary>) -> Self {
        Self {
            surface: m.surface().clone(),
            word_info: m.word_info,
            is_oov: m.is_oov,
            dictionary_id: m.dictionary_id,
            dict,
        }
    }
}

#[pymethods]
impl PyMorpheme {
    // fn begin(&self) -> usize {}  // need to update morpheme
    // fn end(&self) -> usize {}  // need to update morpheme
    fn surface(&self) -> &str {
        &self.surface
    }
    fn part_of_speech(&self) -> Vec<String> {
        self.dict
            .grammar()
            .pos_list
            .get(self.part_of_speech_id() as usize)
            .unwrap()
            .clone()
    }
    fn part_of_speech_id(&self) -> u16 {
        self.word_info.pos_id
    }
    fn dictionary_form(&self) -> &str {
        &self.word_info.dictionary_form
    }
    fn normalized_form(&self) -> &str {
        &self.word_info.normalized_form
    }
    fn reading_form(&self) -> &str {
        &self.word_info.reading_form
    }
    // fn split(&self, mode: Option<Mode>) -> Vec<PyMorpheme> {}
    fn is_oov(&self) -> bool {
        self.is_oov
    }
    // fn word_id(&self) -> u32 {}  // need to update morpheme
    fn dictionary_id(&self) -> i32 {
        self.dictionary_id
    }
    fn synonym_group_ids(&self) -> Vec<u32> {
        self.word_info.synonym_group_ids.clone()
    }
    fn get_word_info(&self) -> PyWordInfo {
        self.word_info.clone().into()
    }
}

#[pyclass]
struct PyWordInfo {
    pub surface: String,
    pub head_word_length: u16,
    pub pos_id: u16,
    pub normalized_form: String,
    pub dictionary_form_word_id: i32,
    pub dictionary_form: String,
    pub reading_form: String,
    pub a_unit_split: Vec<u32>,
    pub b_unit_split: Vec<u32>,
    pub word_structure: Vec<u32>,
    pub synonym_group_ids: Vec<u32>,
}

impl From<WordInfo> for PyWordInfo {
    fn from(word_info: WordInfo) -> Self {
        Self {
            surface: word_info.surface,
            head_word_length: word_info.head_word_length,
            pos_id: word_info.pos_id,
            normalized_form: word_info.normalized_form,
            dictionary_form_word_id: word_info.dictionary_form_word_id,
            dictionary_form: word_info.dictionary_form,
            reading_form: word_info.reading_form,
            a_unit_split: word_info.a_unit_split,
            b_unit_split: word_info.b_unit_split,
            word_structure: word_info.word_structure,
            synonym_group_ids: word_info.synonym_group_ids,
        }
    }
}

#[pymethods]
impl PyWordInfo {
    fn length(&self) -> u16 {
        self.head_word_length
    }
}
