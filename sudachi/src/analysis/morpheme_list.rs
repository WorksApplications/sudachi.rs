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

use std::ops::Deref;

use crate::analysis::node::Node;
use crate::analysis::stateless_tokenizer::DictionaryAccess;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::input_text::Utf8InputText;
use crate::prelude::*;

pub struct MorphemeList<T> {
    pub dict: T,
    pub input_text: String,
    pub path: Vec<Node>,
}

impl<T> MorphemeList<T>
where
    T: Deref,
    <T as Deref>::Target: DictionaryAccess,
{
    pub fn new(dict: T, input_text: &Utf8InputText, mut path: Vec<Node>) -> SudachiResult<Self> {
        for node in &mut path {
            node.fill_word_info(dict.lexicon())?;
            // set range for origina text
            node.set_range(
                input_text.get_original_index(node.begin),
                input_text.get_original_index(node.end),
            )
        }

        Ok(Self {
            dict,
            input_text: input_text.original.to_string(),
            path,
        })
    }

    pub fn empty(dict: T) -> Self {
        Self {
            dict,
            input_text: String::new(),
            path: Vec::new(),
        }
    }

    fn get_grammar(&self) -> &Grammar {
        self.dict.grammar()
    }
}

impl<T> MorphemeList<T>
where
    T: Deref + Clone,
    <T as Deref>::Target: DictionaryAccess,
{
    pub fn split(&self, mode: Mode, index: usize) -> SudachiResult<MorphemeList<T>> {
        let input_text = self.input_text.clone();

        let word_ids = match mode {
            Mode::A => &self.get_word_info(index).a_unit_split,
            Mode::B => &self.get_word_info(index).b_unit_split,
            Mode::C => {
                return Ok(MorphemeList {
                    dict: self.dict.clone(),
                    input_text,
                    path: vec![self.path[index].clone()],
                })
            }
        };

        if word_ids.len() < 2 {
            return Ok(MorphemeList {
                dict: self.dict.clone(),
                input_text,
                path: vec![self.path[index].clone()],
            });
        }

        let mut offset = self.path[index].begin;
        let mut path = Vec::with_capacity(word_ids.len());
        for &wid in word_ids {
            let mut node = Node::new(0, 0, 0, wid);
            let word_info = self.dict.lexicon().get_word_info(wid)?;
            node.set_range(offset, offset + word_info.head_word_length as usize);
            offset += word_info.head_word_length as usize;
            node.set_word_info(word_info);
            path.push(node);
        }

        Ok(MorphemeList {
            dict: self.dict.clone(),
            input_text,
            path,
        })
    }
}

impl<T> MorphemeList<T> {
    pub fn iter(&self) -> MorphemeIter<T> {
        MorphemeIter {
            list: &self,
            index: 0,
        }
    }

    pub fn get_node(&self, index: usize) -> &Node {
        &self.path[index]
    }

    pub fn get_begin(&self, index: usize) -> usize {
        self.path[index].begin
    }

    pub fn get_end(&self, index: usize) -> usize {
        self.path[index].end
    }

    pub fn get_surface(&self, index: usize) -> &str {
        // returns substring of the original text which corresponds to the node at the given index
        let node = &self.path[index];
        &self.input_text[node.begin..node.end]
    }

    pub fn get_word_info(&self, index: usize) -> &WordInfo {
        self.path[index].word_info.as_ref().unwrap()
    }

    pub fn is_oov(&self, index: usize) -> bool {
        self.path[index].is_oov
    }

    pub fn get_internal_cost(&self) -> i32 {
        if self.len() == 0 {
            return 0;
        }

        let first = &self.path[0];
        let last = &self.path.last().unwrap();
        last.total_cost - first.total_cost
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }
}

pub struct MorphemeIter<'a, T> {
    list: &'a MorphemeList<T>,
    index: usize,
}

impl<'a, T> Iterator for MorphemeIter<'a, T>
where
    T: Deref,
    <T as Deref>::Target: DictionaryAccess,
{
    type Item = Morpheme<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let None = self.list.path.get(self.index) {
            return None;
        }

        let morpheme = Morpheme {
            list: self.list,
            index: self.index,
        };

        self.index += 1;
        Some(morpheme)
    }
}

pub struct Morpheme<'a, T> {
    list: &'a MorphemeList<T>,
    index: usize,
}

impl<'a, T> Morpheme<'a, T>
where
    T: Deref,
    <T as Deref>::Target: DictionaryAccess,
{
    pub fn part_of_speech(&self) -> SudachiResult<&[String]> {
        let pos_id = self.part_of_speech_id();
        let pos = self
            .list
            .get_grammar()
            .pos_list
            .get(pos_id as usize)
            .ok_or(SudachiError::MissingPartOfSpeech)?;
        Ok(&pos)
    }
}

impl<'a, T> Morpheme<'a, T>
where
    T: Deref + Clone,
    <T as Deref>::Target: DictionaryAccess,
{
    pub fn split(&self, mode: Mode) -> SudachiResult<MorphemeList<T>> {
        self.list.split(mode, self.index)
    }
}

impl<'a, T> Morpheme<'a, T> {
    pub fn begin(&self) -> usize {
        self.list.get_begin(self.index)
    }

    pub fn end(&self) -> usize {
        self.list.get_end(self.index)
    }

    pub fn surface(&self) -> &str {
        self.list.get_surface(self.index)
    }

    pub fn part_of_speech_id(&self) -> u16 {
        let wi = self.get_word_info();
        wi.pos_id
    }

    pub fn dictionary_form(&self) -> &str {
        let wi = self.get_word_info();
        &wi.dictionary_form
    }

    pub fn normalized_form(&self) -> &str {
        let wi = self.get_word_info();
        &wi.normalized_form
    }

    pub fn reading_form(&self) -> &str {
        let wi = self.get_word_info();
        &wi.reading_form
    }

    pub fn is_oov(&self) -> bool {
        self.list.is_oov(self.index)
    }

    pub fn word_id(&self) -> Option<u32> {
        self.list.get_node(self.index).word_id
    }

    pub fn dictionary_id(&self) -> i32 {
        self.list.get_node(self.index).get_dictionary_id()
    }

    pub fn synonym_group_ids(&self) -> &[u32] {
        let wi = self.get_word_info();
        &wi.synonym_group_ids
    }

    pub fn get_word_info(&self) -> &WordInfo {
        self.list.get_word_info(self.index)
    }
}
