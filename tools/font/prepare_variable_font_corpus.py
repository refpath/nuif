#!/usr/bin/env python3
"""Prepare the pinned OFL variable-font corpus used by conformance gates."""

from __future__ import annotations

import argparse
import hashlib
import pathlib
import shutil
import struct
import subprocess
from dataclasses import dataclass


HARFBUZZ_VERSION = "14.4.0"
UNICODES = "41,48,66,69,78,C5,E9"
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


@dataclass(frozen=True)
class Fixture:
    source_sha256: str
    license_sha256: str
    output_sha256: str
    output_directory: str
    output_filename: str
    maximum_bytes: int


FIXTURES = {
    "noto-sans": Fixture(
        source_sha256="bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d",
        license_sha256="cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a",
        output_sha256="0afd77effc877ff84fa7995a58c396c124514855f8084056846b54b8cb76f3ce",
        output_directory="noto-sans-variable-subset",
        output_filename="NotoSans-variable-subset.ttf",
        maximum_bytes=16 * 1024,
    ),
    "recursive": Fixture(
        source_sha256="653221ca467f4732fe6856ac493f6c409e9f56a7674abe36b2364acc89796f7c",
        license_sha256="f9f539cf7549bd417159dbdb9c400943a5b60a7366c2c6fbde9f095173d82479",
        output_sha256="11fca6aeeaa73644a2174d2608cab7eb5d9828f5d88a7feca2c299415f3fa604",
        output_directory="recursive-variable-subset",
        output_filename="Recursive-variable-subset.ttf",
        maximum_bytes=72 * 1024,
    ),
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


def prepare(
    name: str,
    fixture: Fixture,
    source_font: pathlib.Path,
    source_license: pathlib.Path,
    output_root: pathlib.Path,
) -> None:
    require_digest(source_font, fixture.source_sha256, f"{name} source font")
    require_digest(source_license, fixture.license_sha256, f"{name} source license")
    if not REQUIRED_TABLES <= table_tags(source_font):
        raise RuntimeError(f"{name} source font lacks a required variable table")

    output_directory = output_root / fixture.output_directory
    output_directory.mkdir(parents=True, exist_ok=True)
    output_font = output_directory / fixture.output_filename
    output_license = output_directory / "OFL.txt"
    subprocess.run(
        [
            "hb-subset",
            str(source_font),
            f"--unicodes={UNICODES}",
            "--name-IDs=*",
            "--name-languages=*",
            "--layout-features=*",
            f"--output-file={output_font}",
        ],
        check=True,
    )
    shutil.copyfile(source_license, output_license)

    require_digest(output_font, fixture.output_sha256, f"{name} derived font")
    require_digest(output_license, fixture.license_sha256, f"{name} copied license")
    if not REQUIRED_TABLES <= table_tags(output_font):
        raise RuntimeError(f"{name} derived font lost a required variable table")
    if output_font.stat().st_size > fixture.maximum_bytes:
        raise RuntimeError(f"{name} derived font exceeds its fixture byte budget")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("noto_sans_font", type=pathlib.Path)
    parser.add_argument("noto_sans_license", type=pathlib.Path)
    parser.add_argument("recursive_font", type=pathlib.Path)
    parser.add_argument("recursive_license", type=pathlib.Path)
    parser.add_argument("output_root", type=pathlib.Path)
    arguments = parser.parse_args()

    version = subprocess.check_output(["hb-subset", "--version"], text=True).split()[-1]
    if version != HARFBUZZ_VERSION:
        raise RuntimeError(
            f"hb-subset {HARFBUZZ_VERSION} is required; observed {version}"
        )

    prepare(
        "noto-sans",
        FIXTURES["noto-sans"],
        arguments.noto_sans_font,
        arguments.noto_sans_license,
        arguments.output_root,
    )
    prepare(
        "recursive",
        FIXTURES["recursive"],
        arguments.recursive_font,
        arguments.recursive_license,
        arguments.output_root,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
