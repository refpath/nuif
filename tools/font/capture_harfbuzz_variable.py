#!/usr/bin/env python3
"""Capture the RFC 0013 variable-font oracle through HarfBuzz's public C API."""

from __future__ import annotations

import argparse
import ctypes
import ctypes.util
import hashlib
import json
import math
import pathlib
import subprocess
import sys


class AxisInfo(ctypes.Structure):
    _fields_ = [
        ("axis_index", ctypes.c_uint),
        ("tag", ctypes.c_uint32),
        ("name_id", ctypes.c_uint),
        ("flags", ctypes.c_uint),
        ("minimum", ctypes.c_float),
        ("default", ctypes.c_float),
        ("maximum", ctypes.c_float),
        ("reserved", ctypes.c_uint),
    ]


class Variation(ctypes.Structure):
    _fields_ = [("tag", ctypes.c_uint32), ("value", ctypes.c_float)]


MoveCallback = ctypes.CFUNCTYPE(
    None,
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_float,
    ctypes.c_float,
    ctypes.c_void_p,
)
LineCallback = MoveCallback
QuadraticCallback = ctypes.CFUNCTYPE(
    None,
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_float,
    ctypes.c_float,
    ctypes.c_float,
    ctypes.c_float,
    ctypes.c_void_p,
)
CubicCallback = ctypes.CFUNCTYPE(
    None,
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_float,
    ctypes.c_float,
    ctypes.c_float,
    ctypes.c_float,
    ctypes.c_float,
    ctypes.c_float,
    ctypes.c_void_p,
)
CloseCallback = ctypes.CFUNCTYPE(
    None, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p
)


CASES = [
    ("default", {"FILL": 0.0, "GRAD": 0.0, "opsz": 24.0, "wght": 400.0}),
    ("maximum", {"FILL": 1.0, "GRAD": 200.0, "opsz": 48.0, "wght": 700.0}),
    ("minimum", {"FILL": 0.0, "GRAD": -50.0, "opsz": 20.0, "wght": 100.0}),
    ("positive_interior", {"FILL": 0.5, "GRAD": 75.0, "opsz": 36.0, "wght": 550.0}),
    ("negative_interior", {"FILL": 0.25, "GRAD": -25.0, "opsz": 22.0, "wght": 250.0}),
]

SHAPING_CASES = CASES + [
    (
        "feature_variation_below",
        {"FILL": 0.98, "GRAD": 0.0, "opsz": 24.0, "wght": 400.0},
    ),
    (
        "feature_variation_at",
        {"FILL": 0.99, "GRAD": 0.0, "opsz": 24.0, "wght": 400.0},
    ),
]


def library_path() -> str:
    discovered = ctypes.util.find_library("harfbuzz")
    if discovered:
        return discovered
    libdir = subprocess.check_output(
        ["pkg-config", "--variable=libdir", "harfbuzz"], text=True
    ).strip()
    for name in ("libharfbuzz.dylib", "libharfbuzz.so"):
        candidate = pathlib.Path(libdir, name)
        if candidate.exists():
            return str(candidate)
    raise RuntimeError("HarfBuzz shared library not found")


