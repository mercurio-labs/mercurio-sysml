import unittest,copy,json
from generate_reference_link import INPUT,generate
class ReferenceLinkTests(unittest.TestCase):
 def setUp(self): self.data=json.loads(INPUT.read_text(encoding='utf-8'))
 def test_valid(self): self.assertIn('TransitionPerformances::TransitionPerformance::transitionLink',generate(self.data))
 def test_source_drift(self):
  self.data['constants']['TRANSITION_LINK_FEATURE']['value']='Other::role'
  with self.assertRaises(ValueError):generate(self.data)
 def test_missing_control(self):
  self.data['controls'].pop()
  with self.assertRaises(ValueError):generate(self.data)
 def test_ordering_disagreement(self):
  self.data['controls'][2]['redefinitions'].reverse()
  with self.assertRaises(ValueError):generate(self.data)
if __name__=='__main__':unittest.main()
