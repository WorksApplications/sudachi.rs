# sudachi.rs - 日本語README

<p align="center"><img width="100" src="logo.png" alt="sudachi.rs logo"></p>

sudachi.rs は日本語形態素解析器 [Sudachi](https://github.com/WorksApplications/Sudachi) のRust実装です。

[English README](README.md) [SudachiPy Documentation](https://worksapplications.github.io/sudachi.rs/python)

## TL;DR

SudachiPyとして使うには
```bash
$ pip install --upgrade 'sudachipy>=0.6.2'
```

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

### 利用例

複数粒度での分割

```
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

正規化表記

```
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


## セットアップ

sudachi.rs本体に加え、デフォルトで使用するプラグイン、また辞書が必要になります。※パッケージには辞書が含まれていません。

### 1. ソースコードの取得

```
$ git clone https://github.com/WorksApplications/sudachi.rs.git
```

### 2. Sudachi辞書のダウンロード

[WorksApplications/SudachiDict](https://github.com/WorksApplications/SudachiDict)から辞書のzipファイル（ `small` 、 `core` 、 `full` から一つ選択）し、解凍して、必要であれば中にある `system_*.dic` ファイルをわかりやすい位置に置いてください。
デフォルトの設定ファイルでは、辞書ファイルが `resources/system.dic` に存在していると指定しています（ファイル名が `system.dic` に変わっていることに注意）。

#### ダウンロードスクリプト

上記のように手動で設置する以外に、レポジトリにあるスクリプトを使って自動的に辞書をダウンロードし `resources/system.dic` として設置することもできます。

```
$ ./fetch_dictionary.sh
```

### 3. ビルド

#### ビルド（デフォルト）

`--all` フラグを使って付属のプラグインもまとめてビルドすることができます。

```
$ cargo build --release
```

#### ビルド（辞書バイナリの埋め込み）

**以下は現在未対応となっています**　https://github.com/WorksApplications/sudachi.rs/issues/35 をご参考ください。

`bake_dictionary` フィーチャーフラグを立ててビルドすることで、辞書ファイルをバイナリに埋め込むことができます。
これによってビルドされた実行ファイルは、**辞書バイナリを内包しています**。
オプションや設定ファイルで辞書が指定されなかった場合、この内包辞書が使用されます。

ビルド時、埋め込む辞書へのパスを `SUDACHI_DICT_PATH` 環境変数によって指定する必要があります。
このパスは絶対パスもしくは sudachi.rs ディレクトリからの相対パスで指定してください。

Unix-likeシステムでの例:
```sh
# resources/system.dic への辞書ダウンロード
$ ./fetch_dictionary.sh

# bake_dictionary フィーチャーフラグ付きでビルド (辞書を相対パスで指定)
$ env SUDACHI_DICT_PATH=resources/system.dic cargo build --release --features bake_dictionary

# もしくは

# bake_dictionary フィーチャーフラグ付きでビルド (辞書を絶対パスで指定)
$ env SUDACHI_DICT_PATH=/path/to/my-sudachi.dic cargo build --release --features bake_dictionary
```


### 4. インストール
```
sudachi.rs/ $ cargo install --path sudachi-cli/

$ which sudachi
/Users/<USER>/.cargo/bin/sudachi

$ sudachi -h
sudachi 0.6.0
A Japanese tokenizer
...
```


## 利用方法

```bash
$ sudachi -h
sudachi 0.6.0
A Japanese tokenizer

USAGE:
    sudachi [FLAGS] [OPTIONS] [file]

FLAGS:
    -d, --debug      Debug mode: Print the debug information
    -h, --help       Prints help information
    -a, --all        Prints all fields
    -V, --version    Prints version information
    -w, --wakati     Outputs only surface form

OPTIONS:
    -r, --config-file <config-file>      Path to the setting file in JSON format
    -l, --dict <dictionary-path>         Path to sudachi dictionary. If None, it refer config and then baked dictionary
    -m, --mode <mode>                    Split unit: "A" (short), "B" (middle), or "C" (Named Entity) [default: C]
    -o, --output <output-file>
    -p, --resource_dir <resource-dir>    Path to the root directory of resources

ARGS:
    <file>    Input text file: If not present, read from STDIN
```

### 出力

タブ区切りで出力されます。 デフォルトは以下の情報が含まれます。

- 表層形
- 品詞（コンマ区切り）
- 正規化表記

オプションで -a (--all) を指定すると以下の情報が追加されます。

- 辞書形
- 読み
- 辞書ID
    - 0 システム辞書
    - 1 ユーザー辞書
    - -1 未知語（辞書に含まれない単語）
- 同義語グループID
- "OOV" 未知語（辞書に含まれない単語）の場合のみ

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

オプションで -w (--wakati) を指定すると代わりに表層形のみをスペース区切りで出力します。

```bash
$ echo "外国人参政権" | sudachi -m A -w
外国 人 参政 権
```


## ToDo

- [x] 未知語処理
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
