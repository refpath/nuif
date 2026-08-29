import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import nuif_profile0 as profile


def entity(
    identifier, *, width, height, family="freeform", direction="row", children=None
):
    return {
        "schema_version": 1,
        "id": identifier,
        "name": identifier,
        "kind": {"type": "container"},
        "children": children or [],
        "authored": {
            "width": width,
            "height": height,
            "position": {"x": 0, "y": 0},
            "layout": {
                "family": family,
                "direction": direction,
                "align": "stretch",
                "gap": 0,
                "padding": {"top": 0, "right": 0, "bottom": 0, "left": 0},
            },
            "responsive": [],
            "fill": None,
            "text": None,
            "values": {},
        },
        "semantics": {"role": None, "accessible_name": None, "states": {}},
        "extensions": {},
    }


class IndependentProfileTests(unittest.TestCase):
    def test_canonical_writer_and_duplicate_rejection(self):
        value = {"schema_version": 1, "z": 2, "aa": 1}
        self.assertEqual(
            profile.canonical_text(value),
            b'{\n  "aa": 1,\n  "schema_version": 1,\n  "z": 2\n}\n',
        )
        with self.assertRaises(profile.ProfileError):
            profile.parse_document(b'{"schema_version":1,"schema_version":1}')

    def test_stack_layout_is_computed_from_authored_intent(self):
        root_id = "0" * 31 + "1"
        first_id = "0" * 31 + "2"
        second_id = "0" * 31 + "3"
        root = entity(
            root_id,
            width={"type": "fill"},
            height={"type": "fill"},
            family="stack",
            direction="row",
            children=[first_id, second_id],
        )
        first = entity(
            first_id, width={"type": "fixed", "value": 20}, height={"type": "fill"}
        )
        second = entity(
            second_id, width={"type": "fill"}, height={"type": "fixed", "value": 10}
        )
        document = {
            "schema_version": 1,
            "id": "0" * 31 + "9",
            "entities": {root_id: root, first_id: first, second_id: second},
            "roots": [root_id],
            "tokens": {},
            "relations": [],
            "extension_declarations": {"required": [], "used": [], "fallback_kind": {}},
            "extensions": {},
        }
        profile.validate_document(document)
        boxes = profile.evaluate_layout(document, 100, 40)
        self.assertEqual(boxes[first_id], profile.Rect(0, 0, 20, 40))
        self.assertEqual(boxes[second_id], profile.Rect(20, 0, 80, 10))

    def test_fractional_ahem_coverage_and_png_roundtrip(self):
        text = {
            "font": "Ahem",
            "font_sha256": profile.PINNED_FONT_SHA256,
            "size": 2,
            "line_height": 2,
            "content": "A",
        }
        rgba = bytearray([255] * (4 * 2 * 4))
        profile._draw_ahem_text(rgba, 4, 2, profile.Rect(0.5, 0, 2, 2), text)
        self.assertEqual(tuple(rgba[0:4]), (128, 128, 128, 255))
        self.assertEqual(tuple(rgba[4:8]), (0, 0, 0, 255))
        self.assertEqual(tuple(rgba[8:12]), (128, 128, 128, 255))
        encoded = profile.encode_png(4, 2, bytes(rgba))
        width, height, decoded = profile.decode_png_rgba(encoded)
        self.assertEqual((width, height, decoded), (4, 2, bytes(rgba)))


if __name__ == "__main__":
    unittest.main()
