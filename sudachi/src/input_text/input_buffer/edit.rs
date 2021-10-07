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

use std::ops::Range;

pub struct ReplaceOp<'a> {
    what: Range<usize>,
    with: ReplaceTgt<'a>,
}

enum ReplaceTgt<'a> {
    Ref(&'a str),
    Char(char),
    Str(String),
}

pub struct EditInput<'a> {
    replaces: &'a mut Vec<ReplaceOp<'a>>,
}

impl<'a> EditInput<'a> {
    pub(super) fn new(replaces: &'a mut Vec<ReplaceOp<'a>>) -> EditInput {
        EditInput { replaces }
    }

    /// Replace range with a &str
    pub fn replace_ref(&mut self, range: Range<usize>, result: &'a str) {
        let op = ReplaceOp {
            what: range,
            // SAFETY: Lifetime parameters enforce correct lifetime of this reference.
            // This reference can be created ONLY inside with_replacer call.
            // This reference lifetime will end before returning from with_replacer call.
            with: ReplaceTgt::Ref(unsafe { std::mem::transmute(result) }),
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
        let nchars = result.chars().count();
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

pub fn resolve_edits(
    source: &str,
    source_mapping: &Vec<usize>,
    target: &mut String,
    target_mapping: &mut Vec<usize>,
    edits: &mut Vec<ReplaceOp>,
) {
    let mut start: usize = 0;
    for edit in edits.drain(..) {
        target.push_str(&source[start..edit.what.start]);
        target_mapping.extend(source_mapping[start..edit.what.start].iter());
        start = edit.what.end;
        match edit.with {
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
        }
    }
    target.push_str(&source[start..]);
    target_mapping.extend(source_mapping[start..].iter());
}

fn add_replace(
    source_mapping: &Vec<usize>,
    target: &mut String,
    target_mapping: &mut Vec<usize>,
    what: Range<usize>,
    with: &str,
) {
    target.push_str(with);
    let old_mapping = &source_mapping[what.clone()];
    let old_len = what.len();
    let new_len = with.len();
    if new_len >= old_len {
        for i in 0..old_len {
            target_mapping.push(old_mapping[i]);
        }
        let last_value = source_mapping[what.end];
        for _ in old_len..new_len {
            target_mapping.push(last_value);
        }
    } else {
        for i in 0..new_len {
            target_mapping.push(old_mapping[i]);
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::InputBuffer;
    use super::*;
    use crate::input_text::InputTextIndex;

    #[test]
    fn edit_ref_1() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_replacer(|_, mut r| {
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
            .with_replacer(|_, mut r| {
                r.replace_char(3..6, '銀');
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "宇銀人");
        assert_eq!(buffer.orig_slice(3..6), "宙");
    }

    #[test]
    fn edit_str_1() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_replacer(|_, mut r| {
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
            .with_replacer(|_, mut r| {
                r.replace_ref(0..3, "銀河");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.m2o, &[0, 1, 2, 3, 3, 3, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(buffer.current(), "銀河宙人");
        assert_eq!(buffer.orig_slice(0..6), "宇");
        assert_eq!(buffer.orig_slice(0..3), "宇");
        assert_eq!(buffer.orig_slice(3..6), "");
    }

    #[test]
    fn replace_mid_w_longer() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_replacer(|_, mut r| {
                r.replace_ref(3..6, "銀河");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.m2o, &[0, 1, 2, 3, 4, 5, 6, 6, 6, 6, 7, 8, 9]);
        assert_eq!(buffer.current(), "宇銀河人");
        assert_eq!(buffer.orig_slice(3..9), "宙");
        assert_eq!(buffer.orig_slice(3..6), "宙");
        assert_eq!(buffer.orig_slice(6..9), "");
    }

    #[test]
    fn replace_end_w_longer() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_replacer(|_, mut r| {
                r.replace_ref(6..9, "銀河");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.m2o, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 9, 9]);
        assert_eq!(buffer.current(), "宇宙銀河");
        assert_eq!(buffer.orig_slice(6..12), "人");
        assert_eq!(buffer.orig_slice(6..9), "人");
        assert_eq!(buffer.orig_slice(9..12), "");
    }

    #[test]
    fn replace_end_w_shorter() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_replacer(|_, mut r| {
                r.replace_ref(3..9, "河");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "宇河");
        assert_eq!(buffer.m2o, &[0, 1, 2, 3, 4, 5, 9]);
        assert_eq!(buffer.orig_slice(0..3), "宇");
        assert_eq!(buffer.orig_slice(3..6), "宙人");
    }

    #[test]
    fn replace_start_w_shorter() {
        let mut buffer = InputBuffer::from("宇宙人");
        buffer
            .with_replacer(|_, mut r| {
                r.replace_ref(0..6, "河");
                Ok(r)
            })
            .expect("should not break");
        assert_eq!(0, buffer.replaces.len());
        assert_eq!(buffer.current(), "河人");
        assert_eq!(buffer.m2o, &[0, 1, 2, 6, 7, 8, 9]);
        assert_eq!(buffer.orig_slice(0..3), "宇宙");
        assert_eq!(buffer.orig_slice(3..6), "人");
    }
}
