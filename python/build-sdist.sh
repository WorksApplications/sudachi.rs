#!/bin/bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)

if ! command -v uv >/dev/null 2>&1 ; then
  echo "uv is required to build the SudachiPy sdist" >&2
  exit 1
fi

uv run --no-project --with build python -m build --installer uv --sdist --outdir "$SCRIPT_DIR/dist" "$REPO_ROOT"
