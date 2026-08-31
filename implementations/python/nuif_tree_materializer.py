#!/usr/bin/env python3
"""Independent structural replay for ``nuif-collab-tree-0``.

This oracle intentionally implements only the tree materializer.  It does not
import the Rust workspace, decode NUIF CBOR, or claim ownership of semantic
conflict attribution.  Its input contains the canonical base document and the
same immutable structural changes supplied to the Rust conformance binary; the
oracle computes parent/order/anchor decisions and compares that projection with
Rust's checkpoint.
"""

from __future__ import annotations

import argparse
import json
import platform
from collections import defaultdict
from pathlib import Path
from typing import Any


class TreeOracleError(Exception):
    """A malformed or unsupported tree fixture."""


def _id_key(change_id: dict[str, Any]) -> tuple[int, str]:
    counter = change_id.get("counter")
    replica = change_id.get("replica")
    if not isinstance(counter, int) or isinstance(counter, bool) or counter <= 0:
        raise TreeOracleError("change counter must be a positive integer")
    if not isinstance(replica, str) or not replica:
        raise TreeOracleError("change replica must be a non-empty string")
    return counter, replica


def _position_key(position: dict[str, Any]) -> tuple[Any, ...]:
    kind = position.get("kind")
    if kind == "base":
        value = position.get("value")
        if not isinstance(value, str):
            raise TreeOracleError("base position value must be an entity id")
        return (0, value)
    if kind == "change":
        value = position.get("value")
        if not isinstance(value, dict):
            raise TreeOracleError("change position value must be a change id")
        counter, replica = _id_key(value)
        return (1, counter, replica)
    raise TreeOracleError(f"unsupported position kind: {kind!r}")


def _position_json(key: tuple[Any, ...]) -> dict[str, Any]:
    if key[0] == 0:
        return {"kind": "base", "value": key[1]}
    return {
        "kind": "change",
        "value": {"counter": key[1], "replica": key[2]},
    }


def _change_key(change: dict[str, Any]) -> tuple[int, str]:
    identifier = change.get("id")
    if not isinstance(identifier, dict):
        raise TreeOracleError("change id must be an object")
    return _id_key(identifier)


def _canonical_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, allow_nan=False, sort_keys=True, separators=(",", ":"))


def _conflict_key(conflict: dict[str, Any]) -> tuple[int, str]:
    # Match the Rust enum's declared BTreeSet variant order, then use a
    # canonical representation for fields within one variant.
    ranks = {
        "concurrent_move": 0,
        "delete_move": 1,
        "delete_descendant_move": 2,
        "deleted_parent": 3,
        "cycle_rejected": 4,
        "anchor_unavailable": 5,
        "self_anchor": 6,
    }
    kind = conflict.get("kind")
    return ranks.get(kind, 99), _canonical_json(conflict)


def _validate_base(base: dict[str, Any]) -> tuple[dict[str, str | None], dict[str, tuple[Any, ...]]]:
    entities = base.get("entities")
    roots = base.get("roots")
    if not isinstance(entities, dict) or not isinstance(roots, list):
        raise TreeOracleError("base document requires entities and roots")
    parents: dict[str, str | None] = {}
    positions: dict[str, tuple[Any, ...]] = {}
    for identifier, entity in entities.items():
        if not isinstance(identifier, str) or not isinstance(entity, dict):
            raise TreeOracleError("base entity entries must be objects")
        if entity.get("id") != identifier:
            raise TreeOracleError(f"base entity key/id mismatch for {identifier}")
        children = entity.get("children")
        if not isinstance(children, list):
            raise TreeOracleError(f"base entity {identifier} children must be a list")
        for child in children:
            if not isinstance(child, str) or child not in entities:
                raise TreeOracleError(f"base entity {identifier} references an unknown child")
            if child in parents:
                raise TreeOracleError(f"base entity {child} has more than one parent")
            parents[child] = identifier
    if len(set(roots)) != len(roots):
        raise TreeOracleError("base roots must be unique")
    for root in roots:
        if not isinstance(root, str) or root not in entities or root in parents:
            raise TreeOracleError(f"invalid base root {root!r}")
    for identifier in entities:
        current = identifier
        seen: set[str] = set()
        while current in parents:
            if current in seen:
                raise TreeOracleError("base containment cycle")
            seen.add(current)
            current = parents[current]  # type: ignore[assignment]
    for identifier in entities:
        if identifier not in parents and identifier not in roots:
            raise TreeOracleError(f"base entity {identifier} is unreachable")
    for parent, children in [(None, roots)] + [
        (identifier, entity["children"]) for identifier, entity in entities.items()
    ]:
        origin: tuple[Any, ...] | None = None
        for entity in children:
            if entity in positions:
                raise TreeOracleError(f"base entity {entity} appears twice")
            position = (0, entity)
            positions[entity] = position
            origin = position
    return parents, positions


