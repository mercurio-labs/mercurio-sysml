"""Execute the fixed general value-binding dependency bundle atomically.

Full V01 acceptance remains unchanged. Stage construction is independently
audited here; it is not a lifecycle, validation or family certificate.
"""
from pathlib import Path
import argparse,collections,json
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from verify_value_result_library_dependency_bundle import observe_sources
from audit_cached_native_dependencies import assess_successful_wave,exact_json

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--build-revision",required=True);args=parser.parse_args()
    sealed_path=EV/"canonical-chain-result-resolved-sealed-progress.json";sealed=read(sealed_path)
    assert sealed["status"]=="bounded_canonical_chain_result_consumer_stage_sealed"
    record=Path(sealed["current_record_path"]);assert sha(record)==sealed["current_record_sha256"]
    build_path=EV/("general-value-bindings-"+args.build_revision+"-build-run.json");build=read(build_path);assert build["status"]=="passed" and build["inputs_unchanged"]
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    bundle_path=EV/"value-result-general-value-provider-dependency-bundle.json";bundle=read(bundle_path)
    contract_path=EV/"general-value-bindings-batch-contract.json";contract=read(contract_path)
    assert sha(bundle_path)==contract["source_bundle_sha256"]
    valuations=bundle["valuations"];assert len(valuations)==45
    eligible=[v for v in valuations if v["eligible_for_binding_stage"]];assert len(eligible)==41
    assert sum(v["is_default"] for v in eligible)==3 and sum(v["is_initial"] and not v["is_default"] for v in eligible)==1
    ordinary=[v for v in eligible if not v["is_default"] and not v["is_initial"]];assert len(ordinary)==37
    receivers=sorted(set(v["owner_id"] for v in eligible))
    roots=[dict(kind="general_value_binding_batch",owner_ids=receivers)]
    original=read(record);inspection=original["inspection"]
    original_index={n["id"]:n for n in inspection["constructed_elements"]}
    folder=ROOT/("target/general-value-bindings-"+args.revision);folder.mkdir(exist_ok=False)
    plan=folder/"requirements.json";save(plan,dict(requirements=roots))
    output=EV/("general-value-bindings-"+args.revision+"-native-run.json");assert not output.exists()
    inputs=observe_sources(original);inputs.update({str(ROOT/p):h for p,h in build["input_sha256"].items()})
    inputs.update({str(p):sha(p) for p in [sealed_path,record,build_path,binary,bundle_path,contract_path,plan,Path(__file__),ROOT/"tools/audit_cached_native_dependencies.py"]})
    manifest=dict(schema="dev.mercurio.general-value-bindings-native-batch.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],
        consumer_sha256=build["input_sha256"],required_valuations=45,eligible_valuations=41,ordinary_bindings=37,initial_bindings=1,default_no_bindings=3,
        fixed_contexts=26,fixed_resources=119,fixed_libraries=94,requested_requirements=roots,strict_contexts_qualified=0,strict_obligations_qualified=0,strict_families_qualified=0,candidate_promoted=False)
    save(output,manifest);native=folder/"transformed.jsonl"
    code=run([str(binary),"--definition-plan-records",str(record),str(plan),str(native)],folder,"native",manifest,output)
    if code:
        failure=read(native) if native.exists() else None
        manifest.update(status="native_value_binding_batch_rejected",failure=failure,original_candidate_preserved=sha(record)==sealed["current_record_sha256"])
        save(output,manifest);print("General value-binding batch rejected; atomic failure recorded, strict families 0/34",flush=True);raise SystemExit(code)
    current=read(native)
    integrity=assess_successful_wave(inspection["constructed_elements"],inspection["pending_references"],current,roots)
    assert current["input_files"]==original["input_files"] and integrity["native_nodes_added"]>0
    index={n["id"]:n for n in current["inspection"]["constructed_elements"]}
    physical=[];queries=[]
    for value in valuations:
        identity=value["valuation_id"]+".implicit.value-binding"
        required=value["eligible_for_binding_stage"] and not value["is_default"]
        assert (identity in index)==required
        if not required:continue
        binding=index[identity];assert binding["kind"]=="SysML::BindingConnector" and binding["properties"]["is_implied"] and binding["properties"]["is_implied_included"]
        chain=index[identity+".source"];rels=[index[x] for x in chain["properties"]["owned_relationship"]]
        assert len(rels)==2 and all(n["kind"]=="SysML::FeatureChaining" for n in rels)
        endpoints=[n["properties"]["chaining_feature"] for n in rels];assert endpoints[0]==value["expression_id"]
        assert index[identity+".end.1.reference"]["properties"]["referenced_feature"]==value["owner_id"]
        assert index[identity+".end.0.reference"]["properties"]["referenced_feature"]==chain["id"]
        assert chain["properties"]["owning_relationship"]==identity+".end.0.reference"
        assert binding["properties"]["owning_relationship"]==identity+".membership"
        assert index[identity+".membership"]["properties"]["owning_related_element"]==value["owner_id"]
        physical.append(dict(context=value["context"],valuation_id=value["valuation_id"],binding_id=identity,source_chain_endpoints=endpoints,initial=value["is_initial"]))
        queries.extend(dict(owner_id=identity,field=field) for field in ["source","target","related_feature","featuring_type"])
    assert len(physical)==38
    manifest.update(status="native_value_binding_batch_constructed",integrity=integrity,physical_bindings=physical,current_record_path=str(native),current_record_sha256=sha(native));save(output,manifest)
    spec=folder/"queries-spec.json";save(spec,dict(inspection_queries=queries))
    query_path=folder/"queries.json";code=run([str(binary),"--definition-query-records",str(native),str(spec),str(query_path)],folder,"queries",manifest,output);assert code==0
    answers=read(query_path)["queries"];assert len(answers)==152 and all(row["status"]=="query_evaluated" for row in answers)
    replay=folder/"replay.jsonl";code=run([str(binary),"--definition-plan-records",str(native),str(plan),str(replay)],folder,"replay",manifest,output);assert code==0
    restored=read(replay);replay_integrity=assess_successful_wave(current["inspection"]["constructed_elements"],current["inspection"]["pending_references"],restored,[])
    assert replay_integrity["native_nodes_added"]==replay_integrity["native_ports_committed"]==0
    assert exact_json(restored["inspection"]["constructed_elements"],current["inspection"]["constructed_elements"])
    assert restored["inspection"]["pending_references"]==current["inspection"]["pending_references"]
    fresh=folder/"fresh-queries.json";code=run([str(binary),"--definition-query-records",str(replay),str(spec),str(fresh)],folder,"fresh-queries",manifest,output);assert code==0
    assert exact_json(read(fresh)["queries"],answers)
    manifest.update(status="native_value_binding_stage_verified_reference_comparison_required",bindings_constructed=38,default_dispatches_without_binding=3,
        focused_query_outcomes=dict(collections.Counter(row["status"] for row in answers)),query_path=str(query_path),query_sha256=sha(query_path),replay_path=str(replay),replay_sha256=sha(replay),
        replay_integrity=replay_integrity,graph_unchanged_on_replay=True,fresh_queries_exact=True,fresh_query_path=str(fresh),fresh_query_sha256=sha(fresh),
        original_candidate_preserved=sha(record)==sealed["current_record_sha256"],
        remaining_dependencies=["Independent same-stage Pilot comparison with explicit normative chain/fallback disagreements","Bound specialization and complete enclosing Feature/Expression lifecycle","Both source-negative scopes, both original Ecore mutations and every applicable validator","Complete terminal publication/persistence and strict qualification gates"],
        boundary="This audits the indicated native value-binding construction stage across the unchanged bundle. No terminal context, obligation, family or release gate is qualified.")
    save(output,manifest);print("38 native value bindings, 3 defaults, 152 public getters, exact fresh replay; independent comparison pending; strict families 0/34",flush=True)

if __name__=="__main__":main()
