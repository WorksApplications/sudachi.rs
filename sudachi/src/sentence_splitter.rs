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

use crate::dic::lexicon_set::LexiconSet;
use crate::sentence_detector::{NonBreakChecker, SentenceDetector};
use std::ops::Range;

pub trait SplitSentences {
    fn split<'a, 'b>(&'b self, data: &'a str) -> SentenceIter<'a, 'b>;
}

pub struct SentenceIter<'s, 'x> {
    splitter: &'x SentenceDetector,
    checker: Option<&'x NonBreakChecker<'x>>,
    data: &'s str,
    position: usize,
}

impl<'s, 'x> Iterator for SentenceIter<'s, 'x> {
    type Item = (Range<usize>, &'s str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.position == self.data.len() {
            return None;
        }
        let slice = &self.data[self.position..];
        let rv = self.splitter.get_eos(slice, self.checker).unwrap();
        let end = if rv < 0 {
            self.data.len()
        } else {
            self.position + rv as usize
        };

        let range = self.position..end;
        let real_slice = &self.data[range.clone()];
        self.position = end;
        Some((range, real_slice))
    }
}

pub struct SentenceSplitter<'a> {
    detector: SentenceDetector,
    checker: Option<NonBreakChecker<'a>>,
}

impl SentenceSplitter<'_> {
    pub fn new() -> Self {
        SentenceSplitter {
            detector: SentenceDetector::new(),
            checker: None,
        }
    }

    pub fn with_limit(limit: usize) -> Self {
        SentenceSplitter {
            detector: SentenceDetector::with_limit(limit),
            checker: None,
        }
    }

    pub fn with_checker<'a>(self, lexicon: &'a LexiconSet<'a>) -> SentenceSplitter<'a> {
        let checker = NonBreakChecker::new(lexicon);
        SentenceSplitter {
            detector: self.detector,
            checker: Some(checker),
        }
    }
}

impl SplitSentences for SentenceSplitter<'_> {
    fn split<'a, 'b>(&'b self, data: &'a str) -> SentenceIter<'a, 'b> {
        SentenceIter {
            data,
            position: 0,
            splitter: &self.detector,
            checker: self.checker.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_simple() {
        let splitter = SentenceSplitter::new();
        let mut iter = splitter.split("テスト。テスト");
        assert_eq!(iter.next(), Some((0..12, "テスト。")));
        assert_eq!(iter.next(), Some((12..21, "テスト")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn split_longer_sentence() {
        let splitter = SentenceSplitter::new();
        let mut iter = splitter.split("　振り返って見ると白い物！　女が軒下で招いている。");
        assert_eq!(
            iter.next(),
            Some((0..39, "\u{3000}振り返って見ると白い物！"))
        );
        assert_eq!(
            iter.next(),
            Some((39..75, "\u{3000}女が軒下で招いている。"))
        );
        assert_eq!(iter.next(), None)
    }
}
