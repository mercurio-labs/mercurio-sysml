"""Reject definition/dispatch/control drift in the bounded State translation."""
import copy,json,unittest
from generate_state_contributions import INPUT,OUTPUT,generate
class StateContributions(unittest.TestCase):
 @classmethod
 def setUpClass(cls):cls.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def rejected(self,change):
  d=copy.deepcopy(self.data);change(d)
  with self.assertRaises(ValueError):generate(d)
 def test_current_native_generation(self):self.assertEqual(generate(self.data),OUTPUT.read_text(encoding='utf-8'))
 def test_program_drift(self):self.rejected(lambda d:d['methods'].pop(next(iter(d['methods']))))
 def test_delegate_dispatch_drift(self):self.rejected(lambda d:d['operation_binding'].__setitem__('selected_delegate','unresolved'))
 def test_contribution_matrix_gap(self):self.rejected(lambda d:d['controls'].pop())
 def test_operation_matrix_duplicate(self):self.rejected(lambda d:d['operation_controls'].__setitem__(-1,d['operation_controls'][0]))
 def test_operation_result_disagreement(self):self.rejected(lambda d:d['operation_controls'][0].__setitem__('result',not d['operation_controls'][0]['result']))
 def test_unknown_contribution(self):self.rejected(lambda d:d['controls'][0]['generals'][0].__setitem__('kind','FeatureTyping'))
 def test_missing_nested_declaration(self):self.rejected(lambda d:d['owned_member_controls'].pop())
if __name__=='__main__':unittest.main()
