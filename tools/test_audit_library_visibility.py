"""Checks that the library audit detects sequence and multiplicity loss."""
import unittest

from audit_library_visibility import compare_collection_sequences


class CollectionSequenceTests(unittest.TestCase):
    def setUp(self):
        self.raw = {"relationships": [
            {"source": "A", "relation": "members", "target": "B"},
            {"source": "A", "relation": "members", "target": "C"},
            {"source": "A", "relation": "chaining_feature", "target": "B"},
            {"source": "A", "relation": "chaining_feature", "target": "B"},
        ]}
        self.by_id = {"A": {"properties": {
            "members": ["B", "C"], "chaining_feature": ["B", "B"]}}}

    def test_matching_sequence_and_duplicates(self):
        result = compare_collection_sequences(self.raw, self.by_id)
        self.assertEqual(result["compared_references"], 4)
        self.assertEqual(result["duplicate_targets"]["chaining_feature"], 1)

    def test_reordered_members_fail(self):
        self.by_id["A"]["properties"]["members"] = ["C", "B"]
        with self.assertRaisesRegex(ValueError, "order or duplicates"):
            compare_collection_sequences(self.raw, self.by_id)

    def test_collapsed_nonunique_reference_fails(self):
        self.by_id["A"]["properties"]["chaining_feature"] = ["B"]
        with self.assertRaisesRegex(ValueError, "order or duplicates"):
            compare_collection_sequences(self.raw, self.by_id)


if __name__ == "__main__":
    unittest.main()
