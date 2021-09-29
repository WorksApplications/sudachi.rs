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

use multiset::HashMultiSet;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::iter::FromIterator;
use std::path::PathBuf;
use std::u32;
use thiserror::Error;

use crate::dic::category_type::{CategoryType, CategoryTypes};
use crate::prelude::*;

/// Sudachi error
#[derive(Error, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    #[error("Invalid format at line {0}")]
    InvalidFormat(usize),

    #[error("Invalid type {1} at line {0}")]
    InvalidCategoryType(usize, String),

    #[error("Multiple definition for type {1} at line {0}")]
    MultipleTypeDefinition(usize, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Range {
    begin: u32,
    end: u32,
    categories: CategoryTypes,
}

impl Range {
    fn contains(&self, cp: u32) -> bool {
        self.begin <= cp && cp < self.end
    }
    fn lower(&self, cp: u32) -> bool {
        self.end <= cp
    }
    fn _higher(&self, cp: u32) -> bool {
        cp < self.begin
    }
}

/// CharacterCategory holds mapping from character to character category type
#[derive(Debug, Default, Clone)]
pub struct CharacterCategory {
    ranges: Vec<Range>,
}

impl CharacterCategory {
    /// Creates a character category from file
    pub fn from_file(path: &PathBuf) -> SudachiResult<CharacterCategory> {
        let reader = BufReader::new(fs::File::open(path)?);
        Self::from_reader(reader)
    }

    pub fn from_reader<T: BufRead>(data: T) -> SudachiResult<CharacterCategory> {
        let ranges = Self::read_character_definition(data)?;
        Ok(Self::compile(ranges))
    }

    /// Reads character type definition as a list of Ranges
    ///
    /// Definition file syntax:
    ///     Each line contains [TARGET_CHARACTER_CODE_POINT] [TYPES], where
    ///     TARGET_CHARACTER_CODE_POINT:
    ///         a code_point in hexadecimal format or two separated by ".."
    ///     TYPES:
    ///         one or more Category_types separated by white space
    ///     Loads only lines start with "0x" are loaded and ignore others
    ///
    /// Definition example:
    ///     "0x0030..0x0039 NUMERIC"
    ///     "0x3008         KANJI KANJINUMERIC"
    fn read_character_definition<T: BufRead>(reader: T) -> SudachiResult<Vec<Range>> {
        let mut ranges: Vec<Range> = Vec::new();
        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim();
            if line.is_empty()
                || line.chars().next().unwrap() == '#'
                || line.chars().take(2).collect::<Vec<_>>() != vec!['0', 'x']
            {
                continue;
            }

            let cols: Vec<_> = line.split_whitespace().collect();
            if cols.len() < 2 {
                return Err(SudachiError::InvalidCharacterCategory(
                    Error::InvalidFormat(i),
                ));
            }

            let r: Vec<_> = cols[0].split("..").collect();
            let begin = u32::from_str_radix(String::from(r[0]).trim_start_matches("0x"), 16)?;
            let end = if r.len() > 1 {
                u32::from_str_radix(String::from(r[1]).trim_start_matches("0x"), 16)? + 1
            } else {
                begin + 1
            };
            if begin >= end {
                return Err(SudachiError::InvalidCharacterCategory(
                    Error::InvalidFormat(i),
                ));
            }

            let mut categories = CategoryTypes::new();
            for elem in cols[1..]
                .iter()
                .take_while(|elem| elem.chars().next().unwrap() != '#')
            {
                categories.insert(match elem.parse() {
                    Ok(t) => t,
                    Err(_) => {
                        return Err(SudachiError::InvalidCharacterCategory(
                            Error::InvalidCategoryType(i, elem.to_string()),
                        ))
                    }
                });
            }

            ranges.push(Range {
                begin,
                end,
                categories,
            });
        }

        Ok(ranges)
    }

    /// Creates a character category from given range_list
    ///
    /// Transforms given range_list to non overlapped range list
    /// to apply binary search in get_category_types
    fn compile(mut ranges: Vec<Range>) -> CharacterCategory {
        if ranges.is_empty() {
            return CharacterCategory { ranges: Vec::new() };
        }

        // implement order for Heap
        // note that here we use min-heap, based on the end of range
        impl Ord for Range {
            fn cmp(&self, other: &Self) -> Ordering {
                other.end.cmp(&self.end)
            }
        }
        impl PartialOrd for Range {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        // sort in the descending order to pop from the minimum value
        ranges.sort_by_key(|a| Reverse(a.begin));
        let mut non_overlap_ranges = vec![];
        let mut left_chain: BinaryHeap<Range> = BinaryHeap::new();
        let mut category_state: HashMultiSet<CategoryType> = HashMultiSet::new();
        let mut pivot = 0;
        loop {
            let left = match left_chain.pop() {
                None => {
                    let right = match ranges.pop() {
                        Some(v) => v,
                        None => break,
                    };
                    pivot = right.begin;
                    category_state = right.categories.iter().map(|v| *v).collect();
                    left_chain.push(right);
                    continue;
                }
                Some(v) => v,
            };
            let right = match ranges.last() {
                None => Range {
                    begin: u32::MAX,
                    end: 0,
                    categories: CategoryTypes::new(),
                },
                Some(v) => v.clone(),
            };
            if left.end <= right.begin {
                non_overlap_ranges.push(Range {
                    begin: pivot,
                    end: left.end,
                    categories: category_state.distinct_elements().map(|v| *v).collect(),
                });
                pivot = left.end;
                category_state = category_state - left.categories.iter().map(|v| *v).collect();
            } else {
                non_overlap_ranges.push(Range {
                    begin: pivot,
                    end: right.begin,
                    categories: category_state.distinct_elements().map(|v| *v).collect(),
                });
                pivot = right.begin;
                category_state = category_state + right.categories.iter().map(|v| *v).collect();

                left_chain.push(right);
                left_chain.push(left);
                ranges.pop();
            }
        }

        // merge adjacent ranges with same categories
        let mut new_ranges = vec![];
        let mut left = non_overlap_ranges[0].clone();
        for right in non_overlap_ranges.iter().skip(1) {
            if left.end == right.begin && left.categories == right.categories {
                left.end = right.end;
            } else {
                new_ranges.push(left);
                left = right.clone();
            }
        }
        new_ranges.push(left);

        CharacterCategory { ranges: new_ranges }
    }

    /// Returns a set of category types which given char has
    pub fn get_category_types(&self, c: char) -> CategoryTypes {
        if self.ranges.is_empty() {
            return FromIterator::from_iter([CategoryType::DEFAULT]);
        }

        let code_point = c as u32;

        // binary search
        let mut begin = 0;
        let mut end = self.ranges.len();
        let mut pivot = (begin + end) / 2;
        loop {
            let range = self.ranges.get(pivot).unwrap();
            if range.contains(code_point) {
                return range.categories.clone();
            }
            if range.lower(code_point) {
                begin = pivot;
            } else {
                end = pivot;
            }
            let new_pivot = (begin + end) / 2;
            if new_pivot == pivot {
                break;
            }
            pivot = new_pivot;
        }

        FromIterator::from_iter([CategoryType::DEFAULT])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::char;
    use std::io::{Seek, SeekFrom, Write};
    use tempfile::tempfile;

    const TEST_RESOURCE_DIR: &str = "./tests/resources/";
    const TEST_CHAR_DEF_FILE: &str = "char.def";

    impl Range {
        pub fn containing_length(&self, text: &str) -> usize {
            for (i, c) in text.chars().enumerate() {
                let code_point = c as u32;
                if code_point < self.begin || self.end < code_point {
                    return i;
                }
            }
            text.chars().count()
        }
    }

    #[test]
    fn range_containing_length() {
        let range = Range {
            begin: 0x41u32,
            end: 0x54u32,
            categories: CategoryTypes::default(),
        };
        assert_eq!(3, range.containing_length("ABC12"));
        assert_eq!(0, range.containing_length("熙"));
    }

    #[test]
    fn get_category_types() {
        let path = PathBuf::from(TEST_RESOURCE_DIR).join(TEST_CHAR_DEF_FILE);
        let cat = CharacterCategory::from_file(&path).expect("failed to load char.def for test");
        let cats = cat.get_category_types('熙');
        assert_eq!(1, cats.len());
        assert!(cats.contains(&CategoryType::KANJI));
    }

    #[test]
    fn read_character_definition() {
        let cats_default = [CategoryType::DEFAULT];
        let cats_num = [CategoryType::NUMERIC];
        let cats_alp = [CategoryType::ALPHA];
        let cats_kanji = [CategoryType::KANJI];
        let cats_kanji_num = [CategoryType::KANJI, CategoryType::NUMERIC];
        let cats_kanjinumeric = [CategoryType::KANJI, CategoryType::KANJINUMERIC];
        let cats_num_kanji_kana = [
            CategoryType::NUMERIC,
            CategoryType::KANJI,
            CategoryType::KATAKANA,
        ];
        let cats_num_alpha_kana = [
            CategoryType::NUMERIC,
            CategoryType::ALPHA,
            CategoryType::KATAKANA,
        ];

        // 1.
        let mut file = tempfile().expect("Failed to get temporary file");
        writeln!(file, "#\n").unwrap();
        writeln!(file, "0x0030..0x0039 NUMERIC").unwrap();
        writeln!(file, "0x0032         KANJI").unwrap();
        file.flush().expect("Failed to flush");
        file.seek(SeekFrom::Start(0)).expect("Failed to seek");
        let ranges = CharacterCategory::read_character_definition(BufReader::new(file))
            .expect("Failed to read tmp char def file");
        let cat = CharacterCategory::compile(ranges);

        assert_eq!(
            cats_num.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0030).unwrap())
        );
        assert_eq!(
            cats_num.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0031).unwrap())
        );
        assert_eq!(
            cats_kanji_num.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0032).unwrap())
        );
        assert_eq!(
            cats_num.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0033).unwrap())
        );
        assert_eq!(
            cats_num.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0039).unwrap())
        );

        // 2.
        let mut file = tempfile().expect("Failed to get temporary file");
        writeln!(file, "#\n ").unwrap();
        writeln!(file, "0x0030..0x0039 NUMERIC").unwrap();
        writeln!(file, "0x0070..0x0079 ALPHA").unwrap();
        writeln!(file, "0x3007         KANJI").unwrap();
        writeln!(file, "0x0030         KANJI").unwrap();
        file.flush().expect("Failed to flush");
        file.seek(SeekFrom::Start(0)).expect("Failed to seek");
        let ranges = CharacterCategory::read_character_definition(BufReader::new(file))
            .expect("Failed to read tmp char def file");
        let cat = CharacterCategory::compile(ranges);

        assert_eq!(
            cats_kanji_num.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0030).unwrap())
        );
        assert_eq!(
            cats_num.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0039).unwrap())
        );
        assert_eq!(
            cats_kanji.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x3007).unwrap())
        );
        assert_eq!(
            cats_default.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0069).unwrap())
        );
        assert_eq!(
            cats_alp.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0070).unwrap())
        );
        assert_eq!(
            cats_default.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0080).unwrap())
        );

        // 3.
        let mut file = tempfile().expect("Failed to get temporary file");
        writeln!(file, "#\n ").unwrap();
        writeln!(file, "0x0030..0x0039 KATAKANA").unwrap();
        writeln!(file, "0x3007         KANJI KANJINUMERIC").unwrap();
        writeln!(file, "0x3008         KANJI KANJINUMERIC").unwrap();
        writeln!(file, "0x3009         KANJI KANJINUMERIC").unwrap();
        writeln!(file, "0x0039..0x0040 ALPHA").unwrap();
        writeln!(file, "0x0030..0x0039 NUMERIC").unwrap();
        writeln!(file, "0x0030         KANJI").unwrap();
        file.flush().expect("Failed to flush");
        file.seek(SeekFrom::Start(0)).expect("Failed to seek");
        let ranges = CharacterCategory::read_character_definition(BufReader::new(file))
            .expect("Failed to read tmp char def file");
        let cat = CharacterCategory::compile(ranges);

        assert_eq!(
            cats_default.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0029).unwrap())
        );
        assert_eq!(
            cats_num_kanji_kana
                .iter()
                .map(|v| *v)
                .collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0030).unwrap())
        );
        assert_eq!(
            cats_num_alpha_kana
                .iter()
                .map(|v| *v)
                .collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0039).unwrap())
        );
        assert_eq!(
            cats_alp.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0040).unwrap())
        );
        assert_eq!(
            cats_default.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x0041).unwrap())
        );
        assert_eq!(
            cats_kanjinumeric
                .iter()
                .map(|v| *v)
                .collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x3007).unwrap())
        );
        assert_eq!(
            cats_default.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x4007).unwrap())
        );

        // 4.
        let mut file = tempfile().expect("Failed to get temporary file");
        writeln!(file, "#\n ").unwrap();
        writeln!(file, "0x4E00..0x9FFF KANJI").unwrap();
        writeln!(file, "0x4E8C         KANJI KANJINUMERIC").unwrap();
        file.flush().expect("Failed to flush");
        file.seek(SeekFrom::Start(0)).expect("Failed to seek");
        let ranges = CharacterCategory::read_character_definition(BufReader::new(file))
            .expect("Failed to read tmp char def file");
        let cat = CharacterCategory::compile(ranges);

        assert_eq!(
            cats_kanji.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types('男')
        );
        assert_eq!(
            cats_kanji.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x4E8B).unwrap())
        );
        assert_eq!(
            cats_kanjinumeric
                .iter()
                .map(|v| *v)
                .collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x4E8C).unwrap())
        );
        assert_eq!(
            cats_kanji.iter().map(|v| *v).collect::<CategoryTypes>(),
            cat.get_category_types(char::from_u32(0x4E8D).unwrap())
        );
    }

    #[test]
    #[should_panic]
    fn read_character_definition_with_invalid_format() {
        let mut file = tempfile().expect("Failed to get temporary file");
        writeln!(file, "0x0030..0x0039").unwrap();
        file.flush().expect("Failed to flush");
        file.seek(SeekFrom::Start(0)).expect("Failed to seek");
        let _ranges = CharacterCategory::read_character_definition(BufReader::new(file))
            .expect("Failed to read tmp char def file");
    }

    #[test]
    #[should_panic]
    fn read_character_definition_with_invalid_range() {
        let mut file = tempfile().expect("Failed to get temporary file");
        writeln!(file, "0x0030..0x0029 NUMERIC").unwrap();
        file.flush().expect("Failed to flush");
        file.seek(SeekFrom::Start(0)).expect("Failed to seek");
        let _ranges = CharacterCategory::read_character_definition(BufReader::new(file))
            .expect("Failed to read tmp char def file");
    }

    #[test]
    #[should_panic]
    fn read_character_definition_with_invalid_type() {
        let mut file = tempfile().expect("Failed to get temporary file");
        writeln!(file, "0x0030..0x0039 FOO").unwrap();
        file.flush().expect("Failed to flush");
        file.seek(SeekFrom::Start(0)).expect("Failed to seek");
        let _ranges = CharacterCategory::read_character_definition(BufReader::new(file))
            .expect("Failed to read tmp char def file");
    }
}
