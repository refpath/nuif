#!/usr/bin/env python3
"""Prepare the pinned OFL Roboto Flex subset used by the MVAR experiment."""

from __future__ import annotations

import argparse
import hashlib
import pathlib
import shutil
import struct
import subprocess

SOURCE_SHA256 = "9b523f7d82593df0107173849ebb8c817471a1df4b4fb2c3cbf40cfd810c8281"
LICENSE_SHA256 = "9cbaed04b20c853f99840efe5dc96956f6f6120ed83a0ade35f9281a2b63e5d0"
OUTPUT_SHA256 = "4fe568be6e73133adf9eb03e87d094ddd7c73f4250c61d3356b55e2ea7886ea9"
HARFBUZZ_VERSION = "14.4.0"
REQUIRED_TABLES = {
    "HVAR",
    "MVAR",
    "OS/2",
    "STAT",
    "avar",
    "cmap",
    "fvar",
    "glyf",
    "gvar",
    "head",
    "hhea",
    "hmtx",
    "loca",
    "maxp",
    "name",
}


def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def table_tags(path: pathlib.Path) -> set[str]:
    data = path.read_bytes()
    if len(data) < 12 or data[:4] != b"\x00\x01\x00\x00":
        raise RuntimeError(f"{path} is not a single-face TrueType sfnt")
    count = struct.unpack_from(">H", data, 4)[0]
    directory_end = 12 + count * 16
    if directory_end > len(data):
        raise RuntimeError(f"{path} has a truncated sfnt directory")
    return {
        data[offset : offset + 4].decode("ascii")
        for offset in range(12, directory_end, 16)
    }


def require_digest(path: pathlib.Path, expected: str, label: str) -> None:
    observed = digest(path)
    if observed != expected:
        raise RuntimeError(f"{label} digest mismatch: {observed}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("source_font", type=pathlib.Path)
    parser.add_argument("source_license", type=pathlib.Path)
    parser.add_argument("output_directory", type=pathlib.Path)
    arguments = parser.parse_args()

    require_digest(arguments.source_font, SOURCE_SHA256, "source font")
    require_digest(arguments.source_license, LICENSE_SHA256, "source license")
    if not REQUIRED_TABLES <= table_tags(arguments.source_font):
        raise RuntimeError("source font lacks a required variable TrueType table")

    version = subprocess.check_output(["hb-subset", "--version"], text=True).split()[-1]
    if version != HARFBUZZ_VERSION:
        raise RuntimeError(
            f"hb-subset {HARFBUZZ_VERSION} is required; observed {version}"
        )

    arguments.output_directory.mkdir(parents=True, exist_ok=True)
    output_font = arguments.output_directory / "RobotoFlex-MVAR-subset.ttf"
    output_license = arguments.output_directory / "OFL.txt"
    subprocess.run(
        [
            "hb-subset",
            str(arguments.source_font),
            "--unicodes=48,78",
            "--name-IDs=*",
            "--name-languages=*",
            "--layout-features=*",
            f"--output-file={output_font}",
        ],
        check=True,
    )
    shutil.copyfile(arguments.source_license, output_license)

    require_digest(output_font, OUTPUT_SHA256, "derived font")
    require_digest(output_license, LICENSE_SHA256, "copied license")
    if not REQUIRED_TABLES <= table_tags(output_font):
        raise RuntimeError("derived font lost a required variable TrueType table")
    if output_font.stat().st_size > 32 * 1024:
        raise RuntimeError("derived font exceeds the bounded fixture budget")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
