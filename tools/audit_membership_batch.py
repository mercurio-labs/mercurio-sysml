"""Bounded membership/import graph checks against pinned isolated Pilot controls."""
import json
import argparse
import os
import subprocess
from pathlib import Path
from run_namespace_batch import ROOT, read, write, digest
from run_declaration_milestone import sources

REFS = "source target owned_related_element owned_relationship member_element imported_membership imported_namespace".split()
SCALARS = "member_name member_short_name owned_member_name owned_member_short_name visibility is_implied is_import_all is_recursive".split()
KINDS = {"Membership", "OwningMembership", "FeatureMembership", "MembershipImport", "NamespaceImport", "MembershipExpose", "NamespaceExpose"}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", choices=["membership-bodies", "expose-bodies", "relationship-owned-elements"], default="membership-bodies")
    suite = parser.parse_args().suite
    count = {"membership-bodies":28, "expose-bodies":20, "relationship-owned-elements":6}[suite]
    before = sources()
    support = ROOT.parent / "target/support-2026-08"
    folder = support / (suite + "-controls")
    evidence = ROOT / "docs/conformance/2026-08-support"
    oracle = read(evidence / (suite + "-pilot-controls.json"))
    helper = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java"
    jar = ROOT.parent / "target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"
    if oracle["helper_sha256"] != digest(helper) or oracle["jar_sha256"] != digest(jar): raise ValueError("Pilot provenance drift")
    cases, rows, unassessed = [], [], []
    for case in read(folder / "spec.json")["cases"]:
        name = case["relative_path"]
        if digest(Path(case["input_files"][0])) != oracle["source_sha256"][name]: raise ValueError("Fixture drift")
        if not name.startswith("positive"): continue
        if suite == "relationship-owned-elements" and not name.endswith(".kerml"): continue
        cases.append(case)
        graph = read(folder / name / "graph.json")
        nodes = {node["qualified_name"]:node for node in graph["elements"]}
        if len(nodes) != len(graph["elements"]): raise ValueError("Ambiguous Pilot graph")
        def anchor(identity):
            node = nodes[identity]
            return {"kind":node["kind"], "name":node["properties"].get("declared_name"), "line":node["source"]["start_line"]}
        for node in graph["elements"]:
            if node["kind"] not in KINDS: continue
            if suite == "relationship-owned-elements" and "relationship-owned-elements/" not in node["source"].get("file", ""): continue
            if "start_line" not in node["source"]:
                unassessed.append({"source":name,"node":node,"assigned_gap":"G06/G07 implicit multiplicity ownership"})
                continue
            properties = node["properties"]
            refs = {}
            for field in REFS:
                sequence = properties.get("reference_sequences", {}).get(field)
                targets = sequence["targets"] if sequence is not None else [r["target"] for r in graph["relationships"] if r["source"] == node["qualified_name"] and r["relation"] == field]
                refs[field] = [anchor(target) for target in targets]
            rows.append({"source":name,"line":node["source"]["start_line"],"name":properties.get("declared_name"),"kind":"SysML::"+node["kind"],"property":"@membership_graph","pilot":{"refs":refs,"scalars":{field:properties.get(field) for field in SCALARS}}})
    if len(rows) != count: raise ValueError("Unexpected source membership/import count")
    if len(unassessed) != (4 if suite == "expose-bodies" else 0): raise ValueError("Unexpected unassessed implicit memberships")
    spec = folder / "membership-graph-spec.json"
    write(spec,{"cases":cases,"expectations":rows})
    library = support / "stdlib.inherited-types.kir.json"
    binary = ROOT / "target/debug/audit_release_properties.exe"
    env = dict(os.environ,MERCURIO_STDLIB_PATH=str(library),MERCURIO_KERNEL_LIBRARY_PATH=str(library))
    def run(spec,name,code):
        output = folder / (name+".jsonl")
        with (folder / (name+".log")).open("w",encoding="utf-8") as log:
            result = subprocess.run([str(binary),str(spec),str(output)],env=env,stdout=log,stderr=subprocess.STDOUT)
        if result.returncode != code: raise ValueError("Audit failed: "+name)
        return [json.loads(line) for line in output.read_text(encoding="utf-8").splitlines()]
    graphs = run(spec,"native-memberships",0)
    if len(graphs) != count or any(r["status"] != "match" for r in graphs): raise ValueError("Membership graphs differ")
    wrong = json.loads(json.dumps(next(row for row in rows if row["kind"] in ["SysML::MembershipImport", "SysML::MembershipExpose"])))
    wrong["pilot"]["refs"]["target"] = wrong["pilot"]["refs"]["source"]
    negative_spec = folder / "wrong-membership-target-spec.json"
    write(negative_spec,{"cases":[c for c in cases if c["relative_path"] == wrong["source"]],"expectations":[wrong]})
    negative = run(negative_spec,"wrong-membership-target",1)
    if len(negative) != 1 or negative[0]["status"] != "different": raise ValueError("Wrong membership target was ignored")
    if sources() != before: raise ValueError("Source drift")
    write(evidence / ({"membership-bodies":"membership", "expose-bodies":"expose", "relationship-owned-elements":"relationship-owned-elements"}[suite]+"-identity-graph-evidence.json"),{"scope":str(count)+" explicit source memberships and imports/exposes; selected ordered references and scalar fields. This is not complete namespace/visibility or library graph parity.","unassessed_implicit_memberships":unassessed,"compiler_source_sha256":before,"source_sha256":oracle["source_sha256"],"helper_sha256":digest(helper),"jar_sha256":digest(jar),"library_sha256":digest(library),"binary_sha256":digest(binary),"graphs":graphs,"wrong_target_control":negative})
    print(f"Verified {count} source membership/import/expose graphs and wrong-target rejection; {len(unassessed)} implicit multiplicity memberships remain assigned to G06/G07")

if __name__ == "__main__": main()
