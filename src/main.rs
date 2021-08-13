use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process;

use structopt::StructOpt;

use sudachi::prelude::*;

#[cfg(feature = "bake_dictionary")]
const BAKED_DICTIONARY_BYTES: &[u8] = include_bytes!(env!("SUDACHI_DICT_PATH"));

/// A Japanese tokenizer
#[derive(StructOpt)]
#[structopt(name = "sudachi")]
struct Cli {
    /// Input text file: If not present, read from STDIN
    #[structopt(parse(from_os_str))]
    file: Option<PathBuf>,

    // Output text file: If not present, use stdout
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output_file: Option<PathBuf>,

    /// Split unit: "A" (short), "B" (middle), or "C" (Named Entity)
    #[structopt(short = "m", long = "mode", default_value = "C")]
    mode: String,

    /// Prints all fields
    #[structopt(short = "a", long = "all")]
    print_all: bool,

    /// Outputs only surface form
    #[structopt(short = "w", long = "wakati")]
    wakati: bool,

    /// Debug mode: Print the debug information
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

    let storage_buf = dictionary_bytes_from_path(&dictionary_path)
        .expect("Failed to get dictionary bytes from file");
    Cow::Owned(storage_buf)
}

fn main() {
    let args = Cli::from_args();

    let mode = match args.mode.as_str().parse() {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!("Invalid mode: {}", err);
            process::exit(1);
        }
    };
    let print_all = args.print_all;
    let wakati = args.wakati;
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
    let mut writer: Box<dyn Write> = match &args.output_file {
        Some(output_path) => Box::new(BufWriter::new(
            File::create(&output_path)
                .unwrap_or_else(|_| panic!("Failed to open output file {:?}", &output_path)),
        )),
        None => Box::new(BufWriter::new(io::stdout())),
    };

    // load and parse dictionary binary to create a tokenizer
    let dictionary_bytes = get_dictionary_bytes(&args);
    let tokenizer = Tokenizer::from_dictionary_bytes(&dictionary_bytes)
        .expect("Failed to create Tokenizer from dictionary bytes");

    // tokenize and output results
    for line in reader.lines() {
        let input = line.expect("Failed to reead line");
        let morpheme_list = tokenizer
            .tokenize(&input, mode, enable_debug)
            .expect("Failed to tokenize input");
        write_results(&mut writer, morpheme_list, print_all, wakati)
            .expect("Failed to write output");
    }
}

fn write_results(
    writer: &mut Box<dyn Write>,
    morpheme_list: Vec<Morpheme>,
    print_all: bool,
    wakati: bool,
) -> io::Result<()> {
    if wakati {
        let surface_list = morpheme_list
            .iter()
            .map(|m| m.surface().to_string())
            .collect::<Vec<_>>();
        writer.write(surface_list.join(" ").as_bytes())?;
    } else {
        for morpheme in morpheme_list {
            writer.write(
                format!(
                    "{}\t{}\t{}",
                    morpheme.surface(),
                    morpheme.pos().expect("Missing part of speech").join(","),
                    morpheme.normalized_form()
                )
                .as_bytes(),
            )?;
            if print_all {
                writer.write(
                    format!(
                        "\t{}\t{}",
                        morpheme.dictionary_form(),
                        morpheme.reading_form(),
                    )
                    .as_bytes(),
                )?;
                if morpheme.is_oov {
                    writer.write("\t(OOV)".as_bytes())?;
                }
            }
            writer.write("\n".as_bytes())?;
        }
        writer.write("EOS\n".as_bytes())?;
    }

    Ok(())
}
