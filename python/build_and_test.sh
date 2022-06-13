#!/bin/bash

cd "$(dirname 0)" || ( echo "failed to cd" && exit 1 )

VENV_NAME=".env"

# create venv
if ! [ -e $VENV_NAME ] ; then
    python -m venv $VENV_NAME
    $VENV_NAME/bin/pip install setuptools-rust
fi

source $VENV_NAME/bin/activate

# build with tests extras
pip install --no-use-pep517 --no-build-isolation -vvv -e '.[tests]'

# run test
python -m unittest
