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

use std::fmt;
use std::iter::FusedIterator;
use std::ops::Range;

use crate::analysis::inner::Node;
use crate::analysis::lattice::Lattice;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::subset::InfoSubset;
use crate::dic::word_id::WordId;
use crate::dic::word_info::{WordInfo, WordInfoResolver};
use crate::input_text::InputBuffer;
use crate::prelude::*;

/// Accessor trait for right connection id
pub trait RightId {
    fn right_id(&self) -> u16;
}

/// Accessor trait for the full path cost
pub trait PathCost {
    fn total_cost(&self) -> i32;

    #[inline]
    fn is_connected_to_bos(&self) -> bool {
        self.total_cost() != i32::MAX
    }
}

pub trait LatticeNode: RightId {
    fn begin(&self) -> usize;
    fn end(&self) -> usize;
    fn cost(&self) -> i16;
    fn word_id(&self) -> WordId;
    fn left_id(&self) -> u16;

    /// Is true when the word does not come from the dictionary.
    /// BOS and EOS are also treated as OOV.
    #[inline]
    fn is_oov(&self) -> bool {
        self.word_id().is_oov()
    }

    /// If a node is a special system node like BOS or EOS.
    /// Java name isSystem (which is similar to a regular node coming from the system dictionary)
    #[inline]
    fn is_special_node(&self) -> bool {
        self.word_id().is_special()
    }

    /// Returns number of codepoints in the current node
    #[inline]
    fn num_codepts(&self) -> usize {
        self.end() - self.begin()
    }

    /// Utility method for extracting [begin, end) codepoint range.
    #[inline]
    fn char_range(&self) -> Range<usize> {
        self.begin()..self.end()
    }
}

#[derive(Clone)]
/// Full lattice node, as the result of analysis.
/// All indices (including inner) are in the modified sentence space
/// Indices are converted to original sentence space when user request them.
pub struct ResultNode {
    inner: Node,
    total_cost: i32,
    begin_bytes: u16,
    end_bytes: u16,

    word_info: WordInfo,
}

impl ResultNode {
    pub fn new(
        inner: Node,
        total_cost: i32,
        begin_bytes: u16,
        end_bytes: u16,
        word_info: WordInfo,
    ) -> ResultNode {
        ResultNode {
            inner,
            total_cost,
            begin_bytes,
            end_bytes,
            word_info,
        }
    }
}

impl RightId for ResultNode {
    fn right_id(&self) -> u16 {
        self.inner.right_id()
    }
}

impl PathCost for ResultNode {
    fn total_cost(&self) -> i32 {
        self.total_cost
    }
}

impl LatticeNode for ResultNode {
    fn begin(&self) -> usize {
        self.inner.begin()
    }

    fn end(&self) -> usize {
        self.inner.end()
    }

    fn cost(&self) -> i16 {
        self.inner.cost()
    }

    fn word_id(&self) -> WordId {
        self.inner.word_id()
    }

    fn left_id(&self) -> u16 {
        self.inner.left_id()
    }
}

impl ResultNode {
    pub fn word_info(&self) -> &WordInfo {
        &self.word_info
    }

    /// Returns begin offset in bytes of node surface in a sentence
    pub fn begin_bytes(&self) -> usize {
        self.begin_bytes as usize
    }

    /// Returns end offset in bytes of node surface in a sentence
    pub fn end_bytes(&self) -> usize {
        self.end_bytes as usize
    }

    /// Returns range in bytes (for easy string slicing)
    pub fn bytes_range(&self) -> Range<usize> {
        self.begin_bytes()..self.end_bytes()
    }

    pub fn set_bytes_range(&mut self, begin: u16, end: u16) {
        self.begin_bytes = begin;
        self.end_bytes = end;
    }

    pub fn set_char_range(&mut self, begin: u16, end: u16) {
        self.inner.set_range(begin, end)
    }

    /// Returns number of splits in a specified mode
    pub fn num_splits(&self, mode: Mode) -> usize {
        match mode {
            Mode::A => self.word_info.a_unit_split().len(),
            Mode::B => self.word_info.b_unit_split().len(),
            Mode::C => 0,
        }
    }

    /// Split the node with a specified mode using the dictionary data
    pub fn split<'a>(
        &'a self,
        mode: Mode,
        lexicon_set: &'a LexiconSet<'a>,
        subset: InfoSubset,
        text: &'a InputBuffer,
    ) -> NodeSplitIterator<'a> {
        let splits: &[WordId] = match mode {
            Mode::A => self.word_info.a_unit_split(),
            Mode::B => self.word_info.b_unit_split(),
            Mode::C => panic!("splitting Node with Mode::C is not supported"),
        };

        NodeSplitIterator {
            splits,
            index: 0,
            lexicon_set,
            subset,
            text,
            byte_offset: self.begin_bytes,
            byte_end: self.end_bytes,
            char_offset: self.begin() as u16,
            char_end: self.end() as u16,
        }
    }
}

impl fmt::Display for ResultNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {} {}",
            self.begin(),
            self.end(),
            self.word_id(),
            self.word_info().pos_id(),
            self.left_id(),
            self.right_id(),
            self.cost()
        )
    }
}

pub struct NodeSplitIterator<'a> {
    splits: &'a [WordId],
    lexicon_set: &'a LexiconSet<'a>,
    index: usize,
    subset: InfoSubset,
    text: &'a InputBuffer,
    char_offset: u16,
    byte_offset: u16,
    char_end: u16,
    byte_end: u16,
}

impl<'a> Iterator for NodeSplitIterator<'a> {
    type Item = ResultNode;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.index;
        if idx >= self.splits.len() {
            return None;
        }

