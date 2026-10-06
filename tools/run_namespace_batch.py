"""Focused G05 source-anchor and import controls; no full comparison or benchmark."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "docs/conformance/2026-08-support"

def read(path):
    return json.loads(path.read_text(encoding="utf-8"))
def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", choices=["prepare", "oracle", "checks", "audit"], required=True)
    parser.add_argument("--java-bin", type=Path)
    args = parser.parse_args()
    artifacts = ROOT.parent / "target"
    support = artifacts / "support-2026-08"
    fixtures = ROOT / "crates/mercurio-tools/corpus/release-2026-08/namespace-imports"
    cases = [{"relative_path": p.name, "input_files": [str(p)]} for p in sorted(fixtures.glob("*")) if p.suffix in [".sysml", ".kerml"]]
    if len(cases) != 8: raise ValueError("Expected eight isolated import fixtures")
    expected = {c["relative_path"]: ("error" if c["relative_path"].startswith(("root-public", "root-protected")) else "ok") for c in cases}
    hashes = {c["relative_path"]: digest(Path(c["input_files"][0])) for c in cases}
    if args.stage == "prepare":
        rows = read(EVIDENCE / "namespace-anchor-inventory.json")["rows"]
        anchors = [r for r in rows if r["status"] == "pilot_only" and r["source_line"].lstrip().startswith("#") and r["same_name_other_line_candidates"]]
        if len(anchors) != 60: raise ValueError("Expected 60 original prefixed anchors")
        anchors += [r for r in rows if r["status"] == "pilot_only" and r["source"].endswith(("CommentTest.sysml", "Comments.kerml"))]
        if len(anchors) != 67: raise ValueError("Expected 67 total source anchors")
        sources = {r["source"] for r in anchors}
        corpus = read(artifacts / "release-audit-2026-08-final/corpus.json")["cases"]
        write(support / "g05-source-anchor-spec.json", {"cases": [c for c in corpus if c["relative_path"] in sources], "expectations": [{"source":r["source"], "line":r["line"], "name":r["name"], "property":"declared_name", "pilot":r["name"]} for r in anchors]})
        write(support / "g05-import-controls-spec.json", {"cases":cases})
        return
    if args.stage == "checks":
        from run_declaration_milestone import sources
        before = sources()
        env = dict(os.environ, CARGO_INCREMENTAL="0")
        env.pop("MERCURIO_STDLIB_PATH", None)
        env.pop("MERCURIO_KERNEL_LIBRARY_PATH", None)
        commands = [
            ("namespace-tests", ["cargo", "test", "--locked", "-j1", "-p", "mercurio-sysml", "--lib", "release_2026_08_namespace"]),
            ("import-regressions", ["cargo", "test", "--locked", "-j1", "-p", "mercurio-sysml", "--lib", "import"]),
            ("foundation-lexer", ["cargo", "test", "--locked", "-j1", "-p", "mercurio-foundation", "--lib", "lexer"]),
            ("foundation-authoring", ["cargo", "test", "--locked", "-j1", "-p", "mercurio-foundation", "--lib", "authoring::"]),
            ("audit-build", ["cargo", "build", "--locked", "-j1", "-p", "mercurio-tools", "--bin", "audit_release_properties", "--bin", "audit_release_compile"]),
        ]
        for name, command in commands:
            with (support / ("g05-"+name+".log")).open("w", encoding="utf-8") as log:
                subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
            print("Passed: " + name, flush=True)
        env["MERCURIO_STDLIB_PATH"] = str(support / "stdlib.inherited-types.kir.json")
        env["MERCURIO_KERNEL_LIBRARY_PATH"] = env["MERCURIO_STDLIB_PATH"]
        with (support / "g05-candidate-tests.log").open("w", encoding="utf-8") as log:
            subprocess.run(commands[0][1], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        if before != sources(): raise ValueError("Sources changed during focused checks")
        write(support / "g05-checks.json", {"source_sha256":before, "binary_sha256":{name:digest(ROOT / "target/debug" / (name+".exe")) for name in ["audit_release_compile", "audit_release_properties"]}, "checks":[n for n,c in commands] + ["candidate-tests"]})
        return
    if args.stage == "oracle":
        pilot = artifacts / "upstream/SysML-v2-Pilot-Implementation"
        jar = artifacts / "upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"
        classes = support / "g05-oracle-classes"
        classes.mkdir(exist_ok=True)
        helper = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java"
        java = str(args.java_bin / "java.exe") if args.java_bin else "java"
        javac = str(args.java_bin / "javac.exe") if args.java_bin else "javac"
        subprocess.run([javac, "-cp", str(jar), "-d", str(classes), str(helper)], check=True)
        def run(case):
            folder = support / "g05-oracle" / case["relative_path"]
            spec, result = folder / "spec.json", folder / "result.json"
            write(spec, {"cases":[case]})
            with (folder / "oracle.log").open("w", encoding="utf-8") as log:
                subprocess.run([java, "-Xmx3g", "-cp", str(classes)+os.pathsep+str(jar), "dev.mercurio.pilot.PilotModelExporter", "--assessment", str(pilot / "sysml.library"), str(spec), "-", str(result)], stdout=log, stderr=subprocess.STDOUT, check=True, timeout=600)
            rows = read(result)["cases"]
            if len(rows) != 1 or rows[0]["status"] != expected[case["relative_path"]]:
                raise ValueError(f"Unexpected Pilot result: {rows}")
            if rows[0]["status"] == "error" and "validateImportTopLevelVisibility" not in json.dumps(rows[0]):
                raise ValueError("Missing expected Pilot visibility diagnostic")
            print("Pilot verified: " + case["relative_path"], flush=True)
            return rows[0]
        with ThreadPoolExecutor(max_workers=2) as pool:
            rows = list(pool.map(run, cases))
        write(EVIDENCE / "namespace-import-pilot.json", {"scope":"Isolated CheckMode.ALL controls; not benchmark timings", "source_sha256":hashes, "jar_sha256":digest(jar), "helper_sha256":digest(helper), "cases":rows})
        return
    from run_declaration_milestone import sources
    checks = read(support / "g05-checks.json")
    if checks["source_sha256"] != sources(): raise ValueError("Sources changed after focused checks")
    for name, sha in checks["binary_sha256"].items():
        if digest(ROOT / "target/debug" / (name+".exe")) != sha: raise ValueError("Binary changed after focused checks")
    oracle = read(EVIDENCE / "namespace-import-pilot.json")
    if hashes != oracle["source_sha256"]: raise ValueError("Pilot fixture drift")
    env = dict(os.environ)
    library = support / "stdlib.inherited-types.kir.json"
    env["MERCURIO_STDLIB_PATH"] = str(library)
    env["MERCURIO_KERNEL_LIBRARY_PATH"] = str(library)
    results = {}
    for label, binary, spec, code in [("source-anchors", "audit_release_properties", "g05-source-anchor-spec.json", 0), ("import-controls", "audit_release_compile", "g05-import-controls-spec.json", 1)]:
        output = support / ("g05-" + label + ".jsonl")
        binary = ROOT / "target/debug" / (binary + ".exe")
        with (support / ("g05-"+label+".log")).open("w", encoding="utf-8") as log:
            result = subprocess.run([str(binary), str(support / spec), str(output)], env=env, stdout=log, stderr=subprocess.STDOUT)
        if result.returncode != code: raise ValueError("Failed " + label)
        results[label] = [json.loads(line) for line in output.read_text(encoding="utf-8").splitlines()]
    if len(results["source-anchors"]) != 67 or any(r["status"] != "match" for r in results["source-anchors"]): raise ValueError("Source anchor failure")
    rows = results["import-controls"]
    if len(rows) != len(expected) or {r["relative_path"] for r in rows} != set(expected): raise ValueError("Wrong native target set")
    for row in rows:
        if row["status"] != expected[row["relative_path"]]: raise ValueError(f"Native mismatch: {row}")
        if row["status"] == "error" and "validateImportTopLevelVisibility" not in json.dumps(row): raise ValueError("Wrong native diagnostic")
    test_results = {}
    for name in ["namespace-tests", "import-regressions", "foundation-lexer", "foundation-authoring", "candidate-tests"]:
        log = support / ("g05-"+name+".log")
        matches = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored", log.read_text(encoding="utf-8"))
        if len(matches) != 1 or int(matches[0][1]) != 0: raise ValueError("Missing successful test log: " + name)
        test_results[name] = {"passed":int(matches[0][0]), "failed":0, "ignored":int(matches[0][2]), "log_sha256":digest(log)}
    write(EVIDENCE / "namespace-source-import-evidence.json", {"scope":"67 original declaration locations and eight import acceptance/diagnostic controls; G05 remains open", "library_sha256":digest(library), "binary_sha256": {name:digest(ROOT / "target/debug" / (name+".exe")) for name in ["audit_release_compile", "audit_release_properties"]}, "fixture_sha256":hashes, "checked_source_sha256":checks["source_sha256"], "checks":checks["checks"], "focused_test_results":test_results, "runner_sha256":digest(Path(__file__)), "source_anchor_rows":results["source-anchors"], "import_control_rows":rows})
    print("Verified 67 original anchors and eight shared import controls", flush=True)

if __name__ == "__main__":
    main()
