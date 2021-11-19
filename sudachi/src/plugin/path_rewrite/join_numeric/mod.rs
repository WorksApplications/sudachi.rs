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

use self::numeric_parser::NumericParser;
use crate::analysis::lattice::Lattice;
use crate::analysis::node::{concat_nodes, LatticeNode, ResultNode};
use crate::config::Config;
use crate::dic::category_type::CategoryType;
use crate::dic::grammar::Grammar;
use crate::input_text::InputBuffer;
use crate::input_text::InputTextIndex;
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::prelude::*;

mod numeric_parser;
#[cfg(test)]
mod test;

/// Concatenates numeric nodes as one
#[derive(Default)]
pub struct JoinNumericPlugin {
    /// The pos_id to concatenate
    numeric_pos_id: u16,
    /// Whether if to normalize the normalized_form
    enable_normalize: bool,
}

/// Struct corresponds with raw config json file.
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct PluginSettings {
    enableNormalize: Option<bool>,
}

impl JoinNumericPlugin {
    fn concat(
        &self,
        mut path: Vec<ResultNode>,
        begin: usize,
        end: usize,
        parser: &mut NumericParser,
    ) -> SudachiResult<Vec<ResultNode>> {
        let word_info = path[begin].word_info();

        if word_info.pos_id() != self.numeric_pos_id {
            return Ok(path);
        }

        if self.enable_normalize {
            let normalized_form = parser.get_normalized();
            if end - begin > 1 || normalized_form != word_info.normalized_form() {
                path = concat_nodes(path, begin, end, Some(normalized_form))?;
            }
            return Ok(path);
        }

        if end - begin > 1 {
            path = concat_nodes(path, begin, end, None)?;
        }
        Ok(path)
    }

    fn rewrite_gen<T: InputTextIndex>(
        &self,
        text: &T,
        mut path: Vec<ResultNode>,
    ) -> SudachiResult<Vec<ResultNode>> {
        let mut begin_idx = -1;
        let mut comma_as_digit = true;
        let mut period_as_digit = true;
        let mut parser = NumericParser::new();
        let mut i = -1;
        while i < path.len() as i32 - 1 {
            i += 1;
            let node = &path[i as usize];
            let ctypes = text.cat_of_range(node.char_range());
            let s = node.word_info().normalized_form();
            if ctypes.intersects(CategoryType::NUMERIC | CategoryType::KANJINUMERIC)
                || (comma_as_digit && s == ",")
                || (period_as_digit && s == ".")
            {
                if begin_idx < 0 {
                    parser.clear();
                    begin_idx = i;
                }
                for c in s.chars() {
                    if !parser.append(&c) {
                        if begin_idx >= 0 {
                            if parser.error_state == numeric_parser::Error::COMMA {
                                comma_as_digit = false;
                                i = begin_idx - 1;
                            } else if parser.error_state == numeric_parser::Error::POINT {
                                period_as_digit = false;
                                i = begin_idx - 1;
                            }
                            begin_idx = -1;
                        }
                        break;
                    }
                }
                continue;
            }

            let c = if s.len() == 1 {
                // must be 1 byte utf-8: ASCII
                s.as_bytes()[0] as char
            } else {
                char::MAX
            };

            // can't use s below this line

            if begin_idx >= 0 {
                if parser.done() {
                    path = self.concat(path, begin_idx as usize, i as usize, &mut parser)?;
                    i = begin_idx + 1;
                } else {
                    let ss = path[i as usize - 1].word_info().normalized_form();
                    if (parser.error_state == numeric_parser::Error::COMMA && ss == ",")
                        || (parser.error_state == numeric_parser::Error::POINT && ss == ".")
                    {
                        path =
                            self.concat(path, begin_idx as usize, i as usize - 1, &mut parser)?;
                        i = begin_idx + 2;
                    }
                }
            }
            begin_idx = -1;
            if !comma_as_digit && c != ',' {
                comma_as_digit = true;
            }
            if !period_as_digit && c != '.' {
                period_as_digit = true;
            }
        }

        // process last part
        if begin_idx >= 0 {
            let len = path.len();
            if parser.done() {
                path = self.concat(path, begin_idx as usize, len, &mut parser)?;
            } else {
                let ss = path[len - 1].word_info().normalized_form();
                if (parser.error_state == numeric_parser::Error::COMMA && ss == ",")
                    || (parser.error_state == numeric_parser::Error::POINT && ss == ".")
                {
                    path = self.concat(path, begin_idx as usize, len - 1, &mut parser)?;
                }
            }
        }

        Ok(path)
    }
}

impl PathRewritePlugin for JoinNumericPlugin {
    fn set_up(
        &mut self,
        settings: &Value,
        _config: &Config,
        grammar: &Grammar,
    ) -> SudachiResult<()> {
        let settings: PluginSettings = serde_json::from_value(settings.clone())?;

        // this pos is fixed
        let numeric_pos_string = vec!["名詞", "数詞", "*", "*", "*", "*"];
        let numeric_pos_id = grammar.get_part_of_speech_id(&numeric_pos_string).ok_or(
            SudachiError::InvalidPartOfSpeech(format!("{:?}", numeric_pos_string)),
        )?;
        let enable_normalize = settings.enableNormalize;

        self.numeric_pos_id = numeric_pos_id;
        self.enable_normalize = enable_normalize.unwrap_or(true);

        Ok(())
    }

    fn rewrite(
        &self,
        text: &InputBuffer,
        path: Vec<ResultNode>,
        _lattice: &Lattice,
    ) -> SudachiResult<Vec<ResultNode>> {
        self.rewrite_gen(text, path)
    }
}
