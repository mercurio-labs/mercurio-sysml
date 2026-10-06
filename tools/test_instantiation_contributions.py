import copy,json,unittest
from generate_instantiation_contributions import INPUT,OUTPUT,generate
class InstantiationContributionTests(unittest.TestCase):
 def setUp(self):self.d=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_current(self):self.assertEqual(generate(self.d),OUTPUT.read_text(encoding='utf-8'))
 def test_resolved_program_drift(self):
  self.d['definitions']['changed']={}
  with self.assertRaises(ValueError):generate(self.d)
 def test_adapter_dispatch_drift(self):
  self.d['bindings']['IndexExpression']='unreviewed'
  with self.assertRaises(ValueError):generate(self.d)
 def test_duplicate_case(self):
  self.d['controls'][0]=copy.deepcopy(self.d['controls'][1])
  with self.assertRaises(ValueError):generate(self.d)
 def test_missing_package_context(self):
  self.d['controls']=[c for c in self.d['controls'] if c['context']!='package']
  with self.assertRaises(ValueError):generate(self.d)
 def test_missing_equivalence(self):
  self.d['equivalence_controls'].pop()
  with self.assertRaises(ValueError):generate(self.d)
if __name__=='__main__':unittest.main()
