import copy
import json
import unittest
from export_pilot_connector_defaults import OUTPUT, expected_selector, render, validate


class ConnectorDefaultTests(unittest.TestCase):
    def setUp(self):
        self.doc = json.loads(OUTPUT.read_text(encoding="utf-8"))

    def test_pinned_resolved_selector_and_observations(self):
        validate(self.doc)
        self.assertEqual(len(self.doc["controls"]),36)
        self.assertEqual(self.doc["methods"]["selector"],expected_selector())
        generated = render(self.doc)
        for name in self.doc["defaults"].values():
            self.assertIn(json.dumps(name),generated)

    def test_changed_comparison_is_rejected(self):
        expression = self.doc["methods"]["selector"]["statements"][2]["expression"]
        for branch in ("true","false"):
            before = copy.deepcopy(expression[branch]["condition"])
            expression[branch]["condition"]["kind"] = "EQUAL_TO"
            with self.assertRaisesRegex(ValueError,"selector AST"):
                render(self.doc)
            expression[branch]["condition"] = before

    def test_changed_constant_and_branch_are_rejected(self):
        expression = self.doc["methods"]["selector"]["statements"][2]["expression"]
        expression["true"]["condition"]["right"]["value"] = 3
        with self.assertRaisesRegex(ValueError,"selector AST"):
            render(self.doc)
        expression["true"]["condition"]["right"]["value"] = 2
        expression["true"]["true"]["arguments"][0]["value"] = "base"
        with self.assertRaisesRegex(ValueError,"selector AST"):
            render(self.doc)

    def test_changed_resolved_method_and_extra_statements_rejected(self):
        statements = self.doc["methods"]["selector"]["statements"]
        original = copy.deepcopy(statements)
        statements[1]["initializer"]["receiver"]["symbol"] = "org.omg.sysml.util.TypeUtil#getEndFeatureOf"
        with self.assertRaisesRegex(ValueError,"selector AST"):
            render(self.doc)
        self.doc["methods"]["selector"]["statements"] = original + original[:1]
        with self.assertRaisesRegex(ValueError,"selector AST"):
            render(self.doc)

    def test_changed_inherited_dependencies_rejected(self):
        for name in self.doc["methods"]["dispatch"]:
            before = self.doc["methods"]["dispatch"][name]
            self.doc["methods"]["dispatch"][name] = "unassessed#"+name
            with self.assertRaisesRegex(ValueError,"dependencies"):
                render(self.doc)
            self.doc["methods"]["dispatch"][name] = before

    def test_missing_duplicate_or_disagreeing_control_rejected(self):
        original = copy.deepcopy(self.doc["controls"])
        for replacement in (original[:-1],original[:-1]+original[:1]):
            self.doc["controls"] = replacement
            with self.assertRaisesRegex(ValueError,"inventory"):
                render(self.doc)
        self.doc["controls"] = original
        self.doc["controls"][0]["default_supertype"] = "wrong"
        with self.assertRaisesRegex(ValueError,"disagrees"):
            render(self.doc)

    def test_no_subclass_admission(self):
        self.doc["binding"] = "BindingConnector"
        with self.assertRaisesRegex(ValueError,"binding"):
            render(self.doc)

    def test_binding_connector_dispatch_and_map(self):
        row=self.doc["binding_connector"]
        self.assertEqual(len(row["controls"]),36)
        self.assertEqual(row["defaults"]["binary"],"Links::selfLinks")
        self.assertIn("BindingConnector",render(self.doc))
        row["methods"]["dispatch"]["getDefaultSupertype"]="unassessed#selector"
        with self.assertRaisesRegex(ValueError,"dependencies"):render(self.doc)

    def test_unresolved_or_changed_typing_dispatch_is_rejected(self):
        for binding in [self.doc, self.doc["binding_connector"]]:
            for name in ["getAllTypes", "getTypes", "getFeatureTypes"]:
                original = binding["methods"]["typing_dispatch"].pop(name)
                with self.assertRaisesRegex(ValueError, "typing dependencies"):
                    render(self.doc)
                binding["methods"]["typing_dispatch"][name] = "unassessed#" + name
                with self.assertRaisesRegex(ValueError, "typing dependencies"):
                    render(self.doc)
                binding["methods"]["typing_dispatch"][name] = original

    def test_missing_binding_context_is_rejected(self):
        self.doc["binding_connector"]["controls"].pop()
        with self.assertRaisesRegex(ValueError,"inventory"):render(self.doc)


if __name__ == "__main__":
    unittest.main()
