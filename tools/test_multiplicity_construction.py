import copy,json,unittest
from generate_multiplicity_construction import INPUT,generate
class MultiplicityConstructionTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_resolved_policy_and_complete_matrix(self):self.assertIn('"ViewUsage" => Some(true)',generate(self.data))
 def test_ast_drift_rejected(self):
  self.data['methods'].popitem()
  with self.assertRaisesRegex(ValueError,'algorithms changed'):generate(self.data)
 def test_dispatch_drift_rejected(self):
  self.data['bindings']['ViewUsage']='org.omg.sysml.adapter.UsageAdapter#isAddMultiplicity'
  with self.assertRaisesRegex(ValueError,'dispatch inventory'):generate(self.data)
 def test_missing_or_duplicate_control_rejected(self):
  original=copy.deepcopy(self.data['controls'])
  for cases in (original[:-1],original[:-1]+original[:1]):
   self.data['controls']=cases
   with self.assertRaisesRegex(ValueError,'Incomplete controls'):generate(self.data)
 def test_admission_disagreement_rejected(self):
  self.data['controls'][0]['admitted']=not self.data['controls'][0]['admitted']
  with self.assertRaisesRegex(ValueError,'Reference disagreement'):generate(self.data)
 def test_replay_disagreement_rejected(self):
  self.data['controls'][0]['replay']=2
  with self.assertRaisesRegex(ValueError,'Reference disagreement'):generate(self.data)

 def test_unassessed_custom_lifecycle_cannot_disappear(self):
  self.data['excluded_bindings'].pop('TransitionUsage')
  with self.assertRaisesRegex(ValueError,'excluded dispatch inventory'):generate(self.data)
