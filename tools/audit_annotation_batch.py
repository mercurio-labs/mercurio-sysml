"""Compare ordered comment/Annotation graphs to isolated pinned Pilot exports."""
import json
import os
import re
import sys
from pathlib import Path
import subprocess
from run_namespace_batch import ROOT, read, write, digest
from run_declaration_milestone import sources


def projection(graph, comment):
    nodes = {}
    for element in graph["elements"]:
        name = element["qualified_name"]
        if name in nodes: raise ValueError("Ambiguous Pilot identity: " + name)
        nodes[name] = element
    def anchor(name):
        node = nodes[name]
        return {"kind": node["kind"], "name": node["properties"].get("declared_name"), "line": node["source"]["start_line"]}
    def ids(node, field):
        sequence = node["properties"].get("reference_sequences", {}).get(field)
        if sequence is not None: return sequence["targets"]
        return [r["target"] for r in graph["relationships"] if r["source"] == node["qualified_name"] and r["relation"] == field]
    def refs(node, field): return [anchor(name) for name in ids(node, field)]
    owners = ids(comment, "owner")
    memberships = ids(comment, "owning_membership")
    if len(owners) != 1 or len(memberships) != 1: raise ValueError("Unexpected Pilot comment ownership")
    membership = nodes[memberships[0]]
    annotations = []
    targets = []
    for identity in ids(comment, "owned_relationship"):
        node = nodes[identity]
        if node["kind"] != "Annotation": raise ValueError("Unexpected owned relationship")
        target = refs(node, "annotated_element")
        targets += target
        annotations.append({"kind": "Annotation", "line": node["source"]["start_line"], "source": refs(node, "source"), "target": refs(node, "target"), "annotated_element": target, "owned_related_element": refs(node, "owned_related_element")})
    # The pinned ElementUtil.getAnnotatedElementOf derives the namespace target
    # only when there are no Annotation relationships.
    return {"owner": anchor(owners[0]), "body": comment["properties"]["body"], "membership": {"kind": membership["kind"], "owner": refs(membership, "membership_owning_namespace"), "member": refs(membership, "member_element")}, "annotated_elements": targets or [anchor(owners[0])], "annotations": annotations}


