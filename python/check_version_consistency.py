#!/usr/bin/env python3

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent


def read_toml(path: Path) -> dict:
    with path.open("rb") as f:
        return tomllib.load(f)


def find(pattern: str, text: str, label: str) -> str:
    match = re.search(pattern, text, re.MULTILINE)
    if not match:
        print(f"could not find {label}", file=sys.stderr)
        raise SystemExit(1)
    return match.group(1)


def main() -> None:
    cargo_version = read_toml(ROOT / "Cargo.toml")["workspace"]["package"]["version"]
    pyproject_version = read_toml(ROOT / "pyproject.toml")["project"]["version"]

    init_text = (ROOT / "python/py_src/sudachipy/__init__.py").read_text(encoding="utf-8")
    init_version = find(r'^__version__ = "([^"]+)"$', init_text, "sudachipy __version__")

    docs_text = (ROOT / "python/docs/source/conf.py").read_text(encoding="utf-8")
    docs_version = find(r"^release = '([^']+)'$", docs_text, "docs release")

    versions = {
        "Cargo workspace": cargo_version,
        "pyproject": pyproject_version,
        "sudachipy.__version__": init_version,
        "docs release": docs_version,
    }
    if len(set(versions.values())) != 1:
        for label, version in versions.items():
            print(f"{label}: {version}", file=sys.stderr)
        raise SystemExit(1)

    print(f"version metadata is consistent: {cargo_version}")


if __name__ == "__main__":
    main()
