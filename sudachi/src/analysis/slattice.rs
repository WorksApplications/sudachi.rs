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

use crate::analysis::inner::{InnerNode, NodeIdx};
use crate::analysis::node::{LatticeNode, Node, PathCost, RightId};
use crate::dic::connect::ConnectionMatrix;
use crate::dic::grammar::Grammar;

struct XNode {
    total_cost: i32,
    right_id: u16,
}

impl RightId for XNode {
    #[inline]
    fn right_id(&self) -> u16 {
        self.right_id
    }
}

impl PathCost for XNode {
    #[inline]
    fn total_cost(&self) -> i32 {
        self.total_cost
    }
}

impl XNode {
    #[inline]
    fn new(right_id: u16, total_cost: i32) -> XNode {
        XNode {
            right_id,
            total_cost,
        }
    }
}

pub struct XLattice {
    ends: Vec<Vec<XNode>>,
    ends_full: Vec<Vec<InnerNode>>,
    indices: Vec<Vec<NodeIdx>>,
}

impl XLattice {
    fn reset_vec<T>(data: &mut Vec<Vec<T>>, target: usize) {
        let i = 0;
        for v in data.iter_mut() {
            v.clear();
        }
        data.reserve(target - i);
        for _ in i..target {
            data.push(Vec::with_capacity(16))
        }
    }

    pub fn reset(&mut self, length: usize) {
        Self::reset_vec(&mut self.ends, length);
        Self::reset_vec(&mut self.ends_full, length);
        Self::reset_vec(&mut self.indices, length);
    }

    pub fn insert(&mut self, mut node: InnerNode, conn: &ConnectionMatrix) {
        let (idx, cost) = self.connect_node(&node, conn);
        let end_idx = node.end();
        self.ends[end_idx].push(XNode::new(node.right_id(), cost));
        self.indices[end_idx].push(idx);
        self.ends_full[end_idx].push(node);
    }

    #[inline]
    pub fn connect_node(&self, r_node: &InnerNode, conn: &ConnectionMatrix) -> (NodeIdx, i32) {
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
}
