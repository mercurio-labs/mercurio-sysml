import copy,json,unittest
from generate_transition_source_queries import INPUT,ECORE,generate
class SourceQueryTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'));self.ecore=json.loads(ECORE.read_text(encoding='utf-8'))
 def test_complete_source_dispatch(self):self.assertIn('FILTER_TARGET: &str = "ActionUsage"',generate(self.data,self.ecore))
 def test_resolved_algorithm_drift(self):
  self.data['methods'].popitem()
  with self.assertRaisesRegex(ValueError,'algorithms changed'):generate(self.data,self.ecore)
 def test_missing_or_duplicate_controls(self):
  original=copy.deepcopy(self.data['controls'])
  for controls in [original[:-1],original[:-1]+original[:1]]:
   self.data['controls']=controls
   with self.assertRaisesRegex(ValueError,'Incomplete source query matrix'):generate(self.data,self.ecore)
 def test_raw_source_disagreement(self):
  self.data['controls'][0]['raw']='invented'
  with self.assertRaisesRegex(ValueError,'Reference source/no-op disagreement'):generate(self.data,self.ecore)
 def test_feature_target_disagreement(self):
  self.data['controls'][0]['source']='invented'
  with self.assertRaisesRegex(ValueError,'Reference source/no-op disagreement'):generate(self.data,self.ecore)
 def test_noop_cannot_hide_constructor(self):
  c=next(c for c in self.data['controls'] if not c['noop']);c['noop']=True
  with self.assertRaisesRegex(ValueError,'Reference source/no-op disagreement'):generate(self.data,self.ecore)
 def test_producer_mutation_disagreement(self):
  self.data['controls'][0]['members_after']+=1
  with self.assertRaisesRegex(ValueError,'producer mutation disagreement'):generate(self.data,self.ecore)
 def test_imported_operation_contract(self):
  o=next(o for o in self.ecore['operations'] if o['owner'].endswith('#//TransitionUsage') and o['name']=='sourceFeature');o['upper_bound']=-1
  with self.assertRaisesRegex(ValueError,'operation contract'):generate(self.data,self.ecore)
