import copy,json,unittest
from generate_action_redefinitions import INPUT,BINDINGS,generate

class ActionPolicyTests(unittest.TestCase):
    def setUp(self):
        self.data=json.loads(INPUT.read_text(encoding='utf-8'));self.bindings=json.loads(BINDINGS.read_text(encoding='utf-8'))
    def test_reviewed_policy_and_all_controls(self):
        code=generate(self.data,self.bindings);self.assertIn('AcceptActionUsage" => trigger_action',code)
    def test_resolved_policy_drift_rejected(self):
        self.data['methods']['accept_redefinitions'].reverse()
        with self.assertRaisesRegex(ValueError,'policy changed'):generate(self.data,self.bindings)
    def test_missing_duplicate_and_wrong_selector_controls_rejected(self):
        original=copy.deepcopy(self.data['controls'])
        for controls in [original[:-1],original[:-1]+original[:1]]:
            self.data['controls']=controls
            with self.assertRaisesRegex(ValueError,'control inventory'):generate(self.data,self.bindings)
        self.data['controls']=original;self.data['controls'][0]['default_redefined_feature']='wrong'
        with self.assertRaisesRegex(ValueError,'observation disagrees'):generate(self.data,self.bindings)
    def test_default_definitions_drift_rejected_by_independent_controls(self):
        self.data['defaults']['ActionUsage']['trigger']='Wrong::feature'
        with self.assertRaisesRegex(ValueError,'observation disagrees'):generate(self.data,self.bindings)
    def test_dispatch_drift_rejected(self):
        self.data['bindings']['ActionUsage']['addComputedRedefinitions']='unassessed'
        with self.assertRaisesRegex(ValueError,'dispatch'):generate(self.data,self.bindings)
    def test_ordinary_queries_are_separate_and_complete(self):
        self.data['ordinary_controls'].pop()
        with self.assertRaisesRegex(ValueError,'ordinary inventory'):generate(self.data,self.bindings)
    def test_ordinary_behavior_disagreement_rejected(self):
        self.data['ordinary_controls'][0]['redefined_features']=['unexpected']
        with self.assertRaisesRegex(ValueError,'ordinary observation'):generate(self.data,self.bindings)
    def test_selector_does_not_admit_unassessed_lifecycles(self):
        code=generate(self.data,self.bindings);self.assertNotIn('"SendActionUsage" => true',code);self.assertNotIn('"TerminateActionUsage" => true',code)
