"""Audit documentation controls and the original unmatched Documentation Example."""
import json
import os
import re
from pathlib import Path
import subprocess
from run_namespace_batch import ROOT, read, write, digest
from summarize_release_assessment import load, direct
from run_declaration_milestone import sources

def main():
    source_hashes = sources()
    artifacts=ROOT.parent/"target"
    support=artifacts/"support-2026-08/documentation-controls"
    evidence=ROOT/"docs/conformance/2026-08-support"
    oracle=read(evidence/"documentation-pilot-controls.json")
    specs=read(support/"spec.json")
    for case in specs["cases"]:
        if digest(Path(case["input_files"][0]))!=oracle["source_sha256"][case["relative_path"]]: raise ValueError("Fixture drift")
    positive=[c for c in specs["cases"] if c["relative_path"].startswith("positive")]
    expectations=[]
    for case in positive:
        graph=read(support/case["relative_path"]/"graph.json")
        docs=[e for e in graph["elements"] if e["kind"]=="Documentation" and e["properties"].get("declared_name")]
        if len(docs)!=2: raise ValueError("Expected two named Pilot documentation controls")
        for doc in docs:
            for field in ["declared_name","body","locale","declared_short_name"]:
                if field in doc["properties"]:
                    expectations.append({"source":case["relative_path"],"line":doc["source"]["start_line"],"name":doc["properties"]["declared_name"],"property":field,"pilot":doc["properties"][field]})
    original="sysml/src/training/01. Packages/Documentation Example.sysml"
    corpus=read(artifacts/"release-audit-2026-08-final/corpus.json")["cases"]
    selected=[c for c in corpus if c["relative_path"]==original]
    if len(selected)!=1: raise ValueError("Original documentation context missing")
    summary=read(artifacts/"assessment-2026-08-declaration-m1/semantic-summary.json")
    cases=[c for c in summary["cases"] if c["relative_path"]==original]
    graph=load(cases[0]["artifact"]["path"])
    docs=[e for e in graph["pilot_snapshot"]["elements"] if e.get("declared_name")=="Document1" and e["source_span"]["start_line"]==7]
    if len(docs)!=1: raise ValueError("Original Pilot anchor missing or ambiguous")
    properties=direct(docs[0])
    for field in ["declared_name","body"]:
        expectations.append({"source":original,"line":7,"name":"Document1","property":field,"pilot":properties[field]})
    write(support/"properties-spec.json",{"cases":positive+selected,"expectations":expectations})
    env=dict(os.environ)
    library=artifacts/"support-2026-08/stdlib.inherited-types.kir.json"
    env["MERCURIO_STDLIB_PATH"]=str(library);env["MERCURIO_KERNEL_LIBRARY_PATH"]=str(library)
    results={}
    for label,binary,spec,code in [("controls","audit_release_compile","spec.json",1),("properties","audit_release_properties","properties-spec.json",0)]:
        output=support/(label+".jsonl")
        with (support/(label+".log")).open("w",encoding="utf-8") as log:
            result=subprocess.run([str(ROOT/"target/debug"/(binary+".exe")),str(support/spec),str(output)],env=env,stdout=log,stderr=subprocess.STDOUT)
        if result.returncode!=code: raise ValueError("Native audit failed: "+label)
        results[label]=[json.loads(line) for line in output.read_text(encoding="utf-8").splitlines()]
    expected={r["relative_path"]:r["status"] for r in oracle["cases"]}
    if len(results["controls"])!=len(expected) or {r["relative_path"]:r["status"] for r in results["controls"]}!=expected: raise ValueError("Native/Pilot acceptance mismatch")
    if len(results["properties"])!=len(expectations) or any(r["status"]!="match" for r in results["properties"]): raise ValueError("Documentation property mismatch")
    checks = {}
    for label, expected_count in [("final-tests", 9), ("candidate", 9), ("lexer", 14), ("authoring", 47)]:
        log = artifacts / "support-2026-08" / ("g05-documentation-" + label + ".log")
        counts = re.findall(r"test result: ok\. (\d+) passed; 0 failed;", log.read_text(encoding="utf-8-sig"))
        if counts != [str(expected_count)]: raise ValueError("Unexpected focused test result: " + label)
        checks[label] = {"passed": expected_count, "log_sha256": digest(log)}
    prior_path = artifacts / "support-2026-08/g05-documentation-prior-anchors.jsonl"
    prior = [json.loads(line) for line in prior_path.read_text(encoding="utf-8").splitlines()]
    if len(prior) != 67 or any(row["status"] != "match" for row in prior): raise ValueError("Prior anchor regression")
    if source_hashes != sources(): raise ValueError("Sources changed during documentation audit")
    write(evidence/"documentation-header-evidence.json",{"checks":checks,"prior_source_anchors":{"matched":67,"sha256":digest(prior_path)},"compiler_source_sha256":source_hashes,"scope":"Documentation header/ownership batch; anonymous legacy docs and complete Annotation relationships remain G05.c", "library_sha256":digest(library),"source_sha256":oracle["source_sha256"],"binary_sha256":{n:digest(ROOT/"target/debug"/(n+".exe")) for n in ["audit_release_compile","audit_release_properties"]},**results})
    print(f"Verified {len(results['controls'])} shared controls and {len(expectations)} original/fresh Pilot property rows")
if __name__=="__main__": main()
