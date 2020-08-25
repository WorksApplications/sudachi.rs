use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process;

use structopt::StructOpt;

use sudachi::prelude::*;

#[cfg(feature = "bake_dictionary")]
const BAKED_DICTIONARY_BYTES: &[u8] = include_bytes!(env!("SUDACHI_DICT_PATH"));

/// A Japanese tokenizer
#[derive(StructOpt)]
#[structopt(name = "sudachi", author = "")]
struct Cli {
    /// Input text file: If not present, read from STDIN
    #[structopt(parse(from_os_str))]
    file: Option<PathBuf>,

    /// Split unit: "A" (short), "B" (middle), or "C" (Named Entity)
    #[structopt(short = "m", long = "mode", default_value = "C")]
    mode: String,

    /// Prints all fields
    #[structopt(short = "a", long = "all")]
    print_all: bool,

    /// Outputs only surface form
    #[structopt(short = "w", long = "wakati")]
    wakati: bool,

    /// Debug mode: Dumps lattice
    #[structopt(short = "d", long = "debug")]
    enable_debug: bool,

    // Dictionary is optional if baked in
    /// Path to sudachi dictionary
    #[cfg(feature = "bake_dictionary")]
    #[structopt(short = "l", long = "dict")]
    dictionary_path: Option<PathBuf>,

    // Dictionary is not baked in, so it must be specified
    /// Path to sudachi dictionary
    #[cfg(not(feature = "bake_dictionary"))]
    #[structopt(short = "l", long = "dict")]
    dictionary_path: PathBuf,
}

fn get_dictionary_bytes(args: &Cli) -> Cow<'static, [u8]> {
    let dictionary_path = {
        cfg_if::cfg_if! {
            if #[cfg(feature="bake_dictionary")] {
                if let Some(dictionary_path) = &args.dictionary_path {
                    dictionary_path
                } else {
                    return Cow::Borrowed(BAKED_DICTIONARY_BYTES);
                }
            } else {
                &args.dictionary_path
            }
        }
    };

    let dictionary_stat = fs::metadata(&dictionary_path).expect("Unable to stat dictionary");
    let mut dictionary_file = File::open(dictionary_path).expect("Unable to open dictionary");
    let mut storage_buf = Vec::with_capacity(dictionary_stat.len() as usize);
    dictionary_file
        .read_to_end(&mut storage_buf)
        .expect("Failed to read from dictionary file");
    Cow::Owned(storage_buf)
}

fn main() {
    let args = Cli::from_args();
    let mode = match args.mode.as_str() {
        "A" | "a" => Mode::A,
        "B" | "b" => Mode::B,
        "C" | "c" => Mode::C,
        _ => {
            eprintln!("Invalid mode: Mode must be one of \"A\", \"B\", or \"C\" (in lower or upper case).");
            process::exit(1);
        }
    };
    let print_all = args.print_all;
    let wakati = args.wakati;
    let enable_debug = args.enable_debug;

    // load and parse dictionary binary to create a tokenizer

    let dictionary_bytes = get_dictionary_bytes(&args);
    let tokenizer = Tokenizer::from_dictionary_bytes(&dictionary_bytes)
        .expect("Failed to create Tokenizer from dictionary bytes");

    // input: stdin or file
    let reader: Box<dyn BufRead> = match args.file {
        None => Box::new(BufReader::new(io::stdin())),
        Some(input_path) => Box::new(BufReader::new(
            fs::File::open(&input_path)
                .unwrap_or_else(|_| panic!("Failed to open file {:?}", &input_path)),
        )),
    };

    for line in reader.lines() {
        let input = line.expect("Failed to reead line").to_string();
        let morpheme_list = tokenizer
            .tokenize(&input, mode, enable_debug)
            .expect("failed to tokenize input");

        if wakati {
            let surface_list = morpheme_list
                .iter()
                .map(|m| m.surface().to_string())
                .collect::<Vec<_>>();
            println!("{}", surface_list.join(" "));
        } else {
            for morpheme in morpheme_list {
                print!(
                    "{}\t{}\t{}",
                    morpheme.surface(),
                    morpheme.pos().expect("Missing part of speech").join(","),
                    morpheme.normalized_form(),
                );
                if print_all {
                    print!(
                        "\t{}\t{}",
                        morpheme.dictionary_form(),
                        morpheme.reading_form(),
                        // TODO: is_oov
                    );
                }
                println!();
            }
            println!("EOS");
        }
    }
}
