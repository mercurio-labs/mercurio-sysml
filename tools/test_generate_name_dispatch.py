import copy
import json
import unittest
from generate_name_dispatch import SOURCE, render

class NameDispatchTests(unittest.TestCase):
    def setUp(self): self.doc = json.loads(SOURCE.read_text(encoding="utf-8"))
    def test_every_resolved_class_has_a_native_dispatch(self):
        result = render(self.doc)
        for kind in self.doc["classes"]: self.assertIn('"'+kind+'" => [', result)
    def test_duplicate_class_identity_rejects(self):
        self.doc["classes"].append(self.doc["classes"][0])
        with self.assertRaisesRegex(ValueError,"Duplicate class"): render(self.doc)
    def test_missing_binding_is_not_a_default(self):
        self.doc["dispatch"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete"): render(self.doc)
    def test_duplicate_binding_rejects(self):
        self.doc["dispatch"].append(copy.deepcopy(self.doc["dispatch"][0]))
        with self.assertRaisesRegex(ValueError,"Duplicate"): render(self.doc)
    def test_unknown_delegate_rejects(self):
        self.doc["dispatch"][0]["delegate"] = "Unassessed"
        with self.assertRaisesRegex(ValueError,"Unassessed"): render(self.doc)
    def test_wrong_operation_cannot_reuse_a_known_algorithm(self):
        self.doc["dispatch"][0]["delegate"] = "org.omg.sysml.delegate.invocation.Element_effectiveShortName_InvocationDelegate"
        with self.assertRaisesRegex(ValueError,"Unassessed"): render(self.doc)
    def test_missing_source_provenance_rejects(self):
        self.doc["provenance"]["source_sha256"] = {}
        with self.assertRaisesRegex(ValueError,"provenance"): render(self.doc)
    def test_changed_pin_rejects(self):
        self.doc["provenance"]["pilot_revision"] = "unknown"
        with self.assertRaisesRegex(ValueError,"pin"): render(self.doc)

if __name__ == "__main__": unittest.main()
