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

use crate::analysis::inner::{Node, NodeIdx};
use crate::analysis::lattice::Lattice;
use crate::analysis::node::{LatticeNode, ResultNode};
use crate::analysis::stateless_tokenizer::{dump_path, split_path, DictionaryAccess};
use crate::analysis::Mode;
use crate::dic::category_type::CategoryType;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::error::{SudachiError, SudachiResult};
use crate::input_text::InputBuffer;
use crate::input_text::InputTextIndex;
use crate::prelude::MorphemeList;

pub struct StatefulTokenizer<D> {
    dictionary: D,
    input: InputBuffer,
    debug: bool,
    mode: Mode,
    oov: Vec<Node>,
    lattice: Lattice,
    top_path_ids: Vec<NodeIdx>,
    top_path: Option<Vec<ResultNode>>,
}

impl<D: DictionaryAccess + Clone> StatefulTokenizer<D> {
    /// Get a clone of current dictionary
    pub fn dict_clone(&self) -> D {
        self.dictionary.clone()
    }
}

impl<D: DictionaryAccess> StatefulTokenizer<D> {
    /// Create a new non-debug stateful tokenizer
    pub fn new(dic: D, mode: Mode) -> Self {
        Self::create(dic, false, mode)
    }

    /// Create a new debug stateful tokenizer with the following options
    pub fn create(dic: D, debug: bool, mode: Mode) -> Self {
        Self {
            dictionary: dic,
            input: InputBuffer::default(),
            debug,
            mode,
            oov: Vec::with_capacity(10),
            lattice: Lattice::default(),
            top_path_ids: Vec::new(),
            top_path: Some(Vec::new()),
        }
    }

    /// Set debug flag and returns the current one
    pub fn set_debug(&mut self, debug: bool) -> bool {
        std::mem::replace(&mut self.debug, debug)
    }

    /// Set the analysis mode and returns the current one
    pub fn set_mode(&mut self, mode: Mode) -> Mode {
        std::mem::replace(&mut self.mode, mode)
    }

    /// Prepare StatefulTokenizer for the next data.
    /// Data must be written in the returned reference.
    pub fn reset(&mut self) -> &mut String {
        self.top_path.as_mut().map(|p| p.clear());
        self.oov.clear();
        self.input.reset()
    }

    /// Borrow current dictionary
    pub fn dict(&self) -> &D {
        &self.dictionary
    }

    /// Perform the actual tokenization so the analysis result will be available
    /// for consumption
    pub fn do_tokenize(&mut self) -> SudachiResult<()> {
        self.input.start_build()?;
        self.rewrite_input()?;
        self.input.build(self.dictionary.grammar())?;

        if self.input.current().is_empty() {
            return Ok(());
        }

        let debug = self.debug;

        if debug {
            println!("=== Input dump:\n{}", self.input.current());
        }

        self.build_lattice()?;

        if debug {
            println!("=== Lattice dump:");
            let dict = &self.dictionary;
            let mut writer = std::io::stdout();
            self.lattice
                .dump(&self.input, dict.grammar(), dict.lexicon(), &mut writer)?;
        };

        let mut path = self.resolve_best_path()?;

        if debug {
            println!("=== Before Rewriting:");
            dump_path(self.top_path.as_ref().unwrap());
        };

        for plugin in self.dictionary.path_rewrite_plugins() {
            path = plugin.rewrite(&self.input, path, &self.lattice)?;
        }

        path = split_path(&self.dictionary, path, self.mode)?;

        self.translate_indices(&mut path);

        self.top_path = Some(path);

        if debug {
            println!("=== After Rewriting:");
            dump_path(self.top_path.as_ref().unwrap());
            println!("===");
        };

        Ok(())
    }

