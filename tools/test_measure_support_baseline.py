import copy
import json
import unittest
from measure_support_baseline import ROOT, PROFILE, measure


class MeasurementTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = [json.loads(p.read_text(encoding="utf-8")) for p in [
            ROOT / "docs/conformance/2026-08-support/structural-source-coverage.json",
            PROFILE / "grammar.structure.extract.json",
            PROFILE / "xtext-fragment-programs.json",
            PROFILE / "ecore-semantics.extract.json",
            PROFILE / "xtext-finite-programs.experimental.json"]]

    def test_duplicate_language_program_does_not_inflate_admission(self):
        original = measure(*self.inputs)
        inputs = copy.deepcopy(self.inputs)
        inputs[2]["language_programs"]["duplicate"] = inputs[2]["language_programs"]["kerml"]
        self.assertEqual(original, measure(*inputs))

    def test_unknown_rule_and_unadmitted_root_reject(self):
        for field in ["rules", "roots"]:
            inputs = copy.deepcopy(self.inputs)
            if field == "rules": inputs[2][field]["unknown"] = {}
            else: inputs[2][field].append("unknown")
            with self.assertRaises(ValueError): measure(*inputs)

    def test_admission_does_not_close_semantic_or_release_gates(self):
        inputs = copy.deepcopy(self.inputs)
        inputs[2]["rules"].update({r["id"]: {} for g in inputs[1]["grammars"] for r in g["rules"]})
        result = measure(*inputs)
        self.assertEqual(result["strict_generated_admission"]["rule_percent"], 100)
        self.assertIsNone(result["overall_completion_percent"])
        self.assertEqual(result["completion_gates"]["qualified_release_gates"], 0)
        self.assertTrue(all(v["fully_qualified"] == 0 for v in result["families"].values()))


    def test_candidate_is_separate_from_strict_admission_and_closure(self):
        result = measure(*self.inputs)
        self.assertEqual(result["strict_generated_admission"]["rule_count"], 141)
        self.assertEqual(result["candidate_emission"]["rule_count"], 718)
        self.assertEqual(result["candidate_emission"]["rules_by_language"], {"kerml": 275, "sysml": 542})
        self.assertEqual(len(result["candidate_emission"]["declared_but_not_emitted_ids"]), 9)
        self.assertIsNone(result["overall_completion_percent"])
        self.assertEqual(result["completion_gates"]["qualified_release_gates"], 0)

    def test_unknown_candidate_rule_and_root_reject(self):
        for field in ["rules", "roots"]:
            inputs = copy.deepcopy(self.inputs)
            program = inputs[4]["language_programs"]["kerml"]
            if field == "rules": program[field]["unknown"] = {}
            else: program[field].append("unknown")
            with self.assertRaisesRegex(ValueError, "Candidate emitted"):
                measure(*inputs)

    def test_overlapping_candidate_programs_count_shared_rules_once(self):
        inputs = copy.deepcopy(self.inputs)
        before = measure(*inputs)["candidate_emission"]
        inputs[4]["language_programs"]["duplicate"] = inputs[4]["language_programs"]["kerml"]
        after = measure(*inputs)["candidate_emission"]
        self.assertEqual(before["rule_ids"], after["rule_ids"])
        self.assertEqual(before["entry_root_ids"], after["entry_root_ids"])


if __name__ == "__main__":
    unittest.main()
