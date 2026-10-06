"""Controlled override bodies around pinned inheritance/call identities.

This is test evidence, not an export of Pilot behavior or production rule coverage.
"""
import argparse
import copy
import json
from pathlib import Path
from generate_xtext_fragment_programs import build, GRAMMAR, ECORE, SEMANTICS, EXPRESSIONS, LANGUAGES

OUTPUT = Path(__file__).resolve().parents[1] / 'docs/conformance/2026-08-support/xtext-context-controls.json'


def fixture(grammar):
    grammar = copy.deepcopy(grammar)
    targets = {EXPRESSIONS[:-2]: 'LiteralInteger', LANGUAGES['kerml']: 'LiteralBoolean',
               LANGUAGES['sysml']: 'LiteralString'}
    for group in grammar['grammars']:
        for row in group['rules']:
            if row['fields']['name'] != 'ExpressionBody':
                continue
            call = {'kind': 'RuleCall', 'id': row['id'] + '/alternatives',
                    'fields': {'rule': {'$ref': EXPRESSIONS + targets[group['id']]},
                               'cardinality': None, 'arguments': [], 'explicitlyCalled': False}}
            row['fields']['alternatives'] = call
            for context in grammar['resolution_contexts'].values():
                context['rule_calls'] = {k: v for k, v in context['rule_calls'].items()
                                         if not k.startswith(row['id'] + '/')}
                if row['id'] in context['effective_rules'].values():
                    context['rule_calls'][call['id']] = call['fields']['rule']['$ref']
    return grammar



def prediction_fixture(grammar):
    """Controlled common prefixes, retaining pinned literal constructors."""
    grammar = copy.deepcopy(grammar)
    row = next(r for g in grammar['grammars'] for r in g['rules']
               if r['id'] == EXPRESSIONS + 'LiteralExpression')
    def keyword(value):
        return {'id': 'prediction/keyword/' + value, 'kind': 'Keyword',
                'fields': {'cardinality': None, 'value': value}}
    def call(name):
        return {'id': 'prediction/call/' + name, 'kind': 'RuleCall',
                'fields': {'cardinality': None, 'rule': {'$ref': EXPRESSIONS + name}}}
    def group(identity, children, cardinality=None, kind='Group'):
        return {'id': 'prediction/' + identity, 'kind': kind,
                'fields': {'cardinality': cardinality, 'elements': children}}
    row['fields']['alternatives'] = group('root', [
        group('optional', [keyword('.'), keyword('['), call('LiteralInteger'), keyword(']')], '?'),
        group('choice', [
            group('integer', [keyword('.'), call('LiteralInteger'), keyword(';')]),
            group('boolean', [keyword('.'), keyword('('), call('LiteralBoolean'), keyword(')')])
        ], kind='Alternatives')
    ])
    return grammar


def generate(grammar, ecore, semantics):
    prediction = build(prediction_fixture(grammar), ecore, semantics, roots=[EXPRESSIONS + 'LiteralExpression'], upstream_predictions=False)
    grammar = fixture(grammar)
    roots = [EXPRESSIONS + 'ExpressionBodyMember', EXPRESSIONS + 'ExpressionBody']
    contexts = {'base': EXPRESSIONS[:-2], **LANGUAGES}
    return {'scope': 'Controlled ExpressionBody replacements; pinned call, ownership and override contracts. Controlled two-symbol prediction fixture included. Not pinned expression-body qualification.',
            'language_programs': {'prediction': prediction, **{
                name: build(grammar, ecore, semantics, roots=roots, context=context, upstream_predictions=False)
                for name, context in contexts.items()} },
            'rules': {}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    document = generate(*(json.loads(p.read_text(encoding='utf-8')) for p in [GRAMMAR, ECORE, SEMANTICS]))
    text = json.dumps(document, indent=2) + '\n'
    if args.check:
        if OUTPUT.read_text(encoding='utf-8') != text:
            raise ValueError('Stale Xtext context controls')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print('Controlled Xtext dispatch programs current: base, KerML, SysML')


if __name__ == '__main__':
    main()
