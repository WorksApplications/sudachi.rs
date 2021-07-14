#!/bin/sh

DICT_NAME_BASE="sudachi-dictionary-20200722"
DICT_TYPE="core"
DICT_NAME="${DICT_NAME_BASE}-${DICT_TYPE}"

echo "Downloading a dictionary file \`${DICT_NAME}\` ..."
echo

curl \
    http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict/${DICT_NAME}.zip \
    > ${DICT_NAME}.zip

unzip ${DICT_NAME}.zip

mv ${DICT_NAME_BASE}/system_${DICT_TYPE}.dic src/resources/system.dic

rm -rf ${DICT_NAME}.zip ${DICT_NAME_BASE}

echo
echo "Placed a dictionary file to \`src/resources/system.dic\` ."
