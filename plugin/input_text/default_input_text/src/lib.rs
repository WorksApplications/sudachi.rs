use std::cmp;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use unicode_normalization::UnicodeNormalization;

use sudachi::declare_input_text_plugin;
use sudachi::dic::grammar::Grammar;
use sudachi::input_text::utf8_input_text_builder::Utf8InputTextBuilder;
use sudachi::plugin::input_text::InputTextPlugin;
use sudachi::prelude::*;

const DEFAULT_REWRITE_DEF_FILE_PATH: &str = "./src/resources/rewrite.def";

declare_input_text_plugin!(DefaultInputTextPlugin, DefaultInputTextPlugin::default);

#[derive(Default)]
pub struct DefaultInputTextPlugin {
    ignore_normalize_set: HashSet<char>,
    key_lengths: HashMap<char, usize>,
    replace_char_map: HashMap<String, String>,
}

impl DefaultInputTextPlugin {
    fn read_rewrite_lists(&mut self, path: &str) -> SudachiResult<()> {
        let mut ignore_normalize_set = HashSet::new();
        let mut key_lengths = HashMap::new();
        let mut replace_char_map = HashMap::new();

        let reader = BufReader::new(fs::File::open(&path)?);
        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() || line.chars().next().unwrap() == '#' {
                continue;
            }
            let cols: Vec<_> = line.split_whitespace().collect();

            // ignored normalize list
            if cols.len() == 1 {
                if cols[0].chars().count() != 1 {
                    return Err(SudachiError::InvalidDataFormat(
                        i,
                        format!("{} is not character", cols[0]),
                    ));
                }
                ignore_normalize_set.insert(cols[0].chars().next().unwrap());
                continue;
            }
            // replace char list
            if cols.len() == 2 {
                if replace_char_map.contains_key(cols[0]) {
                    return Err(SudachiError::InvalidDataFormat(
                        i,
                        format!("{} is alreadry defined", cols[0]),
                    ));
                }
                let first_char = cols[0].chars().next().unwrap();
                if key_lengths.get(&first_char).map(|v| *v).unwrap_or(0) < cols[0].len() {
                    key_lengths.insert(first_char, cols[0].chars().count());
                }
                replace_char_map.insert(cols[0].to_string(), cols[1].to_string());
                continue;
            }
            return Err(SudachiError::InvalidDataFormat(i, "".to_string()));
        }

        self.ignore_normalize_set = ignore_normalize_set;
        self.key_lengths = key_lengths;
        self.replace_char_map = replace_char_map;

        Ok(())
    }
}

impl InputTextPlugin for DefaultInputTextPlugin {
    fn set_up(&mut self, _grammar: &Grammar) -> SudachiResult<()> {
        // todo: load from config
        self.read_rewrite_lists(DEFAULT_REWRITE_DEF_FILE_PATH)?;

        Ok(())
    }

    fn rewrite(&self, builder: &mut Utf8InputTextBuilder) {
        let mut offset: i32 = 0;
        let mut next_offset: i32 = 0;
        let text = builder.modified.clone();
        let chars: Vec<_> = text.chars().collect();

        let mut i: i32 = -1;
        loop {
            i += 1;
            if i as usize >= chars.len() {
                break;
            }
            let mut textloop = false;
            offset += next_offset;
            next_offset = 0;
            let original = chars[i as usize];

            // 1. replace char without normalize
            let max_length = cmp::min(
                self.key_lengths.get(&original).map(|v| *v).unwrap_or(0),
                chars.len() - i as usize,
            );
            for j in (1..max_length + 1).rev() {
                if let Some(replace) = self
                    .replace_char_map
                    .get(&chars[i as usize..i as usize + j].iter().collect::<String>())
                {
                    let start = (i + offset) as usize;
                    builder.replace(start..start + j, replace);
                    next_offset += replace.chars().count() as i32 - j as i32;
                    i += (j - 1) as i32;
                    textloop = true;
                    break;
                }
            }
            if textloop {
                continue;
            }

            // 2. normalize
            // 2-1. capital alphabet (not only Latin but Greek, Cyrillic, etc.) -> small
            let original = original.to_string();
            let lower = original.to_lowercase();
            // todo: in rust, char::to_lowercase may returns multiple chars
            let lower_first_char = lower.chars().next().unwrap();
            let replace: String;
            if self.ignore_normalize_set.contains(&lower_first_char) {
                if original == lower {
                    continue;
                }
                replace = lower;
            } else {
                // 2-2. normalize (except in ignore_normalize)
                // e.g. full-width alphabet -> half-width / ligature / etc.
                replace = lower.nfkc().collect::<String>();
            }
            next_offset = replace.chars().count() as i32 - 1;
            if original != replace {
                let start = (i + offset) as usize;
                builder.replace(start..start + 1, &replace);
            }
        }
    }
}
