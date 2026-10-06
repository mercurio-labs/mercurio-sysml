"""Focused checks for the pinned Xtext/Ecore construction projection."""
import copy
import json
import unittest

from generate_xtext_assignment_contracts import GRAMMAR, ECORE, build, render_rust


class AssignmentContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grammar = json.loads(GRAMMAR.read_text(encoding="utf-8"))
        cls.ecore = json.loads(ECORE.read_text(encoding="utf-8"))

    def test_pinned_coverage_and_unassigned_relationship_arm(self):
        contract = build(self.grammar, self.ecore)
        self.assertEqual(contract["counts"], {
            "parser_rules": 704, "assignments": 771, "resolved_assignments": 771,
            "actions": 53, "resolved_action_features": 26, "bare_actions": 27,
        })
        self.assertTrue(all(not row["semantic_verified"] for row in contract["assignments"]))
        self.assertEqual(contract["unverified_semantics_by_dependency"]["handwritten_scoping_and_linking"], 71)
        self.assertGreater(contract["unverified_semantics_by_dependency"]["unassigned_rule_call_current_object_flow"], 0)
        self.assertEqual(len(contract["unassigned_calls"]), 975)
        edges = {(r["rule"], r["target"]) for r in contract["unassigned_calls"]}
        self.assertIn(("org.omg.kerml.xtext.KerML::RelationshipBody",
                       "org.omg.kerml.xtext.KerML::RelationshipOwnedElement"), edges)
        slots = [r for r in contract["assignments"]
                 if r["rule"] == "org.omg.kerml.xtext.KerML::RelationshipOwnedElement"
                 and r["feature"] == "ownedRelatedElement"]
        self.assertEqual(len(slots), 1)
        self.assertTrue(all(slot["ecore_feature"]["target"] == "SysML::Element" for slot in slots))
        self.assertTrue(all(slot["ecore_feature"]["containment"] for slot in slots))
        self.assertIn('feature: "ownedRelatedElement"', render_rust(contract))

    def test_effective_ecore_feature_change_reaches_native_output(self):
        ecore = copy.deepcopy(self.ecore)
        feature = next(r for r in ecore["features"] if r["id"].endswith("#//Relationship/ownedRelatedElement"))
        feature["containment"] = False
        contract = build(self.grammar, ecore)
        row = next(r for r in contract["assignments"]
                   if r["rule"] == "org.omg.kerml.xtext.KerML::RelationshipOwnedElement")
        self.assertFalse(row["ecore_feature"]["containment"])
        self.assertIn('feature: "ownedRelatedElement", operator: "+=", ecore_owner: "SysML::Relationship", ecore_target: "SysML::Element", kind: "reference", containment: false', render_rust(contract))

    def test_incompatible_cardinality_is_explicitly_unresolved(self):
        ecore = copy.deepcopy(self.ecore)
        feature = next(r for r in ecore["features"] if r["id"].endswith("#//Relationship/ownedRelatedElement"))
        feature["upper_bound"] = 1
        contract = build(self.grammar, ecore)
        row = next(r for r in contract["assignments"]
                   if r["rule"] == "org.omg.kerml.xtext.KerML::RelationshipOwnedElement")
        self.assertIsNone(row["ecore_feature"])
        self.assertEqual(row["unresolved"], "additive_assignment_to_singular_feature")
        self.assertNotIn('rule: "org.omg.kerml.xtext.KerML::RelationshipOwnedElement", feature: "ownedRelatedElement"', render_rust(contract))

    def test_mismatched_provenance_fails_closed(self):
        ecore = copy.deepcopy(self.ecore)
        ecore["provenance"]["ecore_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "source hash mismatch"):
            build(self.grammar, ecore)


if __name__ == "__main__":
    unittest.main()
