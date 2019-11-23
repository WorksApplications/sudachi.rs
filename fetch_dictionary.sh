#!/bin/sh

DICT_NAME_BASE="sudachi-dictionary-20191030"
DICT_NAME="${DICT_NAME_BASE}-core"

echo "Downloading a dictionary file \`${DICT_NAME}\` ..."
echo

curl \
    https://object-storage.tyo2.conoha.io/v1/nc_2520839e1f9641b08211a5c85243124a/sudachi/sudachi-dictionary-20191030-core.zip \
    > ${DICT_NAME}.zip

unzip ${DICT_NAME}.zip

mv ${DICT_NAME_BASE}/system_core.dic src/resources/system.dic

rm -rf ${DICT_NAME}.zip ${DICT_NAME_BASE}

echo
echo "Placed a dictionary file to \`src/resources/system.dic\` ."