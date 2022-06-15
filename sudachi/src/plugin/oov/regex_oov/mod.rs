/*
 *  Copyright (c) 2022 Works Applications Co., Ltd.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use crate::analysis::created::{CreatedWords, HasWord};
use crate::analysis::node::LatticeNode;
use crate::analysis::Node;
use crate::config::{Config, ConfigError};
use crate::dic::grammar::Grammar;
use crate::dic::word_id::WordId;
use crate::error::{SudachiError, SudachiResult};
use crate::input_text::{InputBuffer, InputTextIndex};
use crate::plugin::oov::OovProviderPlugin;
use crate::util::check_params::CheckParams;
use crate::util::user_pos::{UserPosMode, UserPosSupport};
use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use serde_json::Value;

#[cfg(test)]
mod test;

#[derive(Default)]
pub(crate) struct RegexOovProvider {
    regex: Option<Regex>,
    left_id: u16,
    right_id: u16,
    cost: i16,
    pos: u16,
    max_length: usize,
    debug: bool,
}

fn default_max_length() -> usize {
    32
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct RegexProviderConfig {
    #[serde(alias = "oovPOS")]
    pos: Vec<String>,
    leftId: i64,
    rightId: i64,
    cost: i64,
    regex: String,
    #[serde(default = "default_max_length")]
    maxLength: usize,
    #[serde(default)]
    debug: bool,
    #[serde(default)]
    userPOS: UserPosMode,
}

impl OovProviderPlugin for RegexOovProvider {
    fn set_up(
        &mut self,
        settings: &Value,
        _config: &Config,
        mut grammar: &mut Grammar,
    ) -> SudachiResult<()> {
        let mut parsed: RegexProviderConfig = serde_json::from_value(settings.clone())?;

        if !parsed.regex.starts_with("^") {
            parsed.regex.insert(0, '^');
        }

        self.left_id = grammar.check_left_id(parsed.leftId)?;
        self.right_id = grammar.check_right_id(parsed.rightId)?;
        self.cost = grammar.check_cost(parsed.cost)?;
        self.max_length = parsed.maxLength;
        self.debug = parsed.debug;
        self.pos = grammar.handle_user_pos(&parsed.pos, parsed.userPOS)?;

        match RegexBuilder::new(&parsed.regex).build() {
            Ok(re) => self.regex = Some(re),
            Err(e) => {
                return Err(SudachiError::ConfigError(ConfigError::InvalidFormat(
                    format!("regex {:?} is invalid: {:?}", &parsed.regex, e),
                )))
            }
        };

        Ok(())
    }

    fn provide_oov(
        &self,
        input_text: &InputBuffer,
        offset: usize,
        other_words: CreatedWords,
        result: &mut Vec<Node>,
    ) -> SudachiResult<usize> {
        if offset > 0 {
            // check that we have discontinuity in character categories
            let this_cat = input_text.cat_continuous_len(offset);
            let prev_cat = input_text.cat_continuous_len(offset - 1);
            if this_cat + 1 == prev_cat {
                // no discontinuity
                return Ok(0);
            }
        }

        let regex = self
            .regex
            .as_ref()
            .ok_or_else(|| SudachiError::InvalidDictionaryGrammar)?;

        let end = input_text
            .current_chars()
            .len()
            .min(offset + self.max_length);
        let text_data = input_text.curr_slice_c(offset..end);
        match regex.find(text_data) {
            None => Ok(0),
            Some(m) => {
                if m.start() != 0 {
                    return if self.debug {
                        Err(SudachiError::InvalidDataFormat(m.start(), format!("in input {:?} regex {:?} matched non-starting text in non-starting position: {}", text_data, regex, m.as_str())))
                    } else {
                        Ok(0)
                    };
                }

                let byte_offset = input_text.to_curr_byte_idx(offset);
                let match_start = offset;
                let match_end = input_text.ch_idx(byte_offset + m.end());

                let match_length = match_end - match_start;

                match other_words.has_word(match_length as i64) {
                    HasWord::Yes => return Ok(0),
                    HasWord::No => {} // do nothing
                    HasWord::Maybe => {
                        // need to check actual lengths for long words
                        for node in result.iter() {
                            if node.end() == match_end {
                                return Ok(0);
                            }
                        }
                    }
                }

                let node = Node::new(
                    match_start as _,
                    match_end as _,
                    self.left_id,
                    self.right_id,
                    self.cost,
                    WordId::oov(self.pos as _),
                );
                result.push(node);
                Ok(1)
            }
        }
    }
}
