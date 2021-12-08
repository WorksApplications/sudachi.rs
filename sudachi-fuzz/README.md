# Sudachi.rs Fuzzing

It uses https://github.com/rust-fuzz/honggfuzz-rs project to do coverage-based fuzzing.
Currently, fuzzing is done only on Linux.

## How to fuzz Sudachi.rs

From the current directory, run

```
nice -n 19 cargo hfuzz run sudachi-fuzz
```

If there are crashes/panics, it is possible to minimize examples by running

```
HFUZZ_RUN_ARGS='-M' cargo -vv hfuzz run sudachi-fuzz
```

To run failures with the debugger, run (need to have lldb installed)

```
cargo hfuzz run-debug sudachi-fuzz hfuzz_workspace/*/*.fuzz
```