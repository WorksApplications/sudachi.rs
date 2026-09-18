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

use std::io::Write;

use nom::number::complete::{le_i16, le_i32, le_i8, le_u32};

use crate::dic::lexicon::strings::StringPointer;
use crate::dic::read::error::SudachiNomResult;
use crate::dic::read::utf16_string::{skip_utf16_string, utf16_string};
use crate::dic::word_info::layout;
use crate::dic::word_info::{WordInfoFixedData, WordInfoVariableData};
use crate::error::SudachiResult;

impl WordInfoFixedData {
    pub fn parse(input: &[u8]) -> SudachiNomResult<&[u8], Self> {
        let (input, pos_id) = le_i16(input)?;
        let (input, headword_strptr) =
            le_u32(input).map(|(rest, pointer)| (rest, StringPointer::decode(pointer)))?;
        let (input, reading_form_strptr) =
            le_u32(input).map(|(rest, pointer)| (rest, StringPointer::decode(pointer)))?;
        let (input, normalized_form) = le_u32(input)?;
        let (input, dictionary_form) = le_u32(input)?;
        let (input, index_form_length) = le_i16(input)?;
        let (input, c_unit_split_length) = le_i8(input)?;
        let (input, b_unit_split_length) = le_i8(input)?;
        let (input, a_unit_split_length) = le_i8(input)?;
        let (input, word_structure_length) = le_i8(input)?;
        let (input, synonym_group_ids_length) = le_i8(input)?;
        let (input, user_data_flag) = le_i8(input)?;
        Ok((
            input,
            Self {
                pos_id,
                headword_strptr,
                reading_form_strptr,
                normalized_form,
                dictionary_form,
                index_form_length,
                c_unit_split_length,
                b_unit_split_length,
                a_unit_split_length,
                word_structure_length,
                synonym_group_ids_length,
                user_data_flag,
            },
        ))
    }

    pub fn from_entry_bytes(data: &[u8]) -> Option<Self> {
        let fixed = data.get(..layout::FIXED_PART_SIZE)?;
        Some(Self {
            pos_id: i16::from_le_bytes([
                fixed[layout::PARAMS_SIZE],
                fixed[layout::PARAMS_SIZE + 1],
            ]),
            headword_strptr: StringPointer::decode(u32::from_le_bytes([
                fixed[8], fixed[9], fixed[10], fixed[11],
            ])),
            reading_form_strptr: StringPointer::decode(u32::from_le_bytes([
                fixed[12], fixed[13], fixed[14], fixed[15],
            ])),
            normalized_form: u32::from_le_bytes([fixed[16], fixed[17], fixed[18], fixed[19]]),
            dictionary_form: u32::from_le_bytes([fixed[20], fixed[21], fixed[22], fixed[23]]),
            index_form_length: i16::from_le_bytes([fixed[24], fixed[25]]),
            c_unit_split_length: fixed[layout::OFFSET_C_UNIT_SPLIT_LENGTH] as i8,
            b_unit_split_length: fixed[layout::OFFSET_B_UNIT_SPLIT_LENGTH] as i8,
            a_unit_split_length: fixed[layout::OFFSET_A_UNIT_SPLIT_LENGTH] as i8,
            word_structure_length: fixed[layout::OFFSET_WORD_STRUCTURE_LENGTH] as i8,
            synonym_group_ids_length: fixed[layout::OFFSET_SYNONYM_GROUP_IDS_LENGTH] as i8,
            user_data_flag: fixed[layout::OFFSET_USER_DATA_FLAG] as i8,
        })
    }

    pub fn has_user_data(&self) -> bool {
        self.user_data_flag == 1
    }

    pub fn write_to<W: Write>(&self, w: &mut W) -> std::io::Result<usize> {
        w.write_all(&self.pos_id.to_le_bytes())?;
        w.write_all(&self.headword_strptr.encode().to_le_bytes())?;
        w.write_all(&self.reading_form_strptr.encode().to_le_bytes())?;
        w.write_all(&self.normalized_form.to_le_bytes())?;
        w.write_all(&self.dictionary_form.to_le_bytes())?;
        w.write_all(&self.index_form_length.to_le_bytes())?;
        w.write_all(&self.c_unit_split_length.to_le_bytes())?;
        w.write_all(&self.b_unit_split_length.to_le_bytes())?;
        w.write_all(&self.a_unit_split_length.to_le_bytes())?;
        w.write_all(&self.word_structure_length.to_le_bytes())?;
        w.write_all(&self.synonym_group_ids_length.to_le_bytes())?;
        w.write_all(&self.user_data_flag.to_le_bytes())?;
        Ok(layout::WORD_INFO_FIXED_SIZE)
    }
}

