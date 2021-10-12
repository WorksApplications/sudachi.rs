#!/bin/bash

cd `dirname $0`

VENV_NAME=".env"

# create venv
if ! [ -e $VENV_NAME ] ; then
    python -m venv $VENV_NAME
    $VENV_NAME/bin/pip install setuptools-rust
fi

source $VENV_NAME/bin/activate

# build
python setup.py develop

# run test
python -m unittest
