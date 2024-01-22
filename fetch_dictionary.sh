#!/bin/sh

DICT_VERSION=${1:-"latest"}
DICT_TYPE=${2:-"core"}

DICT_NAME="sudachi-dictionary-${DICT_VERSION}-${DICT_TYPE}"

echo "Downloading a dictionary file \`${DICT_NAME}\` ..."
echo

curl -L \
    https://d2ej7fkh96fzlu.cloudfront.net/sudachidict/${DICT_NAME}.zip \
    > ${DICT_NAME}.zip

unzip -j ${DICT_NAME}.zip -d ${DICT_NAME}

mv ${DICT_NAME}/system_${DICT_TYPE}.dic resources/system.dic

rm -rf ${DICT_NAME}.zip ${DICT_NAME}

echo
echo "Placed a dictionary file to \`resources/system.dic\` ."
