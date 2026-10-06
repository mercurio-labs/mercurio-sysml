"""Reject incomplete or altered bounded ancestry/featuring observations."""
import copy,json,unittest
from run_explicit_specialization_probe import OUTPUT,validate
class ExplicitSpecializationControls(unittest.TestCase):
 @classmethod
 def setUpClass(cls):cls.data=json.loads(OUTPUT.read_text(encoding='utf-8'))
 def test_complete_matrix(self):validate(self.data)
 def rejected(self,change):
  d=copy.deepcopy(self.data);change(d)
  with self.assertRaises(ValueError):validate(d)
 def test_duplicate_context(self):self.rejected(lambda d:d['controls'].__setitem__(-1,d['controls'][0]))
 def test_changed_graph(self):self.rejected(lambda d:d['controls'][0]['edges'].append(['goal','root']))
 def test_completed_input(self):self.rejected(lambda d:d['controls'][0].__setitem__('complete',True))
 def test_missing_featuring(self):self.rejected(lambda d:d['featuring_controls'].pop())
 def test_physical_disagreement(self):self.rejected(lambda d:d['featuring_controls'][0].__setitem__('after',[]))
 def test_variable_context(self):self.rejected(lambda d:d['featuring_controls'][0].__setitem__('variable',True))
if __name__=='__main__':unittest.main()
