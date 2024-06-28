/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use std::borrow::{Borrow, Cow};
use std::fmt::{Debug, Formatter};

use indexmap::map::IndexMap;
use indexmap::Equivalent;

#[cfg(test)]
use crate::analysis::Mode;
#[cfg(test)]
use crate::dic::build::error::BuildFailure;
use crate::dic::build::error::DicCompilationCtx;
#[cfg(test)]
use crate::dic::build::MAX_POS_IDS;
use crate::dic::grammar::Grammar;
use crate::dic::pos::POS_DEPTH;
use crate::dic::word_info::WordInfos;

#[cfg(test)]
mod test;

mod entry;
mod layout;
mod parser;
mod refs;
mod resolution;
mod string_store;
mod writer;

pub(crate) use entry::{ParsedLexiconEntry, ResolvedLexiconEntry};
pub(crate) use refs::{ResolvedWordRef, WordRef, WordRefResolver};
pub use string_store::StringStore;
pub use writer::LexiconWriter;

#[derive(Hash, Eq, PartialEq)]
pub struct StrPosEntry {
    data: [Cow<'static, str>; POS_DEPTH],
}

impl<'a> Borrow<[Cow<'a, str>; POS_DEPTH]> for StrPosEntry {
    fn borrow(&self) -> &[Cow<'a, str>; POS_DEPTH] {
        &self.data
    }
}

impl<'a> Equivalent<[Cow<'a, str>; POS_DEPTH]> for StrPosEntry {
    fn equivalent(&self, key: &[Cow<'_, str>; POS_DEPTH]) -> bool {
        self.data.eq(key)
    }
}

impl StrPosEntry {
    fn rewrap(data: Cow<str>) -> Cow<'static, str> {
        match data {
            Cow::Borrowed(b) => Cow::Owned(b.to_owned()),
            Cow::Owned(s) => Cow::Owned(s),
        }
    }

    pub fn new(data: [Cow<str>; POS_DEPTH]) -> Self {
        let [d1, d2, d3, d4, d5, d6] = data;
        let owned: [Cow<'static, str>; POS_DEPTH] = [
            Self::rewrap(d1),
            Self::rewrap(d2),
            Self::rewrap(d3),
            Self::rewrap(d4),
            Self::rewrap(d5),
            Self::rewrap(d6),
        ];
        Self { data: owned }
    }

    pub fn from_built_pos(data: &[String]) -> Self {
        let mut iter = data.iter().map(|x| x.as_str());
        let p1 = Cow::Borrowed(iter.next().unwrap());
        let p2 = Cow::Borrowed(iter.next().unwrap());
        let p3 = Cow::Borrowed(iter.next().unwrap());
        let p4 = Cow::Borrowed(iter.next().unwrap());
        let p5 = Cow::Borrowed(iter.next().unwrap());
        let p6 = Cow::Borrowed(iter.next().unwrap());
        Self::new([p1, p2, p3, p4, p5, p6])
    }

    pub fn fields(&self) -> &[Cow<'static, str>; 6] {
        &self.data
    }
}

impl Debug for StrPosEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{},{},{},{}",
            self.data[0], self.data[1], self.data[2], self.data[3], self.data[4], self.data[5]
        )
    }
}

pub struct LexiconReader {
    pos: IndexMap<StrPosEntry, u16>,
    ctx: DicCompilationCtx,
    parsed_entries: Vec<ParsedLexiconEntry>,
    resolved_entries: Vec<ResolvedLexiconEntry>,
    start_pos: usize,
    max_left: i16,
    max_right: i16,
    max_system_entry_id: usize,
}

impl LexiconReader {
    pub(crate) const ENTRY_INITIAL_OFFSET: usize = 32;

    pub fn new() -> Self {
        Self {
            pos: IndexMap::new(),
            ctx: DicCompilationCtx::default(),
            parsed_entries: Vec::new(),
            resolved_entries: Vec::new(),
            start_pos: 0,
            max_left: i16::MAX,
            max_right: i16::MAX,
            max_system_entry_id: usize::MAX,
        }
    }

    pub(crate) fn entries(&self) -> &[ParsedLexiconEntry] {
        &self.parsed_entries
    }

    pub(crate) fn resolved_entries(&self) -> &[ResolvedLexiconEntry] {
        &self.resolved_entries
    }

    pub(crate) fn pos_obj(&self, pos_id: u16) -> Option<&StrPosEntry> {
        self.pos.get_index(pos_id as usize).map(|(k, v)| {
            assert_eq!(v, &pos_id);
            k
        })
    }

    pub fn set_max_conn_sizes(&mut self, left: i16, right: i16) {
        self.max_left = left;
        self.max_right = right;
    }

    pub fn preload_pos(&mut self, grammar: &Grammar) {
        assert_eq!(self.pos.len(), 0);
        for (i, pos) in grammar.pos_list.iter().enumerate() {
            let key = StrPosEntry::from_built_pos(pos);
            self.pos.insert(key, i as u16);
        }
        self.start_pos = self.pos.len();
    }

    pub fn set_max_system_entry_id(&mut self, max: usize) {
        self.max_system_entry_id = max;
    }

    pub(crate) fn next_entry_id(&self) -> u32 {
        let mut offset = Self::ENTRY_INITIAL_OFFSET;
        for e in &self.parsed_entries {
            offset += e.expected_entry_size();
        }
        (offset >> WordInfos::WORD_ID_ALIGNMENT_BITS) as u32
    }
}
