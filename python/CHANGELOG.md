# Change log

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased

- Print Debug feature is disabled now.
  - `-d` option of `sudachipy` cli does nothing.
  - `sudachipy.Tokenizer` will ignore the provided logger.
  - Ref: [#76]

## [0.6.3] - 2022/01/10

- Changed path resolution algorithm for resources [#203](https://github.com/WorksApplications/sudachi.rs/pull/203)
- Added set operations to `PosMatcher` [#204](https://github.com/WorksApplications/sudachi.rs/pull/204)
- Added `pos_of()` function to `Dictionary` which returns a POS tuple for a given POS id.

## [0.6.2] - 2021/12/09

- Fixed analysis differences with 0.5.4

## [0.6.1] - 2021/12/08

- [`Morpheme.part_of_speech`](https://worksapplications.github.io/sudachi.rs/python/api/sudachipy.html#sudachipy.Morpheme.part_of_speech) method now returns Tuple of POS components instead of a list.
- [Partial Dictionary Read](https://worksapplications.github.io/sudachi.rs/python/topics/subsetting.html)
  - It is possible to ask for a subset of morpheme fields instead of all fields
  - Supported API: [`Dictionary.create()`](https://worksapplications.github.io/sudachi.rs/python/api/sudachipy.html#sudachipy.Dictionary.create), [`Dictionary.pre_tokenizer()`](https://worksapplications.github.io/sudachi.rs/python/api/sudachipy.html#sudachipy.Dictionary.pre_tokenizer)
- HuggingFace PreTokenizer support
  - We provide a built-in HuggingFace-compatible pre-tokenizer
  - API: [`Dictionary.pre_tokenizer()`](https://worksapplications.github.io/sudachi.rs/python/api/sudachipy.html#sudachipy.Dictionary.pre_tokenizer)
  - It is multithreading-compatible and supports customization
- [Memory allocation reuse](https://worksapplications.github.io/sudachi.rs/python/topics/out_param.html)
  - It is possible to reduce re-allocation overhead by using `out` parameters which accept `MorphemeList`s
  - Supported API: [`Tokenizer.tokenize()`](https://worksapplications.github.io/sudachi.rs/python/api/sudachipy.html#sudachipy.Tokenizer.tokenize), [`Morpheme.split()`](https://worksapplications.github.io/sudachi.rs/python/api/sudachipy.html#sudachipy.Morpheme.split)
  - It is now a recommended way to use both those APIs
- [PosMatcher](https://worksapplications.github.io/sudachi.rs/python/api/sudachipy.html#sudachipy.Dictionary.pos_matcher)
  - New API for checking if a morpheme has a POS tag from a set
  - Strongly prefer using it instead of string comparison of POS components
- Performance
  - Greatly decreased cost of accessing POS components
- `len(Morpheme)` now returns the length of the morpheme in Unicode codepoints. Use it instead of `len(m.surface())`
- [`Morpheme.split()`](https://worksapplications.github.io/sudachi.rs/python/api/sudachipy.html#sudachipy.Morpheme.split) has new `add_single` parameter, which can be used to check whether the split has produced anything
  - E.g. with `if m.split(SplitMode.A, out=res, add_single=False): handle_splits(res)`
  - `add_single=True`, returning the list with the current morpheme is the current behavior
- `Morpheme`/`MorphemeList` now have readable `__repr__` and `__str__`
  - https://github.com/WorksApplications/sudachi.rs/pull/187

### Deprecated
* `dict_type` parameter of `Dictionary()` constructor. Use `dict` instead which is a complete alias.

### Note
* Do not use `mode` parameter of `Tokenizer.tokenize()` method if you always tokenize with a single mode.
  * Use the mode parameter of `Dictionary.create()` method instead.

## [0.6.0] - 2021/10/11

### Changed

- Support building dictionary
- `sudachidict_*` packages starting from 20210802.post1 are compatible with 0.6.0 release and will work as is

## [0.6.0-rc1] - 2021/10/22

### Note

- From this version, SudachiPy is provided as a binding of [the Rust implementation](https://github.com/WorksApplications/sudachi.rs).
- See [API reference page](https://worksapplications.github.io/sudachi.rs/python/) for all APIs.
- Since this is release-candidate version, you need to explicitly specify version to install.
    - `pip install sudachipy==0.6.0rc1`
    - You also need to install `sudachidict_*` before since installing it will overwrite this version.

### Changed

- Module structure changed: every classes locate at the root module.
    - Import is now like: `from sudachipy import Dictionary, Tokenizer`
    - You can still import them in the previous way (not recommended).
        - `from sudachipy.dictionary import Dictionary`
- `MorphemeList.empty` now needs a `sudachipy.Dictionary` instance as arguments.
    - __This method is also marked as deprecated.__

### Deprecated

- `MorphemeList.empty(dict)`
    - Users should not generate MorphemeList by themselves.
    - Use `Tokenizer.tokenize("")` if you need.
- `Morpheme.get_word_info()`
    - Users should not touch the raw WordInfo.
    - Necessary fields are provided via `Morpheme`.
        - Please create an issue if fields you need is not implemented to `Morpheme`.
- `Morpheme.split(mode)`
    - The API around this feature will change.
    - See issue [#92].

### Removed

- Some of APIs are not supported.
    - See [API reference page](https://worksapplications.github.io/sudachi.rs/python/) for the full list of supported APIs.
- Most of instance attributes are unaccessible.
    - You cannot access `Dictionary.grammar` or `Dictionary.lexicon`.


## [0.5.4]

Please see [python version repository](https://github.com/WorksApplications/SudachiPy).
