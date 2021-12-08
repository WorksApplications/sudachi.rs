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

use crate::analysis::morpheme::Morpheme;
use crate::analysis::node::{PathCost, ResultNode};
use crate::analysis::stateful_tokenizer::StatefulTokenizer;
use crate::analysis::stateless_tokenizer::DictionaryAccess;
use crate::analysis::Mode;
use crate::dic::subset::InfoSubset;
use crate::error::{SudachiError, SudachiResult};
use crate::input_text::InputBuffer;
use std::cell::{Ref, RefCell};
use std::iter::FusedIterator;
use std::ops::{Deref, DerefMut, Index};
use std::rc::Rc;

struct InputPart {
    input: InputBuffer,
    subset: InfoSubset,
}

impl Default for InputPart {
    fn default() -> Self {
        let mut input = InputBuffer::new();
        input.start_build().unwrap();
        Self {
            input,
            subset: Default::default(),
        }
    }
}

#[derive(Default)]
struct Nodes {
    data: Vec<ResultNode>,
}

impl Nodes {
    fn mut_data(&mut self) -> &mut Vec<ResultNode> {
        &mut self.data
    }
}

pub struct MorphemeList<T> {
    dict: T,
    input: Rc<RefCell<InputPart>>,
    nodes: Nodes,
}

impl<T: DictionaryAccess> MorphemeList<T> {
    /// Returns an empty morpheme list
    pub fn empty(dict: T) -> Self {
        let input = Default::default();
        Self {
            dict,
            input: Rc::new(RefCell::new(input)),
            nodes: Default::default(),
        }
    }

    /// Creates MorphemeList from components
    pub fn from_components(
        dict: T,
        input: InputBuffer,
        path: Vec<ResultNode>,
        subset: InfoSubset,
    ) -> Self {
        let input = InputPart { input, subset };
        Self {
            dict,
            input: Rc::new(RefCell::new(input)),
            nodes: Nodes { data: path },
        }
    }

    pub fn collect_results<U: DictionaryAccess>(
        &mut self,
        analyzer: &mut StatefulTokenizer<U>,
    ) -> SudachiResult<()> {
        match self.input.try_borrow_mut() {
            Ok(mut i) => {
                let mref = i.deref_mut();
                analyzer.swap_result(
                    &mut mref.input,
                    &mut self.nodes.mut_data(),
                    &mut mref.subset,
                );
                Ok(())
            }
            Err(_) => Err(SudachiError::MorphemeListBorrowed),
        }
    }

    /// Splits morphemes and writes them into the resulting list
    /// The resulting list is _not_ cleared before that
    /// Returns true if split produced more than two elements
    pub fn split_into(&self, mode: Mode, index: usize, out: &mut Self) -> SudachiResult<bool> {
        let node = self.node(index);
        let num_splits = node.num_splits(mode);

        if num_splits == 0 {
            Ok(false)
        } else {
            out.assign_input(self);
            let data = out.nodes.mut_data();
            let input = self.input();
            let subset = self.subset();
            data.reserve(num_splits);
            for n in node.split(mode, self.dict().lexicon(), subset, input.deref()) {
                data.push(n);
            }
            Ok(true)
        }
    }

    /// Clears morphemes from analysis result
    pub fn clear(&mut self) {
        self.nodes.mut_data().clear();
    }

    pub fn len(&self) -> usize {
        self.nodes.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.data.is_empty()
    }

    pub fn get(&self, idx: usize) -> Morpheme<T> {
        return Morpheme::for_list(self, idx);
    }

    pub fn surface(&self) -> Ref<str> {
        let inp = self.input();
        Ref::map(inp, |i| i.original())
    }

    pub fn iter(&self) -> MorphemeIter<T> {
        MorphemeIter {
            index: 0,
            list: self,
        }
    }

    /// Gets the whole cost of the path
    pub fn get_internal_cost(&self) -> i32 {
        let len = self.len();
        if len == 0 {
            return 0;
        }

        let first_node = self.node(0);
        let last_node = self.node(len - 1);
        last_node.total_cost() - first_node.total_cost()
    }

    pub(crate) fn node(&self, idx: usize) -> &ResultNode {
        self.nodes.data.index(idx)
    }

    pub fn dict(&self) -> &T {
        &self.dict
    }

    pub(crate) fn input(&self) -> Ref<InputBuffer> {
        Ref::map(self.input.deref().borrow(), |x| &x.input)
    }

    /// Makes this point to the input of another MorphemeList
    pub(crate) fn assign_input(&mut self, other: &Self) {
        if self.input.as_ptr() != other.input.as_ptr() {
            self.input = other.input.clone();
        }
    }

    pub fn subset(&self) -> InfoSubset {
        self.input.deref().borrow().subset
    }

    pub fn copy_slice(&self, start: usize, end: usize, out: &mut Self) {
        let out_data = out.nodes.mut_data();
        out_data.extend_from_slice(&self.nodes.data[start..end]);
    }
}

impl<T: DictionaryAccess + Clone> MorphemeList<T> {
    pub fn empty_clone(&self) -> Self {
        Self {
            dict: self.dict.clone(),
            input: self.input.clone(),
            nodes: Default::default(),
        }
    }

    /// Returns a new morpheme list splitting the morpheme with a given mode.
    /// Returns an empty list if there was no splits
    #[deprecated(note = "use split_into", since = "0.6.1")]
    pub fn split(&self, mode: Mode, index: usize) -> SudachiResult<MorphemeList<T>> {
        let mut list = self.empty_clone();
        if !self.split_into(mode, index, &mut list)? {
            list.nodes.mut_data().push(self.node(index).clone())
        }
        Ok(list)
    }
}

/// Iterates over morpheme list
pub struct MorphemeIter<'a, T> {
    list: &'a MorphemeList<T>,
    index: usize,
}

impl<'a, T: DictionaryAccess> Iterator for MorphemeIter<'a, T> {
    type Item = Morpheme<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.list.len() {
            return None;
        }

        let morpheme = Morpheme::for_list(self.list, self.index);

        self.index += 1;
        Some(morpheme)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.list.len() - self.index;
        (rem, Some(rem))
    }
}

impl<'a, T: DictionaryAccess> FusedIterator for MorphemeIter<'a, T> {}

impl<'a, T: DictionaryAccess> ExactSizeIterator for MorphemeIter<'a, T> {
    fn len(&self) -> usize {
        self.size_hint().0
    }
}
