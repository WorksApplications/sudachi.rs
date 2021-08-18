use serde::Deserialize;
use serde_json::Value;

use sudachi::config::Config;
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

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct PluginSettings {
    oovPOS: Vec<String>,
    leftId: i16,
    rightId: i16,
    cost: i16,
}

impl OovProviderPlugin for SimpleOovPlugin {
    fn set_up(
        &mut self,
        settings: &Value,
        _config: &Config,
        grammar: &Grammar,
    ) -> SudachiResult<()> {
        let settings: PluginSettings = serde_json::from_value(settings.clone())?;

        let oov_pos_string: Vec<&str> = settings.oovPOS.iter().map(|s| s.as_str()).collect();
        let oov_pos_id = grammar.get_part_of_speech_id(&oov_pos_string).ok_or(
            SudachiError::InvalidPartOfSpeech(format!("{:?}", oov_pos_string)),
        )?;
        let left_id = settings.leftId;
        let right_id = settings.rightId;
        let cost = settings.cost;

        self.oov_pos_id = oov_pos_id;
        self.left_id = left_id;
        self.right_id = right_id;
        self.cost = cost;

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
                head_word_length: length as u16,
                pos_id: self.oov_pos_id,
                dictionary_form_word_id: -1,
                ..Default::default()
            },
        )])
    }
}
