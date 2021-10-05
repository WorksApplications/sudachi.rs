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

use crate::dic::category_type::CategoryType;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon_set::LexiconSet;
use crate::error::{SudachiError, SudachiResult};
use crate::input_text::{Utf8InputText, Utf8InputTextBuilder};
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::sentence_detector::{NonBreakChecker, SentenceDetector};

use super::lattice::Lattice;
use super::morpheme::MorphemeList;
use super::node::Node;
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

impl<T> StatelessTokenizer<T>
where
    T: Deref,
    <T as Deref>::Target: DictionaryAccess,
{
    pub fn new(dict: T) -> StatelessTokenizer<T> {
        StatelessTokenizer { dict }
    }

    pub fn as_dict(&self) -> &<T as Deref>::Target {
        return Deref::deref(&self.dict);
    }
}

impl<T> Tokenize for StatelessTokenizer<T>
where
    T: Deref + Clone,
    <T as Deref>::Target: DictionaryAccess,
{
    type Dictionary = T;

    fn tokenize<'a>(
        &'a self,
        input: &'a str,
        mode: Mode,
        enable_debug: bool,
    ) -> SudachiResult<MorphemeList<Self::Dictionary>> {
        let dict = Deref::deref(&self.dict);
        let input = build_input_text(dict, input);
        let path = tokenize_input_text(dict, &input, mode, enable_debug)?;

        Ok(MorphemeList::new(self.dict.clone(), &input, path)?)
    }

    fn tokenize_sentences<'a>(
        &'a self,
        input: &'a str,
        mode: Mode,
        enable_debug: bool,
    ) -> SudachiResult<Vec<MorphemeList<Self::Dictionary>>> {
        if input.is_empty() {
            return Ok(vec![MorphemeList::empty(self.dict.clone())]);
        }

        let dict = Deref::deref(&self.dict);
        let input = build_input_text(dict, input);
        split_sentences(dict.lexicon(), &input)?
            .iter()
            .map(|s| {
                let path = tokenize_input_text(dict, s, mode, enable_debug)?;
                MorphemeList::new(self.dict.clone(), &s, path)
            })
            .collect()
    }
}

fn split_sentences<'a, 'b: 'a>(
    lexicon: &'a LexiconSet,
    input: &'b Utf8InputText<'a>,
) -> SudachiResult<Vec<Utf8InputText<'a>>> {
    let mut sentences = Vec::new();
    let mut checker = NonBreakChecker::new(lexicon, input);
    let detector = SentenceDetector::new();
    loop {
        let byte_length = detector
            .get_eos(&input.modified[checker.bos..], Some(&checker))?
            .abs() as usize; // detector mey return negative value
        if byte_length == 0 {
            break;
        }
        let mut eos = checker.bos + byte_length;
        if eos < input.modified.len() {
            eos = input.get_next_in_original(eos - 1);
        }
        sentences.push(input.slice(checker.bos, eos));
        checker.bos = eos;
    }
    Ok(sentences)
}

fn build_input_text<'b, T: DictionaryAccess + ?Sized>(
    dict: &T,
    input: &'b str,
) -> Utf8InputText<'b> {
    let mut builder = Utf8InputTextBuilder::new(input, dict.grammar());

    for plugin in dict.input_text_plugins() {
        plugin.rewrite(&mut builder);
    }
    builder.build()
}

fn tokenize_input_text<'a, T: DictionaryAccess + ?Sized>(
    dict: &'a T,
    input: &'a Utf8InputText,
    mode: Mode,
    enable_debug: bool,
) -> SudachiResult<Vec<Node>> {
    if enable_debug {
        println!("=== Input dump:\n{}", input.modified);
    }

    let lattice = build_lattice(dict, input)?;
    if enable_debug {
        println!("=== Lattice dump:");
        lattice.dump(dict.grammar(), dict.lexicon())?;
    };

    let mut path = lattice.get_best_path()?;
    // fill word_info to safely unwrap during path_rewrite and split_path
    for node in &mut path {
        node.fill_word_info(dict.lexicon())?;
    }
    if enable_debug {
        println!("=== Before Rewriting:");
        dump_path(&path);
    };

    for plugin in dict.path_rewrite_plugins() {
        path = plugin.rewrite(&input, path, &lattice)?;
    }
    let path = split_path(dict, path, mode)?;
    if enable_debug {
        println!("=== After Rewriting:");
        dump_path(&path);
        println!("===");
    };

    Ok(path)
}

fn build_lattice<'a, 'b, T: DictionaryAccess + ?Sized>(
    dict: &'a T,
    input: &'b Utf8InputText,
) -> SudachiResult<Lattice<'a>> {
    let input_bytes = input.modified.as_bytes();
    let mut lattice = Lattice::new(dict.grammar(), input_bytes.len());
    for (i, _) in input_bytes.iter().enumerate() {
        if !input.can_bow(i) || !lattice.has_previous_node(i) {
            continue;
        }

        let mut has_word = false;
        for e in dict.lexicon().lookup(&input_bytes, i) {
            if (e.end < input_bytes.len()) && !input.can_bow(e.end) {
                continue;
            }
            has_word = true;
            let (left_id, right_id, cost) = dict.lexicon().get_word_param(e.word_id)?;
            let node = Node::new(left_id, right_id, cost, e.word_id);
            lattice.insert(i, e.end, node)?;
        }

        // OOV
        if !input
            .get_char_category_types(i)
            .contains(CategoryType::NOOOVBOW)
        {
            for oov_provider in dict.oov_provider_plugins() {
                for node in oov_provider.get_oov(&input, i, has_word)? {
                    has_word = true;
                    lattice.insert(node.begin, node.end, node)?;
                }
            }
        }
        if !has_word {
            // use last oov_provider as default
            for node in dict
                .oov_provider_plugins()
                .last()
                .unwrap()
                .get_oov(&input, i, has_word)?
            {
                has_word = true;
                lattice.insert(node.begin, node.end, node)?;
            }
        }

        if !has_word {
            panic!("no morpheme found at {}", i);
        }
    }
    lattice.connect_eos_node()?;

    Ok(lattice)
}

pub(super) fn split_path<T: DictionaryAccess + ?Sized>(
    dict: &T,
    path: Vec<Node>,
    mode: Mode,
) -> SudachiResult<Vec<Node>> {
    if mode == Mode::C {
        return Ok(path);
    }

    let mut new_path = Vec::with_capacity(path.len());
    for node in path {
        let word_info = node
            .word_info
            .as_ref()
            .ok_or(SudachiError::MissingWordInfo)?;
        let word_ids = match mode {
            Mode::A => &word_info.a_unit_split,
            Mode::B => &word_info.b_unit_split,
            _ => unreachable!(),
        };

        if word_ids.len() <= 1 {
            new_path.push(node);
        } else {
            let mut offset = node.begin;
            for wid in word_ids {
                let mut n = Node::new(0, 0, 0, *wid);
                n.fill_word_info(dict.lexicon())?;
                let length = n
                    .word_info
                    .as_ref()
                    .ok_or(SudachiError::MissingWordInfo)?
                    .head_word_length as usize;
                n.set_range(offset, offset + length);
                new_path.push(n);

                offset += length;
            }
        }
    }

    new_path.shrink_to_fit();
    Ok(new_path)
}

pub(super) fn dump_path(path: &Vec<Node>) {
    for (i, node) in (&path).iter().enumerate() {
        println!("{}: {}", i, node);
    }
}
