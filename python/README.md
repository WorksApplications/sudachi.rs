# SudachiPy
[![PyPi version](https://img.shields.io/pypi/v/sudachipy.svg)](https://pypi.python.org/pypi/sudachipy/)
[![](https://img.shields.io/badge/python-3.6+-blue.svg)](https://www.python.org/downloads/release/python-360/)
[Documentation](https://worksapplications.github.io/sudachi.rs/python)

SudachiPy is a Python version of [Sudachi](https://github.com/WorksApplications/Sudachi), a Japanese morphological analyzer.

This is not a pure Python implementation, but bindings for the
[Sudachi.rs](https://github.com/WorksApplications/sudachi.rs).

## Binary wheels

We provide binary builds for macOS (10.14+), Windows and Linux only for x86_64 architecture.
x86 32-bit architecture is not supported and is not tested.
MacOS source builds seem to work on ARM-based (Aarch64) Macs,
but this architecture also is not tested and require installing Rust toolchain and Cargo.

More information [here](https://worksapplications.github.io/sudachi.rs/python/wheels.html).

## TL;DR

```bash
$ pip install sudachipy sudachidict_core

$ echo "高輪ゲートウェイ駅" | sudachipy
高輪ゲートウェイ駅	名詞,固有名詞,一般,*,*,*	高輪ゲートウェイ駅
EOS

$ echo "高輪ゲートウェイ駅" | sudachipy -m A
高輪	名詞,固有名詞,地名,一般,*,*	高輪
ゲートウェイ	名詞,普通名詞,一般,*,*,*	ゲートウェー
駅	名詞,普通名詞,一般,*,*,*	駅
EOS

$ echo "空缶空罐空きカン" | sudachipy -a
空缶	名詞,普通名詞,一般,*,*,*	空き缶	空缶	アキカン	0
空罐	名詞,普通名詞,一般,*,*,*	空き缶	空罐	アキカン	0
空きカン	名詞,普通名詞,一般,*,*,*	空き缶	空きカン	アキカン	0
EOS
```

```python
from sudachipy import Dictionary, SplitMode

tokenizer = Dictionary().create()

morphemes = tokenizer.tokenize("国会議事堂前駅")
print(morphemes[0].surface())  # '国会議事堂前駅'
print(morphemes[0].reading_form())  # 'コッカイギジドウマエエキ'
print(morphemes[0].part_of_speech())  # ['名詞', '固有名詞', '一般', '*', '*', '*']

morphemes = tokenizer.tokenize("国会議事堂前駅", SplitMode.A)
print([m.surface() for m in morphemes])  # ['国会', '議事', '堂', '前', '駅']
```


## Setup

You need SudachiPy and a dictionary.

### Step 1. Install SudachiPy

```bash
$ pip install sudachipy
```

### Step 2. Get a Dictionary

You can get dictionary as a Python package. It make take a while to download the dictionary file (around 70MB for the `core` edition).

```bash
$ pip install sudachidict_core
```

Alternatively, you can choose other dictionary editions. See [this section](#dictionary-edition) for the detail.


## Usage: As a command

There is a CLI command `sudachipy`.

```bash
$ echo "外国人参政権" | sudachipy
外国人参政権	名詞,普通名詞,一般,*,*,*	外国人参政権
EOS
$ echo "外国人参政権" | sudachipy -m A
外国	名詞,普通名詞,一般,*,*,*	外国
人	接尾辞,名詞的,一般,*,*,*	人
参政	名詞,普通名詞,一般,*,*,*	参政
権	接尾辞,名詞的,一般,*,*,*	権
EOS
```

```bash
$ sudachipy tokenize -h
usage: sudachipy tokenize [-h] [-r file] [-m {A,B,C}] [-o file] [-s string]
                          [-a] [-d] [-v]
                          [file [file ...]]

Tokenize Text

positional arguments:
  file           text written in utf-8

optional arguments:
  -h, --help     show this help message and exit
  -r file        the setting file in JSON format
  -m {A,B,C}     the mode of splitting
  -o file        the output file
  -s string      sudachidict type
  -a             print all of the fields
  -d             print the debug information
  -v, --version  print sudachipy version
```

__Note: The Debug option (`-d`) is disabled in version 0.6.0.__


### Output

Columns are tab separated.

- Surface
- Part-of-Speech Tags (comma separated)
- Normalized Form

When you add the `-a` option, it additionally outputs

- Dictionary Form
- Reading Form
- Dictionary ID
  - `0` for the system dictionary
  - `1` and above for the [user dictionaries](#user-dictionary)
  - `-1` if a word is Out-of-Vocabulary (not in the dictionary)
- Synonym group IDs
- `(OOV)` if a word is Out-of-Vocabulary (not in the dictionary)

```bash
$ echo "外国人参政権" | sudachipy -a
外国人参政権	名詞,普通名詞,一般,*,*,*	外国人参政権	外国人参政権	ガイコクジンサンセイケン	0	[]
EOS
```

```bash
echo "阿quei" | sudachipy -a
阿	名詞,普通名詞,一般,*,*,*	阿	阿		-1	[]	(OOV)
quei	名詞,普通名詞,一般,*,*,*	quei	quei		-1	[]	(OOV)
EOS
```


## Usage: As a Python package

### API

See [API reference page](https://worksapplications.github.io/sudachi.rs/python/).


### Example

```python
from sudachipy import Dictionary, SplitMode

tokenizer_obj = Dictionary().create()
```

```python
# Multi-granular Tokenization

# SplitMode.C is the default mode
[m.surface() for m in tokenizer_obj.tokenize("国家公務員", SplitMode.C)]
# => ['国家公務員']

[m.surface() for m in tokenizer_obj.tokenize("国家公務員", SplitMode.B)]
# => ['国家', '公務員']

[m.surface() for m in tokenizer_obj.tokenize("国家公務員", SplitMode.A)]
# => ['国家', '公務', '員']
```

```python
# Morpheme information

m = tokenizer_obj.tokenize("食べ")[0]

m.surface() # => '食べ'
m.dictionary_form() # => '食べる'
m.reading_form() # => 'タベ'
m.part_of_speech() # => ['動詞', '一般', '*', '*', '下一段-バ行', '連用形-一般']
```

```python
# Normalization

tokenizer_obj.tokenize("附属", mode)[0].normalized_form()
# => '付属'
tokenizer_obj.tokenize("SUMMER", mode)[0].normalized_form()
# => 'サマー'
tokenizer_obj.tokenize("シュミレーション", mode)[0].normalized_form()
# => 'シミュレーション'
```

(With `20210802` `core` dictionary. The results may change when you use other versions)


## Dictionary Edition

There are three editions of Sudachi Dictionary, namely, `small`, `core`, and `full`. See [WorksApplications/SudachiDict](https://github.com/WorksApplications/SudachiDict) for the detail.

SudachiPy uses `sudachidict_core` by default.

Dictionaries are installed as Python packages `sudachidict_small`, `sudachidict_core`, and `sudachidict_full`.

* [SudachiDict-small · PyPI](https://pypi.org/project/SudachiDict-small/)
* [SudachiDict-core · PyPI](https://pypi.org/project/SudachiDict-core/)
* [SudachiDict-full · PyPI](https://pypi.org/project/SudachiDict-full/)

The dictionary files are not in the package itself, but it is downloaded upon installation.

### Dictionary option: command line

You can specify the dictionary with the tokenize option `-s`.

```bash
$ pip install sudachidict_small
$ echo "外国人参政権" | sudachipy -s small
```

```bash
$ pip install sudachidict_full
$ echo "外国人参政権" | sudachipy -s full
```


### Dictionary option: Python package

You can specify the dictionary with the `Dicionary()` argument; `config_path` or `dict_type`.

```python
class Dictionary(config_path=None, resource_dir=None, dict_type=None)
```

1. `config_path`
    * You can specify the file path to the setting file with `config_path` (See [Dictionary in The Setting File](#Dictionary in The Setting File) for the detail).
    * If the dictionary file is specified in the setting file as `systemDict`, SudachiPy will use the dictionary.
2. `dict_type`
    * You can also specify the dictionary type with `dict_type`.
    * The available arguments are `small`, `core`, or `full`.
    * If different dictionaries are specified with `config_path` and `dict_type`, **a dictionary defined `dict_type` overrides** those defined in the config path.

```python
from sudachipy import Dictionary

# default: sudachidict_core
tokenizer_obj = Dictionary().create()

# The dictionary given by the `systemDict` key in the config file (/path/to/sudachi.json) will be used
tokenizer_obj = Dictionary(config_path="/path/to/sudachi.json").create()

# The dictionary specified by `dict_type` will be set.
tokenizer_obj = Dictionary(dict_type="core").create()  # sudachidict_core (same as default)
tokenizer_obj = Dictionary(dict_type="small").create()  # sudachidict_small
tokenizer_obj = Dictionary(dict_type="full").create()  # sudachidict_full

# The dictionary specified by `dict_type` overrides those defined in the config path.
# In the following code, `sudachidict_full` will be used regardless of a dictionary defined in the config file.
tokenizer_obj = Dictionary(config_path="/path/to/sudachi.json", dict_type="full").create()
```


### Dictionary in The Setting File

Alternatively, if the dictionary file is specified in the setting file, `sudachi.json`, SudachiPy will use that file.

```js
{
    "systemDict" : "relative/path/from/resourceDir/to/system.dic",
    ...
}
```

The default setting file is [sudachi.json](https://github.com/WorksApplications/sudachi.rs/blob/develop/python/py_src/sudachi/resources/sudachi.json). You can specify your `sudachi.json` with the `-r` option.

```bash
$ sudachipy -r path/to/sudachi.json
```


## User Dictionary

To use a user dictionary, `user.dic`, place [sudachi.json](https://github.com/WorksApplications/sudachi.rs/blob/develop/python/py_src/sudachi/resources/sudachi.json) to anywhere you like, and add `userDict` value with the relative path from `sudachi.json` to your `user.dic`.

```js
{
    "userDict" : ["relative/path/to/user.dic"],
    ...
}
```

Then specify your `sudachi.json` with the `-r` option.

```bash
$ sudachipy -r path/to/sudachi.json
```


You can build a user dictionary with the subcommand `ubuild`.


```bash
$ sudachipy ubuild -h
usage: sudachipy ubuild [-h] [-d string] [-o file] [-s file] file [file ...]

Build User Dictionary

positional arguments:
  file        source files with CSV format (one or more)

optional arguments:
  -h, --help  show this help message and exit
  -d string   description comment to be embedded on dictionary
  -o file     output file (default: user.dic)
  -s file     system dictionary path (default: system core dictionary path)
```

About the dictionary file format, please refer to [this document](https://github.com/WorksApplications/Sudachi/blob/develop/docs/user_dict.md) (written in Japanese, English version is not available yet).


## Customized System Dictionary

```bash
$ sudachipy build -h
usage: sudachipy build [-h] [-o file] [-d string] -m file file [file ...]

Build Sudachi Dictionary

positional arguments:
  file        source files with CSV format (one of more)

optional arguments:
  -h, --help  show this help message and exit
  -o file     output file (default: system.dic)
  -d string   description comment to be embedded on dictionary

required named arguments:
  -m file     connection matrix file with MeCab's matrix.def format
```

To use your customized `system.dic`, place [sudachi.json](https://github.com/WorksApplications/sudachi.rs/blob/develop/python/py_src/sudachi/resources/sudachi.json) to anywhere you like, and overwrite `systemDict` value with the relative path from `sudachi.json` to your `system.dic`.

```js
{
    "systemDict" : "relative/path/to/system.dic",
    ...
}
```

Then specify your `sudachi.json` with the `-r` option.

```bash
$ sudachipy -r path/to/sudachi.json
```


## For Developers

### Build from source

#### Install sdist via pip

1. Install python module `setuptools` and `setuptools-rust`.
2. Run `./build-sdist.sh` in `python` dir.
    - source distribution will be generated under `python/dist/` dir.
3. Install it via pip: `pip install ./python/dist/SudachiPy-[version].tar.gz`


#### Install develop build

1. Install python module `setuptools` and `setuptools-rust`.
2. Run `python3 setup.py develop`.
    - `develop` will create a debug build, while `install` will create a release build.
3. Now you can import the module by `import sudachipy`.

ref: [setuptools-rust](https://github.com/PyO3/setuptools-rust)


### Test

Run `build_and_test.sh` to run the tests.


## Contact

Sudachi and SudachiPy are developed by [WAP Tokushima Laboratory of AI and NLP](http://nlp.worksap.co.jp/).

Open an issue, or come to our Slack workspace for questions and discussion.

https://sudachi-dev.slack.com/ (Get invitation [here](https://join.slack.com/t/sudachi-dev/shared_invite/enQtMzg2NTI2NjYxNTUyLTMyYmNkZWQ0Y2E5NmQxMTI3ZGM3NDU0NzU4NGE1Y2UwYTVmNTViYjJmNDI0MWZiYTg4ODNmMzgxYTQ3ZmI2OWU))

Enjoy tokenization!
