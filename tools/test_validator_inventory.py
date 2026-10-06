"""Inventory completeness, scope and actual frontend mutation controls."""
from copy import deepcopy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from export_validator_inventory import PROFILE, ROOT, HELPER, validate
from export_pilot_xtend import XTEND_VERSION, encode
from classify_validator_inventory import classify


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


class InventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inventory = read(PROFILE / "validators.inventory.extract.json")
        cls.boundary = read(ROOT / "docs/conformance/2026-08-support/validation-implementation-boundary.json")
        cls.overlay = read(PROFILE / "validator-coverage.overlay.json")
        cls.legacy = read(PROFILE / "validators.extract.json")

    def setUp(self):
        self.document = deepcopy(self.inventory)

    def report(self):
        return classify(self.document, self.boundary, self.overlay, self.legacy)

    def test_complete_inventory_and_generated_coverage_are_distinct(self):
        report = self.report()
        self.assertEqual(report["summary"]["checks"], 113)
        self.assertEqual(report["summary"]["helpers"], 43)
        self.assertEqual(report["summary"]["generated_rust"], 11)
        self.assertEqual(report["summary"]["not_translated"], 102)
        self.assertEqual(report["summary"]["upstream_empty_checks"], 5)
        self.assertEqual(encode(report), encode(self.report()))
        self.assertTrue(all(r["implementation"] == "not-translated" for r in report["checks"] if r["upstream_empty_body"]))
        self.assertTrue(any(d["category"] == "constructor" for d in report["dependencies"]))

    def test_duplicate_method_is_rejected(self):
        self.document["methods"].append(deepcopy(self.document["methods"][0]))
        with self.assertRaisesRegex(ValueError, "Duplicated method"):
            validate(self.document)

    def test_omitted_call_is_rejected(self):
        next(m for m in self.document["methods"] if m["calls"])["calls"].pop()
        with self.assertRaisesRegex(ValueError, "Call inventory omits"):
            validate(self.document)

    def test_omitted_method_is_rejected(self):
        self.document["methods"].pop()
        with self.assertRaisesRegex(ValueError, "counts do not cover"):
            validate(self.document)

    def test_resolution_issue_cannot_be_concealed(self):
        method = self.document["methods"][0]
        method["resolution_issues"].append({"kind": "unresolved-feature"})
        with self.assertRaisesRegex(ValueError, "Resolution status conceals"):
            validate(self.document)

    def test_unresolved_call_cannot_be_concealed(self):
        method = next(m for m in self.document["methods"] if m["calls"])
        method["calls"][0]["resolved"] = False
        with self.assertRaisesRegex(ValueError, "Unresolved call concealed"):
            validate(self.document)

    def test_annotation_classification_is_not_based_on_name(self):
        method = next(m for m in self.document["methods"] if m["is_check"])
        method["annotations"] = []
        with self.assertRaisesRegex(ValueError, "resolved annotation"):
            validate(self.document)

    def test_changed_source_provenance_is_rejected(self):
        source = next(iter(self.document["provenance"]["sources_sha256"]))
        self.document["provenance"]["sources_sha256"][source] = "changed"
        with self.assertRaisesRegex(ValueError, "source hashes differ"):
            self.report()

    def test_generated_and_inventoried_jar_provenance_must_match(self):
        self.document["provenance"]["jar_sha256"] = "different-executable"
        with self.assertRaisesRegex(ValueError, "upstream provenance"):
            self.report()

    def test_generated_rule_must_be_in_frontend_inventory(self):
        boundary = deepcopy(self.boundary)
        boundary["rules"][0]["id"] = "missing.Validator#checkMissing"
        with self.assertRaisesRegex(ValueError, "Generated rule absent"):
            classify(self.document, boundary, self.overlay, self.legacy)

    def test_selected_adapter_does_not_cover_other_checks(self):
        source = next(m for m in self.document["methods"] if m["name"] == "checkImport")
        replacement = next(c for c in source["calls"] if c.get("target", {}).get("id") == "org.omg.sysml.lang.sysml.Import.getVisibility()")
        destination = next(m for m in self.document["methods"] if m["name"] == "checkReferenceUsage")
        original = next(c for c in destination["calls"] if c.get("target", {}).get("id") == "org.omg.sysml.lang.sysml.Usage.isReference()")
        original["target"] = deepcopy(replacement["target"])
        row = next(r for r in self.report()["checks"] if r["rule_id"].endswith("#checkReferenceUsage"))
        self.assertIn(replacement["target"]["id"], row["translation_planning"]["unbound_model_dependencies"])

    def test_helper_cycles_terminate_and_remain_visible(self):
        helper = next(m for m in self.document["methods"] if not m["is_check"] and m["jvm_ids"])
        target = next(c["target"] for m in self.document["methods"] for c in m["calls"] if c.get("target", {}).get("id") == helper["jvm_ids"][0])
        helper["calls"].append({"node_kind": "XFeatureCall", "resolved": True, "target": deepcopy(target)})
        helper["body_node_kinds"]["XFeatureCall"] = helper["body_node_kinds"].get("XFeatureCall", 0) + 1
        row = next(r for r in self.report()["helper_call_graph"] if r["id"] == helper["id"])
        self.assertIn(helper["id"], row["calls"])


