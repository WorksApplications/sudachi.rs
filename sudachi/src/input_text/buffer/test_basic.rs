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

use super::*;
use crate::test::zero_grammar;

#[test]
fn new_build() {
    let mut buffer = InputBuffer::new();
    buffer.reset().push_str("宇宙人");
    buffer.start_build().expect("does not fail");
    assert_eq!(buffer.current(), "宇宙人")
}

#[test]
fn curr_slice_c() {
    let mut buffer = InputBuffer::from("宇宙人");
    buffer.build(&zero_grammar()).expect("built");
    assert_eq!(buffer.curr_slice_c(0..1), "宇");
    assert_eq!(buffer.curr_slice_c(1..2), "宙");
    assert_eq!(buffer.curr_slice_c(2..3), "人");
}

#[test]
fn orig_slice() {
    let buffer = InputBuffer::from("宇宙人");
    assert_eq!(buffer.orig_slice(0..3), "宇");
    assert_eq!(buffer.orig_slice(3..6), "宙");
    assert_eq!(buffer.orig_slice(6..9), "人");
}

#[test]
fn char_distance() {
    let mut buffer = InputBuffer::from("宇宙人");
    let g = zero_grammar();
    buffer.build(&g).expect("failed");
    assert_eq!(1, buffer.char_distance(0, 1));
    assert_eq!(2, buffer.char_distance(0, 2));
    assert_eq!(3, buffer.char_distance(0, 3));
    // this returns result to last character if out of bounds
    assert_eq!(3, buffer.char_distance(0, 4));
}

#[test]
fn allows_0_chars_start() {
    let mut buffer = InputBuffer::from("\0宇宙人");
    let g = zero_grammar();
    buffer.build(&g).expect("failed");
    assert_eq!(buffer.modified, "\0宇宙人")
}

#[test]
fn allows_0_chars_mid() {
    let mut buffer = InputBuffer::from("宇宙\0人");
    let g = zero_grammar();
    buffer.build(&g).expect("failed");
    assert_eq!(buffer.modified, "宇宙\0人")
}

#[test]
fn allows_0_chars_end() {
    let mut buffer = InputBuffer::from("宇宙人\0");
    let g = zero_grammar();
    buffer.build(&g).expect("failed");
    assert_eq!(buffer.modified, "宇宙人\0")
}
