use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::process;

use structopt::StructOpt;

use sudachi::tokenizer::Mode;
use sudachi::tokenizer::Tokenizer;

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

    // embed dictionary binary file
    let bytes = include_bytes!("resources/system.dic");

    let tokenizer = Tokenizer::new(bytes);

    // input: stdin or file
    let reader: Box<dyn BufRead> = match args.file {
        None => Box::new(BufReader::new(io::stdin())),
        Some(input_path) => Box::new(BufReader::new(fs::File::open(input_path).unwrap())),
    };

    for line in reader.lines() {
        let input = line.unwrap().to_string();
        let morpheme_list = tokenizer.tokenize(&input, &mode, enable_debug);

        if wakati {
            let surface_list = morpheme_list.iter().map(|m| m.surface().to_string()).collect::<Vec<_>>();
            println!("{}", surface_list.join(" "));
        }
        else {
            for morpheme in morpheme_list {
                print!(
                    "{}\t{}\t{}",
                    morpheme.surface(),
                    morpheme.pos().join(","),
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
