import copy
import json
import unittest
from generate_value_converter_dispatch import SOURCE, PROFILE, render

class ValueConverterDispatchTests(unittest.TestCase):
    def setUp(self):
        self.doc = json.loads(SOURCE.read_bytes())
        self.grammar = json.loads((PROFILE / "grammar.structure.extract.json").read_bytes())
        self.ecore = json.loads((PROFILE / "ecore-effective.extract.json").read_bytes())
    def generate(self): return render(self.doc, self.grammar, self.ecore)
    def test_complete_import_resolves(self):
        self.assertEqual(self.generate().count(" => "), 176)
    def test_missing_binding_is_not_an_implicit_default(self):
        self.doc["bindings"].pop()
        with self.assertRaisesRegex(ValueError, "Incomplete"): self.generate()
    def test_duplicate_binding_rejects(self):
        self.doc["bindings"].append(copy.deepcopy(self.doc["bindings"][0]))
        with self.assertRaisesRegex(ValueError, "Duplicate"): self.generate()
    def test_unassessed_converter_rejects(self):
        self.doc["bindings"][0]["converter"] = "Unknown"
        with self.assertRaisesRegex(ValueError, "Unassessed"): self.generate()
    def test_unassessed_injected_service_rejects(self):
        self.doc["bindings"][0]["service"] = "Unknown"
        with self.assertRaisesRegex(ValueError, "Unassessed"): self.generate()
    def test_changed_return_type_rejects(self):
        self.doc["bindings"][0]["datatype"] = "Unknown"
        with self.assertRaisesRegex(ValueError, "type"): self.generate()
    def test_changed_pin_rejects(self):
        self.doc["provenance"]["pilot_revision"] = "Unknown"
        with self.assertRaisesRegex(ValueError, "pin"): self.generate()

if __name__ == "__main__": unittest.main()
