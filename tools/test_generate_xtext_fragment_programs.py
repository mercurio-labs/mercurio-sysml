import copy
import json
import unittest
from generate_xtext_fragment_programs import GRAMMAR, ECORE, SEMANTICS, ROOTS, NAME, LITERALS, EXPRESSIONS, build


class FragmentProgramTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grammar = json.loads(GRAMMAR.read_text())
        cls.ecore = json.loads(ECORE.read_text())
        cls.semantics = json.loads(SEMANTICS.read_text())

    def compile(self, grammar):
        # Mutation controls for the native generator exclude the upstream-DFA
        # root; its whole caller graph has a separate exact-provenance test.
        roots = ROOTS if grammar == self.grammar else [r for r in ROOTS if r not in [EXPRESSIONS + 'FeatureChainMember', EXPRESSIONS + 'InstantiatedTypeMember']]
        return build(grammar, self.ecore, self.semantics, roots=roots, upstream_predictions=grammar == self.grammar)

    def root(self, grammar):
        return next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == ROOTS[0])['fields']['alternatives']

    def test_nonunique_converter_binding_is_explicit_and_signature_checked(self):
        for language, title in [('kerml', 'KerML'), ('sysml', 'SysML')]:
            identity = f'org.omg.{language}.xtext.{title}::Nonunique'
            program = build(self.grammar, self.ecore, self.semantics, roots=[identity])
            self.assertEqual(program['rules'][identity]['converter'], 'pilot_nonunique_boolean')
            grammar = copy.deepcopy(self.grammar)
            rule = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == identity)
            rule['fields']['alternatives']['fields']['value'] = 'changed'
            with self.assertRaisesRegex(ValueError, 'Changed Nonunique converter binding'):
                build(grammar, self.ecore, self.semantics, roots=[identity], upstream_predictions=False)

    def test_partial_predictions_preserve_failures_without_softening_progress_checks(self):
        grammar, root = self.recursive_fixture()
        alternatives = root['fields']['alternatives']['fields']['elements']
        alternatives.append(copy.deepcopy(alternatives[1]))
        with self.assertRaisesRegex(ValueError, 'Ambiguous fragment alternatives'):
            build(grammar, self.ecore, self.semantics, roots=[root['id']])
        result = build(grammar, self.ecore, self.semantics, roots=[root['id']], partial_predictions=True)
        self.assertTrue(result['partial_predictions'])
        self.assertTrue(result['unsupported_prediction_sites'])
        found = set()
        def visit(value):
            if isinstance(value, dict):
                if value.get('id') in result['unsupported_prediction_sites']:
                    self.assertEqual(value['unsupported'], result['unsupported_prediction_sites'][value['id']])
                    found.add(value['id'])
                for item in value.values(): visit(item)
            elif isinstance(value, list):
                for item in value: visit(item)
        visit(result['rules'])
        self.assertEqual(found, set(result['unsupported_prediction_sites']))
        grammar, root = self.recursive_fixture()
        root['fields']['alternatives'] = root['fields']['alternatives']['fields']['elements'][0]['fields']['elements'][1]
        with self.assertRaisesRegex(ValueError, 'Non-consuming recursive rule cycle'):
            build(grammar, self.ecore, self.semantics, roots=[root['id']], partial_predictions=True)

    def test_candidate_document_roots_are_resolved_from_pinned_grammars(self):
        from generate_xtext_fragment_programs import PROFILE, LANGUAGES
        graph = json.loads((PROFILE / 'xtext-prediction-nfa.experimental.json').read_text(encoding='utf-8'))
        for language, context in LANGUAGES.items():
            root = context + '::RootNamespace'
            compiled = build(self.grammar, self.ecore, self.semantics,
                             roots=[EXPRESSIONS + 'OwnedExpression', root], context=context,
                             partial_predictions=True, experimental_graph=graph)
            self.assertEqual(compiled['entry_rules'][root], root)
            self.assertTrue(compiled['rules'][root]['construct'])
            self.assertTrue(compiled['rules'][root]['owner'].endswith('#//Namespace'))
            action = compiled['rules'][root]['body']['elements'][0]
            self.assertEqual(action['kind'], 'create')
            self.assertTrue(action['classifier'].endswith('#//Namespace'))
            self.assertFalse(compiled['unsupported_prediction_sites'])
            self.assertTrue(compiled['partial_predictions'])

    def test_finite_graph_generation_proves_nullable_calls_and_fails_closed(self):
        from generate_xtext_fragment_programs import PROFILE, LANGUAGES
        graph = json.loads((PROFILE / 'xtext-prediction-nfa.experimental.json').read_text(encoding='utf-8'))
        context = LANGUAGES['sysml']
        def compile_graph(value):
            return build(self.grammar, self.ecore, self.semantics,
                         roots=[EXPRESSIONS + 'OwnedExpression'], context=context,
                         partial_predictions=True, experimental_graph=value)
        compiled = compile_graph(graph)
        self.assertFalse(compiled['unsupported_prediction_sites'])
        self.assertEqual(len(compiled['experimental_guards']), 16)
        guarded = compiled['rules']['org.omg.sysml.xtext.SysML::FeatureSpecializationPart']['body']['elements'][0]['elements'][0]
        self.assertTrue(guarded['first_set_predicated'])
        self.assertNotIn('decision', guarded)
        for mutation in ['source_id', 'first_set', 'signature', 'kind']:
            changed = copy.deepcopy(graph)
            condition = next(edge['condition'] for state in changed['contexts'][context]['states'].values()
                             for edge in state['edges'] if edge['kind'] == 'predicate')
            if mutation == 'source_id': condition[mutation] = 'missing'
            elif mutation == 'first_set': condition[mutation] = not condition.get(mutation, False)
            elif mutation == 'kind': condition[mutation] = 'external_java'
            else: condition[mutation] = ['keyword:changed']
            with self.assertRaisesRegex(ValueError, 'finite graph predicate|Finite graph predicate'):
                compile_graph(changed)
        parent = 'org.omg.sysml.xtext.SysML::AssignmentNodeDeclaration'
        body = compiled['rules'][parent]['body']
        dependencies = set().union(*map(set, body['nullable_entry_dependencies'].values()))
        self.assertEqual(dependencies, {'org.omg.sysml.xtext.SysML::TargetParameter/alternatives/elements/0'})
        missing = copy.deepcopy(graph)
        for identity in dependencies:
            del missing['contexts'][context]['decisions'][identity]
        unsupported = compile_graph(missing)
        self.assertIn(parent + '/alternatives', unsupported['unsupported_prediction_sites'])
        self.assertIn('unsupported', unsupported['rules'][parent]['body'])
        changed = copy.deepcopy(graph)
        changed['contexts'][context]['decisions'][next(iter(dependencies))]['alternatives'][0].append('keyword:drift')
        with self.assertRaisesRegex(ValueError, 'Finite graph decision signature drift'):
            compile_graph(changed)
        changed = copy.deepcopy(graph)
        changed['provenance']['pilot_revision'] = 'wrong'
        with self.assertRaisesRegex(ValueError, 'Changed finite graph grammar provenance'):
            compile_graph(changed)
        with self.assertRaisesRegex(ValueError, 'isolated contextual partial'):
            build(self.grammar, self.ecore, self.semantics, context=context, experimental_graph=graph)

    def test_finite_nullable_feature_prefix_preserves_canonical_entry_evidence(self):
        from generate_xtext_fragment_programs import PROFILE, LANGUAGES
        graph = json.loads((PROFILE / 'xtext-prediction-nfa.experimental.json').read_text(encoding='utf-8'))
        context = LANGUAGES['kerml']
        def compile_graph(value):
            return build(self.grammar, self.ecore, self.semantics,
                         roots=[EXPRESSIONS + 'OwnedExpression'], context=context,
                         partial_predictions=True, experimental_graph=value)
        program = compile_graph(graph)
        self.assertFalse(program['unsupported_prediction_sites'])
        identity = 'org.omg.kerml.xtext.KerML::Feature/alternatives/elements/0/elements/0'
        body = program['rules']['org.omg.kerml.xtext.KerML::Feature']['body']['elements'][0]['elements'][0]
        dependencies = set().union(*map(set, body['nullable_entry_dependencies'].values()))
        self.assertEqual(len(dependencies), 2)
        for dependency in dependencies:
            missing = copy.deepcopy(graph)
            del missing['contexts'][context]['decisions'][dependency]
            fallback = compile_graph(missing)
            # A canonical DFA can independently satisfy the same dependency.
            # Removing only a redundant graph entry must not invalidate it.
            def decisions(value):
                if isinstance(value, dict):
                    if value.get('id') == dependency and 'decision' in value:
                        yield value['decision']
                    for child in value.values():
                        yield from decisions(child)
                elif isinstance(value, list):
                    for child in value:
                        yield from decisions(child)
            retained = list(decisions(fallback['rules']))
            if retained:
                self.assertTrue(all('experimental_nfa' not in row for row in retained))
                self.assertNotIn(identity, fallback['unsupported_prediction_sites'])
            else:
                self.assertIn(identity, fallback['unsupported_prediction_sites'])

    def test_discovered_decisions_partition_every_resolved_alternative(self):
        from export_pilot_prediction import decision_requests
        from generate_xtext_fragment_programs import PREDICTIONS
        document = json.loads(PREDICTIONS.read_text(encoding='utf-8'))
        for context, exported in document['contexts'].items():
            requests, missing = decision_requests(self.grammar, context)
            required = {row['source_id'] for row in requests} | set(missing)
            supported = {row['source_id'] for name, row in exported.items() if name != 'keywords'}
            unsupported = set(document['unsupported'][context])
            self.assertFalse(supported & unsupported)
            self.assertEqual(supported | unsupported, required)
        self.assertTrue(any('Deterministic analysis budget' in reason for rows in document['unsupported'].values()
                            for reason in rows.values()))

    def test_upstream_predicate_retry_keeps_exact_guard_binding(self):
        from export_pilot_prediction import bind_source_nodes
        from generate_xtext_fragment_programs import PREDICTIONS, LANGUAGES
        document = json.loads(PREDICTIONS.read_text(encoding='utf-8'))
        context = LANGUAGES['sysml']
        decision = document['contexts'][context]['SendNode#/elements/3']
        self.assertEqual(decision['analysis'], 'bounded_predicate_retry_k1')
        conditions = [condition for state in decision['states']
                      for condition in ([state['gate']] if 'gate' in state else [])
                      + [edge['condition'] for edge in state.get('predicate_edges', [])]
                      if condition['kind'] == 'syntax']
        self.assertTrue(conditions)
        self.assertTrue(all(c['source_id'].startswith(context + '::SendNode/alternatives/') for c in conditions))
        conditions[0]['signature'].append('keyword:changed_guard')
        with self.assertRaisesRegex(ValueError, 'Missing or ambiguous guarded source identity'):
            bind_source_nodes(document['contexts'], self.grammar)

    def test_resolved_first_set_signatures_preserve_language_context(self):
        from export_pilot_prediction import source_signature, first_set_signature
        for language, keyword in [('KerML', 'typed'), ('SysML', 'defined')]:
            context = f'org.omg.{language.lower()}.xtext.{language}'
            sets = self.grammar['resolution_contexts'][context]['first_set_predicates']
            root = next(r for g in self.grammar['grammars'] for r in g['rules']
                        if r['id'] == context + '::FeatureSpecializationPart')
            guard = root['fields']['alternatives']['fields']['elements'][0]['fields']['elements'][0]
            signature = source_signature(guard, sets)
            self.assertIn('keyword:' + keyword, signature)
            self.assertNotIn('keyword:' + ('defined' if keyword == 'typed' else 'typed'), signature)
            self.assertIn('predicate:begin', signature)
            self.assertIn('call:FeatureSpecialization', signature)
            with self.assertRaisesRegex(ValueError, 'Missing resolved first-set predicate'):
                first_set_signature(guard['id'], {})

    def test_hoisted_predicates_are_resolved_by_upstream_identity(self):
        from export_pilot_prediction import PredicateContext, decision_requests, source_signature
        context = 'org.omg.sysml.xtext.SysML'
        predicates = PredicateContext(self.grammar, context)
        self.assertEqual(len(predicates.hoisted), 1)
        source, guard = next(iter(predicates.hoisted.items()))
        self.assertTrue(source.startswith(context + '::CalculationBody/'))
        self.assertEqual(guard['id'], context + '::CalculationBodyPart/alternatives/elements/0')
        requests, missing = decision_requests(self.grammar, context)
        body = next(row for row in requests if row['key'] == 'CalculationBody')
        self.assertEqual(body['alternatives'][1], ['keyword:{', 'predicate:begin',
            'call:CalculationBodyItem', 'predicate:end', 'call:CalculationBodyPart', 'keyword:}'])
        self.assertNotIn(context + '::CalculationBody/alternatives', missing)
        from export_pilot_prediction import bind_source_nodes
        from generate_xtext_fragment_programs import PREDICTIONS
        document = json.loads(PREDICTIONS.read_text(encoding='utf-8'))
        omitted = copy.deepcopy(self.grammar)
        omitted['resolution_contexts'][context]['hoisted_predicates'] = {}
        with self.assertRaisesRegex(ValueError, 'Missing or ambiguous source decision identity'):
            bind_source_nodes(document['contexts'], omitted)
        changed = copy.deepcopy(self.grammar)
        changed['resolution_contexts'][context]['hoisted_predicates'][source] = 'missing::guard'
        with self.assertRaisesRegex(ValueError, 'Unresolved hoisted predicate identity'):
            PredicateContext(changed, context)

    def test_optional_metadata_entry_binds_only_at_overlapping_site(self):
        from generate_xtext_fragment_programs import LANGUAGES
        root = LANGUAGES['kerml'] + '::MetadataFeatureDeclaration'
        result = build(self.grammar, self.ecore, self.semantics, roots=[root], context=LANGUAGES['kerml'])
        entry = result['rules'][root]['body']['elements'][0]
        self.assertEqual(entry['decision']['mode'], 'entry')
        self.assertEqual(entry['decision']['exit_alternative'], 1)
        self.assertNotIn('decision', entry['elements'][0])
        with self.assertRaisesRegex(ValueError, 'Ambiguous optional'):
            build(self.grammar, self.ecore, self.semantics, roots=[root], context=LANGUAGES['kerml'], upstream_predictions=False)

    def test_contextual_decision_binding_preserves_repeated_endpoint_identity(self):
        from export_pilot_prediction import bind_source_nodes
        from generate_xtext_fragment_programs import PREDICTIONS, LANGUAGES
        document = json.loads(PREDICTIONS.read_text(encoding='utf-8'))
        context = LANGUAGES['kerml']
        for rule in ['Specialization', 'Conjugation', 'Disjoining', 'FeatureInverting', 'Subsetting', 'Redefinition']:
            for occurrence, position in enumerate([2, 4]):
                row = document['contexts'][context][rule + '#/elements/' + str(position)]
                self.assertEqual(row['contextual_binding']['occurrence'], occurrence)
                self.assertEqual(row['contextual_binding']['occurrences'], 2)
                self.assertTrue(row['source_id'].endswith('/elements/' + str(position)))
        changed = copy.deepcopy(document['contexts'])
        changed[context]['Specialization#/elements/2']['contextual_binding']['occurrence'] = 1
        with self.assertRaisesRegex(ValueError, 'Changed contextual source decision identity'):
            bind_source_nodes(changed, self.grammar)
        changed = copy.deepcopy(document['contexts'])
        changed[context]['Specialization#/elements/2']['contextual_binding']['rule_signature'].append('keyword:drift')
        with self.assertRaisesRegex(ValueError, 'Changed contextual source decision identity'):
            bind_source_nodes(changed, self.grammar)

    def test_pinned_syntactic_predicate_signatures_preserve_guard_and_cardinality(self):
        from export_pilot_prediction import source_signature, decision_signature, bind_source_nodes
        from generate_xtext_fragment_programs import PREDICTIONS, LANGUAGES
        root = next(r for g in self.grammar['grammars'] for r in g['rules']
                    if r['id'] == LANGUAGES['sysml'] + '::ActionBodyItem')
        guard = root['fields']['alternatives']['fields']['elements'][6]['fields']['elements'][1]
        expected = ['repeat:*', 'predicate:begin', 'call:TargetSuccessionMember',
                    'predicate:end', 'call:TargetSuccessionMember', 'repeat:end']
        self.assertEqual(source_signature(guard), expected)
        optional = copy.deepcopy(guard)
        optional['fields']['cardinality'] = '?'
        self.assertEqual(decision_signature(optional), [expected[1:-1]])
        changed = copy.deepcopy(guard)
        changed['fields']['predicated'] = False
        self.assertNotEqual(source_signature(changed), expected)
        changed = copy.deepcopy(guard)
        changed['fields']['cardinality'] = '+'
        self.assertEqual(source_signature(changed)[0], 'repeat:+')
        changed = copy.deepcopy(guard)
        changed['fields']['firstSetPredicated'] = True
        with self.assertRaisesRegex(ValueError, 'Missing resolved first-set predicate'):
            source_signature(changed)
        document = json.loads(PREDICTIONS.read_text(encoding='utf-8'))
        bind_source_nodes(document['contexts'], self.grammar)
        for position, occurrence in [(5, 0), (7, 1)]:
            row = document['contexts'][LANGUAGES['sysml']][f'ActionBodyItem#entry/elements/{position}/elements/0']
            self.assertEqual(row['contextual_binding']['occurrence'], occurrence)
            self.assertIn('predicate:begin', row['contextual_binding']['rule_signature'])
        # A surrounding guard change must invalidate the whole-rule proof used
        # to distinguish otherwise identical EmptySuccessionMember occurrences.
        grammar = copy.deepcopy(self.grammar)
        changed_root = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == root['id'])
        changed_root['fields']['alternatives']['fields']['elements'][6]['fields']['elements'][1]['fields']['predicated'] = False
        with self.assertRaisesRegex(ValueError, 'Changed contextual source decision identity'):
            bind_source_nodes(document['contexts'], grammar)

    def test_resolved_predicate_conditions_bind_only_verified_native_occurrences(self):
        from generate_xtext_fragment_programs import PREDICTIONS, LANGUAGES, bind_syntax_conditions
        document = json.loads(PREDICTIONS.read_text(encoding='utf-8'))
        decision = document['contexts'][LANGUAGES['sysml']]['TargetParameter#entry/elements/0']
        predicates = [edge['condition'] for state in decision['states'] for edge in state.get('predicate_edges', [])]
        self.assertTrue(any(p['kind'] == 'always' for p in predicates))
        syntax = next(p for p in predicates if p['kind'] == 'syntax')
        self.assertEqual(syntax['signature'], ['call:TargetBinding'])
        self.assertTrue(syntax['source_id'].endswith('/elements/0/elements/0'))
        # Controlled binding validates a compiled occurrence; it does not claim
        # the still-blocked TargetBinding model closure is implemented.
        node = {'id': syntax['source_id'], 'kind': 'call', 'rule': 'TargetBinding',
                'cardinality': '*', 'predicated': True}
        signature = lambda n: ['call:' + n['rule']]
        nodes = {node['id']: node}
        bound = copy.deepcopy(decision)
        bind_syntax_conditions(bound, nodes, signature)
        for state in bound['states']:
            conditions = ([state['gate']] if 'gate' in state else []) + [e['condition'] for e in state.get('predicate_edges', [])]
            for condition in conditions:
                if condition['kind'] == 'syntax':
                    self.assertEqual(condition['probe']['cardinality'], '')
                    self.assertFalse(condition['probe']['predicated'])
        self.assertEqual(node['cardinality'], '*')
        self.assertTrue(node['predicated'])
        for altered in [{}, {node['id']: dict(node, predicated=False)}, {node['id']: dict(node, rule='Other')}]:
            with self.assertRaisesRegex(ValueError, 'Unbound or changed native predicate'):
                bind_syntax_conditions(copy.deepcopy(decision), altered, signature)

    def test_nested_source_signatures_preserve_choice_and_repetition(self):
        from export_pilot_prediction import source_signature
        rule = next(r for g in self.grammar['grammars'] for r in g['rules']
                    if r['id'] == EXPRESSIONS + 'ClassificationExpression')
        child = copy.deepcopy(rule['fields']['alternatives']['fields']['elements'][0])
        before = source_signature(child)
        self.assertIn('repeat:?', before)
        self.assertIn('choice:begin', before)
        child['fields']['elements'][1]['fields']['cardinality'] = '*'
        after = source_signature(child)
        self.assertIn('repeat:*', after)
        self.assertNotEqual(before, after)

    def test_upstream_binding_resolves_unique_nested_source_nodes(self):
        from export_pilot_prediction import bind_source_nodes
        from generate_xtext_fragment_programs import PREDICTIONS, LANGUAGES
        document = json.loads(PREDICTIONS.read_text(encoding='utf-8'))
        bind_source_nodes(document['contexts'], self.grammar)
        for context in LANGUAGES.values():
            argument = document['contexts'][context]['ArgumentList']
            self.assertEqual(argument['source_id'], EXPRESSIONS + 'ArgumentList/alternatives/elements/1')
            self.assertEqual(argument['exit_alternative'], 2)

    def test_upstream_binding_rejects_ambiguous_nested_identity(self):
        from export_pilot_prediction import bind_source_nodes
        from generate_xtext_fragment_programs import PREDICTIONS
        document = json.loads(PREDICTIONS.read_text(encoding='utf-8'))
        changed = copy.deepcopy(self.grammar)
        rule = next(r for g in changed['grammars'] for r in g['rules']
                    if r['id'] == EXPRESSIONS + 'ArgumentList')
        duplicate = copy.deepcopy(rule['fields']['alternatives']['fields']['elements'][1])
        def rename(node):
            if isinstance(node, dict):
                if 'id' in node:
                    node['id'] = 'controlled/duplicate/' + node['id']
                for child in node.values():
                    rename(child)
            elif isinstance(node, list):
                for child in node:
                    rename(child)
        rename(duplicate)
        rule['fields']['alternatives']['fields']['elements'].append(duplicate)
        with self.assertRaisesRegex(ValueError, 'ambiguous source decision identity'):
            bind_source_nodes(document['contexts'], changed)

    def test_upstream_decision_binding_rejects_changed_caller_graph(self):
        from generate_xtext_fragment_programs import LANGUAGES
        root = EXPRESSIONS + 'FeatureChainMember'
        for context in LANGUAGES.values():
            compiled = build(self.grammar, self.ecore, self.semantics, roots=[root], context=context)
            self.assertEqual(len(compiled['rules']), 8)
            self.assertIn('decision', compiled['rules'][root]['body'])
        changed = copy.deepcopy(self.grammar)
        self.root(changed)['fields']['elements'][0]['fields']['elements'][0]['fields']['value'] = 'changed'
        with self.assertRaisesRegex(ValueError, 'Changed prediction grammar provenance'):
            build(changed, self.ecore, self.semantics, roots=[root])

    def test_language_dependent_calls_are_not_flattened(self):
        with self.assertRaisesRegex(ValueError, 'Context-dependent rule dispatch'):
            build(self.grammar, self.ecore, self.semantics,
                  roots=[EXPRESSIONS + 'ExpressionBodyMember'])

    def test_context_specialization_selects_overrides_and_preserves_types(self):
        from generate_xtext_context_controls import fixture
        from generate_xtext_fragment_programs import LANGUAGES
        grammar = fixture(self.grammar)
        root = EXPRESSIONS + 'ExpressionBodyMember'
        for context, expected in [(EXPRESSIONS[:-2], 'LiteralInteger'),
                                  (LANGUAGES['kerml'], 'LiteralBoolean'),
                                  (LANGUAGES['sysml'], 'LiteralString')]:
            result = build(grammar, self.ecore, self.semantics, roots=[root, EXPRESSIONS + 'ExpressionBody'], context=context)
            target = context + '::ExpressionBody'
            self.assertEqual(result['rules'][root]['body']['terminal']['rule'], target)
            self.assertEqual(result['entry_rules'][EXPRESSIONS + 'ExpressionBody'], target)
            self.assertEqual(result['rules'][target]['body']['rule'], EXPRESSIONS + expected)
            if context != EXPRESSIONS[:-2]:
                self.assertEqual(result['rules'][target]['override_of'], [EXPRESSIONS + 'ExpressionBody'])
            else:
                self.assertNotIn('override_of', result['rules'][target])

    def test_context_admission_fails_on_wrong_language_annotation_and_type(self):
        from generate_xtext_context_controls import fixture
        from generate_xtext_fragment_programs import LANGUAGES
        grammar = fixture(self.grammar)
        with self.assertRaisesRegex(ValueError, 'Root outside grammar context'):
            build(grammar, self.ecore, self.semantics, roots=[LANGUAGES['sysml'] + '::ExpressionBody'], context=LANGUAGES['kerml'])
        for mutation in ['annotation', 'fragment', 'type']:
            changed = copy.deepcopy(grammar)
            row = next(r for g in changed['grammars'] for r in g['rules'] if r['id'] == LANGUAGES['kerml'] + '::ExpressionBody')
            if mutation == 'annotation':
                row['fields']['annotations'][0]['fields']['name'] = 'Unknown'
            elif mutation == 'fragment':
                row['fields']['fragment'] = True
            else:
                row['fields']['type']['fields']['classifier']['$ref'] = 'https://www.omg.org/spec/SysML/20250201#//Membership'
            with self.assertRaisesRegex(ValueError, 'Unsupported rule annotation|Incompatible Override'):
                build(changed, self.ecore, self.semantics, roots=[EXPRESSIONS + 'ExpressionBodyMember'], context=LANGUAGES['kerml'])

    def test_context_resolution_precedes_recursion_analysis(self):
        from generate_xtext_context_controls import fixture
        from generate_xtext_fragment_programs import LANGUAGES
        grammar = fixture(self.grammar)
        context = LANGUAGES['kerml']
        row = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == context + '::ExpressionBody')
        call = row['fields']['alternatives']
        # Recur in this language only, after the inherited call is rebound.
        target = context + '::ExpressionBody'
        call['fields']['rule']['$ref'] = target
        grammar['resolution_contexts'][context]['rule_calls'][call['id']] = target
        with self.assertRaisesRegex(ValueError, 'Non-consuming recursive'):
            build(grammar, self.ecore, self.semantics, roots=[EXPRESSIONS + 'ExpressionBodyMember'], context=context)
        result = build(grammar, self.ecore, self.semantics, roots=[EXPRESSIONS + 'ExpressionBodyMember'], context=LANGUAGES['sysml'])
        self.assertIn(LANGUAGES['sysml'] + '::ExpressionBody', result['rules'])

    def test_generated_prediction_resolves_shared_prefixes_and_rejects_overlap(self):
        from generate_xtext_context_controls import prediction_fixture
        grammar = prediction_fixture(self.grammar)
        result = build(grammar, self.ecore, self.semantics, roots=[EXPRESSIONS + 'LiteralExpression'])
        body = result['rules'][EXPRESSIONS + 'LiteralExpression']['body']
        self.assertEqual(body['elements'][0]['prediction'], [('.', '[')])
        branches = body['elements'][1]['elements']
        self.assertEqual(branches[0]['prediction'], [('.', '<number:1>')])
        self.assertEqual(branches[1]['prediction'], [('.', '(')])
        row = next(r for g in grammar['grammars'] for r in g['rules']
                   if r['id'] == EXPRESSIONS + 'LiteralExpression')
        optional = row['fields']['alternatives']['fields']['elements'][0]
        optional['fields']['elements'][1]['fields']['value'] = '('
        with self.assertRaisesRegex(ValueError, 'Ambiguous optional fragment sequence'):
            build(grammar, self.ecore, self.semantics, roots=[row['id']])

    def test_prediction_proof_handles_punctuation_aliases_and_recursive_prefixes(self):
        from xtext_prediction import bounded_prefixes, disjoint
        self.assertFalse(disjoint({('..',)}, {('.', '.')}))
        self.assertFalse(disjoint({('<=',)}, {('<', '=')}))
        self.assertFalse(disjoint({('.',)}, {('.?', '{')}))
        self.assertTrue(disjoint({('.', '<Name>')}, {('.?', '{')}))
        self.assertFalse(disjoint({('<Name>',)}, {('identifier',)}))
        def keyword(value):
            return {'kind': 'keyword', 'value': value, 'cardinality': ''}
        recursive = {'kind': 'choice', 'cardinality': '', 'elements': [
            keyword('end'), {'kind': 'sequence', 'cardinality': '', 'elements': [
                keyword('('), {'kind': 'call', 'rule': 'recursive', 'cardinality': ''}]}]}
        prefixes = bounded_prefixes({'recursive': {'body': recursive}},
                                    lambda n: ({n['value']}, False))
        self.assertEqual(prefixes(recursive), {('end',), ('(', 'end'), ('(', '(')})
        repeated = {'kind': 'sequence', 'cardinality': '', 'elements': [
            dict(keyword('.'), cardinality='*'), keyword('end')]}
        self.assertEqual(prefixes(repeated), {('end',), ('.', 'end'), ('.', '.')})

    def test_pinned_expression_closures_retain_language_body_dependencies(self):
        from generate_xtext_fragment_programs import LANGUAGES
        for context in LANGUAGES.values():
            with self.assertRaisesRegex(ValueError, r'Ambiguous .*::(?:FeatureElement|ActionBodyItem)/alternatives'):
                build(self.grammar, self.ecore, self.semantics,
                      roots=[EXPRESSIONS + 'AdditiveExpression'], context=context)

    def test_full_syntactic_predicate_guards_an_overlapping_choice(self):
        grammar = copy.deepcopy(self.grammar)
        row = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == EXPRESSIONS + 'LiteralExpression')
        integer = next(c for c in row['fields']['alternatives']['fields']['elements']
                       if c['fields']['rule']['$ref'] == EXPRESSIONS + 'LiteralInteger')
        guarded = {'id':'controlled/predicate', 'kind':'Group',
                   'fields':{'cardinality':None,'predicated':True,'elements':[
                       copy.deepcopy(integer), {'id':'controlled/suffix','kind':'Keyword',
                                                'fields':{'cardinality':None,'value':';'}}]}}
        row['fields']['alternatives']['fields']['elements'] = [guarded, integer]
        program = build(grammar, self.ecore, self.semantics, roots=[row['id']], upstream_predictions=False)
        self.assertTrue(program['rules'][row['id']]['body']['elements'][0]['predicated'])
        guarded['fields']['predicated'] = False
        guarded['fields']['firstSetPredicated'] = True
        first_set = build(grammar, self.ecore, self.semantics, roots=[row['id']], upstream_predictions=False)
        self.assertTrue(first_set['rules'][row['id']]['body']['elements'][0]['first_set_predicated'])
        guarded['fields']['firstSetPredicated'] = False
        with self.assertRaisesRegex(ValueError, 'Ambiguous fragment alternatives'):
            build(grammar, self.ecore, self.semantics, roots=[row['id']], upstream_predictions=False)

    def test_specialized_projection_cannot_drop_a_predicate(self):
        for key in ['predicated', 'firstSetPredicated']:
            grammar = copy.deepcopy(self.grammar)
            name = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == NAME)
            name['fields']['alternatives']['fields'][key] = True
            with self.assertRaisesRegex(ValueError, 'Unsupported predicate projection'):
                self.compile(grammar)
        grammar = copy.deepcopy(self.grammar)
        scalar = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == EXPRESSIONS + 'BooleanValue')
        scalar['fields']['alternatives']['fields']['predicated'] = True
        with self.assertRaisesRegex(ValueError, 'Unsupported predicate projection'):
            self.compile(grammar)

    def test_enum_datatype_keywords_use_ecore_literal_spellings(self):
        root = 'org.omg.sysml.xtext.SysML::TimeTriggerKind'
        result = self.compile(self.grammar)['rules'][root]
        self.assertTrue(result['scalar'])
        self.assertEqual([v['literal'] for v in result['body']['elements']], ['at','after'])
        self.assertTrue(all(v['ecore_literal'].startswith(result['owner']+'/literal:') for v in result['body']['elements']))
        grammar = copy.deepcopy(self.grammar)
        row = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == root)
        row['fields']['alternatives']['fields']['elements'][0]['fields']['value'] = 'unknown'
        with self.assertRaisesRegex(ValueError,'Unknown Ecore enum spelling'):
            self.compile(grammar)

    def test_resolved_programs_and_explicit_boundary(self):
        programs = self.compile(self.grammar)
        self.assertEqual(len(programs['rules']), 45)
        self.assertEqual(len(programs['roots']), 18)
        assignment = programs['rules'][ROOTS[0]]['body']['elements'][1]
        self.assertEqual(assignment['feature'], 'declared_name')
        self.assertTrue(assignment['feature_id'].endswith('/Element/declaredName'))
        self.assertTrue(programs['outside_scope'])

    def test_construction_and_datatype_bindings_are_resolved(self):
        programs = self.compile(self.grammar)['rules']
        for identity in [EXPRESSIONS + name for name in ['LiteralBoolean', 'LiteralInteger', 'LiteralString']]:
            self.assertTrue(programs[identity]['construct'])
            self.assertEqual(programs[identity]['body']['feature'], 'value')
        self.assertTrue(programs[EXPRESSIONS + 'BooleanValue']['scalar'])
        self.assertEqual(programs[EXPRESSIONS + 'DECIMAL_VALUE']['body']['carrier'], 'number')

    def test_unsupported_value_assembly_and_terminal_type_fail_closed(self):
        for name, mutate in [('BooleanValue', 'assembly'), ('DECIMAL_VALUE', 'type')]:
            grammar = copy.deepcopy(self.grammar)
            row = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == EXPRESSIONS + name)
            if mutate == 'assembly':
                row['fields']['alternatives']['kind'] = 'Group'
            else:
                row['fields']['type']['fields']['classifier']['$ref'] = 'http://www.eclipse.org/emf/2002/Ecore#//EDouble'
            with self.assertRaisesRegex(ValueError, 'Unsupported datatype|Changed terminal'):
                self.compile(grammar)

    def test_source_keyword_change_changes_execution_program(self):
        grammar = copy.deepcopy(self.grammar)
        self.root(grammar)['fields']['elements'][0]['fields']['elements'][0]['fields']['value'] = '['
        program = self.compile(grammar)['rules'][ROOTS[0]]
        self.assertEqual(program['body']['elements'][0]['elements'][0]['value'], '[')

    def test_predicates_and_unknown_assignments_fail_closed(self):
        for key in ['guardCondition']:
            grammar = copy.deepcopy(self.grammar)
            self.root(grammar)['fields'][key] = True
            with self.assertRaisesRegex(ValueError, 'Unsupported predicate'):
                self.compile(grammar)
        grammar = copy.deepcopy(self.grammar)
        self.root(grammar)['fields']['elements'][1]['fields']['feature'] = 'unknownFeature'
        with self.assertRaisesRegex(ValueError, 'Missing or ambiguous Ecore feature'):
            self.compile(grammar)

    def test_overlapping_alternatives_and_nullable_loops_fail(self):
        grammar = copy.deepcopy(self.grammar)
        elements = self.root(grammar)['fields']['elements']
        elements.append(copy.deepcopy(elements[0]))
        with self.assertRaisesRegex(ValueError, 'Ambiguous fragment alternatives'):
            self.compile(grammar)
        grammar = copy.deepcopy(self.grammar)
        root = self.root(grammar)
        root['fields']['cardinality'] = '*'
        root['fields']['elements'] = [root['fields']['elements'][1]]
        root['fields']['elements'][0]['fields']['cardinality'] = '?'
        with self.assertRaisesRegex(ValueError, 'Nullable fragment repetition'):
            self.compile(grammar)

    def test_actions_and_hidden_overrides_are_not_silently_accepted(self):
        grammar = copy.deepcopy(self.grammar)
        self.root(grammar)['kind'] = 'Action'
        with self.assertRaisesRegex(ValueError, 'Unsupported assigned/malformed action'):
            self.compile(grammar)
        grammar = copy.deepcopy(self.grammar)
        row = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == ROOTS[0])
        row['fields']['definesHiddenTokens'] = True
        with self.assertRaisesRegex(ValueError, 'hidden override'):
            self.compile(grammar)

    def action(self, grammar):
        return next(r for g in grammar['grammars'] for r in g['rules']
                    if r['id'] == EXPRESSIONS + 'LiteralInfinity')['fields']['alternatives']['fields']['elements'][0]

    def test_simple_action_preserves_ecore_identity_and_rejects_dependencies(self):
        programs = self.compile(self.grammar)['rules']
        action = programs[EXPRESSIONS + 'LiteralInfinity']['body']['elements'][0]
        self.assertEqual(action['kind'], 'create')
        self.assertTrue(action['classifier'].endswith('#//LiteralInfinity'))
        for mutate, error in [('assigned', 'capture delegate'), ('unknown', 'concrete Ecore'), ('transition', 'type transition')]:
            grammar = copy.deepcopy(self.grammar)
            fields = self.action(grammar)['fields']
            if mutate == 'assigned':
                fields.update(feature='ownedRelationship', operator='+=')
            else:
                target = 'NoSuchClass' if mutate == 'unknown' else 'LiteralBoolean'
                fields['type']['fields']['classifier']['$ref'] = 'https://www.omg.org/spec/SysML/20250201#//' + target
            with self.assertRaisesRegex(ValueError, error):
                self.compile(grammar)
        grammar = copy.deepcopy(self.grammar)
        self.action(grammar)['fields']['cardinality'] = '*'
        with self.assertRaisesRegex(ValueError, 'Nullable fragment repetition'):
            self.compile(grammar)

    def test_unassigned_call_binds_full_pinned_literal_family(self):
        result = self.compile(self.grammar)
        self.assertEqual(len(result['rules'][EXPRESSIONS + 'LiteralExpression']['body']['elements']), 5)
        grammar = copy.deepcopy(self.grammar)
        parent = next(r for g in grammar['grammars'] for r in g['rules']
                      if r['id'] == EXPRESSIONS + 'LiteralExpression')
        parent['fields']['type']['fields']['classifier']['$ref'] = 'https://www.omg.org/spec/SysML/20250201#//Package'
        with self.assertRaisesRegex(ValueError, 'Incompatible object-returning call'):
            build(grammar, self.ecore, self.semantics, roots=[EXPRESSIONS + 'LiteralExpression'])

    def test_numeric_language_partition_rejects_overlapping_integer_real_rules(self):
        grammar = copy.deepcopy(self.grammar)
        real = next(r for g in grammar['grammars'] for r in g['rules']
                    if r['id'] == EXPRESSIONS + 'RealValue')
        # Replacing the real rule by DECIMAL_VALUE creates real/integer overlap.
        call = copy.deepcopy(real['fields']['alternatives']['fields']['elements'][0]['fields']['elements'][0])
        call['fields']['cardinality'] = None
        real['fields']['alternatives'] = call
        with self.assertRaisesRegex(ValueError, 'Ambiguous fragment alternatives'):
            self.compile(grammar)

    def test_containment_and_cross_reference_bindings_are_resolved(self):
        programs = self.compile(self.grammar)['rules']
        member = programs['org.omg.kerml.xtext.KerML::MultiplicityExpressionMember']['body']
        self.assertEqual(member['kind'], 'contain')
        self.assertEqual(member['feature'], 'owned_related_element')
        self.assertEqual(member['terminal']['kind'], 'choice')
        reference = programs[EXPRESSIONS + 'FeatureReferenceMember']['body']
        self.assertEqual(reference['kind'], 'link')
        self.assertTrue(reference['terminal']['target'].endswith('#//Feature'))
        self.assertEqual(programs[EXPRESSIONS + 'QualifiedName']['body']['kind'], 'token_datatype')

    def test_token_datatype_rejects_unimplemented_scalar_carriers(self):
        grammar = copy.deepcopy(self.grammar)
        qualified = next(r for g in grammar['grammars'] for r in g['rules']
                         if r['id'] == EXPRESSIONS + 'QualifiedName')
        qualified['fields']['alternatives']['fields']['elements'][0]['fields']['rule']['$ref'] = EXPRESSIONS + 'STRING_VALUE'
        with self.assertRaisesRegex(ValueError, 'Unsupported token datatype syntax'):
            self.compile(grammar)

    def test_containment_type_and_write_policy_changes_fail_closed(self):
        for change, error in [('target', 'Containment value type mismatch'), ('derived', 'write policy'), ('changeable', 'write policy')]:
            ecore = copy.deepcopy(self.ecore)
            feature = next(f for f in ecore['features'] if f['id'].endswith('#//Relationship/ownedRelatedElement'))
            if change == 'target':
                feature['type'] = 'https://www.omg.org/spec/SysML/20250201#//Package'
            elif change == 'derived':
                feature['derived'] = True
            else:
                feature['changeable'] = False
            with self.assertRaisesRegex(ValueError, error):
                build(self.grammar, ecore, self.semantics)

    def expression_fixture(self):
        grammar = copy.deepcopy(self.grammar)
        rule = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == EXPRESSIONS + 'AdditiveExpression')
        # Controlled acyclic call context for the real additive group/action.
        for call in [rule['fields']['alternatives']['fields']['elements'][0],
                     rule['fields']['alternatives']['fields']['elements'][1]['fields']['elements'][2]['fields']['terminal']]:
            call['fields']['rule']['$ref'] = EXPRESSIONS + 'LiteralExpression'
        return grammar, rule

    def test_current_type_flow_binds_subtype_fields_and_repeated_captures(self):
        grammar, root = self.expression_fixture()
        result = build(grammar, self.ecore, self.semantics, roots=[root['id']])
        program = result['rules'][root['id']]
        loop = program['body']['elements'][1]
        action, operator, operand = loop['elements']
        self.assertEqual(action['kind'], 'capture')
        self.assertIn('https://www.omg.org/spec/SysML/20250201#//OperatorExpression', action['captured_types'])
        self.assertTrue(operator['feature_id'].endswith('/OperatorExpression/operator'))
        self.assertEqual(operand['kind'], 'append_operand')
        self.assertIn('https://www.omg.org/spec/SysML/20250201#//LiteralInteger', program['result_types'])
        self.assertIn('https://www.omg.org/spec/SysML/20250201#//OperatorExpression', program['result_types'])

    def test_optional_action_does_not_erase_uncaptured_type_path(self):
        grammar, root = self.expression_fixture()
        elements = root['fields']['alternatives']['fields']['elements']
        action, operator, _ = elements[1]['fields']['elements']
        action['fields']['cardinality'] = '?'
        root['fields']['alternatives']['fields']['elements'] = [elements[0], action, operator]
        with self.assertRaisesRegex(ValueError, 'Missing or ambiguous Ecore feature'):
            build(grammar, self.ecore, self.semantics, roots=[root['id']])

    def test_capture_inventory_and_delegate_drift_are_explicit(self):
        result = self.compile(self.grammar)
        self.assertEqual(len(result['capture_actions']), 26)
        action = result['capture_actions'][result['operator_captures']['+']]
        self.assertTrue(action['enclosing_rule'].endswith('::AdditiveExpression'))
        self.assertNotIn(action['enclosing_rule'], result['rules'])
        semantics = copy.deepcopy(self.semantics)
        delegate = next(b for b in semantics['delegate_bindings'] if b['element'].endswith('/InvocationExpression/operand'))
        delegate['candidate_source_classes'] = ['different.Delegate']
        with self.assertRaisesRegex(ValueError, 'Unsupported capture delegate'):
            build(self.grammar, self.ecore, semantics)

        ecore = copy.deepcopy(self.ecore)
        feature = next(f for f in ecore['features'] if f['id'].endswith('/InvocationExpression/operand'))
        feature['type'] = 'https://www.omg.org/spec/SysML/20250201#//Element'
        with self.assertRaisesRegex(ValueError, 'Unsupported capture delegate'):
            build(self.grammar, ecore, self.semantics)

    def recursive_fixture(self):
        grammar = copy.deepcopy(self.grammar)
        source = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == EXPRESSIONS + 'LiteralExpression')
        root = copy.deepcopy(source)
        root['id'] = 'test::Recursive'
        root['fields']['name'] = 'Recursive'
        root['fields']['type']['fields']['classifier']['$ref'] = 'https://www.omg.org/spec/SysML/20250201#//Expression'
        def node(kind, **fields):
            return {'id':'controlled/' + kind, 'kind':kind, 'fields':dict(cardinality=None, **fields)}
        recurse = node('RuleCall', rule={'$ref':root['id']}, arguments=[], explicitlyCalled=False)
        literal = node('RuleCall', rule={'$ref':EXPRESSIONS+'LiteralInteger'}, arguments=[], explicitlyCalled=False)
        group = node('Group', elements=[node('Keyword', value='('), recurse, node('Keyword', value=')')])
        root['fields']['alternatives'] = node('Alternatives', elements=[group,literal])
        grammar['grammars'][0]['rules'].append(root)
        return grammar, root

    def test_recursive_fragments_preserve_caller_type_without_weakening_bindings(self):
        grammar, fragment = self.recursive_fixture()
        fragment['fields']['fragment'] = True
        fragment['fields']['type']['fields']['classifier']['$ref'] = 'https://www.omg.org/spec/SysML/20250201#//Element'
        call = {'id':'fragment/name', 'kind':'RuleCall',
                'fields':{'cardinality':None,'rule':{'$ref':NAME}}}
        leaf = {'id':'fragment/assignment','kind':'Assignment',
                'fields':{'cardinality':None,'feature':'declaredName','operator':'=','terminal':call}}
        fragment['fields']['alternatives']['fields']['elements'][1] = leaf
        caller = copy.deepcopy(fragment)
        caller['id'] = 'test::Caller'
        caller['fields']['name'] = 'Caller'
        caller['fields']['fragment'] = False
        caller['fields']['type']['fields']['classifier']['$ref'] = 'https://www.omg.org/spec/SysML/20250201#//PartUsage'
        caller['fields']['alternatives'] = {'id':'caller/body','kind':'Group','fields':{
            'cardinality':None,'elements':[
                {'id':'caller/fragment','kind':'RuleCall','fields':{'cardinality':None,'rule':{'$ref':fragment['id']}}},
                {'id':'caller/ordered','kind':'Assignment','fields':{'cardinality':None,'feature':'isOrdered','operator':'?=',
                    'terminal':{'id':'caller/keyword','kind':'Keyword','fields':{'cardinality':None,'value':'ordered'}}}}]}}
        grammar['grammars'][0]['rules'].append(caller)
        result = build(grammar,self.ecore,self.semantics,roots=[caller['id']])
        self.assertTrue(result['rules'][fragment['id']]['preserves_current'])
        self.assertEqual(result['rules'][caller['id']]['result_types'], ['https://www.omg.org/spec/SysML/20250201#//PartUsage'])
        self.assertTrue(result['rules'][caller['id']]['body']['elements'][1]['feature_id'].endswith('/Feature/isOrdered'))
        nonprogress = copy.deepcopy(grammar)
        cycle = next(r for g in nonprogress['grammars'] for r in g['rules'] if r['id'] == fragment['id'])
        cycle['fields']['alternatives'] = {'id':'no-progress','kind':'RuleCall',
                                          'fields':{'cardinality':None,'rule':{'$ref':fragment['id']}}}
        with self.assertRaisesRegex(ValueError,'Non-consuming recursive rule cycle'):
            build(nonprogress,self.ecore,self.semantics,roots=[caller['id']])
        # Mutually recursive fragment calls retain the same proof.
        other = copy.deepcopy(fragment)
        other['id'] = 'test::OtherFragment'
        other['fields']['name'] = 'OtherFragment'
        grammar['grammars'][0]['rules'].append(other)
        fragment['fields']['alternatives']['fields']['elements'][0]['fields']['elements'][1]['fields']['rule']['$ref'] = other['id']
        result = build(grammar,self.ecore,self.semantics,roots=[caller['id']])
        self.assertTrue(result['rules'][other['id']]['preserves_current'])
        # A reachable current-object replacement invalidates the entire cycle.
        other['fields']['alternatives']['fields']['elements'][1] = {
            'id':'replacement','kind':'RuleCall','fields':{'cardinality':None,'rule':{'$ref':EXPRESSIONS+'LiteralInteger'}}}
        with self.assertRaisesRegex(ValueError,'Unsupported recursive scalar/fragment'):
            build(grammar,self.ecore,self.semantics,roots=[caller['id']])

    def test_recursive_model_calls_have_finite_prediction(self):
        grammar, root = self.recursive_fixture()
        result = build(grammar, self.ecore, self.semantics, roots=[root['id']])
        self.assertIn(root['id'], result['rules'])
        self.assertTrue(result['rules'][root['id']]['construct'])
        self.assertIn('https://www.omg.org/spec/SysML/20250201#//Expression',
                      result['rules'][root['id']]['result_types'])
        # Mutually recursive call after '(' must also compile.
        other = copy.deepcopy(root)
        other['id'] = 'test::Other'
        other['fields']['name'] = 'Other'
        grammar['grammars'][0]['rules'].append(other)
        root['fields']['alternatives']['fields']['elements'][0]['fields']['elements'][1]['fields']['rule']['$ref'] = other['id']
        result = build(grammar, self.ecore, self.semantics, roots=[root['id']])
        self.assertIn(other['id'], result['rules'])

    def test_non_consuming_recursive_cycles_and_recursive_ambiguity_fail(self):
        for variant in ['direct','optional','mutual','ambiguous']:
            grammar, root = self.recursive_fixture()
            group, literal = root['fields']['alternatives']['fields']['elements']
            recurse = group['fields']['elements'][1]
            if variant == 'direct':
                root['fields']['alternatives'] = recurse
            elif variant == 'optional':
                group['fields']['elements'][0]['fields']['cardinality'] = '?'
            elif variant == 'mutual':
                other = copy.deepcopy(root)
                other['id'] = 'test::Other'
                other['fields']['name'] = 'Other'
                other['fields']['alternatives'] = copy.deepcopy(recurse)
                grammar['grammars'][0]['rules'].append(other)
                recurse['fields']['rule']['$ref'] = other['id']
                root['fields']['alternatives'] = recurse
            else:
                root['fields']['alternatives']['fields']['elements'].append(copy.deepcopy(literal))
            error = 'Ambiguous fragment alternatives' if variant == 'ambiguous' else 'Non-consuming recursive rule cycle'
            with self.subTest(variant=variant), self.assertRaisesRegex(ValueError, error):
                build(grammar, self.ecore, self.semantics, roots=[root['id']])

    def test_external_name_binding_type_and_repetition_drift_fail(self):
        for mutate in ['type', 'cardinality']:
            grammar = copy.deepcopy(self.grammar)
            row = next(r for g in grammar['grammars'] for r in g['rules'] if r['id'] == NAME)
            if mutate == 'type':
                row['fields']['type']['fields']['classifier']['$ref'] = 'http://www.eclipse.org/emf/2002/Ecore#//EInt'
            else:
                row['fields']['alternatives']['fields']['cardinality'] = '*'
            with self.assertRaisesRegex(ValueError, 'Changed external Name token binding'):
                self.compile(grammar)


if __name__ == '__main__':
    unittest.main()
