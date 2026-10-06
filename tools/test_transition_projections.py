import copy,json,unittest
from generate_transition_projections import INPUT,ECORE,generate
class TransitionProjectionTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'));self.ecore=json.loads(ECORE.read_text(encoding='utf-8'))
 def test_complete_dispatch(self):self.assertIn('"guard_expression" => Some(("guard","Expression"',generate(self.data,self.ecore))
 def test_resolved_tree_drift(self):
  self.data['methods'].popitem()
  with self.assertRaisesRegex(ValueError,'algorithms changed'):generate(self.data,self.ecore)
 def test_missing_or_duplicate_matrix_controls(self):
  original=copy.deepcopy(self.data['controls'])
  for controls in [original[:-1],original[:-1]+original[:1]]:
   self.data['controls']=controls
   with self.assertRaisesRegex(ValueError,'Incomplete projection matrix'):generate(self.data,self.ecore)
 def test_reference_getter_disagreement(self):
  c=next(c for c in self.data['controls'] if not c['rejected']);c['result']['trigger_action']=['invented']
  with self.assertRaisesRegex(ValueError,'projection disagreement'):generate(self.data,self.ecore)
 def test_setter_rejection_disagreement(self):
  self.data['controls'][0]['rejected']=not self.data['controls'][0]['rejected']
  with self.assertRaisesRegex(ValueError,'setter disagreement'):generate(self.data,self.ecore)
 def test_imported_type_drift(self):
  f=next(f for f in self.ecore['features'] if f['owner'].endswith('#//TransitionUsage') and f['name']=='triggerAction');f['type']=f['type'].replace('AcceptActionUsage','ActionUsage')
  with self.assertRaisesRegex(ValueError,'Ecore contract'):generate(self.data,self.ecore)
 def test_unresolved_initializer_not_an_enum_default(self):
  self.data['fresh_transition_kind']='trigger'
  with self.assertRaisesRegex(ValueError,'constructor observation'):generate(self.data,self.ecore)
 def test_imported_lower_bound_drift(self):
  f=next(f for f in self.ecore['features'] if f['owner'].endswith('#//TransitionFeatureMembership') and f['name']=='kind');f['lower_bound']=0
  with self.assertRaisesRegex(ValueError,'required enum contract'):generate(self.data,self.ecore)
