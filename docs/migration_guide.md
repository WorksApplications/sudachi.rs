
# from v0.6 to v0.7

## バイナリ辞書

Sudachi 辞書のバイナリ形式が変更され、v0.6 までのバイナリ辞書は使用できなくなりました。
SudachiDict-\* (small/core/full) から取得できるシステム辞書は、バージョン v\* ([TBA]) 以降のみ使用可能です。
それ以前の辞書を使用するには、個別にソースファイルからのビルドが必要です。

ユーザー辞書についても再ビルドが必要です。
詳細はJava版の[ユーザー辞書移行ガイド](https://github.com/WorksApplications/Sudachi/blob/develop/docs/migrate_user_dictionary.md)を参照してください。
辞書バイナリは Java/Rust/Python で共通です。辞書のビルドには本リポジトリやCLIも使用可能です。

また、ユーザ辞書を使用する際は、システム辞書がそのユーザ辞書のビルドに使用したものと異なる場合はエラーとなるようになりました。
システム辞書を変更・ビルドした際は、合わせてユーザ辞書の再ビルドが必要になります。

## 解析アルゴリズム

Java版と挙動を一致させることを目的として、解析アルゴリズムが一部修正されています。
同一の辞書を使用しても、v0.6 とは結果が異なることがあります。

### char.def / can_bow の算出

`NOOOVBOW2` カテゴリは廃止され、`NOOOVEOW` が導入されました。
`NOOOVBOW` と `NOOOVEOW` の複合が `NOOOVBOW2` に相当します。

これらはある文字が語頭になりうるかの事前計算では使われなくなりました。
カテゴリ名の通り、OOVの語頭判定でのみ使用されるようになりました。

参照: https://github.com/WorksApplications/sudachi.rs/pull/325

### カタカナOOVの構成

`join_katakana_oov` プラグインにおいてノードの結合を行う際、結合後のノードがラティスに既に存在している場合はそれを利用します。
v0.6 ではこの候補が複数ある際に連接コスト込みで選択を行っていましたが、v0.7 ではそのノードのコストのみを参照して選択するように修正されました。

参照: https://github.com/WorksApplications/sudachi.rs/pull/323

### Character category continuous length

文字種の連続長の計算アルゴリズムをJava版と同じものに修正しました。

参照: https://github.com/WorksApplications/sudachi.rs/pull/326

## CLI

### 出力のTSV化

`-a` 指定時の出力について、その語がOOVでない場合に末尾に TAB (`\t`) が出力されるようになりました。
これにより出力が EOS 表示行を除いて TSV 形式となります。

## Config

### (Rust) リソースファイル解決の指定

`ConfigBuilder` の内部構造が実設定値とファイル解決方法に整理されました。
リソースファイル解決のためのファイルパスは `PathResolver` にまとめて保持されます。
これに伴い `ConfigBuilder.resource_path` や `ConfigBuilder.root_directory` は廃止されました。
`with_resolver`, `append_resolver` や `push_resolver_root` などを用いて解決先の指定や優先順序の調整を行ってください。

### (Rust) デフォルトリソースファイルの読み込み

`char.def` などのデフォルトリソースファイルはバイナリに埋め込まれるようになりました。
コンフィグでは `PathResolver` に `ResolverRoot::Embedded` を与えることでこれらを指定することができます。
`PathResolver.from_embedded` や `ConfigBuilder.push_embedded` を用いて設定してください。
`ConfigBuilder` の初期化時にはこれは設定されていないため、デフォルトリソースファイルを使用するには手動での設定が必要であることに注意してください。

### リソースファイル解決順序の変更

コンフィグの作成 `Config::new(config_file, resource_dir, dictionary_path)` (Rust) と, 辞書の読み込み `Dictionary(config, resource_dir, dict)` (Python) におけるリソースファイル解決の順序が変更されています。
v0.6 での解決順は、コンフィグ内 `path` フィールド > デフォルトもしくは指定のリソースディレクトリ > コンフィグファイルの親ディレクトリ > デフォルトリソースファイル、の順でしたが、
v0.7 での解決順は、指定のリソースディレクトリ > コンフィグ内 `path` フィールド > コンフィグファイルの親ディレクトリ > デフォルトリソースファイル の順になりました。

## 廃止

### (Python) Tokenizer の生成

トークナイザーを生成する `Dictionary.create()` は非推奨になりました。
代わりに `Dictionary.tokenizer()` を使用してください。

### (Python) WordInfo

`WordInfo` クラスおよびそれを取得するメソッド `Morpheme.get_word_info` は廃止されました。
