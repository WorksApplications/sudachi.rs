/*
 * Copyright (c) 2021 Works Applications Co., Ltd.
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

use crate::dic::character_category::CharacterCategory;
use crate::dic::connect::ConnectionMatrix;
use crate::dic::read::u16str::utf16_string_parser;
use crate::dic::POS_DEPTH;
use crate::error::SudachiNomResult;
use crate::prelude::*;
use nom::{
    bytes::complete::take,
    number::complete::{le_i16, le_u16},
};
use std::ops::Index;

/// Dictionary grammar
///
/// Contains part_of_speech list and connection cost map.
/// It also holds character category.
pub struct Grammar<'a> {
    _bytes: &'a [u8],
    pub pos_list: Vec<Vec<String>>,
    pub storage_size: usize,

    /// The mapping to overload cost table
    connection: ConnectionMatrix<'a>,

    /// The mapping from character to character_category_type
    pub character_category: CharacterCategory,
}

impl<'a> Grammar<'a> {
    pub const INHIBITED_CONNECTION: i16 = i16::MAX;

    pub const BOS_PARAMETER: (i16, i16, i16) = (0, 0, 0); // left_id, right_id, cost
    pub const EOS_PARAMETER: (i16, i16, i16) = (0, 0, 0); // left_id, right_id, cost

    /// Creates a Grammar from dictionary bytes
    ///
    /// buf: reference to the dictionary bytes
    /// offset: offset to the grammar section in the buf
    pub fn parse(buf: &[u8], offset: usize) -> SudachiResult<Grammar> {
        let (rest, (pos_list, left_id_size, right_id_size)) = grammar_parser(buf, offset)
            .map_err(|e| SudachiError::InvalidDictionaryGrammar.with_context(e.to_string()))?;

        let connect_table_offset = buf.len() - rest.len();
        let storage_size =
            (connect_table_offset - offset) + 2 * left_id_size as usize * right_id_size as usize;

        let conn = ConnectionMatrix::from_offset_size(
            buf,
            connect_table_offset,
            left_id_size as usize,
            right_id_size as usize,
        )?;

        Ok(Grammar {
            _bytes: buf,
            pos_list,
            connection: conn,
            storage_size,
            character_category: CharacterCategory::default(),
        })
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
    pub fn conn_matrix(&self) -> &ConnectionMatrix {
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
    pub fn get_part_of_speech_id(&self, pos1: &[&str]) -> Option<u16> {
        for (i, pos2) in self.pos_list.iter().enumerate() {
            if pos1.len() == pos2.len() && pos1.iter().zip(pos2).all(|(a, b)| a == b) {
                return Some(i as u16);
            }
        }
        None
    }

    /// Gets POS components for POS ID.
    /// Panics if out of bounds.
    pub fn pos_components(&self, pos_id: u16) -> &[String] {
        self.pos_list.index(pos_id as usize)
    }

    /// Merge a another grammar into this grammar
    ///
    /// Only pos_list is merged
    pub fn merge(&mut self, other: Grammar) {
        self.pos_list.extend(other.pos_list);
    }
}

fn pos_list_parser(input: &[u8]) -> SudachiNomResult<&[u8], Vec<Vec<String>>> {
    let (rest, pos_size) = le_u16(input)?;
    nom::multi::count(
        nom::multi::count(utf16_string_parser, POS_DEPTH),
        pos_size as usize,
    )(rest)
}

fn grammar_parser(
    input: &[u8],
    offset: usize,
) -> SudachiNomResult<&[u8], (Vec<Vec<String>>, i16, i16)> {
    nom::sequence::preceded(
        take(offset),
        nom::sequence::tuple((pos_list_parser, le_i16, le_i16)),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_size() {
        let bytes = setup_bytes();
        let grammar = Grammar::parse(&bytes, 0).expect("failed to create grammar");
        assert_eq!(bytes.len(), grammar.storage_size);
    }

    #[test]
    fn partofspeech_string() {
        let bytes = setup_bytes();
        let grammar = Grammar::parse(&bytes, 0).expect("failed to create grammar");
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
        let bytes = setup_bytes();
        let grammar = Grammar::parse(&bytes, 0).expect("failed to create grammar");
        assert_eq!(0, grammar.connect_cost(0, 0));
        assert_eq!(-100, grammar.connect_cost(2, 1));
        assert_eq!(200, grammar.connect_cost(1, 2));
    }

    #[test]
    fn set_connect_cost() {
        let bytes = setup_bytes();
        let mut grammar = Grammar::parse(&bytes, 0).expect("failed to create grammar");
        grammar.set_connect_cost(0, 0, 300);
        assert_eq!(300, grammar.connect_cost(0, 0));
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

    #[test]
    fn read_from_file() {
        // todo: after tidying up dictionary management
    }

    fn setup_bytes() -> Vec<u8> {
        let mut storage: Vec<u8> = Vec::new();
        build_partofspeech(&mut storage);
        build_connect_table(&mut storage);
        storage
    }
    fn string_to_bytes(s: &str) -> Vec<u8> {
        s.encode_utf16()
            .map(|c| c.to_le_bytes())
            .flatten()
            .collect()
    }
    fn build_partofspeech(storage: &mut Vec<u8>) -> () {
        // number of part of speech
        storage.extend(&(3 as i16).to_le_bytes());

        storage.extend(
            b"\x07B\x00O\x00S\x00/\x00E\x00O\x00S\x00\x01*\x00\x01*\x00\x01*\x00\x01*\x00\x01*\x00",
        );

        storage.extend(b"\x02");
        storage.extend(string_to_bytes("名刺"));
        storage.extend(b"\x02");
        storage.extend(string_to_bytes("一般"));
        storage.extend(b"\x01*\x00\x01*\x00\x01*\x00\x01*\x00");

        storage.extend(b"\x02");
        storage.extend(string_to_bytes("動詞"));
        storage.extend(b"\x02");
        storage.extend(string_to_bytes("一般"));
        storage.extend(b"\x01*\x00\x01*\x00\x05");
        storage.extend(string_to_bytes("五段-サ行"));
        storage.extend(b"\x06");
        storage.extend(string_to_bytes("終止形-一般"));
    }
    fn build_connect_table(storage: &mut Vec<u8>) -> () {
        storage.extend(&(3 as i16).to_le_bytes());
        storage.extend(&(3 as i16).to_le_bytes());

        storage.extend(&(0 as i16).to_le_bytes());
        storage.extend(&(-300 as i16).to_le_bytes());
        storage.extend(&(300 as i16).to_le_bytes());

        storage.extend(&(300 as i16).to_le_bytes());
        storage.extend(&(-500 as i16).to_le_bytes());
        storage.extend(&(-100 as i16).to_le_bytes());

        storage.extend(&(-3000 as i16).to_le_bytes());
        storage.extend(&(200 as i16).to_le_bytes());
        storage.extend(&(2000 as i16).to_le_bytes());
    }
}