        let char_start = self.char_offset;
        let byte_start = self.byte_offset;

        let word_id = self.splits[idx];
        // data comes from dictionary, panicking here is OK
        let word_info = self
            .lexicon_set
            .get_word_info_subset(word_id, self.subset)
            .unwrap();

        let (char_end, byte_end) = if idx + 1 == self.splits.len() {
            (self.char_end, self.byte_end)
        } else {
            let byte_end = byte_start as usize + word_info.index_form_length();
            let char_end = self.text.ch_idx(byte_end);
            (char_end as u16, byte_end as u16)
        };

        self.char_offset = char_end;
        self.byte_offset = byte_end;

        let inner = Node::new(char_start, char_end, u16::MAX, u16::MAX, i16::MAX, word_id);

        let node = ResultNode::new(inner, i32::MAX, byte_start, byte_end, word_info);

        self.index += 1;
        Some(node)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.splits.len(), Some(self.splits.len()))
    }
}

impl FusedIterator for NodeSplitIterator<'_> {}

/// Concatenate the nodes in the range and replace normalized_form if given.
pub fn concat_nodes(
    mut path: Vec<ResultNode>,
    begin: usize,
    end: usize,
    normalized_form: Option<String>,
    resolver: &dyn WordInfoResolver,
) -> SudachiResult<Vec<ResultNode>> {
    if begin >= end {
        return Err(SudachiError::InvalidRange(begin, end));
    }

    let end_bytes = path[end - 1].end_bytes();
    let beg_bytes = path[begin].begin_bytes();

    let mut headword = String::with_capacity(end_bytes - beg_bytes);
    let mut reading_form = String::with_capacity(end_bytes - beg_bytes);
    let mut dictionary_form = String::with_capacity(end_bytes - beg_bytes);
    let mut index_form_length = 0;

    for node in path[begin..end].iter() {
        headword.push_str(node.word_info().headword(resolver));
        reading_form.push_str(node.word_info().reading_form(resolver));
        dictionary_form.push_str(node.word_info().dictionary_form(resolver));
        index_form_length += node.word_info().index_form_length();
    }

    let normalized_form = normalized_form.unwrap_or_else(|| {
        let mut norm = String::with_capacity(end_bytes - beg_bytes);
        for node in path[begin..end].iter() {
            norm.push_str(node.word_info().normalized_form(resolver));
        }
        norm
    });

    let pos_id = path[begin].word_info().pos_id() as i16;

    let wid = WordId::oov(pos_id as u32);
    let new_wi = WordInfo::new_with_strings(
        pos_id,
        index_form_length as i16,
        wid,
        headword,
        reading_form,
        normalized_form,
        dictionary_form,
    );

    let inner = Node::new(
        path[begin].begin() as u16,
        path[end - 1].end() as u16,
        u16::MAX,
        u16::MAX,
        i16::MAX,
        wid,
    );

    let node = ResultNode::new(
        inner,
        path[end - 1].total_cost,
        path[begin].begin_bytes,
        path[end - 1].end_bytes,
        new_wi,
    );

    path[begin] = node;
    path.drain(begin + 1..end);
    Ok(path)
}

/// Concatenate the nodes in the range and set pos_id.
pub fn concat_oov_nodes(
    mut path: Vec<ResultNode>,
    begin: usize,
    end: usize,
    pos_id: u16,
    text: &InputBuffer,
    lattice: &Lattice,
    resolver: &dyn WordInfoResolver,
) -> SudachiResult<Vec<ResultNode>> {
    if begin >= end {
        return Err(SudachiError::InvalidRange(begin, end));
    }

    let byte_begin = path[begin].begin_bytes;
    let byte_end = path[end - 1].end_bytes;
    let node_begin = path[begin].begin();
    let node_end = path[end - 1].end();

    // Use node in the path if exists.
    if let Some((existing_node, cost)) = lattice.get_minimum_node(node_begin, node_end) {
        if !existing_node.is_special_node() {
            let word_info = if existing_node.is_oov() {
                let surface = text
                    .curr_slice_c(existing_node.begin()..existing_node.end())
                    .to_owned();
                WordInfo::new_oov(
                    existing_node.word_id().entry().as_raw() as u16,
                    surface.len() as i16,
                    existing_node.word_id(),
                    surface,
                )
            } else {
                resolver
                    .lexicon()
                    .get_word_info_subset(existing_node.word_id(), InfoSubset::all())?
            };
            let node =
                ResultNode::new(existing_node.clone(), cost, byte_begin, byte_end, word_info);
            path[begin] = node;
            path.drain(begin + 1..end);
            return Ok(path);
        }
    }

    // concat nodes in the range to compose new oov node
    let capa = path[end - 1].end_bytes() - path[begin].begin_bytes();

    let mut headword = String::with_capacity(capa);
    let mut index_form_length = 0;
    for node in path[begin..end].iter() {
        headword.push_str(node.word_info().headword(resolver));
        index_form_length += node.word_info().index_form_length();
    }

    // Synthetic concatenation in Java's concatenateOov() is always marked as OOV
    // when we are not reusing an existing lattice node.
    let wid = WordId::oov(pos_id as u32);

    let new_wi = WordInfo::new_oov(pos_id, index_form_length as i16, wid, headword);

    let inner = Node::new(
        path[begin].begin() as u16,
        path[end - 1].end() as u16,
        u16::MAX,
        u16::MAX,
        i16::MAX,
        wid,
    );

    let node = ResultNode::new(
        inner,
        path[end - 1].total_cost,
        byte_begin,
        byte_end,
        new_wi,
    );

    path[begin] = node;
    path.drain(begin + 1..end);
    Ok(path)
}
