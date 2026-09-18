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

use csv::StringRecord;

use crate::dic::build::error::{BuildFailure, DicCompilationCtx};
use crate::error::SudachiResult;

pub(crate) trait CsvColumn<const N: usize>: Copy {
    fn as_usize(self) -> usize;
    fn label(self) -> &'static str;
    fn from_normalized(data: &str) -> Option<Self>;
}

pub(crate) fn parse_column_name<C, const N: usize>(data: &str) -> Option<C>
where
    C: CsvColumn<N>,
{
    let mut normalized = String::with_capacity(data.len());
    for c in data.chars() {
        if c != '_' {
            normalized.push(c.to_ascii_lowercase());
        }
    }
    C::from_normalized(&normalized)
}

pub(crate) fn parse_header_mapping<C, const N: usize>(
    record: &StringRecord,
    ctx: &DicCompilationCtx,
    invalid_column_error: &'static str,
    duplicated_column_error: &'static str,
) -> SudachiResult<[i16; N]>
where
    C: CsvColumn<N>,
{
    let mut mapping = [-1_i16; N];
    for (idx, field) in record.iter().enumerate() {
        let col = match parse_column_name::<C, N>(field) {
            Some(c) => c,
            None => return ctx.err(BuildFailure::NoRawField(invalid_column_error)),
        };
        let prev = &mut mapping[col.as_usize()];
        if *prev >= 0 {
            return ctx.err(BuildFailure::NoRawField(duplicated_column_error));
        }
        *prev = idx as i16;
    }
    Ok(mapping)
}

pub(crate) fn validate_required_columns<C, const N: usize>(
    mapping: &[i16; N],
    required: &[C],
    ctx: &DicCompilationCtx,
) -> SudachiResult<()>
where
    C: CsvColumn<N>,
{
    for &col in required {
        if mapping[col.as_usize()] < 0 {
            return ctx.err(BuildFailure::NoRawField(col.label()));
        }
    }
    Ok(())
}
