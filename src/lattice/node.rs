use std::fmt;

use crate::dic::grammar::Grammar;

// TODO: clone?
#[derive(Clone)]
pub struct Node {
    pub begin: usize,
    pub end: usize,

    pub left_id: i16,
    pub right_id: i16,
    pub cost: i16,

    pub word_id: Option<u32>,

    pub total_cost: i32,
    //pub best_previous_node: Option<&'a Node<'a>>, // TODO: lifetime problem
    pub best_previous_node_index: Option<(usize, usize)>,
    pub is_connected_to_bos: bool,
}

impl Node {
    pub fn new(left_id: i16, right_id: i16, cost: i16, word_id: u32) -> Node {
        Node {
            begin: 0,
            end: 0,
            left_id,
            right_id,
            cost,
            word_id: Some(word_id),
            total_cost: 0,
            best_previous_node_index: None,
            is_connected_to_bos: false,
        }
    }

    pub fn set_range(&mut self, begin: usize, end: usize) {
        self.begin = begin;
        self.end = end;
    }

    pub fn new_bos() -> Node {
        let (left_id, right_id, cost) = Grammar::BOS_PARAMETER;
        Node {
            begin: 0,
            end: 0,
            left_id,
            right_id,
            cost,
            word_id: None,
            total_cost: 0,
            best_previous_node_index: None,
            is_connected_to_bos: true,
        }
    }

    pub fn new_eos(size: usize) -> Node {
        let (left_id, right_id, cost) = Grammar::EOS_PARAMETER;
        Node {
            begin: size,
            end: size,
            left_id,
            right_id,
            cost,
            word_id: None,
            total_cost: 0,
            best_previous_node_index: None,
            is_connected_to_bos: false,
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: print `surface` here
        // "{begin} {end} {surface}({word_id}) {left_id} {right_id} {cost}"
        // To get surface, we need lexicon - `lexicon.get_word_info(word_id).surface`

        write!(
            f,
            "{} {} ({}) {} {} {}",
            self.begin,
            self.end,
            match self.word_id {
                Some(word_id) => word_id.to_string(),
                None => "-1".to_string(),
            },
            self.left_id,
            self.right_id,
            self.cost
        )
    }
}
