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

use crate::analysis::Node;
use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::dic::word_id::WordId;
use crate::input_text::InputBuffer;
use crate::plugin::oov::OovProviderPlugin;
use crate::prelude::*;

/// Provides a OOV node with single character if no words found in the dictionary
#[derive(Default)]
pub struct SimpleOovPlugin {
    left_id: u16,
    right_id: u16,
    cost: i16,
    oov_pos_id: u16,
}

/// Struct corresponds with raw config json file.
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
        let cost = settings.cost;

        self.oov_pos_id = oov_pos_id;
        self.left_id = settings.leftId as u16;
        self.right_id = settings.rightId as u16;
        self.cost = cost;

        if self.left_id as usize > grammar.conn_matrix().num_left() {
            return Err(SudachiError::InvalidDataFormat(
                self.left_id as usize,
                format!(
                    "max grammar left_id is {}",
                    grammar.conn_matrix().num_left()
                ),
            ));
        }

        if self.right_id as usize > grammar.conn_matrix().num_right() {
            return Err(SudachiError::InvalidDataFormat(
                self.right_id as usize,
                format!(
                    "max grammar left_id is {}",
                    grammar.conn_matrix().num_right()
                ),
            ));
        }

        Ok(())
    }

    fn provide_oov(
        &self,
        input_text: &InputBuffer,
        offset: usize,
        has_other_words: bool,
        result: &mut Vec<Node>,
    ) -> SudachiResult<()> {
        if has_other_words {
            return Ok(());
        }

        let length = input_text.get_word_candidate_length(offset);

        result.push(Node::new(
            offset as u16,
            (offset + length) as u16,
            self.left_id,
            self.right_id,
            self.cost,
            WordId::oov(self.oov_pos_id as u32),
        ));
        Ok(())
    }
}
