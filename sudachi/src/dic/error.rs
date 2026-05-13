/*
 * Copyright (c) 2026 Works Applications Co., Ltd.
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

use thiserror::Error;

#[derive(Error, Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum DictionaryCompatibilityError {
    #[error(
        "{user_index}-th user dictionary is not compatible with the system dictionary (expected signature: {system_signature}, actual reference: {user_reference})"
    )]
    UserDictionary {
        user_index: usize,
        system_signature: String,
        user_reference: String,
    },

    #[error(
        "user dictionary is not compatible with the system dictionary (expected signature: {system_signature}, actual reference: {user_reference})"
    )]
    UserDictionaryWithoutIndex {
        system_signature: String,
        user_reference: String,
    },
}
