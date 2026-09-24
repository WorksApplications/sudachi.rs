
# from v0.6 to v0.7

## バイナリ辞書

sudachi.rs / SudachiPy v0.7 にて辞書のバイナリ形式が V1 形式に変更され、v0.6 までのバイナリ辞書（V0 形式）は使用できなくなりました。
移行には以下の対応が必要です。

- V1 形式のシステム辞書を取得する
  - [V1 形式バイナリ辞書配布ページ](http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict/v1) にて配布しています。
- ユーザー辞書を V1 形式に再ビルドする
  - このとき、実行時に使用するものと同じシステム辞書を使用する必要があります。
- 辞書配布ページの URL を使用している場合は更新する
  - V1 形式バイナリ辞書配布ページ：http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict/v1
  - V1 形式辞書ソース配布ページ：http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict-raw/v1

SudachiDict-\* (small/core/full) から取得できるシステム辞書は、バージョン v20260723.1 以降のみ使用可能です。
それ以前の辞書を使用するには、上記配布ページから取得するか、個別にソースファイルからのビルドが必要です。

ユーザー辞書の再ビルドについてはJava版の[ユーザー辞書移行ガイド](https://github.com/WorksApplications/Sudachi/blob/develop/docs/migrate_user_dictionary.md)を参照してください。
辞書バイナリは Java/Rust/Python で共通です。辞書のビルドには sudachi.rs/ SudachiPy も使用可能です。

ユーザー辞書には、ビルド時に使用したシステム辞書の識別情報が記録されるようになりました。実行時にこれと異なるシステム辞書が指定された場合はエラーとなります。
システム辞書を更新する際には、使用するすべてのユーザー辞書についてもそのシステム辞書で再ビルドする必要があります。

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

### デフォルトリソースディレクトリの削除

`char.def` などのデフォルトリソースファイルはバイナリに埋め込まれるようになりました。
これに伴い v0.6 で CLI が使用していたデフォルトリソースディレクトリ（`resources/`）は使用されなくなりました。
このためデフォルトで参照するシステム辞書ファイルも `resources/system.dic` から `$PWD/system.dic` に変更されています。

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

### (Python) サブセットでの同義語IDの指定

解析時に取得するフィールドを指定する [subsetting](../python/docs/source/topics/subsetting.rst) について、同義語グループIDを指定する値 `synonym_group_id` は廃止されました。
代わりに `synonym_group_ids` を使用してください。
