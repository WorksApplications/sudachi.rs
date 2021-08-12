pub mod numeric_parser;

use crate::dic::category_type::CategoryType;
use crate::dic::grammar::Grammar;
use crate::input_text::utf8_input_text::Utf8InputText;
use crate::lattice::{node::Node, Lattice};
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::prelude::*;
use numeric_parser::NumericParser;

pub struct JoinNumericPlugin {
    numeric_pos_id: u16,
    enable_normalize: bool,
}

impl JoinNumericPlugin {
    pub fn new(grammar: &Grammar) -> SudachiResult<JoinNumericPlugin> {
        // todo: load from config
        let numeric_pos_string = vec!["名詞", "数詞", "*", "*", "*", "*"];
        let numeric_pos_id = grammar
            .get_part_of_speech_id(&numeric_pos_string)
            .ok_or(SudachiError::InvalidPartOfSpeech)?;

        let enable_normalize = true;

        Ok(JoinNumericPlugin {
            numeric_pos_id,
            enable_normalize,
        })
    }

    fn concat(
        &self,
        mut path: Vec<Node>,
        begin: usize,
        end: usize,
        parser: &mut NumericParser,
    ) -> SudachiResult<Vec<Node>> {
        let word_info = path[begin]
            .word_info
            .clone()
            .ok_or(SudachiError::MissingWordInfo)?;

        if word_info.pos_id != self.numeric_pos_id {
            return Ok(path);
        }

        if self.enable_normalize {
            let normalized_form = parser.get_normalized();
            if end - begin > 1 || normalized_form != word_info.normalized_form {
                path = self.concatenate(path, begin, end, Some(normalized_form))?;
            }
            return Ok(path);
        }

        if end - begin > 1 {
            path = self.concatenate(path, begin, end, None)?;
        }
        Ok(path)
    }
}

impl PathRewritePlugin for JoinNumericPlugin {
    fn rewrite(
        &self,
        text: &Utf8InputText,
        mut path: Vec<Node>,
        _lattice: &Lattice,
    ) -> SudachiResult<Vec<Node>> {
        let mut begin_idx = -1;
        let mut comma_as_digit = true;
        let mut period_as_digit = true;
        let mut parser = NumericParser::new();

        let mut i = -1;
        while i < path.len() as i32 - 1 {
            i += 1;
            let node = &path[i as usize];
            let ctypes = text.get_char_category_types_range(node.begin, node.end);
            let s = node
                .word_info
                .clone()
                .ok_or(SudachiError::MissingWordInfo)?
                .normalized_form;
            if ctypes.contains(&CategoryType::NUMERIC)
                || ctypes.contains(&CategoryType::KANJINUMERIC)
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

            if begin_idx >= 0 {
                if parser.done() {
                    path = self.concat(path, begin_idx as usize, i as usize, &mut parser)?;
                    i = begin_idx + 1;
                } else {
                    let ss = path[i as usize - 1]
                        .word_info
                        .clone()
                        .ok_or(SudachiError::MissingWordInfo)?
                        .normalized_form;
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
            if !comma_as_digit && s != "," {
                comma_as_digit = true;
            }
            if !period_as_digit && s != "." {
                period_as_digit = true;
            }
        }

        // process last part
        if begin_idx >= 0 {
            let len = path.len();
            if parser.done() {
                path = self.concat(path, begin_idx as usize, len, &mut parser)?;
            } else {
                let ss = path[len - 1]
                    .word_info
                    .clone()
                    .ok_or(SudachiError::MissingWordInfo)?
                    .normalized_form;
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
