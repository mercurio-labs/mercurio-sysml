import json,unittest
from generate_transition_scope import INPUT,OUTPUT,generate
class TransitionScopeTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_current(self):self.assertEqual(generate(self.data),OUTPUT.read_text(encoding='utf-8'))
 def test_method_drift(self):self.data['methods'][0]['name']='changed';self.assertRaises(ValueError,generate,self.data)
 def test_body_drift(self):self.data['methods'][0]['fields']['expression']['kind']='changed';self.assertRaises(ValueError,generate,self.data)
 def test_binding_drift(self):self.data['symbols']['org.omg.sysml.lang.sysml.Membership']['simple_name']='changed';self.assertRaises(ValueError,generate,self.data)
 def test_unresolved(self):self.data['coverage']['unresolved_references']=1;self.assertRaises(ValueError,generate,self.data)
if __name__=='__main__':unittest.main()
