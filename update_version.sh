#!/bin/bash

set -eu

if [ $# -lt 2 ] && [ "$1" != "show" ] ; then
    echo "Provide 2 arguments [from] and [to] to update version, or 'show' to print current one."
    exit 1
fi

VERSION_CRR=$(cat ./sudachi/Cargo.toml | grep -m1 "version \?=" | sed -r 's/^.*"(.*)".*$/\1/')

if [ $1 = "show" ] ; then
    echo ${VERSION_CRR}
    exit 0
fi

VERSION_FROM=$1
VERSION_TO=$2

if [ $VERSION_FROM != $VERSION_CRR ] ; then
    echo "Specified base version ${VERSION_FROM} does not match the current version ${VERSION_CRR}."
    exit 1
fi

# update
echo "Update version from ${VERSION_FROM} to ${VERSION_TO}"

CARGO_FILES="$(find . -name Cargo.toml)"

for FILE in $CARGO_FILES ; do
    echo $FILE
    # replace the first occurrence of `^version = "<version>"$`
    sed -i -r "1,/^version = / s/^version = \"${VERSION_FROM}\"$/version = \"${VERSION_TO}\"/" $FILE
done

PY_SETUP_FILE="./python/setup.py"
echo $PY_SETUP_FILE
sed -i -r "1,/^ *version=/ s/^ *version=\"${VERSION_FROM}\",$/    version=\"${VERSION_TO}\",/" $PY_SETUP_FILE

PY_INIT_FILE="./python/py_src/sudachipy/__init__.py"
echo $PY_INIT_FILE
sed -i -r "s/^__version__ = \"${VERSION_FROM}\"$/__version__ = \"${VERSION_TO}\"/" $PY_INIT_FILE

PYDOC_CONF_FILE="./python/docs/source/conf.py"
echo $PYDOC_CONF_FILE
sed -i -r "1,/^release = '/ s/^release = '${VERSION_FROM}'$/release = '${VERSION_TO}'/" $PYDOC_CONF_FILE


# check
echo ""
echo "files which include the previous version number:"

set +e # allow grep to exit with 1 (no line match)

git grep -F "$VERSION_FROM"
