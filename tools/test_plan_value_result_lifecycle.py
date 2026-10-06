
"""Failure controls for the whole V01 dependency plan, never qualification."""
import copy
import json
import unittest
from plan_value_result_lifecycle import BASE, EVIDENCE, META, build_plan, read

class LifecyclePlanTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = [
            read(BASE/"value-result-obligation-bundle.json"),
            read(BASE/"value-result-dependency-inventory.json"),
            read(META/"ecore-effective.extract.json"),
            read(META/"validators.inventory.extract.json"),
            [json.loads(line) for line in (EVIDENCE/"value-result-lifecycle-source-inspection.jsonl").read_text(encoding="utf-8").splitlines()],
            read(EVIDENCE/"value-result-lifecycle-parse-negative.jsonl")]
    def inputs_copy(self):
        return copy.deepcopy(self.inputs)
    def test_complete_resolved_plan_never_awards_native_credit(self):
        plan = build_plan(*self.inputs)
        self.assertEqual(plan["counts"]["external_symbols"],183)
        self.assertEqual(plan["native_complete_contexts"],0)
        self.assertEqual(plan["strict_families_qualified"],0)
        self.assertTrue(all(not d["native_semantically_verified"] for d in plan["dependencies"]))
        self.assertTrue(all(not s["mutation_applied"] and not s["native_context_complete"] for s in plan["source_contexts"]))
    def test_pruned_or_reordered_context_matrix_rejects(self):
        for mutation in ["missing","reordered"]:
            data = self.inputs_copy()
            if mutation == "missing":
                data[4].pop()
            else:
                data[4][0],data[4][1] = data[4][1],data[4][0]
            with self.assertRaisesRegex(ValueError,"All26"):
                build_plan(*data)
    def test_unrelated_blocker_cannot_establish_native_scope(self):
        data = self.inputs_copy()
        data[4][0]["status"] = "blocked"
        with self.assertRaisesRegex(ValueError,"unrelated implementation failure"):
            build_plan(*data)
    def test_wrong_negative_stage_or_source_is_rejected(self):
        for field,value in [("stage","construction_or_linking"),("input_files",["different.kerml"])]:
            data = self.inputs_copy()
            data[5][field] = value
            with self.assertRaisesRegex(ValueError,"parse-negative|Parse-negative"):
                build_plan(*data)
    def test_prepared_flags_or_publication_claims_reject(self):
        for mutation in ["prepared","published"]:
            data = self.inputs_copy()
            if mutation == "prepared":
                data[4][0]["inspection"]["constructed_elements"][0]["properties"]["is_implied_included"] = True
            else:
                data[4][0]["publication"] = "qualified"
            with self.assertRaisesRegex(ValueError,"prepared|publication"):
                build_plan(*data)
    def test_unknown_metaclass_and_duplicate_source_ids_reject(self):
        for mutation in ["unknown","duplicate"]:
            data = self.inputs_copy()
            graph = data[4][0]["inspection"]["constructed_elements"]
            if mutation == "unknown":
                graph[0]["kind"] = "SysML::UnknownKind"
            else:
                graph[1]["id"] = graph[0]["id"]
            with self.assertRaisesRegex(ValueError,"Unknown|identities/count"):
                build_plan(*data)
    def test_missing_check_or_unresolved_method_closure_rejects(self):
        data = self.inputs_copy()
        data[1]["applicable_check_candidates"].pop()
        with self.assertRaisesRegex(ValueError,"All35"):
            build_plan(*data)
        data = self.inputs_copy()
        identity = data[1]["applicable_check_candidates"][0]["id"]
        method = next(m for m in data[3]["methods"] if m["id"] == identity)
        method["resolution_status"] = "unresolved"
        with self.assertRaisesRegex(ValueError,"Unresolved imported"):
            build_plan(*data)

if __name__ == "__main__":
    unittest.main()
