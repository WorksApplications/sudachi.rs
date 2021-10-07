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

use crate::analysis::lattice::Lattice;
use crate::analysis::node::Node;
use crate::analysis::stateless_tokenizer::{dump_path, split_path, DictionaryAccess};
use crate::analysis::Mode;
use crate::dic::category_type::CategoryType;
use crate::error::SudachiResult;
use crate::input_text::input_buffer::InputBuffer;
use crate::input_text::InputTextIndex;

struct StatefulTokenizer<D>
where
    D: DictionaryAccess,
{
    dictionary: D,
    input: InputBuffer,
    debug: bool,
    mode: Mode,
    top_path: Vec<Node>,
    oov: Vec<Node>,
}

impl<D: DictionaryAccess> StatefulTokenizer<D> {
    pub fn reset(&mut self) -> &mut String {
        self.input.reset()
    }

    pub fn do_tokenize(&mut self) -> SudachiResult<()> {
        self.input.start_build();
        self.rewrite_input()?;
        self.input.build(self.dictionary.grammar())?;
        let debug = self.debug;

        if debug {
            println!("=== Input dump:\n{}", self.input.current());
        }

        //HACK: fix lattice later, borrow checker can't be satisfied easily now
        let lattice: Lattice = unsafe { std::mem::transmute(self.build_lattice()?) };

        if debug {
            println!("=== Lattice dump:");
            let dict = &self.dictionary;
            lattice.dump(dict.grammar(), dict.lexicon())?;
        };

        let mut top_path = lattice.get_best_path()?;

        let lexicon = self.dictionary.lexicon();
        for node in &mut top_path {
            node.fill_word_info(lexicon)?;
        }

        if debug {
            println!("=== Before Rewriting:");
            dump_path(&top_path);
        };

        for plugin in self.dictionary.path_rewrite_plugins() {
            top_path = plugin.rewrite2(&self.input, top_path, &lattice)?;
        }

        self.top_path = split_path(&self.dictionary, top_path, self.mode)?;

        if debug {
            println!("=== After Rewriting:");
            dump_path(&self.top_path);
            println!("===");
        };

        Ok(())
    }

    pub fn swap_result(&mut self, input: &mut String, result: &mut Vec<Node>) {
        self.input.swap_original(input);
        std::mem::swap(&mut self.top_path, result);
    }

    fn rewrite_input(&mut self) -> SudachiResult<()> {
        for p in self.dictionary.input_text_plugins() {
            p.apply_rewrite(&mut self.input)?;
        }
        Ok(())
    }

    fn build_lattice(&mut self) -> SudachiResult<Lattice> {
        let input = &self.input;
        let dict = &self.dictionary;
        let input_bytes = input.current().as_bytes();
        let oovs = &mut self.oov;
        let mut lattice = Lattice::new(self.dictionary.grammar(), input_bytes.len());

        for i in 0..input_bytes.len() {
            if !input.can_bow(i) || !lattice.has_previous_node(i) {
                continue;
            }

            let mut has_word = false;
            for e in dict.lexicon().lookup(input_bytes, i) {
                if (e.end < input_bytes.len()) && !input.can_bow(e.end) {
                    continue;
                }
                has_word = true;
                let (left_id, right_id, cost) = dict.lexicon().get_word_param(e.word_id)?;
                let node = Node::new(left_id, right_id, cost, e.word_id);
                lattice.insert(i, e.end, node)?;
            }

            // OOV
            if !input.cat_at_byte(i).contains(CategoryType::NOOOVBOW) {
                for oov_provider in dict.oov_provider_plugins() {
                    oov_provider.get_oov2(&input, i, has_word, oovs)?;
                }
                for node in oovs.drain(..) {
                    has_word = true;
                    lattice.insert(node.begin, node.end, node)?;
                }
            }
            if !has_word {
                dict.oov_provider_plugins()
                    .last()
                    .unwrap()
                    .get_oov2(&input, i, has_word, oovs)?;
                // use last oov_provider as default
                for node in oovs.drain(..) {
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
}