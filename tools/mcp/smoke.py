#!/usr/bin/env python3
"""Independent subprocess oracle for the nuif-mcp-tools-0 stdio profile."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import selectors
import statistics
import subprocess
import tempfile
import time
from typing import Any


PROTOCOL_VERSION = "2026-07-28"
API_PROFILE = "nuif-mcp-tools-0"
MAX_MESSAGE_BYTES = 4 * 1024 * 1024
EXPECTED_TOOLS = [
    "nuif_apply_patch",
    "nuif_canonicalize",
    "nuif_inspect",
    "nuif_snapshot_package",
    "nuif_validate",
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    index = min(len(ordered) - 1, int((len(ordered) - 1) * fraction))
    return ordered[index]


class Client:
    def __init__(self, executable: Path) -> None:
        self.process = subprocess.Popen(
            [str(executable)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        require(self.process.stdin is not None, "server stdin was not piped")
        require(self.process.stdout is not None, "server stdout was not piped")
        require(self.process.stderr is not None, "server stderr was not piped")
        self.request_id = 0
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)

    @staticmethod
    def metadata() -> dict[str, Any]:
        return {
            "io.modelcontextprotocol/protocolVersion": PROTOCOL_VERSION,
            "io.modelcontextprotocol/clientInfo": {
                "name": "nuif-mcp-wire-oracle",
                "version": "0.0.1",
            },
            "io.modelcontextprotocol/clientCapabilities": {},
        }

    def request(
        self,
        method: str,
        params: dict[str, Any] | None = None,
        *,
        include_metadata: bool = True,
    ) -> tuple[dict[str, Any], float]:
        self.request_id += 1
        body = dict(params or {})
        if include_metadata:
            body["_meta"] = self.metadata()
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": body,
        }
        wire = json.dumps(request, separators=(",", ":")).encode("utf-8") + b"\n"
        require(len(wire) <= MAX_MESSAGE_BYTES, "test request exceeds profile frame limit")
        started = time.perf_counter()
        self.process.stdin.write(wire)
        self.process.stdin.flush()
        events = self.selector.select(timeout=10.0)
        require(bool(events), f"server timed out responding to {method}")
        line = self.process.stdout.readline()
        elapsed_ms = (time.perf_counter() - started) * 1000.0
        require(bool(line), f"server closed stdout while handling {method}")
        try:
            response = json.loads(line)
        except json.JSONDecodeError as error:
            raise RuntimeError(f"stdout was not one MCP JSON message: {error}") from error
        require(response.get("jsonrpc") == "2.0", "response is not JSON-RPC 2.0")
        require(response.get("id") == self.request_id, "response ID changed")
        return response, elapsed_ms

    def call(self, name: str, arguments: dict[str, Any]) -> tuple[dict[str, Any], float]:
        response, elapsed_ms = self.request(
            "tools/call", {"name": name, "arguments": arguments}
        )
        require("error" not in response, f"{name} returned a protocol error: {response}")
        result = response.get("result")
        require(isinstance(result, dict), f"{name} returned no result object")
        return result, elapsed_ms

    def close(self) -> str:
        self.process.stdin.close()
        return_code = self.process.wait(timeout=10.0)
        remaining_stdout = self.process.stdout.read()
        stderr = self.process.stderr.read().decode("utf-8", errors="replace")
        require(return_code == 0, f"server exited {return_code}: {stderr}")
        require(remaining_stdout == b"", "server wrote an unsolicited stdout message")
        return stderr


def structured(result: dict[str, Any], name: str) -> dict[str, Any]:
    require(result.get("isError") is not True, f"{name} returned a tool error: {result}")
    value = result.get("structuredContent")
    require(isinstance(value, dict), f"{name} did not return structured content")
    content = result.get("content")
    require(isinstance(content, list) and content, f"{name} omitted text fallback content")
    return value


def native_canonicalize(cli: Path, fixture: Path) -> bytes:
    result = subprocess.run(
        [str(cli), "canonicalize", str(fixture), "-"],
        check=True,
        capture_output=True,
    )
    require(result.stderr == b"", "native canonicalize wrote unexpected stderr")
    return result.stdout


def native_apply(cli: Path, fixture: Path, patch: str) -> bytes:
    with tempfile.TemporaryDirectory(prefix="nuif-mcp-") as directory:
        patch_path = Path(directory) / "patch.json"
        output_path = Path(directory) / "output.nuif.json"
        patch_path.write_text(patch, encoding="utf-8")
        result = subprocess.run(
            [str(cli), "patch", str(fixture), str(patch_path), str(output_path)],
            check=True,
            capture_output=True,
        )
        require(result.stdout == b"" and result.stderr == b"", "native patch was noisy")
        return output_path.read_bytes()


def native_snapshot(cli: Path, package: Path) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="nuif-mcp-snapshot-") as directory:
        result = subprocess.run(
            [str(cli), "snapshot", str(package), directory, "640", "96"],
            check=True,
            capture_output=True,
        )
        require(result.stderr == b"", "native snapshot wrote unexpected stderr")
        summary = json.loads(result.stdout)
        require(summary.get("status") == "passed", "native snapshot summary failed")
        return json.loads((Path(directory) / "expected.report.json").read_text())


def exercise(
    executable: Path, cli: Path, fixture: Path, variable_font_package: Path
) -> dict[str, Any]:
    document = fixture.read_text(encoding="utf-8")
    client = Client(executable)
    latencies: dict[str, float] = {}

    discover, latencies["discover"] = client.request("server/discover")
    require("error" not in discover, f"server discovery failed: {discover}")
    discovery = discover.get("result")
    require(isinstance(discovery, dict), "server discovery returned no result")
    versions = discovery.get("supportedVersions")
    require(versions == [PROTOCOL_VERSION], f"unexpected protocol versions: {versions}")

    listing, latencies["tools_list"] = client.request("tools/list")
    require("error" not in listing, f"tool listing failed: {listing}")
    tools = listing.get("result", {}).get("tools")
    require(isinstance(tools, list), "tools/list returned no tools")
    require([tool.get("name") for tool in tools] == EXPECTED_TOOLS, "tool set drifted")
    for tool in tools:
        require(isinstance(tool.get("inputSchema"), dict), "tool input schema missing")
        require(isinstance(tool.get("outputSchema"), dict), "tool output schema missing")
        annotations = tool.get("annotations")
        require(
            annotations
            == {
                "title": annotations.get("title"),
                "readOnlyHint": True,
                "destructiveHint": False,
                "idempotentHint": True,
                "openWorldHint": False,
            },
            f"unsafe or incomplete annotations for {tool.get('name')}",
        )

    validation_result, latencies["validate"] = client.call(
        "nuif_validate", {"document": document}
    )
    validation = structured(validation_result, "nuif_validate")
    require(validation.get("status") == "passed", "fixture validation failed")
    before_hash = validation.get("canonical_hash")
    require(isinstance(before_hash, str), "validation omitted canonical hash")

    inspect_result, latencies["inspect"] = client.call(
        "nuif_inspect", {"document": document}
    )
    inspection = structured(inspect_result, "nuif_inspect")
    roots = inspection.get("roots")
    require(isinstance(roots, list) and roots, "fixture has no root to edit")
    require(inspection.get("canonical_hash") == before_hash, "inspect hash diverged")

    canonical_result, latencies["canonicalize"] = client.call(
        "nuif_canonicalize", {"document": document}
    )
    canonical = structured(canonical_result, "nuif_canonicalize")
    canonical_bytes = canonical.get("document", "").encode("utf-8")
    require(canonical_bytes == native_canonicalize(cli, fixture), "MCP and CLI canonical bytes differ")
    require(canonical.get("canonical_hash") == before_hash, "canonical hash changed")

    patch = json.dumps(
        {
            "base_revision": before_hash,
            "transactions": [
                {
                    "id": 9101,
                    "operations": [
                        {
                            "op": "rename",
                            "entity": roots[0],
                            "name": "MCP conformance root",
                        }
                    ],
                }
            ],
        },
        separators=(",", ":"),
    )
    apply_result, latencies["apply_patch"] = client.call(
        "nuif_apply_patch", {"document": document, "patch": patch}
    )
    applied = structured(apply_result, "nuif_apply_patch")
    output = applied.get("document", "").encode("utf-8")
    require(output == native_apply(cli, fixture, patch), "MCP and CLI patch bytes differ")
    require(applied.get("canonical_hash") != before_hash, "patch did not change hash")
    require(applied.get("transactions") == 1 and applied.get("operations") == 1, "patch usage changed")

    variable_bytes = variable_font_package.read_bytes()
    variable_base64 = base64.b64encode(variable_bytes).decode("ascii")
    variable_arguments = {
        "package_base64": variable_base64,
        "capabilities": [],
        "width": 640,
        "height": 96,
    }
    variable_missing_result, latencies["variable_font_missing"] = client.call(
        "nuif_snapshot_package", variable_arguments
    )
    require(variable_missing_result.get("isError") is True, "unauthorized variable font snapshot passed")
    variable_missing_text = variable_missing_result.get("content", [{}])[0].get("text", "")
    require(
        variable_missing_text.startswith("NUIF_PACKAGE_CAPABILITIES_UNAVAILABLE:"),
        "variable font capability error class changed",
    )
    variable_arguments["capabilities"] = [
        "nuif-opentype-variable-truetype-single-0"
    ]
    variable_result, latencies["variable_font_snapshot"] = client.call(
        "nuif_snapshot_package", variable_arguments
    )
    variable_observed = structured(variable_result, "nuif_snapshot_package")
    variable_expected = native_snapshot(cli, variable_font_package)
    require(variable_observed == variable_expected, "MCP and CLI variable font snapshots differ")
    variable_run = next(
        command["run"]
        for command in variable_observed["scene"]["commands"]
        if command.get("command") == "text"
    )
    require(
        variable_run["font"]["sha256"]
        == "0afd77effc877ff84fa7995a58c396c124514855f8084056846b54b8cb76f3ce"
        and len(variable_run["variation_coordinates"]) == 2,
        "MCP variable font evidence is incomplete",
    )

    stale_result, latencies["stale_patch"] = client.call(
        "nuif_apply_patch", {"document": output.decode("utf-8"), "patch": patch}
    )
    require(stale_result.get("isError") is True, "stale patch was accepted")
    error_text = stale_result.get("content", [{}])[0].get("text", "")
    require(error_text.startswith("NUIF_PATCH_APPLY_FAILED:"), "stale error class changed")

    malformed_result, latencies["malformed_document"] = client.call(
        "nuif_validate", {"document": "{"}
    )
    require(malformed_result.get("isError") is True, "malformed document was accepted")
    malformed_text = malformed_result.get("content", [{}])[0].get("text", "")
    require(malformed_text.startswith("NUIF_DOCUMENT_DECODE_FAILED:"), "decode error class changed")

    missing_meta, latencies["missing_metadata"] = client.request(
        "tools/list", include_metadata=False
    )
    require(missing_meta.get("error", {}).get("code") == -32602, "missing metadata was not rejected")

    after_rejection, latencies["after_rejection"] = client.request("tools/list")
    require("error" not in after_rejection, "connection did not survive request rejection")

    benchmark_ms: list[float] = []
    for _ in range(25):
        result, elapsed_ms = client.call("nuif_validate", {"document": document})
        require(structured(result, "nuif_validate").get("canonical_hash") == before_hash, "repeated validation drifted")
        benchmark_ms.append(elapsed_ms)

    stderr = client.close()
    require(stderr == "", f"healthy server wrote unexpected stderr: {stderr}")
    require(percentile(benchmark_ms, 0.95) < 2000.0, "wire validation exceeded catastrophic budget")

    return {
        "before_hash": before_hash,
        "after_hash": applied["canonical_hash"],
        "fixture_bytes": len(document.encode("utf-8")),
        "output_bytes": len(output),
        "output_sha256": hashlib.sha256(output).hexdigest(),
        "variable_font": {
            "package_bytes": len(variable_bytes),
            "canonical_hash": variable_observed["canonical_hash"],
            "raster_sha256": variable_observed["raster"]["rgba_sha256"],
            "coordinates": variable_run["variation_coordinates"],
            "matches_cli": True,
            "unauthorized_rejected": True,
        },
        "latency_ms": {name: round(value, 3) for name, value in latencies.items()},
        "validate_benchmark": {
            "samples": len(benchmark_ms),
            "median_ms": round(statistics.median(benchmark_ms), 3),
            "p95_ms": round(percentile(benchmark_ms, 0.95), 3),
            "max_ms": round(max(benchmark_ms), 3),
        },
    }


def oversized_frame_is_bounded(executable: Path) -> dict[str, Any]:
    process = subprocess.Popen(
        [str(executable)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    payload = b"x" * (MAX_MESSAGE_BYTES + 1) + b"\n"
    started = time.perf_counter()
    stdout, stderr = process.communicate(payload, timeout=10.0)
    elapsed_ms = (time.perf_counter() - started) * 1000.0
    require(process.returncode != 0, "oversized frame did not close the server with failure")
    require(stdout == b"", "oversized non-protocol frame reached stdout")
    require(
        stderr.startswith(b"NUIF_MCP_SERVER_FAILED:"),
        "oversized frame did not produce the stable server failure class",
    )
    require(elapsed_ms < 2000.0, "oversized frame exceeded catastrophic time budget")
    return {
        "bytes": len(payload) - 1,
        "elapsed_ms": round(elapsed_ms, 3),
        "exit_status": process.returncode,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--server", type=Path, required=True)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--variable-font-package", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    for path in (args.server, args.cli, args.fixture, args.variable_font_package):
        require(path.is_file(), f"required input does not exist: {path}")

    result = exercise(
        args.server.resolve(),
        args.cli.resolve(),
        args.fixture.resolve(),
        args.variable_font_package.resolve(),
    )
    oversized = oversized_frame_is_bounded(args.server.resolve())
    report = {
        "schema_version": 1,
        "experiment": "nuif:experiment:mcp-cross-surface",
        "status": "passed",
        "api_profile": API_PROFILE,
        "protocol_version": PROTOCOL_VERSION,
        "server_version": "0.0.1",
        "source": {
            "revision": os.environ.get("GITHUB_SHA")
            or subprocess.run(
                ["git", "rev-parse", "HEAD"], capture_output=True, check=True, text=True
            ).stdout.strip(),
            "dirty": bool(subprocess.run(["git", "status", "--porcelain"], capture_output=True, check=True).stdout),
        },
        "tools": EXPECTED_TOOLS,
        "authorities": [],
        "cross_surface": result,
        "oversized_frame": oversized,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": "passed", "report": str(args.report)}))


if __name__ == "__main__":
    main()
