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

use crate::dic::word_id::{WordId, WordRef as DicWordRef};
use crate::dic::word_info::WordInfos;

use super::{
    LexiconReader, ParsedLexiconEntry, ResolvedLexiconEntry, ResolvedWordRef, WordRef,
    WordRefResolver,
};

impl LexiconReader {
    pub(crate) fn resolve_entries<R: WordRefResolver>(
        &mut self,
        resolver: &R,
        user: bool,
    ) -> Result<usize, (String, usize)> {
        let mut total = 0;
        let mut resolved_entries = Vec::with_capacity(self.parsed_entries.len());
        let mut phantom_resolved: Vec<ResolvedLexiconEntry> = Vec::new();
        let mut phantom_headwords: HashMap<String, DicWordRef> = HashMap::new();
        let mut next_phantom_offset =
            (self.next_entry_id() as usize) << WordInfos::WORD_ID_ALIGNMENT_BITS;
        for (line, entry) in self.parsed_entries.iter().cloned().enumerate() {
            let (resolved, resolved_count, phantom_headword) = self
                .resolve_entry(
                    entry,
                    resolver,
                    &phantom_headwords,
                    next_phantom_offset,
                    user,
                )
                .map_err(|split_info| (split_info, line))?;
            total += resolved_count;
            if let Some(headword) = phantom_headword {
                phantom_resolved.push(ResolvedLexiconEntry::make_phantom(&resolved, headword));
                let phantom_ref = DicWordRef::new(
                    !user,
                    (next_phantom_offset >> WordInfos::WORD_ID_ALIGNMENT_BITS) as u32,
                );
                phantom_headwords.insert(
                    phantom_resolved.last().unwrap().headword().to_owned(),
                    phantom_ref,
                );
                next_phantom_offset += phantom_resolved.last().unwrap().expected_entry_size();
            }
            resolved_entries.push(resolved);
        }
        resolved_entries.extend(phantom_resolved);
        self.resolved_entries = resolved_entries;
        Ok(total)
    }

    pub(crate) fn row_word_ids(&self, dic_id: u8) -> Vec<WordId> {
        let mut result = Vec::with_capacity(self.parsed_entries.len());
        let mut offset = Self::ENTRY_INITIAL_OFFSET;
        for e in &self.parsed_entries {
            let entry_id = (offset >> WordInfos::WORD_ID_ALIGNMENT_BITS) as u32;
            result.push(WordId::new(dic_id, entry_id));
            offset += e.expected_entry_size();
        }
        result
    }

    pub(crate) fn row_word_refs(&self, user: bool) -> Vec<DicWordRef> {
        let mut result = Vec::with_capacity(self.parsed_entries.len());
        let mut offset = Self::ENTRY_INITIAL_OFFSET;
        for e in &self.parsed_entries {
            let entry_id = (offset >> WordInfos::WORD_ID_ALIGNMENT_BITS) as u32;
            result.push(DicWordRef::new(!user, entry_id));
            offset += e.expected_entry_size();
        }
        result
    }

    fn resolve_entry<R: WordRefResolver>(
        &self,
        entry: ParsedLexiconEntry,
        resolver: &R,
        phantom_headwords: &HashMap<String, DicWordRef>,
        next_phantom_offset: usize,
        user: bool,
    ) -> Result<(ResolvedLexiconEntry, usize, Option<String>), String> {
        let mut total = 0;
        let (norm_form, phantom_headword) = self.resolve_norm_form(
            &entry,
            resolver,
            phantom_headwords,
            next_phantom_offset,
            user,
            &mut total,
        )?;
        let dic_form = self
            .resolve_dic_form_ref(&entry.dic_form, resolver, &mut total)
            .map_err(|r| self.format_word_ref(&r))?;
        let splits_a = self
            .resolve_word_refs(&entry.splits_a, resolver, &mut total)
            .map_err(|r| self.format_word_ref(&r))?;
        let splits_b = self
            .resolve_word_refs(&entry.splits_b, resolver, &mut total)
            .map_err(|r| self.format_word_ref(&r))?;
        let splits_c = self
            .resolve_word_refs(&entry.splits_c, resolver, &mut total)
            .map_err(|r| self.format_word_ref(&r))?;
        let word_structure = self
            .resolve_word_refs(&entry.word_structure, resolver, &mut total)
            .map_err(|r| self.format_word_ref(&r))?;

        let resolved = ResolvedLexiconEntry {
            left_id: entry.left_id,
            right_id: entry.right_id,
            cost: entry.cost,
            index_form: entry.index_form,
            headword: entry.headword,
            reference_id: entry.reference_id,
            dic_form,
            norm_form,
            pos: entry.pos,
            splits_a,
            splits_b,
            splits_c,
            reading: entry.reading,
            splitting: entry.splitting,
            word_structure,
            synonym_groups: entry.synonym_groups,
            user_data: entry.user_data,
        };

        Ok((resolved, total, phantom_headword))
    }

