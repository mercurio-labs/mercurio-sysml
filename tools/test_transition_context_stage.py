import copy,json,unittest
from export_transition_context_stage import OUTPUT,render
class ContextStageTests(unittest.TestCase):
 def setUp(self):self.doc=json.loads(OUTPUT.read_text(encoding='utf-8'))
 def test_complete_stage_export(self):self.assertIn('PROGRAM_SHA',render(self.doc))
 def test_changed_guard_rejected(self):
  self.doc['definitions']['org.omg.sysml.adapter.ElementAdapter#transform']['kind']='EMPTY_STATEMENT'
  with self.assertRaisesRegex(ValueError,'Changed resolved'):render(self.doc)
 def test_duplicate_control_cannot_replace_boundary(self):
  self.doc['controls'][0]=copy.deepcopy(self.doc['controls'][1])
  with self.assertRaisesRegex(ValueError,'Incomplete'):render(self.doc)
 def test_missing_negative_parent_context_rejected(self):
  self.doc['controls']=[r for r in self.doc['controls'] if r['parent']!='Namespace']
  with self.assertRaisesRegex(ValueError,'Incomplete'):render(self.doc)
 def test_stage_controls_cannot_claim_completion(self):
  self.doc['controls'][0]['completion_flags']=True
  with self.assertRaisesRegex(ValueError,'Unproved'):render(self.doc)
