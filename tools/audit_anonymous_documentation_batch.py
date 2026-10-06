"""Focused G05 anonymous/header documentation audit against pinned Pilot exports."""
import json
import os
import re
import sys
from pathlib import Path
import subprocess
from run_namespace_batch import ROOT, read, write, digest
from run_declaration_milestone import sources


def main():
    before = sources()
    artifacts = ROOT.parent / "target"
    support = artifacts / "support-2026-08"
    output = support / "anonymous-documentation-controls"
    evidence = ROOT / "docs/conformance/2026-08-support"
    cases, expected, expectations, fixture_hashes = [], {}, [], {}
    for suite in ["documentation", "anonymous-documentation"]:
        oracle = read(evidence / (suite + "-pilot-controls.json"))
        if oracle["jar_sha256"] != digest(artifacts / "upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"):
            raise ValueError("Pilot runtime drift")
        if oracle["helper_sha256"] != digest(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java"):
            raise ValueError("Pilot helper drift")
        for case in read(support / (suite + "-controls/spec.json"))["cases"]:
            name = case["relative_path"]
            source = suite + "/" + name
            if digest(Path(case["input_files"][0])) != oracle["source_sha256"][name]:
                raise ValueError("Fixture drift: " + source)
            fixture_hashes[source] = oracle["source_sha256"][name]
            cases.append({**case, "relative_path": source})
            expected[source] = next(row["status"] for row in oracle["cases"] if row["relative_path"] == name)
            if expected[source] == "ok":
                graph = read(support / (suite + "-controls") / name / "graph.json")
                docs = [e for e in graph["elements"] if e["kind"] == "Documentation"]
                if not docs: raise ValueError("Missing Pilot documentation")
                for doc in docs:
                    for field in ["body", "declared_name", "declared_short_name", "locale"]:
                        if field in doc["properties"]:
                            expectations.append({"source": source, "line": doc["source"]["start_line"], "name": doc["properties"].get("declared_name"), "kind": "SysML::Documentation", "property": field, "pilot": doc["properties"][field]})
    write(output / "native-spec.json", {"cases": cases})
    property_cases = [case for case in cases if expected[case["relative_path"]] == "ok"]
    original = "sysml/src/training/01. Packages/Documentation Example.sysml"
    header_spec = read(support / "documentation-controls/properties-spec.json")
    original_cases = [case for case in header_spec["cases"] if case["relative_path"] == original]
    original_rows = [row for row in header_spec["expectations"] if row["source"] == original]
    if len(original_cases) != 1 or len(original_rows) != 2: raise ValueError("Original documentation anchor missing")
    property_cases += original_cases
    expectations += original_rows
    write(output / "native-properties-spec.json", {"cases": property_cases, "expectations": expectations})
    library = support / "stdlib.inherited-types.kir.json"
    env = dict(os.environ, MERCURIO_STDLIB_PATH=str(library), MERCURIO_KERNEL_LIBRARY_PATH=str(library))
    def run(binary, spec, label, expected_code):
        path = output / (label + ".jsonl")
        with (output / (label + ".log")).open("w", encoding="utf-8") as log:
            result = subprocess.run([str(ROOT / "target/debug" / (binary + ".exe")), str(spec), str(path)], env=env, stdout=log, stderr=subprocess.STDOUT)
        if result.returncode != expected_code: raise ValueError("Unexpected audit exit: " + label)
        return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]
    controls = run("audit_release_compile", output / "native-spec.json", "native-controls", 1)
    if len(controls) != len(expected) or {r["relative_path"]: r["status"] for r in controls} != expected: raise ValueError("Acceptance mismatch")
    properties = run("audit_release_properties", output / "native-properties-spec.json", "native-properties", 0)
    if len(properties) != len(expectations) or any(r["status"] != "match" for r in properties): raise ValueError("Property mismatch")
    prior = run("audit_release_properties", support / "g05-source-anchor-spec.json", "prior-anchors", 0)
    if len(prior) != 67 or any(r["status"] != "match" for r in prior): raise ValueError("Prior anchor regression")
    # An anonymous anchor must be metaclass-qualified; a wrong metaclass must fail,
    # rather than accidentally pair the document with its ownership relationship.
    anonymous = next(e for e in expectations if e["name"] is None)
    case = next(c for c in cases if c["relative_path"] == anonymous["source"])
    bad = {**anonymous, "kind": "SysML::Nonexistent"}
    write(output / "bad-kind-spec.json", {"cases": [case], "expectations": [bad]})
    negative = run("audit_release_properties", output / "bad-kind-spec.json", "bad-kind", 1)
    if len(negative) != 1 or negative[0]["status"] != "unpaired_or_ambiguous": raise ValueError("Audit paired an invalid metaclass")
    if before != sources(): raise ValueError("Sources changed during audit")
    write(evidence / "anonymous-documentation-evidence.json", {"scope": "Canonical anonymous and header documentation; Annotation relationships remain open", "source_sha256": fixture_hashes, "compiler_source_sha256": before, "library_sha256": digest(library), "binary_sha256": {name: digest(ROOT / "target/debug" / (name + ".exe")) for name in ["audit_release_compile", "audit_release_properties"]}, "controls": controls, "properties": properties, "prior_source_anchors": {"matched": 67, "sha256": digest(output / "prior-anchors.jsonl")}, "invalid_metaclass_control": negative})
    print(f"Verified {len(controls)} Pilot/native controls, {len(properties)} documentation properties and 67 prior anchors")


def publish_checks():
    support = ROOT.parent / "target/support-2026-08"
    path = ROOT / "docs/conformance/2026-08-support/anonymous-documentation-evidence.json"
    evidence = read(path)
    if evidence["compiler_source_sha256"] != sources(): raise ValueError("Sources changed since focused audit")
    for name, expected in evidence["binary_sha256"].items():
        if digest(ROOT / "target/debug" / (name + ".exe")) != expected: raise ValueError("Audit binary drift")
    checks = {}
    for label, count in [("sysml-final", 454), ("authoring", 47), ("lexer", 14), ("candidate", 11)]:
        log = support / ("g05-anonymous-" + label + ".log")
        observed = re.findall(r"test result: ok\. (\d+) passed; 0 failed;", log.read_text(encoding="utf-8-sig"))
        if observed != [str(count)]: raise ValueError("Unexpected test result: " + label)
        checks[label] = {"passed": count, "log_sha256": digest(log)}
    sample_path = support / "g05-anonymous-sample-compilation.jsonl"
    samples = [json.loads(line) for line in sample_path.read_text(encoding="utf-8").splitlines()]
    corpus = ROOT.parent / "target/release-audit-2026-08-final/corpus.json"
    expected = {case["relative_path"] for case in read(corpus)["cases"]}
    if len(samples) != 310 or len(expected) != 310 or {row["relative_path"] for row in samples} != expected or any(row["status"] != "ok" for row in samples):
        raise ValueError("Sample compilation incomplete or failing")
    evidence["checks"] = checks
    evidence["sample_compilation"] = {"passed": 310, "result_sha256": digest(sample_path), "corpus_sha256": digest(corpus)}
    evidence["audit_script_sha256"] = digest(Path(__file__))
    write(path, evidence)
    print("Published checked source/binary provenance, focused regressions and 310 sample compilations")


if __name__ == "__main__":
    if sys.argv[1:] == ["--publish-checks"]:
        publish_checks()
    elif not sys.argv[1:]:
        main()
    else:
        raise SystemExit("usage: audit_anonymous_documentation_batch.py [--publish-checks]")
