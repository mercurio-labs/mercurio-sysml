import copy,json,unittest
from generate_multiplicity_defaults import INPUT,generate
class MultiplicityGenerationTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_exact_dispatch_and_owner_precedence(self):
  text=generate(self.data)
  self.assertIn('OWNER_CLASSIFIER: &str = "Classifier"',text)
  self.assertIn('if classifier',text)
  self.assertIn('Base::exactlyOne',text)
 def test_changed_semantics_rejected(self):
  self.data['methods']['org.omg.sysml.adapter.MultiplicityAdapter#getRelevantFeatures']['statements'][0]['expression']['symbol']='different'
  with self.assertRaisesRegex(ValueError,'selector changed'):generate(self.data)
 def test_missing_case_and_binding_rejected(self):
  for mutation in ['case','binding']:
   data=copy.deepcopy(self.data)
   if mutation=='case':data['controls'].pop()
   else:data['bindings'].pop('MultiplicityRange')
   with self.assertRaises(ValueError):generate(data)
 def test_changed_relevant_policy_rejected(self):
  self.data['controls'][0]['relevant_count']=1
  with self.assertRaisesRegex(ValueError,'relevant-feature'):generate(self.data)
if __name__=='__main__':unittest.main()
