/*
 * Copyright (c) 2021 Works Applications Co., Ltd.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use crate::config::Config;
use crate::dic::category_type::CategoryType;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::{DictionaryLoader, LoadedDictionary};
use crate::input_text::{Utf8InputText, Utf8InputTextBuilder};
use crate::lattice::node::Node;
use crate::lattice::Lattice;
use crate::morpheme::Morpheme;
use crate::plugin::{connect_cost, PluginProvider};
use crate::plugin::input_text::{self, InputTextPluginManager, InputTextPlugin};
use crate::plugin::oov::{self, OovProviderPluginManager, OovProviderPlugin};
use crate::plugin::path_rewrite::{self, PathRewritePluginManager, PathRewritePlugin};
use crate::prelude::*;
use crate::sentence_detector::{NonBreakChecker, SentenceDetector};

/// Able to tokenize Japanese text
pub trait Tokenize {
    /// Break text into `Morpheme`s
    fn tokenize(&self, input: &str, mode: Mode, enable_debug: bool)
        -> SudachiResult<Vec<Morpheme>>;

    /// Split text into sentences then tokenize
    fn tokenize_sentences(
        &self,
        input: &str,
        mode: Mode,
        enable_debug: bool,
    ) -> SudachiResult<Vec<Vec<Morpheme>>>;
}

/// Tokenizes Japanese text
pub struct Tokenizer<'a> {
    pub grammar: Grammar<'a>,
    pub lexicon: LexiconSet<'a>,
    input_text_plugins: InputTextPluginManager,
    oov_provider_plugins: OovProviderPluginManager,
    path_rewrite_plugins: PathRewritePluginManager,
}

/// Unit to split text
///
/// Some examples:
/// ```text
/// A：選挙/管理/委員/会
/// B：選挙/管理/委員会
/// C：選挙管理委員会
///
/// A：客室/乗務/員
/// B：客室/乗務員
/// C：客室乗務員
///
/// A：労働/者/協同/組合
/// B：労働者/協同/組合
/// C：労働者協同組合
///
/// A：機能/性/食品
/// B：機能性/食品
/// C：機能性食品
/// ```
///
/// See [Sudachi documentation](https://github.com/WorksApplications/Sudachi#the-modes-of-splitting)
/// for more details
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Short
    A,

    /// Middle (similar to "word")
    B,

    /// Named Entity
    C,
}

impl FromStr for Mode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A" | "a" => Ok(Mode::A),
            "B" | "b" => Ok(Mode::B),
            "C" | "c" => Ok(Mode::C),
            _ => Err("Mode must be one of \"A\", \"B\", or \"C\" (in lower or upper case)."),
        }
    }
}

/// Return bytes of a `dictionary_path`
pub fn dictionary_bytes_from_path<P: AsRef<Path>>(dictionary_path: P) -> SudachiResult<Vec<u8>> {
    let dictionary_path = dictionary_path.as_ref();
    let dictionary_stat = fs::metadata(&dictionary_path)?;
    let mut dictionary_file = File::open(dictionary_path)?;
    let mut dictionary_bytes = Vec::with_capacity(dictionary_stat.len() as usize);
    dictionary_file.read_to_end(&mut dictionary_bytes)?;

    Ok(dictionary_bytes)
}

impl<'a> Tokenizer<'a> {
    /// Create `Tokenizer` from the raw bytes of a Sudachi dictionary.
    pub fn from_dictionary_bytes(
        dictionary_bytes: &'a [u8],
        user_dictionary_bytes: &'a Vec<Box<[u8]>>,
        config: &Config,
    ) -> SudachiResult<Tokenizer<'a>> {
        let dictionary = LoadedDictionary::from_system_dictionary(
            dictionary_bytes,
            &config.character_definition_file,
        )?;
        let mut grammar = dictionary.grammar;
        let lexicon = dictionary.lexicon_set;

        let edit_connection_cost_plugins =
            connect_cost::get_edit_connection_cost_plugins(&config, &grammar)?;
        for plugin in edit_connection_cost_plugins.plugins() {
            plugin.edit(&mut grammar);
        }

        let input_text_plugins = input_text::get_input_text_plugins(config, &grammar)?;
        let oov_provider_plugins = oov::get_oov_plugins(config, &grammar)?;
        if oov_provider_plugins.is_empty() {
            return Err(SudachiError::NoOOVPluginProvided);
        }
        let path_rewrite_plugins = path_rewrite::get_path_rewrite_plugins(config, &grammar)?;

        let mut tokenizer = Tokenizer {
            grammar,
            lexicon,
            input_text_plugins,
            oov_provider_plugins,
            path_rewrite_plugins,
        };

        for user_dict in user_dictionary_bytes {
            tokenizer.merge_user_dictionary(user_dict)?;
        }

        Ok(tokenizer)
    }

    fn merge_user_dictionary(&mut self, dictionary_bytes: &'a [u8]) -> SudachiResult<()> {
        let user_dict = DictionaryLoader::read_user_dictionary(dictionary_bytes)?;

        // we need to update lexicon first, since it needs the current number of pos
        let mut user_lexicon = user_dict.lexicon;
        user_lexicon.update_cost(self)?;
        self.lexicon
            .append(user_lexicon, self.grammar.pos_list.len())?;

        if let Some(g) = user_dict.grammar {
            self.grammar.merge(g);
        }

        Ok(())
    }

    fn build_input_text(&'a self, input: &'a str) -> Utf8InputText<'a> {
        let mut builder = Utf8InputTextBuilder::new(input, &self.grammar);

        for plugin in self.input_text_plugins.plugins() {
            plugin.rewrite(&mut builder);
        }
        builder.build()
    }

    fn split_sentences(&self, input: &'a Utf8InputText) -> SudachiResult<Vec<Utf8InputText>> {
        let mut sentences = Vec::new();
        let mut checker = NonBreakChecker::new(&self.lexicon, input);
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
}

impl Tokenize for Tokenizer<'_> {
    fn tokenize_sentences(
        &self,
        input: &str,
        mode: Mode,
        enable_debug: bool,
    ) -> SudachiResult<Vec<Vec<Morpheme>>> {
        if input.is_empty() {
            return Ok(vec![Vec::new()]);
        }

        let input = self.build_input_text(input);
        self.split_sentences(&input)?
            .iter()
            .map(|s| self.tokenize_input_text(s, mode, enable_debug))
            .collect()
    }

    fn tokenize(
        &self,
        input: &str,
        mode: Mode,
        enable_debug: bool,
    ) -> SudachiResult<Vec<Morpheme>> {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        let input = self.build_input_text(input);
        self.tokenize_input_text(&input, mode, enable_debug)
    }
}

impl Tokenizer<'_> {
    fn tokenize_input_text(
        &self,
        input: &Utf8InputText,
        mode: Mode,
        enable_debug: bool,
    ) -> SudachiResult<Vec<Morpheme>> {
        if enable_debug {
            println!("=== Input dump:\n{}", input.modified);
        }

        let lattice = self.build_lattice(input)?;
        if enable_debug {
            println!("=== Lattice dump:");
            lattice.dump(&self.grammar, &self.lexicon)?;
        };

        let mut path = lattice.get_best_path()?;
        // fill word_info to safely unwrap during path_rewrite and split_path
        for node in &mut path {
            node.fill_word_info(&self.lexicon)?;
        }
        if enable_debug {
            println!("=== Before Rewriting:");
            Tokenizer::dump_path(&path);
        };

        for plugin in self.path_rewrite_plugins.plugins() {
            path = plugin.rewrite(&input, path, &lattice)?;
        }
        let path = self.split_path(path, mode)?;
        if enable_debug {
            println!("=== After Rewriting:");
            Tokenizer::dump_path(&path);
            println!("===");
        };

        path.iter()
            .map(|node| Morpheme::new(&node, &input, &self.grammar, &self.lexicon))
            .collect::<SudachiResult<Vec<_>>>()
    }

    fn build_lattice(&self, input: &Utf8InputText) -> SudachiResult<Lattice> {
        let input_bytes = input.modified.as_bytes();
        let mut lattice = Lattice::new(&self.grammar, input_bytes.len());
        for (i, _) in input_bytes.iter().enumerate() {
            if !input.can_bow(i) || !lattice.has_previous_node(i) {
                continue;
            }

            let mut has_word = false;
            for (word_id, end) in self.lexicon.lookup(&input_bytes, i)? {
                if (end < input_bytes.len()) && !input.can_bow(end) {
                    continue;
                }
                has_word = true;
                let (left_id, right_id, cost) = self.lexicon.get_word_param(word_id)?;
                let node = Node::new(left_id, right_id, cost, word_id);
                lattice.insert(i, end, node)?;
            }

            // OOV
            if !input
                .get_char_category_types(i)
                .contains(&CategoryType::NOOOVBOW)
            {
                for oov_provider in self.oov_provider_plugins.plugins() {
                    for node in oov_provider.get_oov(&input, i, has_word)? {
                        has_word = true;
                        lattice.insert(node.begin, node.end, node)?;
                    }
                }
            }
            if !has_word {
                // use last oov_provider as default
                for node in self
                    .oov_provider_plugins
                    .plugins()
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

    fn split_path(&self, path: Vec<Node>, mode: Mode) -> SudachiResult<Vec<Node>> {
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
                    n.fill_word_info(&self.lexicon)?;
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

    fn dump_path(path: &Vec<Node>) {
        for (i, node) in (&path).iter().enumerate() {
            println!("{}: {}", i, node);
        }
    }
}
