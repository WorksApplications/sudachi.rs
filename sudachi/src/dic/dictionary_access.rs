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

use crate::dic::description::Description;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon_set::LexiconSet;
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