@unittest.skipUnless(os.environ.get("MERCURIO_INVENTORY_JAVA_BIN"), "Set MERCURIO_INVENTORY_JAVA_BIN for actual upstream frontend mutation tests")
class FrontendMutationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix="validator frontend controls ")
        cls.addClassCleanup(cls.temp.cleanup)
        cls.work = Path(cls.temp.name)
        cls.java_bin = Path(os.environ["MERCURIO_INVENTORY_JAVA_BIN"])
        cls.suffix = ".exe" if os.name == "nt" else ""
        cls.pilot = ROOT.parent / "target/upstream/SysML-v2-Pilot-Implementation"
        inventory = read(PROFILE / "validators.inventory.extract.json")
        cls.sources = sorted(inventory["provenance"]["sources_sha256"])
        jar = ROOT.parent / "target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"
        xtend = Path.home() / f".m2/repository/org/eclipse/xtend/org.eclipse.xtend.core/{XTEND_VERSION}/org.eclipse.xtend.core-{XTEND_VERSION}.jar"
        cls.cp = os.pathsep.join(map(str, [cls.work, jar, xtend]))
        subprocess.run([str(cls.java_bin / ("javac" + cls.suffix)), "-encoding", "UTF-8", "-cp", cls.cp, "-d", str(cls.work), str(HELPER)], check=True, capture_output=True)

    def run_frontend(self, change):
        with tempfile.TemporaryDirectory(dir=self.work) as directory:
            root = Path(directory)
            for source in self.sources:
                target = root / source
                target.parent.mkdir(parents=True, exist_ok=True)
                text = (self.pilot / source).read_text(encoding="utf-8")
                if source.endswith("/KerMLValidator.xtend"):
                    text = change(text)
                target.write_text(text, encoding="utf-8", newline="\n")
            sources = root / "sources.json"
            sources.write_text(json.dumps(self.sources), encoding="utf-8")
            output = root / "inventory.json"
            result = subprocess.run([str(self.java_bin / ("java" + self.suffix)), "-Xmx2g", "-cp", self.cp,
                                     "dev.mercurio.pilot.PilotValidatorInventory", str(root), str(sources), str(output)], capture_output=True, text=True, timeout=90)
            return result, read(output) if output.exists() else None

    def test_renamed_check_is_discovered_and_packages_resolve(self):
        result, document = self.run_frontend(lambda s: s.replace("def checkImport(", "def checkInventoryRenamedImport("))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(validate(document)["checks"], 113)
        self.assertEqual(validate(document)["check_resolution"], {"resolved": 113})
        self.assertTrue(any(m["name"] == "checkInventoryRenamedImport" and m["is_check"] for m in document["methods"]))
        self.assertTrue(any(c.get("target", {}).get("kind") == "PackageFragment" for m in document["methods"] for c in m["calls"]))

    def test_commented_annotation_does_not_count_as_check(self):
        def change(text):
            position = text.rfind("@Check", 0, text.index("def checkImport("))
            return text[:position] + text[position:].replace("@Check", "// @Check", 1)
        result, document = self.run_frontend(change)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(validate(document)["checks"], 112)
        self.assertFalse(next(m for m in document["methods"] if m["name"] == "checkImport")["is_check"])

    def test_unknown_property_is_retained_as_resolution_failure(self):
        def change(text):
            position = text.index("def checkImport(")
            return text[:position] + text[position:].replace("importOwningNamespace", "missingInventoryNamespace", 1)
        result, document = self.run_frontend(change)
        self.assertEqual(result.returncode, 0, result.stderr)
        method = next(m for m in document["methods"] if m["name"] == "checkImport")
        self.assertEqual(method["resolution_status"], "incomplete")
        self.assertTrue(method["resolution_issues"])
        validate(document)

    def test_invalid_syntax_fails_extraction(self):
        result, document = self.run_frontend(lambda s: s + "\nclass {\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(document)
        self.assertIn("Xtend parse failed", result.stderr)


if __name__ == "__main__":
    unittest.main()
