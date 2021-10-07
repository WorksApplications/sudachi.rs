use pyo3::prelude::*;

use sudachi::dic::lexicon::word_infos::WordInfo;

#[pyclass(module = "sudachi.wordinfo", name = "WordInfo")]
pub struct PyWordInfo {
    #[pyo3(get)]
    surface: String,
    #[pyo3(get)]
    head_word_length: u16,
    #[pyo3(get)]
    pos_id: u16,
    #[pyo3(get)]
    normalized_form: String,
    #[pyo3(get)]
    dictionary_form_word_id: i32,
    #[pyo3(get)]
    dictionary_form: String,
    #[pyo3(get)]
    reading_form: String,
    #[pyo3(get)]
    a_unit_split: Vec<u32>,
    #[pyo3(get)]
    b_unit_split: Vec<u32>,
    #[pyo3(get)]
    word_structure: Vec<u32>,
    #[pyo3(get)]
    synonym_group_ids: Vec<u32>,
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
