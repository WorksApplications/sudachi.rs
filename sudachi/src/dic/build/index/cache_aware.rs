/*
 *  Copyright (c) 2026 Works Applications Co., Ltd.
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

use super::{CacheAwareOptions, TrieProfileMode, MAX_TRIE_VALUE};
use crate::dic::build::error::BuildFailure;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use yada::unit::Unit;

const BLOCK_SIZE: usize = 256;
const MAX_OFFSET: u32 = 1 << 29;
const MAX_EXTENDED_OFFSET_LOW_BITS: u32 = 0xff;

pub(super) struct CacheAwareDartsBuilder;

impl CacheAwareDartsBuilder {
    pub(super) fn build<T>(
        keyset: &[(T, u32)],
        options: CacheAwareOptions,
    ) -> Result<Vec<u8>, BuildFailure>
    where
        T: AsRef<[u8]>,
    {
        for (key, value) in keyset {
            if *value > MAX_TRIE_VALUE {
                return Err(BuildFailure::TrieValueLimitExceeded {
                    entry: String::from_utf8_lossy(key.as_ref()).into_owned(),
                    value: *value,
                });
            }
        }

        let child_weights = build_child_weights(keyset, &options.profile_mode)?;

        let mut state = BuilderState::new(options, child_weights);
        state.reserve(0)?;
        if !keyset.is_empty() {
            state.build_recursive(keyset, 0, 0, keyset.len(), 0)?;
        }
        Ok(state.into_bytes())
    }
}

struct BuilderState {
    options: CacheAwareOptions,
    units: Vec<Unit>,
    occupied: Vec<bool>,
    block_occupied: Vec<u16>,
    used_offsets: HashSet<u32>,
    child_weights: ChildWeights,
}

impl BuilderState {
    fn new(options: CacheAwareOptions, child_weights: ChildWeights) -> Self {
        let mut state = Self {
            options,
            units: Vec::new(),
            occupied: Vec::new(),
            block_occupied: Vec::new(),
            used_offsets: HashSet::new(),
            child_weights,
        };
        state.extend_block();
        state
    }

    fn build_recursive<T>(
        &mut self,
        keyset: &[(T, u32)],
        depth: usize,
        begin: usize,
        end: usize,
        unit_id: usize,
    ) -> Result<(), BuildFailure>
    where
        T: AsRef<[u8]>,
    {
        let mut children = collect_children(keyset, &self.child_weights, depth, begin, end)?;
        children.sort_by(hot_first);

        let offset = self.find_offset(unit_id, &children)?;
        let has_leaf = children.iter().any(|child| child.label == 0);
        {
            let parent = self
                .units
                .get_mut(unit_id)
                .ok_or(BuildFailure::TrieBuildFailure)?;
            if parent.offset() != 0 {
                return Err(BuildFailure::TrieBuildFailure);
            }
            parent.set_offset(offset ^ unit_id as u32);
            parent.set_has_leaf(has_leaf);
        }

        for child in &children {
            let child_id = (offset ^ child.label as u32) as usize;
            self.reserve(child_id)?;
            let unit = self
                .units
                .get_mut(child_id)
                .ok_or(BuildFailure::TrieBuildFailure)?;
            if unit.offset() != 0 || unit.label() != 0 || unit.value() != 0 || unit.has_leaf() {
                return Err(BuildFailure::TrieBuildFailure);
            }
            if child.label == 0 {
                let value = child.value.ok_or(BuildFailure::TrieBuildFailure)?;
                unit.set_value(value);
            } else {
                unit.set_label(child.label);
            }
        }

        for child in children {
            if child.label == 0 {
                continue;
            }
            let child_id = (offset ^ child.label as u32) as usize;
            self.build_recursive(keyset, depth + 1, child.begin, child.end, child_id)?;
        }

        Ok(())
    }

    fn reserve(&mut self, unit_id: usize) -> Result<(), BuildFailure> {
        while unit_id >= self.units.len() {
            self.extend_block();
        }
        if self.occupied[unit_id] {
            return Err(BuildFailure::TrieBuildFailure);
        }
        self.occupied[unit_id] = true;
        self.block_occupied[unit_id / BLOCK_SIZE] += 1;
        Ok(())
    }

    fn extend_block(&mut self) {
        self.units
            .resize(self.units.len() + BLOCK_SIZE, Unit::new());
        self.occupied
            .resize(self.occupied.len() + BLOCK_SIZE, false);
        self.block_occupied.push(0);
    }

    fn find_offset(&mut self, unit_id: usize, children: &[Child]) -> Result<u32, BuildFailure> {
        loop {
            if let Some(offset) = self.find_offset_in_existing_blocks(unit_id, children) {
                self.used_offsets.insert(offset);
                return Ok(offset);
            }
            self.extend_block();
            if self.units.len() / BLOCK_SIZE >= (MAX_OFFSET as usize / BLOCK_SIZE) {
                return Err(BuildFailure::TrieBuildFailure);
            }
        }
    }

    fn find_offset_in_existing_blocks(&self, unit_id: usize, children: &[Child]) -> Option<u32> {
        let mut best: Option<Candidate> = None;
        for block_id in self.candidate_blocks(unit_id) {
            if self.block_occupied[block_id] as usize == BLOCK_SIZE {
                continue;
            }
            let block_base = block_id * BLOCK_SIZE;
            for low in 0..BLOCK_SIZE {
                let offset = (block_base + low) as u32;
                if !self.is_valid_offset(unit_id, offset, children) {
                    continue;
                }
                let score = self.score_offset(unit_id, offset, children);
                let candidate = Candidate {
                    offset,
                    block_id,
                    score,
                };
                if best.as_ref().map_or(true, |current| candidate < *current) {
                    best = Some(candidate);
                }
            }
        }
        best.map(|candidate| candidate.offset)
    }

    fn candidate_blocks(&self, unit_id: usize) -> Vec<usize> {
        let mut blocks = Vec::new();
        let parent_block = unit_id / BLOCK_SIZE;
        if parent_block < self.block_occupied.len() {
            blocks.push(parent_block);
        }

        let window = self.options.candidate_window_blocks.max(1);
        let start = self.block_occupied.len().saturating_sub(window);
        for block_id in start..self.block_occupied.len() {
            if !blocks.contains(&block_id) {
                blocks.push(block_id);
            }
        }
        blocks
    }

    fn is_valid_offset(&self, unit_id: usize, offset: u32, children: &[Child]) -> bool {
        if offset >= MAX_OFFSET || self.used_offsets.contains(&offset) {
            return false;
        }

        let relative_offset = unit_id as u32 ^ offset;
        if relative_offset >= MAX_OFFSET {
            return false;
        }
        if relative_offset >= (1 << 21) && (relative_offset & MAX_EXTENDED_OFFSET_LOW_BITS) != 0 {
            return false;
        }

        children.iter().all(|child| {
            let child_id = (offset ^ child.label as u32) as usize;
            self.occupied
                .get(child_id)
                .is_some_and(|is_occupied| !*is_occupied)
        })
    }

    fn score_offset(&self, unit_id: usize, offset: u32, children: &[Child]) -> u128 {
        let line_units = (self.options.cache_line_bytes / std::mem::size_of::<u32>()).max(1);
        let parent_line = unit_id / line_units;
        let mut cache_lines = Vec::with_capacity(children.len());
        let mut weighted_distance = 0u128;
        let mut min_pos = usize::MAX;
        let mut max_pos = 0usize;

        for child in children {
            let pos = (offset ^ child.label as u32) as usize;
            let line = pos / line_units;
            if !cache_lines.contains(&line) {
                cache_lines.push(line);
            }
            let distance = line.abs_diff(parent_line) as u128;
            weighted_distance += distance * child.weight as u128;
            min_pos = min_pos.min(pos);
            max_pos = max_pos.max(pos);
        }

        let block_id = offset as usize / BLOCK_SIZE;
        let occupied = self.block_occupied[block_id] as u128;
        let free_after = (BLOCK_SIZE as u128)
            .saturating_sub(occupied)
            .saturating_sub(children.len() as u128);
        let spread = max_pos.saturating_sub(min_pos) as u128;

        cache_lines.len() as u128 * self.options.scoring.cache_line_weight as u128
            + weighted_distance * self.options.scoring.distance_weight as u128
            + spread * self.options.scoring.spread_weight as u128
            + free_after * self.options.scoring.density_weight as u128
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.units.len() * std::mem::size_of::<u32>());
        for unit in self.units {
            bytes.extend_from_slice(&unit.as_u32().to_le_bytes());
        }
        bytes
    }
}

#[derive(Debug)]
struct Child {
    label: u8,
    begin: usize,
    end: usize,
    value: Option<u32>,
    weight: u64,
    entries: usize,
}

fn collect_children<T>(
    keyset: &[(T, u32)],
    child_weights: &ChildWeights,
    depth: usize,
    begin: usize,
    end: usize,
) -> Result<Vec<Child>, BuildFailure>
where
    T: AsRef<[u8]>,
{
    let mut children = Vec::with_capacity(16);
    let mut i = begin;
    while i < end {
        let label = label_at(keyset[i].0.as_ref(), depth)?;
        let child_begin = i;
        let mut value = None;
        if label == 0 {
            value = Some(keyset[i].1);
            i += 1;
            if i < end && label_at(keyset[i].0.as_ref(), depth)? == 0 {
                return Err(BuildFailure::TrieBuildFailure);
            }
        } else {
            while i < end && label_at(keyset[i].0.as_ref(), depth)? == label {
                i += 1;
            }
        }
        let weight = child_weights.child_weight(child_begin, i);
        let entries = child_weights.child_entries(child_begin, i);
        children.push(Child {
            label,
            begin: child_begin,
            end: i,
            value,
            weight,
            entries,
        });
    }
    Ok(children)
}

fn label_at(key: &[u8], depth: usize) -> Result<u8, BuildFailure> {
    if depth == key.len() {
        Ok(0)
    } else {
        key.get(depth)
            .copied()
            .ok_or(BuildFailure::TrieBuildFailure)
    }
}

fn hot_first(left: &Child, right: &Child) -> std::cmp::Ordering {
    right
        .weight
        .cmp(&left.weight)
        .then_with(|| right.entries.cmp(&left.entries))
        .then_with(|| (right.label == 0).cmp(&(left.label == 0)))
        .then_with(|| left.label.cmp(&right.label))
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Candidate {
    offset: u32,
    block_id: usize,
    score: u128,
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score
            .cmp(&other.score)
            .then_with(|| self.block_id.cmp(&other.block_id))
            .then_with(|| self.offset.cmp(&other.offset))
    }
}

enum ChildWeights {
    Uniform,
    PrefixCount,
    ExternalProfile(Vec<u64>),
}

impl ChildWeights {
    fn child_weight(&self, begin: usize, end: usize) -> u64 {
        match self {
            Self::Uniform => 1,
            Self::PrefixCount => (end - begin) as u64,
            Self::ExternalProfile(weight_prefix) => {
                weight_prefix[end].saturating_sub(weight_prefix[begin])
            }
        }
    }

    fn child_entries(&self, begin: usize, end: usize) -> usize {
        match self {
            Self::Uniform => 1,
            Self::PrefixCount | Self::ExternalProfile(_) => end - begin,
        }
    }
}

fn build_child_weights<T>(
    keyset: &[(T, u32)],
    profile_mode: &TrieProfileMode,
) -> Result<ChildWeights, BuildFailure>
where
    T: AsRef<[u8]>,
{
    match profile_mode {
        TrieProfileMode::Uniform => Ok(ChildWeights::Uniform),
        TrieProfileMode::DictionaryPrefixCount => Ok(ChildWeights::PrefixCount),
        TrieProfileMode::ExternalKeyProfile(path) => {
            let profile = read_external_profile(path)?;
            let weights = keyset
                .iter()
                .map(|(key, _)| profile.get(key.as_ref()).copied().unwrap_or(1))
                .collect();
            Ok(ChildWeights::ExternalProfile(weight_prefix(weights)))
        }
    }
}

fn weight_prefix(weights: Vec<u64>) -> Vec<u64> {
    let mut prefix = Vec::with_capacity(weights.len() + 1);
    prefix.push(0);
    for weight in weights {
        let next = prefix
            .last()
            .copied()
            .unwrap_or(0u64)
            .saturating_add(weight);
        prefix.push(next);
    }
    prefix
}

fn read_external_profile(path: &std::path::Path) -> Result<HashMap<Vec<u8>, u64>, BuildFailure> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut result = HashMap::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (key, weight) = line.split_once('\t').ok_or_else(|| {
            BuildFailure::InvalidTrieProfile(format!(
                "{}:{}: expected <surface-or-hex>\\t<count>",
                path.display(),
                line_no + 1
            ))
        })?;
        let weight = weight.trim().parse::<u64>().map_err(|e| {
            BuildFailure::InvalidTrieProfile(format!(
                "{}:{}: invalid count: {}",
                path.display(),
                line_no + 1,
                e
            ))
        })?;
        let key = parse_profile_key(key).map_err(|e| {
            BuildFailure::InvalidTrieProfile(format!("{}:{}: {}", path.display(), line_no + 1, e))
        })?;
        result.insert(key, weight);
    }
    Ok(result)
}

fn parse_profile_key(raw: &str) -> Result<Vec<u8>, &'static str> {
    let Some(hex) = raw.strip_prefix("hex:") else {
        if raw.is_empty() {
            return Err("empty key");
        }
        return Ok(raw.as_bytes().to_vec());
    };
    if hex.is_empty() || hex.len() % 2 != 0 {
        return Err("hex key must contain an even number of digits");
    }
    let mut result = Vec::with_capacity(hex.len() / 2);
    for chunk in hex.as_bytes().chunks_exact(2) {
        let high = decode_hex_digit(chunk[0])?;
        let low = decode_hex_digit(chunk[1])?;
        result.push((high << 4) | low);
    }
    Ok(result)
}

fn decode_hex_digit(byte: u8) -> Result<u8, &'static str> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("hex key contains a non-hex digit"),
    }
}
