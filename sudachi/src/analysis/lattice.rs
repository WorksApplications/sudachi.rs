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
use crate::analysis::node::{LatticeNode, PathCost, RightId};
use crate::dic::connect::ConnectionMatrix;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::subset::InfoSubset;
use crate::dic::word_id::WordId;
use crate::error::SudachiResult;
use crate::input_text::InputBuffer;
use crate::prelude::SudachiError;
use std::fmt::{Display, Formatter};
use std::io::Write;

/// Lattice Node for Viterbi Search.
/// Extremely small for better cache locality.
/// Current implementation has 25% efficiency loss because of padding :(
/// Maybe we should use array-of-structs layout instead, but I want to try to measure the
/// efficiency of that without the effects of the current rewrite.
struct VNode {
    total_cost: i32,
    right_id: u16,
}

impl RightId for VNode {
    #[inline]
    fn right_id(&self) -> u16 {
        self.right_id
    }
}

impl PathCost for VNode {
    #[inline]
    fn total_cost(&self) -> i32 {
        self.total_cost
    }
}

impl VNode {
    #[inline]
    fn new(right_id: u16, total_cost: i32) -> VNode {
        VNode {
            right_id,
            total_cost,
        }
    }
}

/// Lattice which is constructed for performing the Viterbi search.
/// Contain several parallel arrays.
/// First level of parallel arrays is indexed by end word boundary.
/// Word boundaries are always aligned to codepoint boundaries, not to byte boundaries.
///
/// During the successive analysis, we do not drop inner vectors, so
/// the size of vectors never shrink.
/// You must use the size parameter to check the current size and never
/// access vectors after the end.
pub struct Lattice {
    ends: Vec<Vec<VNode>>,
    ends_full: Vec<Vec<Node>>,
    indices: Vec<Vec<NodeIdx>>,
    eos: Option<(NodeIdx, i32)>,
    size: usize,
}

impl Default for Lattice {
    fn default() -> Self {
        Lattice {
            ends: Vec::new(),
            ends_full: Vec::new(),
            indices: Vec::new(),
            eos: None,
            size: 0,
        }
    }
}

impl Lattice {
    fn reset_vec<T>(data: &mut Vec<Vec<T>>, target: usize) {
        for v in data.iter_mut() {
            v.clear();
        }
        let cur_len = data.len();
        if cur_len <= target {
            data.reserve(target - cur_len);
            for _ in cur_len..target {
                data.push(Vec::with_capacity(16))
            }
        }
    }

    /// Prepare lattice for the next analysis of a sentence with the
    /// specified length (in codepoints)
    pub fn reset(&mut self, length: usize) {
        Self::reset_vec(&mut self.ends, length + 1);
        Self::reset_vec(&mut self.ends_full, length + 1);
        Self::reset_vec(&mut self.indices, length + 1);
        self.eos = None;
        self.size = length + 1;
        self.connect_bos();
    }

    fn connect_bos(&mut self) {
        self.ends[0].push(VNode::new(0, 0));
    }

    /// Find EOS node -- finish the lattice construction
    pub fn connect_eos(&mut self, conn: &ConnectionMatrix) -> SudachiResult<()> {
        let len = self.size;
        let eos_start = (len - 1) as u16;
        let eos_end = (len - 1) as u16;
        let node = Node::new(eos_start, eos_end, 0, 0, 0, WordId::EOS);
        let (idx, cost) = self.connect_node(&node, conn);
        if cost == i32::MAX {
            Err(SudachiError::EosBosDisconnect)
        } else {
            self.eos = Some((idx, cost));
            Ok(())
        }
    }

    /// Insert a single node in the lattice, founding the path to the previous node
    /// Assumption: lattice for all previous boundaries is already constructed
    pub fn insert(&mut self, node: Node, conn: &ConnectionMatrix) -> i32 {
        let (idx, cost) = self.connect_node(&node, conn);
        let end_idx = node.end();
        self.ends[end_idx].push(VNode::new(node.right_id(), cost));
        self.indices[end_idx].push(idx);
        self.ends_full[end_idx].push(node);
        cost
    }

