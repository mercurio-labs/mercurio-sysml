import copy
import json
import unittest
from generate_constructor_constants import SOURCE, ECORE, render

class ConstructorConstantTests(unittest.TestCase):
    def setUp(self):
        self.doc=json.loads(SOURCE.read_bytes());self.ecore=json.loads(ECORE.read_bytes())
    def generate(self): return render(self.doc,self.ecore)
    def test_complete_resolved_constructor_inventory(self):
        self.assertEqual(self.generate().count('    ("'),4)
    def test_missing_constructor_rejects(self):
        self.doc['controls'].pop()
        with self.assertRaisesRegex(ValueError,'Incomplete'): self.generate()
    def test_duplicate_constructor_rejects(self):
        self.doc['controls'].append(copy.deepcopy(self.doc['controls'][0]))
        with self.assertRaisesRegex(ValueError,'Duplicate'): self.generate()
    def test_unassessed_writes_reject(self):
        self.doc['controls'][0]['writes']=['operator']
        with self.assertRaisesRegex(ValueError,'assignment'): self.generate()
    def test_runtime_disagreement_rejects(self):
        self.doc['controls'][0]['observed_value']='changed'
        with self.assertRaisesRegex(ValueError,'disagreement'): self.generate()
    def test_changed_type_rejects(self):
        self.doc['controls'][0]['constant_type']='int'
        with self.assertRaisesRegex(ValueError,'assignment'): self.generate()
    def test_missing_provenance_rejects(self):
        self.doc['provenance']['source_sha256']={}
        with self.assertRaisesRegex(ValueError,'provenance'): self.generate()

if __name__=='__main__': unittest.main()