    /// Resolve the path (as ResultNodes) with the smallest cost
    fn resolve_best_path(&mut self) -> SudachiResult<Vec<ResultNode>> {
        let lex = self.dictionary.lexicon();
        let mut path = std::mem::replace(&mut self.top_path, None).unwrap_or_else(|| Vec::new());
        self.lattice.fill_top_path(&mut self.top_path_ids);
        self.top_path_ids.reverse();
        for pid in self.top_path_ids.drain(..) {
            let (inner, cost) = self.lattice.node(pid);
            let wi = if inner.word_id().is_oov() {
                let curr_slice = self.input.curr_slice_c(inner.char_range()).to_owned();
                WordInfo {
                    pos_id: inner.word_id().word() as u16,
                    dictionary_form: curr_slice.clone(),
                    normalized_form: curr_slice,
                    ..Default::default()
                }
            } else {
                lex.get_word_info(inner.word_id())?
            };

            let byte_begin = self.input.to_curr_byte_idx(inner.begin());
            let byte_end = self.input.to_curr_byte_idx(inner.end());

            path.push(ResultNode::new(
                inner.clone(),
                cost,
                byte_begin as u16,
                byte_end as u16,
                wi,
            ));
        }
        Ok(path)
    }

    /// Translate ResultNode indices from normalized data to original data
    fn translate_indices(&self, path: &mut Vec<ResultNode>) {
        let input = &self.input;
        for elem in path {
            let char_begin = input.to_orig_char_idx(elem.begin());
            let char_end = input.to_orig_char_idx(elem.end());
            let byte_begin = input.to_orig_byte_idx(elem.begin());
            let byte_end = input.to_orig_byte_idx(elem.end());
            elem.set_char_range(char_begin as u16, char_end as u16);
            elem.set_bytes_range(byte_begin as u16, byte_end as u16);
        }
    }

    /// Swap result data with the current analyzer
    pub fn swap_result(&mut self, input: &mut String, result: &mut Vec<ResultNode>) {
        self.input.swap_original(input);
        std::mem::swap(self.top_path.as_mut().unwrap(), result);
    }

    fn rewrite_input(&mut self) -> SudachiResult<()> {
        for p in self.dictionary.input_text_plugins() {
            p.rewrite(&mut self.input)?;
        }
        Ok(())
    }

    fn build_lattice(&mut self) -> SudachiResult<()> {
        let input = &self.input;
        let dict = &self.dictionary;
        let input_bytes = input.current().as_bytes();
        let oovs = &mut self.oov;
        let lattice = &mut self.lattice;

        lattice.reset(input.current_chars().len());

        for (ch_off, &byte_off) in input.curr_byte_offsets().iter().enumerate() {
            if !input.can_bow(byte_off) {
                continue;
            }

            if !lattice.has_previous_node(ch_off) {
                continue;
            }

            let mut has_word = false;
            for e in dict.lexicon().lookup(input_bytes, byte_off) {
                if (e.end < input_bytes.len()) && !input.can_bow(e.end) {
                    continue;
                }
                has_word = true;
                let (left_id, right_id, cost) = dict.lexicon().get_word_param(e.word_id)?;
                let end_c = input.ch_idx(e.end);
                let node = Node::new(
                    ch_off as u16,
                    end_c as u16,
                    left_id as u16,
                    right_id as u16,
                    cost,
                    e.word_id,
                );
                lattice.insert(node, dict.grammar().conn_matrix());
            }

            // OOV
            if !input.cat_at_char(ch_off).contains(CategoryType::NOOOVBOW) {
                for oov_provider in dict.oov_provider_plugins() {
                    oov_provider.get_oov(&input, ch_off, has_word, oovs)?;
                }
                for node in oovs.drain(..) {
                    has_word = true;
                    lattice.insert(node, dict.grammar().conn_matrix());
                }
            }

            if !has_word {
                dict.oov_provider_plugins()
                    .last()
                    .unwrap()
                    .get_oov(&input, ch_off, has_word, oovs)?;
                // use last oov_provider as default
                for node in oovs.drain(..) {
                    has_word = true;
                    lattice.insert(node, dict.grammar().conn_matrix());
                }
            }

            if !has_word {
                panic!("no morpheme found at {}", byte_off);
            }
        }
        lattice.connect_eos(dict.grammar().conn_matrix())?;

        Ok(())
    }

    /// Consume the Tokenizer and produce MorphemeList
    pub fn into_morpheme_list(self) -> SudachiResult<MorphemeList<D>> {
        match self.top_path {
            None => Err(SudachiError::EosBosDisconnect),
            Some(path) => {
                MorphemeList::from_components(self.dictionary, self.input.into_original(), path)
            }
        }
    }
}
