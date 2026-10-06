"""Resolved program/binding and boundary-matrix invalidation controls."""
import copy,json,unittest
import generate_feature_chain_members as generation
import run_feature_chain_members_probe as probe
class Controls(unittest.TestCase):
 def setUp(self):self.data=json.loads(generation.INPUT.read_text(encoding='utf-8'))
 def test_complete_reference_matrix_and_generated_constants(self):probe.validate(self.data);self.assertEqual(generation.generate(self.data),generation.OUTPUT.read_text(encoding='utf-8'))
 def test_changed_method_is_rejected(self):
  d=copy.deepcopy(self.data);next(iter(d['methods'].values()))['type']='Unresolved'
  with self.assertRaises(ValueError):generation.generate(d)
 def test_changed_binding_is_rejected(self):
  d=copy.deepcopy(self.data);d['bindings']['FeatureChainExpression']='DifferentProducer'
  with self.assertRaises(ValueError):generation.generate(d)
 def test_missing_boundary_control_is_rejected(self):
  d=copy.deepcopy(self.data);d['controls'].pop()
  with self.assertRaises(AssertionError):probe.validate(d)
if __name__=='__main__':unittest.main()
