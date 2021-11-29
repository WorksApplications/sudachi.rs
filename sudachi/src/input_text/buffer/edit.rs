/*
 *  Copyright (c) 2021 Works Applications Co., Ltd.
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

use crate::input_text::buffer::REALLY_MAX_LENGTH;
use std::ops::Range;

#[derive(Clone)]
pub struct ReplaceOp<'a> {
    what: Range<usize>,
    with: ReplaceTgt<'a>,
}

#[derive(Clone)]
enum ReplaceTgt<'a> {
    Ref(&'a str),
    Char(char),
    Str(String),
}

pub struct InputEditor<'a> {
    replaces: &'a mut Vec<ReplaceOp<'a>>,
}

impl<'a> InputEditor<'a> {
    pub(super) fn new(replaces: &'a mut Vec<ReplaceOp<'a>>) -> InputEditor {
        InputEditor { replaces }
    }

    /// Replace range with a &str
    pub fn replace_ref(&mut self, range: Range<usize>, result: &'a str) {
        let op = ReplaceOp {
            what: range,
            with: ReplaceTgt::Ref(result),
        };
        self.replaces.push(op);
    }

    /// Replace range with char
    pub fn replace_char(&mut self, range: Range<usize>, result: char) {
        let op = ReplaceOp {
            what: range,
            with: ReplaceTgt::Char(result),
        };
        self.replaces.push(op);
    }

    /// Replace range with owned String
    pub fn replace_own(&mut self, range: Range<usize>, result: String) {
        let op = ReplaceOp {
            what: range,
            with: ReplaceTgt::Str(result),
        };
        self.replaces.push(op);
    }

    /// Replace range with char, followed by chars from iterator
    pub fn replace_char_iter<It>(&mut self, range: Range<usize>, ch: char, mut rest: It)
    where
        It: Iterator<Item = char>,
    {
        match rest.next() {
            None => self.replace_char(range, ch),
            Some(ch2) => {
                let mut s = String::with_capacity(12); //4 japanese chars
                s.push(ch);
                s.push(ch2);
                s.extend(rest);
                self.replace_own(range, s)
            }
        }
    }
}

// Edits are assumed to be sorted (from start to end) and non-overlapping.
// This is not checked right now (may be we should check this in debug mode)
// Current plugin implementations satisfy this criteria.
pub fn resolve_edits(
    source: &str,
    source_mapping: &Vec<usize>,
    target: &mut String,
    target_mapping: &mut Vec<usize>,
    edits: &mut Vec<ReplaceOp>,
) -> usize {
    let mut start: usize = 0;
    let mut cur_len: isize = source.len() as isize;
    for edit in edits.drain(..) {
        target.push_str(&source[start..edit.what.start]);
        target_mapping.extend(source_mapping[start..edit.what.start].iter());
        start = edit.what.end;
        cur_len += match edit.with {
            ReplaceTgt::Str(s) => {
                add_replace(source_mapping, target, target_mapping, edit.what, &s)
            }
            ReplaceTgt::Ref(s) => add_replace(source_mapping, target, target_mapping, edit.what, s),
            ReplaceTgt::Char(c) => add_replace(
                source_mapping,
                target,
                target_mapping,
                edit.what,
                c.encode_utf8(&mut [0; 4]),
            ),
        };
        if cur_len > REALLY_MAX_LENGTH as isize {
            return cur_len as usize;
        }
    }
    target.push_str(&source[start..]);
    target_mapping.extend(source_mapping[start..].iter());
    // first byte of mapping MUST be 0
    if let Some(v) = target_mapping.first_mut() {
        *v = 0;
    }
    cur_len as usize
}

fn add_replace(
    source_mapping: &Vec<usize>,
    target: &mut String,
    target_mapping: &mut Vec<usize>,
    what: Range<usize>,
    with: &str,
) -> isize {
    if with.is_empty() {
        return -(what.len() as isize);
    }
    target.push_str(with);

    // the first char of replacing string will correspond with whole replaced string
    target_mapping.push(source_mapping[what.start]);
    let pos = source_mapping[what.end];
    for _ in 1..with.len() {
        target_mapping.push(pos);
    }
    with.len() as isize - what.len() as isize
}

#[cfg(test)]
mod test {
    use super::super::InputBuffer;
    use crate::input_text::InputTextIndex;

    #[test]
    fn edit_ref_1() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(3..6, "銀");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "宇銀人");
        assert_eq!(buffer.orig_slice(3..6), "宙");
    }

    #[test]
    fn edit_char_1() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_char(3..6, '銀');
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "宇銀人");
        assert_eq!(buffer.orig_slice(3..6), "宙");
    }

    #[test]
    fn edit_ref_2_borrow() {
        let s = String::from("銀");
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(3..6, &s);
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "宇銀人");
    }

    #[test]
    fn edit_str_1() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_own(3..6, String::from("銀"));
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "宇銀人");
        assert_eq!(buffer.orig_slice(3..6), "宙");
    }

    #[test]
    fn replace_start_w_longer() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(0..3, "銀河");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.m2o, &[0, 3, 3, 3, 3, 3, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(buffer.current(), "銀河宙人");
        assert_eq!(buffer.orig_slice(0..6), "宇");
        assert_eq!(buffer.orig_slice(0..3), "宇");
        assert_eq!(buffer.orig_slice(3..6), "");
    }

    #[test]
    fn replace_mid_w_longer() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(3..6, "銀河");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.m2o, &[0, 1, 2, 3, 6, 6, 6, 6, 6, 6, 7, 8, 9]);
        assert_eq!(buffer.current(), "宇銀河人");
        assert_eq!(buffer.orig_slice(3..9), "宙");
        assert_eq!(buffer.orig_slice(3..6), "宙");
        assert_eq!(buffer.orig_slice(6..9), "");
    }

    #[test]
    fn replace_end_w_longer() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(6..9, "銀河");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.m2o, &[0, 1, 2, 3, 4, 5, 6, 9, 9, 9, 9, 9, 9]);
        assert_eq!(buffer.current(), "宇宙銀河");
        assert_eq!(buffer.orig_slice(6..12), "人");
        assert_eq!(buffer.orig_slice(6..9), "人");
        assert_eq!(buffer.orig_slice(9..12), "");
    }

    #[test]
    fn replace_start_w_shorter() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(0..6, "河");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "河人");
        assert_eq!(buffer.m2o, &[0, 6, 6, 6, 7, 8, 9]);
        assert_eq!(buffer.orig_slice(0..3), "宇宙");
        assert_eq!(buffer.orig_slice(3..6), "人");
    }

    #[test]
    fn replace_end_w_shorter() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(3..9, "河");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "宇河");
        assert_eq!(buffer.m2o, &[0, 1, 2, 3, 9, 9, 9]);
        assert_eq!(buffer.orig_slice(0..3), "宇");
        assert_eq!(buffer.orig_slice(3..6), "宙人");
    }

    #[test]
    fn replace_start_w_none() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(0..6, "");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "人");
        assert_eq!(buffer.m2o, &[0, 7, 8, 9]);
        assert_eq!(buffer.orig_slice(0..3), "宇宙人");
    }

    #[test]
    fn replace_start_w_none_2() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(0..3, "");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "宙人");
        assert_eq!(buffer.m2o, &[0, 4, 5, 6, 7, 8, 9]);
        assert_eq!(buffer.orig_slice(0..3), "宇宙");
        assert_eq!(buffer.orig_slice(3..6), "人");
    }

    #[test]
    fn replace_end_w_none() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(3..9, "");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "宇");
        assert_eq!(buffer.m2o, &[0, 1, 2, 9]);
        assert_eq!(buffer.orig_slice(0..3), "宇宙人");
    }

    #[test]
    fn replace_diff_width() {
        let mut buffer = InputBuffer::from("âｂC1あ");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(0..2, "a");
                r.replace_ref(2..5, "b");
                r.replace_ref(5..6, "c");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "abc1あ");
        assert_eq!(buffer.m2o, &[0, 2, 5, 6, 7, 8, 9, 10]);
        assert_eq!(buffer.orig_slice(0..3), "âｂC");
        assert_eq!(buffer.orig_slice(0..1), "â");
        assert_eq!(buffer.orig_slice(1..2), "ｂ");
        assert_eq!(buffer.orig_slice(2..3), "C");
    }

    #[test]
    fn replace_with_more_cnt() {
        let mut buffer = InputBuffer::from("あ");
        buffer
            .with_editor(|_, mut r| {
                r.replace_ref(0..3, "abc");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "abc");
        assert_eq!(buffer.m2o, &[0, 3, 3, 3]);
        assert_eq!(buffer.orig_slice(0..3), "あ");
        assert_eq!(buffer.orig_slice(0..1), "あ");
        assert_eq!(buffer.orig_slice(1..2), "");
        assert_eq!(buffer.orig_slice(2..3), "");
    }
}
