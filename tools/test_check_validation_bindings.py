"""Negative controls for exact binding coverage and reproducible boundary metadata."""
from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest

from check_validation_bindings import build_report, digest, encode, main


class BindingBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="validation binding controls ")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.symbols = ["model.A.getLeft()", "model.A.getRight()"]
        self.rules = {
            "schema_version": 1, "source_format": "typed-validation-rules",
            "dependencies": [{"symbol": symbol, "receiver_type": "model.A", "result_type": "model.A"} for symbol in self.symbols],
            "rules": [{
                "id": "example.Validator#checkA", "name": "checkA",
                "dependencies": self.symbols.copy(),
                "source": {"path": "Validator.xtend", "span": {"start_line": 1}},
                "body": [{"op": "let", "name": f"local{index}", "value": {"kind": "get", "type": "model.A", "symbol": symbol, "receiver": {"kind": "local", "name": "subject", "type": "model.A"}}} for index, symbol in enumerate(self.symbols)],
            }],
            "provenance": {"input_sha256": "fixture-source-hash"},
        }
        self.manifest = {
            "schema_version": 1, "scope": "selected translated validators",
            "runtime": {"implementation_kind": "handwritten-rust", "path": "runtime.rs", "entrypoint": "fn evaluate(", "role": "generated-rust-support", "java_required": False},
            "reference_interpreter": {"implementation_kind": "handwritten-rust", "path": "interpreter.rs", "entrypoint": "fn evaluate(", "role": "typed-ir-reference-interpreter", "test_only": True, "java_required": False, "test_gate": {"path": "runtime.rs", "entrypoint": '#[cfg(test)]\n#[path = "interpreter.rs"]\nmod interpreter;'}},
            "generation": {"implementation_kind": "generated-rust", "role": "selected-rule-predicates", "manifest_path": "generation.json", "generator_path": "generator.py", "module_gate": {"path": "runtime.rs", "entrypoint": '#[path = "generated.rs"]\nmod generated;'}, "runtime_dispatch": {"path": "runtime.rs", "entrypoint": "generated::evaluate("}},
            "bindings": [{"symbol": symbol, "implementation_kind": "handwritten-rust", "path": "getters.rs", "entrypoint": "impl SemanticModel for Model", "status": "implemented", "qualification": "focused-controls", "limitations": ["Synthetic checker fixture only"], "tests": [{"path": "tests.rs", "entrypoint": "fn controlled_getters("}]} for symbol in self.symbols],
        }
        self.runtime_source = '#[path = "generated.rs"]\nmod generated;\n#[cfg(test)]\n#[path = "interpreter.rs"]\nmod interpreter;\nfn evaluate() { generated::evaluate() }\n'
        self.write("runtime.rs", self.runtime_source)
        self.write("interpreter.rs", "fn evaluate() {}\n")
        self.write("generated.rs", "fn rule_0000() {}\n")
        self.write("generator.py", "# Synthetic generated-Rust boundary fixture\n")
        self.generation_overrides = {}
        self.write("getters.rs", "impl SemanticModel for Model {}\n" + "\n".join(self.symbols))
        self.write("tests.rs", "fn controlled_getters() {}\n")
        self.save()

    def write(self, path, text):
        (self.root / path).write_text(text, encoding="utf-8")

    def save(self):
        self.write("rules.json", json.dumps(self.rules))
        self.write("bindings.json", json.dumps(self.manifest))
        generation = {
            "schema_version": 1, "backend": "rust-source",
            "input_sha256": digest(self.root / "rules.json"),
            "generator_sha256": digest(self.root / "generator.py"),
            "output_path": "generated.rs", "output_sha256": digest(self.root / "generated.rs"),
            "rules": [{"id": rule["id"], "name": rule["name"], "entrypoint": f"fn rule_{index:04d}(", "function_name": f"rule_{index:04d}", "dependencies": rule["dependencies"].copy()} for index, rule in enumerate(self.rules["rules"])],
        }
        generation.update(self.generation_overrides)
        self.write("generation.json", json.dumps(generation))

    def report(self):
        self.save()
        return build_report(self.root, "rules.json", "bindings.json")

    def assert_rejected(self, pattern):
        with self.assertRaisesRegex(ValueError, pattern):
            self.report()

    def test_complete_exact_boundary_is_deterministic_and_bounded(self):
        report = self.report()
        self.assertEqual(encode(report), encode(self.report()))
        self.assertEqual((report["selected_rule_count"], report["selected_getter_count"]), (1, 2))
        pipeline = report["execution_pipeline"]
        self.assertTrue(pipeline["rust_source_generation"])
        self.assertEqual(pipeline["reference_interpreter"], "test-only")
        self.assertFalse(pipeline["java_required_at_runtime"])
        self.assertEqual(pipeline["jvm_free_release_qualification"], "pending")
        self.assertEqual(report["total_pilot_validator_coverage"], "unassessed")
        self.assertEqual(set(report["provenance"]["referenced_files_sha256"]), {"runtime.rs", "interpreter.rs", "getters.rs", "tests.rs", "generation.json", "generator.py", "generated.rs"})
        self.assertEqual(report["rules"][0]["full_semantic_qualification"], "not-established")

    def test_new_dependency_requires_explicit_binding(self):
        symbol = "model.A.getUnimplemented()"
        self.rules["dependencies"].append({"symbol": symbol, "receiver_type": "model.A", "result_type": "model.A"})
        self.assert_rejected("Binding inventory mismatch; missing=.*getUnimplemented")

    def test_removed_manifest_entry_fails(self):
        self.manifest["bindings"].pop()
        self.assert_rejected("Binding inventory mismatch; missing=")

    def test_stale_manifest_entry_fails(self):
        self.manifest["bindings"].append({**deepcopy(self.manifest["bindings"][0]), "symbol": "model.A.getUnused()"})
        self.assert_rejected("stale=.*getUnused")

    def test_duplicate_binding_fails(self):
        self.manifest["bindings"].append(deepcopy(self.manifest["bindings"][0]))
        self.assert_rejected("Duplicate binding symbol")

    def test_duplicate_global_dependency_fails(self):
        self.rules["dependencies"].append(deepcopy(self.rules["dependencies"][0]))
        self.assert_rejected("Duplicate global getter dependency")

    def test_duplicate_per_rule_dependency_fails(self):
        self.rules["rules"][0]["dependencies"].append(self.symbols[0])
        self.assert_rejected("Duplicate dependencies for")

    def test_unreported_getter_in_nested_ir_fails(self):
        self.rules["rules"][0]["body"][0]["value"]["symbol"] = "model.A.getHidden()"
        self.assert_rejected("IR getter use/declaration mismatch.*getHidden")

    def test_unused_per_rule_dependency_fails(self):
        self.rules["rules"][0]["body"].pop()
        self.assert_rejected("IR getter use/declaration mismatch.*unused=")

    def test_unused_global_dependency_fails(self):
        self.rules["rules"][0]["body"].pop()
        self.rules["rules"][0]["dependencies"].pop()
        self.assert_rejected("Stale global dependencies")

    def test_unknown_per_rule_dependency_fails(self):
        new = "model.A.getUnknown()"
        self.rules["rules"][0]["dependencies"][0] = new
        self.rules["rules"][0]["body"][0]["value"]["symbol"] = new
        self.assert_rejected("Unknown per-rule dependency")

    def test_missing_implementation_anchor_fails(self):
        self.write("getters.rs", "\n".join(self.symbols))
        self.assert_rejected("exactly one binding anchor")

    def test_ambiguous_anchor_fails(self):
        self.write("getters.rs", "impl SemanticModel for Model\nimpl SemanticModel for Model\n" + "\n".join(self.symbols))
        self.assert_rejected("found 2")

    def test_symbol_must_appear_in_declared_source(self):
        self.write("getters.rs", "impl SemanticModel for Model {}\n" + self.symbols[0])
        self.assert_rejected("Bound getter symbol absent")

    def test_missing_test_anchor_fails(self):
        self.write("tests.rs", "fn unrelated_test() {}\n")
        self.assert_rejected("exactly one test anchor")

    def test_invalid_generated_binding_classification_fails(self):
        self.manifest["bindings"][0]["implementation_kind"] = "generated-rust"
        self.assert_rejected("Invalid handwritten binding classification")

    def test_java_runtime_classification_fails(self):
        self.manifest["runtime"]["java_required"] = True
        self.assert_rejected("without Java")

    def test_production_interpreter_classification_rejected(self):
        self.manifest["runtime"]["role"] = "typed-ir-interpreter"
        self.assert_rejected("Runtime classification")

    def test_reference_interpreter_must_be_test_only(self):
        self.manifest["reference_interpreter"]["test_only"] = False
        self.assert_rejected("test-only Rust")

    def test_missing_reference_test_gate_rejected(self):
        self.write("runtime.rs", self.runtime_source.replace("#[cfg(test)]\n", ""))
        self.assert_rejected("interpreter test gate")

    def test_generation_classification_rejected(self):
        self.manifest["generation"]["implementation_kind"] = "handwritten-rust"
        self.assert_rejected("Generation classification")

    def test_stale_generation_input_hash_rejected(self):
        self.generation_overrides["input_sha256"] = "0" * 64
        self.assert_rejected("generation input hash")

    def test_stale_generator_hash_rejected(self):
        self.generation_overrides["generator_sha256"] = "0" * 64
        self.assert_rejected("generator source hash")

    def test_stale_rust_output_hash_rejected(self):
        self.generation_overrides["output_sha256"] = "0" * 64
        self.assert_rejected("Rust output hash")

    def test_unknown_generation_backend_rejected(self):
        self.generation_overrides["backend"] = "interpreter"
        self.assert_rejected("version or backend")

    def test_unknown_generated_rule_rejected(self):
        self.generation_overrides["rules"] = [{"id": "unknown", "name": "unknown", "entrypoint": "fn rule_0000(", "dependencies": self.symbols}]
        self.assert_rejected("rule inventory mismatch")

    def test_missing_generated_rule_rejected(self):
        self.generation_overrides["rules"] = []
        self.assert_rejected("rule inventory mismatch")

    def test_generated_dependency_mismatch_rejected(self):
        self.generation_overrides["rules"] = [{"id": self.rules["rules"][0]["id"], "name": "checkA", "entrypoint": "fn rule_0000(", "dependencies": []}]
        self.assert_rejected("dependency mismatch")

    def test_generated_name_mismatch_rejected(self):
        self.generation_overrides["rules"] = [{"id": self.rules["rules"][0]["id"], "name": "checkOther", "entrypoint": "fn rule_0000(", "dependencies": self.symbols}]
        self.assert_rejected("name mismatch")

    def test_generated_function_metadata_mismatch_rejected(self):
        self.generation_overrides["rules"] = [{"id": self.rules["rules"][0]["id"], "name": "checkA", "function_name": "other", "entrypoint": "fn rule_0000(", "dependencies": self.symbols}]
        self.assert_rejected("function name/anchor mismatch")

    def test_duplicate_generated_anchor_rejected(self):
        self.write("generated.rs", "fn rule_0000() {}\nfn rule_0000() {}\n")
        self.assert_rejected("generated rule anchor.*found 2")

    def test_missing_generated_anchor_rejected(self):
        self.write("generated.rs", "fn wrong_function() {}\n")
        self.assert_rejected("generated rule anchor.*found 0")

    def test_missing_generated_dispatch_rejected(self):
        self.write("runtime.rs", self.runtime_source.replace("generated::evaluate()", "interpreter::evaluate()"))
        self.assert_rejected("generated runtime dispatch")

    def test_missing_generated_artifact_rejected(self):
        self.generation_overrides["output_path"] = "missing.rs"
        self.assert_rejected("Missing repository file")

    def test_path_outside_root_fails(self):
        self.manifest["runtime"]["path"] = "../runtime.rs"
        self.assert_rejected("Unsafe or nonportable")

    def test_code_or_test_change_updates_report_provenance(self):
        before = self.report()
        self.write("tests.rs", "// meaningful source version changed\nfn controlled_getters() {}\n")
        after = self.report()
        self.assertNotEqual(before["provenance"]["referenced_files_sha256"], after["provenance"]["referenced_files_sha256"])
        self.assertNotEqual(encode(before), encode(after))

    def test_cli_check_rejects_stale_report_without_rewriting(self):
        output = self.root / "report.json"
        args = ["--root", str(self.root), "--rules", "rules.json", "--manifest", "bindings.json", "--out", str(output)]
        self.assertEqual(main(args), 0)
        first = output.read_bytes()
        self.assertEqual(main([*args, "--check"]), 0)
        self.write("runtime.rs", "// runtime changed\n" + self.runtime_source)
        with self.assertRaisesRegex(ValueError, "Stale validation implementation boundary"):
            main([*args, "--check"])
        self.assertEqual(output.read_bytes(), first)


if __name__ == "__main__":
    unittest.main()
