import copy,json,unittest
from generate_occurrence_contributions import INPUT,ECORE,Translator,generate,nullable_defaults
class OccurrenceContributionTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_complete_resolved_translation(self):
  code=generate(self.data);self.assertIn('"ViewUsage"',code);self.assertIn('c.owner_is("ViewDefinition")',code)
 def test_method_drift_rejected(self):
  self.data['methods'].pop('org.omg.sysml.adapter.FeatureAdapter#addDefaultGeneralType')
  with self.assertRaisesRegex(ValueError,'algorithms changed'):generate(self.data)
 def test_dispatch_or_mapping_drift_rejected(self):
  self.data['bindings']['ViewUsage']['names']['subview']='Wrong::target'
  with self.assertRaisesRegex(ValueError,'binding/default'):generate(self.data)
 def test_missing_or_duplicate_control_rejected(self):
  original=copy.deepcopy(self.data['controls'])
  for rows in (original[:-1],original[:-1]+original[:1]):
   self.data['controls']=rows
   with self.assertRaisesRegex(ValueError,'Incomplete contribution matrix'):generate(self.data)
 def test_contribution_disagreement_rejected(self):
  self.data['controls'][0]['generals']=[]
  with self.assertRaisesRegex(ValueError,'contribution disagreement'):generate(self.data)
 def test_unassessed_predicate_rejected(self):
  with self.assertRaisesRegex(ValueError,'Unassessed predicate node'):Translator(self.data,self.data['bindings']['ViewUsage']).expression({'kind':'ASSIGNMENT'})
 def test_nullable_initialization_drift_rejected(self):
  self.data['portion_initialization']['PORTION_KIND_EDEFAULT']['initializer']['value']='timeslice'
  with self.assertRaisesRegex(ValueError,'portion initialization'):generate(self.data)
 def test_absent_observation_disagreement_rejected(self):
  self.data['absent_portion_values']['ViewUsage']='timeslice'
  with self.assertRaisesRegex(ValueError,'portion observation'):generate(self.data)
 def test_nullable_default_does_not_replace_ecore_metadata(self):
  ecore=json.loads(ECORE.read_text(encoding='utf-8'));before=copy.deepcopy(ecore);code=nullable_defaults(self.data,ecore);self.assertIn('"ViewUsage","portion_kind"',code);self.assertEqual(ecore,before)
 def test_nullable_ecore_shape_drift_rejected(self):
  ecore=json.loads(ECORE.read_text(encoding='utf-8'));next(f for f in ecore['features'] if f['name']=='portionKind')['lower_bound']=1
  with self.assertRaisesRegex(ValueError,'nullable enum contract'):nullable_defaults(self.data,ecore)
