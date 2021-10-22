#!/bin/bash

set -e

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

echo "Update version from ${VERSION_FROM} to ${VERSION_TO}"

if [ $VERSION_FROM != $VERSION_CRR ] ; then
    echo "Specified base version ${VERSION_FROM} does not match the current version ${VERSION_CRR}."
    exit 1
fi

function replace() {
    # replace the first occurrence of `$1 = "<version>"` in the file $2
    echo $2
    sed -i -r "1,/$1 ?= / s/^(.*)"${VERSION_FROM}"(.*)$/\1"${VERSION_TO}"\2/" $2
}

for FILE in $(find . -name Cargo.toml) ; do
    replace "version" $FILE
done

replace "version" "./python/setup.py"
replace "release" "./python/docs/source/conf.py"
