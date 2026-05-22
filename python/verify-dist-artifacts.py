#!/usr/bin/env python3

from __future__ import annotations

import re
import sys
import tarfile
import zipfile
from pathlib import Path


WHEEL_REQUIRED = (
    "sudachipy/__init__.py",
    "sudachipy/sudachipy.pyi",
    "sudachipy/resources/sudachi.json",
    "sudachipy/resources/char.def",
    "sudachipy/resources/rewrite.def",
    "sudachipy/resources/unk.def",
)

SDIST_REQUIRED = (
    "pyproject.toml",
    "Cargo.toml",
    "Cargo.lock",
    "python/Cargo.toml",
    "sudachi/Cargo.toml",
    "resources/sudachi.json",
    "resources/char.def",
    "resources/rewrite.def",
    "resources/unk.def",
    "python/py_src/sudachipy/resources/sudachi.json",
)


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def wheel_names(path: Path) -> set[str]:
    with zipfile.ZipFile(path) as zf:
        return set(zf.namelist())


def sdist_names(path: Path) -> set[str]:
    with tarfile.open(path, "r:*") as tf:
        names = set()
        for member in tf.getmembers():
            parts = Path(member.name).parts
            if len(parts) > 1:
                names.add(str(Path(*parts[1:])))
        return names


def check_wheel(path: Path) -> None:
    names = wheel_names(path)
    for required in WHEEL_REQUIRED:
        if required not in names:
            fail(f"{path.name} is missing {required}")

    if not any(re.fullmatch(r"sudachipy/sudachipy.*\.(so|pyd)", name) for name in names):
        fail(f"{path.name} is missing the compiled sudachipy extension")

    if "-linux_" in path.name:
        fail(f"{path.name} has an unrepaired linux platform tag")

    if not (
        "cp310-abi3" in path.name
        or "cp313-cp313t" in path.name
        or "cp314-cp314t" in path.name
    ):
        fail(f"{path.name} has an unexpected Python ABI tag")


def check_sdist(path: Path) -> None:
    names = sdist_names(path)
    for required in SDIST_REQUIRED:
        if required not in names:
            fail(f"{path.name} is missing {required}")


def main() -> None:
    dist_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("dist")
    wheels = sorted(dist_dir.glob("*.whl"))
    sdists = sorted(dist_dir.glob("*.tar.gz"))

    if not wheels:
        fail(f"no wheels found in {dist_dir}")
    if not sdists:
        fail(f"no sdist found in {dist_dir}")

    for wheel in wheels:
        check_wheel(wheel)
    for sdist in sdists:
        check_sdist(sdist)

    print(f"verified {len(wheels)} wheel(s) and {len(sdists)} sdist(s)")


if __name__ == "__main__":
    main()