    /// Find the path with the minimal cost through the lattice to the attached node
    /// Assumption: lattice for all previous boundaries is already constructed
    #[inline]
    pub fn connect_node(&self, r_node: &Node, conn: &ConnectionMatrix) -> (NodeIdx, i32) {
        let begin = r_node.begin();

        let node_cost = r_node.cost() as i32;
        let mut min_cost = i32::MAX;
        let mut prev_idx = NodeIdx::empty();

        for (i, l_node) in self.ends[begin].iter().enumerate() {
            if !l_node.is_connected_to_bos() {
                continue;
            }

            let connect_cost = conn.cost(l_node.right_id(), r_node.left_id()) as i32;
            let new_cost = l_node.total_cost() + connect_cost + node_cost;
            if new_cost < min_cost {
                min_cost = new_cost;
                prev_idx = NodeIdx::new(begin as u16, i as u16);
            }
        }

        (prev_idx, min_cost)
    }

    /// Checks if there exist at least one at the word end boundary
    pub fn has_previous_node(&self, i: usize) -> bool {
        self.ends.get(i).map(|d| !d.is_empty()).unwrap_or(false)
    }

    /// Lookup a node for the index
    pub fn node(&self, id: NodeIdx) -> (&Node, i32) {
        let node = &self.ends_full[id.end() as usize][id.index() as usize];
        let cost = self.ends[id.end() as usize][id.index() as usize].total_cost;
        (node, cost)
    }

    /// Fill the path with the minimum cost (indices only).
    /// **Attention**: the path will be reversed (end to beginning) and will need to be traversed
    /// in the reverse order.
    pub fn fill_top_path(&self, result: &mut Vec<NodeIdx>) {
        if self.eos.is_none() {
            return;
        }
        // start with EOS
        let (mut idx, _) = self.eos.unwrap();
        result.push(idx);
        loop {
            let prev_idx = self.indices[idx.end() as usize][idx.index() as usize];
            if prev_idx.end() != 0 {
                // add if not BOS
                result.push(prev_idx);
                idx = prev_idx;
            } else {
                // finish if BOS
                break;
            }
        }
    }
}

impl Lattice {
    pub fn dump<W: Write>(
        &self,
        input: &InputBuffer,
        grammar: &Grammar,
        lexicon: &LexiconSet,
        out: &mut W,
    ) -> SudachiResult<()> {
        enum PosData<'a> {
            Bos,
            Borrow(&'a [String]),
        }

        impl Display for PosData<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    PosData::Bos => write!(f, "BOS/EOS"),
                    PosData::Borrow(data) => {
                        for (i, s) in data.iter().enumerate() {
                            write!(f, "{}", s)?;
                            if i + 1 != data.len() {
                                write!(f, ", ")?;
                            }
                        }
                        Ok(())
                    }
                }
            }
        }

        let mut dump_idx = 0;

        for boundary in (0..self.indices.len()).rev() {
            let nodes = &self.ends_full[boundary];

            for node_idx in 0..nodes.len() {
                let r_node = &nodes[node_idx];
                let (surface, pos) = if r_node.is_special_node() {
                    ("(null)", PosData::Bos)
                } else if r_node.is_oov() {
                    let pos_id = r_node.word_id().word() as usize;
                    (
                        input.curr_slice_c(r_node.begin()..r_node.end()),
                        PosData::Borrow(&grammar.pos_list[pos_id]),
                    )
                } else {
                    let winfo =
                        lexicon.get_word_info_subset(r_node.word_id(), InfoSubset::POS_ID)?;
                    (
                        input.orig_slice_c(r_node.begin()..r_node.end()),
                        PosData::Borrow(&grammar.pos_list[winfo.pos_id() as usize]),
                    )
                };

                write!(
                    out,
                    "{}: {} {} {}{} {} {} {} {}:",
                    dump_idx,
                    r_node.begin(),
                    r_node.end(),
                    surface,
                    r_node.word_id(),
                    pos,
                    r_node.left_id(),
                    r_node.right_id(),
                    r_node.cost()
                )?;

                let conn = grammar.conn_matrix();

                for l_node in &self.ends[r_node.begin()] {
                    let connect_cost = conn.cost(l_node.right_id(), r_node.left_id());
                    write!(out, " {}", connect_cost)?;
                }

                write!(out, "\n")?;

                dump_idx += 1;
            }
        }
        Ok(())
    }
}
