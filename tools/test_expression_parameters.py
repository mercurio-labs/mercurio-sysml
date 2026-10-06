import copy,json,unittest
import generate_expression_parameters as g
class ExpressionParameters(unittest.TestCase):
 def setUp(self):self.d=json.loads(g.INPUT.read_text())
 def test_current(self):self.assertEqual(g.generate(self.d),g.OUTPUT.read_text())
 def test_general_type_drift(self):
  self.d['methods']['org.omg.sysml.adapter.ExpressionAdapter#getGeneralTypes']['statements'].reverse()
  with self.assertRaisesRegex(ValueError,'program changed'):g.generate(self.d)
 def test_dispatch_drift(self):
  self.d['bindings']['Expression']['getRelevantParameters']='unknown#method'
  with self.assertRaisesRegex(ValueError,'parameter dependency'):g.generate(self.d)
 def test_missing_context(self):
  self.d['controls'].pop()
  with self.assertRaisesRegex(ValueError,'Incomplete'):g.generate(self.d)
 def test_duplicate_context(self):
  self.d['controls'][0]=copy.deepcopy(self.d['controls'][1])
  with self.assertRaisesRegex(ValueError,'Incomplete'):g.generate(self.d)
if __name__=='__main__':unittest.main()
