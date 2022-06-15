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

use crate::analysis::created::CreatedWords;
use serde::Deserialize;
use serde_json::Value;

use crate::analysis::Node;
use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::dic::word_id::WordId;
use crate::input_text::InputBuffer;
use crate::plugin::oov::OovProviderPlugin;
use crate::prelude::*;
use crate::util::check_params::CheckParams;
use crate::util::user_pos::{UserPosMode, UserPosSupport};

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
    leftId: i64,
    rightId: i64,
    cost: i64,
    #[serde(default)]
    userPOS: UserPosMode,
}

impl OovProviderPlugin for SimpleOovPlugin {
    fn set_up(
        &mut self,
        settings: &Value,
        _config: &Config,
        mut grammar: &mut Grammar,
    ) -> SudachiResult<()> {
        let settings: PluginSettings = serde_json::from_value(settings.clone())?;

        self.oov_pos_id = grammar.handle_user_pos(&settings.oovPOS, settings.userPOS)?;
        self.left_id = grammar.check_left_id(settings.leftId)?;
        self.right_id = grammar.check_right_id(settings.rightId)?;
        self.cost = grammar.check_cost(settings.cost)?;
        Ok(())
    }

    fn provide_oov(
        &self,
        input_text: &InputBuffer,
        offset: usize,
        other_words: CreatedWords,
        result: &mut Vec<Node>,
    ) -> SudachiResult<usize> {
        if other_words.not_empty() {
            return Ok(0);
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
        Ok(1)
    }
}
