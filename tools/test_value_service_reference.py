"""Reference cache integrity: this suite never awards native qualification."""
import copy, gzip, json, unittest
from pathlib import Path
from run_value_service_reference import validate, comparison_view

class ValueServiceReferenceTests(unittest.TestCase):
    def setUp(self):
        self.manifest = {"cases": [{"id": "accepted"}, {"id": "rejected"}]}
        self.doc = {"native_qualification": "not_assessed", "controls": [
            {"id": "accepted", "parse_ok": True, "validation_status": "accepted",
             "diagnostics": [], "model_status": "exported", "model": {
                "metadata": {"element_count": 2, "relationship_count": 1},
                "elements": [
                    {"qualified_name": "P", "kind": "Package", "properties": {}},
                    {"qualified_name": "P::v", "kind": "Feature", "properties": {
                        "element_id": "11111111-1111-4111-8111-111111111111", "is_variable": False}}],
                "relationships": [{"source": "P", "target": "P::v", "kind": "owned_member"}]}},
            {"id": "rejected", "parse_ok": True, "validation_status": "rejected",
             "model_status": "not_published", "diagnostics": [{"severity": "ERROR"}]}]}

    def rejects(self, mutate):
        d = copy.deepcopy(self.doc)
        mutate(d)
        with self.assertRaises(ValueError): validate(d, self.manifest)

    def test_complete_reference_inventory(self):
        validate(self.doc, self.manifest)
        self.rejects(lambda d: d["controls"].pop())
        self.rejects(lambda d: d["controls"].__setitem__(0, d["controls"][1]))

    def test_export_cannot_award_native_support(self):
        self.rejects(lambda d: d.__setitem__("native_qualification", "qualified"))
        self.rejects(lambda d: d["controls"][0].__setitem__("validation_status", "qualified"))

    def test_success_requires_full_model_and_clean_validation(self):
        self.rejects(lambda d: d["controls"][0].pop("model"))
        self.rejects(lambda d: d["controls"][0].__setitem__("model_status", "not_published"))
        self.rejects(lambda d: d["controls"][0]["diagnostics"].append({"severity": "ERROR"}))
        self.rejects(lambda d: d["controls"][0]["model"].__setitem__("elements", []))

    def test_rejection_cannot_carry_successful_model(self):
        self.rejects(lambda d: d["controls"][1].__setitem__("model", d["controls"][0]["model"]))

    def test_exact_identity_and_relationship_closure(self):
        self.rejects(lambda d: d["controls"][0]["model"]["elements"][1].__setitem__("qualified_name", "P"))
        self.rejects(lambda d: d["controls"][0]["model"]["relationships"][0].__setitem__("target", "missing"))
        self.rejects(lambda d: d["controls"][0]["model"]["metadata"].__setitem__("element_count", 3))
        self.rejects(lambda d: d["controls"][0]["model"]["metadata"].__setitem__("relationship_count", 2))

    def test_identity_comparison_preserves_raw_data_and_semantics(self):
        before = copy.deepcopy(self.doc)
        changed = copy.deepcopy(self.doc)
        props = changed["controls"][0]["model"]["elements"][1]["properties"]
        props["element_id"] = "22222222-2222-4222-8222-222222222222"
        self.assertEqual(comparison_view(self.doc), comparison_view(changed))
        self.assertEqual(self.doc, before)
        props["is_variable"] = True
        self.assertNotEqual(comparison_view(self.doc), comparison_view(changed))
        props["element_id"] = "not-a-generated-uuid"
        with self.assertRaises(ValueError): comparison_view(changed)

    def test_member_identity_alias_must_match_exported_endpoint(self):
        d = copy.deepcopy(self.doc)
        model = d["controls"][0]["model"]
        props = model["elements"][0]["properties"]
        props["member_element_id"] = model["elements"][1]["properties"]["element_id"]
        props["owned_member_element_id"] = props["member_element_id"]
        model["relationships"][0]["relation"] = "member_element"
        view = comparison_view(d)
        self.assertEqual(view["controls"][0]["model"]["elements"][0]["properties"]["member_element_id"], {"generated_identity": "P::v"})
        model["relationships"][0]["target"] = "P"
        with self.assertRaises(ValueError): comparison_view(d)

    def test_uuid_literal_and_duplicate_identity_cannot_be_hidden(self):
        d = copy.deepcopy(self.doc)
        model = d["controls"][0]["model"]
        model["elements"][0]["properties"]["declared_name"] = model["elements"][1]["properties"]["element_id"]
        self.assertNotEqual(comparison_view(d), comparison_view(self.doc))
        model["elements"][0]["properties"]["element_id"] = model["elements"][1]["properties"]["element_id"]
        with self.assertRaises(ValueError): comparison_view(d)

    def test_order_and_diagnostics_are_not_normalized(self):
        changed = copy.deepcopy(self.doc)
        changed["controls"][0]["model"]["elements"].reverse()
        self.assertNotEqual(comparison_view(self.doc), comparison_view(changed))
        changed = copy.deepcopy(self.doc)
        changed["controls"][1]["diagnostics"][0]["code"] = "different"
        self.assertNotEqual(comparison_view(self.doc), comparison_view(changed))

    def test_checked_in_whole_batch(self):
        root = Path(__file__).resolve().parents[1] / "docs/conformance/2026-08-support"
        manifest = json.loads((root / "value-service-reference-manifest.json").read_text(encoding="utf-8"))
        doc = json.loads(gzip.decompress((root / "value-service-reference-controls.json.gz").read_bytes()))
        validate(doc, manifest)
        outcomes = [r["validation_status"] for r in doc["controls"]]
        self.assertEqual((outcomes.count("accepted"), outcomes.count("rejected"), outcomes.count("infrastructure_error")), (19, 7, 0))
        self.assertEqual(len(manifest["cases"]), 26)

if __name__ == "__main__": unittest.main()
