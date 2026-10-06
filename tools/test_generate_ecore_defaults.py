import copy
import json
import unittest
from generate_ecore_defaults import INPUT, EVIDENCE, generate

class EcoreDefaultReadGenerationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc=json.loads(INPUT.read_text(encoding="utf-8"))
        cls.evidence=json.loads(EVIDENCE.read_text(encoding="utf-8"))
    def test_known_runtime_dependencies_are_preserved(self):
        output,count=generate(self.doc,self.evidence)
        self.assertEqual(count,24)
        self.assertIn(b'("NamespaceExpose", "is_import_all")',output)
        self.assertIn(b'("PartUsage", "is_variable")',output)
        self.assertIn(b'Constructor dependency evidence SHA256',output)
    def test_unknown_dependency_class_is_rejected(self):
        evidence=copy.deepcopy(self.evidence)
        evidence['constructor_overrides'][0]['class']='Unknown'
        with self.assertRaisesRegex(ValueError,'dependency class'): generate(self.doc,evidence)
    def test_unknown_dependency_attribute_is_rejected(self):
        evidence=copy.deepcopy(self.evidence)
        evidence['constructor_overrides'][0]['feature']='nonexistent'
        with self.assertRaisesRegex(ValueError,'dependency attribute'): generate(self.doc,evidence)
    def test_resolved_default_disagreement_is_rejected(self):
        doc=copy.deepcopy(self.doc)
        feature=next(f for f in doc['features'] if f['default_literal']=='true')
        feature['default_value']=False
        with self.assertRaisesRegex(ValueError,'default value mismatch'): generate(doc,self.evidence)

if __name__=='__main__': unittest.main()
