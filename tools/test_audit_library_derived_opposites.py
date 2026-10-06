"""Fault controls for source-derived Ecore inverse graph evidence."""
import copy
import unittest

from audit_library_derived_opposites import compare


class DerivedOppositeAuditTests(unittest.TestCase):
    def setUp(self):
        self.previous = {"elements": [
            {"id": "owner", "kind": "Namespace", "properties": {}},
            {"id": "first", "kind": "Feature", "properties": {"owner": "owner"}},
            {"id": "second", "kind": "Feature", "properties": {"owner": "owner"}},
            {"id": "membership", "kind": "OwningMembership", "properties": {}},
        ]}
        self.candidate = copy.deepcopy(self.previous)
        self.candidate["elements"][0]["properties"]["owned_element"] = ["second", "first"]
        self.candidate["elements"][3]["properties"]["owned_member_element"] = "first"
        self.raw = {"relationships": [
            {"source": "second", "relation": "owner", "target": "owner"},
            {"source": "first", "relation": "owner", "target": "owner"},
            {"source": "first", "relation": "owning_membership", "target": "membership"},
        ]}
        self.counts = {"owned_element": 2, "owned_member_element": 1}

    def test_exact_inverse_graph_passes(self):
        self.assertEqual(compare(self.previous, self.candidate, self.raw, self.counts), self.counts)

    def test_reordered_inverse_fails(self):
        self.candidate["elements"][0]["properties"]["owned_element"].reverse()
        with self.assertRaisesRegex(ValueError, "cardinality/order"):
            compare(self.previous, self.candidate, self.raw, self.counts)

    def test_prior_property_loss_fails(self):
        del self.candidate["elements"][1]["properties"]["owner"]
        with self.assertRaisesRegex(ValueError, "prior library property"):
            compare(self.previous, self.candidate, self.raw, self.counts)


if __name__ == "__main__":
    unittest.main()
