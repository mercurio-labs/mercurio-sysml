import copy
import json
import unittest
from generate_required_feature_dependencies import SOURCE, ECORE, render

class RequiredFeatureDependencyTests(unittest.TestCase):
    def setUp(self):
        self.doc = json.loads(SOURCE.read_bytes())
        self.ecore = json.loads(ECORE.read_bytes())
    def generate(self): return render(self.doc,self.ecore)
    def test_complete_inventory_identifies_constructor_dependencies(self):
        self.assertEqual(self.generate().count('    ("'),4)
        for kind in ['CollectExpression','FeatureChainExpression','IndexExpression','SelectExpression']:
            self.assertIn(f'("{kind}", "operator")',self.generate())
    def test_missing_observation_rejects(self):
        self.doc['controls'].pop()
        with self.assertRaisesRegex(ValueError,'Incomplete'): self.generate()
    def test_duplicate_observation_rejects(self):
        self.doc['controls'].append(copy.deepcopy(self.doc['controls'][0]))
        with self.assertRaisesRegex(ValueError,'Duplicate'): self.generate()
    def test_unknown_feature_rejects(self):
        self.doc['controls'][0]['feature']='unknown'
        with self.assertRaisesRegex(ValueError,'Unknown'): self.generate()
    def test_ambiguous_outcome_rejects(self):
        self.doc['controls'][0].update(valid=True,error='error')
        with self.assertRaisesRegex(ValueError,'outcome'): self.generate()
    def test_changed_pin_rejects(self):
        self.doc['provenance']['pilot_revision']='unknown'
        with self.assertRaisesRegex(ValueError,'pin'): self.generate()
    def test_error_does_not_become_a_missing_value(self):
        row=next(r for r in self.doc['controls'] if r['class']=='CollectExpression' and r['feature']=='operator')
        row.pop('valid');row['error']='unresolved'
        self.assertIn('("CollectExpression", "operator")',self.generate())

if __name__ == '__main__': unittest.main()
