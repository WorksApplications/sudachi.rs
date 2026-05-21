/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
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
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use clap::{Args, Subcommand, ValueEnum};
use memmap2::Mmap;

use sudachi::dic::binary_loader::{BinaryDictionary, LoadedDictionary};
use sudachi::dic::build::report::DictPartReport;
use sudachi::dic::build::{CacheAwareOptions, DictBuilder, TrieBuildStrategy, TrieProfileMode};
use sudachi::dic::description::Description;
use sudachi::dic::grammar::Grammar;
use sudachi::dic::lexicon::Lexicon;
use sudachi::dic::lexicon_set::LexiconSet;
use sudachi::dic::word_id::WordId;
use sudachi::error::SudachiResult;
use sudachi::text_normalizer::TextNormalizer;

#[derive(Copy, Clone, Eq, PartialEq)]
enum PosDumpFormat {
    Components,
    Id,
}

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum DumpPart {
    Description,
    Matrix,
    Pos,
    Winfo,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum TrieLayoutArg {
    Classic,
    CacheAware,
}

/// Check that the first argument is a subcommand and the file with the same name does
/// not exists.
/// If the file does exists, probably it's safer to use default Sudachi analysis mode.
pub fn is_build_mode(subcommand: &Option<BuildCli>) -> bool {
    match subcommand {
        Some(subcommand) => {
            let raw = match subcommand {
                BuildCli::System { .. } => "build",
                BuildCli::User { .. } => "ubuild",
                BuildCli::Dump { .. } => "dump",
            };

            !Path::new(&raw).exists()
        }
        None => false,
    }
}

#[derive(Subcommand)]
pub(crate) enum BuildCli {
    /// Builds system dictionary
    #[command(name = "build")]
    System {
        #[command(flatten)]
        common: BuildCmd,

        /// Path to matrix definition
        #[arg(short, long)]
        matrix: PathBuf,

        /// Path to POS csv definition
        #[arg(long)]
        pos: Option<PathBuf>,
    },

    /// Builds user dictionary
    #[command(name = "ubuild")]
    User {
        #[command(flatten)]
        common: BuildCmd,

        /// Path to system dictionary
        #[arg(short = 's', long = "system")]
        dictionary: PathBuf,
    },

    #[command(name = "dump")]
    Dump {
        /// target dictionary to dump
        dictionary: PathBuf,
        /// dump target (description, matrix, pos, winfo)
        part: DumpPart,
        /// output file
        output: PathBuf,

        /// reference system dictionary.
        /// required to dump winfo of an user dictionary
        #[arg(short = 's', long = "system")]
        system: Option<PathBuf>,

        /// Use POS_ID instead of POS1..POS6 in dump winfo output.
        /// entrykey references also use POS_ID.
        #[arg(long = "pos-id")]
        pos_id: bool,
    },
}

#[derive(Args)]
pub(crate) struct BuildCmd {
    /// Input csv files
    inputs: Vec<PathBuf>,

    /// Where to place compiled dictionary.
    /// If there was an existing one it will be overwritten.
    #[arg(short = 'o', long = "output")]
    output_file: PathBuf,

    /// Description string to embed into dictionary
    #[arg(short, long, default_value = "")]
    description: String,

    /// Trie layout strategy for dictionary compilation.
    #[arg(long = "trie-layout", value_enum, default_value_t = TrieLayoutArg::Classic)]
    trie_layout: TrieLayoutArg,

    /// External key profile for cache-aware trie layout: <surface-or-hex>\t<count>.
    #[arg(long = "trie-profile")]
    trie_profile: Option<PathBuf>,

    /// Cache line size in bytes used by the cache-aware trie layout scorer.
    #[arg(long = "trie-cache-line-bytes", default_value_t = 64)]
    trie_cache_line_bytes: usize,

    /// Number of recent 256-unit blocks searched by the cache-aware trie layout builder.
    #[arg(long = "trie-candidate-window", default_value_t = 16)]
    trie_candidate_window: usize,
}

impl BuildCmd {
    fn trie_build_strategy(&self) -> TrieBuildStrategy {
        match self.trie_layout {
            TrieLayoutArg::Classic => {
                if self.trie_profile.is_some() {
                    panic!("--trie-profile requires --trie-layout cache-aware");
                }
                TrieBuildStrategy::ClassicYada
            }
            TrieLayoutArg::CacheAware => {
                let profile_mode = self
                    .trie_profile
                    .as_ref()
                    .map(|path| TrieProfileMode::ExternalKeyProfile(path.clone()))
                    .unwrap_or(TrieProfileMode::DictionaryPrefixCount);
                TrieBuildStrategy::CacheAware(CacheAwareOptions {
                    cache_line_bytes: self.trie_cache_line_bytes,
                    candidate_window_blocks: self.trie_candidate_window,
                    profile_mode,
                    ..CacheAwareOptions::default()
                })
            }
        }
    }
}

pub fn build_main(subcommand: BuildCli) {
    match subcommand {
        BuildCli::System {
            common,
            matrix,
            pos,
        } => build_system(common, matrix, pos),
        BuildCli::User { common, dictionary } => build_user(common, dictionary),
        BuildCli::Dump {
            dictionary,
            part,
            output,
            system,
            pos_id,
        } => dump_part(
            dictionary,
            system,
            part,
            output,
            if pos_id {
                PosDumpFormat::Id
            } else {
                PosDumpFormat::Components
            },
        ),
    }
}

fn build_system(mut cmd: BuildCmd, matrix: PathBuf, pos: Option<PathBuf>) {
    let mut builder = DictBuilder::new_system();
    builder.set_description(std::mem::take(&mut cmd.description));
    builder.set_trie_build_strategy(cmd.trie_build_strategy());
    builder
        .read_conn(matrix.as_path())
        .expect("failed to read matrix");
    if let Some(pos_file) = pos {
        builder
            .read_pos(pos_file.as_path())
            .unwrap_or_else(|e| panic!("failed to read {:?}\n{:?}", pos_file, e));
    }
    for d in cmd.inputs.iter() {
        builder
            .read_lexicon(d.as_path())
            .unwrap_or_else(|e| panic!("failed to read {:?}\n{:?}", d, e));
    }
    builder.resolve().expect("failed to resolve references");
    let file = output_file(&cmd.output_file);
    let mut buf_writer = BufWriter::with_capacity(16 * 1024, file);
    builder
        .compile(&mut buf_writer)
        .expect("failed to compile dictionary");
    buf_writer.flush().expect("failed to flush");
    print_stats(builder.report());
}

fn build_user(mut cmd: BuildCmd, system: PathBuf) {
    let system_file = File::open(&system).expect("failed to open system dictionary");
    let system_data = unsafe { Mmap::map(&system_file) }.expect("failed to mmap system dictionary");
    let dict =
        LoadedDictionary::load_system(&system_data).expect("failed to load system dictionary");

    let mut builder = DictBuilder::new_user(&dict);
    builder.set_description(std::mem::take(&mut cmd.description));
    builder.set_trie_build_strategy(cmd.trie_build_strategy());
    for d in cmd.inputs.iter() {
        builder
            .read_lexicon(d.as_path())
            .unwrap_or_else(|e| panic!("failed to read {:?}\n{:?}", d, e));
    }
    builder.resolve().expect("failed to resolve references");
    let file = output_file(&cmd.output_file);
    let mut buf_writer = BufWriter::with_capacity(16 * 1024, file);
    builder
        .compile(&mut buf_writer)
        .expect("failed to compile dictionary");
    buf_writer.flush().expect("failed to flush");
    print_stats(builder.report());
}

fn print_stats(report: &[DictPartReport]) {
    let max_len = report.iter().map(|r| r.part().len()).max().unwrap_or(0);

    for part in report {
        let unit = if part.is_write() { "bytes" } else { "entries" };
        eprintln!(
            "{0:1$} {2} {3} in {4:.3} sec",
            part.part(),
            max_len,
            part.size(),
            unit,
            part.time().as_secs_f32()
        )
    }
}

fn output_file(p: &Path) -> File {
    if p.exists() {
        std::fs::remove_file(p).unwrap_or_else(|e| panic!("failed to delete {:?}\n{:?}", p, e));
    }

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(p)
        .unwrap_or_else(|e| panic!("failed to open {:?} for writing:\n{:?}", p, e))
}

fn dump_part(
    dict: PathBuf,
    system: Option<PathBuf>,
    part: DumpPart,
    output: PathBuf,
    pos_format: PosDumpFormat,
) {
    let file = File::open(&dict).expect("open dict failed");
    let data = unsafe { Mmap::map(&file) }.expect("mmap dict failed");
    let desc = Description::load(&data).expect("failed to load dictionary description");
    let loaded = if desc.is_system_dictionary() {
        BinaryDictionary::load_system(&data).expect("failed to load system dictionary")
    } else {
        BinaryDictionary::load_user(&data).expect("failed to load user dictionary")
    };

    let outf = output_file(&output);
    let mut writer = BufWriter::new(outf);

    match part {
        DumpPart::Description => dump_description(&dict, &loaded.description, &mut writer).unwrap(),
        DumpPart::Pos => dump_pos(loaded, &mut writer),
        DumpPart::Matrix => dump_matrix(loaded, &mut writer),
        DumpPart::Winfo => dump_word_info(loaded, system, pos_format, &mut writer).unwrap(),
    }
    writer.flush().unwrap();
}

fn dump_description<W: Write>(path: &Path, desc: &Description, w: &mut W) -> SudachiResult<()> {
    writeln!(w, "File: {}", path.display())?;
    if desc.is_system_dictionary() {
        writeln!(w, "type: system dictionary")?;
    } else if desc.is_user_dictionary() {
        writeln!(w, "type: user dictionary")?;
    } else {
        writeln!(w, "invalid file")?;
        return Ok(());
    }

    writeln!(
        w,
        "Creation time: {}",
        format_unix_timestamp_utc(desc.creation_time())
    )?;
    writeln!(w, "Comment: {}", desc.comment())?;
    writeln!(w, "Signature: {}", desc.signature())?;
    writeln!(w, "Reference: {}", desc.reference())?;
    writeln!(w, "Entries total: {}", desc.num_total_entries())?;
    writeln!(w, "Entries indexed: {}", desc.num_indexed_entries())?;
    for block in desc.blocks() {
        writeln!(
            w,
            "Block {}: {} - {}",
            block.name(),
            block.start(),
            block.end()
        )?;
    }
    writeln!(w, "Flag isRuntimeCosts: {}", desc.is_runtime_costs())?;
    Ok(())
}

fn format_unix_timestamp_utc(duration: Duration) -> String {
    let datetime = time::OffsetDateTime::from(UNIX_EPOCH + duration);
    datetime
        .format(&time::format_description::well_known::Rfc3339)
        .expect("failed to format dictionary creation time")
}

fn dump_pos<W: Write>(dict: BinaryDictionary, w: &mut W) {
    w.write_all(b"POS_ID,POS1,POS2,POS3,POS4,POS5,POS6\n")
        .unwrap();
    for (id, p) in dict.grammar.pos_list.iter().enumerate() {
        write!(w, "{}", id).unwrap();
        for e in p.iter() {
            write!(w, ",{}", csv_field(e)).unwrap();
        }
        w.write_all(b"\n").unwrap();
    }
}

fn dump_matrix<W: Write>(dict: BinaryDictionary, w: &mut W) {
    if dict.description.is_user_dictionary() {
        panic!("user dictionary does not have connection matrix.")
    }

    let conn = dict.grammar.connection.unwrap();
    writeln!(w, "{} {}", conn.num_left(), conn.num_right()).unwrap();
    for left in 0..conn.num_left() {
        for right in 0..conn.num_right() {
            let cost = conn.cost(left as _, right as _);
            writeln!(w, "{} {} {}", left, right, cost).unwrap();
        }
    }
}

fn dump_word_info<W: Write>(
    dict: BinaryDictionary,
    system: Option<PathBuf>,
    pos_format: PosDumpFormat,
    w: &mut W,
) -> SudachiResult<()> {
    let is_user = dict.description.is_user_dictionary();
    let did = if is_user { 1 } else { 0 };

    let data = system.map(|system_path| {
        let file = File::open(system_path).expect("open system failed");
        unsafe { Mmap::map(&file) }.expect("mmap system failed")
    });
    let system = data.as_ref().map(|data| {
        let system_dict =
            BinaryDictionary::load_system(data).expect("failed to load system dictionary");
        let reference_ids = system_dict
            .reference_id_table()
            .expect("failed to load reference-id table");
        let grammar = Grammar::from_system_binary(system_dict.grammar)
            .expect("failed to load system dictionary");
        let lexicon_set =
            LexiconSet::from_system_binary(system_dict.lexicon, grammar.pos_list.len());
        (grammar, lexicon_set, reference_ids)
    });

    let (base, user) = if is_user {
        let (grammar, lexicon_set, reference_ids) =
            system.expect("system dictionary is required to dump user dictionary lexicon");
        (((grammar, lexicon_set), reference_ids), Some(dict))
    } else {
        let system_reference_ids = dict.reference_id_table()?;
        let grammar = Grammar::from_system_binary(dict.grammar).expect("failed to load dictionary");
        let lexicon_set = LexiconSet::from_system_binary(dict.lexicon, grammar.pos_list.len());
        (((grammar, lexicon_set), system_reference_ids), None)
    };

    let ((mut grammar, mut lex), system_reference_ids) = base;
    let mut word_ids = Vec::new();
    let mut user_reference_ids = HashMap::new();
    if let Some(udic) = user {
        user_reference_ids = udic.reference_id_table()?;
        let user_lex = Lexicon::from_binary(udic.lexicon);
        for entry in user_lex.entry_ids_in_order() {
            word_ids.push(WordId::new(1, entry.as_raw()));
        }
        lex.append(user_lex, grammar.pos_list.len())?;
        grammar.merge_binary(udic.grammar);
    } else {
        for entry in lex.system_word_ids_in_order() {
            word_ids.push(entry);
        }
    }

    let normalizer = TextNormalizer::new(&grammar)?;
    writeln!(w, "{}", word_info_header(pos_format))?;
    for wid in word_ids {
        if wid.dict().as_raw() != did {
            continue;
        }
        let (left, right, cost) = lex.get_word_param(wid);
        // Internally generated phantom entries should not be dumped as source CSV rows.
        if left == -1 && right == -1 && cost == i16::MAX {
            continue;
        }
        let winfo = lex.get_word_info(wid)?;
        let headword = winfo.headword(&lex);
        let index_form = normalizer.normalize(headword)?;
        write!(w, "{},", csv_field(&index_form))?;
        write!(w, "{},{},{},", left, right, cost)?;
        if headword == index_form {
            write!(w, ",")?;
        } else {
            write!(w, "{},", csv_field(headword))?;
        }
        write!(w, "{},", pos_string(&grammar, winfo.pos_id(), pos_format))?;
        let reading = winfo.reading_form(&lex);
        write!(w, "{},", csv_field(reading))?;
        let normalized = winfo.normalized_form(&lex);
        if normalized == headword {
            write!(w, ",")?;
        } else {
            write!(w, "{},", csv_field(normalized))?;
        }
        let dict_form = dictionary_form_string(
            &grammar,
            &lex,
            wid,
            winfo.borrow_data().dictionary_form_word_id(),
            pos_format,
            &system_reference_ids,
            &user_reference_ids,
        )?;
        write!(w, "{},", dict_form)?;
        dump_wids(
            w,
            &grammar,
            &lex,
            winfo.a_unit_split(),
            pos_format,
            &system_reference_ids,
            &user_reference_ids,
        )?;
        w.write_all(b",")?;
        dump_wids(
            w,
            &grammar,
            &lex,
            winfo.b_unit_split(),
            pos_format,
            &system_reference_ids,
            &user_reference_ids,
        )?;
        w.write_all(b",")?;
        dump_wids(
            w,
            &grammar,
            &lex,
            winfo.c_unit_split(),
            pos_format,
            &system_reference_ids,
            &user_reference_ids,
        )?;
        w.write_all(b",")?;
        dump_wids(
            w,
            &grammar,
            &lex,
            winfo.word_structure(),
            pos_format,
            &system_reference_ids,
            &user_reference_ids,
        )?;
        w.write_all(b",")?;
        dump_gids(w, winfo.synonym_group_ids())?;
        write!(
            w,
            ",{},{}",
            csv_field(winfo.user_data()),
            csv_field(&reference_id_for_word_id(
                wid,
                &system_reference_ids,
                &user_reference_ids
            ))
        )?;
        w.write_all(b"\n")?;
    }
    Ok(())
}

fn unicode_escape(raw: &str) -> String {
    // replace '"' in raw data
    raw.replace('"', "\\u0022")
}

fn entrykey_ref_escape(raw: &str) -> String {
    raw.replace('"', "\\u0022")
        .replace(',', "\\u002c")
        .replace('/', "\\u002f")
}

fn csv_field(raw: &str) -> String {
    let escaped = unicode_escape(raw);
    if raw.contains(',') {
        format!("\"{}\"", escaped)
    } else {
        escaped
    }
}

fn word_info_header(pos_format: PosDumpFormat) -> &'static str {
    match pos_format {
        PosDumpFormat::Components => {
            "INDEX_FORM,LEFT_ID,RIGHT_ID,COST,HEADWORD,POS1,POS2,POS3,POS4,POS5,POS6,READING_FORM,NORMALIZED_FORM,DICTIONARY_FORM,SPLIT_A,SPLIT_B,SPLIT_C,WORD_STRUCTURE,SYNONYM_GROUPS,USER_DATA,REFERENCE_ID"
        }
        PosDumpFormat::Id => {
            "INDEX_FORM,LEFT_ID,RIGHT_ID,COST,HEADWORD,POS_ID,READING_FORM,NORMALIZED_FORM,DICTIONARY_FORM,SPLIT_A,SPLIT_B,SPLIT_C,WORD_STRUCTURE,SYNONYM_GROUPS,USER_DATA,REFERENCE_ID"
        }
    }
}

