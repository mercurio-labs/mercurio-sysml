"""Compile a bounded Xtext fragment family into native-interpreted programs.

Selection is explicit. Unsupported nodes, ambiguous first sets, nullable loops,
unresolved Ecore assignments and changed external bindings fail generation.
"""
import argparse
import copy
import hashlib
import json
import re

from generate_xtext_assignment_contracts import GRAMMAR, ECORE, PROFILE, EffectiveEcore, ref, SYSML
from generate_release_enum_rules import extract as enum_contract
from export_pilot_grammar_structure import validate_contexts
from export_pilot_prediction import first_set_signature
from xtext_prediction import bounded_prefixes, disjoint

OUTPUT = PROFILE / 'xtext-fragment-programs.json'
SEMANTICS = PROFILE / 'ecore-semantics.extract.json'
PREDICTIONS = PROFILE / 'xtext-prediction.extract.json'
EXPRESSIONS = 'org.omg.kerml.expressions.xtext.KerMLExpressions::'
NAME = EXPRESSIONS + 'Name'
CHAIN = EXPRESSIONS + 'FeatureChainMember'
TYPE_MEMBER = EXPRESSIONS + 'InstantiatedTypeMember'
LITERALS = [EXPRESSIONS + name for name in ['LiteralExpression', 'NullExpression']]
ECORE_NS = 'http://www.eclipse.org/emf/2002/Ecore#//'
GRAPH_ROOT = 'org.omg.kerml.xtext.KerML::OwnedMultiplicityRange'
ROOTS = [f'org.omg.{language}.xtext.{title}::{rule}'
         for language, title, rules in [('kerml', 'KerML', ['Identification', 'MemberPrefix', 'BasicFeaturePrefix', 'Documentation', 'TextualRepresentation', 'Comment']),
                                        ('sysml', 'SysML', ['Identification', 'MemberPrefix', 'Documentation', 'TextualRepresentation', 'Comment'])]
         for rule in rules] + LITERALS + [CHAIN, TYPE_MEMBER, 'org.omg.kerml.xtext.KerML::OwnedMultiplicityRange', 'org.omg.sysml.xtext.SysML::MultiplicityExpressionMember', 'org.omg.sysml.xtext.SysML::TimeTriggerKind']


CONTEXT_ROOTS = {'kerml': ['org.omg.kerml.xtext.KerML::MetadataFeatureDeclaration'], 'sysml': ['org.omg.sysml.xtext.SysML::EmptySuccessionMember']}

for language, title in [('kerml', 'KerML'), ('sysml', 'SysML')]:
    CONTEXT_ROOTS[language] += [f'org.omg.{language}.xtext.{title}::{name}' for name in
                              ['OwnedFeatureTyping', 'OwnedSubsetting', 'OwnedRedefinition', 'OwnedReferenceSubsetting', 'OwnedCrossSubsetting', 'FeatureSpecializationPart', 'FeatureDeclaration']]

CONTEXT_ROOTS['kerml'] += ['org.omg.kerml.xtext.KerML::TypeDeclaration', 'org.omg.kerml.xtext.KerML::ClassifierDeclaration']

CONTEXT_ROOTS['sysml'] += ['org.omg.sysml.xtext.SysML::DefinitionDeclaration', 'org.omg.sysml.xtext.SysML::OccurrenceDefinitionPrefix']

LANGUAGES = {'kerml': 'org.omg.kerml.xtext.KerML', 'sysml': 'org.omg.sysml.xtext.SysML'}


def bind_syntax_conditions(decision, compiled_nodes, signature):
    for state in decision['states']:
        conditions = ([state['gate']] if 'gate' in state else [])
        conditions += [edge['condition'] for edge in state.get('predicate_edges', [])]
        for condition in conditions:
            if condition['kind'] == 'always':
                continue
            guard = compiled_nodes.get(condition['source_id'])
            if (guard is None or not (guard.get('predicated') or guard.get('first_set_predicated'))
                    or bool(condition.get('first_set')) != bool(guard.get('first_set_predicated'))
                    or signature(guard) != condition['signature']):
                raise ValueError('Unbound or changed native predicate: ' + condition['source_id'])
            probe = copy.deepcopy(guard)
            probe['cardinality'] = ''
            probe['predicated'] = False
            probe['first_set_predicated'] = False
            condition['probe'] = probe

