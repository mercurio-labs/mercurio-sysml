import copy
import json
import unittest
from map_ordinary_strategy_integration import PROFILE, inventory

class IntegrationInventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = [json.loads((PROFILE / n).read_text(encoding="utf-8")) for n in ("feature-redefinitions.extract.json", "grammar.structure.extract.json", "definition-document-pilot-controls.json", "feature-defaults.extract.json")]

    def test_exact_binding_gap_is_not_qualification(self):
        result = inventory(*self.inputs)
        self.assertEqual(len(result["bindings"]), 30)
        self.assertEqual(result["bindings_with_document_observations"], 27)
        self.assertEqual(result["qualified_bindings"], 0)
        self.assertEqual(result["bindings_without_grammar_candidates"], [])
        present = {r["kind"] for r in result["bindings"] if r["accepted_document_observations"]}
        self.assertTrue({"Feature", "PartUsage", "ItemUsage", "AttributeUsage", "PortUsage"} <= present)
        self.assertEqual(len(present), 27)

    def test_rejected_and_auxiliary_controls_never_count(self):
        inputs = copy.deepcopy(self.inputs)
        inputs[2]["controls"] = [c for c in inputs[2]["controls"] if c.get("accepted") is False]
        inputs[2]["ordinary_strategy_source_controls"] = []
        inputs[3]["usage_source_controls"] = []
        result = inventory(*inputs)
        self.assertEqual(result["bindings_with_document_observations"], 0)
        self.assertEqual(result["qualified_bindings"], 0)

    def test_disappearing_binding_requires_review(self):
        inputs = copy.deepcopy(self.inputs)
        inputs[0]["bindings"] = [b for b in inputs[0]["bindings"] if b["kind"] != "Feature"]
        with self.assertRaisesRegex(ValueError, "binding inventory"):
            inventory(*inputs)

    def test_return_types_do_not_become_explicit_actions(self):
        rows = inventory(*self.inputs)["bindings"]
        actions = {r["kind"] for r in rows if any(s["category"] == "action" for s in r["grammar_candidates"])}
        self.assertEqual(actions, {"Feature", "Multiplicity"})
        self.assertTrue(all(any(s["category"] == "return_type" for s in r["grammar_candidates"]) for r in rows))

if __name__ == "__main__": unittest.main()
