#!/usr/bin/env python3
"""Independent stored-ZIP writer for the bounded nuif-package-0 profile.

The oracle deliberately treats document.cbor and manifest.cbor as opaque
canonical payloads. It independently validates and rewrites the deterministic
ZIP container, so archive headers, member ordering, offsets and CRCs are not
proved by the Rust ZIP implementation itself.
"""

from __future__ import annotations

import argparse
import binascii
import hashlib
import json
import struct
from pathlib import Path


MAX_PACKAGE_BYTES = 80 * 1024 * 1024
MAX_MEMBERS = 8_195
MAX_PATH_BYTES = 96
EOCD_SIGNATURE = 0x06054B50
LOCAL_SIGNATURE = 0x04034B50
CENTRAL_SIGNATURE = 0x02014B50
VERSION_NEEDED = 10
VERSION_MADE_BY = 0x030A
DOS_TIME = 0
DOS_DATE = 33
EXTERNAL_ATTRIBUTES = 0x81A40000


class PackageOracleError(ValueError):
    """Malformed input outside the deterministic package profile."""


def _u16(data: bytes, offset: int) -> int:
    try:
        return struct.unpack_from("<H", data, offset)[0]
    except struct.error as error:
        raise PackageOracleError("truncated uint16") from error


def _u32(data: bytes, offset: int) -> int:
    try:
        return struct.unpack_from("<I", data, offset)[0]
    except struct.error as error:
        raise PackageOracleError("truncated uint32") from error


def _path_allowed(name: str) -> bool:
    if not name or len(name.encode("ascii", "ignore")) != len(name):
        return False
    if len(name.encode()) > MAX_PATH_BYTES or name.startswith("/") or name.endswith("/"):
        return False
    if name in {"mimetype", "manifest.cbor", "document.cbor"}:
        return True
    prefix = "blobs/sha256/"
    digest = name[len(prefix) :] if name.startswith(prefix) else ""
    return len(digest) == 64 and all(character in "0123456789abcdef" for character in digest)


def _member_sequence(members: list[tuple[str, bytes]]) -> None:
    if not members or members[0][0] != "mimetype":
        raise PackageOracleError("mimetype must be the first member")
    names = [name for name, _ in members]
    if len(names) > MAX_MEMBERS or len(set(names)) != len(names):
        raise PackageOracleError("member count or uniqueness is invalid")
    if any(not _path_allowed(name) for name in names):
        raise PackageOracleError("member path is outside the profile")
    if names[1:] != sorted(names[1:], key=lambda value: value.encode()):
        raise PackageOracleError("members after mimetype are not bytewise sorted")


def read_archive(data: bytes) -> list[tuple[str, bytes]]:
    """Parse the deterministic stored ZIP without using zipfile."""
    if len(data) > MAX_PACKAGE_BYTES or len(data) < 22:
        raise PackageOracleError("package size is outside the profile")
    end_offset = len(data) - 22
    if _u32(data, end_offset) != EOCD_SIGNATURE:
        raise PackageOracleError("end record must be the final 22 bytes")
    if _u16(data, end_offset + 4) or _u16(data, end_offset + 6):
        raise PackageOracleError("split archives are forbidden")
    disk_count = _u16(data, end_offset + 8)
    count = _u16(data, end_offset + 10)
    central_size = _u32(data, end_offset + 12)
    central_offset = _u32(data, end_offset + 16)
    if disk_count != count or _u16(data, end_offset + 20):
        raise PackageOracleError("archive comments or entry counts are invalid")
    if central_offset + central_size != end_offset or count > MAX_MEMBERS:
        raise PackageOracleError("central directory bounds are invalid")

    entries: list[tuple[str, int, int, int]] = []
    cursor = central_offset
    for _ in range(count):
        if _u32(data, cursor) != CENTRAL_SIGNATURE:
            raise PackageOracleError("central directory signature is invalid")
        if _u16(data, cursor + 4) != VERSION_MADE_BY or _u16(data, cursor + 6) != VERSION_NEEDED:
            raise PackageOracleError("central version is not the deterministic profile")
        if any(_u16(data, cursor + offset) != 0 for offset in (8, 10, 12)):
            raise PackageOracleError("central flags, method or time are not stored-profile values")
        if _u16(data, cursor + 14) != DOS_DATE:
            raise PackageOracleError("central date is not deterministic")
        crc = _u32(data, cursor + 16)
        compressed = _u32(data, cursor + 20)
        size = _u32(data, cursor + 24)
        name_length = _u16(data, cursor + 28)
        extra_length = _u16(data, cursor + 30)
        comment_length = _u16(data, cursor + 32)
        if compressed != size or extra_length or comment_length:
            raise PackageOracleError("central compression or metadata is not deterministic")
        if _u16(data, cursor + 34) or _u16(data, cursor + 36):
            raise PackageOracleError("central disk or internal attributes are invalid")
        if _u32(data, cursor + 38) != EXTERNAL_ATTRIBUTES:
            raise PackageOracleError("central permissions are not deterministic")
        name_start = cursor + 46
        name_end = name_start + name_length
        try:
            name = data[name_start:name_end].decode("ascii")
        except UnicodeDecodeError as error:
            raise PackageOracleError("member name is not ASCII") from error
        entries.append((name, crc, size, _u32(data, cursor + 42)))
        cursor = name_end
    if cursor != end_offset:
        raise PackageOracleError("central directory has trailing bytes")
    _member_sequence([(name, b"") for name, _, _, _ in entries])

    members: list[tuple[str, bytes]] = []
    expected_offset = 0
    for name, crc, size, offset in entries:
        if offset != expected_offset or offset >= central_offset:
            raise PackageOracleError("local members are not contiguous")
        if _u32(data, offset) != LOCAL_SIGNATURE:
            raise PackageOracleError("local member signature is invalid")
        if _u16(data, offset + 4) != VERSION_NEEDED:
            raise PackageOracleError("local version is not deterministic")
        if any(_u16(data, offset + value) != 0 for value in (6, 8, 10)):
            raise PackageOracleError("local flags, method or time are not stored-profile values")
        if _u16(data, offset + 12) != DOS_DATE:
            raise PackageOracleError("local date is not deterministic")
        if _u32(data, offset + 14) != crc or _u32(data, offset + 18) != size or _u32(data, offset + 22) != size:
            raise PackageOracleError("local and central sizes or CRC differ")
        name_length = _u16(data, offset + 26)
        extra_length = _u16(data, offset + 28)
        if extra_length:
            raise PackageOracleError("local extra fields are forbidden")
        name_start = offset + 30
        name_end = name_start + name_length
        try:
            local_name = data[name_start:name_end].decode("ascii")
        except UnicodeDecodeError as error:
            raise PackageOracleError("local member name is not ASCII") from error
        if local_name != name:
            raise PackageOracleError("local and central names differ")
        value_end = name_end + size
        value = data[name_end:value_end]
        if len(value) != size or (binascii.crc32(value) & 0xFFFFFFFF) != crc:
            raise PackageOracleError("member CRC or bounds are invalid")
        members.append((name, value))
        expected_offset = value_end
    if expected_offset != central_offset:
        raise PackageOracleError("bytes exist between local members and central directory")
    return members


