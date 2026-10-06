"""Guard the bounded resolved contribution program, bindings and control matrix."""
import copy,json,unittest
from generate_action_contributions import INPUT,OUTPUT,generate
class ActionContributions(unittest.TestCase):
 @classmethod
 def setUpClass(cls):cls.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def rejected(self,change):
  d=copy.deepcopy(self.data);change(d)
  with self.assertRaises(ValueError):generate(d)
 def test_current_native_generation(self):self.assertEqual(generate(self.data),OUTPUT.read_text(encoding='utf-8'))
 def test_resolved_program_drift(self):self.rejected(lambda d:d['methods'].pop(next(iter(d['methods']))))
 def test_dispatch_drift(self):self.rejected(lambda d:d['bindings']['TransitionUsage']['dispatch'].__setitem__('getDefaultSupertype','unresolved'))
 def test_default_map_drift(self):self.rejected(lambda d:d['bindings']['ActionUsage']['names'].__setitem__('base','Unreviewed::actions'))
 def test_matrix_missing_context(self):self.rejected(lambda d:d['controls'].pop())
 def test_matrix_duplicate_context(self):self.rejected(lambda d:d['controls'].__setitem__(-1,d['controls'][0]))
 def test_nested_declaration_inventory(self):self.rejected(lambda d:d['owned_member_controls'].pop())
 def test_unknown_relationship(self):self.rejected(lambda d:d['controls'][0]['generals'][0].__setitem__('kind','FeatureTyping'))
if __name__=='__main__':unittest.main()
