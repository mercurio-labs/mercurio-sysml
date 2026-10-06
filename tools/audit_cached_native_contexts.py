"""Audit exact context dependency waves; never award terminal or family credit."""
import argparse
import json
from pathlib import Path
from audit_cached_native_dependencies import assess_successful_wave, exact_json, require, sha

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "docs/conformance/2026-08-support/definition-pipeline-evidence"
PREFIX = "value-result-cached-dependency-"

def read(name):
    return json.loads((BASE/(PREFIX+name)).read_text(encoding="utf-8"))

def root_id(uri):
    return "definition.resource."+uri.encode("utf-8").hex()

def resource_id(identity):
    require(identity.startswith("definition.resource."), "Foreign identity prefix")
    return "definition.resource."+identity[len("definition.resource."):].split(".",1)[0]

def assess():
    scope,plan,run = read("context-scope.json"),read("context-plan-spec.json"),read("context-execution-run.json")
    bundle_path=ROOT/"docs/conformance/2026-08-support/value-result-obligation-bundle.json"
    bundle=json.loads(bundle_path.read_text(encoding="utf-8"))
    cache=ROOT/"target/value-result-fresh-structure/constructor-records.jsonl"
    require(sha(cache)==plan["input_sha256"] and sha(bundle_path)==plan["obligation_bundle_sha256"],
            "Context cache or frozen obligation bundle changed")
    require(scope["fixed_context_count"]==26 and len(scope["contexts"])==len(bundle["contexts"])==26
        and len(plan["batches"])==scope["planned_constructed_context_waves"]==25
        and scope["actual_library_resources_per_wave"]==94, "Context denominator changed")
    require(not scope["acceptance_changed"] and not scope["candidate_promoted"]
        and scope["complete_native_contexts"]==scope["strict_families_qualified"]==0,
        "Component execution claims qualification")
    for proof in [scope,run]:
        require(proof["qualification_certificate"] is False,"Execution claims a certificate")
        for path,digest in proof["input_sha256"].items():
            require(sha(Path(path))==digest,"Context input changed: "+path)
    require(run["inputs_unchanged"],"Context run input changed")
    for path,digest in run["consumer_sha256"].items():
        require(sha(ROOT/path)==digest,"Context consumer changed: "+path)
    output=Path(run["output_path"])
    require(sha(output)==run["output_sha256"] and sha(BASE/(PREFIX+"context-execution.log"))==run["log_sha256"],
            "Context output or log changed")
    original=json.loads(cache.read_text(encoding="utf-8"))
    paths=original["input_files"]
    nodes,pending=original["inspection"]["constructed_elements"],original["inspection"]["pending_references"]
    require(len(paths)==119 and len(nodes)==93708 and len(pending)==18929,"Constructor denominator changed")
    release=ROOT.parent/"target/upstream/SysML-v2-Release"
    libraries=[path for path in paths if Path(path).is_relative_to(release)]
    require(len(libraries)==94,"All actual library resources required")
    for path in libraries:
        relative=Path(path).relative_to(release).as_posix()
        require(sha(Path(path))==bundle["resource_environment"]["pinned_library_fingerprints"][relative],
                "Pinned library changed: "+relative)
    preserved=["id","role","source","source_sha256","mutation","independent_outcome","required_rejection_stage","required_stages"]
    by_batch={}
    for frozen,current in zip(bundle["contexts"],scope["contexts"],strict=True):
        require(all(exact_json(frozen[k],current[k]) for k in preserved),"Frozen context contract changed")
        require(sha(ROOT/current["source"])==current["source_sha256"],"Source changed")
        require(current["complete"] is False and current["qualification_certificate"] is False,"Context claims qualification")
        if "batch_name" in current:
            require(current["batch_name"] not in by_batch,"Duplicate context batch")
            by_batch[current["batch_name"]]=current
        else:
            require(current["id"]=="valuation-missing-value" and current["required_rejection_stage"]=="parse",
                    "Non-parse context omitted")
    frozen=read("plan-spec.json")["batches"][0]["requirements"]
    key=lambda request:json.dumps(request,sort_keys=True,separators=(",",":"))
    require({key(p) for batch in plan["batches"] for p in batch["requirements"]}=={key(p) for p in frozen},
            "Dependency inventory pruned")
    summaries=[]
    with output.open(encoding="utf-8") as stream:
        for batch,line in zip(plan["batches"],stream,strict=True):
            row=json.loads(line);context=by_batch[batch["name"]]
            selected=libraries+[context["original_source_uri"]]
            require(batch["resource_uris"]==selected and len(selected)==95 and len(set(selected))==95,
                    "Actual context resource envelope changed")
            require(Path(selected[-1])==ROOT/context["source"],"Source identity differs")
            roots={root_id(path) for path in selected}
            before=[node for node in nodes if resource_id(node["id"]) in roots]
            ports=[port for port in pending if resource_id(port["owner_id"]) in roots]
            expected=[p for p in frozen if resource_id(p["owner_id"]) in roots]
            require(batch["requirements"]==expected,"Context wave changed")
            require(row["batch_name"]==batch["name"] and row["requested_requirements"]==expected,
                    "Context result identity or requirements differ")
            require(row["qualification_certificate"] is False and row["publication"]=="not_attempted"
                and row["semantic_qualification"]=="not_assessed","Context execution claims qualification")
            projection=row["resource_projection"]
            require(projection=={"status":"exact_resource_subset","original_resource_count":119,
                "selected_resource_count":95,"excluded_resource_count":24,"selected_input_files":selected,
                "properties_rewritten":False,"identities_rewritten":False},"Projection record changed")
            item={"context":context["id"],"batch":batch["name"],"status":row["status"],
                "actual_library_resources":94,"requested_dependencies":len(expected),
                "required_terminal_outcome":context["independent_outcome"],
                "required_rejection_stage":context["required_rejection_stage"],
                "explicit_mutation_applied":False,"complete_native_context":False}
            if row["status"]=="blocked":
                require("inspection" not in row and "model" not in row,"Failed context returned a partial model")
                require(row["attempted_reference_commits_discarded"]>=0,"Discarded count absent")
                failures=row.get("failed_native_reference_reads",[])
                for failure in failures:
                    expected_ports=[p for p in ports if p["owner_id"]==failure["owner_id"] and p["field"]==failure["field"]]
                    require(expected_ports and exact_json(expected_ports,failure["pending_descriptors"]),
                            "Failed read does not identify actual context ports")
                item.update(error=row["error"],discarded_commits=row["attempted_reference_commits_discarded"],
                            failed_native_reference_reads=failures)
            else:
                require(row["status"]=="dependency_inspection","Unknown context outcome")
                require(row["input_files"]==selected and row["inspection"]["linking"]=="requested_plan_completed"
                    and row["inspection"]["semantic_validation"]==row["inspection"]["transformation_completion"]=="not_assessed",
                    "Native context phase changed")
                item.update(assess_successful_wave(before,ports,row,expected))
            summaries.append(item)
    return {"schema":"dev.mercurio.context-dependency-integrity.v1","qualification_certificate":False,
        "fixed_context_count":26,"context_dependency_waves_audited":25,
        "successful_dependency_waves":sum(row["status"]=="dependency_inspection" for row in summaries),
        "complete_native_contexts":0,"strict_families_qualified":0,"candidate_promoted":False,
        "explicit_ecore_mutations_pending":2,"actual_parse_negative_pending":1,"contexts":summaries,
        "boundary":"Requested dependency execution, full semantic transformation/validation, publication, persistence and independent comparison remain separate. Every original acceptance stage is preserved; no terminal credit is awarded by this checker."}

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--check",action="store_true")
    args=parser.parse_args();report=assess();destination=BASE/(PREFIX+"context-integrity.json")
    if args.check:
        require(read("context-integrity.json")==report,"Context integrity report stale")
    else:
        destination.write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
    print(json.dumps({k:report[k] for k in ["context_dependency_waves_audited","successful_dependency_waves","complete_native_contexts","strict_families_qualified"]}))

if __name__=="__main__":
    main()
