"""Finite shared-service batch evidence; never full V01/family qualification."""
import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path
from audit_value_result_provider_plan import native_identity

ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT/"docs/conformance/2026-08-support"
EVIDENCE=BASE/"definition-pipeline-evidence"
PREFIX="value-result-lifecycle-shared-services"

def read(name):
    return json.loads((EVIDENCE/name).read_text(encoding="utf-8"))

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def require(value,message):
    if not value:raise ValueError(message)

def compare(spec,reference,native,identities):
    ports=spec["ports"]
    require(len(reference["initial_source_node_counts"])==len(reference["final_source_node_counts"])==25,
        "Reference source structural-state denominator changed")
    require(len(ports)==len(reference["observations"])==1065,"All1065 concrete shared-service reads are required")
    require(len({(p["owner_id"],p["field"]) for p in ports})==1065,"Duplicate concrete port")
    require(spec["fixed_contexts"]==26 and spec["source_contexts_parsed"]==25
        and not spec["mutations_applied"],"Frozen lifecycle context boundary changed")
    require(native["status"]=="read_only_diagnostic" and native["publication"]=="not_attempted"
        and native["semantic_qualification"]=="not_assessed" and not native["qualification_certificate"],
        "Reference reads were promoted to lifecycle support")
    answers={(q["owner_id"],q["field"]):q for q in native["queries"]}
    require(len(answers)==len(native["queries"])==4462,"Full native read denominator changed")
    mapping=identities["canonical_resource_fragment_to_native_id"]
    require(identities["nodes_mapped"]==len(mapping)==388 and identities["source_contexts_mapped"]==25,
        "Native canonical source identity map pruned")
    def endpoint_id(value):
        key=value["resource"].replace(chr(92),"/")+"#"+value["emf_fragment"]
        if key in mapping:return mapping[key]
        if value["resource"].startswith("sysml.library/"):
            return native_identity(value["resource"],value["emf_fragment"])
        return None
    rows=[]
    for port,observed in zip(ports,reference["observations"]):
        key=(port["owner_id"],port["field"])
        require(key==(observed["root"]["owner_id"],observed["root"]["field"]),"Upstream port order/identity changed")
        require(observed["status"] in {"reference_observed","reference_getter_failed"} and not observed["provider_completion_before"]
            and not observed["provider_completion_after"],"Pilot receiver was prepared or query failed")
        require(endpoint_id(observed["owner"])==port["owner_id"]
            and observed["owner"]["kind"]==port["owner_kind"].split("::")[-1],"Canonical source owner differs")
        resolved=observed["feature_declaring_kind"]+"/"+observed["feature"]
        require(any(f.endswith("#//"+resolved) for f in port["feature_candidates"]),
            "Observed getter is not a resolved imported feature")
        answer=answers[key]
        expected=[endpoint_id(target) for target in observed["endpoints"]]
        if observed["status"]=="reference_getter_failed":status="reference_getter_failed"
        elif answer["status"]!="query_evaluated":status=answer["status"]
        elif any(target is None for target in expected):status="reference_endpoint_unrepresented"
        elif [target["id"] for target in answer["targets"]]!=expected:status="ordered_endpoint_difference"
        elif [target["kind"].split("::")[-1] for target in answer["targets"]]!=[p["kind"] for p in observed["endpoints"]]:
            status="endpoint_metaclass_difference"
        else:status="exact_ordered_match"
        require(all(isinstance(observed[k],int) and observed[k]>=0 for k in [
            "source_nodes_before","source_nodes_after","owner_relationships_before","owner_relationships_after"]),
            "Upstream structural state omitted")
        reference_construction=observed["source_nodes_before"]!=observed["source_nodes_after"]
        rows.append({"context":port["context"],"owner_id":port["owner_id"],"field":port["field"],
            "comparison":status,"reference_constructed_during_getter":reference_construction,"native_status":answer["status"],"independent_endpoint_count":len(expected) if observed["status"]=="reference_observed" else None,
            "complete_native_context":False})
    return {"reference_initial_source_nodes":sum(reference["initial_source_node_counts"].values()),
        "reference_final_source_nodes":sum(reference["final_source_node_counts"].values()),
        "native_source_nodes":388,
        "reference_construction_queries":sum(row["reference_constructed_during_getter"] for row in rows),
        "counts":dict(sorted(Counter(row["comparison"] for row in rows).items())),
        "by_service":{field:dict(sorted(Counter(row["comparison"] for row in rows if row["field"]==field).items()))
            for field in sorted({row["field"] for row in rows})},"ports":rows}

