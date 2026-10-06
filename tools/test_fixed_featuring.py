"""Fail-closed cache admission for the fixed featuring batch."""
import copy
import json
import unittest
from run_fixed_featuring_probe import OUTPUT, validate

class FixedFeaturingControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls): cls.controls=json.loads(OUTPUT.read_text(encoding="utf-8"))
    def test_complete_independent_matrix(self): validate(self.controls)
    def test_duplicate_context_cannot_replace_a_binding(self):
        d=copy.deepcopy(self.controls);d['controls'][-1]=d['controls'][0]
        with self.assertRaisesRegex(ValueError,'matrix'): validate(d)
    def test_unknown_class_rejected(self):
        d=copy.deepcopy(self.controls);d['feature_kinds'][0]='InventedFeature'
        with self.assertRaisesRegex(ValueError,'inventory'): validate(d)
    def test_stage_cannot_assert_completion(self):
        d=copy.deepcopy(self.controls);d['controls'][0]['complete']=True
        with self.assertRaisesRegex(ValueError,'completion'): validate(d)
    def test_read_and_physical_stage_disagreement_rejected(self):
        d=copy.deepcopy(self.controls);d['controls'][0]['after']=[]
        with self.assertRaisesRegex(ValueError,'stage'): validate(d)
    def test_unresolved_endpoint_rejected(self):
        d=copy.deepcopy(self.controls);d['controls'][0]['query']=[None];d['controls'][0]['after']=[None]
        with self.assertRaisesRegex(ValueError,'endpoints'): validate(d)

if __name__=='__main__': unittest.main()
