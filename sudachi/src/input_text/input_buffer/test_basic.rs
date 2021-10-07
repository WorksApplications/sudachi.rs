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

#[test]
fn new_build() {
    let mut buffer = InputBuffer::new();
    buffer.reset().push_str("宇宙人");
    buffer.start_build();
    assert_eq!(buffer.current(), "宇宙人")
}

#[test]
fn curr_slice() {
    let buffer = InputBuffer::from("宇宙人");
    assert_eq!(buffer.curr_slice(0..3), "宇");
    assert_eq!(buffer.curr_slice(3..6), "宙");
    assert_eq!(buffer.curr_slice(6..9), "人");
}

#[test]
fn orig_slice() {
    let buffer = InputBuffer::from("宇宙人");
    assert_eq!(buffer.orig_slice(0..3), "宇");
    assert_eq!(buffer.orig_slice(3..6), "宙");
    assert_eq!(buffer.orig_slice(6..9), "人");
}
