import json
import unittest
from export_pilot_definition_defaults import PROFILE, GENERATED, PREFIX, render

class DefinitionDefaultTranslationTests(unittest.TestCase):
    def setUp(self):
        self.doc = json.loads((PROFILE / "definition-defaults.extract.json").read_text(encoding="utf-8"))

    def condition(self):
        return self.doc["methods"][PREFIX + "ConnectionDefinitionAdapter#getDefaultSupertype"]["statements"][0]["expression"]["condition"]

    def test_committed_program_is_generated_from_resolved_trees(self):
        self.assertEqual(render(self.doc), GENERATED.read_text(encoding="utf-8"))
        self.assertEqual(len(self.doc["bindings"]), 26)

    def test_threshold_comes_from_source_expression(self):
        self.condition()["right"]["value"] = 3
        self.assertIn("owned_end_count != 3", render(self.doc))

    def test_unknown_operator_fails_closed(self):
        self.condition()["kind"] = "EQUAL_TO"
        with self.assertRaises(ValueError): render(self.doc)

    def test_changed_resolved_call_is_not_accepted_by_spelling(self):
        self.condition()["left"]["symbol"] = "custom.List#size"
        with self.assertRaises(ValueError): render(self.doc)

    def test_changed_expression_type_fails_closed(self):
        self.condition()["type"] = "int"
        with self.assertRaises(ValueError): render(self.doc)

    def test_nonempty_additional_member_algorithm_is_not_a_signature(self):
        self.doc["methods"][PREFIX + "NamespaceAdapter#addAdditionalMembers"]["statements"] = [{"kind":"RETURN"}]
        with self.assertRaises(ValueError): render(self.doc)

if __name__ == "__main__": unittest.main()
