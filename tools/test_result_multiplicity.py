import copy,json,unittest
import generate_result_multiplicity as generation
import run_result_multiplicity_probe as probe
class Controls(unittest.TestCase):
 def setUp(self):self.data=json.loads(generation.INPUT.read_text())
 def test_complete_matrix_and_generated_guard(self):probe.validate(self.data);self.assertEqual(generation.generate(self.data),generation.OUTPUT.read_text())
 def test_changed_resolved_method_rejects(self):
  d=copy.deepcopy(self.data);next(iter(d['methods'].values()))['kind']='UNKNOWN'
  with self.assertRaises(ValueError):generation.generate(d)
 def test_missing_shape_rejects(self):
  d=copy.deepcopy(self.data);d['controls'].pop()
  with self.assertRaises(AssertionError):probe.validate(d)
if __name__=='__main__':unittest.main()
