# [0.6.0](https://github.com/WorksApplications/sudachi.rs/releases/tag/v0.6.0) (2020-11-11)
## Highlights
* Full feature parity with Java version
* ~15% analysis speed improvement over 0.6.0-rc1

## Rust
* Added dictionary build functionality
  * https://github.com/WorksApplications/sudachi.rs/pull/143
* Added an option to perform analysis without sentence splitting
  * Use it with `--split-sentences=no`

## Python
* Added bindings for dictionary build (undocumented and not supported as API).
  * See https://github.com/WorksApplications/sudachi.rs/issues/157
* `sudachipy build` and `sudachipy ubuild` should work once more
  * Report on build times and dictionary part sizes can differ from the original SudachiPy


# [0.6.0-rc1](https://github.com/WorksApplications/sudachi.rs/releases/tag/v0.6.0-rc1) (2021-10-26) 
## Highlights

* First release of Sudachi.rs
* SudachiPy compatible Python bindings
* ~30x speed improvement over original SudachiPy
* Dictionary build mode will be done before 0.6.0 final (See #13)

## Rust 

* Analysis: feature parity with Python and Java version
* Dictionary build is not supported in rc1
* ~2x faster than Java version (with sentence splitting)
* No public API at the moment (contact us if you want to use Rust version directly, internals will significantly change and names are not finalized)

## Python

* Mostly compatible with SudachiPy 0.5.4
* We provide binary wheels for popular platforms
* ~30x faster than 0.5.4
* IgnoreYomigana input text plugin is now supported (and enabled by default)
* We provide [binary wheels for convenience (and additional speed on Linux)](https://worksapplications.github.io/sudachi.rs/python/wheels.html)

## Known Issues

* List of deprecated SudachiPy API:
    * `MorphemeList.empty(dict: Dictionary)`
        * This also needs a dictionary as an argument.
    * `Morpheme.split(mode: SplitMode)`
    * `Morpheme.get_word_info()`
    * Most of instance attributes are not exported: e.g. `Dictionary.grammar`, `Dictionary.lexicon`.
        * See [API reference page](https://worksapplications.github.io/sudachi.rs/python/) for supported APIs.
* Dictionary Build is not supported: `sudachipy build` and `sudachipy ubuild` will not work, please use 0.5.3 in another virtual environment for the time being until the feature is implemented: #13