fn pos_string(grammar: &Grammar, posid: u16, pos_format: PosDumpFormat) -> String {
    match pos_format {
        PosDumpFormat::Components => grammar
            .pos_components(posid)
            .into_iter()
            .map(|p| csv_field(p))
            .collect::<Vec<_>>()
            .join(","),
        PosDumpFormat::Id => posid.to_string(),
    }
}

fn pos_string_for_entrykey(grammar: &Grammar, posid: u16, pos_format: PosDumpFormat) -> String {
    match pos_format {
        PosDumpFormat::Components => grammar
            .pos_components(posid)
            .into_iter()
            .map(|p| entrykey_ref_escape(p))
            .collect::<Vec<_>>()
            .join(","),
        PosDumpFormat::Id => posid.to_string(),
    }
}

fn dictionary_form_string(
    grammar: &Grammar,
    lex: &LexiconSet,
    self_wid: WordId,
    wid: WordId,
    pos_format: PosDumpFormat,
    system_reference_ids: &HashMap<u32, String>,
    user_reference_ids: &HashMap<u32, String>,
) -> SudachiResult<String> {
    if self_wid == wid {
        return Ok(String::new());
    }

    let dict_form_wi = lex.get_word_info(wid)?;
    Ok(format!(
        "\"{}\"",
        entrykey_ref_string(
            grammar,
            wid,
            dict_form_wi.headword(lex),
            dict_form_wi.pos_id(),
            dict_form_wi.reading_form(lex),
            pos_format,
            system_reference_ids,
            user_reference_ids,
        )
    ))
}

