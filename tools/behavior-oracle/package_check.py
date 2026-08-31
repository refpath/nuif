#!/usr/bin/env python3
"""Independent container checks for the behavior package resource fixture."""

from __future__ import annotations

import argparse
import hashlib
import json
import stat
import zipfile
from pathlib import Path


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("package", type=Path)
    parser.add_argument("expected", type=Path)
    parser.add_argument("static_report", type=Path)
    parser.add_argument("report", type=Path)
    arguments = parser.parse_args()

    package_bytes = arguments.package.read_bytes()
    expected = json.loads(arguments.expected.read_text(encoding="utf-8"))
    static_report = json.loads(arguments.static_report.read_text(encoding="utf-8"))
    checks: dict[str, bool] = {
        "static_profile_passed": static_report.get("status") == "passed",
        "package_size_exact": len(package_bytes) == expected["package_bytes"],
        "package_sha256_exact": sha256(package_bytes) == expected["package_sha256"],
    }

    with zipfile.ZipFile(arguments.package, "r") as archive:
        entries = archive.infolist()
        names = [entry.filename for entry in entries]
        checks.update(
            {
                "archive_comment_empty": archive.comment == b"",
                "member_count_exact": len(entries) == expected["expected_members"],
                "mimetype_first": bool(names) and names[0] == "mimetype",
                "remaining_members_byte_sorted": names[1:]
                == sorted(names[1:], key=lambda name: name.encode("ascii")),
                "behavior_member_present": expected["behavior_path"] in names,
            }
        )
        metadata_ok = True
        crc_reads_ok = True
        for entry in entries:
            mode = entry.external_attr >> 16
            metadata_ok = metadata_ok and all(
                (
                    entry.compress_type == zipfile.ZIP_STORED,
                    entry.flag_bits == 0,
                    entry.date_time == (1980, 1, 1, 0, 0, 0),
                    entry.create_system == 3,
                    stat.S_ISREG(mode),
                    stat.S_IMODE(mode) == 0o644,
                    entry.extra == b"",
                    entry.comment == b"",
                )
            )
            try:
                archive.read(entry)
            except (zipfile.BadZipFile, RuntimeError):
                crc_reads_ok = False
        checks["deterministic_member_metadata"] = metadata_ok
        checks["all_member_crc_reads_pass"] = crc_reads_ok
        checks["mimetype_exact"] = (
            archive.read("mimetype").decode("ascii") == expected["mime_type"]
        )
        behavior = archive.read(expected["behavior_path"])
        checks["behavior_size_exact"] = len(behavior) == expected["behavior_bytes"]
        checks["behavior_sha256_exact"] = sha256(behavior) == expected["behavior_sha256"]
        checks["behavior_digest_path_exact"] = expected["behavior_path"].endswith(
            expected["behavior_digest"].removeprefix("sha256:")
        )

    passed = all(checks.values())
    report = {
        "schema_version": 1,
        "experiment": "nuif:experiment:behavior-package-resource",
        "status": "passed" if passed else "failed",
        "profile": expected["profile"],
        "oracle": {
            "name": "python-standard-library-zip-reader",
            "python": __import__("platform").python_version(),
            "dependencies": ["python-standard-library"],
        },
        "checks": checks,
        "boundaries": [
            "the independent oracle validates ZIP structure and exact behavior bytes but does not decode canonical CBOR",
            "the Rust profile independently validates canonical CBOR and document references",
            "neither check grants behavior execution authority",
        ],
    }
    arguments.report.parent.mkdir(parents=True, exist_ok=True)
    arguments.report.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(
        f"behavior package oracle: {len(checks)} checks, status "
        f"{'passed' if passed else 'failed'}"
    )
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
