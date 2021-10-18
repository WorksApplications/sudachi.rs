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

use crate::analysis::node::{LatticeNode, RightId};
use crate::dic::word_id::WordId;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
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

    pub fn end(&self) -> u16 {
        self.end
    }

    pub fn index(&self) -> u16 {
        self.index
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    begin: u16,
    end: u16,
    left_id: u16,
    right_id: u16,
    cost: i16,
    word_id: WordId,
}

impl Node {
    pub fn new(
        begin: u16,
        end: u16,
        left_id: u16,
        right_id: u16,
        cost: i16,
        word_id: WordId,
    ) -> Node {
        Node {
            begin,
            end,
            left_id,
            right_id,
            cost,
            word_id,
        }
    }

    pub fn set_range(&mut self, begin: u16, end: u16) {
        self.begin = begin;
        self.end = end;
    }
}

impl RightId for Node {
    #[inline(always)]
    fn right_id(&self) -> u16 {
        self.right_id
    }
}

impl LatticeNode for Node {
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
    use super::*;
    use claim::*;

    #[test]
    fn lesser_than_32b() {
        assert_le!(core::mem::size_of::<Node>(), 32);
    }
}
