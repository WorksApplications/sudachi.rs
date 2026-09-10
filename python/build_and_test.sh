#!/bin/bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
VENV_NAME="${VENV_NAME:-$SCRIPT_DIR/.env}"
if [ -n "${PYTHON:-}" ] ; then
    PYTHON_BIN="$PYTHON"
elif [ "${RUNNER_OS:-}" = "Windows" ] && [ -n "${pythonLocation:-}" ] ; then
    PYTHON_BIN="${pythonLocation//\\//}/python.exe"
elif command -v python >/dev/null 2>&1 ; then
    PYTHON_BIN="$(command -v python)"
elif command -v python3 >/dev/null 2>&1 ; then
    PYTHON_BIN="$(command -v python3)"
else
    echo "could not find Python executable" >&2
    exit 1
fi
if [ ! -e "$PYTHON_BIN" ] && [ -e "$PYTHON_BIN.exe" ] ; then
    PYTHON_BIN="$PYTHON_BIN.exe"
fi

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

if [ "$(uname -s)" = "Darwin" ] && "$VENV_PYTHON" -c 'import sysconfig; raise SystemExit(0 if sysconfig.get_config_var("Py_GIL_DISABLED") else 1)' ; then
    export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-undefined -C link-arg=dynamic_lookup"
fi

rm -f "$SCRIPT_DIR"/py_src/sudachipy/sudachipy*.so "$SCRIPT_DIR"/py_src/sudachipy/sudachipy*.pyd

(cd "$REPO_ROOT" && uv pip install --python "$VENV_PYTHON" -e ".[tests]")
"$VENV_PYTHON" -m unittest discover -s "$SCRIPT_DIR/tests"
