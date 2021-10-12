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

use std::hash::{BuildHasher, Hasher};

const MULTIPLIER: u64 = 0x6eed0e9da4d94a4fu64;
const SEED: u64 = 0x16f11fe89b0d677cu64;

/// Ro(tate) + Mu(ltiply) Hasher Factory
pub struct RoMu {}

impl RoMu {
    pub fn new() -> RoMu {
        RoMu {}
    }
}

impl Default for RoMu {
    fn default() -> Self {
        RoMu::new()
    }
}

impl BuildHasher for RoMu {
    type Hasher = RoMuHash;

    #[inline(always)]
    fn build_hasher(&self) -> Self::Hasher {
        RoMuHash::new()
    }
}

pub struct RoMuHash {
    state: u64,
}

// from https://github.com/ku-nlp/jumanpp/blob/master/src/util/fast_hash_rot.h
// It is very fast (xor+mul+rot) for extremely small values (e.g. 1 field)
impl RoMuHash {
    #[inline(always)]
    pub fn new() -> RoMuHash {
        RoMuHash { state: SEED }
    }

    #[inline(always)]
    fn consume(&mut self, value: u64) {
        let data = self.state ^ value;
        let data = data.wrapping_mul(MULTIPLIER);
        self.state = data.rotate_left(32);
    }
}

impl Hasher for RoMuHash {
    #[inline(always)]
    fn finish(&self) -> u64 {
        self.state
    }

    #[inline(always)]
    fn write(&mut self, _bytes: &[u8]) {
        panic!("not supported for bytes")
    }

    #[inline(always)]
    fn write_u8(&mut self, _: u8) {
        panic!("not supported for u8")
    }

    #[inline(always)]
    fn write_u16(&mut self, _: u16) {
        panic!("not supported for u16")
    }

    #[inline(always)]
    fn write_u32(&mut self, i: u32) {
        self.consume(i as u64);
    }

    #[inline(always)]
    fn write_u64(&mut self, i: u64) {
        self.consume(i as u64);
    }

    #[inline(always)]
    fn write_u128(&mut self, _: u128) {
        panic!("not supported for u128")
    }

    #[inline(always)]
    fn write_usize(&mut self, i: usize) {
        self.consume(i as u64);
    }

    #[inline(always)]
    fn write_i8(&mut self, _: i8) {
        panic!("not supported for i8")
    }

    #[inline(always)]
    fn write_i16(&mut self, _: i16) {
        panic!("not supported for i16")
    }

    #[inline(always)]
    fn write_i32(&mut self, i: i32) {
        self.consume(i as u64)
    }

    #[inline(always)]
    fn write_i64(&mut self, i: i64) {
        self.consume(i as u64)
    }

    #[inline(always)]
    fn write_i128(&mut self, _: i128) {
        panic!("not supported for i128")
    }

    #[inline(always)]
    fn write_isize(&mut self, i: isize) {
        self.consume(i as u64)
    }
}

#[cfg(test)]
mod test {
    use crate::hash::RoMu;
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};

    #[derive(Eq, PartialEq)]
    struct Small(i32, i32);

    impl Hash for Small {
        fn hash<H: Hasher>(&self, state: &mut H) {
            state.write_u64((self.0 as u64) << 32 | (self.1 as u64))
        }
    }

    #[test]
    fn works_in_hashmap() {
        let mut map = HashMap::with_hasher(RoMu::new());
        map.insert(Small(5, 6), "data");
        map.insert(Small(6, 5), "data2");
        assert_eq!(*map.get(&Small(5, 6)).unwrap(), "data");
        assert!(!map.contains_key(&Small(0, 0)));
    }
}
