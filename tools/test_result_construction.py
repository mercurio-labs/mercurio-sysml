import copy
import json
import unittest
from export_pilot_result_construction import OUTPUT, render
class ResultConstructionTests(unittest.TestCase):
    def setUp(self): self.doc=json.loads(OUTPUT.read_text())
    def test_generated_dispatch_matches_resolved_bindings(self):
        generated=render(self.doc)
        for kind in self.doc["bindings"]: self.assertIn('"'+kind+'"',generated)
    def test_changed_constructor_rejected(self):
        self.doc["definitions"]["TypeUtil#addResultParameterTo/2"]["children"][0]["kind"]="WHILE_LOOP"
        with self.assertRaisesRegex(ValueError,"Changed resolved"): render(self.doc)
    def test_unknown_provider_rejected(self):
        self.doc["bindings"]["ConstructorExpression"]="ConstructorExpressionAdapter#addAdditionalMembers/0"
        with self.assertRaisesRegex(ValueError,"Unassessed"): render(self.doc)
    def test_omitted_context_rejected(self):
        self.doc["cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete"): render(self.doc)
    def test_nonidempotent_provider_rejected(self):
        self.doc["cases"][0]["idempotent"]=False
        with self.assertRaisesRegex(ValueError,"not idempotent"): render(self.doc)
