import copy,json,unittest
import generate_usage_variability as generation
import run_usage_variability_probe as probe
class Controls(unittest.TestCase):
 def setUp(self):self.data=json.loads(generation.INPUT.read_text())
 def test_complete_matrix_and_semantic_program_guard(self):probe.validate(self.data);self.assertIn('GETTER_BINDINGS',generation.generate(self.data))
 def test_changed_resolved_semantics_reject(self):
  d=copy.deepcopy(self.data);next(iter(d['variability_methods'].values()))['kind']='UNKNOWN'
  with self.assertRaises(ValueError):generation.generate(d)
 def test_missing_context_rejects(self):
  d=copy.deepcopy(self.data);d['controls'].pop()
  with self.assertRaises(ValueError):probe.validate(d)
 def test_fixed_owner_cannot_silently_gain_variability(self):
  d=copy.deepcopy(self.data);c=next(c for c in d['controls'] if c['shape']=='fixed');c['nodes']['a']='Class'
  with self.assertRaises(ValueError):probe.validate(d)
 def test_changed_getter_dispatch_rejects(self):
  d=copy.deepcopy(self.data);next(iter(d['bindings']));d['bindings'][next(iter(d['bindings']))]='unknown'
  with self.assertRaises(ValueError):probe.validate(d)
if __name__=='__main__':unittest.main()
