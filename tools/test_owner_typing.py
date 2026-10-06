import copy,json,unittest
from generate_owner_typing import INPUT,OUTPUT,ROOT,generate,nullable
class OwnerTypingTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_current(self):self.assertEqual(generate(self.data),OUTPUT.read_text(encoding='utf-8'));self.assertEqual(nullable(self.data),(ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/ecore_owner_nullable_defaults_generated.rs').read_text(encoding='utf-8'))
 def test_resolved_tree_drift(self):self.data['methods'].pop(next(iter(self.data['methods'])));self.assertRaises(ValueError,generate,self.data)
 def test_dispatch_drift(self):self.data['bindings']['StateUsage']['computeImplicitGeneralTypes']='unresolved';self.assertRaises(ValueError,generate,self.data)
 def test_missing_context(self):self.data['controls'].pop();self.assertRaises(ValueError,generate,self.data)
 def test_implicit_typing(self):self.data['controls'][0]['implicit_after']=['missing'];self.assertRaises(ValueError,generate,self.data)
 def test_completed_receiver(self):self.data['controls'][0]['is_implied_included']=True;self.assertRaises(ValueError,generate,self.data)
 def test_absent_portion_inventory(self):self.data['absent_portion_values'].pop('StateUsage');self.assertRaises(ValueError,nullable,self.data)
 def test_initializer_drift(self):self.data['portion_initialization'].clear();self.assertRaises(ValueError,nullable,self.data)
 def test_constructor_observation_drift(self):self.data['controls'][0]['portion_default_is_null']=False;self.assertRaises(ValueError,nullable,self.data)
if __name__=='__main__':unittest.main()
