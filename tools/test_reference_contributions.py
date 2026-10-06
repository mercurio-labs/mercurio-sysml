import unittest,json
from check_reference_contributions import INPUT,validate
class ContributionTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_valid(self):self.assertEqual(validate(self.data),12)
 def test_source_drift(self):
  self.data['methods'].pop(next(iter(self.data['methods'])))
  with self.assertRaises(ValueError):validate(self.data)
 def test_incomplete_controls(self):
  self.data['controls'].pop()
  with self.assertRaises(ValueError):validate(self.data)
 def test_physical_flags(self):
  self.data['controls'][0]['stored'][0]['implied']=False
  with self.assertRaises(ValueError):validate(self.data)
 def test_false_completion(self):
  self.data['controls'][0]['complete']=True
  with self.assertRaises(ValueError):validate(self.data)
if __name__=='__main__':unittest.main()
