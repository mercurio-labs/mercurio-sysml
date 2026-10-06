import copy
import json
import unittest
import generate_expression_redefinitions as generator

class ExpressionGenerationTests(unittest.TestCase):
    def setUp(self):
        self.data = json.loads(generator.INPUT.read_text(encoding='utf-8'))
        self.bindings = json.loads(generator.BINDINGS.read_text(encoding='utf-8'))

    def test_capture_identity_is_not_semantic(self):
        changed = copy.deepcopy(self.data)
        node = changed['methods']['org.omg.sysml.adapter.ExpressionAdapter#getRelevantFeatures']['statements'][2]['expression']['false']['false']
        node['type'] = node['type'].replace('capture#686', 'capture#999')
        self.assertEqual(generator.generate(changed, self.bindings), generator.generate(self.data, self.bindings))

    def test_changed_resolved_algorithm_rejected(self):
        changed = copy.deepcopy(self.data)
        changed['methods']['org.omg.sysml.adapter.ExpressionAdapter#getRelevantFeatures']['statements'][2]['expression']['false']['false']['symbol'] = 'other#getRelevantFeatures'
        with self.assertRaisesRegex(ValueError, 'tree changed'):
            generator.generate(changed, self.bindings)

    def test_overridden_inherited_algorithm_rejected(self):
        changed = copy.deepcopy(self.bindings)
        row = next(r for r in changed['bindings'] if r['kind'] == 'Invariant')
        row['methods']['addComputedRedefinitions'] = 'other#addComputedRedefinitions'
        with self.assertRaisesRegex(ValueError, 'Nonordinary'):
            generator.generate(self.data, changed)

    def test_duplicate_control_cannot_hide_missing_context(self):
        for key in ('controls', 'guard_controls'):
            changed = copy.deepcopy(self.data)
            changed[key][-1] = changed[key][0]
            with self.assertRaisesRegex(ValueError, 'Incomplete or duplicate'):
                generator.generate(changed, self.bindings)

    def test_missing_binding_rejected(self):
        changed = copy.deepcopy(self.data)
        changed['bindings'].pop()
        with self.assertRaisesRegex(ValueError, 'inventories disagree'):
            generator.generate(changed, self.bindings)

if __name__ == '__main__':
    unittest.main()
