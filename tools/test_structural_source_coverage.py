import copy
import json
import unittest
from check_structural_source_coverage import PROFILE, inventory


class StructuralCoverageTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grammar = json.loads((PROFILE / 'grammar.structure.extract.json').read_text(encoding='utf-8'))
        cls.ecore = json.loads((PROFILE / 'ecore-semantics.extract.json').read_text(encoding='utf-8'))

    def test_complete_counts_and_no_execution_credit(self):
        report = inventory(self.grammar, self.ecore)
        self.assertEqual(sum(r['occurrences'] for r in report['xtext']), self.grammar['coverage']['nodes'])
        for row in report['xtext'] + report['ecore'] + report['delegate_bindings']:
            self.assertEqual(row['implementation'], 'unassessed')
            self.assertEqual(row['verification'], 'unassessed')
        self.assertEqual(len(report['delegate_bindings']), len(self.ecore['delegate_bindings']))

    def test_missing_tree_fails(self):
        altered = copy.deepcopy(self.ecore)
        del altered['source_tree']
        with self.assertRaisesRegex(ValueError, 'source tree'):
            inventory(self.grammar, altered)

    def test_new_xtext_construct_requires_review_even_with_updated_counts(self):
        altered = copy.deepcopy(self.grammar)
        node = altered['grammars'][0]['rules'][0]
        old = node['kind']
        node['kind'] = 'FutureRule'
        altered['coverage']['node_kinds'][old] -= 1
        altered['coverage']['node_kinds']['FutureRule'] = 1
        with self.assertRaisesRegex(ValueError, 'Unreviewed Xtext construct'):
            inventory(altered, self.ecore)


if __name__ == '__main__':
    unittest.main()
