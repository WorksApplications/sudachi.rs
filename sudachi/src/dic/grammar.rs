/*
 * Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use itertools::Itertools;
use std::ops::Index;

use crate::dic::binary_loader::BinaryGrammar;
use crate::dic::character_category::CharacterCategory;
use crate::dic::connect::ConnectionMatrix;
use crate::dic::pos::{PosList, POS_DEPTH};
use crate::prelude::*;

/// Dictionary grammar
///
/// Contains part_of_speech list and connection cost map.
/// It also holds character category.
pub struct Grammar<'a> {
    /// The list of part of speechs used in the dictionary
    pub pos_list: PosList,

    /// The mapping to overload cost table
    connection: ConnectionMatrix<'a>,

    /// The mapping from character to character_category_type
    pub character_category: CharacterCategory,
}

impl<'a> Grammar<'a> {
    pub const INHIBITED_CONNECTION: i16 = i16::MAX;

    pub const BOS_PARAMETER: (i16, i16, i16) = (0, 0, 0); // left_id, right_id, cost
    pub const EOS_PARAMETER: (i16, i16, i16) = (0, 0, 0); // left_id, right_id, cost

    pub fn from_system_binary(binary_grammar: BinaryGrammar<'a>) -> SudachiResult<Grammar<'a>> {
        let connection = binary_grammar
            .connection
            .ok_or(SudachiError::ConnectionMatrixMissing)?;

        Ok(Self::from_parts(binary_grammar.pos_list, connection))
    }

    pub(crate) fn from_parts(pos_list: PosList, connection: ConnectionMatrix<'a>) -> Self {
        Grammar {
            pos_list,
            connection,
            character_category: CharacterCategory::default(),
        }
    }

    /// Merge a another (user) grammar into this grammar
    ///
    /// Only pos_list is merged
    pub fn merge(&mut self, other: Grammar) {
        self.pos_list.extend(other.pos_list);
    }

    /// Merge a another (user) binary grammar into this grammar
    ///
    /// Only pos_list is merged
    pub fn merge_binary(&mut self, other: BinaryGrammar) {
        self.pos_list.extend(other.pos_list);
    }

    /// Returns connection cost of nodes
    ///
    /// left_id: right_id of left node
    /// right_id: left_if of right node
    #[inline(always)]
    pub fn connect_cost(&self, left_id: i16, right_id: i16) -> i16 {
        self.connection.cost(left_id as u16, right_id as u16)
    }

    #[inline]
    pub fn conn_matrix(&self) -> &ConnectionMatrix<'_> {
        &self.connection
    }

    /// Sets character category
    ///
    /// This is the only way to set character category.
    /// Character category will be a empty map by default.
    pub fn set_character_category(&mut self, character_category: CharacterCategory) {
        self.character_category = character_category;
    }

    /// Sets connect cost for a specific pair of ids
    ///
    /// left_id: right_id of left node
    /// right_id: left_if of right node
    pub fn set_connect_cost(&mut self, left_id: i16, right_id: i16, cost: i16) {
        // for edit connection cost plugin
        self.connection
            .update(left_id as u16, right_id as u16, cost);
    }

    /// Returns a pos_id of given pos in the grammar
    pub fn get_part_of_speech_id<S>(&self, pos1: &[S]) -> Option<u16>
    where
        S: AsRef<str>,
    {
        if pos1.len() != POS_DEPTH {
            return None;
        }
        for (i, pos2) in self.pos_list.iter().enumerate() {
            if pos1.iter().zip(pos2).all(|(a, b)| a.as_ref() == b) {
                return Some(i as u16);
            }
        }
        None
    }

    pub fn register_pos<S>(&mut self, pos: &[S]) -> SudachiResult<u16>
    where
        S: AsRef<str> + ToString,
    {
        if pos.len() != POS_DEPTH {
            let pos_string = pos.iter().map(|x| x.as_ref()).join(",");
            return Err(SudachiError::InvalidPartOfSpeech(pos_string));
        }
        match self.get_part_of_speech_id(pos) {
            Some(id) => Ok(id),
            None => {
                let new_id = self.pos_list.len();
                if new_id > u16::MAX as usize {
                    return Err(SudachiError::InvalidPartOfSpeech(
                        "Too much POS tags registered".to_owned(),
                    ));
                }
                let components = pos.iter().map(|x| x.to_string()).collect();
                self.pos_list.push(components);
                Ok(new_id as u16)
            }
        }
    }

    /// Gets POS components for POS ID.
    /// Panics if out of bounds.
    pub fn pos_components(&self, pos_id: u16) -> &[String] {
        self.pos_list.index(pos_id as usize)
    }
}

impl Grammar<'static> {
    pub fn empty() -> Self {
        Self::from_parts(PosList::default(), ConnectionMatrix::empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_size() {
        let grammar = setup_grammar();
        assert_eq!(grammar.pos_list.len(), 3);
        assert_eq!(grammar.conn_matrix().num_left(), 3);
        assert_eq!(grammar.conn_matrix().num_right(), 3);
    }

    #[test]
    fn partofspeech_string() {
        let grammar = setup_grammar();
        assert_eq!(6, grammar.pos_list[0].len());
        assert_eq!("BOS/EOS", grammar.pos_list[0][0]);
        assert_eq!("*", grammar.pos_list[0][5]);

        assert_eq!("一般", grammar.pos_list[1][1]);
        assert_eq!("*", grammar.pos_list[1][5]);

        assert_eq!("五段-サ行", grammar.pos_list[2][4]);
        assert_eq!("終止形-一般", grammar.pos_list[2][5]);
    }

    #[test]
    fn get_connect_cost() {
        let grammar = setup_grammar();
        assert_eq!(0, grammar.connect_cost(0, 0));
        assert_eq!(-100, grammar.connect_cost(2, 1));
        assert_eq!(200, grammar.connect_cost(1, 2));
    }

    #[test]
    fn set_connect_cost() {
        let mut grammar = setup_grammar();
        grammar.set_connect_cost(0, 0, 300);
        assert_eq!(300, grammar.connect_cost(0, 0));
    }

    #[test]
    fn register_pos() {
        let mut grammar = setup_grammar();

        let id1 = grammar
            .register_pos(["a", "b", "c", "d", "e", "f"].as_slice())
            .expect("failed");
        let id2 = grammar
            .register_pos(["a", "b", "c", "d", "e", "f"].as_slice())
            .expect("failed");
        assert_eq!(id1, id2);
    }

    #[test]
    fn bos_parameter() {
        assert_eq!(0, Grammar::BOS_PARAMETER.0);
        assert_eq!(0, Grammar::BOS_PARAMETER.1);
        assert_eq!(0, Grammar::BOS_PARAMETER.2);
    }

    #[test]
    fn eos_parameter() {
        assert_eq!(0, Grammar::EOS_PARAMETER.0);
        assert_eq!(0, Grammar::EOS_PARAMETER.1);
        assert_eq!(0, Grammar::EOS_PARAMETER.2);
    }

    fn setup_grammar() -> Grammar<'static> {
        let mut pos_list = PosList::default();
        let mut conn_bytes: Vec<u8> = Vec::new();
        build_connect_table(&mut conn_bytes);
        build_part_of_speech(&mut pos_list);
        let connection = ConnectionMatrix::from_bytes(Box::leak(conn_bytes.into_boxed_slice()))
            .expect("failed to create conn");
        Grammar {
            pos_list,
            connection,
            character_category: CharacterCategory::default(),
        }
    }

    fn build_part_of_speech(pos_list: &mut PosList) {
        pos_list.push(vec![
            "BOS/EOS".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
        ]);
        pos_list.push(vec![
            "名詞".to_string(),
            "一般".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
        ]);
        pos_list.push(vec![
            "動詞".to_string(),
            "一般".to_string(),
            "*".to_string(),
            "*".to_string(),
            "五段-サ行".to_string(),
            "終止形-一般".to_string(),
        ]);
    }
    fn build_connect_table(storage: &mut Vec<u8>) {
        storage.extend(&3_i16.to_le_bytes());
        storage.extend(&3_i16.to_le_bytes());

        storage.extend(&0_i16.to_le_bytes());
        storage.extend(&(-300_i16).to_le_bytes());
        storage.extend(&300_i16.to_le_bytes());

        storage.extend(&300_i16.to_le_bytes());
        storage.extend(&(-500_i16).to_le_bytes());
        storage.extend(&(-100_i16).to_le_bytes());

        storage.extend(&(-3000_i16).to_le_bytes());
        storage.extend(&200_i16.to_le_bytes());
        storage.extend(&2000_i16.to_le_bytes());
    }
}
