"""Validate the one curated Ecore/Xtext coverage plan against imported definitions."""
import json
import hashlib
from pathlib import Path
from collections import Counter
from strategy_qualification import validate_strategy_batches
from qualification_certificates import validate_certificates

PLAN_NAME = 'structural-coverage-plan.json'
DEPENDENCIES = {'native_parser', 'model_construction', 'scoping', 'linking', 'value_conversion', 'delegate_algorithms', 'operation_dispatch'}


RELEASE_GATES = {'obligation_inventory', 'native_pipeline', 'sample_pipeline',
                 'pilot_semantics', 'timing_assessment'}


def completion_accounting(plan, checked, qualification):
    """Report coarse gate closure without inventing a behavioral percentage.

    Only certificates for reviewed behavioral contracts can establish closure.
    Imported identities and bounded strategy certificates remain separate.
    """
    gates = plan.get('release_gates', [])
    ids = [gate['id'] for gate in gates]
    if len(ids) != len(set(ids)) or set(ids) != RELEASE_GATES:
        raise ValueError('Every release gate needs exactly one record')
    for gate in gates:
        if gate.get('status') not in ('pending', 'qualified'):
            raise ValueError('Invalid release qualification state')
        if not gate.get('acceptance', '').strip() or (gate['status'] == 'pending' and not gate.get('blocker', '').strip()):
            raise ValueError('Release gates require acceptance criteria and explicit blockers')
    active = [row for row in checked if row['applicability'] == 'used_by_pin']
    # Hash identities, not evidence prose. Pin changes alter the denominator
    # fingerprint; overlapping family selectors deliberately remain separate.
    identities = {row['id']: sorted(set(row['upstream_ids'])) for row in active}
    return {
        'measure': 'whole feature families and release gates; not behavioral percent',
        'required_feature_families': len(active),
        'qualified_feature_families': sum(row['completion'] == 'complete' for row in active),
        'unused_feature_families': len(checked) - len(active),
        'required_release_gates': len(gates),
        'qualified_release_gates': sum(gate['status'] == 'qualified' for gate in gates),
        'behavioral_denominator_status': qualification['behavioral_denominator_status'],
        'overall_percent': None,
        'source_inventory_sha256': hashlib.sha256(json.dumps(identities, sort_keys=True).encode()).hexdigest(),
        'source_family_instances': {key: len(value) for key, value in identities.items()},
        'release_gates': gates,
    }


