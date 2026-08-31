#!/usr/bin/env python3
"""Capture the OFL variable-font corpus through HarfBuzz's public APIs."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import pathlib
import re
import subprocess
import sys

from capture_harfbuzz_variable import (
    AxisInfo,
    Variation,
    bind,
    draw,
    tag_text,
    tag_value,
)


HARFBUZZ_VERSION = "14.4.0"
TEXT = "AHfixÅé"
METRIC_TAGS = ("hasc", "hdsc", "hlgp", "xhgt", "cpht")
FIXTURES = {
    "noto-sans": {
        "name": "OFL-1.1 Noto Sans variable conformance subset",
        "capture_path": (
            "conformance/font/fixtures/noto-sans-variable-subset/"
            "NotoSans-variable-subset.ttf"
        ),
    },
    "recursive": {
        "name": "OFL-1.1 Recursive variable conformance subset",
        "capture_path": (
            "conformance/font/fixtures/recursive-variable-subset/"
            "Recursive-variable-subset.ttf"
        ),
    },
}


def location(axes: list[dict[str, object]], kind: str) -> dict[str, float]:
    result: dict[str, float] = {}
    for axis in axes:
        tag = str(axis["tag"])
        minimum = float(axis["minimum"])
        default = float(axis["default"])
        maximum = float(axis["maximum"])
        if kind == "default":
            value = default
        elif kind == "minimum":
            value = minimum
        elif kind == "maximum":
            value = maximum
        elif kind == "interior":
            value = (minimum + maximum) / 2.0
        else:
            raise ValueError(f"unknown location kind: {kind}")
        result[tag] = value
    return result


def shape(
    path: pathlib.Path, settings: dict[str, float], axis_tags: list[str]
) -> str:
    variations = ",".join(f"{tag}={settings[tag]:g}" for tag in axis_tags)
    return subprocess.check_output(
        [
            "hb-shape",
            str(path),
            TEXT,
            "--direction=ltr",
            "--language=en",
            "--no-glyph-names",
            f"--variations={variations}",
        ],
        text=True,
    ).strip()


def glyph_ids(serialized: str) -> list[int]:
    return [int(value) for value in re.findall(r"(?:\[|\|)(\d+)=", serialized)]


def metric(library: ctypes.CDLL, font: int, tag: str) -> int | None:
    value = ctypes.c_int()
    available = library.hb_ot_metrics_get_position(
        font, tag_value(tag), ctypes.byref(value)
    )
    return value.value if available else None


def capture(path: pathlib.Path, fixture_key: str) -> dict[str, object]:
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
        if version != HARFBUZZ_VERSION or shape_version != version:
            raise RuntimeError(
                f"HarfBuzz library and CLI must both be {HARFBUZZ_VERSION}"
            )

        axis_count = library.hb_ot_var_get_axis_count(face)
        requested = ctypes.c_uint(axis_count)
        axis_array = (AxisInfo * axis_count)()
        returned = library.hb_ot_var_get_axis_infos(
            face, 0, ctypes.byref(requested), axis_array
        )
        if returned != axis_count or requested.value != axis_count:
            raise RuntimeError("HarfBuzz returned an incomplete axis array")
        axes = [
            {
                "tag": tag_text(axis.tag),
                "minimum": axis.minimum,
                "default": axis.default,
                "maximum": axis.maximum,
                "hidden": bool(axis.flags & 1),
                "name_id": axis.name_id,
            }
            for axis in axis_array
        ]
        axis_tags = [str(axis["tag"]) for axis in axes]
        cases = []
        for label in ("default", "minimum", "maximum", "interior"):
            settings = location(axes, label)
            variations = (Variation * axis_count)(
                *(Variation(tag_value(tag), settings[tag]) for tag in axis_tags)
            )
            library.hb_font_set_variations(font, variations, axis_count)
            length = ctypes.c_uint()
            normalized = library.hb_font_get_var_coords_normalized(
                font, ctypes.byref(length)
            )
            if length.value != axis_count:
                raise RuntimeError("HarfBuzz returned an incomplete coordinate vector")
            serialized = shape(path, settings, axis_tags)
            shaped_glyphs = glyph_ids(serialized)
            if not shaped_glyphs:
                raise RuntimeError(f"{fixture_key} {label} produced no glyphs")
            cases.append(
                {
                    "label": label,
                    "user": settings,
                    "normalized_2_14": [
                        normalized[index] for index in range(axis_count)
                    ],
                    "text": TEXT,
                    "serialized_glyphs": serialized,
                    "glyph_advances_font_units": [
                        library.hb_font_get_glyph_h_advance(font, glyph_id)
                        for glyph_id in shaped_glyphs
                    ],
                    "outline_glyph_id": shaped_glyphs[0],
                    "outline_serialized_path": draw(
                        library, font, shaped_glyphs[0]
                    ),
                    "global_metrics_font_units": {
                        tag: metric(library, font, tag) for tag in METRIC_TAGS
                    },
                }
            )

        fixture = FIXTURES[fixture_key]
        return {
            "schema_version": 1,
            "tool": "HarfBuzz public C API and hb-shape",
            "version": version,
            "capture_command": (
                "python3 tools/font/capture_harfbuzz_corpus.py "
                f"--fixture {fixture_key} {fixture['capture_path']}"
            ),
            "fixture": fixture["name"],
            "font_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "axes": axes,
            "named_instance_count": library.hb_ot_var_get_named_instance_count(face),
            "cases": cases,
        }
    finally:
        library.hb_font_destroy(font)
        library.hb_face_destroy(face)
        library.hb_blob_destroy(blob)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("font", type=pathlib.Path)
    parser.add_argument("--fixture", choices=sorted(FIXTURES), required=True)
    parser.add_argument("--output", type=pathlib.Path)
    arguments = parser.parse_args()
    result = json.dumps(
        capture(arguments.font, arguments.fixture), indent=2, sort_keys=True
    ) + "\n"
    if arguments.output:
        arguments.output.write_text(result, encoding="utf-8")
    else:
        sys.stdout.write(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
