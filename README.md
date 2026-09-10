# sudachi.rs - English README

[![Rust](https://github.com/WorksApplications/sudachi.rs/actions/workflows/rust.yml/badge.svg)](https://github.com/WorksApplications/sudachi.rs/actions/workflows/rust.yml)

<p align="center"><img width="100" src="logo.png" alt="sudachi.rs logo"></p>

sudachi.rs is a Rust implementation of [Sudachi](https://github.com/WorksApplications/Sudachi), a Japanese morphological analyzer.

[日本語 README](README.ja.md).

Python implementation is also available: [SudachiPy Documentation](./python/README.md).

## TL;DR

Install Python version

```bash
pip install --upgrade 'sudachipy>=0.6.11'
```

or Rust version

```bash
$ git clone https://github.com/WorksApplications/sudachi.rs.git
$ cd ./sudachi.rs

$ cargo build --release
$ cargo install --path sudachi-cli/
$ ./fetch_dictionary.sh

$ echo "高輪ゲートウェイ駅" | sudachi
高輪ゲートウェイ駅  名詞,固有名詞,一般,*,*,*    高輪ゲートウェイ駅
EOS
```

### Example

Multi-granular Tokenization

```bash
$ echo 選挙管理委員会 | sudachi
選挙管理委員会  名詞,固有名詞,一般,*,*,*        選挙管理委員会
EOS

$ echo 選挙管理委員会 | sudachi --mode A
選挙    名詞,普通名詞,サ変可能,*,*,*    選挙
管理    名詞,普通名詞,サ変可能,*,*,*    管理
委員    名詞,普通名詞,一般,*,*,*        委員
会      名詞,普通名詞,一般,*,*,*        会
EOS
```

Normalized Form

```bash
$ echo 打込む かつ丼 附属 vintage | sudachi
打込む  動詞,一般,*,*,五段-マ行,終止形-一般     打ち込む
        空白,*,*,*,*,*
かつ丼  名詞,普通名詞,一般,*,*,*        カツ丼
        空白,*,*,*,*,*
附属    名詞,普通名詞,サ変可能,*,*,*    付属
        空白,*,*,*,*,*
vintage 名詞,普通名詞,一般,*,*,*        ビンテージ
EOS
```

Wakati (space-delimited surface form) Output

```bash
$ cat lemon.txt
えたいの知れない不吉な塊が私の心を始終圧えつけていた。
焦躁と言おうか、嫌悪と言おうか――酒を飲んだあとに宿酔があるように、酒を毎日飲んでいると宿酔に相当した時期がやって来る。
それが来たのだ。これはちょっといけなかった。

$ sudachi --wakati lemon.txt
えたい の 知れ ない 不吉 な 塊 が 私 の 心 を 始終 圧え つけ て い た 。
焦躁 と 言おう か 、 嫌悪 と 言おう か ― ― 酒 を 飲ん だ あと に 宿酔 が ある よう に 、 酒 を 毎日 飲ん で いる と 宿酔 に 相当 し た 時期 が やっ て 来る 。
それ が 来 た の だ 。 これ は ちょっと いけ なかっ た 。
```

## Setup

You need sudachi.rs, default plugins, and a dictionary. (This crate don't include dictionary.)

### 1. Get the source code

```sh
git clone https://github.com/WorksApplications/sudachi.rs.git
```

### 2. Download a Sudachi Dictionary

Sudachi requires a dictionary to operate.
You can download a dictionary ZIP file from [WorksApplications/SudachiDict](https://github.com/WorksApplications/SudachiDict) (choose one from `small`, `core`, or `full`), unzip it, and place the `system_*.dic` file somewhere.
By the default setting file, sudachi.rs assumes that it is placed at `resources/system.dic`.

#### Convenience Script

Optionally, you can use the [`fetch_dictionary.sh`](fetch_dictionary.sh) shell script to download a dictionary and install it to `resources/system.dic` (overrides).

```sh
# fetch latest core dictionary
./fetch_dictionary.sh

# fetch dictionary of specified version and type
./fetch_dictionary.sh 20241021 small
```

### 3. Build

```sh
cargo build --release
```

#### Build (bake dictionary into binary)

**This was un-implemented and does not work currently**, see https://github.com/WorksApplications/sudachi.rs/issues/35

Specify the `bake_dictionary` feature to embed a dictionary into the binary.
The `sudachi` executable will **contain the dictionary binary**.
The baked dictionary will be used if no one is specified via cli option or setting file.

You must specify the path the dictionary file in the `SUDACHI_DICT_PATH` environment variable when building.
`SUDACHI_DICT_PATH` is relative to the sudachi.rs directory (or absolute).

Example on Unix-like system:

```sh
# Download dictionary to resources/system.dic
$ ./fetch_dictionary.sh

# Build with bake_dictionary feature (relative path)
$ env SUDACHI_DICT_PATH=resources/system.dic cargo build --release --features bake_dictionary

# or

# Build with bake_dictionary feature (absolute path)
$ env SUDACHI_DICT_PATH=/path/to/my-sudachi.dic cargo build --release --features bake_dictionary
```

### 4. Install

```sh
$ cd sudachi.rs/
$ cargo install --path sudachi-cli/

$ which sudachi
/Users/<USER>/.cargo/bin/sudachi

$ sudachi -h
sudachi 0.6.0
A Japanese tokenizer
...
```

## Usage as a command

```bash
$ sudachi -h
A Japanese tokenizer

Usage: sudachi [OPTIONS] [FILE] [COMMAND]

Commands:
  build
          Builds system dictionary
  ubuild
          Builds user dictionary
  dump

  help
          Print this message or the help of the given subcommand(s)

Arguments:
  [FILE]
          Input text file: If not present, read from STDIN

Options:
  -r, --config-file <CONFIG_FILE>
          Path to the setting file in JSON format
  -p, --resource_dir <RESOURCE_DIR>
          Path to the root directory of resources
  -m, --mode <MODE>
          Split unit: "A" (short), "B" (middle), or "C" (Named Entity) [default: C]
  -o, --output <OUTPUT_FILE>
          Output text file: If not present, use stdout
  -a, --all
          Prints all fields
  -w, --wakati
          Outputs only surface form
  -d, --debug
          Debug mode: Print the debug information
  -l, --dict <DICTIONARY_PATH>
          Path to sudachi dictionary. If None, it refer config and then baked dictionary
      --split-sentences <SPLIT_SENTENCES>
          How to split sentences [default: yes]
  -h, --help
          Print help (see more with '--help')
  -V, --version
          Print version
```

### Output

Columns are tab separated.

- Surface
- Part-of-Speech Tags (comma separated)
- Normalized Form

When you add the `-a` (`--all`) flag, it additionally outputs

- Dictionary Form
- Reading Form
- Dictionary ID
  - `0` for the system dictionary
  - `1` and above for the user dictionaries
  - `-1` if a word is Out-of-Vocabulary (not in the dictionary)
- Synonym group IDs
- `(OOV)` if a word is Out-of-Vocabulary (not in the dictionary)

```bash
$ echo "外国人参政権" | sudachi -a
外国人参政権    名詞,普通名詞,一般,*,*,*        外国人参政権    外国人参政権    ガイコクジンサンセイケン      0       []
EOS
```

```bash
echo "阿quei" | sudachipy -a
阿      名詞,普通名詞,一般,*,*,*        阿      阿              -1      []      (OOV)
quei    名詞,普通名詞,一般,*,*,*        quei    quei            -1      []      (OOV)
EOS
```

When you add `-w` (`--wakati`) flag, it outputs space-delimited surface instead.

```bash
$ echo "外国人参政権" | sudachi -m A -w
外国 人 参政 権
```

## API

See [API reference page](https://worksapplications.github.io/sudachi.rs/rust/sudachi/).

## ToDo

- [x] Out of Vocabulary handling
- [ ] Easy dictionary file install & management, [similar to SudachiPy](https://github.com/WorksApplications/SudachiPy/issues/73)
- [ ] Registration to crates.io

## References

### Sudachi

- [WorksApplications/Sudachi](https://github.com/WorksApplications/Sudachi)
- [WorksApplications/SudachiDict](https://github.com/WorksApplications/SudachiDict)
- [WorksApplications/SudachiPy](https://github.com/WorksApplications/SudachiPy)
- [msnoigrs/gosudachi](https://github.com/msnoigrs/gosudachi)

### Morphological Analyzers in Rust

- [agatan/yoin: A Japanese Morphological Analyzer written in pure Rust](https://github.com/agatan/yoin)
- [wareya/notmecab-rs: notmecab-rs is a very basic mecab clone, designed only to do parsing, not training.](https://github.com/wareya/notmecab-rs)

### Logo

- [Sudachi Logo](https://github.com/WorksApplications/Sudachi/blob/develop/docs/Sudachi.png)
- Crab illustration: [Pixabay](https://pixabay.com/ja/vectors/%E5%8B%95%E7%89%A9-%E3%82%AB%E3%83%8B-%E7%94%B2%E6%AE%BB%E9%A1%9E-%E6%B5%B7-2029728/)