def _validate_changes(
    changes: list[dict[str, Any]],
    entities: set[str],
) -> list[dict[str, Any]]:
    indexed: dict[tuple[int, str], dict[str, Any]] = {}
    per_replica: dict[str, list[int]] = defaultdict(list)
    for change in changes:
        if not isinstance(change, dict):
            raise TreeOracleError("changes must be objects")
        key = _change_key(change)
        if key in indexed and _canonical_json(indexed[key]) != _canonical_json(change):
            raise TreeOracleError(f"duplicate change id with different contents: {key}")
        indexed[key] = change
        per_replica[key[1]].append(key[0])
        operation = change.get("operation")
        if not isinstance(operation, dict) or operation.get("op") not in {"move", "delete"}:
            raise TreeOracleError("unsupported structural operation")
        if operation.get("entity") not in entities:
            raise TreeOracleError("operation references an unknown entity")
        parent = operation.get("new_parent")
        if parent is not None and parent not in entities:
            raise TreeOracleError("move references an unknown parent")
        if operation["op"] == "move":
            anchor = operation.get("anchor")
            if not isinstance(anchor, dict) or anchor.get("kind") not in {"start", "after"}:
                raise TreeOracleError("move requires a start or after anchor")
            if anchor["kind"] == "after" and not isinstance(anchor.get("position"), dict):
                raise TreeOracleError("after anchor requires a position")
        context = change.get("context", {})
        if not isinstance(context, dict):
            raise TreeOracleError("change context must be an object")
        observed = context.get(key[1], 0)
        if not isinstance(observed, int) or observed + 1 != key[0]:
            raise TreeOracleError(f"invalid local context for {key}")
    for replica, counters in per_replica.items():
        if sorted(set(counters)) != list(range(1, max(counters) + 1)):
            raise TreeOracleError(f"replica {replica} log is not contiguous")
    for change in indexed.values():
        key = _change_key(change)
        context = change.get("context", {})
        for replica, counter in context.items():
            if not isinstance(replica, str) or not isinstance(counter, int) or counter < 0:
                raise TreeOracleError("invalid causal context")
            if counter == 0:
                continue
            dependency = indexed.get((counter, replica))
            if dependency is None:
                raise TreeOracleError(f"missing causal dependency {(counter, replica)}")
            dependency_context = dependency.get("context", {})
            for dep_replica, dep_counter in dependency_context.items():
                if context.get(dep_replica, 0) < dep_counter:
                    raise TreeOracleError(f"causal context is not transitively closed for {key}")
        operation = change["operation"]
        anchor = operation.get("anchor")
        if operation["op"] != "move" or not isinstance(anchor, dict) or anchor.get("kind") != "after":
            continue
        position = anchor.get("position")
        if not isinstance(position, dict) or position.get("kind") != "change":
            continue
        anchor_key = _position_key(position)
        anchor_id = (anchor_key[1], anchor_key[2])
        anchor_change = indexed.get(anchor_id)
        if anchor_change is None:
            raise TreeOracleError(f"missing anchor change {anchor_id}")
        if context.get(anchor_id[1], 0) < anchor_id[0]:
            raise TreeOracleError(f"anchor change {anchor_id} is not causal")
    return sorted(indexed.values(), key=_change_key)


