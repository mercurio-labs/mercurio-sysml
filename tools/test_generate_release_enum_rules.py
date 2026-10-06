import copy
import json
import unittest

from tools.generate_release_enum_rules import ECORE, GRAMMAR, extract


class ReleaseEnumRuleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grammar = json.loads(GRAMMAR.read_text(encoding='utf-8'))
        cls.ecore = json.loads(ECORE.read_text(encoding='utf-8'))

    def test_complete_mapping_and_ecore_values(self):
        result = extract(self.grammar, self.ecore)
        self.assertEqual(len(result['rules']), 14)
        self.assertEqual(sum(len(row['literals']) for row in result['rules']), 24)
        self.assertEqual(next(row for row in result['rules'] if row['name'] == 'ExposeVisibilityKind')
                         ['literals'][0]['value'], 1)

    def test_changed_ecore_literal_fails_closed(self):
        model = copy.deepcopy(self.ecore)
        literal = next(row for row in model['elements'] if row.get('id') ==
                       'https://www.omg.org/spec/SysML/20250201#//VisibilityKind/literal:protected')
        literal['name'] = 'changed'
        with self.assertRaisesRegex(ValueError, 'Xtext/Ecore enum identity mismatch'):
            extract(self.grammar, model)


if __name__ == '__main__':
    unittest.main()
