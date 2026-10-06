"""Verify G05 textual annotations, legacy docs, focused checks and sample acceptance."""
import json
import os
import re
import subprocess
from pathlib import Path
from run_namespace_batch import ROOT, read, write, digest
from run_declaration_milestone import sources

def main():
    before = sources()
    support = ROOT.parent / "target/support-2026-08"
    folder = support / "annotation-completion"
    folder.mkdir(exist_ok=True)
    evidence_dir = ROOT / "docs/conformance/2026-08-support"
    library = support / "stdlib.inherited-types.kir.json"
    env = dict(os.environ, MERCURIO_STDLIB_PATH=str(library), MERCURIO_KERNEL_LIBRARY_PATH=str(library))
    binaries = {tool:digest(ROOT / "target/debug" / (tool + ".exe")) for tool in ["audit_release_compile", "audit_release_properties"]}
    for suite in ["owned-annotations", "bare-comments"]:
        evidence = read(evidence_dir / (suite + "-graph-evidence.json"))
        if evidence["compiler_source_sha256"] != before or evidence["binary_sha256"] != binaries: raise ValueError("Stale ownership evidence")
    def run(tool, spec, name, code):
        output = folder / (name + ".jsonl")
        with (folder / (name + ".log")).open("w", encoding="utf-8") as log:
            result = subprocess.run([str(ROOT / "target/debug" / (tool + ".exe")), str(spec), str(output)], env=env, stdout=log, stderr=subprocess.STDOUT)
        if result.returncode != code: raise ValueError("Audit failed: " + name)
        return [json.loads(line) for line in output.read_text(encoding="utf-8").splitlines()]
    oracle = read(evidence_dir / "annotation-legality-pilot-controls.json")
    helper = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java"
    jar = ROOT.parent / "target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"
    if oracle["helper_sha256"] != digest(helper) or oracle["jar_sha256"] != digest(jar): raise ValueError("Oracle provenance drift")
    spec = support / "annotation-legality-controls/spec.json"
    for case in read(spec)["cases"]:
        if oracle["source_sha256"][case["relative_path"]] != digest(Path(case["input_files"][0])): raise ValueError("Fixture drift")
    legality = run("audit_release_compile", spec, "legality", 1)
    if len(legality) != 8 or {r["relative_path"]:r["status"] for r in legality} != {r["relative_path"]:r["status"] for r in oracle["cases"]}: raise ValueError("Legality differs")
    prior = run("audit_release_properties", support / "annotations-controls/graph-spec.json", "prior-explicit-annotations", 0)
    if len(prior) != 8 or any(r["status"] != "match" for r in prior): raise ValueError("Explicit annotation regression")
    corpus = ROOT.parent / "target/release-audit-2026-08-final/corpus.json"
    samples = run("audit_release_compile", corpus, "samples", 0)
    if len(samples) != 310 or any(r["status"] != "ok" for r in samples): raise ValueError("Sample compilation failed")
    checks = {}
    for label, count in [("sysml",460), ("lexer",14), ("authoring",48), ("candidate",16)]:
        log = support / ("g05-owned-annotations-" + label + ".log")
        values = re.findall(r"test result: ok\. (\d+) passed; 0 failed;", log.read_text(encoding="utf-8-sig"))
        if values != [str(count)]: raise ValueError("Unexpected check result: " + label)
        checks[label] = {"passed":count,"log_sha256":digest(log)}
    log = support / "g05-owned-annotations-workspace.log"
    if "Finished `dev` profile" not in log.read_text(encoding="utf-8-sig"): raise ValueError("Workspace check incomplete")
    checks["workspace_all_targets"] = {"passed":True,"log_sha256":digest(log)}
    commands = [
        ["python", "-B", "tools/generate_release_namespace_checks.py", "--pilot-root", str(ROOT.parent / "target/upstream/SysML-v2-Pilot-Implementation"), "--out", "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/namespace-checks.extract.json", "--check"],
        ["python", "-B", "tools/generate_release_fields.py", "--metamodel", "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/metamodel.extract.json", "--selection", "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/mappings/candidate-fields.selection.json", "--out", "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/mappings/candidate-fields.extract.json", "--check"]]
    for command in commands: subprocess.run(command, cwd=ROOT, check=True)
    if before != sources(): raise ValueError("Source drift during verification")
    write(evidence_dir / "annotation-completion-evidence.json", {"scope":"G05 textual annotations and documentation; metadata annotating elements remain G12. No full comparison or benchmark.", "compiler_source_sha256":before, "binary_sha256":binaries, "library_sha256":digest(library), "checks":checks, "legality":legality, "prior_explicit_graphs":prior, "samples":{"accepted":310,"contexts_sha256":digest(corpus),"results_sha256":digest(folder / "samples.jsonl")}, "graph_evidence":{suite:digest(evidence_dir / (suite + "-graph-evidence.json")) for suite in ["owned-annotations","bare-comments"]}, "extractors_verified":True, "runner_sha256":digest(Path(__file__))})
    print("Verified annotation completion: 24 fresh controls, 26 new graphs, eight prior graphs, 310 sample contexts, and regression checks")
if __name__ == "__main__": main()