    fn resolve_norm_form<R: WordRefResolver>(
        &self,
        entry: &ParsedLexiconEntry,
        resolver: &R,
        phantom_headwords: &HashMap<String, DicWordRef>,
        next_phantom_offset: usize,
        user: bool,
        total: &mut usize,
    ) -> Result<(ResolvedWordRef, Option<String>), String> {
        match &entry.norm_form {
            WordRef::SelfRef => Ok((ResolvedWordRef::SelfRef, None)),
            WordRef::Headword(headword) => {
                *total += 1;
                if let Some(wref) = resolver
                    .resolve_by_headword(headword)
                    .or_else(|| phantom_headwords.get(headword).copied())
                {
                    return Ok((ResolvedWordRef::Ref(wref), None));
                }

                // If the given string does not found in the resolver/previous entries,
                // add phantom entry with that headword.
                let phantom_ref = DicWordRef::new(
                    !user,
                    (next_phantom_offset >> WordInfos::WORD_ID_ALIGNMENT_BITS) as u32,
                );
                Ok((ResolvedWordRef::Ref(phantom_ref), Some(headword.to_owned())))
            }
            word_ref => {
                let wref = resolver
                    .resolve(word_ref)
                    .ok_or_else(|| self.format_word_ref(word_ref))?;
                *total += 1;
                Ok((ResolvedWordRef::Ref(wref), None))
            }
        }
    }

    fn resolve_dic_form_ref<R: WordRefResolver>(
        &self,
        word_ref: &WordRef,
        resolver: &R,
        total: &mut usize,
    ) -> Result<ResolvedWordRef, WordRef> {
        match word_ref {
            WordRef::SelfRef => Ok(ResolvedWordRef::SelfRef),
            other => {
                let wref = resolver.resolve(other).ok_or_else(|| other.clone())?;
                *total += 1;
                Ok(ResolvedWordRef::Ref(wref))
            }
        }
    }

    fn resolve_word_refs<R: WordRefResolver>(
        &self,
        word_refs: &[WordRef],
        resolver: &R,
        total: &mut usize,
    ) -> Result<Vec<DicWordRef>, WordRef> {
        let mut out = Vec::with_capacity(word_refs.len());
        for word_ref in word_refs {
            let wref = resolver.resolve(word_ref).ok_or_else(|| word_ref.clone())?;
            match word_ref {
                WordRef::SelfRef => {}
                _ => *total += 1,
            }
            out.push(wref);
        }
        Ok(out)
    }

    fn format_word_ref(&self, word_ref: &WordRef) -> String {
        match word_ref {
            WordRef::SelfRef => "<self>".to_owned(),
            WordRef::LineRef(id) => id.as_raw().to_string(),
            WordRef::Headword(h) => h.clone(),
            WordRef::EntryKey {
                headword,
                pos,
                reading,
                reference_id,
            } => format!(
                "{},{:?},{}{}",
                headword,
                self.pos_obj(*pos).unwrap(),
                reading.as_ref().unwrap_or(headword),
                reference_id
                    .as_ref()
                    .map(|rid| format!(",{}", rid))
                    .unwrap_or_default()
            ),
        }
    }
}
