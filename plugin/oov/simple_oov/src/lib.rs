use sudachi::declare_oov_provider_plugin;
use sudachi::dic::grammar::Grammar;
use sudachi::dic::lexicon::word_infos::WordInfo;
use sudachi::input_text::utf8_input_text::Utf8InputText;
use sudachi::lattice::node::Node;
use sudachi::plugin::oov::OovProviderPlugin;
use sudachi::prelude::*;

declare_oov_provider_plugin!(SimpleOovPlugin, SimpleOovPlugin::default);

#[derive(Default)]
pub struct SimpleOovPlugin {
    left_id: i16,
    right_id: i16,
    cost: i16,
    oov_pos_id: u16,
}

impl OovProviderPlugin for SimpleOovPlugin {
    fn set_up(&mut self, grammar: &Grammar) -> SudachiResult<()> {
        // todo: load from file
        let left_id = 5968;
        let right_id = 5968;
        let cost = 3857;
        let oov_pos_string = vec!["補助記号", "一般", "*", "*", "*", "*"];

        let oov_pos_id = grammar
            .get_part_of_speech_id(&oov_pos_string)
            .ok_or(SudachiError::InvalidPartOfSpeech)?;

        self.left_id = left_id;
        self.right_id = right_id;
        self.cost = cost;
        self.oov_pos_id = oov_pos_id;

        Ok(())
    }

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