def compile_plan(plan, grammar, ecore, root):
    batches = plan.get('strategy_batches', [])
    reviewed_strategies = {
        'usage-variation-contribution': {'resolved-dispatch-and-selection', 'positive-specialization', 'physical-construction-and-persistence', 'succession-variation-specialization', 'complete-usage-lifecycle'},
        'ordinary-redefinition-query': {'ordered-query', 'effective-names', 'canonical-boundaries', 'candidate-integration'},
        'bounded-binary-crossing-lifecycle': {'owned-cross-selection', 'binary-crossing-construction', 'owned-cross-specialization', 'binary-owned-cross-featuring', 'real-source-integrated-lifecycle'},
    }
    if {b['id'] for b in batches} != set(reviewed_strategies):
        raise ValueError('Reviewed strategy batch inventory cannot disappear or change without review')
    for batch in batches:
        if {o['id'] for o in batch['obligations']} != reviewed_strategies[batch['id']]:
            raise ValueError('Reviewed strategy obligations cannot disappear or change without review')
    strategy_accounting = validate_strategy_batches(plan, root)
    nodes = []
    def walk(value):
        if isinstance(value, list):
            for child in value: walk(child)
        elif isinstance(value, dict):
            if 'kind' in value: nodes.append(value)
            for child in value.values(): walk(child)
    walk(grammar['grammars'])
    rows = plan['rows']
    ids = [r['id'] for r in rows]
    if len(set(ids)) != len(ids): raise ValueError('Duplicate coverage row')
    kinds = {n['kind'] for n in nodes}
    declared = [r['selector']['node_kind'] for r in rows if 'node_kind' in r['selector']]
    if set(declared) != kinds or len(declared) != len(kinds):
        raise ValueError('Every Xtext kind needs exactly one coverage row')
    required_ecore = {'inheritance','types','multiplicities','containment','opposites','ordering','uniqueness','defaults','property_flags','operations','annotations','delegate_bindings','packages_and_enums'}
    if {r['id'].removeprefix('ecore.') for r in rows if r['domain']=='ecore'} != required_ecore:
        raise ValueError('Missing or unknown required Ecore semantic family')
    if not {'xtext.cardinality','xtext.predicates','xtext.hidden_tokens','xtext.rule_parameters'} <= set(ids):
        raise ValueError('Behavioral Xtext fields require separate coverage obligations')
    xml_fields = {}
    def xml(node):
        tag = node['tag'].rsplit('}', 1)[-1]
        xml_fields.setdefault(tag, set()).update(node['attributes'])
        for child in node.get('children', []): xml(child)
    xml(ecore['source_tree'])
    if {k:sorted(v) for k,v in xml_fields.items()} != plan['reviewed_ecore_xml_fields']:
        raise ValueError('Unreviewed Ecore XML structure or attributes')
    if set(plan['dependencies']) != DEPENDENCIES:
        raise ValueError('Missing semantic dependency mapping')
    for dependency in plan['dependencies'].values():
        if not dependency['remaining'] or dependency['status'] not in {'partial','unassessed','unsupported','complete'}:
            raise ValueError('Invalid dependency coverage')
        if dependency.get('implementation_mode') not in {'mixed', 'handwritten_rust', 'unmapped'} or not dependency.get('boundary'):
            raise ValueError('Semantic dependency must identify its native/handwritten boundary')
        for consumer in dependency['native_consumers']:
            path = Path(consumer['path'])
            if path.is_absolute() or '..' in path.parts:
                raise ValueError('Consumer path must stay within repository')
            if consumer['anchor'] not in (root/path).read_text(encoding='utf-8'):
                raise ValueError('Missing dependency consumer anchor')
    evidence_files = {c["path"] for d in plan["dependencies"].values() for c in d["native_consumers"]}
    checked = []
    for row in rows:
        if not row['required'] or not row['remaining_required_behavior'].strip():
            raise ValueError('Required behavior cannot be silently excluded')
        if row['native_support'] not in {'partial','unassessed','unsupported','complete'} or row['completion'] not in {'open','complete'} or (row['native_support'] == 'complete') != (row['completion'] == 'complete'):
            raise ValueError('Closing coverage requires a reviewed qualification gate; extraction cannot close it')
        if not set(row['semantic_dependencies']) <= DEPENDENCIES:
            raise ValueError('Unknown semantic dependency')
        if row['native_support'] in {'partial','complete'} and not row['native_consumers']:
            raise ValueError('Partial implementation requires a native consumer')
        for entry in row['native_consumers'] + row['verification_evidence'] + row['normative_evidence']:
            path = Path(entry['path'])
            if path.is_absolute() or '..' in path.parts:
                raise ValueError('Evidence path must stay within the repository')
            evidence_files.add(entry['path'])
            text = (root / path).read_text(encoding='utf-8')
            if not entry['anchor'] or entry['anchor'] not in text:
                raise ValueError('Missing evidence anchor: ' + entry['path'])
            if not entry['scope'].strip(): raise ValueError('Evidence needs an explicit scope')
        selector = row['selector']
        out = dict(row)
        if row['domain']=='xtext':
            selected = [n for n in nodes if n['kind']==selector['node_kind']] if 'node_kind' in selector else [n for n in nodes if any(f in n['fields'] for f in selector['fields'])]
            if 'node_kind' in selector:
                observed = set().union(*(set(n['fields']) for n in selected))
                if observed != set(selector['reviewed_fields']):
                    raise ValueError('Unreviewed Xtext fields: ' + row['id'])
            else:
                out['non_default_field_occurrences'] = {f: sum(bool(n['fields'].get(f)) for n in selected) for f in selector['fields']}
            out['upstream_ids'] = [n['id'] for n in selected]
        else:
            selected = [e for e in ecore['elements'] if e['kind'] in selector['element_kinds']]
            if row['id']=='ecore.annotations': selected = [e for e in ecore['elements'] if e['annotations']]
            if row['id']=='ecore.delegate_bindings':
                out['upstream_ids'] = sorted({b['element'] for b in ecore['delegate_bindings']})
            else: out['upstream_ids'] = [e['id'] for e in selected]
        out['applicability'] = 'used_by_pin'
        if 'non_default_field_occurrences' in out and not any(out['non_default_field_occurrences'].values()):
            out['applicability'] = 'no_active_occurrences_in_pin'
            out['native_support'] = 'not_used_in_pin'
            out['completion'] = 'not_applicable'
            out['remaining_required_behavior'] = 'No active occurrences in this pin; preserve the fields and reopen automatically if upstream begins using them.'
        out['imported_instances'] = len(out['upstream_ids'])
        out['verification_status'] = 'qualified' if out['completion'] == 'complete' else 'bounded_evidence_only' if out['verification_evidence'] else 'unqualified'
        checked.append(out)
    # Validate gate identities before building automatic dependency edges.
    gate_ids = [gate['id'] for gate in plan.get('release_gates', [])]
    if len(gate_ids) != len(set(gate_ids)) or set(gate_ids) != RELEASE_GATES:
        raise ValueError('Every release gate needs exactly one record')
    has_closure = any(row['completion'] == 'complete' for row in checked) or any(d['status'] == 'complete' for d in plan['dependencies'].values()) or any(g['status'] == 'qualified' for g in plan['release_gates'])
    native_inputs = None
    if has_closure:
        from qualify_definition_pipeline import inputs
        native_inputs = inputs
    qualification = validate_certificates(plan, checked, root, native_inputs)
    complete = all(row['completion'] != 'open' for row in checked) and all(gate['status'] == 'qualified' for gate in plan['release_gates'])
    return {'schema': 'dev.mercurio.structural-coverage-checklist.v1', 'rows':checked,
            'completion_accounting':completion_accounting(plan, checked, qualification),
            'qualification_accounting': qualification,
            'strategy_accounting':strategy_accounting,
            'evidence_file_sha256':{p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in sorted(evidence_files)},
            'dependencies':plan['dependencies'], 'completion': 'complete' if complete else 'open', 'required_open_rows':sum(r['completion']=='open' for r in checked), 'batches':plan['batches'],
            'xtend_evaluation':plan['xtend_evaluation'], 'completion_policy':plan['completion_policy']}