fn entrykey_ref_string(
    grammar: &Grammar,
    wid: WordId,
    headword: &str,
    pos_id: u16,
    reading: &str,
    pos_format: PosDumpFormat,
    system_reference_ids: &HashMap<u32, String>,
    user_reference_ids: &HashMap<u32, String>,
) -> String {
    let mut data = format!(
        "{},{},{}",
        entrykey_ref_escape(headword),
        pos_string_for_entrykey(grammar, pos_id, pos_format),
        entrykey_ref_escape(reading),
    );
    let reference_id = reference_id_for_word_id(wid, system_reference_ids, user_reference_ids);
    if !reference_id.is_empty() {
        data.push(',');
        data.push_str(&entrykey_ref_escape(&reference_id));
    }
    data
}

fn dump_wids<W: Write>(
    w: &mut W,
    grammar: &Grammar,
    lex: &LexiconSet,
    data: &[WordId],
    pos_format: PosDumpFormat,
    system_reference_ids: &HashMap<u32, String>,
    user_reference_ids: &HashMap<u32, String>,
) -> SudachiResult<()> {
    if data.is_empty() {
        return Ok(());
    }

    let mut refs = Vec::with_capacity(data.len());
    for wid in data {
        let wi = lex.get_word_info(*wid)?;
        refs.push(entrykey_ref_string(
            grammar,
            *wid,
            wi.headword(lex),
            wi.pos_id(),
            wi.reading_form(lex),
            pos_format,
            system_reference_ids,
            user_reference_ids,
        ));
    }
    w.write_all(b"\"")?;
    for (i, r) in refs.iter().enumerate() {
        write!(w, "{}", r)?;
        if i + 1 != refs.len() {
            w.write_all(b"/")?;
        }
    }
    w.write_all(b"\"")?;
    Ok(())
}

