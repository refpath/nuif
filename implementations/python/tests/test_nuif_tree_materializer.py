import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import nuif_tree_materializer as tree


def base_document():
    root = "0" * 31 + "1"
    child = "0" * 31 + "2"
    return {
        "schema_version": 1,
        "id": "0" * 31 + "9",
        "roots": [root],
        "entities": {
            root: {"id": root, "children": [child]},
            child: {"id": child, "children": []},
        },
    }, root, child


def change(replica, counter, operation, context=None):
    return {
        "id": {"counter": counter, "replica": replica},
        "context": context or {},
        "operation": operation,
    }


class IndependentTreeTests(unittest.TestCase):
    def test_replays_move_and_exposes_cycle_rejection(self):
        base, root, child = base_document()
        changes = [
            change(
                "alice",
                1,
                {
                    "op": "move",
                    "entity": child,
                    "new_parent": root,
                    "anchor": {"kind": "start"},
                },
            ),
            change(
                "bob",
                1,
                {
                    "op": "move",
                    "entity": root,
                    "new_parent": child,
                    "anchor": {"kind": "start"},
                },
            ),
        ]
        roots, children, active, conflicts = tree._replay(
            base, tree._validate_changes(changes, set(base["entities"]))
        )
        self.assertEqual(roots, [root])
        self.assertEqual(children[root], [child])
        self.assertEqual(active[child]["value"], {"counter": 1, "replica": "alice"})
        self.assertEqual(conflicts[0]["kind"], "cycle_rejected")

    def test_rejects_noncausal_change_anchor(self):
        base, root, child = base_document()
        origin = change(
            "alice",
            1,
            {
                "op": "move",
                "entity": child,
                "new_parent": None,
                "anchor": {"kind": "start"},
            },
        )
        dependent = change(
            "bob",
            1,
            {
                "op": "move",
                "entity": root,
                "new_parent": None,
                "anchor": {
                    "kind": "after",
                    "position": {
                        "kind": "change",
                        "value": {"counter": 1, "replica": "alice"},
                    },
                },
            },
        )
        with self.assertRaises(tree.TreeOracleError):
            tree._validate_changes([origin, dependent], set(base["entities"]))


if __name__ == "__main__":
    unittest.main()