def main():
    before = sources()
    artifacts = ROOT.parent / "target"
    support = artifacts / "support-2026-08"
    folder = support / "annotations-controls"
    evidence = ROOT / "docs/conformance/2026-08-support"
    oracle = read(evidence / "annotations-pilot-controls.json")
    helper = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java"
    jar = artifacts / "upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"
    if oracle["helper_sha256"] != digest(helper) or oracle["jar_sha256"] != digest(jar): raise ValueError("Pilot export provenance drift")
    specs = read(folder / "spec.json")
    expected = {row["relative_path"]: row["status"] for row in oracle["cases"]}
    cases, rows = [], []
    for case in specs["cases"]:
        name = case["relative_path"]
        if digest(Path(case["input_files"][0])) != oracle["source_sha256"][name]: raise ValueError("Fixture drift")
        if expected[name] != "ok": continue
        cases.append(case)
        graph = read(folder / name / "graph.json")
        comments = [e for e in graph["elements"] if e["kind"] == "Comment"]
        if len(comments) != 2: raise ValueError("Expected two comments in each positive control")
        for comment in comments:
            rows.append({"source": name, "line": comment["source"]["start_line"], "name": comment["properties"].get("declared_name"), "kind": "SysML::Comment", "property": "@comment_annotation_graph", "pilot": projection(graph, comment)})
    write(folder / "graph-spec.json", {"cases": cases, "expectations": rows})
    library = support / "stdlib.inherited-types.kir.json"
    env = dict(os.environ, MERCURIO_STDLIB_PATH=str(library), MERCURIO_KERNEL_LIBRARY_PATH=str(library))
    def run(tool, spec, name, code):
        output = folder / (name + ".jsonl")
        with (folder / (name + ".log")).open("w", encoding="utf-8") as log:
            result = subprocess.run([str(ROOT / "target/debug" / (tool + ".exe")), str(spec), str(output)], env=env, stdout=log, stderr=subprocess.STDOUT)
        if result.returncode != code: raise ValueError("Audit failed: " + name)
        return [json.loads(line) for line in output.read_text(encoding="utf-8").splitlines()]
    controls = run("audit_release_compile", folder / "spec.json", "native-controls", 1)
    if len(controls) != 8 or {r["relative_path"]: r["status"] for r in controls} != expected: raise ValueError("Control acceptance differs")
    graphs = run("audit_release_properties", folder / "graph-spec.json", "native-graphs", 0)
    if len(graphs) != 8 or any(r["status"] != "match" for r in graphs): raise ValueError("Annotation graphs differ")
    ordered = next(row for row in rows if len(row["pilot"]["annotations"]) == 2)
    wrong = json.loads(json.dumps(ordered))
    wrong["pilot"]["annotations"].reverse()
    wrong["pilot"]["annotated_elements"].reverse()
    write(folder / "wrong-order-spec.json", {"cases": [case for case in cases if case["relative_path"] == wrong["source"]], "expectations": [wrong]})
    negative = run("audit_release_properties", folder / "wrong-order-spec.json", "wrong-order", 1)
    if len(negative) != 1 or negative[0]["status"] != "different": raise ValueError("Graph audit ignored annotation ordering")
    prior = run("audit_release_properties", support / "g05-source-anchor-spec.json", "prior-anchors", 0)
    if len(prior) != 67 or any(r["status"] != "match" for r in prior): raise ValueError("Prior source anchors regressed")
    if before != sources(): raise ValueError("Sources changed during audit")
    write(evidence / "annotation-graph-evidence.json", {"scope": "Explicit comment-owned Annotation and namespace-owned Comment graphs; relationship-owned annotating elements and complete checkAnnotation remain open", "compiler_source_sha256": before, "source_sha256": oracle["source_sha256"], "library_sha256": digest(library), "binary_sha256": {tool: digest(ROOT / "target/debug" / (tool + ".exe")) for tool in ["audit_release_compile", "audit_release_properties"]}, "controls": controls, "graphs": graphs, "reversed_order_control": negative, "prior_anchors": {"matched": 67, "sha256": digest(folder / "prior-anchors.jsonl")}})
    print("Verified eight Pilot/native controls, eight comment/Annotation graphs and 67 prior anchors")


def publish_checks():
    support = ROOT.parent / "target/support-2026-08"
    path = ROOT / "docs/conformance/2026-08-support/annotation-graph-evidence.json"
    evidence = read(path)
    if evidence["compiler_source_sha256"] != sources(): raise ValueError("Source drift since graph audit")
    for name, expected in evidence["binary_sha256"].items():
        if digest(ROOT / "target/debug" / (name + ".exe")) != expected: raise ValueError("Audit binary drift")
    checks = {}
    for label, count in [("sysml", 455), ("authoring", 47), ("lexer", 14), ("candidate", 12)]:
        log = support / ("g05-annotations-" + label + ".log")
        results = re.findall(r"test result: ok\. (\d+) passed; 0 failed;", log.read_text(encoding="utf-8-sig"))
        if results != [str(count)]: raise ValueError("Unexpected test result: " + label)
        checks[label] = {"passed": count, "log_sha256": digest(log)}
    log = support / "g05-annotations-workspace.log"
    if "Finished `dev` profile" not in log.read_text(encoding="utf-8-sig"): raise ValueError("Workspace check incomplete")
    checks["workspace_all_targets"] = {"passed": True, "log_sha256": digest(log)}
    evidence["checks"] = checks
    evidence["audit_script_sha256"] = digest(Path(__file__))
    write(path, evidence)
    print("Published checked annotation graph and regression evidence")


if __name__ == "__main__":
    if sys.argv[1:] == ["--publish-checks"]:
        publish_checks()
    elif not sys.argv[1:]:
        main()
    else:
        raise SystemExit("usage: audit_annotation_batch.py [--publish-checks]")
