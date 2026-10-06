"""Execute the fixed reference-binding batch and compare all canonical fresh roles."""
from pathlib import Path
import json,hashlib,subprocess,time,collections,argparse
from audit_cached_native_dependencies import assess_successful_wave
from audit_value_result_provider_plan import native_identity
ROOT=Path(__file__).resolve().parents[1]
EV=ROOT/"docs/conformance/2026-08-support/definition-pipeline-evidence"
def read(p):return json.loads(Path(p).read_text(encoding="utf-8"))
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2)+"\n",encoding="utf-8")
def compare(graph,reference,identities,prior_queries):
    index={node["id"]:node for node in graph};queries={(row["owner_id"],row["field"]):row for row in prior_queries["queries"]};rows=[]
    for observed in reference["observations"]:
        control=observed["control"];owner=index[control["owner_id"]]
        if control["link_negative"]:
            assert observed["status"]=="link_negative_not_qualified"
            rows.append(dict(context=control["context"],owner_id=owner["id"],status="link_negative_not_qualified"));continue
        assert observed["status"]=="reference_observed" and len(observed["bindings"])==1
        result_query=queries[(owner["id"],"result")]
        assert result_query["status"]=="query_evaluated" and len(result_query["targets"])==1
        result_id=result_query["targets"][0]["id"];result=index[result_id]
        member=index[result["properties"]["owning_relationship"]]
        assert member["kind"].rsplit("::",1)[-1]=="ReturnParameterMembership"
        assert member["properties"].get("owned_related_element")==[result_id]
        assert member["properties"]["owning_related_element"]==owner["id"]
        assert member["id"] in owner["properties"]["owned_relationship"]
        assert result["properties"]["direction"]=="out"
        binding_id=owner["id"]+".implicit.reference-binding"
        roles={"connector":binding_id,"end.0":binding_id+".end.0","end.1":binding_id+".end.1"}
        def endpoint(value):
            if "local_role" in value:return roles[value["local_role"]]
            if value==observed["result"]:return result_id
            resource=value["resource"];fragment=value["emf_fragment"]
            if resource.startswith("sysml.library/"):return native_identity(resource,fragment)
            key=resource.replace("\\","/")+"#"+fragment
            assert key in identities,"Unreviewed external endpoint "+key
            return identities[key]
        def expected(node):
            return dict(kind=node["kind"],is_implied_included=node["is_implied_included"],is_end=node["is_end"],
                effects=[dict(kind=effect["kind"],is_implied=effect["is_implied"],target_id=endpoint(effect["target"])) for effect in node["effects"]])
        def actual(node_id):
            node=index[node_id];effects=[]
            fields={"ReferenceSubsetting":"referenced_feature","Subsetting":"subsetted_feature","Redefinition":"redefined_feature","FeatureTyping":"type","TypeFeaturing":"featuring_type"}
            for relation_id in node["properties"].get("owned_relationship",[]):
                relation=index[relation_id];kind=relation["kind"].rsplit("::",1)[-1]
                if kind.endswith("Membership"):continue
                assert kind in fields,"Unassessed fresh binding relationship "+kind
                assert relation["properties"]["owning_related_element"]==node_id
                target=relation["properties"][fields[kind]];assert target in index
                effects.append(dict(kind=kind,is_implied=relation["properties"].get("is_implied",False),target_id=target))
            return dict(kind=node["kind"].rsplit("::",1)[-1],is_implied_included=node["properties"].get("is_implied_included",False),
                is_end=node["properties"].get("is_end",False),effects=effects)
        binder=index[binding_id];placement=index[binder["properties"]["owning_relationship"]]
        assert placement["properties"]["owned_related_element"]==[binding_id]
        assert placement["properties"]["owning_related_element"]==owner["id"]
        expected_binding=observed["bindings"][0]
        wanted=dict(membership=expected_binding["membership"],connector=expected(expected_binding),
            ends=[expected(end) for end in expected_binding["ends"]])
        got=dict(membership=placement["kind"].rsplit("::",1)[-1],connector=actual(binding_id),
            ends=[actual(roles["end."+str(i)]) for i in range(2)])
        flags=dict(native=owner["properties"].get("is_implied_included",False),reference_before=observed["completion_before"],reference_after=observed["completion_after"])
        rows.append(dict(context=control["context"],owner_id=owner["id"],status="stage_compared",actual=got,expected=wanted,
            fresh_result_identity_proof=dict(native_id=result_id,upstream=observed["result"],canonical_return_membership=member["id"],source="same-owner result query and canonical out ReturnParameterMembership"),
            original_owner_flags=flags,exact_ordered_match=got==wanted and not any(flags.values())))
    return rows
