import copy
import json
from pathlib import Path
import tempfile
import unittest

import audit_release_samples
import prepare_definition_census as census


class DefinitionCensusTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.release = self.root / "release"
        for name in ("sysml/src/example/a.sysml", "sysml/src/example/b.sysml", "kerml/src/types/c.kerml"):
            path = self.release / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("package P;", encoding="utf-8")
        self.library = self.release / "sysml.library/Kernel/Base.kerml"
        self.library.parent.mkdir(parents=True)
        self.library.write_text("package Base;", encoding="utf-8")
        self.dependencies = self.root / "dependencies.json"
        self.dependencies.write_text(json.dumps({"support_dependencies": {
            "sysml/src/example/a.sysml": ["kerml/src/types/c.kerml"]}}))
        self.corpus = self.root / "original.json"
        paths = audit_release_samples.discover(self.release)
        cases = audit_release_samples.corpus(self.release, paths,
                    json.loads(self.dependencies.read_text())["support_dependencies"])
        self.corpus.write_text(json.dumps({"cases": cases}))
        self.implementation = self.root / "candidate.rs"
        self.implementation.write_text("// candidate")

    def prepare(self, **kwargs):
        return census.prepare(self.corpus, self.release, self.dependencies, [self.implementation], **kwargs)

    def rows(self, lock):
        return [{"relative_path": c["relative_path"], "input_files": c["input_files"],
                 "status": "constructed_unqualified", "semantic_qualification": "not_assessed",
                 "element_count": 2} for c in lock["cases"]]

    def test_preserves_original_groups_order_and_all_files(self):
        manifest, lock = self.prepare()
        expected = json.loads(self.corpus.read_text())
        for case in expected["cases"]:
            case["input_files"] = [str(Path(p).resolve()) for p in case["input_files"]]
        self.assertEqual(manifest, expected)
        self.assertEqual(lock["case_count"], 3)
        self.assertEqual(len(lock["inputs"]), 3)
        self.assertEqual(lock["scope"], "all_discovered_release_samples")
        census.validate_fingerprints(lock)
        # Cross-directory dependency participates in the original first target group.
        a = next(c for c in manifest["cases"] if c["relative_path"].endswith("a.sysml"))
        self.assertEqual(len(a["input_files"]), 3)
        self.assertTrue(a["input_files"][-1].endswith("a.sysml"))

    def test_source_sets_deduplicate_target_order_and_record_languages(self):
        _, lock = self.prepare()
        self.assertEqual(lock["case_count"], 3)
        # Both SysML targets close over the same three files in different order.
        self.assertEqual(lock["source_set_count"], 2)
        self.assertEqual(lock["target_language_counts"], {"kerml": 1, "sysml": 2})

    def test_library_is_recorded_but_not_injected_and_changes_reject(self):
        manifest, lock = self.prepare()
        self.assertEqual(len(lock["library_inventory"]), 1)
        self.assertEqual(lock["library_inventory"][0]["release_path"], "sysml.library/Kernel/Base.kerml")
        self.assertIn("not loaded", lock["library_scope"])
        self.assertFalse(any(str(self.library.resolve()) in c["input_files"] for c in manifest["cases"]))
        self.library.write_text("package Changed;")
        with self.assertRaisesRegex(ValueError, "Changed census source"):
            census.validate_fingerprints(lock)

    def test_rejects_changed_group_and_duplicate_target(self):
        original = json.loads(self.corpus.read_text())
        for mutation in ("group", "duplicate"):
            data = copy.deepcopy(original)
            if mutation == "group":
                data["cases"][-1]["input_files"].reverse()
            else:
                data["cases"].append(data["cases"][0])
            self.corpus.write_text(json.dumps(data))
            with self.assertRaises(ValueError):
                self.prepare()

    def test_subset_requires_explicit_scope(self):
        data = json.loads(self.corpus.read_text())
        data["cases"] = data["cases"][:1]
        self.corpus.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, "every discovered"):
            self.prepare()
        _, lock = self.prepare(allow_subset=True)
        self.assertEqual(lock["scope"], "explicit_subset")

    def test_missing_original_source_is_rejected(self):
        (self.release / "sysml/src/example/a.sysml").unlink()
        with self.assertRaises(ValueError):
            self.prepare()

    def test_changed_source_and_implementation_fingerprints_reject(self):
        for target in (self.release / "sysml/src/example/a.sysml", self.implementation):
            _, lock = self.prepare()
            target.write_text(target.read_text() + " changed")
            with self.assertRaisesRegex(ValueError, "Changed census"):
                census.validate_fingerprints(lock)

    def test_stage_counts_do_not_turn_construction_into_semantic_success(self):
        _, lock = self.prepare()
        rows = self.rows(lock)
        rows[0].update(status="blocked", stage="construction_or_linking", error="pending chain")
        rows[1].update(status="blocked", stage="syntax", error="trailing tokens")
        report = census.summarize(lock, rows)
        self.assertEqual(report["candidate_parse_completed"], 2)
        self.assertEqual(report["candidate_syntax_rejections"], 1)
        self.assertEqual(report["semantically_qualified_cases"], 0)
        self.assertEqual(report["statuses"]["constructed_unqualified"], 1)

    def test_unknown_resource_lexical_and_artifact_failures_are_not_syntax_rejections(self):
        _, lock = self.prepare()
        for stage in ("unsupported", "resource_limit", "lexical", "artifact"):
            rows = self.rows(lock)
            rows[0].update(status="blocked", stage=stage, error="failure")
            report = census.summarize(lock, rows)
            self.assertEqual(report["candidate_syntax_rejections"], 0)
            self.assertEqual(report["candidate_parse_completed"], 2)

    def test_identical_first_blockers_group_without_merging_distinct_failures(self):
        _, lock = self.prepare()
        rows = self.rows(lock)
        for row in rows:
            row.update(status="blocked", stage="construction_or_linking", error="pending chain")
        rows[-1]["error"] = "unassessed delegate"
        groups = census.summarize(lock, rows)["first_blocker_groups"]
        self.assertEqual([g["count"] for g in groups], [2, 1])
        self.assertEqual(groups[0]["message"], "pending chain")

    def test_missing_duplicate_and_unknown_results_reject(self):
        _, lock = self.prepare()
        rows = self.rows(lock)
        for changed in (rows[:-1], rows + [rows[0]], rows + [dict(rows[0], relative_path="extra.sysml")]):
            with self.assertRaises(ValueError):
                census.summarize(lock, changed)

    def test_wrong_inputs_or_success_claims_reject(self):
        _, lock = self.prepare()
        for fields in ({"input_files": []}, {"status": "ok"},
                       {"semantic_qualification": "passed"},
                       {"status": "blocked", "stage": "made_up", "error": "unsupported"}):
            rows = self.rows(lock)
            rows[0].update(fields)
            with self.assertRaises(ValueError):
                census.summarize(lock, rows)

    def test_infrastructure_failure_is_not_hidden_as_an_implementation_blocker(self):
        _, lock = self.prepare()
        rows = self.rows(lock)
        rows[0].update(status="infrastructure_error", error="unreadable input")
        result = census.summarize(lock, rows)
        self.assertEqual(result["statuses"]["infrastructure_error"], 1)
        self.assertEqual(result["first_blocker_stages"], {})
        self.assertEqual(result["candidate_parse_completed"], 2)

    def test_outputs_cannot_overwrite_existing_evidence(self):
        output = self.root / "lock.json"
        census.write_new(output, {"original": True})
        with self.assertRaises(FileExistsError):
            census.write_new(output, {"original": False})
        self.assertEqual(json.loads(output.read_text()), {"original": True})


if __name__ == "__main__":
    unittest.main()
