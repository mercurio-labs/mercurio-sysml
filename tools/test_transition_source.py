import copy,json,unittest
from generate_transition_source import INPUT,generate
class TransitionSourceTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_policy_complete_matrix(self):self.assertIn('MEMBERSHIP: &str = "Membership"',generate(self.data))
 def test_resolved_algorithm_drift_rejected(self):
  self.data['methods'].popitem()
  with self.assertRaisesRegex(ValueError,'algorithms changed'):generate(self.data)
 def test_missing_control_rejected(self):
  self.data['controls'].pop()
  with self.assertRaisesRegex(ValueError,'Incomplete source matrix'):generate(self.data)
 def test_duplicate_control_rejected(self):
  self.data['controls'][-1]=copy.deepcopy(self.data['controls'][0])
  with self.assertRaisesRegex(ValueError,'Incomplete source matrix'):generate(self.data)
 def test_reference_disagreement_rejected(self):
  self.data['controls'][0]['after'][0]['member']='invented'
  with self.assertRaisesRegex(ValueError,'Reference source disagreement'):generate(self.data)
 def test_replay_disagreement_rejected(self):
  self.data['controls'][0]['replay']=[]
  with self.assertRaisesRegex(ValueError,'Reference source disagreement'):generate(self.data)
