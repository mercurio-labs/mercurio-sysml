import copy,json,unittest
from generate_library_instantiation import INPUT,OUTPUT,generate
class LibraryInstantiationControls(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_current(self):self.assertEqual(generate(self.data),OUTPUT.read_text(encoding='utf-8'))
 def test_typed_program_drift(self):
  self.data['definitions']['changed']={}
  with self.assertRaises(ValueError):generate(self.data)
 def test_package_priority_drift(self):
  self.data['operator_packages'].reverse()
  with self.assertRaises(ValueError):generate(self.data)
 def test_constructor_default_drift(self):
  self.data['defaults']['IndexExpression']='changed'
  with self.assertRaises(ValueError):generate(self.data)
 def test_trigger_role_drift(self):
  self.data['trigger_roles']['when']='wrong'
  with self.assertRaises(ValueError):generate(self.data)
 def test_duplicate_matrix(self):
  self.data['controls'][0]=copy.deepcopy(self.data['controls'][1])
  with self.assertRaises(ValueError):generate(self.data)
if __name__=='__main__':unittest.main()
