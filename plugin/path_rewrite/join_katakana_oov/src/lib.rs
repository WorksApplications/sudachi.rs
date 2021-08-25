use serde::Deserialize;
use serde_json::Value;

use sudachi::config::Config;
use sudachi::declare_path_rewrite_plugin;
use sudachi::dic::category_type::CategoryType;
use sudachi::dic::grammar::Grammar;
use sudachi::input_text::utf8_input_text::Utf8InputText;
use sudachi::lattice::{node::Node, Lattice};
use sudachi::plugin::path_rewrite::PathRewritePlugin;
use sudachi::prelude::*;

#[cfg(test)]
mod tests;

declare_path_rewrite_plugin!(JoinKarakanaOovPlugin, JoinKarakanaOovPlugin::default);

#[derive(Default)]
pub struct JoinKarakanaOovPlugin {
    oov_pos_id: u16,
    min_length: usize,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct PluginSettings {
    oovPOS: Vec<String>,
    minLength: usize,
}

impl JoinKarakanaOovPlugin {
    fn is_katakana_node(&self, text: &Utf8InputText, node: &Node) -> bool {
        text.get_char_category_types_range(node.begin, node.end)
            .contains(&CategoryType::KATAKANA)
    }

    // fn is_one_char(&self, text: &Utf8InputText, node: &Node) -> bool {
    //     let b = node.begin;
    //     b + text.get_code_points_offset_length(b, 1) == node.end
    // }

    fn can_oov_bow_node(&self, text: &Utf8InputText, node: &Node) -> bool {
        !text
            .get_char_category_types(node.begin)
            .contains(&CategoryType::NOOOVBOW)
    }

    fn is_shorter(&self, length: usize, text: &Utf8InputText, node: &Node) -> bool {
        text.code_point_count(node.begin, node.end) < length
    }
}

impl PathRewritePlugin for JoinKarakanaOovPlugin {
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
        let min_length = settings.minLength;

        self.oov_pos_id = oov_pos_id;
        self.min_length = min_length;

        Ok(())
    }

    fn rewrite(
        &self,
        text: &Utf8InputText,
        mut path: Vec<Node>,
        _lattice: &Lattice,
    ) -> SudachiResult<Vec<Node>> {
        let mut i = 0;
        loop {
            if i >= path.len() {
                break;
            }

            let node = &path[i];
            if !(node.is_oov || self.is_shorter(self.min_length, text, node))
                || !self.is_katakana_node(text, node)
            {
                i += 1;
                continue;
            }
            let mut begin = i as i32 - 1;
            loop {
                if begin < 0 {
                    break;
                }
                if !self.is_katakana_node(text, &path[begin as usize]) {
                    begin += 1;
                    break;
                }
                begin -= 1;
            }
            let mut begin = if begin < 0 { 0 } else { begin as usize };
            let mut end = i + 1;
            loop {
                if end >= path.len() {
                    break;
                }
                if !self.is_katakana_node(text, &path[end]) {
                    break;
                }
                end += 1;
            }
            while begin != end && !self.can_oov_bow_node(text, &path[begin]) {
                begin += 1;
            }

            if (end - begin) > 1 {
                path = self.concatenate_oov(path, begin, end, self.oov_pos_id)?;
                // skip next node, as we already know it is not a joinable katakana
                i = begin + 1;
            }
            i += 1;
        }

        Ok(path)
    }
}
