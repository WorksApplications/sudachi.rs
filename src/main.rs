use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process;

use structopt::StructOpt;

use sudachi::config::Config;
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

    // Dictionary is optional if baked in
    /// Path to sudachi dictionary
    #[cfg(feature = "bake_dictionary")]
    #[structopt(short = "l", long = "dict")]
    dictionary_path: Option<PathBuf>,

    // Dictionary is not baked in, so it must be specified
    /// Path to sudachi dictionary
    #[cfg(not(feature = "bake_dictionary"))]
    #[structopt(short = "l", long = "dict")]
    dictionary_path: Option<PathBuf>,
}

fn get_dictionary_bytes(system_dict: Option<PathBuf>) -> Option<Cow<'static, [u8]>> {
    let dictionary_path = {
        cfg_if::cfg_if! {
            if #[cfg(feature="bake_dictionary")] {
                if let Some(dictionary_path) = system_dict {
                    dictionary_path
                } else {
                    return Some(Cow::Borrowed(BAKED_DICTIONARY_BYTES));
                }
            } else {
                if let Some(dictionary_path) = system_dict {
                    dictionary_path
                }else {
                    return None;
                }
            }
        }
    };

    let storage_buf = dictionary_bytes_from_path(&dictionary_path)
        .expect("Failed to get dictionary bytes from file");
    Some(Cow::Owned(storage_buf))
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

    // load config file
    let config = Config::new(
        args.config_file.clone(),
        args.resource_dir.clone(),
        args.dictionary_path.clone(),
    )
    .expect("Failed to load config file");

    // load and parse dictionary binary to create a tokenizer
    let dictionary_bytes =
        get_dictionary_bytes(config.system_dict.clone()).expect("No system dictionary found");
    let mut user_dictionary_bytes = Vec::with_capacity(config.user_dicts.len());
    for pb in &config.user_dicts {
        let storage_buf =
            dictionary_bytes_from_path(pb).expect("Failed to get user dictionary bytes from file");
        user_dictionary_bytes.push(storage_buf.into_boxed_slice());
    }
    let tokenizer =
        Tokenizer::from_dictionary_bytes(&dictionary_bytes, &user_dictionary_bytes, &config)
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
                        "\t{}\t{}\t{}\t{:?}",
                        morpheme.dictionary_form(),
                        morpheme.reading_form(),
                        morpheme.dictionary_id,
                        morpheme.word_info.synonym_group_ids,
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
