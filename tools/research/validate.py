#!/usr/bin/env python3
"""Validate research records against research/schema/research-item.schema.json.

Checks: YAML front matter parses, validates against the schema, `id` matches the
file name, relation targets and claim identifiers resolve, and coverage/index
references point at existing records. Exit status is non-zero on any failure.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

import jsonschema
import yaml

ROOT = Path(__file__).resolve().parents[2]
ITEMS = ROOT / "research" / "items"
SCHEMA = ROOT / "research" / "schema" / "research-item.schema.json"
INDEX = ROOT / "research" / "index.yaml"
COVERAGE = ROOT / "research" / "coverage.yaml"
EXPERIMENTS = ROOT / "research" / "experiments" / "index.yaml"
QUESTIONS = ROOT / "research" / "questions.yaml"
FRONT = re.compile(r"\A---\n(.*?)\n---\n", re.S)
BODY_SECTIONS = ["# Summary", "## Evidence", "## Mechanism", "## NUIF relevance", "## Open questions"]


def normalize(value):
    """Coerce YAML dates to ISO strings and drop null source fields."""
    import datetime

    if isinstance(value, (datetime.date, datetime.datetime)):
        return value.isoformat()
    if isinstance(value, dict):
        return {k: normalize(v) for k, v in value.items() if v is not None}
    if isinstance(value, list):
        return [normalize(v) for v in value]
    return value


def front_matter(path: Path) -> tuple[dict, str]:
    text = path.read_text(encoding="utf-8")
    match = FRONT.match(text)
    if not match:
        raise ValueError("missing YAML front matter")
    return normalize(yaml.safe_load(match.group(1))), text[match.end():]


def main() -> int:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    validator = jsonschema.Draft202012Validator(schema, format_checker=jsonschema.FormatChecker())
    errors: list[str] = []
    ids: set[str] = set()
    records: dict[str, dict] = {}
    claims: set[str] = set()
    if INDEX.exists():
        index = yaml.safe_load(INDEX.read_text(encoding="utf-8")) or {}
        claims = {p["id"] for p in index.get("principles", [])}
    experiments: set[str] = set()
    if EXPERIMENTS.exists():
        registry = yaml.safe_load(EXPERIMENTS.read_text(encoding="utf-8")) or {}
        experiments = {e["id"] for e in registry.get("experiments", [])}

    for path in sorted(ITEMS.glob("*.md")):
        if path.name.startswith("_"):
            continue
        try:
            data, body = front_matter(path)
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{path.name}: {exc}")
            continue
        for err in sorted(validator.iter_errors(data), key=lambda e: list(e.path)):
            errors.append(f"{path.name}: {'/'.join(map(str, err.path)) or '<root>'}: {err.message}")
        expected = f"nuif:research:{path.stem}"
        if data.get("id") != expected:
            errors.append(f"{path.name}: id {data.get('id')!r} does not match file name ({expected})")
        if data.get("id") in ids:
            errors.append(f"{path.name}: duplicate id {data['id']}")
        ids.add(data.get("id", ""))
        records[path.name] = data
        required = {"reviewed": ["# Summary", "## NUIF relevance"], "verified": BODY_SECTIONS}.get(data.get("status"), [])
        missing = [s for s in required if s not in body]
        if missing:
            errors.append(f"{path.name}: {data.get('status')} record lacks sections {missing}")

    for name, data in records.items():
        for rel in data.get("relations", []) or []:
            target = rel.get("target", "")
            if target.startswith("nuif:research:") and target not in ids:
                errors.append(f"{name}: relation target {target} does not exist")
        for claim in data.get("claims", []) or []:
            if claims and claim not in claims:
                errors.append(f"{name}: claim {claim} is not declared in research/index.yaml")
        for group, paths in (data.get("links") or {}).items():
            for rel_path in paths or []:
                if group == "experiments":
                    if rel_path not in experiments and not (ROOT / rel_path).exists():
                        errors.append(f"{name}: links.experiments entry {rel_path} is neither a registered experiment id nor a path")
                elif not (ROOT / rel_path).exists():
                    errors.append(f"{name}: links.{group} path {rel_path} does not exist")

    if INDEX.exists():
        for topic, entries in (index.get("topics") or {}).items():
            for entry in entries or []:
                target = f"nuif:research:{entry}"
                if target not in ids:
                    errors.append(f"index.yaml: topic {topic}: record {target} does not exist")

        supported_claims = {
            claim
            for data in records.values()
            for claim in (data.get("claims") or [])
        }
        for claim in sorted(claims - supported_claims):
            errors.append(f"index.yaml: claim {claim} has no linked research record")

    if QUESTIONS.exists():
        question_registry = yaml.safe_load(QUESTIONS.read_text(encoding="utf-8")) or {}
        question_ids: set[str] = set()
        valid_question_status = {
            "open", "experiment-required", "benchmark-required", "governance",
            "research", "deferred", "decided",
        }
        for question in question_registry.get("questions", []) or []:
            question_id = question.get("id", "")
            if question_id in question_ids:
                errors.append(f"questions.yaml: duplicate id {question_id}")
            question_ids.add(question_id)
            if not question_id.startswith("nuif:question:"):
                errors.append(f"questions.yaml: invalid id {question_id}")
            if question.get("status") not in valid_question_status:
                errors.append(f"questions.yaml: {question_id}: invalid status {question.get('status')}")
            if question.get("status") == "decided":
                decision = question.get("decided_by")
                if not decision or not (ROOT / decision).exists():
                    errors.append(f"questions.yaml: {question_id}: decided question lacks an existing decided_by artifact")
            for evidence in question.get("evidence", []) or []:
                if evidence not in ids:
                    errors.append(f"questions.yaml: {question_id}: evidence {evidence} does not exist")

    if EXPERIMENTS.exists():
        experiment_registry = yaml.safe_load(EXPERIMENTS.read_text(encoding="utf-8")) or {}
        valid_experiment_status = {"planned", "active", "automated", "blocked", "completed"}
        seen_experiments: set[str] = set()
        for experiment in experiment_registry.get("experiments", []) or []:
            experiment_id = experiment.get("id", "")
            if experiment_id in seen_experiments:
                errors.append(f"experiments/index.yaml: duplicate id {experiment_id}")
            seen_experiments.add(experiment_id)
            status = experiment.get("status")
            if status not in valid_experiment_status:
                errors.append(f"experiments/index.yaml: {experiment_id}: invalid status {status}")
            if not experiment.get("tests"):
                errors.append(f"experiments/index.yaml: {experiment_id}: tests must not be empty")
            if not experiment.get("oracle"):
                errors.append(f"experiments/index.yaml: {experiment_id}: oracle class is required")
            if not experiment.get("acceptance"):
                errors.append(f"experiments/index.yaml: {experiment_id}: acceptance criteria are required")
            if status in {"active", "automated", "completed"} and not experiment.get("implementation"):
                errors.append(f"experiments/index.yaml: {experiment_id}: {status} experiment requires implementation paths")
            for evidence in experiment.get("evidence", []) or []:
                if evidence not in ids:
                    errors.append(f"experiments/index.yaml: {experiment_id}: evidence {evidence} does not exist")
            for artifact in ([experiment.get("fixture")] if experiment.get("fixture") else []) + (experiment.get("implementation") or []):
                if not (ROOT / artifact).exists():
                    errors.append(f"experiments/index.yaml: {experiment_id}: artifact {artifact} does not exist")

    if COVERAGE.exists():
        coverage = yaml.safe_load(COVERAGE.read_text(encoding="utf-8")) or {}
        for area, spec in (coverage.get("areas") or {}).items():
            for ev in spec.get("evidence", []) or []:
                if ev not in ids:
                    errors.append(f"coverage.yaml: {area}: evidence {ev} does not exist")
            for art in spec.get("artifacts", []) or []:
                if not (ROOT / art).exists():
                    errors.append(f"coverage.yaml: {area}: artifact {art} does not exist")

    for line in errors:
        print(line)
    print(f"{len(records)} records, {len(errors)} errors")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
