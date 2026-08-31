#!/usr/bin/env python3
"""Capture nonzero HVAR advances from the fontations truncated-map fixture."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import pathlib
import subprocess
import sys

from capture_harfbuzz_variable import Variation, bind, tag_value


CASES = [
    ("default", 400.0),
    ("named_medium", 500.0),
    ("interior", 550.0),
    ("maximum", 700.0),
]


def shape(path: pathlib.Path, weight: float) -> str:
    return subprocess.check_output(
        [
            "hb-shape",
            str(path),
            "AIT",
            "--direction=ltr",
            "--language=en",
            "--no-glyph-names",
            f"--variations=wght={weight:g}",
        ],
        text=True,
    ).strip()


def capture(path: pathlib.Path) -> dict[str, object]:
    library = bind()
    blob = library.hb_blob_create_from_file(str(path).encode())
    if not blob:
        raise RuntimeError("HarfBuzz could not open the font")
    face = library.hb_face_create(blob, 0)
    font = library.hb_font_create(face)
    try:
        library.hb_ot_font_set_funcs(font)
        version = library.hb_version_string().decode("ascii")
        shape_version = subprocess.check_output(
            ["hb-shape", "--version"], text=True
        ).splitlines()[0].split()[-1]
        if shape_version != version:
            raise RuntimeError("HarfBuzz library and hb-shape versions differ")
        cases = []
        for label, weight in CASES:
            settings = (Variation * 1)(Variation(tag_value("wght"), weight))
            library.hb_font_set_variations(font, settings, 1)
            length = ctypes.c_uint()
            normalized = library.hb_font_get_var_coords_normalized(
                font, ctypes.byref(length)
            )
            if length.value != 1:
                raise RuntimeError("HarfBuzz returned an incomplete coordinate vector")
            cases.append(
                {
                    "label": label,
                    "user": {"wght": weight},
                    "normalized_2_14": [normalized[0]],
                    "text": "AIT",
                    "serialized_glyphs": shape(path, weight),
                    "glyph_advances_font_units": [
                        library.hb_font_get_glyph_h_advance(font, glyph_id)
                        for glyph_id in (2, 3, 4)
                    ],
                }
            )
        return {
            "schema_version": 1,
            "tool": "HarfBuzz public C API and hb-shape",
            "version": version,
            "capture_command": (
                "python3 tools/font/capture_harfbuzz_hvar.py "
                "hvar_with_truncated_adv_index_map.ttf"
            ),
            "font_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "fixture": "font-test-data 0.9.1 hvar_with_truncated_adv_index_map.ttf",
            "cases": cases,
        }
    finally:
        library.hb_font_destroy(font)
        library.hb_face_destroy(face)
        library.hb_blob_destroy(blob)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("font", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path)
    arguments = parser.parse_args()
    result = json.dumps(capture(arguments.font), indent=2, sort_keys=True) + "\n"
    if arguments.output:
        arguments.output.write_text(result, encoding="utf-8")
    else:
        sys.stdout.write(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
