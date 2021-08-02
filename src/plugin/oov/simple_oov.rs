use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::lattice::node::Node;
use crate::plugin::oov::OovProviderPlugin;
use crate::prelude::*;
use crate::utf8inputtext::Utf8InputText;

pub struct SimpleOovPlugin {
    left_id: i16,
    right_id: i16,
    cost: i16,
    oov_pos_id: u16,
}

impl SimpleOovPlugin {
    pub fn new(grammar: &Grammar) -> SudachiResult<SimpleOovPlugin> {
        // todo: load from file
        let left_id = 5968;
        let right_id = 5968;
        let cost = 3857;
        let oov_pos_string = vec!["補助記号", "一般", "*", "*", "*", "*"];

        let oov_pos_id = grammar
            .get_part_of_speech_id(&oov_pos_string)
            .ok_or(SudachiError::InvalidPartOfSpeech)?;

        Ok(SimpleOovPlugin {
            left_id,
            right_id,
            cost,
            oov_pos_id,
        })
    }
}

impl OovProviderPlugin for SimpleOovPlugin {
    fn provide_oov(
        &self,
        input_text: &Utf8InputText,
        offset: usize,
        has_other_words: bool,
    ) -> SudachiResult<Vec<Node>> {
        if has_other_words {
            return Ok(vec![]);
        }

        let length = input_text.get_word_candidate_length(offset);
        let surface = input_text.get_substring(offset, offset + length)?;

        Ok(vec![Node::new_oov(
            self.left_id,
            self.right_id,
            self.cost,
            WordInfo {
                normalized_form: surface.clone(),
                dictionary_form: surface.clone(),
                surface,
                head_word_length: length as u8,
                pos_id: self.oov_pos_id,
                dictionary_form_word_id: -1,
                ..Default::default()
            },
        )])
    }
}
