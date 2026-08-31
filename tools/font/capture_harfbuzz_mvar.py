#!/usr/bin/env python3
"""Capture MVAR global metrics from the pinned Roboto Flex test subset."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import pathlib
import subprocess
import sys

from capture_harfbuzz_variable import Variation, bind, tag_value

DEFAULT = {
    "opsz": 14.0,
    "wght": 400.0,
    "GRAD": 0.0,
    "wdth": 100.0,
    "slnt": 0.0,
    "XOPQ": 96.0,
    "YOPQ": 79.0,
    "XTRA": 468.0,
    "YTUC": 712.0,
    "YTLC": 514.0,
    "YTAS": 750.0,
    "YTDE": -203.0,
    "YTFI": 738.0,
}
AXIS_ORDER = list(DEFAULT)
CASES = [
    ("default", {}),
    ("x_height_minimum", {"YTLC": 416.0}),
    ("x_height_interior", {"YTLC": 542.0}),
    ("x_height_maximum", {"YTLC": 570.0}),
    ("cap_height_minimum", {"YTUC": 528.0}),
    ("cap_height_maximum", {"YTUC": 760.0}),
    ("optical_minimum", {"opsz": 8.0}),
    ("optical_maximum", {"opsz": 144.0}),
]
METRIC_TAGS = ("hasc", "hdsc", "hlgp", "xhgt", "cpht")


def location(changes: dict[str, float]) -> dict[str, float]:
    result = DEFAULT.copy()
    result.update(changes)
    return result


def shape(path: pathlib.Path, settings: dict[str, float]) -> str:
    variations = ",".join(f"{tag}={settings[tag]:g}" for tag in AXIS_ORDER)
    return subprocess.check_output(
        [
            "hb-shape",
            str(path),
            "Hx",
            "--direction=ltr",
            "--language=en",
            "--no-glyph-names",
            f"--variations={variations}",
        ],
        text=True,
    ).strip()


def metric(library: ctypes.CDLL, font: int, tag: str) -> int:
    value = ctypes.c_int()
    available = library.hb_ot_metrics_get_position(
        font, tag_value(tag), ctypes.byref(value)
    )
    if not available:
        raise RuntimeError(f"HarfBuzz did not expose the {tag} metric")
    return value.value


def capture(path: pathlib.Path) -> dict[str, object]:
    library = bind()
    library.hb_ot_metrics_get_position.argtypes = [
        ctypes.c_void_p,
        ctypes.c_uint32,
        ctypes.POINTER(ctypes.c_int),
    ]
    library.hb_ot_metrics_get_position.restype = ctypes.c_int
    blob = library.hb_blob_create_from_file(str(path).encode())
    if not blob:
        raise RuntimeError("HarfBuzz could not open the font")
    face = library.hb_face_create(blob, 0)
    font = library.hb_font_create(face)
    try:
        library.hb_ot_font_set_funcs(font)
        version = library.hb_version_string().decode("ascii")
        shape_version = (
            subprocess.check_output(["hb-shape", "--version"], text=True)
            .splitlines()[0]
            .split()[-1]
        )
        if version != "14.4.0" or shape_version != version:
            raise RuntimeError("the HarfBuzz library and CLI must both be 14.4.0")
        cases = []
        for label, changes in CASES:
            settings = location(changes)
            variations = (Variation * len(AXIS_ORDER))(
                *(Variation(tag_value(tag), settings[tag]) for tag in AXIS_ORDER)
            )
            library.hb_font_set_variations(font, variations, len(AXIS_ORDER))
            length = ctypes.c_uint()
            normalized = library.hb_font_get_var_coords_normalized(
                font, ctypes.byref(length)
            )
            if length.value != len(AXIS_ORDER):
                raise RuntimeError("HarfBuzz returned an incomplete coordinate vector")
            cases.append(
                {
                    "label": label,
                    "user": settings,
                    "normalized_2_14": [
                        normalized[index] for index in range(length.value)
                    ],
                    "text": "Hx",
                    "serialized_glyphs": shape(path, settings),
                    "global_metrics_font_units": {
                        tag: metric(library, font, tag) for tag in METRIC_TAGS
                    },
                }
            )
        return {
            "schema_version": 1,
            "tool": "HarfBuzz public C API and hb-shape",
            "version": version,
            "capture_command": (
                "python3 tools/font/capture_harfbuzz_mvar.py "
                "conformance/font/fixtures/roboto-flex-mvar-subset/"
                "RobotoFlex-MVAR-subset.ttf"
            ),
            "font_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "fixture": "OFL-1.1 Roboto Flex MVAR conformance subset",
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