pub(crate) fn parse_u32_array(
    input: &[u8],
    length: usize,
    keep: bool,
) -> SudachiResult<(&[u8], Vec<u32>)> {
    if keep {
        let (rest, values) = nom::multi::count(le_u32, length)(input)?;
        Ok((rest, values))
    } else {
        let bytes = length * 4;
        let (rest, _) = nom::bytes::complete::take(bytes)(input)?;
        Ok((rest, Vec::new()))
    }
}

pub(crate) fn parse_i32_array(
    input: &[u8],
    length: usize,
    keep: bool,
) -> SudachiResult<(&[u8], Vec<i32>)> {
    if keep {
        let (rest, values) = nom::multi::count(le_i32, length)(input)?;
        Ok((rest, values))
    } else {
        let bytes = length * 4;
        let (rest, _) = nom::bytes::complete::take(bytes)(input)?;
        Ok((rest, Vec::new()))
    }
}

pub(crate) fn parse_user_data(input: &[u8], keep: bool) -> SudachiResult<(&[u8], String)> {
    if keep {
        utf16_string(input).map_err(Into::into)
    } else {
        skip_utf16_string(input).map_err(Into::into)
    }
}

pub(crate) fn write_u32_slice<W: Write>(w: &mut W, data: &[u32]) -> std::io::Result<usize> {
    let mut size = 0;
    for value in data {
        w.write_all(&value.to_le_bytes())?;
        size += 4;
    }
    Ok(size)
}

pub(crate) fn write_i32_slice<W: Write>(w: &mut W, data: &[i32]) -> std::io::Result<usize> {
    let mut size = 0;
    for value in data {
        w.write_all(&value.to_le_bytes())?;
        size += 4;
    }
    Ok(size)
}

pub(crate) fn write_utf16_string<W: Write>(w: &mut W, data: &str) -> std::io::Result<usize> {
    let utf16: Vec<u16> = data.encode_utf16().collect();
    let mut size = 0;
    w.write_all(&(utf16.len() as i16).to_le_bytes())?;
    size += 2;
    for unit in utf16 {
        w.write_all(&unit.to_le_bytes())?;
        size += 2;
    }
    Ok(size)
}

