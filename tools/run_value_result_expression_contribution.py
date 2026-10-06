"""Execute and compare the shared expression-contribution stage on fixed sources."""
from pathlib import Path
import collections,json,hashlib,subprocess,time
from audit_cached_native_dependencies import assess_successful_wave
from audit_value_result_provider_plan import native_identity
ROOT=Path(__file__).resolve().parents[1]
EV=ROOT/"docs/conformance/2026-08-support/definition-pipeline-evidence"
def read(p):return json.loads(Path(p).read_text(encoding="utf-8"))
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2)+"\n",encoding="utf-8")
def compare(graph,reference,identities):
    index={n["id"]:n for n in graph}; rows=[]
    for row in reference["observations"]:
        control=row["control"];owner=index[control["owner_id"]]
        if control["link_negative"]:
            assert row["status"]=="link_negative_not_qualified"
            rows.append(dict(context=control["context"],owner_id=owner["id"],owner_kind=owner["kind"],
                             status="link_negative_not_qualified",required_link_rejection="not_assessed"))
            continue
        assert row["status"]=="reference_observed"
        expected=[]
        for effect in row["effects"]:
            target=effect["target"];resource=target["resource"];fragment=target["emf_fragment"]
            if resource.startswith("sysml.library/"):
                endpoint=native_identity(resource,fragment)
            else:
                key=resource.replace("\\","/")+"#"+fragment
                assert key in identities,"Unreviewed reference endpoint "+key
                endpoint=identities[key]
            expected.append(dict(kind=effect["kind"],target_id=endpoint))
        actual=[]
        for kind,specific,general in [("FeatureTyping","typed_feature","type"),("Subsetting","subsetting_feature","subsetted_feature")]:
            for relation_id in owner["properties"].get("owned_relationship",[]):
                relation=index[relation_id];props=relation["properties"]
                if relation["kind"].rsplit("::",1)[-1]==kind and props.get("is_implied") is True:
                    assert props[specific]==owner["id"]
                    endpoint=props[general];assert endpoint in index
                    actual.append(dict(kind=kind,target_id=endpoint))
        flags={"native":owner["properties"].get("is_implied_included",False),
               "reference_before":row["completion_before"],"reference_after":row["completion_after"]}
        rows.append(dict(context=control["context"],owner_id=owner["id"],owner_kind=owner["kind"],
                         actual=actual,expected=expected,completion_flags=flags,
                         exact_ordered_match=actual==expected and not any(flags.values()),
                         status="stage_compared"))
    return rows
def main():
    build_path=EV/"value-result-expression-contribution-grouped-build-run.json";build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"]
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    spec_path=EV/"value-result-expression-contribution-grouped-spec.json";spec=read(spec_path)
    record=Path(spec["retained_record_path"]);assert sha(record)==spec["retained_record_sha256"]
    requirements=spec["native_roots"];assert len(requirements)==1
    assert requirements[0]["kind"]=="expression_contribution_batch" and len(requirements[0]["owner_ids"])==52
    assert spec["required_expressions"]==56 and spec["required_kinds"]==8
    ref_run_path=EV/"value-result-expression-contribution-resolved-reference-run.json";ref_run=read(ref_run_path)
    assert ref_run["status"]=="reference_observed" and ref_run["inputs_unchanged"]
    ref_path=Path(ref_run["output_path"]);assert sha(ref_path)==ref_run["output_sha256"]
    assert all(sha(p)==h for p,h in ref_run["input_sha256"].items())
    identity_path=EV/"value-result-lifecycle-shared-services-source-identities.json"
    folder=ROOT/"target/value-result-expression-contribution";folder.mkdir(exist_ok=False)
    plan=folder/"plan.json";save(plan,dict(requirements=requirements))
    out=EV/"value-result-expression-contribution-execution-run.json";assert not out.exists()
    witness={str(p):sha(p) for p in [record,spec_path,ref_path,ref_run_path,binary,identity_path,plan,Path(__file__)]}
    manifest=dict(schema="dev.mercurio.expression-contribution-execution.v1",qualification_certificate=False,
                  status="running",input_sha256=witness,consumer_sha256=build["input_sha256"],
                  required_expressions=56,eligible_expressions=52,required_link_negative_expressions=4,
                  required_kinds=8,fixed_contexts=26,fixed_queries=4462,
                  complete_native_contexts=0,strict_families_qualified=0,candidate_promoted=False)
    save(out,manifest)
    target=folder/"transformed.jsonl";log=folder/"transformation.log"
    command=[str(binary),"--definition-plan-records",str(record),str(plan),str(target)]
    start=time.monotonic()
    with log.open("w",encoding="utf-8") as handle:
        code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
    manifest.update(command=command,exit_code=code,elapsed_seconds=round(time.monotonic()-start,3),
                    log_path=str(log),log_sha256=sha(log),output_path=str(target),
                    output_sha256=sha(target) if target.exists() else None)
    save(out,manifest);print("native expression bundle exit",code,manifest["elapsed_seconds"],flush=True)
    if code:
        manifest.update(status="native_bundle_rejected",failure=read(target) if target.exists() else log.read_text(encoding="utf-8")[-2000:],
                        inputs_unchanged=all(sha(p)==h for p,h in witness.items()))
        save(out,manifest);print(str(manifest["failure"])[:3000],flush=True);raise SystemExit(code)
    prior=read(record)["inspection"];native=read(target)
    manifest["integrity"]=assess_successful_wave(prior["constructed_elements"],prior["pending_references"],native,requirements)
    identities=read(identity_path)["canonical_resource_fragment_to_native_id"]
    rows=compare(native["inspection"]["constructed_elements"],read(ref_path),identities)
    assert len(rows)==56 and sum(row["status"]=="stage_compared" for row in rows)==52
    comparison_path=EV/"value-result-expression-contribution-structural-comparison.json"
    save(comparison_path,dict(schema="dev.mercurio.expression-contribution-comparison.v1",qualification_certificate=False,
                             required_expressions=56,expressions=rows,
                             full_transform_validation_publication="not_assessed",
                             required_link_negative_expressions="not_qualified"))
    manifest.update(status="physical_bundle_compared",expressions_matched=sum(row.get("exact_ordered_match",False) for row in rows),
                    current_record_path=str(target),current_record_sha256=sha(target),
                    comparison_path=str(comparison_path),comparison_sha256=sha(comparison_path),
                    inputs_unchanged=all(sha(p)==h for p,h in witness.items()))
    save(out,manifest);print(manifest["integrity"],"matched",manifest["expressions_matched"],"/52",flush=True)
    assert manifest["inputs_unchanged"] and manifest["expressions_matched"]==52
if __name__=="__main__":main()
