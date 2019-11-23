# Sudachi.rs

An unofficial [Sudachi](https://github.com/WorksApplications/Sudachi) clone in Rust 🦀


## Example

Multi-granular Tokenization

```
$ echo 選挙管理委員会 | sudachi
選挙管理委員会	名詞,固有名詞,一般,*,*,*	選挙管理委員会
EOS

$ echo 選挙管理委員会 | sudachi --mode A
選挙	名詞,普通名詞,サ変可能,*,*,*	選挙
管理	名詞,普通名詞,サ変可能,*,*,*	管理
委員	名詞,普通名詞,一般,*,*,*	委員
会	名詞,普通名詞,一般,*,*,*	会
```

Normalized Form

```
echo 打込む かつ丼 附属 vintage | sudachi
打込む	動詞,一般,*,*,五段-マ行,終止形-一般	打ち込む
 	空白,*,*,*,*,*
かつ丼	名詞,普通名詞,一般,*,*,*	カツ丼
 	空白,*,*,*,*,*
附属	名詞,普通名詞,サ変可能,*,*,*	付属
 	空白,*,*,*,*,*
vintage	名詞,普通名詞,一般,*,*,*	ビンテージ
```

Wakati (space-delimited surface form) Output

```
$ cat lemon.txt
えたいの知れない不吉な塊が私の心を始終圧えつけていた。
焦躁と言おうか、嫌悪と言おうか――酒を飲んだあとに宿酔があるように、酒を毎日飲んでいると宿酔に相当した時期がやって来る。
それが来たのだ。これはちょっといけなかった。

$ sudachi --wakati lemon.txt
えたい の 知れ ない 不吉 な 塊 が 私 の 心 を 始終 圧え つけ て い た 。
焦躁 と 言おう か 、 嫌悪 と 言おう か ― ― 酒 を 飲ん だ あと に 宿酔 が ある よう に 、 酒 を 毎日 飲ん で いる と 宿酔 に 相当 し た 時期 が やっ て 来る 。
それ が 来 た の だ 。 これ は ちょっと いけ なかっ た 。
```

## Usage

```
$ sudachi -h
sudachi 0.1.0
A Japanese tokenizer

USAGE:
    sudachi [FLAGS] [OPTIONS] [file]

FLAGS:
    -d, --debug      Debug mode: dump lattice
    -h, --help       Prints help information
    -a, --all        Print all fields
    -V, --version    Prints version information
    -w, --wakati     Output only surface form

OPTIONS:
    -m, --mode <mode>    Split unit: "A" (short), "B" (middle), or "C" (Named Entity) [default: C]

ARGS:
```

## Setup

### 1. Get the source code

```
$ git clone https://github.com/sorami/sudachi.rs.git
```

### 2. Download a Sudachi Dictionary

You can download a dictionary zip file from [WorksApplications/SudachiDict](https://github.com/WorksApplications/SudachiDict) (choose one from `small`, `core`, or `full`), unzip it, and place the `system-*.dic` file to `src/resources/system.dic` .

Alternatively, you can use a quick shell script in the source code; This script will download the `core` dictionary and place it to `src/resources/system.dic`.

```
$ ./fetch_dictionary.sh
```

### 3. Build, Install

The built executable will **contain the dictionary binary**.

```
$ cargo build
```

or

```
sudachi.rs/ $ cargo install --path .

$ which sudachi
/Users/<USER>/.cargo/bin/sudachi

$ sudachi -h
sudachi 0.1.0
A Japanese tokenizer
...
```


## ToDo

- [ ] Out of Vocabulary handling
- [ ] Easy dictionary file install & management, [similar to SudachiPy](https://github.com/WorksApplications/SudachiPy/issues/73)
- [ ] Registration to crates.io


## References

### Sudachi

- [WorksApplications/Sudachi](https://github.com/WorksApplications/Sudachi)
- [WorksApplications/SudachiDict](https://github.com/WorksApplications/SudachiDict)
- [WorksApplications/SudachiPy](https://github.com/WorksApplications/SudachiPy)

### Morphological Analyzers in Rust

- [agatan/yoin: A Japanese Morphological Analyzer written in pure Rust](https://github.com/agatan/yoin)
- [wareya/notmecab-rs: notmecab-rs is a very basic mecab clone, designed only to do parsing, not training.](https://github.com/wareya/notmecab-rs)
