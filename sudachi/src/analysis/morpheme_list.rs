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
use crate::dic::grammar::Grammar;
// use crate::analysis::morpheme::Morpheme;
use crate::analysis::stateless_tokenizer::DictionaryAccess;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::prelude::*;

pub struct MorphemeList<'a, T> {
    pub dict: T,
    pub input_text: &'a str,
    pub path: Vec<Node>,
}

impl<'a, T> MorphemeList<'a, T>
where
    T: Deref,
    <T as Deref>::Target: DictionaryAccess,
{
    pub fn new(dict: T, input_text: &'a str, path: Vec<Node>) -> Self {
        Self {
            dict,
            input_text,
            path,
        }
    }

    pub fn iter(&self) -> MorphemeIter<T> {
        MorphemeIter {
            list: &self,
            index: 0,
        }
    }
}

impl<T> MorphemeList<'_, T>
where
    T: Deref,
    <T as Deref>::Target: DictionaryAccess,
{
    fn get_node(&self, index: usize) -> &Node {
        &self.path[index]
    }

    fn get_grammar(&self) -> &Grammar {
        self.dict.grammar()
    }

    fn get_begin(&self, index: usize) -> usize {
        self.path[index].begin
    }

    fn get_end(&self, index: usize) -> usize {
        self.path[index].end
    }

    fn get_surface(&self, index: usize) -> &String {
        // returns substring of the original text which corresponds to the node at the given index
        // we will need index mapping between original/modified input text
        todo!();
    }

    fn get_word_info(&self, index: usize) -> WordInfo {
        self.path[index].word_info.clone().unwrap()
    }

    fn split(&self, mode: Mode, index: usize, word_info: WordInfo) -> usize {
        // return type: MorphemeList<'_, T> {
        // Split target node based on the mode
        todo!();
    }

    fn is_oov(&self, index: usize) -> bool {
        self.path[index].is_oov
    }

    fn get_internal_cost(&self) -> i32 {
        if self.size() == 0 {
            return 0;
        }

        let first = &self.path[0];
        let last = &self.path.last().unwrap();
        last.total_cost - first.total_cost
    }

    fn size(&self) -> usize {
        self.path.len()
    }
}

pub struct MorphemeIter<'a, T> {
    list: &'a MorphemeList<'a, T>,
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
            word_info: None,
        };

        self.index += 1;
        Some(morpheme)
    }
}

pub struct Morpheme<'a, T> {
    list: &'a MorphemeList<'a, T>,
    index: usize,
    word_info: Option<crate::dic::lexicon::word_infos::WordInfo>,
}

impl<'a, T> Morpheme<'a, T>
where
    T: Deref,
    <T as Deref>::Target: DictionaryAccess,
{
    pub fn begin(&self) -> usize {
        self.list.get_begin(self.index)
    }

    pub fn end(&self) -> usize {
        self.list.get_end(self.index)
    }

    pub fn surface(&self) -> &String {
        &self.list.get_surface(self.index)
    }

    pub fn part_of_speech(&mut self) -> &Vec<String> {
        let pos_id = self.part_of_speech_id();
        &self.list.get_grammar().pos_list[pos_id as usize]
    }

    pub fn part_of_speech_id(&mut self) -> u16 {
        let wi = self.get_word_info();
        wi.pos_id
    }

    pub fn dictionary_form(&mut self) -> &String {
        let wi = self.get_word_info();
        &wi.dictionary_form
    }

    pub fn normalized_form(&mut self) -> &String {
        let wi = self.get_word_info();
        &wi.normalized_form
    }

    pub fn reading_form(&mut self) -> &String {
        let wi = self.get_word_info();
        &wi.reading_form
    }

    pub fn split(&mut self) -> usize {
        let wi = self.get_word_info();
        todo!();
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

    pub fn synonym_group_ids(&mut self) -> &Vec<u32> {
        let wi = self.get_word_info();
        &wi.synonym_group_ids
    }

    pub fn get_word_info(&mut self) -> &WordInfo {
        if let None = self.word_info {
            self.word_info = Some(self.list.get_word_info(self.index));
        }
        self.word_info.as_ref().unwrap()
    }
}
