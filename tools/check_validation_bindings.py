"""Verify generated Rust predicates and their handwritten semantic boundaries.

This is a static build check of dependency declarations, file/anchor existence,
and provenance. It does not execute Rust tests, qualify packaged products as
JVM-free, or establish completeness of Pilot semantics.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import sys

ROOT = Path(__file__).resolve().parents[1]
PROFILE = "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
RULES = PROFILE + "/validation-rules.extract.json"
MANIFEST = PROFILE + "/validation-bindings.json"
OUTPUT = "docs/conformance/2026-08-support/validation-implementation-boundary.json"
SCOPE = "selected translated validators"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def encode(value: dict) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")


def relative_file(root: Path, relative: str) -> Path:
    if not isinstance(relative, str) or not relative:
        raise ValueError("Expected a repository-relative file path")
    path = PurePosixPath(relative)
    if path.is_absolute() or ".." in path.parts or "\\" in relative or ":" in relative:
        raise ValueError(f"Unsafe or nonportable repository path: {relative}")
    actual = root.joinpath(*path.parts).resolve()
    if not actual.is_relative_to(root.resolve()) or not actual.is_file():
        raise ValueError(f"Missing repository file: {relative}")
    return actual


def nonempty(value, context):
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"Missing {context}")
    return value


def unique(values, context):
    if not isinstance(values, list) or not all(isinstance(v, str) and v for v in values):
        raise ValueError(f"Invalid {context}")
    if len(set(values)) != len(values):
        raise ValueError(f"Duplicate {context}")
    return set(values)


def getter_uses(value):
    if isinstance(value, dict):
        if value.get("kind") in {"get", "is_instance"}:
            yield nonempty(value.get("symbol"), "IR getter symbol")
        for child in value.values():
            yield from getter_uses(child)
    elif isinstance(value, list):
        for child in value:
            yield from getter_uses(child)


def build_report(root: Path, rules_path: str = RULES, manifest_path: str = MANIFEST, checker_path: Path | None = None) -> dict:
    rules_file = relative_file(root, rules_path)
    manifest_file = relative_file(root, manifest_path)
    rules_document = json.loads(rules_file.read_text(encoding="utf-8"))
    manifest = json.loads(manifest_file.read_text(encoding="utf-8"))
    if rules_document.get("schema_version") != 1 or rules_document.get("source_format") != "typed-validation-rules":
        raise ValueError("Unsupported typed validation rules")
    if manifest.get("schema_version") != 1 or manifest.get("scope") != SCOPE:
        raise ValueError("Invalid binding manifest version or scope")
    inventory = rules_document.get("dependencies")
    rules = rules_document.get("rules")
    bindings = manifest.get("bindings")
    if not isinstance(inventory, list) or not isinstance(rules, list) or not rules or not isinstance(bindings, list):
        raise ValueError("Missing rules, dependencies, or binding inventory")
    global_dependencies = unique([entry.get("symbol") for entry in inventory], "global getter dependency")
    dependency_types = {}
    for dependency in inventory:
        dependency_types[dependency["symbol"]] = {
            field: nonempty(dependency.get(field), f"getter {field}")
            for field in ("receiver_type", "result_type")
        }
    declared_bindings = unique([entry.get("symbol") for entry in bindings], "binding symbol")
    if declared_bindings != global_dependencies:
        raise ValueError(f"Binding inventory mismatch; missing={sorted(global_dependencies - declared_bindings)}, stale={sorted(declared_bindings - global_dependencies)}")
    by_symbol = {binding["symbol"]: binding for binding in bindings}
    referenced_files = {}

    def anchor(entry, role):
        path = nonempty(entry.get("path"), f"{role} path")
        file = relative_file(root, path)
        entrypoint = nonempty(entry.get("entrypoint"), f"{role} entrypoint")
        source = file.read_text(encoding="utf-8")
        count = source.count(entrypoint)
        if count != 1:
            raise ValueError(f"Expected exactly one {role} anchor {entrypoint!r} in {path}; found {count}")
        referenced_files[path] = digest(file)
        return source

    runtime = manifest.get("runtime", {})
    if runtime.get("implementation_kind") != "handwritten-rust" or runtime.get("role") != "generated-rust-support" or runtime.get("java_required") is not False:
        raise ValueError("Runtime classification must declare handwritten generated-Rust support without Java")
    if not str(runtime.get("path", "")).endswith(".rs"):
        raise ValueError("Runtime implementation must identify Rust source")
    anchor(runtime, "runtime")
    interpreter = manifest.get("reference_interpreter", {})
    if (interpreter.get("implementation_kind") != "handwritten-rust"
        or interpreter.get("role") != "typed-ir-reference-interpreter"
        or interpreter.get("test_only") is not True
        or interpreter.get("java_required") is not False):
        raise ValueError("Reference interpreter classification must be handwritten test-only Rust without Java")
    if not str(interpreter.get("path", "")).endswith(".rs") or interpreter.get("path") == runtime.get("path"):
        raise ValueError("Reference interpreter must identify separate test-only Rust source")
    anchor(interpreter, "reference interpreter")
    gate = interpreter.get("test_gate", {})
    if gate.get("path") != runtime.get("path"):
        raise ValueError("Reference interpreter test gate must be in the runtime support module")
    expected_gate = '#[cfg(test)]\n#[path = "' + PurePosixPath(interpreter["path"]).name + '"]\nmod interpreter;'
    if gate.get("entrypoint") != expected_gate:
        raise ValueError("Reference interpreter requires an explicit cfg(test) module gate")
    anchor(gate, "interpreter test gate")
    normalized_bindings = {}
    for binding in bindings:
        symbol = binding["symbol"]
        if binding.get("implementation_kind") != "handwritten-rust" or binding.get("status") != "implemented" or binding.get("qualification") != "focused-controls":
            raise ValueError(f"Invalid handwritten binding classification: {symbol}")
        if not str(binding.get("path", "")).endswith(".rs"):
            raise ValueError(f"Binding must identify Rust source: {symbol}")
        source = anchor(binding, "binding")
        if symbol not in source:
            raise ValueError(f"Bound getter symbol absent from declared Rust source: {symbol}")
        limitations = binding.get("limitations")
        if not isinstance(limitations, list) or not all(isinstance(item, str) and item.strip() for item in limitations):
            raise ValueError(f"Invalid binding limitations: {symbol}")
        tests = binding.get("tests")
        if not isinstance(tests, list) or not tests:
            raise ValueError(f"Missing focused test anchors: {symbol}")
        identities = set()
        for test in tests:
            identity = (test.get("path"), test.get("entrypoint"))
            if identity in identities:
                raise ValueError(f"Duplicate focused test anchor: {symbol}")
            identities.add(identity)
            anchor(test, "test")
        normalized_bindings[symbol] = {
            **binding, **dependency_types[symbol],
            "semantic_qualification": "focused-controls-only",
            "test_execution_verified_by_this_report": False,
        }
    identifiers = unique([item.get("id") for item in rules], "rule identity")
    union = set()
    report_rules = []
    for rule in sorted(rules, key=lambda item: item["id"]):
        name = nonempty(rule.get("name"), "rule name")
        dependencies = unique(rule.get("dependencies"), f"dependencies for {rule['id']}")
        used = set(getter_uses(rule.get("body")))
        if used != dependencies:
            raise ValueError(f"IR getter use/declaration mismatch for {rule['id']}; undeclared={sorted(used - dependencies)}, unused={sorted(dependencies - used)}")
        if not dependencies <= global_dependencies:
            raise ValueError(f"Unknown per-rule dependency for {rule['id']}: {sorted(dependencies - global_dependencies)}")
        union.update(dependencies)
        report_rules.append({
            "id": rule["id"], "name": name,
            "translation": {
                "representation": "generated-typed-ir",
                "source": rule.get("source"),
                "artifact": rules_path,
                "rust_source_generated": True,
            },
            "handwritten_semantic_bindings": [normalized_bindings[symbol] for symbol in sorted(dependencies)],
            "full_semantic_qualification": "not-established",
        })
    if union != global_dependencies:
        raise ValueError(f"Stale global dependencies without selected rule use: {sorted(global_dependencies - union)}")
    generation = manifest.get("generation", {})
    if generation.get("implementation_kind") != "generated-rust" or generation.get("role") != "selected-rule-predicates":
        raise ValueError("Generation classification must identify generated Rust predicates")
    generation_path = nonempty(generation.get("manifest_path"), "generation manifest path")
    generator_path = nonempty(generation.get("generator_path"), "generator source path")
    generation_file = relative_file(root, generation_path)
    generator_file = relative_file(root, generator_path)
    generation_record = json.loads(generation_file.read_text(encoding="utf-8"))
    if generation_record.get("schema_version") != 1 or generation_record.get("backend") != "rust-source":
        raise ValueError("Invalid Rust generation manifest version or backend")
    if generation_record.get("input_sha256") != digest(rules_file):
        raise ValueError("Stale Rust generation input hash")
    if generation_record.get("generator_sha256") != digest(generator_file):
        raise ValueError("Stale Rust generator source hash")
    output_path = nonempty(generation_record.get("output_path"), "generated Rust output path")
    if not output_path.endswith(".rs") or output_path in {runtime["path"], interpreter["path"]}:
        raise ValueError("Generated predicates must identify separate Rust source")
    output_file = relative_file(root, output_path)
    module_gate = generation.get("module_gate", {})
    expected_module = '#[path = "' + PurePosixPath(output_path).name + '"]\nmod generated;'
    if module_gate.get("path") != runtime.get("path") or module_gate.get("entrypoint") != expected_module:
        raise ValueError("Generated predicate module must be included by runtime support")
    anchor(module_gate, "generated module")
    dispatch = generation.get("runtime_dispatch", {})
    if dispatch.get("path") != runtime.get("path") or dispatch.get("entrypoint") != "generated::evaluate(":
        raise ValueError("Runtime dispatch must call generated predicate evaluation")
    anchor(dispatch, "generated runtime dispatch")
    if generation_record.get("output_sha256") != digest(output_file):
        raise ValueError("Stale generated Rust output hash")
    generated_rules = generation_record.get("rules")
    if not isinstance(generated_rules, list):
        raise ValueError("Missing generated Rust rule inventory")
    generated_ids = unique([entry.get("id") for entry in generated_rules], "generated rule identity")
    if generated_ids != identifiers:
        raise ValueError(f"Generated Rust rule inventory mismatch; missing={sorted(identifiers - generated_ids)}, stale={sorted(generated_ids - identifiers)}")
    unique([entry.get("entrypoint") for entry in generated_rules], "generated rule entrypoint")
    generated_by_id = {entry["id"]: entry for entry in generated_rules}
    for source_rule, report_rule in zip(sorted(rules, key=lambda item: item["id"]), report_rules):
        generated = generated_by_id[source_rule["id"]]
        if generated.get("name") != source_rule["name"]:
            raise ValueError(f"Generated rule name mismatch for {source_rule['id']}")
        dependencies = unique(generated.get("dependencies"), "generated rule dependencies")
        if dependencies != set(source_rule["dependencies"]):
            raise ValueError(f"Generated rule dependency mismatch for {source_rule['id']}")
        if "function_name" in generated:
            function_name = nonempty(generated["function_name"], "generated function name")
            if not function_name.isidentifier() or generated["entrypoint"] != f"fn {function_name}(":
                raise ValueError(f"Generated function name/anchor mismatch for {source_rule['id']}")
        anchor({"path": output_path, "entrypoint": generated["entrypoint"]}, "generated rule")
        report_rule["generated_predicate"] = {
            "implementation_kind": "generated-rust", "path": output_path,
            "entrypoint": generated["entrypoint"], "dependencies": sorted(dependencies),
        }
    inlined = generation_record.get("inlined_helpers", [])
    if inlined != rules_document.get("inlined_helpers", []):
        raise ValueError("Generated helper specialization metadata differs from typed IR")
    unique([helper.get("id") for helper in inlined], "inlined helper identity")
    referenced_files[generation_path] = digest(generation_file)
    referenced_files[generator_path] = digest(generator_file)
    return {
        "schema_version": 1,
        "scope": SCOPE,
        "selected_rule_count": len(identifiers),
        "inlined_helpers": inlined,
        "selected_getter_count": len(global_dependencies),
        "total_pilot_validator_coverage": "unassessed",
        "execution_pipeline": {
            "rule_representation": "generated-rust-from-translated-typed-ir",
            "rust_source_generation": True,
            "production_execution": "generated-rust-predicates-with-handwritten-support",
            "reference_interpreter": "test-only",
            "java_required_at_runtime": False,
            "java_runtime_statement_basis": "selected generated Rust and handwritten support/binding source; not packaged-product qualification",
            "jvm_free_release_qualification": "pending",
        },
        "check_scope": {
            "kind": "static-local-boundary-and-freshness-check",
            "checks": ["exact global and per-rule getter coverage", "generated predicate, handwritten support/getter and test-only interpreter classifications", "unique generated entrypoints and handwritten source/test anchors", "generation input/output/generator hashes", "input and referenced-file hashes"],
            "does_not_establish": ["Rust test execution", "getter correctness beyond separately recorded focused evidence", "complete Pilot semantic support", "JVM-free packaged product qualification"],
        },
        "runtime": runtime,
        "reference_interpreter": interpreter,
        "generation": {**generation, "output_path": output_path, "verified_rule_count": len(generated_ids)},
        "rules": report_rules,
        "provenance": {
            "rules_sha256": digest(rules_file),
            "manifest_sha256": digest(manifest_file),
            "checker_sha256": digest(checker_path or Path(__file__)),
            "referenced_files_sha256": dict(sorted(referenced_files.items())),
            "translated_rule_provenance": rules_document.get("provenance", {}),
            "pilot_source_provenance": rules_document.get("source", {}),
        },
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--rules", default=RULES)
    parser.add_argument("--manifest", default=MANIFEST)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    output = args.out or args.root / OUTPUT
    result = encode(build_report(args.root, args.rules, args.manifest))
    if args.check:
        if not output.is_file() or output.read_bytes() != result:
            raise ValueError(f"Stale validation implementation boundary report: {output}")
        print("Verified selected validation implementation boundary and source freshness")
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(result)
        print("Wrote selected validation implementation boundary; full semantic and release qualification remain unassessed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
