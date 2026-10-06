import copy,json,unittest
from export_no_owning_type_featuring import OUTPUT,render
class NoOwningTypeFeaturingTests(unittest.TestCase):
 def setUp(self):self.doc=json.loads(OUTPUT.read_text(encoding='utf-8'))
 def test_resolved_dispatch_consumed(self):
  text=render(self.doc);self.assertIn('"ReferenceUsage"',text);self.assertNotIn('"Connector"',text);self.assertNotIn('"Expression"',text)
 def test_resolved_program_change_rejected(self):
  self.doc['definitions']['FeatureAdapter#computeFeaturingType']['kind']='EMPTY_STATEMENT'
  with self.assertRaisesRegex(ValueError,'Changed resolved'):render(self.doc)
 def test_missing_class_rejected(self):
  self.doc['bindings'].pop('ReferenceUsage')
  with self.assertRaisesRegex(ValueError,'inventory'):render(self.doc)
 def test_unknown_producer_rejected(self):
  self.doc['bindings']['ReferenceUsage']['owning_type_provider']='other#computeFeaturingType'
  with self.assertRaisesRegex(ValueError,'Unknown owning'):render(self.doc)
 def test_specialized_context_cannot_be_admitted(self):
  self.doc['bindings']['Connector']['specialized_context']=False
  with self.assertRaisesRegex(ValueError,'ancestry mismatch'):render(self.doc)
 def test_duplicate_case_cannot_replace_missing_boundary(self):
  self.doc['controls'][0]=copy.deepcopy(self.doc['controls'][1])
  with self.assertRaisesRegex(ValueError,'matrix'):render(self.doc)
