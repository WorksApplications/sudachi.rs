use std::fmt;

use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::dic::lexicon_set::LexiconSet;
use crate::prelude::*;

// TODO: clone?
#[derive(Clone, Debug, Default)]
pub struct Node {
    pub begin: usize,
    pub end: usize,

    pub left_id: i16,
    pub right_id: i16,
    pub cost: i16,

    pub word_id: Option<u32>,
    // todo: memoize
    pub word_info: Option<WordInfo>,
    pub is_oov: bool,

    pub total_cost: i32,
    //pub best_previous_node: Option<&'a Node<'a>>, // TODO: lifetime problem
    pub best_previous_node_index: Option<(usize, usize)>,
    pub is_connected_to_bos: bool,
}

impl Node {
    pub fn new(left_id: i16, right_id: i16, cost: i16, word_id: u32) -> Node {
        Node {
            left_id,
            right_id,
            cost,
            word_id: Some(word_id),
            ..Default::default()
        }
    }

    pub fn new_default() -> Node {
        Default::default()
    }

    pub fn set_range(&mut self, begin: usize, end: usize) {
        self.begin = begin;
        self.end = end;
    }

    pub fn set_word_info(&mut self, word_info: WordInfo) {
        self.word_info = Some(word_info);
    }

    pub fn fill_word_info(&mut self, lexicon: &LexiconSet) -> SudachiResult<()> {
        if let None = &self.word_info {
            let word_id = self.word_id.ok_or(SudachiError::MissingWordId)?;
            self.set_word_info(lexicon.get_word_info(word_id)?);
        }
        Ok(())
    }

    pub fn new_bos() -> Node {
        let (left_id, right_id, cost) = Grammar::BOS_PARAMETER;
        Node {
            left_id,
            right_id,
            cost,
            is_connected_to_bos: true,
            ..Default::default()
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
            ..Default::default()
        }
    }

    pub fn new_oov(left_id: i16, right_id: i16, cost: i16, word_info: WordInfo) -> Node {
        Node {
            left_id,
            right_id,
            cost,
            word_id: None,
            word_info: Some(word_info),
            is_oov: true,
            ..Default::default()
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