fn reference_id_for_word_id(
    wid: WordId,
    system_reference_ids: &HashMap<u32, String>,
    user_reference_ids: &HashMap<u32, String>,
) -> String {
    match wid.dict().as_raw() {
        0 => system_reference_ids
            .get(&wid.entry().as_raw())
            .cloned()
            .unwrap_or_default(),
        1 => user_reference_ids
            .get(&wid.entry().as_raw())
            .cloned()
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn dump_gids<W: Write>(w: &mut W, data: &[i32]) -> SudachiResult<()> {
    if data.is_empty() {
        write!(w, "")?;
        return Ok(());
    }
    for (i, e) in data.iter().enumerate() {
        write!(w, "{}", e)?;
        if i + 1 != data.len() {
            w.write_all(b"/")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use sudachi::dic::binary_loader::LoadedDictionary;

    const MATRIX_10_10: &[u8] = include_bytes!("../../sudachi/src/dic/build/test/matrix_10x10.def");
    const SYSTEM_CSV: &[u8] = include_bytes!("../../sudachi/tests/resources/lex.csv");
    const USER1_CSV: &[u8] = include_bytes!("../../sudachi/tests/resources/user1.csv");

    fn make_temp_path(stem: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "sudachi-cli-{stem}-{}-{nanos}.dic",
            std::process::id()
        ))
    }

    fn normalize_source_csv_for_dump(data: &[u8]) -> Vec<Vec<String>> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(data);
        let headers = rdr.headers().unwrap().clone();
        let has_user_data = headers.iter().any(|h| h == "user_data");
        let has_reference_id = headers.iter().any(|h| h == "reference_id");
        let mut rows = Vec::new();
        let dump_header = vec![
            "INDEX_FORM",
            "LEFT_ID",
            "RIGHT_ID",
            "COST",
            "HEADWORD",
            "POS1",
            "POS2",
            "POS3",
            "POS4",
            "POS5",
            "POS6",
            "READING_FORM",
            "NORMALIZED_FORM",
            "DICTIONARY_FORM",
            "SPLIT_A",
            "SPLIT_B",
            "SPLIT_C",
            "WORD_STRUCTURE",
            "SYNONYM_GROUPS",
            "USER_DATA",
            "REFERENCE_ID",
        ];
        rows.push(
            dump_header
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        );

        for rec in rdr.records() {
            let rec = rec.unwrap();
            let index = rec.get(0).unwrap();
            let head = rec.get(4).unwrap();
            let effective_head = if head.is_empty() {
                index.to_string()
            } else {
                head.to_string()
            };
            let reading = rec.get(11).unwrap();
            let effective_reading = reading.to_string();
            let norm = rec.get(12).unwrap();
            let dict = rec.get(13).unwrap();
            assert_ne!(
                dict, "*",
                "new-format CSV must not use '*' in dictionary_form"
            );

            let mut row = vec![
                index.to_string(),
                rec.get(1).unwrap().to_string(),
                rec.get(2).unwrap().to_string(),
                rec.get(3).unwrap().to_string(),
                if effective_head == index {
                    String::new()
                } else {
                    effective_head.clone()
                },
                rec.get(5).unwrap().to_string(),
                rec.get(6).unwrap().to_string(),
                rec.get(7).unwrap().to_string(),
                rec.get(8).unwrap().to_string(),
                rec.get(9).unwrap().to_string(),
                rec.get(10).unwrap().to_string(),
                effective_reading.clone(),
                if norm.is_empty() || norm == effective_head {
                    String::new()
                } else {
                    norm.to_string()
                },
                dict.to_string(),
                rec.get(14).unwrap().to_string(),
                rec.get(15).unwrap().to_string(),
                rec.get(16).unwrap().to_string(),
                rec.get(17).unwrap().to_string(),
                rec.get(18).unwrap().to_string(),
            ];
            row.push(if has_user_data {
                rec.get(19).unwrap().to_string()
            } else {
                String::new()
            });
            row.push(if has_reference_id {
                rec.get(if has_user_data { 20 } else { 19 })
                    .unwrap_or("")
                    .to_string()
            } else {
                String::new()
            });
            rows.push(row);
        }
        rows
    }

    #[test]
    fn csv_field_quotes_fields_with_comma() {
        assert_eq!(csv_field("a,b"), "\"a,b\"");
    }

    #[test]
    fn unicode_escape_replaces_double_quote() {
        assert_eq!(unicode_escape("a\"b"), "a\\u0022b");
    }

    #[test]
    fn entrykey_ref_escape_replaces_entrykey_separators() {
        assert_eq!(entrykey_ref_escape("a,b/c\"d"), "a\\u002cb\\u002fc\\u0022d");
    }

    fn parse_dump_csv(data: &[u8]) -> Vec<Vec<String>> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(data);
        rdr.records()
            .map(|r| r.unwrap().iter().map(|x| x.to_string()).collect())
            .collect()
    }

    fn normalize_pos_source_for_dump(data: &[u8]) -> Vec<Vec<String>> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(data);
        let mut rows = Vec::new();
        rows.push(
            ["POS_ID", "POS1", "POS2", "POS3", "POS4", "POS5", "POS6"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
        );
        for rec in rdr.records() {
            rows.push(rec.unwrap().iter().map(|x| x.to_string()).collect());
        }
        rows
    }

    #[test]
    fn dump_word_info_matches_system_csv() {
        let mut bldr = DictBuilder::new_system();
        bldr.read_conn(MATRIX_10_10).unwrap();
        bldr.read_lexicon(SYSTEM_CSV).unwrap();
        bldr.resolve().unwrap();
        let mut sys_bin = Vec::new();
        bldr.compile(&mut sys_bin).unwrap();

        let dict = BinaryDictionary::load_system(&sys_bin).unwrap();
        let mut dumped = Vec::new();
        dump_word_info(dict, None, PosDumpFormat::Components, &mut dumped).unwrap();

        let expected = normalize_source_csv_for_dump(SYSTEM_CSV);
        let actual = parse_dump_csv(&dumped);
        assert_eq!(actual, expected);
    }

    #[test]
    fn dump_word_info_matches_user_csv() {
        let mut sys_builder = DictBuilder::new_system();
        sys_builder.read_conn(MATRIX_10_10).unwrap();
        sys_builder.read_lexicon(SYSTEM_CSV).unwrap();
        sys_builder.resolve().unwrap();
        let mut sys_bin = Vec::new();
        sys_builder.compile(&mut sys_bin).unwrap();
        let loaded_sys = LoadedDictionary::load_system(&sys_bin).unwrap();

        let mut user_builder = DictBuilder::new_user(&loaded_sys);
        user_builder.read_lexicon(USER1_CSV).unwrap();
        user_builder.resolve().unwrap();
        let mut user_bin = Vec::new();
        user_builder.compile(&mut user_bin).unwrap();

        let system_path = make_temp_path("system");
        fs::write(&system_path, &sys_bin).unwrap();

        let user_dict = BinaryDictionary::load_user(&user_bin).unwrap();
        let mut dumped = Vec::new();
        dump_word_info(
            user_dict,
            Some(system_path.clone()),
            PosDumpFormat::Components,
            &mut dumped,
        )
        .unwrap();

        let _ = fs::remove_file(system_path);

        let expected = normalize_source_csv_for_dump(USER1_CSV);
        let actual = parse_dump_csv(&dumped);
        assert_eq!(actual, expected);
    }

    #[test]
    fn dump_pos_matches_canonical_pos_csv() {
        let pos = concat!(
            "POS_ID,POS1,POS2,POS3,POS4,POS5,POS6\n",
            "0,助詞,接続助詞,*,*,*,*\n",
            "1,名詞,普通名詞,一般,*,*,*\n"
        );
        let lex = concat!(
            "index_form,left_id,right_id,cost,headword,pos_id,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
            "京都,6,6,5293,京都,1,キョウト,京都,,A,,,,\n"
        );

        let mut builder = DictBuilder::new_system();
        builder.read_conn(MATRIX_10_10).unwrap();
        builder.read_pos(pos.as_bytes()).unwrap();
        builder.read_lexicon(lex.as_bytes()).unwrap();
        builder.resolve().unwrap();

        let mut compiled = Vec::new();
        builder.compile(&mut compiled).unwrap();

        let dict = BinaryDictionary::load_system(&compiled).unwrap();
        let mut dumped = Vec::new();
        dump_pos(dict, &mut dumped);

        let expected = normalize_pos_source_for_dump(pos.as_bytes());
        let actual = parse_dump_csv(&dumped);
        assert_eq!(actual, expected);
    }

    #[test]
    fn dump_word_info_uses_pos_id_for_columns_and_entrykey_refs() {
        let pos = concat!(
            "POS_ID,POS1,POS2,POS3,POS4,POS5,POS6\n",
            "0,名詞,固有名詞,地名,一般,*,*\n",
            "1,名詞,普通名詞,一般,*,*,*\n"
        );
        let lex = concat!(
            "INDEX_FORM,LEFT_ID,RIGHT_ID,COST,HEADWORD,POS_ID,READING_FORM,NORMALIZED_FORM,DICTIONARY_FORM,MODE,SPLIT_A,SPLIT_B,SPLIT_C,WORD_STRUCTURE,SYNONYM_GROUPS\n",
            "京都,6,6,5293,京都,0,キョウト,京都,,A,,,,,\n",
            "府,2,2,2914,,1,フ,,,A,,,,,\n",
            "東京府,2,2,2816,,0,トウキョウフ,,,B,\"京都,0,キョウト/府,1,フ\",,,,\n"
        );

        let mut builder = DictBuilder::new_system();
        builder.read_conn(MATRIX_10_10).unwrap();
        builder.read_pos(pos.as_bytes()).unwrap();
        builder.read_lexicon(lex.as_bytes()).unwrap();
        builder.resolve().unwrap();

        let mut compiled = Vec::new();
        builder.compile(&mut compiled).unwrap();

        let dict = BinaryDictionary::load_system(&compiled).unwrap();
        let mut dumped = Vec::new();
        dump_word_info(dict, None, PosDumpFormat::Id, &mut dumped).unwrap();

        let expected = vec![
            vec![
                "INDEX_FORM",
                "LEFT_ID",
                "RIGHT_ID",
                "COST",
                "HEADWORD",
                "POS_ID",
                "READING_FORM",
                "NORMALIZED_FORM",
                "DICTIONARY_FORM",
                "SPLIT_A",
                "SPLIT_B",
                "SPLIT_C",
                "WORD_STRUCTURE",
                "SYNONYM_GROUPS",
                "USER_DATA",
                "REFERENCE_ID",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
            vec![
                "京都",
                "6",
                "6",
                "5293",
                "",
                "0",
                "キョウト",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
            vec![
                "府", "2", "2", "2914", "", "1", "フ", "", "", "", "", "", "", "", "", "",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
            vec![
                "東京府",
                "2",
                "2",
                "2816",
                "",
                "0",
                "トウキョウフ",
                "",
                "",
                "京都,0,キョウト/府,1,フ",
                "",
                "",
                "",
                "",
                "",
                "",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        ];
        let actual = parse_dump_csv(&dumped);
        assert_eq!(actual, expected);
    }

    #[test]
    fn dump_word_info_outputs_reference_id_column_and_entrykey_refs() {
        let mut builder = DictBuilder::new_system();
        builder.read_conn(MATRIX_10_10).unwrap();
        builder
            .read_lexicon(
                concat!(
                    "INDEX_FORM,LEFT_ID,RIGHT_ID,COST,HEADWORD,POS1,POS2,POS3,POS4,POS5,POS6,READING_FORM,NORMALIZED_FORM,DICTIONARY_FORM,MODE,SPLIT_A,SPLIT_B,SPLIT_C,WORD_STRUCTURE,SYNONYM_GROUPS,REFERENCE_ID\n",
                    "東京,6,6,5293,東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,,,,tokyo-ref\n",
                    "東京府,2,2,2816,,名詞,固有名詞,地名,一般,*,*,トウキョウフ,\"東京,0,トウキョウ,tokyo-ref\",\"東京,0,トウキョウ,tokyo-ref\",B,\"東京,0,トウキョウ,tokyo-ref\",,,,,\n"
                )
                .as_bytes(),
            )
            .unwrap();
        builder.resolve().unwrap();

        let mut compiled = Vec::new();
        builder.compile(&mut compiled).unwrap();

        let dict = BinaryDictionary::load_system(&compiled).unwrap();
        let mut dumped = Vec::new();
        dump_word_info(dict, None, PosDumpFormat::Components, &mut dumped).unwrap();

        let actual = parse_dump_csv(&dumped);
        assert_eq!(actual[0].last().map(|s| s.as_str()), Some("REFERENCE_ID"));
        assert_eq!(actual[1].last().map(|s| s.as_str()), Some("tokyo-ref"));
        assert!(actual[2]
            .iter()
            .any(|v| v == "東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,tokyo-ref"));
    }

    #[test]
    fn dump_word_info_pos_id_outputs_reference_id_entrykey_refs() {
        let pos = concat!(
            "POS_ID,POS1,POS2,POS3,POS4,POS5,POS6\n",
            "0,名詞,固有名詞,地名,一般,*,*\n"
        );
        let lex = concat!(
            "INDEX_FORM,LEFT_ID,RIGHT_ID,COST,HEADWORD,POS_ID,READING_FORM,NORMALIZED_FORM,DICTIONARY_FORM,MODE,SPLIT_A,SPLIT_B,SPLIT_C,WORD_STRUCTURE,SYNONYM_GROUPS,REFERENCE_ID\n",
            "東京,6,6,5293,東京,0,トウキョウ,,,A,,,,,,tokyo-ref\n",
            "東京府,2,2,2816,,0,トウキョウフ,,\"東京,0,トウキョウ,tokyo-ref\",B,\"東京,0,トウキョウ,tokyo-ref\",,,,,\n"
        );

        let mut builder = DictBuilder::new_system();
        builder.read_conn(MATRIX_10_10).unwrap();
        builder.read_pos(pos.as_bytes()).unwrap();
        builder.read_lexicon(lex.as_bytes()).unwrap();
        builder.resolve().unwrap();

        let mut compiled = Vec::new();
        builder.compile(&mut compiled).unwrap();

        let dict = BinaryDictionary::load_system(&compiled).unwrap();
        let mut dumped = Vec::new();
        dump_word_info(dict, None, PosDumpFormat::Id, &mut dumped).unwrap();

        let actual = parse_dump_csv(&dumped);
        assert_eq!(actual[0].last().map(|s| s.as_str()), Some("REFERENCE_ID"));
        assert_eq!(actual[1].last().map(|s| s.as_str()), Some("tokyo-ref"));
        assert_eq!(actual[2][8], "東京,0,トウキョウ,tokyo-ref");
        assert_eq!(actual[2][9], "東京,0,トウキョウ,tokyo-ref");
    }

    #[test]
    fn dump_description_matches_expected_system_format() {
        let mut builder = DictBuilder::new_system();
        builder.set_compile_time(UNIX_EPOCH + Duration::from_secs(1));
        builder.set_description("system comment");
        builder.read_conn(MATRIX_10_10).unwrap();
        builder.read_lexicon(SYSTEM_CSV).unwrap();
        builder.resolve().unwrap();

        let mut compiled = Vec::new();
        builder.compile(&mut compiled).unwrap();

        let path = Path::new("/tmp/system.dic");
        let dict = BinaryDictionary::load_system(&compiled).unwrap();
        let desc = dict.description.clone();
        let mut dumped = Vec::new();
        dump_description(path, &desc, &mut dumped).unwrap();

        let mut expected = String::new();
        expected.push_str(&format!("File: {}\n", path.display()));
        expected.push_str("type: system dictionary\n");
        expected.push_str("Creation time: 1970-01-01T00:00:01Z\n");
        expected.push_str("Comment: system comment\n");
        expected.push_str(&format!("Signature: {}\n", desc.signature()));
        expected.push_str("Reference: \n");
        expected.push_str(&format!("Entries total: {}\n", desc.num_total_entries()));
        expected.push_str(&format!(
            "Entries indexed: {}\n",
            desc.num_indexed_entries()
        ));
        for block in desc.blocks() {
            expected.push_str(&format!(
                "Block {}: {} - {}\n",
                block.name(),
                block.start(),
                block.end()
            ));
        }
        expected.push_str("Flag isRuntimeCosts: false\n");

        assert_eq!(String::from_utf8(dumped).unwrap(), expected);
    }

    #[test]
    fn dump_description_marks_user_dictionary() {
        let mut sys_builder = DictBuilder::new_system();
        sys_builder.set_compile_time(UNIX_EPOCH + Duration::from_secs(1));
        sys_builder.set_description("system comment");
        sys_builder.read_conn(MATRIX_10_10).unwrap();
        sys_builder.read_lexicon(SYSTEM_CSV).unwrap();
        sys_builder.resolve().unwrap();
        let mut sys_bin = Vec::new();
        sys_builder.compile(&mut sys_bin).unwrap();
        let loaded_sys = LoadedDictionary::load_system(&sys_bin).unwrap();

        let mut user_builder = DictBuilder::new_user(&loaded_sys);
        user_builder.set_compile_time(UNIX_EPOCH + Duration::from_secs(2));
        user_builder.set_description("user comment");
        user_builder.read_lexicon(USER1_CSV).unwrap();
        user_builder.resolve().unwrap();
        let mut user_bin = Vec::new();
        user_builder.compile(&mut user_bin).unwrap();

        let path = Path::new("/tmp/user.dic");
        let dict = BinaryDictionary::load_user(&user_bin).unwrap();
        let desc = dict.description.clone();
        let mut dumped = Vec::new();
        dump_description(path, &desc, &mut dumped).unwrap();
        let dumped = String::from_utf8(dumped).unwrap();

        assert!(dumped.contains(&format!("File: {}\n", path.display())));
        assert!(dumped.contains("type: user dictionary\n"));
        assert!(dumped.contains("Creation time: 1970-01-01T00:00:02Z\n"));
        assert!(dumped.contains("Comment: user comment\n"));
        assert!(dumped.contains(&format!("Signature: {}\n", desc.signature())));
        assert!(dumped.contains(&format!("Reference: {}\n", desc.reference())));
    }
}
