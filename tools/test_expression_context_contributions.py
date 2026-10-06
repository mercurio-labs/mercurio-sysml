import copy,json,unittest
import check_expression_context_contributions as c
class ContextMatrix(unittest.TestCase):
 def setUp(self):self.d=json.loads(c.INPUT.read_text())
 def test_current(self):c.validate(self.d)
 def test_missing_context(self):
  self.d['controls'].pop()
  with self.assertRaisesRegex(ValueError,'Incomplete'):c.validate(self.d)
 def test_duplicate(self):
  self.d['controls'][0]=copy.deepcopy(self.d['controls'][1])
  with self.assertRaisesRegex(ValueError,'Incomplete'):c.validate(self.d)
 def test_changed_receiver(self):
  self.d['controls'][0]['is_end']=True
  with self.assertRaisesRegex(ValueError,'preconditions'):c.validate(self.d)
 def test_changed_program(self):
  self.d['definitions'].clear()
  with self.assertRaisesRegex(ValueError,'program changed'):c.validate(self.d)
if __name__=='__main__':unittest.main()
