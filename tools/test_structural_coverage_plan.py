import copy
import json
import unittest
from check_structural_source_coverage import ROOT, PROFILE
from structural_coverage_plan import PLAN_NAME, compile_plan, markdown, completion_accounting

class CoveragePlanTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.plan=json.loads((PROFILE/PLAN_NAME).read_text(encoding='utf-8'))
        cls.grammar=json.loads((PROFILE/'grammar.structure.extract.json').read_text(encoding='utf-8'))
        cls.ecore=json.loads((PROFILE/'ecore-semantics.extract.json').read_text(encoding='utf-8'))
    def compile(self, plan=None, grammar=None, ecore=None):
        return compile_plan(plan or self.plan, grammar or self.grammar, ecore or self.ecore, ROOT)
    def test_all_observed_constructs_and_operations_are_accounted(self):
        result=self.compile()
        rows={r['id']:r for r in result['rows']}
        self.assertEqual(len(rows),35)
        self.assertEqual(rows['ecore.delegate_bindings']['imported_instances'],398)
        operation_ids={e['id'] for e in self.ecore['elements'] if e['kind']=='EOperation'}
        self.assertTrue(operation_ids <= set(rows['ecore.operations']['upstream_ids']))
        self.assertEqual(result['required_open_rows'],34)
        self.assertEqual(rows['xtext.rule_parameters']['completion'],'not_applicable')
        self.assertIn('No exclusions',markdown(result))
    def test_strategy_closure_is_checked_separately_from_family_closure(self):
        result = self.compile()
        self.assertEqual(result['strategy_accounting']['closed'], 11)
        self.assertEqual(result['strategy_accounting']['required'], 14)
        self.assertEqual(result['completion_accounting']['qualified_feature_families'], 0)
        batches = {b['id']: (b['closed'], b['required']) for b in result['strategy_accounting']['batches']}
        self.assertEqual(batches, {'ordinary-redefinition-query': (3, 4), 'bounded-binary-crossing-lifecycle': (4, 5), 'usage-variation-contribution': (4, 5)})
        for index in range(len(self.plan['strategy_batches'])):
            for mutation in ['remove_batch', 'remove_obligation', 'add_obligation', 'close_without_evidence']:
                with self.subTest(batch=index, mutation=mutation):
                    plan = copy.deepcopy(self.plan)
                    if mutation == 'remove_batch': plan['strategy_batches'].pop(index)
                    elif mutation == 'remove_obligation': plan['strategy_batches'][index]['obligations'].pop()
                    elif mutation == 'add_obligation':
                        obligation = copy.deepcopy(plan['strategy_batches'][index]['obligations'][0])
                        obligation['id'] = 'unreviewed-addition'
                        plan['strategy_batches'][index]['obligations'].append(obligation)
                    else: plan['strategy_batches'][index]['obligations'][-1]['status'] = 'closed'
                    with self.assertRaises(ValueError): self.compile(plan)

    def test_variation_contract_preserves_required_exception_and_lifecycle(self):
        batch=next(b for b in self.plan['strategy_batches'] if b['id']=='usage-variation-contribution')
        self.assertEqual(len(batch['bindings']),47)
        self.assertIn('SuccessionAsUsage',batch['bindings'])
        obligations={o['id']:o for o in batch['obligations']}
        self.assertEqual(obligations['succession-variation-specialization']['status'],'closed')
        self.assertIn('73 active',obligations['succession-variation-specialization']['acceptance'])
        self.assertEqual(obligations['complete-usage-lifecycle']['status'],'open')
        self.assertIn('succession-variation-specialization',obligations['complete-usage-lifecycle']['dependencies'])

    def test_crossing_contract_keeps_real_source_lifecycle_open(self):
        batch = next(b for b in self.plan['strategy_batches'] if b['id'] == 'bounded-binary-crossing-lifecycle')
        self.assertEqual(batch['bindings'], ['Feature'])
        integrated = next(o for o in batch['obligations'] if o['id'] == 'real-source-integrated-lifecycle')
        self.assertEqual(integrated['status'], 'open')
        self.assertEqual(set(integrated['dependencies']), {o['id'] for o in batch['obligations'] if o['id'] != integrated['id']})
        self.assertIn('Base + ScalarValues + Links', integrated['acceptance'])
        self.assertIn('constructed_unqualified', integrated['acceptance'])

    def test_unused_fields_reopen_when_upstream_uses_them(self):
        grammar=copy.deepcopy(self.grammar)
        rule=next(r for g in grammar['grammars'] for r in g['rules'] if r['kind']=='ParserRule')
        rule['fields']['parameters']=[{'name':'newParameter'}]
        with self.assertRaisesRegex(ValueError, 'inventory or acceptance changed'):
            self.compile(grammar=grammar)
    def test_new_grammar_field_requires_review(self):
        grammar=copy.deepcopy(self.grammar)
        grammar['grammars'][0]['fields']['newBehavior']=True
        with self.assertRaisesRegex(ValueError,'Unreviewed Xtext fields'):self.compile(grammar=grammar)
    def test_new_ecore_field_requires_review(self):
        ecore=copy.deepcopy(self.ecore)
        ecore['source_tree']['attributes']['newBehavior']='true'
        with self.assertRaisesRegex(ValueError,'Unreviewed Ecore'):self.compile(ecore=ecore)
    def test_missing_kind_or_ecore_family_fails(self):
        for prefix in ['xtext.Action','ecore.operations']:
            plan=copy.deepcopy(self.plan)
            plan['rows']=[r for r in plan['rows'] if r['id']!=prefix]
            with self.assertRaises(ValueError):self.compile(plan)
    def test_missing_consumer_or_evidence_anchor_fails(self):
        plan=copy.deepcopy(self.plan)
        row=next(r for r in plan['rows'] if r['native_consumers'])
        row['native_consumers'][0]['anchor']='nonexistent_consumer_123456'
        with self.assertRaisesRegex(ValueError,'Missing evidence'):self.compile(plan)
    def test_extraction_cannot_close_or_exclude_requirements(self):
        for key,value in [('native_support','complete'),('completion','complete'),('required',False)]:
            plan=copy.deepcopy(self.plan)
            plan['rows'][0][key]=value
            with self.assertRaises(ValueError):self.compile(plan)
    def test_dependency_and_predicate_obligations_cannot_disappear(self):
        plan=copy.deepcopy(self.plan)
        del plan['dependencies']['scoping']
        with self.assertRaisesRegex(ValueError,'dependency mapping'):self.compile(plan)
        plan=copy.deepcopy(self.plan)
        plan['rows']=[r for r in plan['rows'] if r['id']!='xtext.predicates']
        with self.assertRaisesRegex(ValueError,'Behavioral Xtext'):self.compile(plan)
    def test_handwritten_semantic_boundary_cannot_disappear(self):
        plan=copy.deepcopy(self.plan)
        del plan['dependencies']['scoping']['boundary']
        with self.assertRaisesRegex(ValueError,'handwritten boundary'):self.compile(plan)

    def test_completion_counts_do_not_claim_behavioral_percentage(self):
        accounting=self.compile()['completion_accounting']
        self.assertEqual(accounting['required_feature_families'],34)
        self.assertEqual(accounting['qualified_feature_families'],0)
        self.assertEqual(accounting['unused_feature_families'],1)
        self.assertEqual(accounting['required_release_gates'],5)
        self.assertIsNone(accounting['overall_percent'])
        self.assertEqual(accounting['behavioral_denominator_status'],'unreviewed')

    def test_release_gates_cannot_disappear_duplicate_or_self_qualify(self):
        for mutation in ['remove','duplicate','qualify','empty_acceptance','empty_blocker']:
            plan=copy.deepcopy(self.plan)
            gates=plan['release_gates']
            if mutation=='remove': gates.pop()
            elif mutation=='duplicate': gates.append(copy.deepcopy(gates[0]))
            elif mutation=='qualify': gates[0]['status']='qualified'
            elif mutation=='empty_acceptance': gates[0]['acceptance']=''
            else: gates[0]['blocker']=''
            with self.assertRaises(ValueError): self.compile(plan)

    def test_inventory_fingerprint_tracks_pin_identity_not_evidence_prose(self):
        original=self.compile()['completion_accounting']['source_inventory_sha256']
        plan=copy.deepcopy(self.plan)
        plan['rows'][0]['native_consumers'][0]['scope']+=' Further explanation.'
        self.assertEqual(original,self.compile(plan)['completion_accounting']['source_inventory_sha256'])
        grammar=copy.deepcopy(self.grammar)
        grammar['grammars'][0]['id']+='-changed'
        with self.assertRaisesRegex(ValueError, 'inventory or acceptance changed'):
            self.compile(grammar=grammar)
        result=self.compile()
        result['rows'][0]['upstream_ids'].append('changed-identity')
        self.assertNotEqual(original,completion_accounting(self.plan,result['rows'],result['qualification_accounting'])['source_inventory_sha256'])

if __name__=='__main__':unittest.main()
