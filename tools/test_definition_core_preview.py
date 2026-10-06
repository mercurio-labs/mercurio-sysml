"""Adversarial controls for the bounded preview acceptance boundary."""
import copy
import unittest
from qualify_definition_core_preview import BASE, PROFILE, read, verify_contract, verify_rows


class PreviewAcceptanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = read(BASE / "definition-core-preview-contract.json")
        cls.reference = read(PROFILE / "definition-document-pilot-controls.json")
        cls.effective = read(PROFILE / "ecore-effective.extract.json")
        cls.rows = []
        for case, oracle in zip(cls.contract["cases"], cls.reference["controls"]):
            row = dict(schema="dev.mercurio.definition-core-preview-model.v1", case_id=case["case_id"],
                       language=case["language"], definition_profile=cls.contract["profile"],
                       qualification_certificate=False, candidate_promoted=False,
                       semantic_validation="not_assessed", transformation_completion="not_assessed")
            if oracle["accepted"]:
                elements = {n["path"]: dict(id=n["path"], kind=n["kind"], properties=copy.deepcopy(n["attributes"]))
                            for n in oracle["nodes"]}
                for node in oracle["nodes"]:
                    elements[node["path"]]["properties"].update(copy.deepcopy(node["children"]))
                for link in oracle["links"]:
                    elements[link["owner_path"]]["properties"][link["field"]] = copy.deepcopy(link["target_paths"])
                document = dict(metadata=dict(semantic_validation="not_assessed"), elements=list(elements.values()))
                row.update(status="core_constructed", stage="stored_structure_and_supported_linking", kir_roundtrip="exact",
                           kir_document=document, roundtrip_kir_document=copy.deepcopy(document), abstract_syntax=[],
                           element_count=len(elements), reference_views=[dict(owner_path=l["owner_path"], field=l["field"], target_ids=l["target_paths"]) for l in oracle["links"]], roundtrip_reference_views=[dict(owner_path=l["owner_path"], field=l["field"], target_ids=l["target_paths"]) for l in oracle["links"]])
            else:
                row.update(status="blocked", stage="syntax")
            cls.rows.append(row)

    def matrix(self, rows):
        return verify_rows(rows, self.contract, self.reference, self.effective)

    def rejected(self, mutate):
        rows = copy.deepcopy(self.rows)
        mutate(rows)
        with self.assertRaises((ValueError, KeyError)):
            self.matrix(rows)

    def test_fixed_reference_matrix_is_admitted(self):
        verify_contract(self.contract, self.reference)
        self.assertEqual(self.matrix(self.rows), dict(supported_documents=90, syntax_negatives=8,
                         canonical_nodes=1403, stored_reference_slots=2595))

    def test_missing_case_rejected(self):
        self.rejected(lambda rows: rows.pop())

    def test_duplicate_case_rejected(self):
        self.rejected(lambda rows: rows[1].update(case_id=rows[0]["case_id"]))

    def test_unsupported_is_not_syntax_rejection(self):
        self.rejected(lambda rows: rows[4].update(stage="unsupported"))

    def test_negative_partial_model_rejected(self):
        self.rejected(lambda rows: rows[4].update(kir_document={}))

    def test_missing_canonical_node_rejected(self):
        self.rejected(lambda rows: rows[0]["kir_document"]["elements"].pop())

    def test_wrong_metaclass_rejected(self):
        self.rejected(lambda rows: rows[0]["kir_document"]["elements"][1].update(kind="Feature"))

    def test_containment_order_preserved(self):
        def mutate(rows):
            for node in rows[0]["kir_document"]["elements"]:
                if len(node["properties"].get("owned_relationship", [])) > 1:
                    node["properties"]["owned_relationship"].reverse()
                    return
            self.fail("fixture lacks ordered containment")
        self.rejected(mutate)

    def test_wrong_stored_endpoint_rejected(self):
        def mutate(rows):
            for node in rows[0]["kir_document"]["elements"]:
                if node["kind"] == "FeatureTyping":
                    node["properties"]["general"] = ["$"]
                    return
            self.fail("fixture lacks typing")
        self.rejected(mutate)

    def test_stored_attribute_difference_rejected(self):
        self.rejected(lambda rows: rows[0]["kir_document"]["elements"][0]["properties"].update(is_implied_included=True))

    def test_fresh_import_difference_rejected(self):
        self.rejected(lambda rows: rows[0]["roundtrip_kir_document"]["elements"].pop())

    def test_semantic_or_promotion_overclaim_rejected(self):
        for field, value in [("semantic_validation", "passed"), ("transformation_completion", "complete"),
                             ("qualification_certificate", True), ("candidate_promoted", True)]:
            with self.subTest(field=field):
                self.rejected(lambda rows: rows[0].update({field: value}))

    def test_native_view_endpoint_rejected(self):
        self.rejected(lambda rows: rows[0]["reference_views"][0].update(target_ids=[rows[0]["kir_document"]["elements"][1]["id"]]))

    def test_native_view_omission_rejected(self):
        self.rejected(lambda rows: rows[0]["reference_views"].pop())

    def test_native_view_duplicate_rejected(self):
        self.rejected(lambda rows: rows[0]["reference_views"].append(copy.deepcopy(rows[0]["reference_views"][0])))

    def test_source_scope_cannot_be_substituted(self):
        contract = copy.deepcopy(self.contract)
        contract["cases"][0]["source_sha256"] = "0" * 64
        with self.assertRaises(ValueError):
            verify_contract(contract, self.reference)


if __name__ == "__main__":
    unittest.main()
