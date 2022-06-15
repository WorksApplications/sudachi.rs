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

use crate::analysis::created::CreatedWords;
use crate::analysis::inner::{Node, NodeIdx};
use crate::analysis::lattice::Lattice;
use crate::analysis::node::{LatticeNode, ResultNode};
use crate::analysis::stateless_tokenizer::{dump_path, split_path, DictionaryAccess};
use crate::analysis::Mode;
use crate::dic::category_type::CategoryType;
use crate::dic::connect::ConnectionMatrix;
use crate::dic::lexicon::word_infos::WordInfoData;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::subset::InfoSubset;
use crate::error::{SudachiError, SudachiResult};
use crate::input_text::InputBuffer;
use crate::input_text::InputTextIndex;
use crate::plugin::oov::OovProviderPlugin;
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
    subset: InfoSubset,
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
            subset: InfoSubset::all(),
        }
    }

    /// Set debug flag and returns the current one
    pub fn set_debug(&mut self, debug: bool) -> bool {
        std::mem::replace(&mut self.debug, debug)
    }

    /// Set the analysis mode and returns the current one
    pub fn set_mode(&mut self, mode: Mode) -> Mode {
        self.subset |= match mode {
            Mode::A => InfoSubset::SPLIT_A,
            Mode::B => InfoSubset::SPLIT_B,
            _ => InfoSubset::empty(),
        };
        std::mem::replace(&mut self.mode, mode)
    }

    /// Analyzer will read only following [`WordInfo`] field subset
    pub fn set_subset(&mut self, subset: InfoSubset) -> InfoSubset {
        let mode_subset = match self.mode {
            Mode::A => InfoSubset::SPLIT_A,
            Mode::B => InfoSubset::SPLIT_B,
            _ => InfoSubset::empty(),
        };
        let new_subset = (subset | mode_subset).normalize();
        std::mem::replace(&mut self.subset, new_subset | mode_subset)
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
            dump_path(&path);
        };

        for plugin in self.dictionary.path_rewrite_plugins() {
            path = plugin.rewrite(&self.input, path, &self.lattice)?;
        }

        path = split_path(&self.dictionary, path, self.mode, self.subset, &self.input)?;

        if debug {
            println!("=== After Rewriting:");
            dump_path(&path);
            println!("===");
        };

        self.top_path = Some(path);

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
                WordInfoData {
                    pos_id: inner.word_id().word() as u16,
                    surface: curr_slice,
                    ..Default::default()
                }
                .into()
            } else {
                lex.get_word_info_subset(inner.word_id(), self.subset)?
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

    /// Swap result data with the current analyzer
    pub fn swap_result(
        &mut self,
        input: &mut InputBuffer,
        result: &mut Vec<ResultNode>,
        subset: &mut InfoSubset,
    ) {
        std::mem::swap(&mut self.input, input);
        std::mem::swap(self.top_path.as_mut().unwrap(), result);
        *subset = self.subset;
    }

    fn rewrite_input(&mut self) -> SudachiResult<()> {
        for p in self.dictionary.input_text_plugins() {
            p.rewrite(&mut self.input)?;
        }
        Ok(())
    }

    fn build_lattice(&mut self) -> SudachiResult<()> {
        let mut builder = LatticeBuilder {
            node_buffer: &mut self.oov,
            lattice: &mut self.lattice,
            matrix: self.dictionary.grammar().conn_matrix(),
            oov_providers: self.dictionary.oov_provider_plugins(),
            lexicon: self.dictionary.lexicon(),
            input: &self.input,
        };
        builder.build_lattice()
    }

    /// Consume the Tokenizer and produce MorphemeList
    pub fn into_morpheme_list(self) -> SudachiResult<MorphemeList<D>> {
        match self.top_path {
            None => Err(SudachiError::EosBosDisconnect),
            Some(path) => Ok(MorphemeList::from_components(
                self.dictionary,
                self.input,
                path,
                self.subset,
            )),
        }
    }
}

// This structure is purely for Rust.
// Otherwise splitting code into functions fails to compile with double borrow errors
struct LatticeBuilder<'a> {
    node_buffer: &'a mut Vec<Node>,
    lattice: &'a mut Lattice,
    matrix: &'a ConnectionMatrix<'a>,
    input: &'a InputBuffer,
    lexicon: &'a LexiconSet<'a>,
    oov_providers: &'a [Box<dyn OovProviderPlugin + Sync + Send>],
}

impl<'a> LatticeBuilder<'a> {
    #[inline]
    fn build_lattice(&mut self) -> SudachiResult<()> {
        self.lattice.reset(self.input.current_chars().len());
        let input_bytes = self.input.current().as_bytes();

        for (ch_off, &byte_off) in self.input.curr_byte_offsets().iter().enumerate() {
            if !self.lattice.has_previous_node(ch_off) {
                continue;
            }

            self.node_buffer.clear();
            let mut created = CreatedWords::default();
            for e in self.lexicon.lookup(input_bytes, byte_off) {
                // do we really need input.can_bow condition?
                if (e.end < input_bytes.len()) && !self.input.can_bow(e.end) {
                    continue;
                }
                let (left_id, right_id, cost) = self.lexicon.get_word_param(e.word_id);
                let end_c = self.input.ch_idx(e.end);
                let node = Node::new(
                    ch_off as u16,
                    end_c as u16,
                    left_id as u16,
                    right_id as u16,
                    cost,
                    e.word_id,
                );
                created = created.add_word((end_c - ch_off) as i64);
                self.node_buffer.push(node.clone());
                self.lattice.insert(node, self.matrix);
            }

            // OOV
            if !self
                .input
                .cat_at_char(ch_off)
                .intersects(CategoryType::NOOOVBOW | CategoryType::NOOOVBOW2)
            {
                for provider in self.oov_providers {
                    created = self.provide_oovs(ch_off, created, provider.as_ref())?;
                }
            }

            if created.is_empty() {
                let provider = self.oov_providers.last().unwrap();
                created = self.provide_oovs(ch_off, created, provider.as_ref())?;
            }

            if created.is_empty() {
                return Err(SudachiError::EosBosDisconnect);
            }
        }
        self.lattice.connect_eos(self.matrix)?;

        Ok(())
    }

    #[inline]
    fn provide_oovs<P>(
        &mut self,
        char_offset: usize,
        mut other: CreatedWords,
        plugin: &P,
    ) -> SudachiResult<CreatedWords>
    where
        P: OovProviderPlugin + 'a + ?Sized,
    {
        let start_size = self.node_buffer.len();
        let num_provided = plugin.provide_oov(self.input, char_offset, other, self.node_buffer)?;
        for idx in start_size..(start_size + num_provided) {
            let node = self.node_buffer[idx].clone();
            other = other.add_word(node.char_range().len() as i64);
            self.lattice.insert(node, self.matrix);
        }
        Ok(other)
    }
}
