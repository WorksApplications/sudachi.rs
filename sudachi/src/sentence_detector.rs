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

use fancy_regex::Regex;
use lazy_static::lazy_static;
use std::cmp::Ordering;

use crate::dic::lexicon_set::LexiconSet;
use crate::prelude::*;

/// A checker for words that cross boundaries
pub struct NonBreakChecker<'a> {
    lexicon: &'a LexiconSet<'a>,
    pub bos: usize,
}
impl<'a> NonBreakChecker<'a> {
    pub fn new(lexicon: &'a LexiconSet<'a>) -> Self {
        NonBreakChecker { lexicon, bos: 0 }
    }
}

impl NonBreakChecker<'_> {
    /// Returns whether there is a word that crosses the boundary

    fn has_non_break_word(&self, input: &str, length: usize) -> bool {
        // assume that SentenceDetector::get_eos called with self.input[self.bos..]
        let eos_byte = self.bos + length;
        let input_bytes = input.as_bytes();
        const LOOKUP_BYTE_LENGTH: usize = 10 * 3; // 10 Japanese characters in UTF-8
        let lookup_start = std::cmp::max(LOOKUP_BYTE_LENGTH, eos_byte) - LOOKUP_BYTE_LENGTH;
        for i in lookup_start..eos_byte {
            for entry in self.lexicon.lookup(input_bytes, i) {
                let end_byte = entry.end;
                // handling cases like モーニング娘。
                match end_byte.cmp(&eos_byte) {
                    // end is after than boundary candidate, this boundary is bad
                    Ordering::Greater => return true,
                    // end is on boundary candidate,
                    // check that there are more than one character in the matched word
                    Ordering::Equal => return input[i..].chars().take(2).count() > 1,
                    _ => {}
                }
            }
        }
        false
    }
}

const PERIODS: &str = "。？！♪…\\?\\!";
const DOT: &str = "\\.．";
const CDOTS: &str = "・{3,}";
const COMMA: &str = ",，、";
const BR_TAG: &str = "(<br>|<BR>){2,}";
const ALPHABET_OR_NUMBER: &str = "a-zA-Z0-9ａ-ｚＡ-Ｚ０-９〇一二三四五六七八九十百千万億兆";
const OPEN_PARENTHESIS: &str = "\\(\\{｛\\[（「【『［≪〔“";
const CLOSE_PARENTHESIS: &str = "\\)\\}\\]）」｝】』］〕≫”";

const DEFAULT_LIMIT: usize = 4096;

/// A sentence boundary detector
pub struct SentenceDetector {
    // The maximum number of characters processed at once
    limit: usize,
}

impl SentenceDetector {
    pub fn new() -> Self {
        SentenceDetector {
            limit: DEFAULT_LIMIT,
        }
    }
    pub fn with_limit(limit: usize) -> Self {
        SentenceDetector { limit }
    }

    /// Returns the byte index of the detected end of the sentence.
    ///
    /// If NonBreakChecker is given, it is used to determine if there is a
    /// word that crosses the detected boundary, and if so, the next boundary is
    /// returned.
    ///
    /// If there is no boundary, this returns a relatively harmles boundary as a
    /// negative value.
    ///
    /// # Examples
    ///
    /// ```
    /// let sd = sudachi::sentence_detector::SentenceDetector::new();
    /// assert_eq!(12, sd.get_eos("あいう。えお", None).unwrap());
    /// assert_eq!(-15, sd.get_eos("あいうえお", None).unwrap());
    /// ```
    pub fn get_eos(&self, input: &str, checker: Option<&NonBreakChecker>) -> SudachiResult<isize> {
        if input.is_empty() {
            return Ok(0);
        }

        // handle at most self.limit chars at once
        let s: String = input.chars().take(self.limit).collect();
        let input_exceeds_limit = s.len() < input.len();

        lazy_static! {
            static ref SENTENCE_BREAKER: Regex = Regex::new(&format!(
                "([{}]|{}+|(?<![{}])[{}](?![{}{}]))[{}{}]*|{}",
                PERIODS,
                CDOTS,
                ALPHABET_OR_NUMBER,
                DOT,
                ALPHABET_OR_NUMBER,
                COMMA,
                DOT,
                PERIODS,
                BR_TAG
            ))
            .unwrap();
            static ref ITEMIZE_HEADER: Regex =
                Regex::new(&format!("^([{}])([{}])$", ALPHABET_OR_NUMBER, DOT)).unwrap();
        }

        for mat in SENTENCE_BREAKER.find_iter(&s) {
            // check if we can split at the match
            let mut eos = mat?.end();
            if parenthesis_level(&s[..eos])? > 0 {
                continue;
            }
            if eos < s.len() {
                eos += prohibited_bos(&s[eos..])?;
            }
            if ITEMIZE_HEADER.is_match(&s)? {
                continue;
            }
            if eos < s.len() && is_continuous_phrase(&s, eos)? {
                continue;
            }
            if let Some(ck) = checker {
                if ck.has_non_break_word(input, eos) {
                    continue;
                }
            }
            return Ok(eos as isize);
        }

        if input_exceeds_limit {
            // search the final whitespace as a provisional split.
            lazy_static! {
                static ref SPACES: Regex = Regex::new(".+\\s+").unwrap();
            }
            if let Some(mat) = SPACES.find(&s)? {
                return Ok(-(mat.end() as isize));
            }
        }

        Ok(-(s.len() as isize))
    }
}

