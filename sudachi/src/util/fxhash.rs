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

// Adopted from: https://github.com/cbreeden/fxhash/blob/master/lib.rs
// Removed functionality that were useless for us
// Original Copyright:

// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Fx Hash
//!
//! This hashing algorithm was extracted from the Rustc compiler.  This is the same hashing
//! algorithm used for some internal operations in Firefox.  The strength of this algorithm
//! is in hashing 8 bytes at a time on 64-bit platforms, where the FNV algorithm works on one
//! byte at a time.
//!
//! ## Disclaimer
//!
//! It is **not a cryptographically secure** hash, so it is strongly recommended that you do
//! not use this hash for cryptographic purproses.  Furthermore, this hashing algorithm was
//! not designed to prevent any attacks for determining collisions which could be used to
//! potentially cause quadratic behavior in `HashMap`s.  So it is not recommended to expose
//! this hash in places where collissions or DDOS attacks may be a concern.

use std::convert::TryInto;
use std::default::Default;
use std::hash::{BuildHasherDefault, Hasher};
use std::ops::BitXor;

/// A builder for default Fx hashers.
pub type FxBuildHasher = BuildHasherDefault<FxHasher64>;

const ROTATE: u32 = 5;
const SEED64: u64 = 0x51_7c_c1_b7_27_22_0a_95;
const SEED32: u32 = 0x9e_37_79_b9;

#[cfg(target_pointer_width = "64")]
const SEED: usize = SEED64 as usize;

trait HashWord {
    fn hash_word(&mut self, _: Self);
}

macro_rules! impl_hash_word {
    ($($ty:ty = $key:ident),* $(,)*) => (
        $(
            impl HashWord for $ty {
                #[inline(always)]
                fn hash_word(&mut self, word: Self) {
                    *self = self.rotate_left(ROTATE).bitxor(word).wrapping_mul($key);
                }
            }
        )*
    )
}

impl_hash_word!(usize = SEED, u32 = SEED32, u64 = SEED64);

#[inline]
fn write64(mut hash: u64, mut bytes: &[u8]) -> u64 {
    while bytes.len() >= 8 {
        let (data, rem) = bytes.split_at(8);
        hash.hash_word(u64::from_ne_bytes(data.try_into().unwrap()));
        bytes = rem;
    }

    if bytes.len() >= 4 {
        let (data, rem) = bytes.split_at(4);
        hash.hash_word(u32::from_ne_bytes(data.try_into().unwrap()) as u64);
        bytes = rem;
    }

    if bytes.len() >= 2 {
        let (data, rem) = bytes.split_at(2);
        hash.hash_word(u16::from_ne_bytes(data.try_into().unwrap()) as u64);
        bytes = rem;
    }

    if let Some(&byte) = bytes.first() {
        hash.hash_word(u64::from(byte));
    }

    hash
}

/// This hashing algorithm was extracted from the Rustc compiler.
/// This is the same hashing algorithm used for some internal operations in Firefox.
/// The strength of this algorithm is in hashing 8 bytes at a time on any platform,
/// where the FNV algorithm works on one byte at a time.
///
/// This hashing algorithm should not be used for cryptographic, or in scenarios where
/// DOS attacks are a concern.
#[derive(Debug, Clone)]
pub struct FxHasher64 {
    hash: u64,
}

impl Default for FxHasher64 {
    #[inline]
    fn default() -> FxHasher64 {
        FxHasher64 { hash: 0 }
    }
}

impl Hasher for FxHasher64 {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.hash = write64(self.hash, bytes);
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.hash.hash_word(u64::from(i));
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.hash.hash_word(u64::from(i));
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.hash.hash_word(u64::from(i));
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.hash.hash_word(i);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.hash.hash_word(i as u64);
    }
}
