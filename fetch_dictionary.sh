#!/bin/sh
set -eu

DICT_VERSION=${1:-"latest"}
DICT_TYPE=${2:-"core"}
DICT_SHA256=${3:-${SUDACHI_DICT_SHA256:-}}

DICT_NAME="sudachi-dictionary-${DICT_VERSION}-${DICT_TYPE}"
DICT_ZIP="${DICT_NAME}.zip"

echo "Downloading a dictionary file \`${DICT_NAME}\` ..."
echo

curl -fL \
    https://d2ej7fkh96fzlu.cloudfront.net/sudachidict/${DICT_NAME}.zip \
    > "${DICT_ZIP}"

if [ -n "${DICT_SHA256}" ] ; then
    if command -v sha256sum >/dev/null 2>&1 ; then
        printf '%s  %s\n' "${DICT_SHA256}" "${DICT_ZIP}" | sha256sum -c -
    elif command -v shasum >/dev/null 2>&1 ; then
        ACTUAL_SHA256=$(shasum -a 256 "${DICT_ZIP}" | awk '{print $1}')
        if [ "${ACTUAL_SHA256}" != "${DICT_SHA256}" ] ; then
            echo "sha256 mismatch for ${DICT_ZIP}: expected ${DICT_SHA256}, got ${ACTUAL_SHA256}" >&2
            exit 1
        fi
    else
        echo "sha256 verification requested, but sha256sum/shasum was not found" >&2
        exit 1
    fi
fi

unzip -j "${DICT_ZIP}" -d "${DICT_NAME}"

mv "${DICT_NAME}/system_${DICT_TYPE}.dic" resources/system.dic

rm -rf "${DICT_ZIP}" "${DICT_NAME}"

echo
echo "Placed a dictionary file to \`resources/system.dic\` ."
