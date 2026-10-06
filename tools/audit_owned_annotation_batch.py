"""Focused exact ownership projection from isolated Pilot annotation controls."""
import json
import argparse
import os
from pathlib import Path
import subprocess
from run_namespace_batch import ROOT, read, write, digest
from run_declaration_milestone import sources
FIELDS = "owner owning_relationship owning_namespace owning_membership annotation annotated_element annotating_element owning_annotating_relationship owned_annotating_relationship owned_annotating_element owning_annotating_element owning_annotated_element owning_related_element related_element source target owned_related_element owned_relationship".split()

def projection(graph, element):
    nodes = {}
    for node in graph["elements"]:
        if node["qualified_name"] in nodes: raise ValueError("Ambiguous Pilot graph")
        nodes[node["qualified_name"]] = node
    def anchor(identity):
        node = nodes[identity]
        return {"kind": node["kind"], "name": node["properties"].get("declared_name"), "line": node["source"]["start_line"]}
    def ids(node, field):
        value = node["properties"].get("reference_sequences", {}).get(field)
        if value is not None: return value["targets"]
        return [r["target"] for r in graph["relationships"] if r["source"] == node["qualified_name"] and r["relation"] == field]
    def project(node):
        return {"anchor": anchor(node["qualified_name"]), "body": node["properties"].get("body"), "language": node["properties"].get("language"), "refs": {f: [anchor(i) for i in ids(node, f)] for f in FIELDS}}
    return {"element": project(element), "annotations": [project(nodes[i]) for i in ids(element, "annotation")]}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", choices=["owned-annotations", "bare-comments", "membership-bodies", "expose-bodies"], default="owned-annotations")
    suite = parser.parse_args().suite
    before = sources()
    support = ROOT.parent / "target/support-2026-08"
    folder = support / (suite + "-controls")
    evidence = ROOT / "docs/conformance/2026-08-support"
    oracle = read(evidence / (suite + "-pilot-controls.json"))
    helper = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java"
    jar = ROOT.parent / "target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"
    if oracle["helper_sha256"] != digest(helper) or oracle["jar_sha256"] != digest(jar): raise ValueError("Pilot provenance drift")
    spec = read(folder / "spec.json")
    expected = {r["relative_path"]:r["status"] for r in oracle["cases"]}
    rows, cases, differences = [], [], []
    for case in spec["cases"]:
        name = case["relative_path"]
        if digest(Path(case["input_files"][0])) != oracle["source_sha256"][name]: raise ValueError("Fixture drift")
        if expected[name] != "ok": continue
        cases.append(case)
        graph = read(folder / name / "graph.json")
        elements = [e for e in graph["elements"] if e["kind"] in ["Comment", "Documentation", "TextualRepresentation"]]
        for element in elements:
            raw = projection(graph, element)
            value = json.loads(json.dumps(raw))
            refs = value["element"]["refs"]
            owning = refs["owning_relationship"]
            if owning and owning[0]["kind"] == "Annotation":
                if len(owning) != 1: raise ValueError("Expected one owning relationship")
                owners = [e for e in graph["elements"] if e["qualified_name"] in element["properties"]["reference_sequences"]["owning_relationship"]["targets"]]
                if len(owners) != 1: raise ValueError("Ambiguous owning Annotation")
                # Pinned delegate uses RELATIONSHIP__OWNING_RELATED_ELEMENT instead
                # of ELEMENT__OWNING_RELATIONSHIP. Preserve the normative inverse
                # explicitly, retaining every raw discrepancy as evidence below.
                if refs["owning_annotating_relationship"]: raise ValueError("Pilot defect changed; re-review exception")
                refs["owning_annotating_relationship"] = owning
                refs["annotation"] = owning + refs["owned_annotating_relationship"]
                owning_node = projection(graph, owners[0])["element"]
                value["annotations"].insert(0, owning_node)
                refs["annotated_element"] = []
                for annotation in value["annotations"]:
                    for target in annotation["refs"]["annotated_element"]:
                        if target not in refs["annotated_element"]: refs["annotated_element"].append(target)
                differences.append({"source":name, "line":element["source"]["start_line"], "raw_pilot":raw, "required_inverse_graph":value})
            rows.append({"source": name, "line": element["source"]["start_line"], "name": element["properties"].get("declared_name"), "kind": "SysML::" + element["kind"], "property": "@owned_annotation_graph", "pilot": value})
    count = 14 if suite == "owned-annotations" else 12
    if len(rows) != count: raise ValueError("Unexpected annotating element count")
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
    if len(controls) != 8 or {r["relative_path"]:r["status"] for r in controls} != expected: raise ValueError("Acceptance differs")
    graphs = run("audit_release_properties", folder / "graph-spec.json", "native-graphs", 0)
    if len(graphs) != count or any(r["status"] != "match" for r in graphs): raise ValueError("Ownership graphs differ")
    wrong = json.loads(json.dumps(next(row for row in rows if row["pilot"]["annotations"])))
    wrong["pilot"]["annotations"][0]["refs"]["owner"] = wrong["pilot"]["annotations"][0]["refs"]["owning_related_element"]
    write(folder / "wrong-owner-spec.json", {"cases": [c for c in cases if c["relative_path"] == wrong["source"]], "expectations": [wrong]})
    negative = run("audit_release_properties", folder / "wrong-owner-spec.json", "wrong-owner", 1)
    if len(negative) != 1 or negative[0]["status"] != "different": raise ValueError("Audit ignored wrong Annotation owner")
    prior = run("audit_release_properties", support / "g05-source-anchor-spec.json", "prior-anchors", 0)
    if len(prior) != 67 or any(r["status"] != "match" for r in prior): raise ValueError("Prior anchors regressed")
    if before != sources(): raise ValueError("Source drift")
    write(evidence / (suite + "-graph-evidence.json"), {"scope": "Comment, Documentation, TextualRepresentation ownership for " + suite + "; exact selected reference projection", "compiler_source_sha256":before, "source_sha256":oracle["source_sha256"], "helper_sha256":digest(helper), "library_sha256":digest(library), "binary_sha256":{name:digest(ROOT / "target/debug" / (name + ".exe")) for name in ["audit_release_compile", "audit_release_properties"]}, "controls":controls, "graphs":graphs, "pilot_inverse_defect": {"delegate":"AnnotatingElement_owningAnnotatingRelationship_SettingDelegate", "reason":"Uses Relationship containment feature ID for an AnnotatingElement. The metamodel opposite Annotation.ownedAnnotatingElement requires the retained inverse; only its dependent annotation and annotatedElement lists differ.", "cases":differences, "source_sha256":{name:digest(ROOT.parent / "target/upstream/SysML-v2-Pilot-Implementation" / name) for name in ["org.omg.sysml/model/kerml.ecore", "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/AnnotatingElement_owningAnnotatingRelationship_SettingDelegate.java", "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Relationship_owningRelatedElement_SettingDelegate.java"]}}, "wrong_owner_control":negative, "prior_anchors":{"matched":67,"sha256":digest(folder / "prior-anchors.jsonl")}})
    print(f"Verified eight acceptance controls, {count} annotation graphs, wrong-owner rejection, and 67 prior anchors")
if __name__ == "__main__": main()
