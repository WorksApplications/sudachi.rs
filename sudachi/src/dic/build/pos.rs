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

use std::borrow::Cow;

use csv::{StringRecord, Trim};
use indexmap::map::IndexMap;

use crate::dic::build::csv_schema::{
    parse_column_name, parse_header_mapping, validate_required_columns, CsvColumn,
};
use crate::dic::build::error::{BuildFailure, DicCompilationCtx};
use crate::dic::build::lexicon::StrPosEntry;
use crate::dic::build::parse::{parse_i16, unescape};
use crate::dic::build::MAX_POS_IDS;
use crate::error::SudachiResult;

#[repr(usize)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum PosCsvColumn {
    PosId = 0,
    Pos1 = 1,
    Pos2 = 2,
    Pos3 = 3,
    Pos4 = 4,
    Pos5 = 5,
    Pos6 = 6,
}

impl PosCsvColumn {
    const fn label(self) -> &'static str {
        match self {
            PosCsvColumn::PosId => "POS_ID",
            PosCsvColumn::Pos1 => "POS1",
            PosCsvColumn::Pos2 => "POS2",
            PosCsvColumn::Pos3 => "POS3",
            PosCsvColumn::Pos4 => "POS4",
            PosCsvColumn::Pos5 => "POS5",
            PosCsvColumn::Pos6 => "POS6",
        }
    }

    fn from_str(data: &str) -> Option<Self> {
        parse_column_name::<Self, 7>(data)
    }
}

impl CsvColumn<7> for PosCsvColumn {
    fn as_usize(self) -> usize {
        self as usize
    }

    fn label(self) -> &'static str {
        match self {
            PosCsvColumn::PosId => "POS_ID",
            PosCsvColumn::Pos1 => "POS1",
            PosCsvColumn::Pos2 => "POS2",
            PosCsvColumn::Pos3 => "POS3",
            PosCsvColumn::Pos4 => "POS4",
            PosCsvColumn::Pos5 => "POS5",
            PosCsvColumn::Pos6 => "POS6",
        }
    }

    fn from_normalized(data: &str) -> Option<Self> {
        match data {
            "posid" => Some(PosCsvColumn::PosId),
            "pos1" => Some(PosCsvColumn::Pos1),
            "pos2" => Some(PosCsvColumn::Pos2),
            "pos3" => Some(PosCsvColumn::Pos3),
            "pos4" => Some(PosCsvColumn::Pos4),
            "pos5" => Some(PosCsvColumn::Pos5),
            "pos6" => Some(PosCsvColumn::Pos6),
            _ => None,
        }
    }
}

const REQUIRED_COLUMNS: [PosCsvColumn; 6] = [
    PosCsvColumn::Pos1,
    PosCsvColumn::Pos2,
    PosCsvColumn::Pos3,
    PosCsvColumn::Pos4,
    PosCsvColumn::Pos5,
    PosCsvColumn::Pos6,
];

#[derive(Copy, Clone)]
enum PosColumnLayout {
    LegacyNoId,
    LegacyWithId,
    Header([i16; 7]),
}

impl PosColumnLayout {
    fn from_record(record: &StringRecord, ctx: &DicCompilationCtx) -> SudachiResult<(Self, bool)> {
        let first = record.get(0).unwrap_or_default();
        if PosCsvColumn::from_str(first).is_none() {
            if record.len() == 6 {
                return Ok((PosColumnLayout::LegacyNoId, false));
            }
            return Ok((PosColumnLayout::LegacyWithId, false));
        }

        let mapping = parse_header_mapping::<PosCsvColumn, 7>(
            record,
            ctx,
            "INVALID_POS_COLUMN_NAME",
            "DUPLICATED_POS_COLUMN_NAME",
        )?;
        validate_required_columns(&mapping, &REQUIRED_COLUMNS, ctx)?;

        Ok((PosColumnLayout::Header(mapping), true))
    }

    const fn has_pos_id(self) -> bool {
        match self {
            PosColumnLayout::LegacyNoId => false,
            PosColumnLayout::LegacyWithId => true,
            PosColumnLayout::Header(mapping) => mapping[PosCsvColumn::PosId as usize] >= 0,
        }
    }

    fn index(self, col: PosCsvColumn) -> Option<usize> {
        match self {
            PosColumnLayout::LegacyNoId => match col {
                PosCsvColumn::PosId => None,
                PosCsvColumn::Pos1 => Some(0),
                PosCsvColumn::Pos2 => Some(1),
                PosCsvColumn::Pos3 => Some(2),
                PosCsvColumn::Pos4 => Some(3),
                PosCsvColumn::Pos5 => Some(4),
                PosCsvColumn::Pos6 => Some(5),
            },
            PosColumnLayout::LegacyWithId => match col {
                PosCsvColumn::PosId => Some(0),
                PosCsvColumn::Pos1 => Some(1),
                PosCsvColumn::Pos2 => Some(2),
                PosCsvColumn::Pos3 => Some(3),
                PosCsvColumn::Pos4 => Some(4),
                PosCsvColumn::Pos5 => Some(5),
                PosCsvColumn::Pos6 => Some(6),
            },
            PosColumnLayout::Header(mapping) => {
                let idx = mapping[col.as_usize()];
                if idx < 0 {
                    None
                } else {
                    Some(idx as usize)
                }
            }
        }
    }
}