def bind() -> ctypes.CDLL:
    library = ctypes.CDLL(library_path())
    library.hb_version_string.restype = ctypes.c_char_p
    library.hb_blob_create_from_file.argtypes = [ctypes.c_char_p]
    library.hb_blob_create_from_file.restype = ctypes.c_void_p
    library.hb_face_create.argtypes = [ctypes.c_void_p, ctypes.c_uint]
    library.hb_face_create.restype = ctypes.c_void_p
    library.hb_font_create.argtypes = [ctypes.c_void_p]
    library.hb_font_create.restype = ctypes.c_void_p
    library.hb_ot_font_set_funcs.argtypes = [ctypes.c_void_p]
    library.hb_ot_var_get_axis_count.argtypes = [ctypes.c_void_p]
    library.hb_ot_var_get_axis_count.restype = ctypes.c_uint
    library.hb_ot_var_get_axis_infos.argtypes = [
        ctypes.c_void_p,
        ctypes.c_uint,
        ctypes.POINTER(ctypes.c_uint),
        ctypes.POINTER(AxisInfo),
    ]
    library.hb_ot_var_get_named_instance_count.argtypes = [ctypes.c_void_p]
    library.hb_ot_var_get_named_instance_count.restype = ctypes.c_uint
    library.hb_font_set_variations.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(Variation),
        ctypes.c_uint,
    ]
    library.hb_font_get_var_coords_normalized.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_uint),
    ]
    library.hb_font_get_var_coords_normalized.restype = ctypes.POINTER(ctypes.c_int)
    library.hb_font_get_glyph_h_advance.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
    library.hb_font_get_glyph_h_advance.restype = ctypes.c_int
    library.hb_draw_funcs_create.restype = ctypes.c_void_p
    library.hb_draw_funcs_set_move_to_func.argtypes = [
        ctypes.c_void_p,
        MoveCallback,
        ctypes.c_void_p,
        ctypes.c_void_p,
    ]
    library.hb_draw_funcs_set_line_to_func.argtypes = [
        ctypes.c_void_p,
        LineCallback,
        ctypes.c_void_p,
        ctypes.c_void_p,
    ]
    library.hb_draw_funcs_set_quadratic_to_func.argtypes = [
        ctypes.c_void_p,
        QuadraticCallback,
        ctypes.c_void_p,
        ctypes.c_void_p,
    ]
    library.hb_draw_funcs_set_cubic_to_func.argtypes = [
        ctypes.c_void_p,
        CubicCallback,
        ctypes.c_void_p,
        ctypes.c_void_p,
    ]
    library.hb_draw_funcs_set_close_path_func.argtypes = [
        ctypes.c_void_p,
        CloseCallback,
        ctypes.c_void_p,
        ctypes.c_void_p,
    ]
    library.hb_font_draw_glyph_or_fail.argtypes = [
        ctypes.c_void_p,
        ctypes.c_uint32,
        ctypes.c_void_p,
        ctypes.c_void_p,
    ]
    library.hb_font_draw_glyph_or_fail.restype = ctypes.c_int
    library.hb_draw_funcs_destroy.argtypes = [ctypes.c_void_p]
    library.hb_font_destroy.argtypes = [ctypes.c_void_p]
    library.hb_face_destroy.argtypes = [ctypes.c_void_p]
    library.hb_blob_destroy.argtypes = [ctypes.c_void_p]
    return library


def tag_value(tag: str) -> int:
    encoded = tag.encode("ascii")
    if len(encoded) != 4:
        raise ValueError(f"variation tag must be four ASCII bytes: {tag!r}")
    return int.from_bytes(encoded, "big")


def tag_text(tag: int) -> str:
    return int(tag).to_bytes(4, "big").decode("ascii")


def shape(font: pathlib.Path, values: dict[str, float], axis_tags: list[str]) -> str:
    variations = ",".join(f"{tag}={values[tag]:g}" for tag in axis_tags)
    result = subprocess.check_output(
        [
            "hb-shape",
            str(font),
            "mail",
            "--direction=ltr",
            "--language=en",
            "--no-glyph-names",
            f"--variations={variations}",
        ],
        text=True,
    )
    return result.strip()


def quantize(value: float) -> int:
    scaled = value * 64.0
    return math.floor(scaled + 0.5) if scaled >= 0 else math.ceil(scaled - 0.5)