/// Returns the count of non-closed open parentheses remaining at the end of input.
fn parenthesis_level(s: &str) -> SudachiResult<usize> {
    lazy_static! {
        static ref PARENTHESIS: Regex = Regex::new(&format!(
            "([{}])|([{}])",
            OPEN_PARENTHESIS, CLOSE_PARENTHESIS
        ))
        .unwrap();
    }
    let mut level = 0;
    for caps in PARENTHESIS.captures_iter(s) {
        if let Some(_) = caps?.get(1) {
            // open
            level += 1;
        } else if level > 0 {
            level -= 1;
        }
    }
    Ok(level)
}

/// Returns a byte length of chars at the beggining of str, which cannot be a bos
fn prohibited_bos(s: &str) -> SudachiResult<usize> {
    lazy_static! {
        static ref PROHIBITED_BOS: Regex = Regex::new(&format!(
            "\\A([{}{}{}])+",
            CLOSE_PARENTHESIS, COMMA, PERIODS
        ))
        .unwrap();
    }

    if let Some(mat) = PROHIBITED_BOS.find(s)? {
        Ok(mat.end())
    } else {
        Ok(0)
    }
}

// Returns if eos is the middle of phrase
fn is_continuous_phrase(s: &str, eos: usize) -> SudachiResult<bool> {
    lazy_static! {
        static ref QUOTE_MARKER: Regex = Regex::new(&format!(
            "(！|？|\\!|\\?|[{}])(と|っ|です)",
            CLOSE_PARENTHESIS
        ))
        .unwrap();
        static ref EOS_ITEMIZE_HEADER: Regex =
            Regex::new(&format!("([{}])([{}])\\z", ALPHABET_OR_NUMBER, DOT)).unwrap();
    }

    // we can safely unwrap since eos > 0
    let last_char_len = s[..eos].chars().last().unwrap().to_string().len();
    if let Some(mat) = QUOTE_MARKER.find(&s[(eos - last_char_len)..])? {
        if mat.start() == 0 {
            return Ok(true);
        }
    }

    // we can safely unwrap since eos < s.len()
    let c = s[eos..].chars().nth(0).unwrap();
    Ok((c == 'と' || c == 'や' || c == 'の') && EOS_ITEMIZE_HEADER.is_match(&s[..eos])?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_eos() {
        let sd = SentenceDetector::new();
        assert_eq!(sd.get_eos("あいうえお。", None).unwrap(), 18);
        assert_eq!(sd.get_eos("あいう。えお。", None).unwrap(), 12);
        assert_eq!(sd.get_eos("あいう。。えお。", None).unwrap(), 15);
        assert_eq!(sd.get_eos("あいうえお", None).unwrap(), -15);
        assert_eq!(sd.get_eos("あいう えお。", None).unwrap(), 19);
        assert_eq!(sd.get_eos("あいう えお", None).unwrap(), -16);
        assert_eq!(sd.get_eos("", None).unwrap(), 0);
    }

    #[test]
    fn get_eos_with_limit() {
        let sd = SentenceDetector::with_limit(5);
        assert_eq!(sd.get_eos("あいうえおか。", None).unwrap(), -15);
        assert_eq!(sd.get_eos("あい。うえお。", None).unwrap(), 9);
        assert_eq!(sd.get_eos("あいうえ", None).unwrap(), -12);
        assert_eq!(sd.get_eos("あい うえお", None).unwrap(), -7);
        assert_eq!(sd.get_eos("あ い うえお", None).unwrap(), -8);
    }

    #[test]
    fn get_eos_with_period() {
        let sd = SentenceDetector::new();
        assert_eq!(sd.get_eos("あいう.えお", None).unwrap(), 10);
        assert_eq!(sd.get_eos("3.141", None).unwrap(), -5);
        assert_eq!(sd.get_eos("四百十.〇", None).unwrap(), -13);
    }

    #[test]
    fn get_eos_with_many_periods() {
        let sd = SentenceDetector::new();
        assert_eq!(sd.get_eos("あいうえお!??", None).unwrap(), 18);
    }

    #[test]
    fn get_eos_with_parentheses() {
        let sd = SentenceDetector::new();
        assert_eq!(sd.get_eos("あ（いう。え）お", None).unwrap(), -24);
        assert_eq!(sd.get_eos("（あ（いう）。え）お", None).unwrap(), -30);
        assert_eq!(sd.get_eos("あ（いう）。えお", None).unwrap(), 18);
    }

    #[test]
    fn get_eos_with_itemize_header() {
        let sd = SentenceDetector::new();
        assert_eq!(sd.get_eos("1. あいう。えお", None).unwrap(), 15);
    }

    #[test]
    fn get_eos_with_prohibited_bos() {
        let sd = SentenceDetector::new();
        assert_eq!(sd.get_eos("あいう?えお", None).unwrap(), 10);
        assert_eq!(sd.get_eos("あいう?)えお", None).unwrap(), 11);
        assert_eq!(sd.get_eos("あいう?,えお", None).unwrap(), 11);
    }

    #[test]
    fn get_eos_with_continuous_phrase() {
        let sd = SentenceDetector::new();
        assert_eq!(sd.get_eos("あいう?です。", None).unwrap(), 19);
        assert_eq!(sd.get_eos("あいう?って。", None).unwrap(), 19);
        assert_eq!(sd.get_eos("あいう?という。", None).unwrap(), 22);
        assert_eq!(sd.get_eos("あいう?の？です。", None).unwrap(), 10);

        assert_eq!(sd.get_eos("1.と2.が。", None).unwrap(), 13);
        assert_eq!(sd.get_eos("1.やb.から。", None).unwrap(), 16);
        assert_eq!(sd.get_eos("1.の12.が。", None).unwrap(), 14);
    }
}
