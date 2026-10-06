import unittest
from release_expression_projection import compare

class OrderedExpressionProjectionTests(unittest.TestCase):
    def setUp(self):
        self.elements = {
            'left': {'kind': 'LiteralInteger', 'value': 1},
            'right': {'kind': 'LiteralInteger', 'value': 2},
            'root': {'kind': 'OperatorExpression', 'operator': '-',
                     'reference_sequences': {'argument': {'targets': ['left', 'right']}}},
        }
        self.native = {'kind': 'binary', 'op': 'subtract',
                       'left': {'kind': 'literal', 'value': 1},
                       'right': {'kind': 'literal', 'value': 2}}

    def compare(self):
        return compare(self.native, ['root'], {}, self.elements, lambda element: element)

    def test_argument_order_and_duplicates_are_observable(self):
        self.assertEqual(self.compare()['status'], 'equal')
        self.elements['root']['reference_sequences']['argument']['targets'] = ['right', 'left']
        self.assertEqual(self.compare()['status'], 'different')
        self.elements['root']['reference_sequences']['argument']['targets'] = ['left', 'left']
        self.assertEqual(self.compare()['status'], 'different')

    def test_boolean_does_not_compare_equal_to_integer(self):
        self.native['left']['value'] = True
        self.assertEqual(self.compare()['status'], 'different')

    def test_missing_ordered_evidence_is_not_a_pass(self):
        del self.elements['root']['reference_sequences']
        self.assertEqual(self.compare()['status'], 'unassessed')

    def test_cycles_and_missing_targets_are_not_a_pass(self):
        self.elements['root']['reference_sequences']['argument']['targets'] = ['root']
        self.assertEqual(self.compare()['status'], 'unassessed')
        self.elements['root']['reference_sequences']['argument']['targets'] = ['unknown']
        self.assertEqual(self.compare()['status'], 'unassessed')

    def test_unpaired_references_are_not_matched_by_name(self):
        self.native = {'kind': 'path', 'segments': [{'name': 'x', 'feature': 'x'}]}
        self.assertEqual(self.compare()['status'], 'unassessed')

if __name__ == '__main__':
    unittest.main()
