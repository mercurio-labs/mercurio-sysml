"""Independent cache/schema and resolved-program invalidation controls."""
import copy,json,unittest
from pathlib import Path
import generate_relative_namespace as generation
import run_relative_namespace_probe as probe
class RelativeNamespaceControls(unittest.TestCase):
 def setUp(self):self.data=json.loads(generation.INPUT.read_text(encoding='utf-8'))
 def test_pinned_matrix_and_generated_program(self):
  probe.validate(self.data);self.assertEqual(generation.generate(self.data),generation.OUTPUT.read_text(encoding='utf-8'))
 def test_changed_resolved_symbol_is_rejected(self):
  changed=copy.deepcopy(self.data);next(iter(changed['methods'].values()))['type']='Unresolved'
  with self.assertRaises(ValueError):generation.generate(changed)
 def test_missing_method_is_rejected(self):
  changed=copy.deepcopy(self.data);changed['methods'].pop(next(iter(changed['methods'])))
  with self.assertRaises(ValueError):generation.generate(changed)
 def test_missing_boundary_control_is_rejected(self):
  changed=copy.deepcopy(self.data);changed['controls'].pop()
  with self.assertRaises(AssertionError):probe.validate(changed)
if __name__=='__main__':unittest.main()
