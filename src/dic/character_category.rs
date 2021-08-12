use multiset::HashMultiSet;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::iter::FromIterator;
use std::u32;
use thiserror::Error;

use crate::dic::category_type::{CategoryType, CategoryTypes};
use crate::prelude::*;

const DEFAULT_CHAR_DEF_FILE_PATH: &str = "./src/resources/char.def";

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
pub struct Range {
    begin: u32,
    end: u32,
    categories: CategoryTypes,
}

impl Range {
    pub fn contains(&self, cp: u32) -> bool {
        self.begin <= cp && cp < self.end
    }
    pub fn lower(&self, cp: u32) -> bool {
        self.end <= cp
    }
    pub fn higher(&self, cp: u32) -> bool {
        cp < self.begin
    }
}

#[derive(Debug, Clone)]
pub struct CharacterCategory {
    ranges: Vec<Range>,
}

impl CharacterCategory {
    pub fn from_file(path: Option<&str>) -> SudachiResult<CharacterCategory> {
        let ranges = CharacterCategory::read_character_definition(path)?;
        Ok(CharacterCategory::compile(ranges))
    }

    fn read_character_definition(path: Option<&str>) -> SudachiResult<Vec<Range>> {
        let path = path.unwrap_or(DEFAULT_CHAR_DEF_FILE_PATH);
        let reader = BufReader::new(fs::File::open(&path)?);

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

    fn compile(mut ranges: Vec<Range>) -> CharacterCategory {
        /// compile transforms given range_list to non overlapped range list
        /// to apply binary search in get_category_types

        /// implement order for Heap
        /// note that here we use min-heap, based on the end of range
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

    pub fn get_category_types(&self, c: char) -> CategoryTypes {
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
    const TEST_RESOURCE_DIR: &str = "./tests/resources/";

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
        // todo: pass correct path
        let path = TEST_RESOURCE_DIR;
        let cat =
            CharacterCategory::from_file(Some(path)).expect("failed to load char.def for test");
        let cats = cat.get_category_types('熙');
        assert_eq!(1, cats.len());
        assert!(cats.contains(&CategoryType::KANJI));
    }

    #[test]
    fn read_character_definition() {
        // todo: immpl after file read
    }

    #[test]
    fn read_character_definition_with_invalid_format() {
        // todo: immpl after file read
    }

    #[test]
    fn read_character_definition_with_invalid_range() {
        // todo: immpl after file read
    }

    #[test]
    fn read_character_definition_with_invalid_type() {
        // todo: immpl after file read
    }
}
