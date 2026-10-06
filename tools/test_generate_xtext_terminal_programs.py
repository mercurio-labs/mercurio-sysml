"""Source coverage and fail-closed controls for the shared terminal compiler."""
import copy
import json
import unittest
from pathlib import Path

from generate_xtext_terminal_programs import PROFILE, build, nodes, render, qualify_committed_decisions


class TerminalProgramTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document = json.loads((PROFILE / 'grammar.structure.extract.json').read_text(encoding='utf-8'))

    def terminal(self, document, name):
        return next(rule for grammar in document['grammars'] for rule in grammar['rules']
                    if rule['kind'] == 'TerminalRule' and rule['fields']['name'] == name)

    def test_every_terminal_and_keyword_occurrence_is_retained(self):
        contract = build(self.document)
        self.assertEqual(len(contract['terminals']), 9)
        self.assertEqual(len(contract['keyword_occurrences']), 663)
        self.assertEqual({row['op'] for row in contract['programs']},
                         {'literal', 'range', 'sequence', 'choice', 'call', 'not_characters', 'until'})
        for grammar, row in contract['languages'].items():
            self.assertEqual(len(row['terminals']), 9)
            self.assertEqual([ref.rsplit('::', 1)[1] for ref in row['inherited_hidden_tokens']],
                             ['WS', 'ML_NOTE', 'SL_NOTE'])
        sysml = contract['languages']['org.omg.sysml.xtext.SysML']['keywords']
        kerml = contract['languages']['org.omg.kerml.xtext.KerML']['keywords']
        self.assertIn('part', sysml)
        self.assertNotIn('part', kerml)
        self.assertEqual(render(contract), render(build(self.document)))

    def test_rule_hidden_override_cannot_be_ignored(self):
        document = copy.deepcopy(self.document)
        rule = next(r for g in document['grammars'] for r in g['rules'] if r['kind']=='ParserRule')
        rule['fields']['definesHiddenTokens'] = True
        with self.assertRaisesRegex(ValueError, 'hidden-token override'): build(document)

    def test_changed_inherited_policy_requires_a_consumer(self):
        document = copy.deepcopy(self.document)
        document['grammars'][-1]['fields']['definesHiddenTokens'] = True
        document['grammars'][-1]['fields']['hiddenTokens'] = []
        with self.assertRaisesRegex(ValueError, 'Distinct hidden-token'): build(document)

    def test_predicate_drift_is_rejected(self):
        document = copy.deepcopy(self.document)
        self.terminal(document, 'ID')['fields']['alternatives']['fields']['predicated'] = True
        with self.assertRaisesRegex(ValueError, 'predicates'):
            build(document)

    def test_multicharacter_complement_is_rejected(self):
        document = copy.deepcopy(self.document)
        root = self.terminal(document, 'STRING_VALUE')['fields']['alternatives']
        complement = next(n for n in nodes(root) if n['kind'] == 'NegatedToken')
        complement['fields']['terminal']['fields']['elements'][0]['fields']['value'] = 'ab'
        with self.assertRaisesRegex(ValueError, 'character operands'):
            build(document)

    def test_invalid_character_range_is_rejected(self):
        document = copy.deepcopy(self.document)
        root = self.terminal(document, 'DECIMAL_VALUE')['fields']['alternatives']
        character_range = next(n for n in nodes(root) if n['kind'] == 'CharacterRange')
        character_range['fields']['left']['fields']['value'] = 'z'
        with self.assertRaisesRegex(ValueError, 'character range'):
            build(document)

    def test_terminal_recursion_is_rejected(self):
        document = copy.deepcopy(self.document)
        rule = self.terminal(document, 'EXP_VALUE')
        call = next(n for n in nodes(rule) if n['kind'] == 'RuleCall')
        call['fields']['rule']['$ref'] = rule['id']
        with self.assertRaisesRegex(ValueError, 'Recursive'):
            build(document)

    def test_empty_until_delimiter_is_rejected(self):
        document = copy.deepcopy(self.document)
        root = self.terminal(document, 'REGULAR_COMMENT')['fields']['alternatives']
        until = next(n for n in nodes(root) if n['kind'] == 'UntilToken')
        until['fields']['terminal']['fields']['value'] = ''
        with self.assertRaisesRegex(ValueError, 'delimiter'):
            build(document)

    def test_source_keyword_change_changes_native_output(self):
        document = copy.deepcopy(self.document)
        root = self.terminal(document, 'WS')['fields']['alternatives']
        root['fields']['elements'][0]['fields']['value'] = '_'
        changed = render(build(document))
        self.assertNotEqual(changed, render(build(self.document)))
        self.assertIn('Operation::Literal(&[95])', changed)

    def test_ambiguous_terminal_choice_needs_upstream_lookahead(self):
        program = [
            dict(source_id='a',op='literal',cardinality='one',units=[97]),
            dict(source_id='ab',op='literal',cardinality='one',units=[97,98]),
            dict(source_id='choice',op='choice',cardinality='one',children=[0,1]),
        ]
        with self.assertRaisesRegex(ValueError,'choice requires imported lookahead'):
            qualify_committed_decisions(program,[dict(program=2)])

    def test_nullable_alternative_is_not_a_first_set_decision(self):
        program = [
            dict(source_id='a',op='literal',cardinality='?',units=[97]),
            dict(source_id='b',op='literal',cardinality='one',units=[98]),
            dict(source_id='choice',op='choice',cardinality='one',children=[0,1]),
        ]
        with self.assertRaisesRegex(ValueError,'choice requires imported lookahead'):
            qualify_committed_decisions(program,[dict(program=2)])

    def test_repetition_follow_conflict_propagates_through_rule_calls(self):
        program = [
            dict(source_id='repeat',op='literal',cardinality='*',units=[97]),
            dict(source_id='call',op='call',cardinality='one',child=0),
            dict(source_id='following',op='literal',cardinality='one',units=[97]),
            dict(source_id='root',op='sequence',cardinality='one',children=[1,2]),
        ]
        with self.assertRaisesRegex(ValueError,'exit requires imported lookahead'):
            qualify_committed_decisions(program,[dict(program=3)])

    def test_nullable_repetition_cannot_make_progress(self):
        program = [
            dict(source_id='optional',op='literal',cardinality='?',units=[97]),
            dict(source_id='repeat',op='call',cardinality='*',child=0),
        ]
        with self.assertRaisesRegex(ValueError,'Nullable terminal decision body'):
            qualify_committed_decisions(program,[dict(program=1)])

    def test_nullable_terminal_root_cannot_become_a_source_token(self):
        program = [dict(source_id='optional',op='literal',cardinality='?',units=[97])]
        with self.assertRaisesRegex(ValueError,'Nullable terminal root'):
            qualify_committed_decisions(program,[dict(program=0)])


if __name__ == '__main__':
    unittest.main()
