import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from strategy_qualification import SCHEMA, contract_digest, validate_strategy_batches


class StrategyQualificationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.write("source.rs", "pinned source")
        self.write("observations.json", '{"positive":true,"negative":false}')
        self.write("tests.log", "test module::positive ... ok\ntest module::negative ... ok\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 99 filtered out;\n")
        self.batch = {"id": "ordinary", "scope": "complete ordinary strategy on declared bindings", "bindings": ["Feature", "Step"], "inputs": {"source.rs": self.digest("source.rs")}, "obligations": [
            {"id": "positive", "acceptance": "positive contexts match", "status": "closed", "dependencies": [], "tests": ["module::positive"]},
            {"id": "negative", "acceptance": "negative and boundary contexts match", "status": "closed", "dependencies": ["positive"], "tests": ["module::negative"]}], "certificate": "certificate.json"}
        self.cert = {"schema": SCHEMA, "batch_id": "ordinary", "contract_sha256": contract_digest(self.batch), "inputs": copy.deepcopy(self.batch["inputs"]), "observations": {"observations.json": self.digest("observations.json")}, "test_log": {"path": "tests.log", "sha256": self.digest("tests.log")}, "passed_tests": ["module::positive", "module::negative"], "closed_obligations": ["positive", "negative"]}
        self.save()

    def write(self, path, text):
        (self.root / path).write_text(text, encoding="utf-8")

    def digest(self, path):
        return hashlib.sha256((self.root / path).read_bytes()).hexdigest()

    def save(self):
        self.write("certificate.json", json.dumps(self.cert))

    def check(self):
        return validate_strategy_batches({"strategy_batches": [self.batch]}, self.root)

    def test_valid_closure_and_focused_log(self):
        result = self.check()
        self.assertEqual((result["required"], result["closed"]), (2, 2))

    def test_open_batch_needs_no_certificate(self):
        for row in self.batch["obligations"]:
            row["status"] = "open"
        del self.batch["certificate"]
        self.assertEqual(self.check()["closed"], 0)

    def test_absent_batches_are_not_completion(self):
        self.assertEqual(validate_strategy_batches({}, self.root), {"required": 0, "closed": 0, "batches": []})

    def test_contract_inventory_and_acceptance_tampering(self):
        for mutate in (lambda: self.batch["bindings"].append("Usage"), lambda: self.batch["obligations"][0].update(acceptance="weaker")):
            with self.subTest(mutate=mutate):
                before = copy.deepcopy(self.batch)
                mutate()
                with self.assertRaisesRegex(ValueError, "contract mismatch"):
                    self.check()
                self.batch = before

    def test_stale_source_and_observations(self):
        for path in ("source.rs", "observations.json", "tests.log"):
            with self.subTest(path=path):
                before = (self.root / path).read_text(encoding="utf-8")
                self.write(path, "changed")
                with self.assertRaisesRegex(ValueError, "stale fingerprint"):
                    self.check()
                self.write(path, before)

    def test_removed_evidence(self):
        (self.root / "observations.json").unlink()
        with self.assertRaisesRegex(ValueError, "missing"):
            self.check()

    def test_missing_test_and_failed_run(self):
        for log in ("test module::positive ... ok\ntest result: ok. 1 passed; 0 failed;", "test module::positive ... ok\ntest module::negative ... FAILED\ntest result: FAILED. 1 passed; 1 failed;"):
            self.write("tests.log", log)
            self.cert["test_log"]["sha256"] = self.digest("tests.log")
            self.cert["passed_tests"] = ["module::positive"]
            self.save()
            with self.assertRaises(ValueError):
                self.check()

    def test_open_dependency_rejects_closure(self):
        self.batch["obligations"][0]["status"] = "open"
        with self.assertRaisesRegex(ValueError, "open dependency"):
            self.check()

    def test_cycles_cannot_qualify_each_other(self):
        self.batch["obligations"][0]["dependencies"] = ["negative"]
        with self.assertRaisesRegex(ValueError, "cyclic"):
            self.check()

    def test_exact_closure_and_test_inventories(self):
        for field in ("closed_obligations", "passed_tests"):
            before = self.cert[field]
            self.cert[field] = before[:1]
            self.save()
            with self.assertRaises(ValueError):
                self.check()
            self.cert[field] = before

    def test_duplicate_and_vacuous_contracts(self):
        for field, value in (("bindings", ["Feature", "Feature"]), ("obligations", []), ("inputs", {})):
            before = self.batch[field]
            self.batch[field] = value
            with self.assertRaises(ValueError):
                self.check()
            self.batch[field] = before

    def test_path_escape_rejected(self):
        for path in ("../source.rs", "C:/source.rs", "/source.rs", "..\\source.rs"):
            self.batch["certificate"] = path
            with self.assertRaises(ValueError):
                self.check()

    def test_symlink_escape_rejected(self):
        with tempfile.TemporaryDirectory() as outside:
            target = Path(outside) / "outside.json"
            target.write_text(json.dumps(self.cert), encoding="utf-8")
            try:
                (self.root / "link.json").symlink_to(target)
            except OSError:
                self.skipTest("OS does not permit symlink creation")
            self.batch["certificate"] = "link.json"
            with self.assertRaisesRegex(ValueError, "escaping"):
                self.check()

    def test_missing_certificate_rejected(self):
        del self.batch["certificate"]
        with self.assertRaisesRegex(ValueError, "require certificate"):
            self.check()

    def test_digest_excludes_status_and_certificate_only(self):
        original = contract_digest(self.batch)
        self.batch["obligations"][0]["status"] = "open"
        self.batch["certificate"] = "other.json"
        self.assertEqual(contract_digest(self.batch), original)
        self.batch["scope"] += " changed"
        self.assertNotEqual(contract_digest(self.batch), original)


if __name__ == "__main__":
    unittest.main()
