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

mod output;

use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process;

use structopt::StructOpt;

use crate::output::SudachiOutput;
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::analysis::stateless_tokenizer::DictionaryAccess;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::prelude::*;
use sudachi::sentence_splitter::{SentenceSplitter, SplitSentences};

#[cfg(feature = "bake_dictionary")]
const BAKED_DICTIONARY_BYTES: &[u8] = include_bytes!(env!("SUDACHI_DICT_PATH"));

/// A Japanese tokenizer
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
    mode: String,

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

    /// Only split sentences, do not perform analysis
    #[structopt(long = "only-split-sentences")]
    only_split_sentences: bool,
}

fn main() {
    let args: Cli = Cli::from_args();

    let mode = match args.mode.as_str().parse() {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!("Invalid mode: {}", err);
            process::exit(1);
        }
    };

    let enable_debug = args.enable_debug;

    // input: stdin or file
    let reader: Box<dyn BufRead> = match &args.file {
        Some(input_path) => Box::new(BufReader::new(
            File::open(&input_path)
                .unwrap_or_else(|_| panic!("Failed to open input file {:?}", &input_path)),
        )),
        None => Box::new(BufReader::new(io::stdin())),
    };

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
    let mut tokenizer = StatefulTokenizer::create(&dict, enable_debug, mode);
    let splitter = SentenceSplitter::with_limit(32 * 1024);
    let mut morphemes = MorphemeList::empty(&dict);

    let is_stdout = args.output_file.is_none();

    let format = make_output::<&JapaneseDictionary>(&args);

    // tokenize and output results
    for line in reader.lines() {
        let input = line.expect("Failed to read line");
        for (_, sentence) in splitter.split(&input) {
            if args.only_split_sentences {
                writeln!(&mut writer, "{}", sentence).expect("Failed to write output");
                continue;
            }
            tokenizer.reset().push_str(sentence);
            tokenizer.do_tokenize().expect("Failed to tokenize input");

            morphemes
                .collect_results(&mut tokenizer)
                .expect("failed to collect results");

            format
                .write(&mut writer, &morphemes)
                .expect("Failed to write output");
        }
        if is_stdout {
            writer.flush().expect("flush failed");
        }
    }
    // it is recommended to call write before dropping BufWriter
    writer.flush().expect("flush failed");
}

fn make_output<T: DictionaryAccess>(cli: &Cli) -> Box<dyn SudachiOutput<T>> {
    if cli.wakati {
        Box::new(output::Wakachi::default())
    } else {
        Box::new(output::Simple::new(cli.print_all))
    }
}
