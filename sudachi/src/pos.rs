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

use crate::util::fxhash::FxBuildHasher;
use std::collections::HashSet;

pub struct PosMatcher {
    ids: HashSet<u16, FxBuildHasher>,
}

impl PosMatcher {
    pub fn new<I: IntoIterator<Item = u16>>(pos: I) -> Self {
        let iter = pos.into_iter();
        let (min_size, max_size) = iter.size_hint();
        let size = max_size.unwrap_or(min_size);
        let mut ids = HashSet::with_capacity_and_hasher(size, FxBuildHasher::default());
        ids.extend(iter);
        Self { ids }
    }

    #[inline]
    pub fn matches_id(&self, pos_id: u16) -> bool {
        self.ids.contains(&pos_id)
    }

    pub fn num_entries(&self) -> usize {
        self.ids.len()
    }

    pub fn entries(&self) -> impl Iterator<Item = u16> + '_ {
        self.ids.iter().cloned()
    }
}
