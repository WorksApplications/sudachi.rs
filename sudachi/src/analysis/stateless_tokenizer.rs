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

use crate::analysis::node::ResultNode;
use crate::analysis::stateful_tokenizer::StatefulTokenizer;
use std::ops::Deref;

use crate::dic::grammar::Grammar;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::subset::InfoSubset;
use crate::error::SudachiResult;
use crate::input_text::InputBuffer;
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;

use super::mlist::MorphemeList;
use super::{Mode, Tokenize};

/// Provides access to dictionary data
pub trait DictionaryAccess {
    fn grammar(&self) -> &Grammar<'_>;
    fn lexicon(&self) -> &LexiconSet<'_>;
    fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin + Sync + Send>];
    fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin + Sync + Send>];
    fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin + Sync + Send>];
}

impl<T> DictionaryAccess for T
where
    T: Deref,
    <T as Deref>::Target: DictionaryAccess,
{
    fn grammar(&self) -> &Grammar<'_> {
        <T as Deref>::deref(self).grammar()
    }

    fn lexicon(&self) -> &LexiconSet<'_> {
        <T as Deref>::deref(self).lexicon()
    }

    fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin + Sync + Send>] {
        <T as Deref>::deref(self).input_text_plugins()
    }

    fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin + Sync + Send>] {
        <T as Deref>::deref(self).oov_provider_plugins()
    }

    fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin + Sync + Send>] {
        <T as Deref>::deref(self).path_rewrite_plugins()
    }
}

/// Implementation of a Tokenizer which does not have tokenization state.
///
/// This is a wrapper which is generic over dictionary pointers.
/// Usable where dictionary is a struct itself, &, &mut, Rc<.>, Arc<.>.
pub struct StatelessTokenizer<T> {
    dict: T,
}

impl<T: DictionaryAccess> StatelessTokenizer<T> {
    pub fn new(dict: T) -> StatelessTokenizer<T> {
        StatelessTokenizer { dict }
    }
}

impl<T> StatelessTokenizer<T>
where
    T: Deref,
    <T as Deref>::Target: DictionaryAccess,
{
    pub fn as_dict(&self) -> &<T as Deref>::Target {
        return Deref::deref(&self.dict);
    }
}

impl<T> Tokenize for StatelessTokenizer<T>
where
    T: DictionaryAccess + Clone,
{
    type Dictionary = T;

    fn tokenize<'a>(
        &'a self,
        input: &'a str,
        mode: Mode,
        enable_debug: bool,
    ) -> SudachiResult<MorphemeList<Self::Dictionary>> {
        let mut tok = StatefulTokenizer::create(self.dict.clone(), enable_debug, mode);
        tok.reset().push_str(input);
        tok.do_tokenize()?;
        tok.into_morpheme_list()
    }
}

pub(super) fn split_path<T: DictionaryAccess + ?Sized>(
    dict: &T,
    path: Vec<ResultNode>,
    mode: Mode,
    subset: InfoSubset,
    input: &InputBuffer,
) -> SudachiResult<Vec<ResultNode>> {
    if mode == Mode::C {
        return Ok(path);
    }

    let mut new_path = Vec::with_capacity(path.len() * 3 / 2);
    for node in path {
        let split_len = node.num_splits(mode);
        if split_len <= 1 {
            new_path.push(node);
        } else {
            new_path.extend(node.split(mode, dict.lexicon(), subset, input));
        }
    }

    Ok(new_path)
}

pub(super) fn dump_path(path: &Vec<ResultNode>) {
    for (i, node) in (&path).iter().enumerate() {
        println!("{}: {}", i, node);
    }
}
