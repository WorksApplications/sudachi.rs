/*
 * Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

pub mod binary_loader;
pub mod build;
pub mod category_type;
pub mod character_category;
pub mod connect;
pub mod description;
pub mod dictionary;
mod dictionary_access;
pub mod error;
pub mod grammar;
pub mod header;
pub mod lexicon;
pub mod lexicon_set;
pub mod pos;
pub mod read;
pub mod storage;
mod strings_cache;
pub mod subset;
pub mod word_id;
pub mod word_info;

pub use dictionary_access::{
    DescriptionAccess, DictionaryAccess, LexiconAccess, ReferenceIdAccess,
};
