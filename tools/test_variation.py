import unittest,json
from generate_variation import INPUT,generate
class VariationTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_valid(self):self.assertIn('ReferenceUsage',generate(self.data))
 def test_resolved_source_drift(self):
  self.data['methods'].pop(next(iter(self.data['methods'])))
  with self.assertRaises(ValueError):generate(self.data)
 def test_inventory_drift(self):
  self.data['usage_kinds'].pop()
  with self.assertRaises(ValueError):generate(self.data)
 def test_pending_disagreement(self):
  next(c for c in self.data['controls'] if c['pending'])['pending']=[]
  with self.assertRaises(ValueError):generate(self.data)
 def test_physical_disagreement(self):
  next(c for c in self.data['controls'] if c['physical_observed'])['physical'][0]['implied']=False
  with self.assertRaises(ValueError):generate(self.data)
if __name__=='__main__':unittest.main()
