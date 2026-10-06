import copy,json,unittest
from pathlib import Path
from run_result_valuation_probe import validate
class ResultValuationEvaluationTests(unittest.TestCase):
 def setUp(self):self.doc=json.loads((Path(__file__).resolve().parents[1]/'docs/conformance/2026-08-support/result-valuation-pilot-controls.json').read_text(encoding='utf-8'))
 def test_complete_evaluation_keeps_dependency_failures_separate(self):
  validate(self.doc);self.assertEqual(sum(r['status']=='observed_component' for r in self.doc['controls']),704);self.assertEqual(sum(r['status']=='fixture_dependency_failure' for r in self.doc['controls']),64)
 def test_missing_and_duplicate_context_reject(self):
  d=copy.deepcopy(self.doc);d['controls'].pop()
  with self.assertRaises(AssertionError):validate(d)
  d=copy.deepcopy(self.doc);d['controls'][0]=d['controls'][1]
  with self.assertRaises(AssertionError):validate(d)
 def test_resolved_program_change_rejects(self):
  d=copy.deepcopy(self.doc);key=next(iter(d['methods']));d['methods'][key]={'changed':'body'}
  with self.assertRaises(AssertionError):validate(d)
 def test_unclassified_outcome_cannot_pass(self):
  d=copy.deepcopy(self.doc);d['controls'][0]['status']='qualified'
  with self.assertRaises(AssertionError):validate(d)
 def test_bad_owned_endpoint_and_effect_reject(self):
  d=copy.deepcopy(self.doc);r=next(r for r in d['controls'] if r['status']=='observed_component');r['canonical_value']=False
  with self.assertRaises(AssertionError):validate(d)
  r['canonical_value']=True;r['generals'][0]['kind']='Subclassification'
  with self.assertRaises(AssertionError):validate(d)
