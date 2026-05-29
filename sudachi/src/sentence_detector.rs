/*
 * Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use std::cmp::Ordering;
use std::sync::OnceLock;

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
        if eos_byte > input.len() || !input.is_char_boundary(eos_byte) {
            return false;
        }

        let input_bytes = input.as_bytes();
        const LOOKUP_BYTE_LENGTH: usize = 10 * 3; // 10 Japanese characters in UTF-8
        let mut lookup_start = eos_byte.saturating_sub(LOOKUP_BYTE_LENGTH);
        while lookup_start < eos_byte && !input.is_char_boundary(lookup_start) {
            lookup_start += 1;
        }

        for (relative, _) in input[lookup_start..eos_byte].char_indices() {
            let i = lookup_start + relative;
            if let Some(result) = self.lexicon.check_prefix_ends(input_bytes, i, |end_byte| {
                // handling cases like モーニング娘。
                match end_byte.cmp(&eos_byte) {
                    // end is after than boundary candidate, this boundary is bad
                    Ordering::Greater => Some(true),
                    // end is on boundary candidate,
                    // check that there are more than one character in the matched word
                    Ordering::Equal => Some(input[i..].chars().nth(1).is_some()),
                    _ => None,
                }
            }) {
                return result;
            }
        }
        false
    }
}

const DEFAULT_LIMIT: usize = 4096;

/// A sentence boundary detector
pub struct SentenceDetector {
    // The maximum number of characters processed at once
    limit: usize,
}

impl Default for SentenceDetector {
    fn default() -> Self {
        Self::new()
    }
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

        let (s, input_exceeds_limit) = limited_prefix(input, self.limit);

        if let Some(eos) = find_sentence_boundary(s, input, checker) {
            return Ok(eos as isize);
        }

        if input_exceeds_limit {
            // search the final whitespace as a provisional split.
            if let Some(end) = legacy_whitespace_end(s) {
                return Ok(-(end as isize));
            }
        }

        Ok(-(s.len() as isize))
    }
}

#[inline]
fn limited_prefix(input: &str, limit: usize) -> (&str, bool) {
    if input.len() <= limit {
        return (input, false);
    }

    match input.char_indices().nth(limit) {
        Some((idx, _)) => (&input[..idx], true),
        None => (input, false),
    }
}

fn find_sentence_boundary(
    limited: &str,
    original: &str,
    checker: Option<&NonBreakChecker>,
) -> Option<usize> {
    let mut index = 0;
    let mut previous = None;
    let mut parenthesis_level = 0usize;

    while index < limited.len() {
        let c = limited[index..]
            .chars()
            .next()
            .expect("valid char boundary");
        let next_index = index + c.len_utf8();

        if is_open_parenthesis(c) {
            parenthesis_level += 1;
            previous = Some(c);
            index = next_index;
            continue;
        }

        if is_close_parenthesis(c) {
            parenthesis_level = parenthesis_level.saturating_sub(1);
            previous = Some(c);
            index = next_index;
            continue;
        }

        if let Some(candidate_end) = sentence_candidate_end(limited, index, c, previous) {
            if parenthesis_level == 0 {
                let mut eos = candidate_end;
                if eos < limited.len() {
                    eos += prohibited_bos_len(&limited[eos..]);
                }

                if !is_itemize_header(limited) && !continues_phrase(limited, eos) {
                    if let Some(ck) = checker {
                        if ck.has_non_break_word(original, eos) {
                            previous = limited[..candidate_end].chars().next_back();
                            index = candidate_end;
                            continue;
                        }
                    }
                    return Some(eos);
                }
            }

            previous = limited[..candidate_end].chars().next_back();
            index = candidate_end;
            continue;
        }

        previous = Some(c);
        index = next_index;
    }

    None
}

fn sentence_candidate_end(s: &str, index: usize, c: char, previous: Option<char>) -> Option<usize> {
    if is_sentence_period(c) {
        return Some(consume_trailing_break_chars(s, index + c.len_utf8()));
    }

    if c == '・' {
        return cdots_candidate_end(s, index);
    }

    if is_dot(c) {
        let next_index = index + c.len_utf8();
        let previous_blocks = previous.map(is_alphabet_or_number).unwrap_or(false);
        let next_blocks = s[next_index..]
            .chars()
            .next()
            .map(|next| is_alphabet_or_number(next) || is_comma(next))
            .unwrap_or(false);

        if !previous_blocks && !next_blocks {
            return Some(consume_trailing_break_chars(s, next_index));
        }
    }

    if c == '<' {
        return br_sequence_end(s, index);
    }

    None
}

fn cdots_candidate_end(s: &str, index: usize) -> Option<usize> {
    let mut count = 0;
    let mut end = index;
    while let Some(c) = s[end..].chars().next() {
        if c != '・' {
            break;
        }
        count += 1;
        end += c.len_utf8();
    }

    if count >= 3 {
        Some(consume_trailing_break_chars(s, end))
    } else {
        None
    }
}

fn br_sequence_end(s: &str, index: usize) -> Option<usize> {
    let mut count = 0;
    let mut end = index;
    while let Some(len) = br_tag_len(&s[end..]) {
        count += 1;
        end += len;
    }

    if count >= 2 {
        Some(end)
    } else {
        None
    }
}

#[inline]
fn br_tag_len(s: &str) -> Option<usize> {
    if s.starts_with("<br>") || s.starts_with("<BR>") {
        Some(4)
    } else {
        None
    }
}

fn consume_trailing_break_chars(s: &str, mut index: usize) -> usize {
    while let Some(c) = s[index..].chars().next() {
        if !is_dot(c) && !is_sentence_period(c) {
            break;
        }
        index += c.len_utf8();
    }
    index
}

/// Returns a byte length of chars at the beginning of str, which cannot be a bos.
fn prohibited_bos_len(s: &str) -> usize {
    let mut end = 0;
    for (index, c) in s.char_indices() {
        if !is_close_parenthesis(c) && !is_comma(c) && !is_sentence_period(c) {
            break;
        }
        end = index + c.len_utf8();
    }
    end
}

fn continues_phrase(s: &str, eos: usize) -> bool {
    if eos >= s.len() {
        return false;
    }

    let last = s[..eos]
        .chars()
        .next_back()
        .expect("eos is after a boundary candidate");
    let rest = &s[eos..];
    if is_quote_marker(last)
        && (rest.starts_with("と") || rest.starts_with("っ") || rest.starts_with("です"))
    {
        return true;
    }

    let next = rest.chars().next().expect("eos is a valid char boundary");
    (next == 'と' || next == 'や' || next == 'の') && ends_with_itemize_header(&s[..eos])
}

fn is_itemize_header(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let Some(second) = chars.next() else {
        return false;
    };
    chars.next().is_none() && is_alphabet_or_number(first) && is_dot(second)
}

fn ends_with_itemize_header(s: &str) -> bool {
    let mut chars = s.chars().rev();
    let Some(last) = chars.next() else {
        return false;
    };
    let Some(previous) = chars.next() else {
        return false;
    };
    is_dot(last) && is_alphabet_or_number(previous)
}

fn legacy_whitespace_end(s: &str) -> Option<usize> {
    static SPACES: OnceLock<regex::Regex> = OnceLock::new();
    SPACES
        .get_or_init(|| regex::Regex::new(r".+\s+").unwrap())
        .find(s)
        .map(|mat| mat.end())
}

#[inline]
fn is_sentence_period(c: char) -> bool {
    matches!(c, '。' | '？' | '！' | '♪' | '…' | '?' | '!')
}

#[inline]
fn is_dot(c: char) -> bool {
    matches!(c, '.' | '．')
}

#[inline]
fn is_comma(c: char) -> bool {
    matches!(c, ',' | '，' | '、')
}

#[inline]
fn is_alphabet_or_number(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            'ａ'..='ｚ'
                | 'Ａ'..='Ｚ'
                | '０'..='９'
                | '〇'
                | '一'
                | '二'
                | '三'
                | '四'
                | '五'
                | '六'
                | '七'
                | '八'
                | '九'
                | '十'
                | '百'
                | '千'
                | '万'
                | '億'
                | '兆'
        )
}

#[inline]
fn is_open_parenthesis(c: char) -> bool {
    matches!(
        c,
        '(' | '{' | '｛' | '[' | '（' | '「' | '【' | '『' | '［' | '≪' | '〔' | '“' | '"'
    )
}

#[inline]
fn is_close_parenthesis(c: char) -> bool {
    matches!(
        c,
        ')' | '}' | ']' | '）' | '」' | '｝' | '】' | '』' | '］' | '〕' | '≫' | '”' | '"'
    )
}

#[inline]
fn is_quote_marker(c: char) -> bool {
    matches!(c, '！' | '？' | '!' | '?') || is_close_parenthesis(c)
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
    fn get_eos_with_multibyte_limit_boundary() {
        let sd = SentenceDetector::with_limit(4);
        assert_eq!(sd.get_eos("あいう。", None).unwrap(), 12);
        assert_eq!(sd.get_eos("あいうえ。", None).unwrap(), -12);
        assert_eq!(sd.get_eos("あい うえ。", None).unwrap(), -7);
    }

    #[test]
    fn get_eos_with_limit_multiline_whitespace_legacy_behavior() {
        let sd = SentenceDetector::with_limit(5);
        assert_eq!(sd.get_eos("a\n b c d", None).unwrap(), -3);
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
    fn get_eos_with_cdots() {
        let sd = SentenceDetector::new();
        assert_eq!(sd.get_eos("あ・・・い", None).unwrap(), 12);
        assert_eq!(sd.get_eos("あ・・い", None).unwrap(), -12);
        assert_eq!(sd.get_eos("あ・・・?!い", None).unwrap(), 14);
    }

    #[test]
    fn get_eos_with_br_tags() {
        let sd = SentenceDetector::new();
        assert_eq!(sd.get_eos("あ<br><br>い", None).unwrap(), 11);
        assert_eq!(sd.get_eos("あ<BR><BR>い", None).unwrap(), 11);
        assert_eq!(sd.get_eos("あ<br><BR>い", None).unwrap(), 11);
        assert_eq!(sd.get_eos("あ<br>い", None).unwrap(), -10);
    }

    #[test]
    fn get_eos_with_parentheses() {
        let sd = SentenceDetector::new();
        assert_eq!(sd.get_eos("あ（いう。え）お", None).unwrap(), -24);
        assert_eq!(sd.get_eos("（あ（いう）。え）お", None).unwrap(), -30);
        assert_eq!(sd.get_eos("あ（いう）。えお", None).unwrap(), 18);
    }

    #[test]
    fn get_eos_with_ascii_quote_legacy_behavior() {
        let sd = SentenceDetector::new();
        assert_eq!(sd.get_eos("\"あ。\"", None).unwrap(), -8);
        assert_eq!(sd.get_eos("あ。\"です。", None).unwrap(), -16);
        assert_eq!(sd.get_eos("あ。\")え。", None).unwrap(), 8);
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
