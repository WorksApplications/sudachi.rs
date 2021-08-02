use crate::dic::category_type::CategoryType;
use crate::dic::grammar::Grammar;
use crate::lattice::{node::Node, Lattice};
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::prelude::*;
use crate::utf8inputtext::Utf8InputText;

pub struct JoinKarakanaOovPlugin {
    oov_pos_id: u16,
    min_length: usize,
}

impl JoinKarakanaOovPlugin {
    pub fn new(grammar: &Grammar) -> SudachiResult<JoinKarakanaOovPlugin> {
        // todo: load from config
        let oov_pos_string = vec!["名詞", "普通名詞", "一般", "*", "*", "*"];
        let oov_pos_id = grammar
            .get_part_of_speech_id(&oov_pos_string)
            .ok_or(SudachiError::InvalidPartOfSpeech)?;

        let min_length = 3;

        Ok(JoinKarakanaOovPlugin {
            oov_pos_id,
            min_length,
        })
    }

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
