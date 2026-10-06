"""Admission checks for independently observed resolved binding transactions."""
import copy,json,unittest
from run_resolved_binding_probe import OUTPUT,validate
class ResolvedBindingControls(unittest.TestCase):
 @classmethod
 def setUpClass(cls):cls.data=json.loads(OUTPUT.read_text(encoding='utf-8'))
 def test_complete_matrix(self):validate(self.data)
 def test_duplicate_context_rejects(self):
  d=copy.deepcopy(self.data);d['controls'][-1]=d['controls'][0]
  with self.assertRaisesRegex(ValueError,'matrix'):validate(d)
 def test_untransformed_end_rejects(self):
  d=copy.deepcopy(self.data);d['controls'][0]['ends'][0]['complete']=False
  with self.assertRaisesRegex(ValueError,'transformation'):validate(d)
 def test_library_input_omission_rejects(self):
  d=copy.deepcopy(self.data);d['controls'][0]['library_inputs']={}
  with self.assertRaisesRegex(ValueError,'library'):validate(d)
if __name__=='__main__':unittest.main()