def build(grammar, ecore, semantics, roots=ROOTS, context=None, upstream_predictions=True, partial_predictions=False, experimental_graph=None):
    if experimental_graph is not None and (not partial_predictions or context is None or not upstream_predictions):
        raise ValueError('Finite graph requires isolated contextual partial generation')
    original_source_sha256 = hashlib.sha256(json.dumps(grammar, sort_keys=True).encode('utf-8')).hexdigest()
    original_contexts = grammar.get('resolution_contexts', {})
    context_map = None
    entry_rules = {}
    if context is not None:
        validate_contexts(grammar)
        if context not in grammar['resolution_contexts']:
            raise ValueError('Unknown grammar context: ' + context)
        context_map = grammar['resolution_contexts'][context]
        grammar = copy.deepcopy(grammar)
        by_id = {g['id']: g for g in grammar['grammars']}
        visible = set()
        def visit(identity):
            if identity in visible: return
            visible.add(identity)
            for parent in by_id[identity]['fields']['usedGrammars']:
                visit(ref(parent))
        visit(context)
        for identity in roots:
            if identity.split('::')[0] not in visible:
                raise ValueError('Root outside grammar context: ' + identity)
            entry_rules[identity] = context_map['effective_rules'][identity.split('::')[1]]
        roots = list(dict.fromkeys(entry_rules.values()))
        def bind(value):
            if isinstance(value, dict):
                if value.get('kind') == 'RuleCall':
                    target = context_map['rule_calls'].get(value['id'])
                    if target is None:
                        raise ValueError('Missing contextual call binding: ' + value['id'])
                    value['fields']['rule'] = {'$ref': target}
                for child in value.values(): bind(child)
            elif isinstance(value, list):
                for child in value: bind(child)
        for group in grammar['grammars']:
            for row in group['rules']:
                if row['id'] in context_map['effective_rules'].values():
                    bind(row)
        # Calls are now specialized using the upstream frontend's resolved map.
        # FIRST, current-type flow and cycle checks run separately in this context.
        grammar.pop('resolution_contexts')

    source = {r['id']: r for g in grammar['grammars'] for r in g['rules']}
    enums = {r['id']: r for r in enum_contract(grammar, semantics)['rules']}
    model = EffectiveEcore(ecore)
    enum_domains = {row['id']: {} for row in semantics['elements'] if row['kind'] == 'EEnum'}
    for row in semantics['elements']:
        if row['kind'] == 'EEnumLiteral':
            spelling = row['attributes'].get('literal', row['name'])
            domain = enum_domains[row['owner']]
            if spelling in domain:
                raise ValueError('Ambiguous Ecore enum spelling: ' + row['owner'])
            domain[spelling] = row['id']
    programs, active = {}, set()

    # Greatest fixed point of fragments that cannot replace the caller's current
    # object. Assignments may construct children, but those child calls never
    # change the parent's current type. Unassigned model calls and actions do.
    preserving_fragments = {
        identity for identity, row in source.items()
        if row['kind'] == 'ParserRule' and row['fields'].get('fragment')
        and ref(row['fields']['type']['fields']['classifier']) in model.classes
    }
    def preserves_current(row):
        kind, fields = row['kind'], row['fields']
        if kind in ['Keyword', 'Assignment', 'CrossReference']:
            return True
        if kind in ['Group', 'Alternatives']:
            return all(preserves_current(child) for child in fields['elements'])
        if kind == 'RuleCall':
            target = ref(fields['rule'])
            called = source[target]
            owner = ref(called['fields']['type']['fields']['classifier'])
            return owner not in model.classes or target in preserving_fragments
        return False
    while True:
        remaining = {identity for identity in preserving_fragments
                     if preserves_current(source[identity]['fields']['alternatives'])}
        if remaining == preserving_fragments:
            break
        preserving_fragments = remaining

    def reject_predicates(value):
        if isinstance(value, dict):
            fields = value.get('fields', {})
            if value.get('kind') == 'RuleCall':
                targets = {context['rule_calls'][value['id']]
                           for context in grammar.get('resolution_contexts', {}).values()
                           if value['id'] in context['rule_calls']}
                if len(targets) > 1:
                    raise ValueError('Context-dependent rule dispatch not yet supported: ' + value['id'])
            if any(fields.get(key) for key in ['guardCondition']):
                raise ValueError('Unsupported predicate: ' + value.get('id', 'unknown'))
            for child in value.values(): reject_predicates(child)
        elif isinstance(value, list):
            for child in value: reject_predicates(child)

    def rule(identity):
        if identity in programs:
            return programs[identity]
        if identity in active:
            raise ValueError('Unsupported recursive scalar/fragment dependency: ' + identity)
        active.add(identity)
        if context_map is not None and identity not in context_map['effective_rules'].values():
            raise ValueError('Rule outside effective grammar: ' + identity)
        row = source[identity]
        fields = row['fields']
        reject_predicates(row)
        overrides = []
        for annotation in fields.get('annotations', []):
            if context_map is None or annotation['fields'] != {'name': 'Override'}:
                raise ValueError('Unsupported rule annotation: ' + identity)
            declaring = next(g for g in grammar['grammars'] if g['id'] == identity.split('::')[0])
            for parent in declaring['fields']['usedGrammars']:
                parent_id = ref(parent)
                # Use the independently exported parent context, never a sibling.
                parent_context = original_contexts[parent_id]
                target = parent_context['effective_rules'].get(fields['name'])
                if target:
                    inherited = source[target]
                    owner = ref(fields['type']['fields']['classifier'])
                    inherited_owner = ref(inherited['fields']['type']['fields']['classifier'])
                    compatible = inherited_owner == owner or owner in model.classes and inherited_owner in model.ancestors(owner)
                    if row['kind'] != inherited['kind'] or fields.get('fragment') != inherited['fields'].get('fragment') or not compatible:
                        raise ValueError('Incompatible Override contract: ' + identity)
                    overrides.append(target)
            if not overrides:
                raise ValueError('Override has no inherited declaration: ' + identity)
        if fields.get('parameters') or fields.get('definesHiddenTokens') or fields.get('wildcard'):
            raise ValueError('Unsupported rule parameters/hidden override/wildcard: ' + identity)
        owner = ref(fields['type']['fields']['classifier'])
        # A declared model return type is a conservative summary for a
        # recursive call. Type-preserving fragments use the independent proof above;
        # type-changing fragment and scalar cycles remain explicitly unsupported.
        if row['kind'] == 'ParserRule' and owner in model.classes and not fields['fragment']:
            programs[identity] = {'owner': owner, 'scalar': False, 'construct': True,
                                  'result_types': [owner]}
        elif identity in preserving_fragments:
            programs[identity] = {'owner': owner, 'scalar': False, 'construct': False,
                                  'preserves_current': True}
        if identity == NAME:
            body = fields['alternatives']
            calls = body['fields'].get('elements', [])
            if owner != 'http://www.eclipse.org/emf/2002/Ecore#//EString' or fields['fragment'] or body['kind'] != 'Alternatives' or body['fields'].get('cardinality') or any(c['kind'] != 'RuleCall' or
                    c['fields'].get('cardinality') or c['fields'].get('arguments') or c['fields'].get('explicitlyCalled') for c in calls) or [ref(c['fields']['rule']) for c in calls] != [NAME.rsplit('::', 1)[0] + '::' + name for name in ['ID', 'UNRESTRICTED_NAME']]:
                raise ValueError('Changed external Name token binding')
            program = {'owner': owner, 'scalar': True, 'body': {'id': identity, 'cardinality': '', 'kind': 'name'}}
        elif row['kind'] == 'EnumRule':
            def check_enum_cardinality(value):
                if isinstance(value, dict):
                    if value.get('fields', {}).get('cardinality'):
                        raise ValueError('Unsupported enum cardinality: ' + identity)
                    for child in value.values(): check_enum_cardinality(child)
                elif isinstance(value, list):
                    for child in value: check_enum_cardinality(child)
            check_enum_cardinality(fields['alternatives'])
            entries = enums[identity]['literals']
            program = {'owner': owner, 'scalar': True, 'body': {'id': identity, 'cardinality': '', 'kind': 'choice',
                'elements': [{'id': v['ecore_literal'], 'cardinality': '', 'kind': 'enum', 'token': v['token'], 'literal': v['literal']} for v in entries]}}
        elif row['kind'] == 'ParserRule' and owner in enum_domains and not fields['fragment']:
            def enum_value(value):
                f = value['fields']
                if f.get('cardinality') or f.get('predicated') or f.get('firstSetPredicated'):
                    raise ValueError('Unsupported enum datatype syntax: ' + value['id'])
                common = {'id': value['id'], 'cardinality': ''}
                if value['kind'] == 'Keyword':
                    spelling = f['value']
                    if spelling not in enum_domains[owner]:
                        raise ValueError('Unknown Ecore enum spelling: ' + value['id'])
                    return dict(common, kind='enum', token=spelling, literal=spelling,
                                ecore_literal=enum_domains[owner][spelling])
                if value['kind'] == 'Alternatives':
                    return dict(common, kind='choice', elements=[enum_value(c) for c in f['elements']])
                raise ValueError('Unsupported enum datatype syntax: ' + value['id'])
            program = {'owner': owner, 'scalar': True, 'body': enum_value(fields['alternatives']),
                       'conversion': 'Ecore enum literal spelling; single-keyword datatype'}
        elif row['kind'] == 'TerminalRule':
            # The shared terminal compiler owns recognition. This adapter only
            # selects the legacy token carrier and an explicitly bounded converter.
            bindings = {'DECIMAL_VALUE': ('EInt', 'number'), 'EXP_VALUE': ('EString', 'number'), 'STRING_VALUE': ('EString', 'string'), 'REGULAR_COMMENT': ('EString', 'comment')}
            name = fields['name']
            if identity != EXPRESSIONS + name or name not in bindings or fields['fragment']:
                raise ValueError('Unsupported terminal binding: ' + identity)
            target, carrier = bindings[name]
            if owner != ECORE_NS + target:
                raise ValueError('Changed terminal value type: ' + identity)
            program = {'owner': owner, 'scalar': True, 'body': {
                'id': identity, 'cardinality': '', 'kind': 'terminal', 'rule': identity, 'carrier': carrier}}
        elif row['kind'] == 'ParserRule' and owner == ECORE_NS + 'EBoolean' and not fields['fragment']:
            # A single keyword (or disjoint choice of keywords) has one spelling.
            # Multi-token datatype concatenation needs a separate implementation.
            def keyword_value(value):
                f = value['fields']
                return not f.get('cardinality') and (value['kind'] == 'Keyword' or
                    value['kind'] == 'Alternatives' and all(keyword_value(c) for c in f['elements']))
            if not keyword_value(fields['alternatives']):
                raise ValueError('Unsupported datatype value assembly: ' + identity)
            program = {'owner': owner, 'scalar': True, 'body': node(fields['alternatives'], owner)}
        elif row['kind'] == 'ParserRule' and owner == ECORE_NS + 'EDouble' and not fields['fragment']:
            program = {'owner': owner, 'scalar': True, 'body': {
                'id': identity, 'cardinality': '', 'kind': 'datatype',
                'syntax': lexical_node(fields['alternatives'])}}
        elif row['kind'] == 'ParserRule' and owner == ECORE_NS + 'EString' and not fields['fragment']:
            syntax = node(fields['alternatives'], owner)
            def scalar_syntax(value):
                if value['kind'] in ['keyword', 'name']: return
                if value['kind'] == 'call':
                    called = programs[value['rule']]
                    if called['scalar'] and called['body']['kind'] in ['name', 'token_datatype']:
                        return
                if value['kind'] in ['sequence', 'choice']:
                    for child in value['elements']: scalar_syntax(child)
                    return
                raise ValueError('Unsupported token datatype syntax: ' + identity)
            scalar_syntax(syntax)
            program = {'owner': owner, 'scalar': True, 'body': {
                'id': identity, 'cardinality': '', 'kind': 'token_datatype', 'syntax': syntax}}
        elif row['kind'] == 'ParserRule' and owner in model.classes:
            program = {'owner': owner, 'scalar': False, 'construct': not fields['fragment'],
                       'body': node(fields['alternatives'], owner)}
        else:
            raise ValueError('Unsupported model-construction call: ' + identity)
        def predicate_ids(value, imported):
            if isinstance(value, dict):
                fields = value.get('fields', {}) if imported else value
                result = {value['id']} if fields.get('predicated') or fields.get('firstSetPredicated' if imported else 'first_set_predicated') else set()
                for child in value.values():
                    result.update(predicate_ids(child, imported))
                return result
            if isinstance(value, list):
                return set().union(*(predicate_ids(child, imported) for child in value))
            return set()
        imported_predicates = predicate_ids(row, True)
        # Scalar recognizers have separate value/longest-match semantics; do not
        # route their internal predicates through the object lookahead engine.
        if (program['scalar'] and imported_predicates or
                imported_predicates - predicate_ids(program, False)):
            raise ValueError('Unsupported predicate projection: ' + identity)
        if program.get('construct'):
            program['result_types'] = sorted(flow(program['body'], {owner}))
        if identity in preserving_fragments:
            program['preserves_current'] = True
        if overrides:
            program['override_of'] = sorted(overrides)
        if identity in {'org.omg.kerml.xtext.KerML::Nonunique', 'org.omg.sysml.xtext.SysML::Nonunique'}:
            body = program['body']
            if not program['scalar'] or owner != ECORE_NS + 'EBoolean' or body.get('kind') != 'keyword' or body.get('value') != 'nonunique' or body.get('cardinality'):
                raise ValueError('Changed Nonunique converter binding: ' + identity)
            # Named handwritten dependency, independently pinned in the Pilot
            # model oracle. The grammar alone does not define Boolean conversion.
            program['converter'] = 'pilot_nonunique_boolean'
        programs[identity] = program
        active.remove(identity)
        return program

    def flow(value, current):
        kind = value['kind']
        if kind in ['create', 'capture']:
            result = {value['classifier']}
        elif kind == 'call' and programs[value['rule']].get('construct'):
            result = set(programs[value['rule']]['result_types'])
        elif kind == 'call' and programs[value['rule']].get('preserves_current'):
            result = set(current)
        elif kind == 'call' and not programs[value['rule']]['scalar']:
            result = flow(programs[value['rule']]['body'], current)
        elif kind == 'sequence':
            result = set(current)
            for child in value['elements']:
                result = flow(child, result)
        elif kind == 'choice':
            result = set().union(*(flow(child, current) for child in value['elements']))
        else:
            result = set(current)
        if value['cardinality'] in ['?', '*']:
            result.update(current)
        return result

    def current_feature(current, field):
        features = [model.feature(cls, field) for cls in sorted(current)]
        if not features or len({f['id'] for f in features}) != 1:
            raise ValueError('Ambiguous current-type assignment: ' + field)
        return features[0]

    def operand_binding(feature):
        bindings = [b for b in semantics['delegate_bindings'] if b['element'] == feature['id'] and b['kind'] == 'setting']
        if (feature['id'] != SYSML + 'InvocationExpression/operand' or feature['type'] != SYSML + 'Expression' or
            not all(feature[k] for k in ['derived', 'transient', 'volatile', 'containment', 'ordered', 'unique', 'changeable']) or
            feature['opposite'] is not None or feature['upper_bound'] != -1 or len(bindings) != 1 or
            bindings[0]['delegate_uri'] != 'http://www.omg.org/spec/SysML' or
            bindings[0]['binding_status'] != 'custom_setting_delegate_source' or
            bindings[0]['candidate_source_classes'] != ['org.omg.sysml.delegate.setting.InvocationExpression_operand_SettingDelegate']):
            raise ValueError('Unsupported capture delegate: ' + feature['id'])

    def node(row, owner, current=None):
        current = {owner} if current is None else set(current)
        if row['fields'].get('cardinality') in ['*', '+']:
            # Every later iteration must bind the same feature identities as the first.
            initial = set(current)
            for _ in range(len(model.classes) + 1):
                value = node_body(row, owner, current)
                expanded = initial | flow(value, current)
                if expanded == current:
                    return value
                current = expanded
            raise ValueError('Current-type repetition did not converge: ' + row['id'])
        return node_body(row, owner, current)

    def node_body(row, owner, current):
        fields, kind = row['fields'], row['kind']
        if fields.get('guardCondition') or fields.get('predicated') and fields.get('firstSetPredicated'):
            raise ValueError('Unsupported predicate: ' + row['id'])
        cardinality = fields.get('cardinality') or ''
        if cardinality not in ['', '?', '*', '+']:
            raise ValueError('Unsupported cardinality: ' + row['id'])
        result = {'id': row['id'], 'cardinality': cardinality}
        if fields.get('predicated'):
            result['predicated'] = True
        if fields.get('firstSetPredicated'):
            result['first_set_predicated'] = True
        if kind == 'Keyword':
            result.update(kind='keyword', value=fields['value'])
        elif kind in ['Group', 'Alternatives']:
            elements = []
            for child in fields['elements']:
                value = node(child, owner, current)
                elements.append(value)
                if kind == 'Group':
                    current = flow(value, current)
            result.update(kind='sequence' if kind == 'Group' else 'choice', elements=elements)
        elif kind == 'Action':
            if not fields.get('type') or bool(fields.get('feature')) != bool(fields.get('operator')):
                raise ValueError('Unsupported assigned/malformed action: ' + row['id'])
            target = ref(fields['type']['fields']['classifier'])
            cls = model.classes.get(target)
            if cls is None or cls['abstract'] or cls['interface']:
                raise ValueError('Action requires concrete Ecore class: ' + row['id'])
            if owner not in model.ancestors(target):
                raise ValueError('Unsupported action type transition: ' + row['id'])
            if fields.get('feature'):
                feature = model.feature(target, fields['feature'])
                operand_binding(feature)
                if fields['operator'] != '+=' or any(feature['type'] not in model.ancestors(t) for t in current):
                    raise ValueError('Incompatible captured current object: ' + row['id'])
                result.update(kind='capture', classifier=target, feature_id=feature['id'],
                              captured_types=sorted(current))
            else:
                result.update(kind='create', classifier=target)
        elif kind == 'RuleCall':
            if fields.get('arguments') or fields.get('explicitlyCalled'):
                raise ValueError('Unsupported rule arguments: ' + row['id'])
            target = ref(fields['rule'])
            called = rule(target)
            if called.get('construct'):
                if owner not in model.ancestors(called['owner']):
                    raise ValueError('Incompatible object-returning call: ' + row['id'])
            elif not called['scalar'] and called['owner'] not in model.ancestors(owner):
                raise ValueError('Incompatible current-object fragment call: ' + row['id'])
            result.update(kind='call', rule=target)
        elif kind == 'CrossReference':
            target = ref(fields['type']['fields']['classifier'])
            terminal = fields.get('terminal')
            if target not in model.classes or not terminal or terminal['kind'] != 'RuleCall':
                raise ValueError('Unsupported typed cross-reference: ' + row['id'])
            called = rule(ref(terminal['fields']['rule']))
            if not called['scalar'] or called['owner'] != ECORE_NS + 'EString':
                raise ValueError('Cross-reference requires EString spelling: ' + row['id'])
            result.update(kind='cross_reference', target=target, terminal=node(terminal, owner))
        elif kind == 'Assignment':
            feature = current_feature(current, fields['feature'])
            operator, terminal = fields['operator'], fields['terminal']
            if operator not in ['=', '?=', '+=']:
                raise ValueError('Unsupported construction assignment: ' + row['id'])
            if (operator == '+=') == (feature['upper_bound'] == 1):
                raise ValueError('Assignment/operator multiplicity mismatch: ' + row['id'])
            common = dict(feature=re.sub(r'(?<!^)(?=[A-Z])', '_', fields['feature']).lower(),
                          feature_id=feature['id'], operator=operator)
            if feature['kind'] == 'reference':
                operand = feature['id'] == SYSML + 'InvocationExpression/operand'
                if operand:
                    operand_binding(feature)
                    if operator != '+=':
                        raise ValueError('Operand adapter requires append: ' + row['id'])
                if operator == '?=' or feature['derived'] and not operand or not feature['changeable'] or feature['container']:
                    raise ValueError('Unsupported reference write policy: ' + row['id'])
                if feature['containment']:
                    if terminal['kind'] not in ['RuleCall', 'Alternatives']:
                        raise ValueError('Unsupported containment value: ' + row['id'])
                    def object_values(value):
                        f = value['fields']
                        if f.get('cardinality'):
                            raise ValueError('Optional/repeated assignment value: ' + value['id'])
                        if value['kind'] == 'Alternatives':
                            for child in f['elements']: object_values(child)
                        elif value['kind'] == 'RuleCall':
                            called = rule(ref(f['rule']))
                            if not called.get('construct') or feature['type'] not in model.ancestors(called['owner']):
                                raise ValueError('Containment value type mismatch: ' + value['id'])
                        else:
                            raise ValueError('Unsupported containment value: ' + value['id'])
                    object_values(terminal)
                    result.update(kind='append_operand' if operand else 'contain', terminal=node(terminal, feature['type']), **common)
                elif terminal['kind'] == 'CrossReference':
                    target = ref(terminal['fields']['type']['fields']['classifier'])
                    if feature['type'] not in model.ancestors(target):
                        raise ValueError('Cross-reference type mismatch: ' + row['id'])
                    result.update(kind='link', terminal=node(terminal, owner), **common)
                else:
                    raise ValueError('Reference assignment requires linking dependency: ' + row['id'])
            else:
                if operator == '?=' and not feature['type'].endswith('#//EBoolean'):
                    raise ValueError('Non-boolean presence assignment: ' + row['id'])
                if terminal['kind'] == 'RuleCall':
                    called = rule(ref(terminal['fields']['rule']))
                    if not called['scalar'] or operator != '?=' and called['owner'] != feature['type']:
                        raise ValueError('Assignment value type mismatch: ' + row['id'])
                elif terminal['kind'] == 'Keyword' and operator != '?=' and not feature['type'].endswith('#//EString'):
                    raise ValueError('Assignment value type mismatch: ' + row['id'])
                if terminal['kind'] not in ['RuleCall', 'Keyword']:
                    raise ValueError('Unsupported assignment value conversion: ' + row['id'])
                result.update(kind='assign', terminal=node(terminal, owner), **common)
        else:
            raise ValueError('Unsupported fragment node ' + kind + ': ' + row['id'])
        return result

    def lexical_node(row):
        fields, kind = row['fields'], row['kind']
        cardinality = fields.get('cardinality') or ''
        if cardinality not in ['', '?', '*', '+']:
            raise ValueError('Unsupported lexical cardinality: ' + row['id'])
        result = {'id': row['id'], 'cardinality': cardinality}
        if kind == 'Keyword':
            result.update(kind='keyword', value=fields['value'])
        elif kind in ['Group', 'Alternatives']:
            result.update(kind='sequence' if kind == 'Group' else 'choice',
                          elements=[lexical_node(c) for c in fields['elements']])
        elif kind == 'RuleCall':
            target = ref(fields['rule'])
            if fields.get('arguments') or fields.get('explicitlyCalled') or source[target]['kind'] != 'TerminalRule':
                raise ValueError('Unsupported datatype lexical call: ' + row['id'])
            called = rule(target)
            if called['body'].get('carrier') != 'number':
                raise ValueError('Unsupported datatype token carrier: ' + row['id'])
            result.update(kind='terminal', rule=target, carrier='number')
        else:
            raise ValueError('Unsupported datatype syntax: ' + row['id'])
        bare = dict(row, fields=dict(fields, cardinality=None))
        if cardinality in ['*', '+'] and 0 in lexical_categories(bare):
            raise ValueError('Nullable datatype repetition: ' + row['id'])
        return result

    # Exact three-state abstraction of the admitted numeric lexical languages:
    # epsilon, nonempty all-ASCII-digits, or a spelling containing a non-digit.
    # This proves integer/real token alternatives disjoint without hardcoding
    # the RealValue expression or sampling possible numeric strings.
    def lexical_categories(row):
        fields, kind = row['fields'], row['kind']
        if kind == 'RuleCall':
            values = lexical_categories(source[ref(fields['rule'])]['fields']['alternatives'])
        elif kind == 'Keyword':
            value = fields['value']
            values = {0 if not value else 1 if all('0' <= c <= '9' for c in value) else 2}
        elif kind == 'CharacterRange':
            left, right = (ord(fields[key]['fields']['value']) for key in ['left', 'right'])
            values = ({1} if left <= ord('9') and right >= ord('0') else set())
            if left < ord('0') or right > ord('9'): values.add(2)
        elif kind in ['Group', 'Alternatives']:
            children = [lexical_categories(c) for c in fields['elements']]
            if kind == 'Alternatives':
                values = set().union(*children)
            else:
                values = {0}
                for child in children:
                    values = {max(a, b) for a in values for b in child}
        else:
            raise ValueError('Unsupported numeric language partition: ' + row['id'])
        if fields.get('cardinality') in ['?', '*']:
            values.add(0)
        return values

    for root in roots:
        rule(root)

    summaries = {identity: (set(), False) for identity in programs}

    unsupported_prediction_sites = {}
    def prediction_failure(node, message):
        if not partial_predictions:
            raise ValueError(message + ': ' + node['id'])
        unsupported_prediction_sites[node['id']] = message

    def nullable_entry_decisions(node, active=frozenset()):
        # A mandatory nullable call is always executed. Its internal entry
        # decisions, not a synthetic caller-side lookahead, decide consumption.
        # Prove every token-consuming nullable prefix is decision-controlled.
        if node['id'] in active:
            return None
        active = active | {node['id']}
        decision = node.get('decision', {})
        if node['cardinality'] in ['?', '*']:
            return {node['id']} if 'exit_alternative' in decision else None
        kind = node['kind']
        if kind in ['create', 'capture']:
            return set()
        if kind == 'call':
            return nullable_entry_decisions(programs[node['rule']]['body'], active)
        if kind in ['assign', 'contain', 'append_operand', 'link', 'cross_reference']:
            return nullable_entry_decisions(node['terminal'], active)
        if kind == 'choice':
            return {node['id']} if decision else None
        if kind == 'sequence':
            dependencies = set()
            for child in node['elements']:
                resolved = nullable_entry_decisions(child, active)
                if resolved is None:
                    return None
                dependencies.update(resolved)
            return dependencies
        return None

    def first(node, bare=False, check=True):
        kind = node['kind']
        nullable = False
        if kind in ['keyword', 'enum']:
            heads = {node.get('value', node.get('token'))}
        elif kind in ['create', 'capture']:
            heads, nullable = set(), True
        elif kind == 'name':
            heads = {'<Name>'}
        elif kind == 'terminal':
            if node['carrier'] == 'number':
                categories = lexical_categories(source[node['rule']]['fields']['alternatives'])
                heads = {'<number:' + str(c) + '>' for c in categories if c}
                nullable = 0 in categories
            else:
                heads = {'<' + node['carrier'] + '>'}
        elif kind == 'token_datatype':
            heads, nullable = first(node['syntax'], check=False)
        elif kind == 'datatype':
            categories = lexical_categories(source[node['id']]['fields']['alternatives'])
            if 0 in categories:
                raise ValueError('Nullable numeric datatype: ' + node['id'])
            heads = {'<number:' + str(c) + '>' for c in categories}
        elif kind == 'call':
            heads, nullable = summaries[node['rule']]
            heads = set(heads)
        elif kind in ['assign', 'contain', 'append_operand', 'link', 'cross_reference']:
            heads, nullable = first(node['terminal'], check=check)
        else:
            heads = set()
            children = [first(child, check=check) for child in node['elements']]
            if kind == 'choice':
                unguarded = set()
                processed = []
                for child, (child_heads, child_empty) in zip(node['elements'], children):
                    if check and not node.get('decision') and child_empty and len(children) > 1:
                        prediction_failure(node, 'Ambiguous fragment alternatives')
                    if check and not node.get('decision') and unguarded & child_heads:
                        current = prefixes(child, bare=True)
                        for previous in node['elements'][:len(processed)]:
                            if previous.get('predicated') or previous.get('first_set_predicated'):
                                continue
                            if not (first(previous, check=False)[0] & child_heads):
                                continue
                            earlier = prefixes(previous, bare=True)
                            if not disjoint(earlier, current):
                                prediction_failure(node, 'Ambiguous fragment alternatives')
                            previous['prediction'] = sorted(earlier)
                        child['prediction'] = sorted(current)
                    processed.append(child)
                    heads.update(child_heads)
                    if not (child.get('predicated') or child.get('first_set_predicated')):
                        unguarded.update(child_heads)
                    nullable |= child_empty
            else:
                nullable = True
                for index, (child_heads, child_empty) in enumerate(children):
                    if child_empty:
                        for later_heads, later_empty in children[index + 1:]:
                            if check and child_heads & later_heads and not (node['elements'][index].get('predicated') or node['elements'][index].get('first_set_predicated') or 'exit_alternative' in node['elements'][index].get('decision', {})):
                                child = node['elements'][index]
                                dependencies = nullable_entry_decisions(child) if experimental_graph is not None else None
                                if dependencies:
                                    node.setdefault('nullable_entry_dependencies', {})[child['id']] = sorted(dependencies)
                                    if not later_empty:
                                        break
                                    continue
                                entering = prefixes(child, bare=True)
                                following = prefixes({'kind': 'sequence', 'cardinality': '',
                                                      'elements': node['elements'][index + 1:]}) - {()}
                                if () in entering or not disjoint(entering, following):
                                    prediction_failure(node, 'Ambiguous optional fragment sequence')
                                child['prediction'] = sorted(entering)
                            if not later_empty: break
                    if nullable:
                        heads.update(child_heads)
                    nullable &= child_empty
        if node['cardinality'] in ['*', '+'] and nullable:
            raise ValueError('Nullable fragment repetition: ' + node['id'])
        return heads, nullable or (not bare and node['cardinality'] in ['?', '*'])

    captures, operator_candidates = {}, {}
    def keywords(value):
        fields = value['fields']
        if fields.get('cardinality') or fields.get('predicated') or fields.get('firstSetPredicated'):
            return []
        if value['kind'] == 'Keyword':
            return [fields['value']]
        if value['kind'] == 'Alternatives':
            return [word for child in fields['elements'] for word in keywords(child)]
        if value['kind'] == 'RuleCall':
            target = source[ref(fields['rule'])]
            if ref(target['fields']['type']['fields']['classifier']) == ECORE_NS + 'EString':
                return keywords(target['fields']['alternatives'])
        return []

    def capture_inventory(value, identity, owner):
        if isinstance(value, dict):
            if value.get('kind') == 'Action' and value['fields'].get('feature'):
                action = node(value, owner)
                captures[value['id']] = dict(action, enclosing_rule=identity,
                    scope='individual action execution; enclosing rule remains independently scoped')
            if value.get('kind') == 'Group':
                children = value['fields']['elements']
                # Resolve an action's adjacent operator assignment from the structured grammar.
                if len(children) >= 2 and children[0]['kind'] == 'Action' and children[0]['fields'].get('feature'):
                    assignment = children[1]
                    if assignment['kind'] == 'Assignment' and assignment['fields'].get('feature') == 'operator':
                        target = ref(children[0]['fields']['type']['fields']['classifier'])
                        field = model.feature(target, 'operator')
                        if assignment['fields']['operator'] != '=' or field['type'] != ECORE_NS + 'EString':
                            raise ValueError('Unsupported capture operator assignment: ' + assignment['id'])
                        for word in keywords(assignment['fields']['terminal']):
                            operator_candidates.setdefault(word, []).append(children[0]['id'])
            for child in value.values():
                capture_inventory(child, identity, owner)
        elif isinstance(value, list):
            for child in value:
                capture_inventory(child, identity, owner)
    for identity, row in source.items():
        if row['kind'] == 'ParserRule':
            capture_inventory(row['fields']['alternatives'], identity,
                              ref(row['fields']['type']['fields']['classifier']))
    operator_captures = {word: ids[0] for word, ids in operator_candidates.items() if len(ids) == 1}

    # Compute nullable/FIRST by least fixed point before diagnosing cycles and
    # ambiguity. Nonproductive cycles must not be mistaken for empty matches.
    while True:
        updated = {identity: first(program['body'], check=False)
                   for identity, program in programs.items()}
        if updated == summaries:
            break
        summaries = updated

    def leading_calls(value):
        kind = value['kind']
        if kind == 'call':
            return {value['rule']}
        if kind in ['assign', 'contain', 'append_operand', 'link', 'cross_reference']:
            return leading_calls(value['terminal'])
        if kind in ['sequence', 'choice']:
            result = set()
            for child in value['elements']:
                result.update(leading_calls(child))
                if kind == 'sequence' and not first(child, check=False)[1]:
                    break
            return result
        return set()

    leading = {identity: leading_calls(program['body']) for identity, program in programs.items()}
    visited, visiting = set(), []
    def progress(identity):
        if identity in visiting:
            raise ValueError('Non-consuming recursive rule cycle: ' + ' -> '.join(visiting + [identity]))
        if identity in visited:
            return
        visiting.append(identity)
        for target in sorted(leading[identity]):
            progress(target)
        visiting.pop()
        visited.add(identity)
    for identity in programs:
        progress(identity)
    prediction_source = json.loads(PREDICTIONS.read_text(encoding='utf-8'))
    prediction_context = prediction_source['contexts'][context or EXPRESSIONS[:-2]]
    def decision_signature(value):
        result = decision_occurrence_signature(value)
        cardinality = value['cardinality']
        return ['repeat:' + cardinality, *result, 'repeat:end'] if cardinality else result
    def decision_occurrence_signature(value):
        result = decision_signature_body(value)
        if value.get('first_set_predicated'):
            guard = first_set_signature(value['id'], original_contexts[context or EXPRESSIONS[:-2]].get('first_set_predicates', {}))
            return ['predicate:begin', *guard, 'predicate:end', *result]
        hoisted = original_contexts.get(context or EXPRESSIONS[:-2], {}).get('hoisted_predicates', {}).get(value['id'])
        if hoisted is not None:
            guard = compiled_nodes.get(hoisted)
            if guard is None or not guard.get('predicated'):
                raise ValueError('Unbound hoisted predicate: ' + value['id'])
            return ['predicate:begin', *decision_signature_body(guard), 'predicate:end', *result]
        if value.get('predicated'):
            result = ['predicate:begin', *result, 'predicate:end', *result]
        return result
    def decision_signature_body(value):
        kind = value['kind']
        if kind == 'call':
            return ['call:' + value['rule'].split('::')[1]]
        if kind == 'keyword':
            return ['keyword:' + value['value']]
        if kind == 'enum':
            return ['keyword:' + value['token']]
        if kind in ['create', 'capture']:
            return []
        if kind in ['contain', 'link', 'cross_reference', 'assign', 'append_operand']:
            return decision_signature(value['terminal'])
        if kind == 'choice':
            result = ['choice:begin']
            for index, child in enumerate(value['elements']):
                if index:
                    result.append('choice:or')
                result.extend(decision_signature(child))
            return result + ['choice:end']
        if kind == 'sequence':
            return [symbol for child in value['elements'] for symbol in decision_signature(child)]
        raise ValueError('Unsupported upstream decision signature: ' + value['id'])
    decisions = {row['source_id']: row for name, row in prediction_context.items() if name != 'keywords'} if upstream_predictions else {}
    finite_context = None
    if experimental_graph is not None:
        if (experimental_graph['provenance']['grammar_sha256'] != hashlib.sha256(GRAMMAR.read_bytes()).hexdigest()
                or prediction_source['structured_grammar_sha256'] != original_source_sha256
                or experimental_graph['provenance']['pilot_revision'] != prediction_source['provenance']['pilot_commit']):
            raise ValueError('Changed finite graph grammar provenance')
        from evaluate_pilot_nfa import validate_graph
        finite_context = experimental_graph['contexts'][context]
        validate_graph(finite_context)
    compiled_nodes = {}
    def index_nodes(value):
        if isinstance(value, dict):
            if 'kind' in value and 'id' in value:
                compiled_nodes[value['id']] = copy.deepcopy(value)
            for child in value.values():
                index_nodes(child)
        elif isinstance(value, list):
            for child in value:
                index_nodes(child)
    for program in programs.values():
        index_nodes(program['body'])
    entry_sites = set()
    def find_entry_sites(value):
        if isinstance(value, dict):
            if value.get('kind') == 'sequence':
                children = value['elements']
                for index, child in enumerate(children):
                    heads, nullable = first(child, check=False)
                    if not nullable:
                        continue
                    follow = set()
                    for sibling in children[index + 1:]:
                        starts, empty = first(sibling, check=False)
                        follow.update(starts)
                        if not empty:
                            break
                    if heads & follow:
                        entry_sites.add(child['id'])
            for child in value.values():
                find_entry_sites(child)
        elif isinstance(value, list):
            for child in value:
                find_entry_sites(child)
    for program in programs.values():
        find_entry_sites(program['body'])
    def bind_decisions(value):
        if isinstance(value, list):
            for child in value:
                bind_decisions(child)
        elif isinstance(value, dict):
            # Scalar recognizers execute over characters/whole carriers, not the
            # upstream parser token stream. Their inner syntax is not a DFA site.
            if value.get('kind') in ['datatype', 'token_datatype', 'name', 'enum', 'terminal']:
                return
            imported = decisions.get(value.get('id'))
            if imported is not None and value.get('kind') == 'choice':
                unguarded, needs_prediction = set(), False
                for child in value['elements']:
                    heads, empty = first(child, check=False)
                    needs_prediction |= bool(unguarded & heads) or empty
                    if not (child.get('predicated') or child.get('first_set_predicated')):
                        unguarded.update(heads)
                if not needs_prediction:
                    imported = None
            if imported is not None and imported.get('mode') == 'entry' and value['id'] not in entry_sites:
                imported = None
            if imported is not None:
                signature = ([decision_occurrence_signature(value)] if imported.get('mode') == 'entry'
                             else [decision_signature(child) for child in value.get('elements', [])])
                if signature != imported['alternatives']:
                    raise ValueError('Upstream decision signature drift: ' + value['id'])
                if 'exit_alternative' in imported and imported['exit_cardinality'] != value['cardinality']:
                    raise ValueError('Upstream decision cardinality drift: ' + value['id'])
                if prediction_source['structured_grammar_sha256'] != original_source_sha256:
                    raise ValueError('Changed prediction grammar provenance')
                value['decision'] = dict(copy.deepcopy(imported), keywords=prediction_context['keywords'])
                bind_syntax_conditions(value['decision'], compiled_nodes, lambda node:
                    first_set_signature(node['id'], original_contexts[context or EXPRESSIONS[:-2]].get('first_set_predicates', {}))
                    if node.get('first_set_predicated') else decision_signature_body(node))
            if finite_context is not None and 'decision' not in value:
                finite = finite_context['decisions'].get(value.get('id'))
                # An explicitly guarded single-body occurrence already has an
                # entry decision: native zero-width syntax/FIRST probing. A
                # second context-insensitive graph entry can introduce spurious
                # enter/exit ambiguity. Choices still need branch prediction.
                if value.get('kind') != 'choice' and (value.get('predicated') or value.get('first_set_predicated')):
                    finite = None
                if finite is not None:
                    signature = ([decision_occurrence_signature(value)] if value['kind'] != 'choice'
                                 else [decision_signature(child) for child in value.get('elements', [])])
                    if signature != finite['alternatives'] or finite['cardinality'] != value['cardinality']:
                        raise ValueError('Finite graph decision signature drift: ' + value['id'])
                    branches, entries = len(signature), len(finite['entries'])
                    if entries != branches + bool(finite['cardinality']):
                        raise ValueError('Finite graph alternative count drift: ' + value['id'])
                    value['decision'] = dict(experimental_nfa=value['id'], start=0, states=[],
                                             keywords=prediction_context['keywords'])
                    if finite['cardinality']:
                        value['decision']['exit_alternative'] = branches
            for key, child in list(value.items()):
                if key != 'decision':
                    bind_decisions(child)
    for program in programs.values():
        bind_decisions(program['body'])

    prefixes = bounded_prefixes(programs, lambda n: first(n, bare=True, check=False))
    for program in programs.values():
        first(program['body'])

    if partial_predictions:
        def mark_unsupported(value):
            if isinstance(value, dict):
                if value.get('id') in unsupported_prediction_sites:
                    value['unsupported'] = unsupported_prediction_sites[value['id']]
                for child in list(value.values()):
                    mark_unsupported(child)
            elif isinstance(value, list):
                for child in value:
                    mark_unsupported(child)
        # Include embedded predicate probes: a copied syntax occurrence must
        # never bypass an unresolved prediction site in its original body.
        for program in programs.values():
            mark_unsupported(program['body'])

    experimental_fields = {}
    if finite_context is not None:
        guards = {}
        def collect_guards(value):
            if isinstance(value, dict):
                if value.get('predicated') or value.get('first_set_predicated'):
                    probe = copy.deepcopy(value)
                    probe.update(cardinality='', predicated=False, first_set_predicated=False)
                    guards[value['id']] = dict(kind='syntax', source_id=value['id'],
                        first_set=bool(value.get('first_set_predicated')), probe=probe)
                for key, child in value.items():
                    if key != 'decision':
                        collect_guards(child)
            elif isinstance(value, list):
                for child in value:
                    collect_guards(child)
        collect_guards(programs)
        for state in finite_context['states'].values():
            for edge in state['edges']:
                if edge['kind'] != 'predicate':
                    continue
                condition = edge['condition']
                if condition.get('kind') != 'syntax':
                    raise ValueError('Unsupported finite graph predicate kind')
                guard = guards.get(condition.get('source_id'))
                if guard is None or guard['first_set'] != bool(condition.get('first_set')):
                    raise ValueError('Unbound finite graph predicate: ' + str(condition.get('source_id')))
                signature = (first_set_signature(guard['source_id'], original_contexts[context].get('first_set_predicates', {}))
                             if guard['first_set'] else decision_signature_body(guard['probe']))
                if signature != condition['signature']:
                    raise ValueError('Finite graph predicate signature drift: ' + guard['source_id'])
        experimental_fields['experimental_guards'] = guards

    return {'schema': 'dev.mercurio.xtext-fragment-programs.v1', 'roots': roots, 'rules': programs,
            **experimental_fields,
            'literal_roots': [r for r in roots if r in LITERALS],
            'context': context, 'entry_rules': entry_rules,
            'partial_predictions': partial_predictions,
            'unsupported_prediction_sites': dict(sorted(unsupported_prediction_sites.items())),
            'capture_actions': captures, 'operator_captures': operator_captures,
            'execution_scope': {'production_roots': [r for r in roots if r.endswith(('::Identification', '::Documentation', '::TextualRepresentation', '::Comment')) or r in LITERALS or r == GRAPH_ROOT],
                                'focused_executor_only_roots': [r for r in roots if not r.endswith(('::Identification', '::Documentation', '::TextualRepresentation', '::Comment')) and r not in LITERALS and r != GRAPH_ROOT],
                                'semantic_qualification': 'open'},
            'boundaries': ['Name uses generated ID/keyword tables; legacy lexer quoted-token adapter, escape conversion and general lexical arbitration remain unqualified',
                           'Disjoint-choice programs and exact upstream reference-member DFAs; no general ANTLR prediction or recovery',
                           'Upstream cyclic prediction is signature/caller-graph pinned; native Name/keyword token adaptation and reference-member models have focused evidence; numeric/string logical tokens and BaseExpression/optional argument decisions have isolated Pilot controls; complete expression construction and linking remain open',
                           'Terminal recognition uses generated programs; EBoolean/EInt/finite EDouble conversion is handwritten, string escape spelling is retained pending converter qualification',
                           'Recursive fragments require a fixed-point proof that current-object type is preserved; type-changing fragment and scalar cycles remain unsupported',
                           'Single-keyword enum-returning datatype rules bind exact Ecore literal spellings; custom converters and multi-token enum conversion remain unqualified',
                           'Nonfragment calls return fresh current objects; actions bind subsequent assignments through current-type flow; individual operand captures are separately executable, not enclosing-rule support',
                           'Numeric datatype syntax executes over a legacy Number token; whitespace-separated numeric components and general token arbitration remain unqualified',
                           'Containment trees and typed deferred cross-references supported in selected roots; scope/link resolution is a separate Rust dependency; direct syntactic and first-set predicates supported; parameter guards and predicates inside scalar-rule projections unsupported; recursive model calls require token progress, disjoint prediction and bounded native call depth'],
            'outside_scope': sorted(set(source) - set(programs))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    grammar, ecore, semantics = (json.loads(p.read_text()) for p in [GRAMMAR, ECORE, SEMANTICS])
    doc = build(grammar, ecore, semantics)
    doc['language_programs'] = {
        language: build(grammar, ecore, semantics,
                        roots=[r for r in ROOTS if r.startswith(identity + '::') or r in LITERALS or r in [CHAIN, TYPE_MEMBER]] + CONTEXT_ROOTS[language],
                        context=identity)
        for language, identity in LANGUAGES.items()
    }
    # The shared literal AST adapter has no language parameter yet. Retain it
    # only while its entire admitted dependency closure is context invariant.
    neutral = build(grammar, ecore, semantics, roots=LITERALS)['rules']
    for program in doc['language_programs'].values():
        if any(program['rules'].get(identity) != rule for identity, rule in neutral.items()):
            raise ValueError('Literal adapter requires explicit language context')
    doc['source_sha256'] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [GRAMMAR, ECORE, SEMANTICS, PREDICTIONS]}
    text = json.dumps(doc, indent=2) + '\n'
    if args.check:
        if OUTPUT.read_text() != text:
            raise ValueError('Stale Xtext fragment programs')
    else:
        OUTPUT.write_text(text, encoding='utf-8', newline='\n')
    print(f"Native fragment programs current: {len(doc['roots'])} roots, {len(doc['rules'])} resolved rules")


if __name__ == '__main__':
    main()
