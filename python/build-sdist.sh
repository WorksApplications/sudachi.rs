#!/bin/bash
set -ex

## Create a symlink for sudachi.rs
ln -sf ../sudachi sudachi-lib
## Modify cargo.toml to include this symlink
sed -i 's/\.\.\/sudachi/\.\/sudachi-lib/' Cargo.toml


# Build the source distribution
python setup.py sdist


# clean up changes
## Modify cargo.toml
sed -i 's/\.\/sudachi-lib/\.\.\/sudachi/' Cargo.toml

## rm files
rm LICENSE sudachi-lib
rm resources/ -rf
