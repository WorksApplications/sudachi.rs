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

extern crate lazy_static;

extern crate sudachi;
use sudachi::dic::header::{HeaderVersion, SystemDictVersion};

mod common;
use common::HEADER;

#[test]
fn version() {
    assert_eq!(
        HeaderVersion::SystemDict(SystemDictVersion::Version2),
        HEADER.version
    );
}

#[test]
fn create_time() {
    assert!(HEADER.create_time > 0);
}

#[test]
fn description() {
    assert_eq!(
        "the system dictionary for the unit tests",
        HEADER.description
    );
}