impl<'a> WordInfoVariableData<'a> {
    pub fn write_to<W: Write>(
        &self,
        w: &mut W,
        fixed: &WordInfoFixedData,
    ) -> std::io::Result<usize> {
        let mut size = 0;
        size += write_u32_slice(w, self.c_unit_split)?;
        if fixed.b_unit_split_length > 0 {
            size += write_u32_slice(w, self.b_unit_split)?;
        }
        if fixed.a_unit_split_length > 0 {
            size += write_u32_slice(w, self.a_unit_split)?;
        }
        if fixed.word_structure_length > 0 {
            size += write_u32_slice(w, self.word_structure)?;
        }
        size += write_i32_slice(w, self.synonym_group_ids)?;
        if fixed.has_user_data() {
            size += write_utf16_string(w, self.user_data)?;
        }
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dic::word_info::WordInfoParser;

    fn sample_fixed() -> WordInfoFixedData {
        WordInfoFixedData {
            pos_id: 123,
            headword_strptr: StringPointer::unchecked(3, 8),
            reading_form_strptr: StringPointer::unchecked(5, 16),
            normalized_form: 42,
            dictionary_form: 84,
            index_form_length: 9,
            c_unit_split_length: 2,
            b_unit_split_length: -1,
            a_unit_split_length: 4,
            word_structure_length: -1,
            synonym_group_ids_length: 3,
            user_data_flag: 1,
        }
    }

    #[test]
    fn fixed_data_write_and_parse_round_trip() {
        let fixed = sample_fixed();
        let mut bytes = Vec::new();
        let written = fixed.write_to(&mut bytes).unwrap();
        assert_eq!(written, layout::WORD_INFO_FIXED_SIZE);

        let (rest, parsed) = WordInfoFixedData::parse(&bytes).unwrap();
        assert!(rest.is_empty());
        assert_eq!(parsed.pos_id, fixed.pos_id);
        assert_eq!(parsed.headword_strptr, fixed.headword_strptr);
        assert_eq!(parsed.reading_form_strptr, fixed.reading_form_strptr);
        assert_eq!(parsed.normalized_form, fixed.normalized_form);
        assert_eq!(parsed.dictionary_form, fixed.dictionary_form);
        assert_eq!(parsed.index_form_length, fixed.index_form_length);
        assert_eq!(parsed.c_unit_split_length, fixed.c_unit_split_length);
        assert_eq!(parsed.b_unit_split_length, fixed.b_unit_split_length);
        assert_eq!(parsed.a_unit_split_length, fixed.a_unit_split_length);
        assert_eq!(parsed.word_structure_length, fixed.word_structure_length);
        assert_eq!(
            parsed.synonym_group_ids_length,
            fixed.synonym_group_ids_length
        );
        assert_eq!(parsed.user_data_flag, fixed.user_data_flag);
    }

    #[test]
    fn fixed_data_reads_from_entry_bytes_after_params() {
        let fixed = sample_fixed();
        let mut entry = vec![0u8; layout::PARAMS_SIZE];
        fixed.write_to(&mut entry).unwrap();
        assert_eq!(entry.len(), layout::FIXED_PART_SIZE);

        let scanned = WordInfoFixedData::from_entry_bytes(&entry).unwrap();
        assert_eq!(scanned.pos_id, fixed.pos_id);
        assert_eq!(scanned.headword_strptr, fixed.headword_strptr);
        assert_eq!(scanned.reading_form_strptr, fixed.reading_form_strptr);
        assert_eq!(scanned.normalized_form, fixed.normalized_form);
        assert_eq!(scanned.dictionary_form, fixed.dictionary_form);
        assert_eq!(scanned.index_form_length, fixed.index_form_length);
        assert_eq!(scanned.c_unit_split_length, fixed.c_unit_split_length);
        assert_eq!(scanned.b_unit_split_length, fixed.b_unit_split_length);
        assert_eq!(scanned.a_unit_split_length, fixed.a_unit_split_length);
        assert_eq!(scanned.word_structure_length, fixed.word_structure_length);
        assert_eq!(
            scanned.synonym_group_ids_length,
            fixed.synonym_group_ids_length
        );
        assert_eq!(scanned.user_data_flag, fixed.user_data_flag);
    }

    #[test]
    fn fixed_and_variable_round_trip_into_raw_data() {
        let fixed = WordInfoFixedData {
            pos_id: 15,
            headword_strptr: StringPointer::unchecked(4, 12),
            reading_form_strptr: StringPointer::unchecked(5, 18),
            normalized_form: 123,
            dictionary_form: 456,
            index_form_length: 9,
            c_unit_split_length: 2,
            b_unit_split_length: 1,
            a_unit_split_length: 3,
            word_structure_length: 2,
            synonym_group_ids_length: 2,
            user_data_flag: 1,
        };
        let variable = WordInfoVariableData {
            c_unit_split: &[10, 11],
            b_unit_split: &[20],
            a_unit_split: &[30, 31, 32],
            word_structure: &[40, 41],
            synonym_group_ids: &[7, 8],
            user_data: "meta",
        };

        let mut bytes = vec![0u8; layout::PARAMS_SIZE];
        fixed.write_to(&mut bytes).unwrap();
        variable.write_to(&mut bytes, &fixed).unwrap();

        let parsed = WordInfoParser::default().parse(&bytes).unwrap();
        assert_eq!(parsed.pos_id, fixed.pos_id);
        assert_eq!(parsed.headword_strptr, fixed.headword_strptr);
        assert_eq!(parsed.reading_form_strptr, fixed.reading_form_strptr);
        assert_eq!(parsed.normalized_form, fixed.normalized_form);
        assert_eq!(parsed.dictionary_form, fixed.dictionary_form);
        assert_eq!(parsed.index_form_length, fixed.index_form_length);
        assert_eq!(parsed.c_unit_split_length, fixed.c_unit_split_length);
        assert_eq!(parsed.b_unit_split_length, fixed.b_unit_split_length);
        assert_eq!(parsed.a_unit_split_length, fixed.a_unit_split_length);
        assert_eq!(parsed.word_structure_length, fixed.word_structure_length);
        assert_eq!(
            parsed.synonym_group_ids_length,
            fixed.synonym_group_ids_length
        );
        assert_eq!(parsed.user_data_flag, fixed.user_data_flag);
        assert_eq!(parsed.c_unit_split, variable.c_unit_split);
        assert_eq!(parsed.b_unit_split, variable.b_unit_split);
        assert_eq!(parsed.a_unit_split, variable.a_unit_split);
        assert_eq!(parsed.word_structure, variable.word_structure);
        assert_eq!(parsed.synonym_group_ids, variable.synonym_group_ids);
        assert_eq!(parsed.user_data, variable.user_data);
    }
}