def markdown(checklist):
    lines = ['<!-- structural-coverage:start -->',
             '## Ecore/Xtext coverage checklist (authoritative)', '',
             'Priority is demonstrated native support for the pinned 2026-08 Ecore and Xtext definitions.',
             'The curated source is `structural-coverage-plan.json` in the candidate profile; edit that file,',
             'then run `python -B tools/check_structural_source_coverage.py` to regenerate this checklist.',
             'The linked [machine-readable checklist](structural-source-coverage.json) records every matched upstream ID,',
             'native consumer, semantic dependency, evidence anchor and remaining obligation.',
             '**Every used feature remains required. No exclusions are approved.** Rows with no active occurrences are marked not used in this pin and reopen automatically on drift. Partial means bounded consumers exist;',
             'unassessed means native behavior has not been exhaustively mapped or qualified, not that parsing necessarily fails.',
             'Imported definitions and generation tests never establish runtime support. The written specification is normative;',
             'Pilot is independent behavioral evidence. Normative references still need to be attached to each qualification row.', '',
             '| Feature | Imported | Native consumer | Semantic evidence | Critical gap |',
             '|---|---:|---|---|---|']
    for row in checklist['rows']:
        gap = row['remaining_required_behavior'].split(';', 1)[0]
        if len(gap) > 140:
            gap = gap[:137].rsplit(' ', 1)[0] + '…'
        lines.append(f"| {row['id']} | {row['imported_instances']} | {row['native_support']} | {row['verification_status']} | {gap} |")
    lines += ['', '### Semantic implementation boundary', '',
              '| Dependency | Current implementation | Missing algorithm or contract |',
              '|---|---|---|']
    for name, dependency in checklist['dependencies'].items():
        lines.append(f"| {name} | {dependency['implementation_mode']} | {dependency['boundary']} |")
    accounting = checklist['completion_accounting']
    lines += ['', '### Completion gates', '',
              f"Whole feature families qualified: {accounting['qualified_feature_families']}/{accounting['required_feature_families']}; {accounting['unused_feature_families']} unused in this pin.",
              'These counts are not a behavioral completion percentage. Behavioral qualification requires reviewed obligations and validated evidence.', '',
              '| Gate | Status | Current blocker |', '|---|---|---|']
    for gate in accounting['release_gates']:
        lines.append(f"| {gate['id']} | {gate['status']} | {gate['blocker']} |")
    lines += ['', 'Full obligations, semantic dependencies, native consumer anchors and evidence are in the machine-readable checklist.',
              '<!-- structural-coverage:end -->']
    return '\n'.join(lines)
