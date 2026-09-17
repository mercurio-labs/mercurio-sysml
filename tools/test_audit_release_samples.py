import json
from pathlib import Path
import tempfile
import unittest
import audit_release_samples as audit

class AuditTests(unittest.TestCase):
    def test_shared_rejection_is_not_success(self):
        rows = [{"native": {"status": "error"}, "pilot": {"status": "error"}}]
        result = audit.summary(rows)
        self.assertEqual(result["both_reject"], 1)
        self.assertEqual(result["both_pass"], 0)

    def test_missing_result_never_counts_as_parity(self):
        with tempfile.TemporaryDirectory() as directory:
            result = audit.complete_results([{"relative_path": "missing.sysml"}],
                                            Path(directory) / "absent.jsonl", {"exit_code": 1})
        row = {"native": result["missing.sysml"], "pilot": {"status": "ok"}}
        self.assertEqual(audit.summary([row])["infrastructure_failures"], 1)
        self.assertEqual(audit.summary([row])["completed_comparisons"], 0)

    def test_cross_file_inputs_are_transitive_deduplicated_and_target_last(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("target/main.sysml", "target/sibling.kerml", "types/a.sysml", "types/b.sysml"):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("")
            cases = audit.corpus(root, ["target/main.sysml"],
                                 {"target/main.sysml": ["types/a.sysml"], "types/a.sysml": ["target/main.sysml"]})
            inputs = cases[0]["input_files"]
            self.assertEqual(len(inputs), 4)
            self.assertEqual(len(set(inputs)), 4)
            self.assertEqual(inputs[-1], str(root / "target/main.sysml"))

    def test_discovery_includes_both_languages_but_not_stdlib(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("sysml/src/training/a.sysml", "kerml/src/examples/b.kerml", "sysml.library/c.sysml"):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("")
            self.assertEqual(audit.discover(root), ["kerml/src/examples/b.kerml", "sysml/src/training/a.sysml"])

if __name__ == "__main__":
    unittest.main()
