import copy
import json
import unittest
from check_ecore_annotation_coverage import INPUT, classify


class AnnotationCoverageTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.model = json.loads(INPUT.read_text(encoding="utf-8"))

    def test_every_annotation_preserved_and_reference_resolved(self):
        result = classify(self.model)
        self.assertEqual(result["annotation_count"], 1674)
        self.assertEqual(sum(result["category_counts"].values()), 1674)
        self.assertEqual([r["annotation"] for r in result["records"]],
                         [a for e in self.model["elements"] for a in e["annotations"]])
        self.assertTrue(all(r["resolved_reference_ids"] for r in result["records"] if r["category"] in {"subsets", "redefines"}))
        self.assertTrue(all(r["support"] == "semantic_dependency_open" for r in result["records"] if r["category"] in {"subsets", "union"}))

    def test_unknown_dialect_payload_or_reference_cannot_disappear(self):
        for mutation in ["source", "payload", "reference", "empty"]:
            model = copy.deepcopy(self.model)
            a = next(a for e in model["elements"] for a in e["annotations"] if a["attributes"].get("source") == "subsets")
            if mutation == "source": a["attributes"]["source"] = "new-semantics"
            elif mutation == "payload": a["children"] = [{"tag": "details", "attributes": {"key": "code", "value": "execute"}}]
            else: a["attributes"]["references"] = "#//Missing/property" if mutation == "reference" else ""
            with self.assertRaises(ValueError): classify(model)

    def test_documentation_formulas_never_gain_execution_credit(self):
        model = copy.deepcopy(self.model)
        a = next(a for e in model["elements"] for a in e["annotations"] if a["attributes"].get("source") == "http://www.eclipse.org/emf/2002/GenModel")
        a["children"][0]["attributes"]["value"] = "self.member->forAll(x | x.valid)"
        result = classify(model)
        docs = [r for r in result["records"] if r["category"] == "documentation"]
        self.assertTrue(all(r["support"] == "metadata_only" for r in docs))

    def test_delegate_marker_requires_imported_binding(self):
        model = copy.deepcopy(self.model)
        model["delegate_bindings"] = []
        with self.assertRaisesRegex(ValueError, "lacks resolved binding"): classify(model)


if __name__ == "__main__": unittest.main()
