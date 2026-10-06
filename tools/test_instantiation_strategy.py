import copy,json,unittest
from generate_instantiation_strategy import INPUT,OUTPUT,generate
class InstantiationStrategyTests(unittest.TestCase):
 def setUp(self):self.d=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_current_native_recipe(self):self.assertEqual(generate(self.d),OUTPUT.read_text(encoding='utf-8'))
 def test_changed_resolved_algorithm_rejects(self):
  self.d['definitions']['unexpected']={}
  with self.assertRaises(ValueError):generate(self.d)
 def test_changed_delegate_rejects(self):
  self.d['bindings']['InvocationExpression']['selected_delegate']='other'
  with self.assertRaises(ValueError):generate(self.d)
 def test_changed_role_rejects(self):
  self.d['bindings']['InvocationExpression']['role_bindings']['base']='Other::base'
  with self.assertRaises(ValueError):generate(self.d)
 def test_changed_classifier_order_rejects(self):
  self.d['generalization_classifier_ids']['Subsetting']=-1
  with self.assertRaises(ValueError):generate(self.d)
 def test_duplicate_control_rejects(self):
  self.d['membership_controls'][1]=copy.deepcopy(self.d['membership_controls'][0])
  with self.assertRaises(ValueError):generate(self.d)
if __name__=='__main__':unittest.main()
