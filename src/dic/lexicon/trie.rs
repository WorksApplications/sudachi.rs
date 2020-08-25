use crate::prelude::*;

pub struct Trie {
    array: Vec<u32>,
    size: u32, // number of elements
}

impl Trie {
    pub fn new(array: Vec<u32>, size: u32) -> Trie {
        Trie { array, size }
    }

    pub fn total_size(&self) -> usize {
        4 * self.size as usize
    }

    pub fn common_prefix_search(
        &self,
        input: &[u8],
        offset: usize,
    ) -> SudachiResult<Vec<(usize, usize)>> {
        let mut result = Vec::new();

        let mut node_pos: usize = 0;
        let mut unit: usize = *self
            .array
            .get(node_pos)
            .ok_or(SudachiError::MissingDictionaryTrie)? as usize;
        node_pos ^= Trie::offset(unit);

        for i in offset..input.len() {
            let k = input.get(i).ok_or(SudachiError::MissingDictionaryTrie)?;
            node_pos ^= *k as usize;
            unit = *self
                .array
                .get(node_pos)
                .ok_or(SudachiError::MissingDictionaryTrie)? as usize;
            if Trie::label(unit) != *k as usize {
                return Ok(result);
            }

            node_pos ^= Trie::offset(unit);
            if Trie::has_leaf(unit) {
                let r = (
                    Trie::value(
                        *self
                            .array
                            .get(node_pos)
                            .ok_or(SudachiError::MissingDictionaryTrie)?
                            as usize,
                    ),
                    i + 1,
                );
                result.push(r);
            }
        }

        Ok(result)
    }

    fn has_leaf(unit: usize) -> bool {
        ((unit >> 8) & 1) == 1
    }

    fn value(unit: usize) -> usize {
        unit & ((1 << 31) - 1)
    }

    fn label(unit: usize) -> usize {
        unit & ((1 << 31) | 0xFF)
    }

    fn offset(unit: usize) -> usize {
        (unit >> 10) << ((unit & (1 << 9)) >> 6)
    }
}