def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--build-revision",required=True);parser.add_argument("--reference-stage",choices=["context","cold"],default="context");args=parser.parse_args()
    build_path=EV/("value-result-reference-binding-"+args.build_revision+"-build-run.json");build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"]
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    spec_path=EV/("value-result-reference-binding-context-spec.json" if args.reference_stage=="context" else "value-result-reference-binding-batch-spec.json");spec=read(spec_path)
    record=Path(spec["retained_record_path"]);assert sha(record)==spec["retained_record_sha256"]
    roots=spec["native_roots"];assert len(roots)==1 and roots[0]["kind"]=="reference_binding_batch" and len(roots[0]["owner_ids"])==15
    assert spec["required_expressions"]==16 and spec["fixed_contexts"]==26
    ref_run_path=EV/("value-result-reference-binding-context-reference-resolved-run.json" if args.reference_stage=="context" else "value-result-reference-binding-resolved-reference-roles-run.json");ref_run=read(ref_run_path)
    assert ref_run["status"]=="reference_observed" and ref_run["inputs_unchanged"]
    reference=Path(ref_run["output_path"]);assert sha(reference)==ref_run["output_sha256"]
    assert all(sha(p)==h for p,h in ref_run["input_sha256"].items())
    identities=EV/"value-result-lifecycle-shared-services-source-identities.json"
    checkpoint=EV/"value-result-expression-contribution-progress.json";old=read(checkpoint);queries=Path(old["current_query_path"])
    inventory=read(EV/"value-result-invocation-input-integrity.json")
    base=EV.parent;release=ROOT.parent/"target/upstream/SysML-v2-Release"
    for p,h in inventory["frozen_acceptance_sha256"].items():assert sha(base/p)==h
    for p,h in inventory["library_sha256"].items():assert sha(release/p)==h
    folder=ROOT/("target/value-result-reference-binding-"+args.revision);folder.mkdir(exist_ok=False)
    plan=folder/"requirements.json";save(plan,dict(requirements=roots));target=folder/"transformed.jsonl";log=folder/"transformation.log"
    out=EV/("value-result-reference-binding-"+args.revision+"-execution-run.json");assert not out.exists()
    files=[plan,binary,build_path,spec_path,record,ref_run_path,reference,identities,checkpoint,queries,Path(__file__)]
    witness={str(p):sha(p) for p in files}
    command=[str(binary),"--definition-plan-records",str(record),str(plan),str(target)]
    manifest=dict(schema="dev.mercurio.reference-binding-execution.v1",qualification_certificate=False,status="running",
        input_sha256=witness,consumer_sha256=build["input_sha256"],required_expressions=16,eligible_expressions=15,required_link_negative_expressions=1,
        reference_stage=args.reference_stage,fixed_contexts=26,fixed_libraries=94,complete_native_contexts=0,strict_families_qualified=0,candidate_promoted=False,command=command)
    save(out,manifest);start=time.monotonic()
    with log.open("w",encoding="utf-8") as handle:code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
    manifest.update(exit_code=code,elapsed_seconds=round(time.monotonic()-start,3),log_path=str(log),log_sha256=sha(log),
        output_path=str(target),output_sha256=sha(target) if target.exists() else None,inputs_unchanged=all(sha(p)==h for p,h in witness.items()))
    if code:
        manifest.update(status="native_bundle_rejected",failure=read(target) if target.exists() else log.read_text(encoding="utf-8")[-5000:])
        save(out,manifest);print("Native fixed bundle rejected:",str(manifest["failure"])[:5500],flush=True);raise SystemExit(code)
    native=read(target)
    prior=read(record)["inspection"]
    manifest["integrity"]=assess_successful_wave(prior["constructed_elements"],prior["pending_references"],native,roots)
    rows=compare(native["inspection"]["constructed_elements"],read(reference),read(identities)["canonical_resource_fragment_to_native_id"],read(queries))
    comparison=EV/("value-result-reference-binding-"+args.revision+"-structural-comparison.json")
    save(comparison,dict(schema="dev.mercurio.reference-binding-structural-comparison.v1",qualification_certificate=False,
        required_expressions=16,expressions=rows,enclosing_transform_validation_publication="not_assessed"))
    matched=sum(row.get("exact_ordered_match",False) for row in rows);assert len(rows)==16
    manifest.update(status="physical_bundle_compared",expressions_matched=matched,current_record_path=str(target),current_record_sha256=sha(target),
        comparison_path=str(comparison),comparison_sha256=sha(comparison))
    save(out,manifest)
    print("Reference bindings matched",matched,"/15;",manifest["integrity"],flush=True)
    assert manifest["inputs_unchanged"]
    if matched!=15:raise SystemExit(2)
if __name__=="__main__":main()
