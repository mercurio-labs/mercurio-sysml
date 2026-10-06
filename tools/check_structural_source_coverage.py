"""Inventory preserved source constructs; never infer native support from extraction."""
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import argparse
from export_pilot_grammar_structure import validate_structure
from structural_coverage_plan import PLAN_NAME, compile_plan, markdown

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / 'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/structural-source-coverage.json'
KINDS = frozenset('Action Alternatives Annotation Assignment CharacterRange CrossReference EnumLiteralDeclaration EnumRule Grammar Group Keyword NegatedToken ParserRule ReferencedMetamodel RuleCall TerminalRule TypeRef UntilToken'.split())


def inventory(grammar, ecore):
    validate_structure(grammar)
    kinds = Counter()
    fields = defaultdict(Counter)
    def walk(value):
        if isinstance(value, list):
            for child in value:
                walk(child)
        elif isinstance(value, dict):
            if 'kind' in value:
                kind = value['kind']
                if kind not in KINDS:
                    raise ValueError('Unreviewed Xtext construct: ' + kind)
                kinds[kind] += 1
                fields[kind].update(value['fields'].keys())
            for child in value.values():
                walk(child)
    walk(grammar['grammars'])
    xml_kinds, attrs = Counter(), defaultdict(Counter)
    def xml(node):
        tag = node['tag'].rsplit('}', 1)[-1]
        xml_kinds[tag] += 1
        attrs[tag].update(node['attributes'].keys())
        for child in node.get('children', []):
            xml(child)
    if 'source_tree' not in ecore:
        raise ValueError('Missing complete Ecore source tree')
    xml(ecore['source_tree'])
    def rows(counts, attributes):
        return [dict(construct=k, occurrences=v, preserved_fields=dict(sorted(attributes[k].items())),
                     preservation='exported', implementation='unassessed', verification='unassessed')
                for k, v in sorted(counts.items())]
    return {
        'schema': 'dev.mercurio.structural-source-coverage.v1',
        'meaning': 'Counts measure preserved source, not native behavior. Unassessed does not mean absent.',
        'xtext': rows(kinds, fields),
        'ecore': rows(xml_kinds, attrs),
        'delegate_bindings': [dict(binding=b, implementation='unassessed', verification='unassessed', coverage_row='ecore.delegate_bindings')
                              for b in ecore['delegate_bindings']],

    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    paths = [PROFILE / 'grammar.structure.extract.json', PROFILE / 'ecore-semantics.extract.json']
    result = inventory(*(json.loads(p.read_text(encoding='utf-8')) for p in paths))
    plan = json.loads((PROFILE / PLAN_NAME).read_text(encoding='utf-8'))
    result['coverage_checklist'] = compile_plan(plan, *(json.loads(p.read_text(encoding='utf-8')) for p in paths), ROOT)
    result['inputs'] = {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                       for p in [*paths, PROFILE / 'ecore-effective.extract.json', ROOT / 'tools/generate_ecore_ownership.py', ROOT / 'tools/generate_ecore_hierarchy.py', PROFILE / PLAN_NAME, ROOT / 'tools/structural_coverage_plan.py', ROOT / 'tools/qualification_certificates.py', ROOT / plan['qualification_contracts'], Path(__file__)]}
    data = (json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True) + '\n').encode('utf-8')
    checklist = ROOT / 'docs/conformance/2026-08-support/remaining-gap-checklist.md'
    current = checklist.read_text(encoding='utf-8')
    start, end = '<!-- structural-coverage:start -->', '<!-- structural-coverage:end -->'
    if start in current:
        before, rest = current.split(start, 1)
        _, after = rest.split(end, 1)
        updated = before + markdown(result['coverage_checklist']) + after
    else:
        marker = '## Structural source coverage expansion (2026-09-29)'
        updated = current.replace(marker, markdown(result['coverage_checklist']) + '\n\n## Structural batch history (2026-09-29)', 1)
    if args.check:
        if updated != current:
            raise ValueError('Stale authoritative coverage checklist')
        if OUTPUT.read_bytes() != data:
            raise ValueError('Stale structural source coverage')
    else:
        OUTPUT.write_bytes(data)
        checklist.write_text(updated, encoding='utf-8', newline='\n')
    print('Structural source inventory current; native coverage remains separately assessed')


if __name__ == '__main__':
    main()
