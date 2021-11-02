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

use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use structopt::StructOpt;
use sudachi::config::Config;
use sudachi::dic::build::DictBuilder;
use sudachi::dic::dictionary::JapaneseDictionary;

pub fn is_build_mode() -> bool {
    let mut args = std::env::args_os();
    let _ = args.next();
    let arg = args.next();
    match arg {
        Some(x) => {
            if !(x == "build" || x == "ubuild") {
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
        #[structopt(parse(from_os_str))]
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
}

#[derive(StructOpt)]
struct BuildCmd {
    /// Input csv files
    #[structopt(parse(from_os_str))]
    inputs: Vec<PathBuf>,

    /// Output text file: If not present, use stdout
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output_file: PathBuf,

    /// Description string to embed into dictionary
    #[structopt(short = "d", long = "description")]
    description: String,
}

pub fn build_main() {
    let args: BuildCli = BuildCli::from_args();

    match args {
        BuildCli::System { common, matrix } => build_system(common, matrix),
        BuildCli::User { common, dictionary } => build_user(common, dictionary),
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
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&cmd.output_file)
        .unwrap_or_else(|e| panic!("failed to open {:?} for writing\n{:?}", cmd.output_file, e));
    let mut buf_writer = BufWriter::with_capacity(16 * 1024, file);
    builder
        .compile(&mut buf_writer)
        .expect("failed to compile dictionary");
    buf_writer.flush().expect("failed to flush");
}

fn build_user(mut cmd: BuildCmd, system: PathBuf) {
    let mut cfg = Config::default();
    cfg.system_dict = Some(system);
    let dict = JapaneseDictionary::from_cfg(&cfg).expect("failed to load system dictionary");

    let mut builder = DictBuilder::new_user(&dict);
    builder.set_description(std::mem::take(&mut cmd.description));
    for d in cmd.inputs.iter() {
        builder
            .read_lexicon(d.as_path())
            .unwrap_or_else(|e| panic!("failed to read {:?}\n{:?}", d, e));
    }
    builder.resolve().expect("failed to resolve references");
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&cmd.output_file)
        .unwrap_or_else(|e| panic!("failed to open {:?} for writing\n{:?}", cmd.output_file, e));
    let mut buf_writer = BufWriter::with_capacity(16 * 1024, file);
    builder
        .compile(&mut buf_writer)
        .expect("failed to compile dictionary");
    buf_writer.flush().expect("failed to flush");
}
