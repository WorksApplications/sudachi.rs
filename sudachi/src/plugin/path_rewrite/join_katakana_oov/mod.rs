/*
 * Copyright (c) 2021 Works Applications Co., Ltd.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use serde::Deserialize;
use serde_json::Value;

use crate::analysis::lattice::Lattice;
use crate::analysis::node::{concat_oov_nodes, LatticeNode, ResultNode};
use crate::config::Config;
use crate::dic::category_type::CategoryType;
use crate::dic::grammar::Grammar;
use crate::input_text::InputBuffer;
use crate::input_text::InputTextIndex;
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::prelude::*;

#[cfg(test)]
mod tests;

/// Concatenates katakana oov nodes into one
#[derive(Default)]
pub struct JoinKatakanaOovPlugin {
    /// The pos_id used for concatenated node
    oov_pos_id: u16,
    /// The minimum node char_length to concatenate even if it is not oov
    min_length: usize,
}

/// Struct corresponds with raw config json file.
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct PluginSettings {
    oovPOS: Vec<String>,
    minLength: usize,
}

impl JoinKatakanaOovPlugin {
    fn is_katakana_node<T: InputTextIndex>(&self, text: &T, node: &ResultNode) -> bool {
        text.cat_of_range(node.begin()..node.end())
            .contains(CategoryType::KATAKANA)
    }

    // fn is_one_char(&self, text: &Utf8InputText, node: &Node) -> bool {
    //     let b = node.begin;
    //     b + text.get_code_points_offset_length(b, 1) == node.end
    // }

    fn can_oov_bow_node<T: InputTextIndex>(&self, text: &T, node: &ResultNode) -> bool {
        !text
            .cat_at_char(node.begin())
            .contains(CategoryType::NOOOVBOW)
    }

    fn is_shorter(&self, node: &ResultNode) -> bool {
        node.num_codepts() < self.min_length
    }

    fn rewrite_gen<T: InputTextIndex>(
        &self,
        text: &T,
        mut path: Vec<ResultNode>,
        _lattice: &Lattice,
    ) -> SudachiResult<Vec<ResultNode>> {
        let mut i = 0;
        loop {
            if i >= path.len() {
                break;
            }

            let node = &path[i];
            if !(node.is_oov() || self.is_shorter(node)) || !self.is_katakana_node(text, node) {
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
                path = concat_oov_nodes(path, begin, end, self.oov_pos_id)?;
                // skip next node, as we already know it is not a joinable katakana
                i = begin + 1;
            }
            i += 1;
        }

        Ok(path)
    }
}

impl PathRewritePlugin for JoinKatakanaOovPlugin {
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
        text: &InputBuffer,
        path: Vec<ResultNode>,
        lattice: &Lattice,
    ) -> SudachiResult<Vec<ResultNode>> {
        self.rewrite_gen(text, path, lattice)
    }
}
