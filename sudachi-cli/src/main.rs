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

mod analysis;
mod build;
mod output;

use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::str::FromStr;

use clap::Parser;

use crate::analysis::{Analysis, AnalyzeNonSplitted, AnalyzeSplitted, SplitSentencesOnly};
use crate::build::{build_main, is_build_mode, BuildCli};
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::dic::DictionaryAccess;
use sudachi::prelude::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SentenceSplitMode {
    /// Do both sentence splitting and analysis
    #[default]
    Default,
    /// Do only sentence splitting and not analysis
    Only,
    /// Do only analysis without sentence splitting
    None,
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
#[derive(Parser)]
#[command(
    name = "sudachi",
    version,
    next_line_help = true,
    propagate_version = true
)]
struct Cli {
    /// Input text file: If not present, read from STDIN
    file: Option<PathBuf>,

    /// Path to the setting file in JSON format
    #[arg(short = 'r', long = "config-file")]
    config_file: Option<PathBuf>,

    /// Path to the root directory of resources
    #[arg(short = 'p', long = "resource_dir")]
    resource_dir: Option<PathBuf>,

    /// Split unit: "A" (short), "B" (middle), or "C" (Named Entity)
    #[arg(short = 'm', long = "mode", default_value = "C")]
    mode: Mode,

    /// Output text file: If not present, use stdout
    #[arg(short = 'o', long = "output")]
    output_file: Option<PathBuf>,

    /// Prints all fields
    #[arg(short = 'a', long = "all")]
    print_all: bool,

    /// Outputs only surface form
    #[arg(short = 'w', long = "wakati")]
    wakati: bool,

    /// Debug mode: Print the debug information
    #[arg(short = 'd', long = "debug")]
    enable_debug: bool,

    /// Path to sudachi dictionary.
    /// If None, it refer config and then baked dictionary
    #[arg(short = 'l', long = "dict")]
    dictionary_path: Option<PathBuf>,

    /// How to split sentences.
    ///
    /// "yes", "default" means split sentences,
    /// "no", "none" means don't split sentences,
    /// "only" means split sentences, do not perform analysis
    #[arg(long = "split-sentences", default_value = "yes")]
    split_sentences: SentenceSplitMode,

    #[command(subcommand)]
    command: Option<BuildCli>,
}

pub fn setup_output<D: DictionaryAccess>(
    wakachi: bool,
    print_all: bool,
) -> Box<dyn output::SudachiOutput<D>> {
    if wakachi {
        Box::new(output::Wakachi::default())
    } else {
        Box::new(output::Simple::new(print_all))
    }
}

fn setup_analyzer<'a>(args: &Cli, dict: &'a impl DictionaryAccess) -> Box<dyn Analysis + 'a> {
    match args.split_sentences {
        SentenceSplitMode::Only => Box::new(SplitSentencesOnly::new(dict)),
        SentenceSplitMode::Default => Box::new(AnalyzeSplitted::new(
            setup_output(args.wakati, args.print_all),
            dict,
            args.mode,
            args.enable_debug,
        )),
        SentenceSplitMode::None => Box::new(AnalyzeNonSplitted::new(
            setup_output(args.wakati, args.print_all),
            dict,
            args.mode,
            args.enable_debug,
        )),
    }
}

fn main() {
    let args: Cli = Cli::parse();

    if is_build_mode(&args.command) {
        build_main(args.command.unwrap());
        return;
    }

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
            File::create(output_path)
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

    let mut analyzer: Box<dyn Analysis> = setup_analyzer(&args, &dict);

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
        len -= 1;
        bytes = &bytes[..len];
        if len > 1 && bytes[len - 1] == b'\r' {
            len -= 1;
            bytes = &bytes[..len];
        }
    }

    // Safety: str was correct and we only removed full characters
    unsafe { std::str::from_utf8_unchecked(bytes) }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    /// Verify that the CLI definition is valid.
    #[test]
    fn verify_cli() {
        Cli::command().debug_assert()
    }
}
