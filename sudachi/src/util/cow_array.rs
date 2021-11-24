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

use std::array::TryFromSliceError;
use std::convert::TryInto;
use std::ops::Deref;

pub trait ReadLE {
    fn from_le_bytes(bytes: &[u8]) -> Result<Self, TryFromSliceError>
    where
        Self: Sized;
}

impl ReadLE for i16 {
    fn from_le_bytes(bytes: &[u8]) -> Result<Self, TryFromSliceError> {
        bytes.try_into().map(|b| Self::from_le_bytes(b))
    }
}

impl ReadLE for u32 {
    fn from_le_bytes(bytes: &[u8]) -> Result<Self, TryFromSliceError>
    where
        Self: Sized,
    {
        bytes.try_into().map(|b| Self::from_le_bytes(b))
    }
}

/// Copy-on-write array.
///
/// Is used for storing performance critical dictionary parts.
/// `slice` is always valid, `storage` is used in owned mode.
/// Unfortunately, `Cow<&[T]>` does not equal to `&[T]` in assembly:
/// See: https://rust.godbolt.org/z/r4a9efjqh
///
/// It implements Deref for `&[T]`, so it can be used as slice.
pub struct CowArray<'a, T> {
    slice: &'a [T],
    storage: Option<Vec<T>>,
}

impl<T: ReadLE + Clone> CowArray<'static, T> {
    /// Creates from the owned data
    pub fn from_owned<D: Into<Vec<T>>>(data: D) -> Self {
        let data = data.into();
        let slice1: &[T] = &data;
        let slice: &'static [T] = unsafe { std::mem::transmute(slice1) };
        Self {
            storage: Some(data),
            slice,
        }
    }
}

impl<'a, T: ReadLE + Clone> CowArray<'a, T> {
    /// Create the CowArray from bytes, reinterpreting bytes as T.
    ///
    /// Original data may or not be aligned.
    /// In non-aligned case, it makes a copy of the original data.
    pub fn from_bytes(data: &'a [u8], offset: usize, size: usize) -> Self {
        let align = std::mem::align_of::<T>();

        let real_size = size * std::mem::size_of::<T>();
        let real_slice = &data[offset..offset + real_size];
        let ptr = real_slice.as_ptr() as *const T;
        if is_aligned(ptr as usize, align) {
            // SAFETY: ptr is aligned and trait bounds are ensuring so the type is sane
            let reslice = unsafe { std::slice::from_raw_parts(ptr, size) };
            Self {
                slice: reslice,
                storage: None,
            }
        } else {
            let data = copy_of_bytes::<T>(real_slice);
            let slice_1: &[T] = data.as_slice();
            // we need transmute to make correct lifetime
            // slice will always point to vector contents and it is impossible to have
            // self-referential types in Rust yet
            let slice: &'a [T] = unsafe { std::mem::transmute(slice_1) };
            Self {
                storage: Some(data),
                slice,
            }
        }
    }

    /// Updates the value of the array
    ///
    /// Copies the data array if needed and updates it in place
    /// Current implementation is not super for Rust because it
    /// violates borrowing rules, but
    /// 1. this object does not expose any references outside
    /// 2. usage of data still follows the pattern 1-mut xor many-read
    pub fn set(&mut self, offset: usize, value: T) {
        if self.storage.is_none() {
            self.storage = Some(self.slice.to_vec());
            //refresh slice
            let slice: &[T] = self.storage.as_ref().unwrap().as_slice();
            self.slice = unsafe { std::mem::transmute(slice) };
        }
        self.storage.as_mut().map(|s| s[offset] = value);
    }
}

impl<'a, T> Deref for CowArray<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

fn is_aligned(offset: usize, alignment: usize) -> bool {
    debug_assert!(alignment.is_power_of_two());
    offset % alignment == 0
}

fn copy_of_bytes<T: ReadLE>(data: &[u8]) -> Vec<T> {
    let size_t = std::mem::size_of::<T>();
    assert_eq!(data.len() % size_t, 0);
    let nelems = data.len() / size_t;
    let mut result = Vec::with_capacity(nelems);
    for i in (0..data.len()).step_by(size_t) {
        let sl = &data[i..i + size_t];
        result.push(T::from_le_bytes(sl).unwrap());
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn aligned_1() {
        assert!(is_aligned(0, 1));
        assert!(is_aligned(1, 1));
        assert!(is_aligned(2, 1));
        assert!(is_aligned(3, 1));
        assert!(is_aligned(4, 1));
        assert!(is_aligned(5, 1));
        assert!(is_aligned(6, 1));
        assert!(is_aligned(7, 1));
        assert!(is_aligned(8, 1));
    }

    #[test]
    fn aligned_2() {
        assert!(is_aligned(0, 2));
        assert!(!is_aligned(1, 2));
        assert!(is_aligned(2, 2));
        assert!(!is_aligned(3, 2));
        assert!(is_aligned(4, 2));
        assert!(!is_aligned(5, 2));
        assert!(is_aligned(6, 2));
        assert!(!is_aligned(7, 2));
        assert!(is_aligned(8, 2));
    }

    #[test]
    fn aligned_4() {
        assert!(is_aligned(0, 4));
        assert!(!is_aligned(1, 4));
        assert!(!is_aligned(2, 4));
        assert!(!is_aligned(3, 4));
        assert!(is_aligned(4, 4));
        assert!(!is_aligned(5, 4));
        assert!(!is_aligned(6, 4));
        assert!(!is_aligned(7, 4));
        assert!(is_aligned(8, 4));
    }
}
