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

use std::collections::HashMap;
use std::ops::Deref;

use crate::analysis::morpheme::SingleMorpheme;
use crate::dic::description::Description;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::subset::InfoSubset;
use crate::error::SudachiResult;
use crate::input_text::InputBuffer;
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;

pub trait LexiconAccess {
    fn lexicon(&self) -> &LexiconSet<'_>;
}

impl<T> LexiconAccess for T
where
    T: Deref,
    <T as Deref>::Target: LexiconAccess,
{
    fn lexicon(&self) -> &LexiconSet<'_> {
        <T as Deref>::deref(self).lexicon()
    }
}

pub trait DescriptionAccess {
    fn description(&self) -> &Description;
}

impl<T> DescriptionAccess for T
where
    T: Deref,
    <T as Deref>::Target: DescriptionAccess,
{
    fn description(&self) -> &Description {
        <T as Deref>::deref(self).description()
    }
}

/// Build-time helper access to dictionary entry reference IDs.
pub trait ReferenceIdAccess {
    fn reference_ids(&self) -> HashMap<u32, String>;
}

impl<T> ReferenceIdAccess for T
where
    T: Deref,
    <T as Deref>::Target: ReferenceIdAccess,
{
    fn reference_ids(&self) -> HashMap<u32, String> {
        <T as Deref>::deref(self).reference_ids()
    }
}

/// Provides access to dictionary data
pub trait DictionaryAccess: LexiconAccess {
    fn grammar(&self) -> &Grammar<'_>;

    fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin + Sync + Send>];
    fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin + Sync + Send>];
    fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin + Sync + Send>];
}

impl<T> DictionaryAccess for T
where
    T: Deref,
    <T as Deref>::Target: DictionaryAccess,
{
    fn grammar(&self) -> &Grammar<'_> {
        <T as Deref>::deref(self).grammar()
    }

    fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin + Sync + Send>] {
        <T as Deref>::deref(self).input_text_plugins()
    }

    fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin + Sync + Send>] {
        <T as Deref>::deref(self).oov_provider_plugins()
    }

    fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin + Sync + Send>] {
        <T as Deref>::deref(self).path_rewrite_plugins()
    }
}

/// Build normalized input text by applying dictionary input-text plugins.
pub fn normalize_input_text<D: DictionaryAccess + ?Sized>(
    dict: &D,
    text: &str,
    buffer: &mut InputBuffer,
) -> SudachiResult<()> {
    buffer.reset().push_str(text);
    buffer.start_build()?;
    for plugin in dict.input_text_plugins() {
        plugin.rewrite(buffer)?;
    }
    buffer.build(dict.grammar())
}

/// Look up entries by scanning every public dictionary entry.
pub fn lookup_all_entries<D>(
    dict: D,
    surface: &str,
    subset: InfoSubset,
) -> SudachiResult<Vec<SingleMorpheme<D>>>
where
    D: DictionaryAccess + Clone,
{
    let mut query_buffer = InputBuffer::new();
    normalize_input_text(&dict, surface, &mut query_buffer)?;
    let mut entry_buffer = InputBuffer::new();
    let mut result = Vec::new();

    for word_id in dict.lexicon().word_ids() {
        let word_info = dict
            .lexicon()
            .get_word_info_subset(word_id, InfoSubset::HEADWORD)?;
        normalize_input_text(&dict, word_info.headword(dict.lexicon()), &mut entry_buffer)?;
        if entry_buffer.current() == query_buffer.current() {
            result.push(SingleMorpheme::from_word_id(dict.clone(), word_id, subset)?);
        }
    }

    Ok(result)
}
