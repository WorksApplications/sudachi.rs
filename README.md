# sudachi.rs

<p align="center"><img width="100" src="logo.png" alt="sudachi.rs logo"></p>

An unofficial [Sudachi](https://github.com/WorksApplications/Sudachi) clone in Rust 🦀

[日本語 README](#sudachirs---日本語readme)


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
EOS
```

Normalized Form

```
$ echo 打込む かつ丼 附属 vintage | sudachi
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
    -d, --debug      Debug mode: Dumps lattice
    -h, --help       Prints help information
    -a, --all        Prints all fields
    -V, --version    Prints version information
    -w, --wakati     Outputs only surface form

OPTIONS:
    -m, --mode <mode>    Split unit: "A" (short), "B" (middle), or "C" (Named Entity) [default: C]

ARGS:
    <file>    Input text file: If not present, read from STDIN
```

## Setup

### 1. Get the source code

```
$ git clone https://github.com/sorami/sudachi.rs.git
```

### 2. Download a Sudachi Dictionary

You can download a dictionary zip file from [WorksApplications/SudachiDict](https://github.com/WorksApplications/SudachiDict) (choose one from `small`, `core`, or `full`), unzip it, and place the `system_*.dic` file to `src/resources/system.dic` (Note that the file name is changed to `system.dic`) .

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
- [msnoigrs/gosudachi](https://github.com/msnoigrs/gosudachi)


### Morphological Analyzers in Rust

- [agatan/yoin: A Japanese Morphological Analyzer written in pure Rust](https://github.com/agatan/yoin)
- [wareya/notmecab-rs: notmecab-rs is a very basic mecab clone, designed only to do parsing, not training.](https://github.com/wareya/notmecab-rs)

### Logo

- [Sudachi Logo](https://github.com/WorksApplications/Sudachi/blob/develop/docs/Sudachi.png)
- Crab illustration: [Pixabay](https://pixabay.com/ja/vectors/%E5%8B%95%E7%89%A9-%E3%82%AB%E3%83%8B-%E7%94%B2%E6%AE%BB%E9%A1%9E-%E6%B5%B7-2029728/)


***


# sudachi.rs - 日本語README

<p align="center"><img width="100" src="logo.png" alt="sudachi.rs logo"></p>

形態素解析器 [Sudachi](https://github.com/WorksApplications/Sudachi)  - 非公式 Rust 🦀 クローン

[English README](#sudachirs)


## 利用例

複数粒度での分割

```
$ echo 選挙管理委員会 | sudachi
選挙管理委員会	名詞,固有名詞,一般,*,*,*	選挙管理委員会
EOS

$ echo 選挙管理委員会 | sudachi --mode A
選挙	名詞,普通名詞,サ変可能,*,*,*	選挙
管理	名詞,普通名詞,サ変可能,*,*,*	管理
委員	名詞,普通名詞,一般,*,*,*	委員
会	名詞,普通名詞,一般,*,*,*	会
EOS
```

正規化表記

```
$ echo 打込む かつ丼 附属 vintage | sudachi
打込む	動詞,一般,*,*,五段-マ行,終止形-一般	打ち込む
 	空白,*,*,*,*,*
かつ丼	名詞,普通名詞,一般,*,*,*	カツ丼
 	空白,*,*,*,*,*
附属	名詞,普通名詞,サ変可能,*,*,*	付属
 	空白,*,*,*,*,*
vintage	名詞,普通名詞,一般,*,*,*	ビンテージ
```

分かち書き出力

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

## 利用方法

```
$ sudachi -h
sudachi 0.1.0
A Japanese tokenizer

USAGE:
    sudachi [FLAGS] [OPTIONS] [file]

FLAGS:
    -d, --debug      Debug mode: Dumps lattice
    -h, --help       Prints help information
    -a, --all        Prints all fields
    -V, --version    Prints version information
    -w, --wakati     Outputs only surface form

OPTIONS:
    -m, --mode <mode>    Split unit: "A" (short), "B" (middle), or "C" (Named Entity) [default: C]

ARGS:
    <file>    Input text file: If not present, read from STDIN
```

## セットアップ

### 1. ソースコードの取得

```
$ git clone https://github.com/sorami/sudachi.rs.git
```

### 2. Sudachi辞書のダウンロード 

[WorksApplications/SudachiDict](https://github.com/WorksApplications/SudachiDict)から辞書のzipファイル（ `small` 、 `core` 、 `full` から一つ選択）し、解凍して、中にある `system_*.dic` ファイルを `src/resources/system.dic` として置いてください （ファイル名が `system.dic` に変わっていることに注意）。

上記のように手動で設置する以外に、レポジトリにあるスクリプトを使って自動的に `core` 辞書をダウンロードし `src/resources/system.dic` として設置することもできます。

```
$ ./fetch_dictionary.sh
```

### 3. ビルド、インストール

ビルドされた実行ファイルは、**辞書バイナリを内包しています**。

```
$ cargo build
```

もしくは

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

- [ ] 未知語処理
- [ ] 簡単な辞書ファイルのインストール、管理（[SudachiPyでの方式を参考に](https://github.com/WorksApplications/SudachiPy/issues/73)）
- [ ] crates.io への登録


## リファレンス

### Sudachi

- [WorksApplications/Sudachi](https://github.com/WorksApplications/Sudachi)
- [WorksApplications/SudachiDict](https://github.com/WorksApplications/SudachiDict)
- [WorksApplications/SudachiPy](https://github.com/WorksApplications/SudachiPy)
- [msnoigrs/gosudachi](https://github.com/msnoigrs/gosudachi)

### Rustによる形態素解析器の実装

- [agatan/yoin: A Japanese Morphological Analyzer written in pure Rust](https://github.com/agatan/yoin)
- [wareya/notmecab-rs: notmecab-rs is a very basic mecab clone, designed only to do parsing, not training.](https://github.com/wareya/notmecab-rs)

### ロゴ

- [Sudachiのロゴ](https://github.com/WorksApplications/Sudachi/blob/develop/docs/Sudachi.png)
- カニのイラスト: [Pixabay](https://pixabay.com/ja/vectors/%E5%8B%95%E7%89%A9-%E3%82%AB%E3%83%8B-%E7%94%B2%E6%AE%BB%E9%A1%9E-%E6%B5%B7-2029728/)
