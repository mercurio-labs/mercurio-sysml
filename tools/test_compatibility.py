import json
import unittest
from export_pilot_compatibility import OUTPUT, render, normalize_capture_ids
class CompatibilityTests(unittest.TestCase):
    def setUp(self): self.doc=json.loads(OUTPUT.read_text())
    def test_resolved_definition_guard(self): self.assertIn("DEFINITION_SHA",render(self.doc))
    def test_missing_context(self):
        self.doc["cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete compatibility"):render(self.doc)
    def test_changed_semantics(self):
        self.doc["definitions"].pop("Feature_isFeaturedWithin_InvocationDelegate#dynamicInvoke")
        with self.assertRaisesRegex(ValueError,"Changed resolved"):render(self.doc)
    def test_invalid_observation(self):
        self.doc["cases"][0]["compatible"]=None
        with self.assertRaisesRegex(ValueError,"Invalid compatibility"):render(self.doc)
    def test_missing_connector_context(self):
        self.doc["context_cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete connector"):render(self.doc)
    def test_nested_global_disagreements_are_preserved(self):
        disagreements={r["shape"] for r in self.doc["cases"] if r["shape"].startswith(("nested_","global_")) and r["compatible"]!=r["can_access"]}
        self.assertEqual(disagreements,{"nested_positive","nested_diamond","global_positive","global_empty","nested_global"})

    def test_missing_placement_rejected(self):
        self.doc["context_cases"][0]["placements"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete binding placements"):render(self.doc)

    def test_invalid_placement_kind_rejected(self):
        self.doc["context_cases"][0]["placements"][0]["membership"]="Membership"
        with self.assertRaisesRegex(ValueError,"Invalid binding placement"):render(self.doc)

    def test_missing_transformed_binding_control(self):
        self.doc["binding_stages"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete transformed binding"):render(self.doc)

    def test_missing_transformed_end(self):
        self.doc["binding_stages"][0]["ends"].pop()
        with self.assertRaisesRegex(ValueError,"Invalid transformed binding"):render(self.doc)

    def test_capture_alpha_renaming_is_stable(self):
        a={"x":{"type":"(capture#90, capture#91, capture#90)"}}
        b={"x":{"type":"(capture#11, capture#72, capture#11)"}}
        self.assertEqual(normalize_capture_ids(a),normalize_capture_ids(b))
        b["x"]["type"]="(capture#11, capture#11, capture#11)"
        self.assertNotEqual(normalize_capture_ids(a),normalize_capture_ids(b))

    def test_capture_normalization_preserves_semantic_text(self):
        value={"type":"List<capture#99 extends Feature>","symbol":"capture#99"}
        self.assertEqual(normalize_capture_ids(value),{"type":"List<capture#0 extends Feature>","symbol":"capture#99"})

    def test_missing_subtree_rejected(self):
        self.doc["binding_stages"][0].pop("stored_subtree")
        with self.assertRaisesRegex(ValueError,"Incomplete stored binding"):render(self.doc)

    def test_duplicate_subtree_path_rejected(self):
        rows=self.doc["binding_stages"][0]["stored_subtree"]
        rows.append(rows[0])
        with self.assertRaisesRegex(ValueError,"Invalid stored binding"):render(self.doc)

    def test_missing_inclusion_context_rejected(self):
        self.doc["inclusion_cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete implied inclusion"):render(self.doc)

    def test_invalid_inclusion_issue_rejected(self):
        self.doc["inclusion_cases"][0]["issues"]=["unknown"]
        with self.assertRaisesRegex(ValueError,"Invalid implied inclusion"):render(self.doc)

    def test_missing_complete_document_rejected(self):
        self.doc["binding_stages"][0].pop("stored_document")
        with self.assertRaisesRegex(ValueError,"Incomplete transformed document"):render(self.doc)

    def test_duplicate_complete_document_node_rejected(self):
        rows=self.doc["binding_stages"][0]["stored_document"]
        rows.append(rows[0])
        with self.assertRaisesRegex(ValueError,"Invalid transformed document"):render(self.doc)

    def test_missing_value_producer_control_rejected(self):
        self.doc["value_cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete bound-value"):render(self.doc)

    def test_invalid_value_producer_observation_rejected(self):
        self.doc["value_cases"][0]["contributions"]=[{"kind":"Subsetting","chain":["result","expression"]}]
        with self.assertRaisesRegex(ValueError,"Invalid bound-value"):render(self.doc)

    def test_missing_expression_featuring_case(self):
        self.doc["expression_featuring_cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete expression featuring"): render(self.doc)

    def test_changed_expression_featuring_dispatch(self):
        self.doc["expression_featuring_cases"][0]["implementation"]="unknown"
        with self.assertRaisesRegex(ValueError,"Unassessed expression featuring"): render(self.doc)

    def test_changed_expression_default_selector(self):
        self.doc["expression_default_definitions"]["getDefaultSupertype"]["children"][0]["children"][0]["children"][1]["value"]="unknown"
        with self.assertRaisesRegex(ValueError,"Changed resolved expression selector"):render(self.doc)

    def test_changed_expression_default_contributions(self):
        self.doc["expression_default_definitions"]["addDefaultGeneralType"]["children"].pop()
        with self.assertRaisesRegex(ValueError,"Changed resolved expression contributions"):render(self.doc)

    def test_missing_expression_default_control(self):
        self.doc["expression_defaults"]["cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete expression default controls"):render(self.doc)

    def test_changed_expression_default_observation(self):
        self.doc["expression_defaults"]["cases"][0]["generals"]=[]
        with self.assertRaisesRegex(ValueError,"Unassessed expression default contribution"):render(self.doc)

    def test_changed_invariant_selector_rejected(self):
        self.doc["expression_default_definitions"]["invariant_selector"]["children"]=[]
        with self.assertRaisesRegex(ValueError,"Changed resolved expression selector tree"):render(self.doc)

    def test_changed_selector_dispatch_rejected(self):
        self.doc["expression_defaults"]["selectors"]["Invariant"]="org.omg.sysml.adapter.ExpressionAdapter"
        with self.assertRaisesRegex(ValueError,"Unassessed expression selector dispatch"):render(self.doc)

    def test_missing_expression_library_result_rejected(self):
        self.doc["expression_defaults"].pop("library_result_case")
        with self.assertRaisesRegex(ValueError,"Unassessed expression library result"):render(self.doc)

    def test_missing_result_default_control(self):
        self.doc["expression_defaults"]["result_defaults"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete result default controls"):render(self.doc)

    def test_missing_expression_featuring_query(self):
        self.doc["expression_featuring_cases"][0].pop("query")
        with self.assertRaisesRegex(ValueError,"Missing expression featuring query"):render(self.doc)

    def test_changed_reference_binding_endpoints(self):
        self.doc["expression_defaults"]["library_result_case"]["reference_binding"][0]["related"].reverse()
        with self.assertRaisesRegex(ValueError,"Unassessed reference binding placement"):render(self.doc)

    def test_missing_expression_owned_binding_control(self):
        self.doc["expression_defaults"]["binding_defaults"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete expression-owned binding controls"):render(self.doc)

    def test_missing_inherited_binding_evidence(self):
        self.doc["expression_defaults"].pop("inherited_binding_case")
        with self.assertRaisesRegex(ValueError,"Missing inherited binding semantic evidence"):render(self.doc)
