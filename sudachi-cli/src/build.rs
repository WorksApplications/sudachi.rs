/*
 *  Copyright (c) 2021 Works Applications Co., Ltd.
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

use memmap2::Mmap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use structopt::StructOpt;
use sudachi::analysis::stateless_tokenizer::DictionaryAccess;
use sudachi::config::Config;
use sudachi::dic::build::report::DictPartReport;
use sudachi::dic::build::DictBuilder;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::dic::grammar::Grammar;
use sudachi::dic::lexicon_set::LexiconSet;
use sudachi::dic::word_id::WordId;
use sudachi::dic::DictionaryLoader;
use sudachi::error::SudachiResult;

/// Check that the first argument is a subcommand and the file with the same name does
/// not exists.
/// If the file does exists, probably it's safer to use default Sudachi analysis mode.
pub fn is_build_mode() -> bool {
    let mut args = std::env::args_os();
    let _ = args.next();
    let arg = args.next();
    match arg {
        Some(x) => {
            if !(x == "build" || x == "ubuild" || x == "dump") {
                return false;
            }

            if Path::new(&x).exists() {
                false
            } else {
                true
            }
        }
        None => false,
    }
}

#[derive(StructOpt)]
#[structopt(name = "sudachi")]
enum BuildCli {
    /// Builds system dictionary
    #[structopt(name = "build")]
    System {
        #[structopt(flatten)]
        common: BuildCmd,

        /// Path to matrix definition
        #[structopt(short, long, parse(from_os_str))]
        matrix: PathBuf,
    },

    /// Builds user dictionary
    #[structopt(name = "ubuild")]
    User {
        #[structopt(flatten)]
        common: BuildCmd,

        /// Path to system dictionary
        #[structopt(short = "s", long = "system")]
        dictionary: PathBuf,
    },

    Dump {
        dict: PathBuf,
        part: String,
        output: PathBuf,
    },
}

#[derive(StructOpt)]
struct BuildCmd {
    /// Input csv files
    #[structopt(required = true, parse(from_os_str))]
    inputs: Vec<PathBuf>,

    /// Where to place compiled dictionary.
    /// If there was an existing one it will be overwritten.
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output_file: PathBuf,

    /// Description string to embed into dictionary
    #[structopt(short, long, default_value = "")]
    description: String,
}

pub fn build_main() {
    let args: BuildCli = BuildCli::from_args();

    match args {
        BuildCli::System { common, matrix } => build_system(common, matrix),
        BuildCli::User { common, dictionary } => build_user(common, dictionary),
        BuildCli::Dump { dict, part, output } => dump_part(dict, part, output),
    }
}

fn build_system(mut cmd: BuildCmd, matrix: PathBuf) {
    let mut builder = DictBuilder::new_system();
    builder.set_description(std::mem::take(&mut cmd.description));
    builder
        .read_conn(matrix.as_path())
        .expect("failed to read matrix");
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
    let cfg =
        Config::new(None, None, Some(system)).expect("failed to create default configuration");
    let dict = JapaneseDictionary::from_cfg(&cfg).expect("failed to load system dictionary");

    let mut builder = DictBuilder::new_user(&dict);
    builder.set_description(std::mem::take(&mut cmd.description));
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
        .open(&p)
        .unwrap_or_else(|e| panic!("failed to open {:?} for writing:\n{:?}", p, e))
}

fn dump_part(dict: PathBuf, part: String, output: PathBuf) {
    let file = File::open(&dict).expect("open failed");
    let data = unsafe { Mmap::map(&file) }.expect("mmap failed");
    let loader =
        unsafe { DictionaryLoader::read_any_dictionary(&data) }.expect("failed to load dictionary");
    let dict = loader.to_loaded().expect("should contain grammar");

    let outf = output_file(&output);
    let mut writer = BufWriter::new(outf);

    match part.as_str() {
        "pos" => dump_pos(dict.grammar(), &mut writer),
        "matrix" => dump_matrix(dict.grammar(), &mut writer),
        "winfo" => dump_word_info(dict.lexicon(), &mut writer).unwrap(),
        _ => unimplemented!(),
    }
    writer.flush().unwrap();
}

fn dump_pos<W: Write>(grammar: &Grammar, w: &mut W) {
    for p in grammar.pos_list.iter() {
        for (i, e) in p.iter().enumerate() {
            w.write_all(e.as_bytes()).unwrap();
            if (i + 1) == p.len() {
                w.write_all(b"\n").unwrap();
            } else {
                w.write_all(b",").unwrap();
            }
        }
    }
}

fn dump_matrix<W: Write>(grammar: &Grammar, w: &mut W) {
    let conn = grammar.conn_matrix();
    write!(w, "{} {}", conn.num_left(), conn.num_right()).unwrap();

    for left in 0..conn.num_left() {
        for right in 0..conn.num_right() {
            let cost = conn.cost(left as _, right as _);
            write!(w, "{} {} {}\n", left, right, cost).unwrap();
        }
    }
}

fn dump_word_info<W: Write>(lex: &LexiconSet, w: &mut W) -> SudachiResult<()> {
    let size = lex.size();
    for i in 0..size {
        let wid = WordId::checked(0, i)?;
        let (left, right, cost) = lex.get_word_param(wid);
        let winfo = lex.get_word_info(wid)?;
        write!(w, "{},{},{},", left, right, cost)?;
        write!(w, "{},", winfo.surface())?;
        write!(w, "{},", winfo.head_word_length())?;
        write!(w, "{},", winfo.normalized_form())?;
        write!(w, "{},", winfo.dictionary_form_word_id())?;
        write!(w, "{},", winfo.reading_form())?;
        dump_wids(w, winfo.a_unit_split())?;
        w.write_all(b",")?;
        dump_wids(w, winfo.b_unit_split())?;
        w.write_all(b",")?;
        dump_wids(w, winfo.word_structure())?;
        w.write_all(b",")?;
        dump_gids(w, winfo.synonym_group_ids())?;
        w.write_all(b"\n")?;
    }
    Ok(())
}

fn dump_wids<W: Write>(w: &mut W, data: &[WordId]) -> SudachiResult<()> {
    for (i, e) in data.iter().enumerate() {
        let prefix = match e.dic() {
            0 => "",
            _ => "U",
        };
        write!(w, "{}{}", prefix, e.word())?;
        if i + 1 != data.len() {
            w.write_all(b"/")?;
        }
    }
    Ok(())
}

fn dump_gids<W: Write>(w: &mut W, data: &[u32]) -> SudachiResult<()> {
    for (i, e) in data.iter().enumerate() {
        write!(w, "{}", e)?;
        if i + 1 != data.len() {
            w.write_all(b"/")?;
        }
    }
    Ok(())
}