def draw(library: ctypes.CDLL, font: int, glyph_id: int) -> str:
    commands: list[str] = []
    contour_start: list[tuple[int, int]] = []

    @MoveCallback
    def move_to(_funcs, _data, _state, x, y, _user):
        point = (quantize(x), quantize(y))
        contour_start[:] = [point]
        commands.append(f"M{point[0]},{point[1]}")

    @LineCallback
    def line_to(_funcs, _data, _state, x, y, _user):
        commands.append(f"L{quantize(x)},{quantize(y)}")

    @QuadraticCallback
    def quadratic_to(_funcs, _data, _state, cx, cy, x, y, _user):
        commands.append(
            f"Q{quantize(cx)},{quantize(cy)} {quantize(x)},{quantize(y)}"
        )

    @CubicCallback
    def cubic_to(_funcs, _data, _state, c1x, c1y, c2x, c2y, x, y, _user):
        commands.append(
            "C"
            f"{quantize(c1x)},{quantize(c1y)} "
            f"{quantize(c2x)},{quantize(c2y)} "
            f"{quantize(x)},{quantize(y)}"
        )

    @CloseCallback
    def close_path(_funcs, _data, _state, _user):
        if contour_start and commands[-1] == f"L{contour_start[0][0]},{contour_start[0][1]}":
            commands.pop()
        commands.append("Z")

    draw_funcs = library.hb_draw_funcs_create()
    try:
        library.hb_draw_funcs_set_move_to_func(draw_funcs, move_to, None, None)
        library.hb_draw_funcs_set_line_to_func(draw_funcs, line_to, None, None)
        library.hb_draw_funcs_set_quadratic_to_func(
            draw_funcs, quadratic_to, None, None
        )
        library.hb_draw_funcs_set_cubic_to_func(draw_funcs, cubic_to, None, None)
        library.hb_draw_funcs_set_close_path_func(
            draw_funcs, close_path, None, None
        )
        if not library.hb_font_draw_glyph_or_fail(font, glyph_id, draw_funcs, None):
            raise RuntimeError(f"HarfBuzz could not draw glyph {glyph_id}")
        return "".join(commands)
    finally:
        library.hb_draw_funcs_destroy(draw_funcs)


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
        coordinates = []
        axis_tags = [axis["tag"] for axis in axes]
        for label, values in CASES:
            if set(values) != set(axis_tags):
                raise RuntimeError("capture cases do not match the font axis set")
            settings = (Variation * axis_count)(
                *(Variation(tag_value(tag), values[tag]) for tag in axis_tags)
            )
            library.hb_font_set_variations(font, settings, axis_count)
            length = ctypes.c_uint()
            normalized = library.hb_font_get_var_coords_normalized(
                font, ctypes.byref(length)
            )
            if length.value != axis_count:
                raise RuntimeError("HarfBuzz returned an incomplete coordinate vector")
            coordinates.append(
                {
                    "label": label,
                    "user": values,
                    "normalized_2_14": [normalized[index] for index in range(axis_count)],
                }
            )
        shaping = []
        for label, values in SHAPING_CASES:
            settings = (Variation * axis_count)(
                *(Variation(tag_value(tag), values[tag]) for tag in axis_tags)
            )
            library.hb_font_set_variations(font, settings, axis_count)
            serialized_glyphs = shape(path, values, axis_tags)
            glyph_id = int(serialized_glyphs.removeprefix("[").split("=", 1)[0])
            shaping.append(
                {
                    "label": label,
                    "text": "mail",
                    "user": values,
                    "serialized_glyphs": serialized_glyphs,
                    "glyph_advance_font_units": library.hb_font_get_glyph_h_advance(
                        font, glyph_id
                    ),
                    "outline_glyph_id": glyph_id,
                    "outline_serialized_path": draw(library, font, glyph_id),
                }
            )
        return {
            "schema_version": 1,
            "tool": "HarfBuzz public C API and hb-shape",
            "version": version,
            "capture_command": (
                "python3 tools/font/capture_harfbuzz_variable.py "
                "material_symbols_subset.ttf"
            ),
            "font_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "axes": axes,
            "named_instance_count": library.hb_ot_var_get_named_instance_count(face),
            "coordinates": coordinates,
            "shaping": shaping,
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
