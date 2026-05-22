#!/bin/bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
VENV_NAME="${VENV_NAME:-$SCRIPT_DIR/.env}"
PYTHON_BIN="${PYTHON:-python}"

case "$VENV_NAME" in
    /*|[A-Za-z]:*) ;;
    *) VENV_NAME="$PWD/$VENV_NAME" ;;
esac

if ! command -v uv >/dev/null 2>&1 ; then
    echo "uv is required to build and test SudachiPy" >&2
    exit 1
fi

if ! [ -e "$VENV_NAME" ] ; then
    uv venv --python "$PYTHON_BIN" "$VENV_NAME"
fi

if [ -x "$VENV_NAME/bin/python" ] ; then
    VENV_PYTHON="$VENV_NAME/bin/python"
elif [ -x "$VENV_NAME/Scripts/python.exe" ] ; then
    VENV_PYTHON="$VENV_NAME/Scripts/python.exe"
else
    echo "could not find Python executable in $VENV_NAME" >&2
    exit 1
fi

(cd "$REPO_ROOT" && uv pip install --python "$VENV_PYTHON" -e ".[tests]")
"$VENV_PYTHON" -m unittest discover -s "$SCRIPT_DIR/tests"
