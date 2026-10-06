"""Focused guards for the frozen trial's denominator and complete caller bodies."""
from copy import deepcopy
import json
import unittest
from check_bounded_translation import SCOPE, AST, ROOT, BASE, HELPER, report

class BoundedScopeTests(unittest.TestCase):
    def setUp(self):
        self.scope = json.loads(SCOPE.read_text(encoding="utf-8"))
        self.ast = json.loads(AST.read_text(encoding="utf-8"))
        self.boundary = json.loads((ROOT / "docs/conformance/2026-08-support/validation-implementation-boundary.json").read_text(encoding="utf-8"))

    def run_report(self):
        return report(self.scope, self.ast, self.boundary)

    def test_exact_counts_are_not_overall_completion(self):
        result = self.run_report()
        self.assertEqual((result["check_count"], result["generated_check_count"]), (11, 11))
        self.assertEqual(result["generated_check_percent"], 100.0)
        self.assertEqual(result["resolved_helper_count"], 1)
        self.assertEqual(result["generated_helper_count"], 1)
        self.assertFalse(result["trial_complete"])
        self.assertEqual(len(result["methods"]), 12)
        self.assertTrue(all(row["resolved_ast"] for row in result["methods"]))

    def test_scope_cannot_silently_shrink(self):
        self.scope["checks"].pop()
        with self.assertRaisesRegex(ValueError, "seven family"):
            self.run_report()

    def test_duplicate_scope_check_rejected(self):
        self.scope["checks"][-1] = self.scope["checks"][0]
        with self.assertRaisesRegex(ValueError, "seven family"):
            self.run_report()

    def test_other_helper_requires_explicit_scope_revision(self):
        self.scope["helpers"] = [HELPER.replace("One", "All")]
        with self.assertRaisesRegex(ValueError, "one bounded helper"):
            self.run_report()

    def test_generated_check_cannot_escape_scope(self):
        self.boundary["rules"].append({"id": "outside#check"})
        with self.assertRaisesRegex(ValueError, "escaped"):
            self.run_report()

    def test_all_callers_and_helper_have_generated_evidence(self):
        for row in self.run_report()["methods"]:
            if row["id"] not in BASE:
                self.assertTrue(row["generated_rust"])
                self.assertIn("not established", row["qualification"])

    def test_whole_caller_cannot_be_replaced_by_other_resolved_call(self):
        method = next(m for m in self.ast["methods"] if m["name"] == "checkEnumerationUsage")
        call = method["fields"]["expression"]["fields"]["expressions"][0]
        call["fields"]["feature"] = {"$ref": "java.lang.Class.isInstance(java.lang.Object)"}
        with self.assertRaisesRegex(ValueError, "unselected helper"):
            self.run_report()

    def test_lost_helper_is_rejected(self):
        self.ast["methods"] = [m for m in self.ast["methods"] if m["id"] != HELPER]
        with self.assertRaisesRegex(ValueError, "Incomplete"):
            self.run_report()

    def test_progress_cannot_claim_trial_qualified_from_generated_count(self):
        self.boundary["rules"] = [{"id": check} for check in self.scope["checks"]]
        result = self.run_report()
        self.assertEqual(result["generated_check_percent"], 100.0)
        self.assertFalse(result["trial_complete"])
        self.assertEqual(result["generated_helper_count"], 1)

if __name__ == "__main__":
    unittest.main()
