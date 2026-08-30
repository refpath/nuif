#!/usr/bin/env python3
"""Independent NUIF profile-0 reader, writer, layout, and raster verifier.

This module intentionally has no dependency on the Rust workspace.  It implements
the bounded v0 contract from the draft specification using only Python's standard
library, then compares its observations with artifacts produced by another
implementation.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import platform
import struct
import subprocess
import sys
import zlib
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

MAX_INPUT_BYTES = 16 * 1024 * 1024
PINNED_FONT_SHA256 = "f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc"
IDENTIFIER_LENGTH = 32


class ProfileError(Exception):
    """A typed failure in the independent bounded profile."""


@dataclass(frozen=True)
class Rect:
    x: float
    y: float
    width: float
    height: float

    def as_json(self) -> dict[str, float]:
        return {
            "x": self.x,
            "y": self.y,
            "width": self.width,
            "height": self.height,
        }


def _object_without_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ProfileError(f"duplicate object key: {key}")
        result[key] = value
    return result


def _reject_nonfinite(value: str) -> None:
    raise ProfileError(f"non-finite number: {value}")


def parse_document(raw: bytes) -> dict[str, Any]:
    if len(raw) > MAX_INPUT_BYTES:
        raise ProfileError(f"input exceeds {MAX_INPUT_BYTES} bytes")
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ProfileError(f"input is not UTF-8: {error}") from error
    try:
        document = json.loads(
            text,
            object_pairs_hook=_object_without_duplicates,
            parse_constant=_reject_nonfinite,
        )
    except (json.JSONDecodeError, TypeError) as error:
        raise ProfileError(f"input is not canonical JSON data: {error}") from error
    if not isinstance(document, dict):
        raise ProfileError("document root must be an object")
    validate_document(document)
    return document


def canonical_text(document: dict[str, Any]) -> bytes:
    try:
        encoded = json.dumps(
            document,
            ensure_ascii=False,
            allow_nan=False,
            indent=2,
            sort_keys=True,
        )
    except (TypeError, ValueError) as error:
        raise ProfileError(
            f"document cannot be encoded canonically: {error}"
        ) from error
    return (encoded + "\n").encode("utf-8")


def _is_identifier(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == IDENTIFIER_LENGTH
        and all(character in "0123456789abcdef" for character in value)
    )


def _walk_values(value: Any) -> Iterable[Any]:
    yield value
    if isinstance(value, dict):
        for child in value.values():
            yield from _walk_values(child)
    elif isinstance(value, list):
        for child in value:
            yield from _walk_values(child)


def validate_document(document: dict[str, Any]) -> None:
    if document.get("schema_version") != 1:
        raise ProfileError("profile 0 requires document schema_version 1")
    if not _is_identifier(document.get("id")):
        raise ProfileError("document id must be 32 lowercase hexadecimal digits")
    entities = document.get("entities")
    roots = document.get("roots")
    if not isinstance(entities, dict) or not isinstance(roots, list):
        raise ProfileError("entities must be an object and roots must be an array")
    for value in _walk_values(document):
        if isinstance(value, float) and not math.isfinite(value):
            raise ProfileError("document contains a non-finite number")

    parent: dict[str, str] = {}
    for identifier, entity in entities.items():
        if not _is_identifier(identifier) or not isinstance(entity, dict):
            raise ProfileError(f"invalid entity entry: {identifier}")
        if entity.get("id") != identifier:
            raise ProfileError(f"entity map key/id mismatch: {identifier}")
        if entity.get("schema_version") != 1:
            raise ProfileError(f"entity {identifier} does not use schema version 1")
        children = entity.get("children")
        if not isinstance(children, list):
            raise ProfileError(f"entity {identifier} children must be an array")
        for child in children:
            if child not in entities:
                raise ProfileError(
                    f"entity {identifier} references missing child {child}"
                )
            if child in parent:
                raise ProfileError(f"entity {child} has more than one parent")
            parent[child] = identifier

    if len(set(roots)) != len(roots):
        raise ProfileError("root identifiers must be unique")
    for root in roots:
        if root not in entities:
            raise ProfileError(f"missing root entity {root}")
        if root in parent:
            raise ProfileError(f"root entity {root} also has a parent")

    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(identifier: str) -> None:
        if identifier in visiting:
            raise ProfileError(f"containment cycle at entity {identifier}")
        if identifier in visited:
            return
        visiting.add(identifier)
        for child in entities[identifier]["children"]:
            visit(child)
        visiting.remove(identifier)
        visited.add(identifier)

    for root in roots:
        visit(root)
    if visited != set(entities):
        missing = sorted(set(entities) - visited)
        raise ProfileError(f"unreachable entities: {', '.join(missing)}")

    tokens = document.get("tokens", {})
    if not isinstance(tokens, dict):
        raise ProfileError("tokens must be an object")
    for identifier, token in tokens.items():
        if not _is_identifier(identifier) or token.get("id") != identifier:
            raise ProfileError(f"token map key/id mismatch: {identifier}")
    for identifier, entity in entities.items():
        for value in entity.get("authored", {}).get("values", {}).values():
            if (
                isinstance(value, dict)
                and value.get("type") == "token"
                and value.get("value") not in tokens
            ):
                raise ProfileError(f"entity {identifier} references a missing token")


def _hard_lines(text: str) -> list[str]:
    lines: list[str] = []
    start = 0
    index = 0
    while index < len(text):
        character = text[index]
        if character == "\r":
            lines.append(text[start:index])
            index += 1
            if index < len(text) and text[index] == "\n":
                index += 1
            start = index
            continue
        if character in ("\n", "\x85", "\u2028", "\u2029"):
            lines.append(text[start:index])
            index += 1
            start = index
            continue
        index += 1
    lines.append(text[start:])
    return lines


def _resolved_authored(
    entity: dict[str, Any], viewport_width: float, theme: str | None
) -> dict[str, Any]:
    authored = entity["authored"]
    resolved = {
        "width": authored["width"],
        "height": authored["height"],
        "x": float(authored["position"]["x"]),
        "y": float(authored["position"]["y"]),
        "direction": authored["layout"]["direction"],
        "gap": float(authored["layout"]["gap"]),
    }
    for rule in authored.get("responsive", []):
        predicate = rule["when"]
        minimum = predicate.get("min_width")
        maximum = predicate.get("max_width")
        required_theme = predicate.get("theme")
        matches = (
            (minimum is None or viewport_width >= float(minimum))
            and (maximum is None or viewport_width <= float(maximum))
            and (required_theme is None or theme == required_theme)
        )
        if matches:
            for field in ("width", "height", "direction", "gap"):
                if rule.get(field) is not None:
                    resolved[field] = rule[field]
    return resolved


def _intrinsic_size(
    entity: dict[str, Any], font_hashes: set[str]
) -> tuple[float, float]:
    text = entity["authored"].get("text")
    if text is None:
        return (0.0, 0.0)
    if text.get("font") != "Ahem" or text.get("font_sha256") != PINNED_FONT_SHA256:
        raise ProfileError("independent profile supports only the pinned Ahem font")
    if PINNED_FONT_SHA256 not in font_hashes:
        raise ProfileError("pinned Ahem font is absent from the evaluation context")
    lines = _hard_lines(text["content"])
    size = float(text["size"])
    return (
        max((len(line) for line in lines), default=0) * size,
        len(lines) * float(text["line_height"]),
    )


def _resolve_axis(intent: dict[str, Any], available: float, intrinsic: float) -> float:
    kind = intent["type"]
    if kind in ("auto", "intrinsic", "min_content", "max_content"):
        return intrinsic
    if kind == "fixed":
        return float(intent["value"])
    if kind == "fill":
        return available
    if kind == "percentage":
        return available * float(intent["value"]) / 100.0
    if kind == "fit_content":
        return min(intrinsic, float(intent["value"]))
    raise ProfileError(f"unsupported size intent: {kind}")


def evaluate_layout(
    document: dict[str, Any],
    width: float,
    height: float,
    *,
    theme: str | None = None,
    font_hashes: set[str] | None = None,
) -> dict[str, Rect]:
    entities = document["entities"]
    fonts = {PINNED_FONT_SHA256} if font_hashes is None else font_hashes
    boxes: dict[str, Rect] = {}

    def layout_entity(identifier: str, available: Rect, is_root: bool) -> None:
        entity = entities[identifier]
        resolved = _resolved_authored(entity, width, theme)
        intrinsic = _intrinsic_size(entity, fonts)
        width_intent = resolved["width"]
        height_intent = resolved["height"]
        entity_width = (
            available.width
            if is_root and width_intent["type"] in ("auto", "fill")
            else _resolve_axis(width_intent, available.width, intrinsic[0])
        )
        entity_height = (
            available.height
            if is_root and height_intent["type"] in ("auto", "fill")
            else _resolve_axis(height_intent, available.height, intrinsic[1])
        )
        rect = Rect(
            available.x if is_root else available.x + resolved["x"],
            available.y if is_root else available.y + resolved["y"],
            max(0.0, entity_width),
            max(0.0, entity_height),
        )
        boxes[identifier] = rect
        layout_children(identifier, rect, resolved)

    def layout_flow(identifier: str, bounds: Rect, resolved: dict[str, Any]) -> None:
        entity = entities[identifier]
        children = entity["children"]
        is_row = resolved["direction"] == "row"
        available_main = bounds.width if is_row else bounds.height
        gap = max(0.0, float(resolved["gap"]))
        gap_total = gap * max(0, len(children) - 1)
        fixed_main = 0.0
        fill_count = 0
        child_data: list[tuple[str, dict[str, Any], tuple[float, float]]] = []
        for child in children:
            item = entities[child]
            child_resolved = _resolved_authored(item, width, theme)
            intrinsic = _intrinsic_size(item, fonts)
            intent = child_resolved["width" if is_row else "height"]
            if intent["type"] == "fill":
                fill_count += 1
            else:
                fixed_main += _resolve_axis(
                    intent, available_main, intrinsic[0 if is_row else 1]
                )
            child_data.append((child, child_resolved, intrinsic))
        fill_main = (
            0.0
            if fill_count == 0
            else max(0.0, available_main - fixed_main - gap_total) / fill_count
        )

        cursor = 0.0
        for child, child_resolved, intrinsic in child_data:
            main_intent = child_resolved["width" if is_row else "height"]
            main = (
                fill_main
                if main_intent["type"] == "fill"
                else _resolve_axis(
                    main_intent, available_main, intrinsic[0 if is_row else 1]
                )
            )
            cross_intent = child_resolved["height" if is_row else "width"]
            available_cross = bounds.height if is_row else bounds.width
            align = entity["authored"]["layout"]["align"]
            if cross_intent["type"] == "fill" or (
                align == "stretch" and cross_intent["type"] == "auto"
            ):
                cross = available_cross
            else:
                cross = _resolve_axis(
                    cross_intent, available_cross, intrinsic[1 if is_row else 0]
                )
            if align in ("start", "stretch"):
                cross_offset = 0.0
            elif align == "center":
                cross_offset = max(0.0, (available_cross - cross) / 2.0)
            elif align == "end":
                cross_offset = max(0.0, available_cross - cross)
            else:
                raise ProfileError(f"unsupported alignment: {align}")
            child_rect = (
                Rect(bounds.x + cursor, bounds.y + cross_offset, main, cross)
                if is_row
                else Rect(bounds.x + cross_offset, bounds.y + cursor, cross, main)
            )
            boxes[child] = child_rect
            layout_children(child, child_rect, child_resolved)
            cursor += main + gap

    def layout_children(identifier: str, rect: Rect, resolved: dict[str, Any]) -> None:
        entity = entities[identifier]
        padding = entity["authored"]["layout"]["padding"]
        bounds = Rect(
            rect.x + float(padding["left"]),
            rect.y + float(padding["top"]),
            max(0.0, rect.width - float(padding["left"]) - float(padding["right"])),
            max(0.0, rect.height - float(padding["top"]) - float(padding["bottom"])),
        )
        family = entity["authored"]["layout"]["family"]
        if family in ("freeform", "constraint"):
            for child in entity["children"]:
                layout_entity(child, bounds, False)
        elif family in ("stack", "flex", "grid"):
            layout_flow(identifier, bounds, resolved)
        else:
            raise ProfileError(f"unsupported layout family: {family}")

    viewport = Rect(0.0, 0.0, max(0.0, width), max(0.0, height))
    for root in document["roots"]:
        layout_entity(root, viewport, True)
    return boxes


def _channel(value: float) -> int:
    return min(255, max(0, math.floor(value * 255.0 + 0.5)))


def _blend_pixel(
    rgba: bytearray, width: int, x: int, y: int, source: tuple[int, int, int, int]
) -> None:
    index = (y * width + x) * 4
    alpha = source[3]
    inverse = 255 - alpha
    for offset in range(3):
        rgba[index + offset] = (
            source[offset] * alpha + rgba[index + offset] * inverse + 127
        ) // 255
    rgba[index + 3] = 255


def _fill_rect(
    rgba: bytearray, width: int, height: int, rect: Rect, fill: dict[str, Any]
) -> None:
    x0 = max(0, min(width, math.floor(rect.x)))
    y0 = max(0, min(height, math.floor(rect.y)))
    x1 = max(0, min(width, math.ceil(rect.x + rect.width)))
    y1 = max(0, min(height, math.ceil(rect.y + rect.height)))
    source = tuple(
        _channel(float(fill[channel])) for channel in ("red", "green", "blue", "alpha")
    )
    for y in range(y0, y1):
        for x in range(x0, x1):
            _blend_pixel(rgba, width, x, y, source)  # type: ignore[arg-type]


def _draw_ahem_text(
    rgba: bytearray,
    width: int,
    height: int,
    rect: Rect,
    text: dict[str, Any],
) -> None:
    if text.get("font") != "Ahem" or text.get("font_sha256") != PINNED_FONT_SHA256:
        raise ProfileError("raster profile supports only pinned Ahem text")
    font_size = float(text["size"])
    line_height = float(text["line_height"])
    for line_index, line in enumerate(_hard_lines(text["content"])):
        line_rect = Rect(
            rect.x,
            rect.y + line_index * line_height,
            rect.width,
            min(line_height, max(0.0, rect.height - line_index * line_height)),
        )
        mask: dict[tuple[int, int], float] = {}
        pen_x = line_rect.x
        for character in line:
            if not character.isspace():
                left = max(line_rect.x, pen_x)
                right = min(line_rect.x + line_rect.width, pen_x + font_size)
                top = max(line_rect.y, line_rect.y)
                bottom = min(line_rect.y + line_rect.height, line_rect.y + font_size)
                for y in range(max(0, math.floor(top)), min(height, math.ceil(bottom))):
                    y_coverage = max(0.0, min(bottom, y + 1.0) - max(top, float(y)))
                    for x in range(
                        max(0, math.floor(left)), min(width, math.ceil(right))
                    ):
                        x_coverage = max(0.0, min(right, x + 1.0) - max(left, float(x)))
                        key = (x, y)
                        mask[key] = min(
                            1.0, mask.get(key, 0.0) + x_coverage * y_coverage
                        )
            pen_x += font_size
        for (x, y), coverage in mask.items():
            alpha = min(255, max(0, math.floor(coverage * 255.0)))
            _blend_pixel(rgba, width, x, y, (0, 0, 0, alpha))


def render_rgba(
    document: dict[str, Any], boxes: dict[str, Rect], width: int, height: int
) -> bytes:
    if width <= 0 or height <= 0 or width * height > 16_777_216:
        raise ProfileError("render target is outside profile-0 bounds")
    rgba = bytearray([255]) * (width * height * 4)
    for identifier in sorted(document["entities"]):
        entity = document["entities"][identifier]
        rect = boxes[identifier]
        kind = entity["kind"]
        kind_type = kind["type"]
        if kind_type == "unknown" or kind_type == "instance":
            continue
        if kind_type == "shape" and kind.get("data") == "path":
            continue
        fill = entity["authored"].get("fill")
        if fill is not None:
            if kind_type == "shape" and kind.get("data") == "ellipse":
                raise ProfileError(
                    "the independent v0 implementation does not claim ellipse raster support"
                )
            _fill_rect(rgba, width, height, rect, fill)
        text = entity["authored"].get("text")
        if text is not None:
            _draw_ahem_text(rgba, width, height, rect, text)
    return bytes(rgba)


def render_fidelity(document: dict[str, Any]) -> list[dict[str, Any]]:
    fidelity: list[dict[str, Any]] = []
    for namespace in sorted(document.get("extensions", {})):
        fidelity.append(
            {
                "entity": None,
                "pointer": f"/extensions/{namespace}",
                "status": {"class": "preserved_unrenderable", "namespace": namespace},
            }
        )
    for identifier in sorted(document["entities"]):
        entity = document["entities"][identifier]
        for namespace in sorted(entity.get("extensions", {})):
            fidelity.append(
                {
                    "entity": identifier,
                    "pointer": f"/entities/{identifier}/extensions/{namespace}",
                    "status": {
                        "class": "preserved_unrenderable",
                        "namespace": namespace,
                    },
                }
            )
        kind = entity["kind"]
        if kind["type"] == "unknown":
            namespace = kind["data"]["namespace"]
            fidelity.append(
                {
                    "entity": identifier,
                    "pointer": f"/entities/{identifier}/kind",
                    "status": {
                        "class": "preserved_unrenderable",
                        "namespace": namespace,
                    },
                }
            )
        elif kind["type"] == "instance":
            fidelity.append(
                {
                    "entity": identifier,
                    "pointer": f"/entities/{identifier}/kind",
                    "status": {
                        "class": "unsupported",
                        "reason": "profile 0 does not materialize component instances",
                    },
                }
            )
        elif kind["type"] == "shape" and kind.get("data") == "path":
            fidelity.append(
                {
                    "entity": identifier,
                    "pointer": f"/entities/{identifier}/kind",
                    "status": {
                        "class": "unsupported",
                        "reason": "profile 0 has no authored path-geometry field",
                    },
                }
            )
        if entity["authored"].get("text") is not None:
            fidelity.append(
                {
                    "entity": identifier,
                    "pointer": f"/entities/{identifier}/authored/text",
                    "status": {"class": "lossless"},
                }
            )
    return fidelity


def _png_chunk(kind: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + kind
        + data
        + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    )


def encode_png(width: int, height: int, rgba: bytes) -> bytes:
    if len(rgba) != width * height * 4:
        raise ProfileError("RGBA byte length does not match dimensions")
    rows = b"".join(
        b"\x00" + rgba[y * width * 4 : (y + 1) * width * 4] for y in range(height)
    )
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _png_chunk(b"IHDR", header)
        + _png_chunk(b"IDAT", zlib.compress(rows, 9))
        + _png_chunk(b"IEND", b"")
    )


def _paeth(a: int, b: int, c: int) -> int:
    prediction = a + b - c
    pa = abs(prediction - a)
    pb = abs(prediction - b)
    pc = abs(prediction - c)
    return a if pa <= pb and pa <= pc else b if pb <= pc else c


def decode_png_rgba(raw: bytes) -> tuple[int, int, bytes]:
    if not raw.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ProfileError("reference raster is not a PNG")
    offset = 8
    compressed = bytearray()
    width = height = 0
    while offset < len(raw):
        if offset + 12 > len(raw):
            raise ProfileError("truncated PNG chunk")
        length = struct.unpack(">I", raw[offset : offset + 4])[0]
        kind = raw[offset + 4 : offset + 8]
        data = raw[offset + 8 : offset + 8 + length]
        expected_crc = struct.unpack(
            ">I", raw[offset + 8 + length : offset + 12 + length]
        )[0]
        if zlib.crc32(kind + data) & 0xFFFFFFFF != expected_crc:
            raise ProfileError(f"PNG chunk {kind!r} has an invalid CRC")
        offset += length + 12
        if kind == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = (
                struct.unpack(">IIBBBBB", data)
            )
            if (depth, color, compression, filtering, interlace) != (8, 6, 0, 0, 0):
                raise ProfileError(
                    "only non-interlaced RGBA8 PNG references are supported"
                )
        elif kind == b"IDAT":
            compressed.extend(data)
        elif kind == b"IEND":
            break
    scanlines = zlib.decompress(bytes(compressed))
    stride = width * 4
    if len(scanlines) != height * (stride + 1):
        raise ProfileError("PNG decompressed length is inconsistent")
    output = bytearray()
    previous = bytearray(stride)
    cursor = 0
    for _ in range(height):
        filter_kind = scanlines[cursor]
        cursor += 1
        row = bytearray(scanlines[cursor : cursor + stride])
        cursor += stride
        for index in range(stride):
            left = row[index - 4] if index >= 4 else 0
            above = previous[index]
            upper_left = previous[index - 4] if index >= 4 else 0
            if filter_kind == 1:
                row[index] = (row[index] + left) & 0xFF
            elif filter_kind == 2:
                row[index] = (row[index] + above) & 0xFF
            elif filter_kind == 3:
                row[index] = (row[index] + (left + above) // 2) & 0xFF
            elif filter_kind == 4:
                row[index] = (row[index] + _paeth(left, above, upper_left)) & 0xFF
            elif filter_kind != 0:
                raise ProfileError(f"unknown PNG filter {filter_kind}")
        output.extend(row)
        previous = row
    return width, height, bytes(output)


def _source_identity() -> dict[str, Any]:
    def command(*arguments: str) -> str | None:
        try:
            result = subprocess.run(
                arguments, check=True, capture_output=True, text=True
            )
        except (OSError, subprocess.CalledProcessError):
            return None
        return result.stdout.strip()

    dirty = command("git", "status", "--porcelain")
    return {
        "revision": command("git", "rev-parse", "HEAD"),
        "dirty": None if dirty is None else bool(dirty),
        "python": platform.python_version(),
        "os": platform.system().lower(),
        "architecture": platform.machine(),
    }


def _first_pixel_difference(
    expected: bytes, observed: bytes, width: int
) -> dict[str, Any] | None:
    for index, (left, right) in enumerate(zip(expected, observed)):
        if left != right:
            pixel = index // 4
            return {
                "x": pixel % width,
                "y": pixel // width,
                "channel": ("red", "green", "blue", "alpha")[index % 4],
                "expected": left,
                "observed": right,
            }
    if len(expected) != len(observed):
        return {"byte_length": [len(expected), len(observed)]}
    return None


def _compare_boxes(
    expected: dict[str, dict[str, Any]], observed: dict[str, dict[str, Any]]
) -> tuple[bool, float]:
    keys_exact = set(observed) == set(expected)
    maximum_delta = 0.0
    for identifier in set(observed) & set(expected):
        for field in ("x", "y", "width", "height"):
            maximum_delta = max(
                maximum_delta,
                abs(
                    float(observed[identifier][field])
                    - float(expected[identifier][field])
                ),
            )
    return keys_exact and maximum_delta == 0.0, maximum_delta


def verify(args: argparse.Namespace) -> int:
    input_path = Path(args.input)
    output_path = Path(args.output)
    artifact_directory = Path(args.artifact_dir)
    issues: list[dict[str, Any]] = []
    cases: list[dict[str, Any]] = []
    negative_trials: list[dict[str, Any]] = []
    try:
        source = input_path.read_bytes()
        document = parse_document(source)
        canonical = canonical_text(document)
        canonical_exact = canonical == source

        unknown_before = [
            entity["kind"]["data"]["payload"]
            for entity in document["entities"].values()
            if entity["kind"]["type"] == "unknown"
        ]
        edited = json.loads(canonical)
        first_neighbour = next(
            entity
            for entity in edited["entities"].values()
            if entity["kind"]["type"] != "unknown"
        )
        first_neighbour["name"] += " independently edited"
        reparsed = parse_document(canonical_text(edited))
        unknown_after = [
            entity["kind"]["data"]["payload"]
            for entity in reparsed["entities"].values()
            if entity["kind"]["type"] == "unknown"
        ]
        opaque_preserved = unknown_before == unknown_after and bool(unknown_before)
        try:
            parse_document(b'{"schema_version":1,"schema_version":1}')
            duplicate_rejected = False
        except ProfileError:
            duplicate_rejected = True
        negative_trials.append(
            {
                "name": "duplicate-key",
                "passed": duplicate_rejected,
                "expected": "typed parse failure",
            }
        )

        artifact_directory.mkdir(parents=True, exist_ok=True)
        (artifact_directory / "canonical.nuif.json").write_bytes(canonical)
        independent_fidelity = render_fidelity(document)
        for case_argument in args.case:
            dimensions, reference_name = case_argument.split("=", 1)
            width_text, height_text = dimensions.lower().split("x", 1)
            width = int(width_text)
            height = int(height_text)
            reference = Path(reference_name)
            expected_layout = json.loads(
                (reference / "expected.layout.json").read_text(encoding="utf-8")
            )
            expected_scene = json.loads(
                (reference / "expected.scene.json").read_text(encoding="utf-8")
            )
            boxes = evaluate_layout(document, float(width), float(height))
            observed_boxes = {
                identifier: rect.as_json() for identifier, rect in sorted(boxes.items())
            }
            expected_boxes = expected_layout["boxes"]
            layout_exact, maximum_layout_delta = _compare_boxes(
                expected_boxes, observed_boxes
            )

            observed_rgba = render_rgba(document, boxes, width, height)
            png_width, png_height, expected_rgba = decode_png_rgba(
                (reference / "expected.png").read_bytes()
            )
            dimensions_exact = (png_width, png_height) == (width, height)
            first_difference = _first_pixel_difference(
                expected_rgba, observed_rgba, width
            )
            raster_exact = dimensions_exact and first_difference is None
            fidelity_exact = independent_fidelity == expected_scene["fidelity"]

            if not negative_trials or len(negative_trials) == 1:
                corrupted_boxes = json.loads(json.dumps(expected_boxes))
                corrupted_identifier = min(corrupted_boxes)
                corrupted_boxes[corrupted_identifier]["x"] = (
                    float(corrupted_boxes[corrupted_identifier]["x"]) + 1.0
                )
                corrupted_layout_rejected = not _compare_boxes(
                    corrupted_boxes, observed_boxes
                )[0]
                corrupted_rgba = bytearray(expected_rgba)
                corrupted_rgba[0] ^= 1
                corrupted_raster_rejected = (
                    _first_pixel_difference(bytes(corrupted_rgba), observed_rgba, width)
                    is not None
                )
                negative_trials.extend(
                    [
                        {
                            "name": "corrupted-reference-layout",
                            "passed": corrupted_layout_rejected,
                            "expected": "exact comparison failure",
                        },
                        {
                            "name": "corrupted-reference-raster",
                            "passed": corrupted_raster_rejected,
                            "expected": "exact comparison failure",
                        },
                    ]
                )

            case_directory = artifact_directory / dimensions
            case_directory.mkdir(parents=True, exist_ok=True)
            (case_directory / "layout.json").write_text(
                json.dumps({"boxes": observed_boxes}, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            (case_directory / "render.png").write_bytes(
                encode_png(width, height, observed_rgba)
            )
            cases.append(
                {
                    "viewport": [width, height],
                    "checks": {
                        "layout_exact": layout_exact,
                        "raster_exact": raster_exact,
                        "fidelity_exact": fidelity_exact,
                    },
                    "layout": {
                        "box_count": len(observed_boxes),
                        "maximum_delta": maximum_layout_delta,
                    },
                    "raster": {
                        "rgba_sha256": hashlib.sha256(observed_rgba).hexdigest(),
                        "reference_rgba_sha256": hashlib.sha256(
                            expected_rgba
                        ).hexdigest(),
                        "first_difference": first_difference,
                    },
                    "artifacts": [
                        str(case_directory / "layout.json"),
                        str(case_directory / "render.png"),
                    ],
                }
            )
        passed = (
            canonical_exact
            and opaque_preserved
            and all(all(case["checks"].values()) for case in cases)
            and all(trial["passed"] for trial in negative_trials)
        )
        report = {
            "schema_version": 1,
            "experiment": "nuif:experiment:independent-profile-zero",
            "status": "passed" if passed else "failed",
            "implementation": {
                "name": "nuif-python-profile-zero",
                "language": "Python standard library",
                "reference_package_dependencies": [],
            },
            "source": _source_identity(),
            "checks": {
                "canonical_text_exact": canonical_exact,
                "opaque_payload_survives_neighbour_edit": opaque_preserved,
            },
            "canonical_sha256": hashlib.sha256(canonical).hexdigest(),
            "cases": cases,
            "negative_trials": negative_trials,
            "issues": issues,
            "artifacts": [str(artifact_directory / "canonical.nuif.json")],
        }
    except (OSError, ValueError, KeyError, ProfileError) as error:
        report = {
            "schema_version": 1,
            "experiment": "nuif:experiment:independent-profile-zero",
            "status": "failed",
            "implementation": {
                "name": "nuif-python-profile-zero",
                "language": "Python standard library",
                "reference_package_dependencies": [],
            },
            "source": _source_identity(),
            "checks": {},
            "cases": cases,
            "negative_trials": negative_trials,
            "issues": [{"code": "INDEPENDENT_PROFILE_FAILED", "message": str(error)}],
            "artifacts": [],
        }
        passed = False

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(
        f"independent profile 0: {len(cases)} contexts, status {report['status']!r}",
        file=sys.stderr,
    )
    return 0 if passed else 1


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    verify_parser = subparsers.add_parser("verify")
    verify_parser.add_argument("--input", required=True)
    verify_parser.add_argument("--case", action="append", required=True)
    verify_parser.add_argument("--output", required=True)
    verify_parser.add_argument("--artifact-dir", required=True)
    arguments = parser.parse_args()
    if arguments.command == "verify":
        return verify(arguments)
    raise AssertionError("argparse accepted an unknown command")


if __name__ == "__main__":
    raise SystemExit(main())
