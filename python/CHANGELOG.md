# Change log

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased

- Print Debug feature is disabled now.
  - `-d` option of `sudachipy` cli does nothing.
  - `sudachipy.Tokenizer` will ignore the provided logger.
  - Ref: [#76]

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