def _replay(base: dict[str, Any], changes: list[dict[str, Any]]) -> tuple[list[str], dict[str, list[str]], dict[str, dict[str, Any]], list[dict[str, Any]]]:
    parents, base_positions = _validate_base(base)
    active: dict[str, tuple[Any, ...]] = dict(base_positions)
    positions: dict[tuple[Any, ...], dict[str, Any]] = {}
    for parent, children in [(None, base["roots"])] + [
        (identifier, entity["children"]) for identifier, entity in base["entities"].items()
    ]:
        origin: tuple[Any, ...] | None = None
        for entity in children:
            position = (0, entity)
            positions[position] = {"entity": entity, "parent": parent, "origin": origin, "active": True}
            origin = position
    conflicts: list[dict[str, Any]] = []

    def is_ancestor(ancestor: str, candidate: str) -> bool:
        current: str | None = candidate
        visited: set[str] = set()
        while current is not None:
            if current == ancestor:
                return True
            if current in visited:
                return True
            visited.add(current)
            current = parents.get(current)
        return False

    def deactivate(entity: str) -> None:
        position = active.pop(entity, None)
        if position is not None and position in positions:
            positions[position]["active"] = False

    for change in changes:
        identifier = change["id"]
        change_id = {"counter": identifier["counter"], "replica": identifier["replica"]}
        operation = change["operation"]
        entity = operation["entity"]
        if operation["op"] == "delete":
            deactivate(entity)
            parents[entity] = None  # None is the foreign oracle's trash marker.
            continue
        parent = operation.get("new_parent")
        if parent == entity or (parent is not None and is_ancestor(entity, parent)):
            conflicts.append({"kind": "cycle_rejected", "change": change_id, "entity": entity, "new_parent": parent})
            continue
        anchor = operation.get("anchor")
        origin: tuple[Any, ...] | None
        if not isinstance(anchor, dict) or anchor.get("kind") == "start":
            origin = None
        else:
            position = _position_key(anchor.get("position", {}))
            anchor_state = positions.get(position)
            if anchor_state is None or anchor_state["parent"] != parent:
                conflicts.append({"kind": "anchor_unavailable", "change": change_id, "anchor": _position_json(position)})
                continue
            if anchor_state["entity"] == entity:
                conflicts.append({"kind": "self_anchor", "change": change_id, "entity": entity})
                continue
            origin = position
        deactivate(entity)
        position = (1, identifier["counter"], identifier["replica"])
        positions[position] = {"entity": entity, "parent": parent, "origin": origin, "active": True}
        active[entity] = position
        parents[entity] = parent

    grouped: dict[str | None, dict[tuple[Any, ...] | None, list[tuple[Any, ...]]]] = defaultdict(lambda: defaultdict(list))
    for position, state in positions.items():
        grouped[state["parent"]][state["origin"]].append(position)
    ordered: dict[str | None, list[str]] = {}
    for parent, descendants in grouped.items():
        for positions_for_origin in descendants.values():
            positions_for_origin.sort()
        stack = list(descendants.pop(None, []))
        result: list[str] = []
        while stack:
            position = stack.pop()
            state = positions[position]
            if state["active"] and parents.get(state["entity"]) == parent:
                result.append(state["entity"])
            stack.extend(descendants.pop(position, []))
        ordered[parent] = result
    roots = ordered.get(None, [])
    children = {identifier: ordered.get(identifier, []) for identifier in base["entities"]}
    reachable: set[str] = set()
    pending = list(roots)
    while pending:
        entity = pending.pop()
        if entity in reachable:
            continue
        reachable.add(entity)
        pending.extend(children.get(entity, []))
    children = {identifier: value for identifier, value in children.items() if identifier in reachable}
    active_json = {
        entity: _position_json(position)
        for entity, position in sorted(active.items())
        if entity in reachable
    }
    conflicts.sort(key=_conflict_key)
    return roots, children, active_json, conflicts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    arguments = parser.parse_args()
    try:
        input_value = json.loads(arguments.input.read_text(encoding="utf-8"))
        if input_value.get("schema_version") != 1 or input_value.get("profile") != "nuif-collab-tree-0":
            raise TreeOracleError("unsupported structural oracle input")
        base = input_value.get("base_document")
        changes = input_value.get("expected_changes")
        expected = input_value.get("expected_tree")
        if not isinstance(base, dict) or not isinstance(changes, list) or not isinstance(expected, dict):
            raise TreeOracleError("structural oracle input is missing tree fields")
        ordered = _validate_changes(changes, set(base.get("entities", {})))
        roots, children, active, conflicts = _replay(base, ordered)
        expected_conflicts = expected.get("replay_conflicts")
        if isinstance(expected_conflicts, list):
            expected_conflicts = sorted(expected_conflicts, key=_conflict_key)
        checks = {
            "base_hash_bound": isinstance(input_value.get("base_canonical_hash"), str),
            "expected_hash_bound": isinstance(input_value.get("expected_canonical_hash"), str),
            "change_count_exact": len(ordered) == len(changes),
            "roots_exact": roots == expected.get("roots"),
            "children_exact": children == expected.get("children"),
            "active_positions_exact": active == expected.get("active_positions"),
            "replay_conflicts_exact": conflicts == expected_conflicts,
        }
        observed = {"roots": roots, "children": children, "active_positions": active, "replay_conflicts": conflicts}
        status = "passed" if all(checks.values()) else "failed"
        report = {
            "schema_version": 1,
            "experiment": "nuif:experiment:crdt-foreign-tree-materializer",
            "status": status,
            "profile": input_value["profile"],
            "oracle": {
                "name": "python-standard-library-tree-replay",
                "python": platform.python_version(),
                "dependencies": ["python-standard-library"],
            },
            "role": "Independent parent/order/anchor replay; Rust remains authoritative for canonical hashing and semantic conflict attribution.",
            "checks": checks,
            "observed": observed,
            "boundaries": [
                "does not decode canonical CBOR or recompute the NUIF canonical hash",
                "does not independently classify concurrent semantic conflict families",
                "does independently reject malformed causal and structural fixture data",
            ],
        }
    except (OSError, json.JSONDecodeError, TreeOracleError, TypeError, ValueError) as error:
        status = "failed"
        report = {
            "schema_version": 1,
            "experiment": "nuif:experiment:crdt-foreign-tree-materializer",
            "status": status,
            "oracle": {"name": "python-standard-library-tree-replay", "python": platform.python_version()},
            "error": str(error),
        }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"foreign tree materializer: status {report['status']}")
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
