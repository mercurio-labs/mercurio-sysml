"""Fail closed when the pinned snapshot matrix or constructor outputs drift."""
import copy,json,unittest
from run_snapshot_featuring_probe import OUTPUT,validate
class SnapshotFeaturingControls(unittest.TestCase):
 @classmethod
 def setUpClass(cls):cls.data=json.loads(OUTPUT.read_text(encoding='utf-8'))
 def rejected(self,change):
  d=copy.deepcopy(self.data);change(d)
  with self.assertRaises(ValueError):validate(d)
 def test_complete_matrix(self):validate(self.data)
 def test_duplicate_context(self):self.rejected(lambda d:d['controls'].__setitem__(-1,d['controls'][0]))
 def test_completed_candidate(self):self.rejected(lambda d:d['controls'][0].__setitem__('candidate_complete',True))
 def test_variable_derivation(self):self.rejected(lambda d:d['controls'][0].__setitem__('variable',False))
 def test_changed_name(self):self.rejected(lambda d:d['controls'][0].__setitem__('target_name','invented'))
 def test_wrong_context(self):self.rejected(lambda d:d['controls'][0].__setitem__('context_is_owner',[False]))
 def test_wrong_redefinition(self):self.rejected(lambda d:d['controls'][0].__setitem__('redefined',[]))
 def test_physical_disagreement(self):self.rejected(lambda d:d['controls'][0].__setitem__('after',[]))
if __name__=='__main__':unittest.main()