def encode_archive(members: list[tuple[str, bytes]]) -> bytes:
    """Write the deterministic stored ZIP byte-for-byte."""
    _member_sequence(members)
    output = bytearray()
    entries: list[tuple[str, int, int, int]] = []
    for name, value in members:
        name_bytes = name.encode("ascii")
        size = len(value)
        crc = binascii.crc32(value) & 0xFFFFFFFF
        offset = len(output)
        output.extend(struct.pack(
            "<IHHHHHIIIHH",
            LOCAL_SIGNATURE,
            VERSION_NEEDED,
            0,
            0,
            DOS_TIME,
            DOS_DATE,
            crc,
            size,
            size,
            len(name_bytes),
            0,
        ))
        output.extend(name_bytes)
        output.extend(value)
        entries.append((name, crc, size, offset))
    central_offset = len(output)
    for name, crc, size, offset in entries:
        name_bytes = name.encode("ascii")
        output.extend(struct.pack(
            "<IHHHHHHIIIHHHHHII",
            CENTRAL_SIGNATURE,
            VERSION_MADE_BY,
            VERSION_NEEDED,
            0,
            0,
            DOS_TIME,
            DOS_DATE,
            crc,
            size,
            size,
            len(name_bytes),
            0,
            0,
            0,
            0,
            EXTERNAL_ATTRIBUTES,
            offset,
        ))
        output.extend(name_bytes)
    central_size = len(output) - central_offset
    output.extend(struct.pack(
        "<IHHHHIIH",
        EOCD_SIGNATURE,
        0,
        0,
        len(entries),
        len(entries),
        central_size,
        central_offset,
        0,
    ))
    if len(output) > MAX_PACKAGE_BYTES:
        raise PackageOracleError("encoded package exceeds the profile limit")
    return bytes(output)


def _digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def run(input_path: Path, output_path: Path, report_path: Path) -> int:
    source = input_path.read_bytes()
    members = read_archive(source)
    encoded = encode_archive(members)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_bytes(encoded)
    report = {
        "schema_version": 1,
        "profile": "nuif-package-0-zip-oracle",
        "oracle": "python-stdlib-stored-zip-0",
        "status": "passed" if source == encoded else "failed",
        "opaque_payloads": ["document.cbor", "manifest.cbor"],
        "members": [name for name, _ in members],
        "input": {"bytes": len(source), "sha256": _digest(source)},
        "output": {"bytes": len(encoded), "sha256": _digest(encoded)},
        "exact_bytes": source == encoded,
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return 0 if source == encoded else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        return run(arguments.input, arguments.output, arguments.report)
    except (OSError, PackageOracleError) as error:
        print(f"package oracle: {error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
