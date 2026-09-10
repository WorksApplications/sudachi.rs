#!/bin/bash
set -ex

# Generate profile for PGO.
# The build process need to use generated profile data.

DIR=$(dirname "$(readlink -f "$0")")
cd "$DIR/.."

PROFDATA=/tmp/sudachi-profdata
MERGED_PROFDATA=${1}

# Install rust toolchain only when it is not already available in the image.
if ! command -v rustc >/dev/null 2>&1; then
  curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y --no-modify-path
fi
export PATH="$HOME/.cargo/bin:$PATH"
rustup component add llvm-tools
## target-triple of the default toolchain
TARGET_TRIPLE=$(rustc -vV | awk '/^host/ {print $2}')

# Compile Binary that will generate PGO data
RUSTFLAGS="-C profile-generate=$PROFDATA -C opt-level=3" \
  cargo build --release -p sudachi-cli

# Download Kyoto Leads corpus original texts
curl -L https://github.com/ku-nlp/KWDLC/releases/download/release_1_0/leads.org.txt.gz | gzip -dc > leads.txt

# Generate Profile
target/release/sudachi -o /dev/null leads.txt
target/release/sudachi --wakati --mode=A -o /dev/null leads.txt
target/release/sudachi --all --mode=B -o /dev/null leads.txt

# Generate Merged PGO data
"$HOME/.rustup/toolchains/stable-$TARGET_TRIPLE/lib/rustlib/$TARGET_TRIPLE/bin/llvm-profdata" \
  merge -o "$MERGED_PROFDATA" "$PROFDATA"