def audit(check=False):
    scope=read(PREFIX+"-scope.json")
    require(len(scope["services"])==6 and sum(s["baseline_unavailable_reads"] for s in scope["services"])==1065,
        "Fixed shared-service denominator changed")
    for path,digest in scope["frozen_inputs"].items():
        require(sha(ROOT/path)==digest,"Frozen baseline changed: "+path)
    pilot=read(PREFIX+"-pilot-actual-run.json")
    require(pilot["exit_code"]==0 and pilot["inputs_unchanged"],"Upstream observation execution failed")
    require(sha(EVIDENCE/(PREFIX+"-pilot-observations.json"))==pilot["output_sha256"],"Reference cache changed")
    require(sha(EVIDENCE/(PREFIX+"-pilot-spec.json"))==pilot["spec_sha256"],"Reference request scope changed")
    for group in [pilot["source_sha256"],pilot["source_input_sha256"]]:
        for path,digest in group.items():require(sha(ROOT/path)==digest,"Upstream source/tool changed")
    jar=Path(pilot["runs"][0]["command"][4])
    require(sha(jar)==pilot["jar_sha256"],"Pinned upstream executable changed")
    release=ROOT.parent/"target/upstream/SysML-v2-Release"
    for path,digest in pilot["pinned_library_source_sha256"].items():
        require(sha(release/path)==digest,"Pinned actual library changed")
    run=read(PREFIX+"-native-run.json")
    require(run["exit_code"]==0 and run["inputs_unchanged"],"Native full read matrix failed")
    require(sha(EVIDENCE/(PREFIX+"-native-queries.json"))==run["output_sha256"],"Native read results changed")
    for path,digest in run["consumer_sha256"].items():
        require(sha(ROOT/path)==digest,"Native consumer changed: "+path)
    require(sha(ROOT/"target/release/audit_release_compile.exe")==run["binary_sha256"],"Native query binary changed")
    identities=read(PREFIX+"-source-identities.json")
    require(sha(EVIDENCE/"value-result-lifecycle-source-inspection.jsonl")==identities["source_constructor_sha256"],
        "Historical source construction changed")
    result=compare(read(PREFIX+"-pilot-spec.json"),read(PREFIX+"-pilot-observations.json"),
        read(PREFIX+"-native-queries.json"),identities)
    focused=read(PREFIX+"-focused-run.json")
    require(focused["exit_code"]==0 and focused["inputs_unchanged"],"Focused native controls failed")
    require(sha(Path(focused["executable"]))==focused["executable_sha256"],"Focused native test binary changed")
    for path,digest in focused["consumer_sha256"].items():require(sha(ROOT/path)==digest,"Focused consumer changed")
    log=EVIDENCE/(PREFIX+"-focused.log")
    require(sha(log)==focused["log_sha256"],"Focused log changed")
    text=log.read_text(encoding="utf-8")
    require("test result: FAILED" not in text and len(re.findall(r"^test .* \.\.\. ok$",text,re.M))==19,
        "All19 focused controls must pass")
    controls=read(PREFIX+"-pilot-controls.json")
    require(len(controls["observations"])==19 and not controls["qualification_certificate"]
        and all(not row["completion_before"] and not row["completion_after"] for row in controls["observations"]),
        "Independent factory controls incomplete or prepared")
    control_run=read(PREFIX+"-pilot-run.json")
    require(control_run["exit_code"]==0 and control_run["inputs_unchanged"]
        and sha(EVIDENCE/(PREFIX+"-pilot-controls.json"))==control_run["output_sha256"],
        "Independent factory control provenance invalid")
    from audit_value_result_provider_semantics import audit as provider_audit
    providers=provider_audit(check,PREFIX+"-provider")
    require(providers["comparison"]["counts"]=={"exact_ordered_match":115}
        and providers["current_root_reads"]["current_native_provider_roots_verified"]==9,
        "Existing provider getter component regressed")
    proof={"schema":"dev.mercurio.shared-reference-batch-comparison.v1","qualification_certificate":False,
        "comparison":result,"native_full_read_counts":run["counts"],
        "current_provider_getters_verified":115,"current_provider_roots_verified":9,"independent_factory_controls":19,
        "factory_exact_matches":18,"reviewed_factory_normative_disagreements":1,"focused_native_controls":19,
        "complete_native_contexts":0,"strict_families_qualified":0,"release_gates_qualified":0,
        "remaining_boundary":"Reference getter components only; actual linking, semantic transformations, validators, publication, persistence and Ecore mutation contexts remain open.",
        "input_sha256":{str(p.relative_to(ROOT)):sha(p) for p in [
            EVIDENCE/(PREFIX+"-scope.json"),EVIDENCE/(PREFIX+"-native-run.json"),
            EVIDENCE/(PREFIX+"-native-queries.json"),EVIDENCE/(PREFIX+"-pilot-actual-run.json"),
            EVIDENCE/(PREFIX+"-pilot-observations.json"),EVIDENCE/(PREFIX+"-pilot-spec.json"),
            EVIDENCE/(PREFIX+"-source-identities.json"),EVIDENCE/(PREFIX+"-focused-run.json"),
            EVIDENCE/(PREFIX+"-pilot-run.json"),EVIDENCE/(PREFIX+"-pilot-controls.json"),
            EVIDENCE/(PREFIX+"-normative-review.json"),
            EVIDENCE/(PREFIX+"-provider-comparison.json"),Path(__file__),ROOT/"tools/test_audit_value_result_shared_services.py"]}}
    output=EVIDENCE/(PREFIX+"-comparison.json")
    rendered=json.dumps(proof,indent=2)+"\n"
    if check:require(output.read_text(encoding="utf-8")==rendered,"Comparison evidence stale")
    else:output.write_text(rendered,encoding="utf-8",newline="\n")
    return proof

if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--check",action="store_true")
    result=audit(parser.parse_args().check)
    print(json.dumps({k:result[k] for k in ["native_full_read_counts","complete_native_contexts","strict_families_qualified"]},indent=2))
    print(json.dumps(result["comparison"]["counts"],indent=2))
