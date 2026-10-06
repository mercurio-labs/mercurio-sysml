import copy,json,unittest
from check_reference_lifecycle import INPUT,validate
class ReferenceLifecycleTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_complete_cache(self):validate(self.data)
 def test_missing_matrix_control(self):
  self.data['controls'].pop()
  with self.assertRaisesRegex(ValueError,'matrix'):validate(self.data)
 def test_resolved_program_drift(self):
  self.data['constants']['TRANSITION_LINK_FEATURE']['value']='Other::role'
  with self.assertRaisesRegex(ValueError,'resolved Reference'):validate(self.data)
 def test_inherited_typing_never_selects_default(self):
  row=next(r for r in self.data['controls'] if r['owned_mask']==0 and r['role_mask']==2);row['default_supertype']='Objects::objects'
  with self.assertRaisesRegex(ValueError,'Default observation'):validate(self.data)
 def test_missing_computed_role_type(self):
  row=next(r for r in self.data['controls'] if r['owned_mask']==0 and r['role_mask']==1);row['types']=[]
  with self.assertRaisesRegex(ValueError,'Inherited type'):validate(self.data)
 def test_false_completion_claim(self):
  self.data['controls'][0]['candidate_complete_after_query']=True
  with self.assertRaisesRegex(ValueError,'fresh selected'):validate(self.data)
 def test_changed_fixture(self):
  self.data['controls'][0]['source']+=' '
  with self.assertRaisesRegex(ValueError,'fixture'):validate(self.data)
