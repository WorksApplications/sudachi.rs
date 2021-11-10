/*
 * Copyright (c) 2021 Works Applications Co., Ltd.
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

mod analysis;
mod build;
mod output;

use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::str::FromStr;

use structopt::StructOpt;

use crate::analysis::{Analysis, AnalyzeNonSplitted, AnalyzeSplitted, SplitSentencesOnly};
use crate::build::{build_main, is_build_mode};
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::prelude::*;

#[cfg(feature = "bake_dictionary")]
const BAKED_DICTIONARY_BYTES: &[u8] = include_bytes!(env!("SUDACHI_DICT_PATH"));

#[derive(StructOpt, Debug, Eq, PartialEq)]
pub enum SentenceSplitMode {
    /// Do both sentence splitting and analysis
    Default,
    /// Do only sentence splitting and not analysis
    Only,
    /// Do only analysis without sentence splitting
    None,
}

impl Default for SentenceSplitMode {
    fn default() -> Self {
        SentenceSplitMode::Default
    }
}

impl FromStr for SentenceSplitMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "yes" | "default" => Ok(SentenceSplitMode::Default),
            "no" | "none" => Ok(SentenceSplitMode::None),
            "only" => Ok(SentenceSplitMode::Only),
            _ => Err("invalid sentence split mode: allowed values - yes, default, no, none, only"),
        }
    }
}

/// A Japanese tokenizer
///
/// If you are looking for options for the dictionary building, try sudachi build/ubuild --help.
#[derive(StructOpt)]
#[structopt(name = "sudachi")]
struct Cli {
    /// Input text file: If not present, read from STDIN
    #[structopt(parse(from_os_str))]
    file: Option<PathBuf>,

    /// Path to the setting file in JSON format
    #[structopt(short = "r", long = "config-file", parse(from_os_str))]
    config_file: Option<PathBuf>,

    /// Path to the root directory of resources
    #[structopt(short = "p", long = "resource_dir", parse(from_os_str))]
    resource_dir: Option<PathBuf>,

    /// Split unit: "A" (short), "B" (middle), or "C" (Named Entity)
    #[structopt(short = "m", long = "mode", default_value = "C")]
    mode: Mode,

    // Output text file: If not present, use stdout
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output_file: Option<PathBuf>,

    /// Prints all fields
    #[structopt(short = "a", long = "all")]
    print_all: bool,

    /// Outputs only surface form
    #[structopt(short = "w", long = "wakati")]
    wakati: bool,

    /// Debug mode: Print the debug information
    #[structopt(short = "d", long = "debug")]
    enable_debug: bool,

    /// Path to sudachi dictionary.
    /// If None, it refer config and then baked dictionary
    #[structopt(short = "l", long = "dict")]
    dictionary_path: Option<PathBuf>,

    /// How to split sentences.
    ///
    /// "yes", "default" means split sentences,
    /// "no", "none" means don't split sentences,
    /// "only" means split sentences, do not perform analysis
    #[structopt(long = "split-sentences", default_value = "yes")]
    split_sentences: SentenceSplitMode,
}

// want to instantiate a different type for different output format
// this takes a f as a function which will be created with a different actual type
macro_rules! with_output {
    ($cli: expr, $f: expr) => {
        if $cli.wakati {
            Box::new($f(output::Wakachi::default()))
        } else {
            Box::new($f(output::Simple::new($cli.print_all)))
        }
    };
}

fn main() {
    if is_build_mode() {
        build_main();
        return;
    }

    let args: Cli = Cli::from_args();

    let inner_reader: Box<dyn Read> = match args.file.as_ref() {
        Some(input_path) => Box::new(
            File::open(input_path)
                .unwrap_or_else(|_| panic!("Failed to open input file {:?}", &input_path)),
        ),
        None => Box::new(io::stdin()),
    };

    // input: stdin or file
    let mut reader = BufReader::new(inner_reader);

    // output: stdout or file
    let inner_writer: Box<dyn Write> = match &args.output_file {
        Some(output_path) => Box::new(
            File::create(&output_path)
                .unwrap_or_else(|_| panic!("Failed to open output file {:?}", &output_path)),
        ),
        None => Box::new(io::stdout()),
    };
    let mut writer = BufWriter::new(inner_writer);

    // load config file
    let config = Config::new(
        args.config_file.clone(),
        args.resource_dir.clone(),
        args.dictionary_path.clone(),
    )
    .expect("Failed to load config file");

    let dict = JapaneseDictionary::from_cfg(&config)
        .unwrap_or_else(|e| panic!("Failed to create dictionary: {:?}", e));

    let mut analyzer: Box<dyn Analysis> = match args.split_sentences {
        SentenceSplitMode::Only => Box::new(SplitSentencesOnly::new(&dict)),
        SentenceSplitMode::Default => with_output!(args, |o| {
            AnalyzeSplitted::new(o, &dict, args.mode, args.enable_debug)
        }),
        SentenceSplitMode::None => with_output!(args, |o| {
            AnalyzeNonSplitted::new(o, &dict, args.mode, args.enable_debug)
        }),
    };

    let mut data = String::with_capacity(4 * 1024);
    let is_stdout = args.output_file.is_none();

    // tokenize and output results
    while reader.read_line(&mut data).expect("readline failed") > 0 {
        let no_eol = strip_eol(&data);
        analyzer.analyze(no_eol, &mut writer);
        if is_stdout {
            // for stdout we want to flush every result
            writer.flush().expect("flush failed");
        }
        data.clear();
    }

    // it is recommended to call write before dropping BufWriter
    writer.flush().expect("flush failed");
}

/// strip (\r?\n)? pattern at the end of string
fn strip_eol(data: &str) -> &str {
    let mut bytes = data.as_bytes();
    let mut len = bytes.len();
    if len > 1 && bytes[len - 1] == b'\n' {
        len = len - 1;
        bytes = &bytes[..len];
        if len > 1 && bytes[len - 1] == b'\r' {
            len = len - 1;
            bytes = &bytes[..len];
        }
    }

    // Safety: str was correct and we only removed full characters
    unsafe { std::str::from_utf8_unchecked(bytes) }
}
