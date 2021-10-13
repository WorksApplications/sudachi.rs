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

use crate::analysis::node::{LatticeNode, PathCost, RightId};
use crate::dic::word_id::WordId;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct NodeIdx {
    end: u16,
    index: u16,
}

impl NodeIdx {
    pub fn empty() -> NodeIdx {
        NodeIdx {
            end: u16::MAX,
            index: u16::MAX,
        }
    }

    pub fn new(end: u16, index: u16) -> NodeIdx {
        NodeIdx { end, index }
    }

    fn end(&self) -> u16 {
        self.end
    }
    fn index(&self) -> u16 {
        self.index
    }
    fn is_empty(&self) -> bool {
        self.index() == u16::MAX
    }
}

#[derive(Clone, Debug)]
pub struct InnerNode {
    begin: u16,
    end: u16,
    left_id: u16,
    right_id: u16,
    cost: i16,
    word_id: WordId,
}

impl RightId for InnerNode {
    #[inline(always)]
    fn right_id(&self) -> u16 {
        self.right_id
    }
}

impl LatticeNode for InnerNode {
    fn begin(&self) -> usize {
        self.begin as usize
    }

    fn end(&self) -> usize {
        self.end as usize
    }

    fn cost(&self) -> i16 {
        self.cost
    }

    fn word_id(&self) -> WordId {
        self.word_id
    }

    fn left_id(&self) -> u16 {
        self.left_id
    }
}

#[cfg(test)]
mod test {
    use crate::analysis::inner::InnerNode;
    use claim::*;

    #[test]
    fn lesser_than_32b() {
        assert_le!(core::mem::size_of::<InnerNode>(), 32);
    }
}