pub(crate) fn read_pos_bytes(
    pos: &mut IndexMap<StrPosEntry, u16>,
    entries_loaded: bool,
    data: &[u8],
    ctx: &mut DicCompilationCtx,
) -> SudachiResult<usize> {
    if entries_loaded || !pos.is_empty() {
        return ctx.err(BuildFailure::InvalidSplit(
            "POS table must be loaded before lexicon".to_owned(),
        ));
    }

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .trim(Trim::None)
        .flexible(true)
        .from_reader(data);
    let mut nread = 0usize;
    let mut layout = PosColumnLayout::LegacyNoId;
    let mut staged_by_id: Vec<Option<StrPosEntry>> = Vec::new();
    let mut first = true;
    for row in reader.records() {
        match row {
            Ok(r) => {
                let line = r.position().map_or(0, |p| p.line()) as usize;
                ctx.set_line(line);
                if first {
                    first = false;
                    let (resolved, skip) = PosColumnLayout::from_record(&r, ctx)?;
                    layout = resolved;
                    if skip {
                        continue;
                    }
                }
                read_pos_record(pos, &mut staged_by_id, &r, layout, ctx)?;
                nread += 1;
            }
            Err(e) => {
                let line = e.position().map_or(0, |p| p.line()) as usize;
                ctx.set_line(line);
                return Err(ctx.to_sudachi_err(BuildFailure::CsvError(e)));
            }
        }
    }

    if layout.has_pos_id() {
        for (pos_id, item) in staged_by_id.into_iter().enumerate() {
            let key = match item {
                Some(k) => k,
                None => {
                    return ctx.err(BuildFailure::InvalidSplit(
                        "POS_ID must be contiguous".to_owned(),
                    ))
                }
            };
            pos.insert(key, pos_id as u16);
        }
    }

    Ok(nread)
}

fn read_pos_record(
    pos: &mut IndexMap<StrPosEntry, u16>,
    staged_by_id: &mut Vec<Option<StrPosEntry>>,
    data: &StringRecord,
    layout: PosColumnLayout,
    ctx: &DicCompilationCtx,
) -> SudachiResult<()> {
    let p1 = pos_field(data, layout, PosCsvColumn::Pos1, ctx)?;
    let p2 = pos_field(data, layout, PosCsvColumn::Pos2, ctx)?;
    let p3 = pos_field(data, layout, PosCsvColumn::Pos3, ctx)?;
    let p4 = pos_field(data, layout, PosCsvColumn::Pos4, ctx)?;
    let p5 = pos_field(data, layout, PosCsvColumn::Pos5, ctx)?;
    let p6 = pos_field(data, layout, PosCsvColumn::Pos6, ctx)?;
    let key = StrPosEntry::new([
        Cow::Owned(p1),
        Cow::Owned(p2),
        Cow::Owned(p3),
        Cow::Owned(p4),
        Cow::Owned(p5),
        Cow::Owned(p6),
    ]);

    if layout.has_pos_id() {
        let idx = layout.index(PosCsvColumn::PosId).unwrap();
        let raw = match data.get(idx) {
            Some(v) => v,
            None => return ctx.err(BuildFailure::NoRawField(PosCsvColumn::PosId.label())),
        };
        if raw.is_empty() {
            return ctx.err(BuildFailure::InvalidSplit(
                "POS_ID is required when POS_ID column exists".to_owned(),
            ));
        }
        let raw_id = ctx.transform(parse_i16(raw))?;
        if raw_id < 0 {
            return ctx.err(BuildFailure::InvalidSplit("POS_ID must be >= 0".to_owned()));
        }
        let pos_id = raw_id as u16;
        let pos_id_usize = pos_id as usize;
        if pos_id_usize > MAX_POS_IDS {
            return ctx.err(BuildFailure::PosLimitExceeded(format!("{:?}", key)));
        }
        if staged_by_id.len() <= pos_id_usize {
            staged_by_id.resize_with(pos_id_usize + 1, || None);
        }
        if staged_by_id[pos_id_usize].is_some() {
            return ctx.err(BuildFailure::InvalidSplit(
                "POS_ID already exists".to_owned(),
            ));
        }
        staged_by_id[pos_id_usize] = Some(key);
    } else {
        if pos.contains_key(&key) {
            return ctx.err(BuildFailure::InvalidSplit("POS already exists".to_owned()));
        }
        let expected = pos.len();
        if expected > MAX_POS_IDS {
            return ctx.err(BuildFailure::PosLimitExceeded(format!("{:?}", key)));
        }
        pos.insert(key, expected as u16);
    }
    Ok(())
}

fn pos_field(
    data: &StringRecord,
    layout: PosColumnLayout,
    col: PosCsvColumn,
    ctx: &DicCompilationCtx,
) -> SudachiResult<String> {
    let idx = match layout.index(col) {
        Some(i) => i,
        None => return ctx.err(BuildFailure::NoRawField(col.label())),
    };
    let raw = match data.get(idx) {
        Some(v) => v,
        None => return ctx.err(BuildFailure::NoRawField(col.label())),
    };
    ctx.transform(unescape(raw))
}
