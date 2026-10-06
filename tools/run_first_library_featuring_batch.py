"""Execute the entire fixed first-library featuring dependency bundle."""
from pathlib import Path
import collections
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from verify_value_result_library_dependency_bundle import observe_sources
from audit_cached_native_dependencies import assess_successful_wave,exact_json
from audit_value_result_provider_plan import native_identity

def main():
    folder=ROOT/"target/chain-first-library-featuring"
    build_path=EV/"first-library-featuring-checks-run.json";build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"] and build["native_controls"]==542
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    binary=Path(build["cli_path"]);assert sha(binary)==build["cli_sha256"]
    previous_path=EV/"canonical-chain-lifecycle-open-progress.json";previous=read(previous_path)
    record=Path(previous["current_record_path"]);assert sha(record)==previous["current_record_sha256"]
    original=read(record);before=original["inspection"];ix={n["id"]:n for n in before["constructed_elements"]}
    old_path=EV/"canonical-chain-lifecycle-native-run.json";old=read(old_path)
    contract_path=folder/"reference-spec.json";contract=read(contract_path)
    assert len(contract["first_feature_inventory"])==44 and len(contract["library_features"])==2
    ref_proof_path=EV/"first-library-featuring-reference-run.json";ref_proof=read(ref_proof_path)
    assert ref_proof["status"]=="first_library_featuring_observed" and ref_proof["inputs_unchanged"]
    assert all(sha(Path(p))==h for p,h in ref_proof["input_sha256"].items())
    ref_path=Path(ref_proof["output_path"]);assert sha(ref_path)==ref_proof["output_sha256"];ref=read(ref_path)
    receivers=[row["owner_id"] for row in contract["library_features"]];assert receivers==sorted(set(receivers))
    wrappers=[row["owner_id"] for row in old["receivers"]];assert len(wrappers)==44
    requirements=[dict(kind="owning_type_featuring_batch",owner_ids=receivers),dict(kind="chain_lifecycle_batch",owner_ids=wrappers)]
    plan=folder/"requirements.json";assert not plan.exists();save(plan,dict(requirements=requirements))
    old_spec=ROOT/"target/canonical-chain-lifecycle/queries-spec.json";queries=read(old_spec)["inspection_queries"]
    queries += [dict(owner_id=id,field=field) for id in receivers for field in ["owned_type_featuring","featuring_type"]]
    spec=folder/"queries-spec.json";save(spec,dict(inspection_queries=queries));assert len(queries)==213
    integrity_path=EV/"value-result-invocation-input-integrity.json";fixed=read(integrity_path);release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert len(original["input_files"])==119 and len(fixed["library_sha256"])==94
    assert all(sha(release/p)==h for p,h in fixed["library_sha256"].items())
    assert all(sha(EV.parent/p)==h for p,h in fixed["frozen_acceptance_sha256"].items())
    inputs=observe_sources(original);inputs.update({str(ROOT/p):h for p,h in build["input_sha256"].items()})
    inputs.update({str(release/p):h for p,h in fixed["library_sha256"].items()});inputs.update({str(EV.parent/p):h for p,h in fixed["frozen_acceptance_sha256"].items()})
    inputs.update({str(p):sha(p) for p in [build_path,binary,previous_path,record,old_path,contract_path,ref_proof_path,ref_path,old_spec,plan,spec,integrity_path,Path(__file__),ROOT/"tools/audit_cached_native_dependencies.py",ROOT/"target/r44-before/witnesses.json"]})
    output=EV/"first-library-featuring-native-run.json";assert not output.exists()
    manifest=dict(schema="dev.mercurio.first-library-featuring-native.v1",qualification_certificate=False,status="running",input_sha256=inputs,consumer_sha256=build["input_sha256"],runs=[],requested_requirements=requirements,strict_contexts_qualified=0,strict_obligations_qualified=0,strict_families_qualified=0,candidate_promoted=False);save(output,manifest)
    native=folder/"native.jsonl";code=run([str(binary),"--definition-plan-records",str(record),str(plan),str(native)],folder,"native",manifest,output)
    if code:
        manifest.update(status="native_first_library_featuring_rejected",failure=read(native) if native.exists() else None);save(output,manifest);raise SystemExit(code)
    after=read(native);graph=after["inspection"]["constructed_elements"];index={n["id"]:n for n in graph}
    integrity=assess_successful_wave(before["constructed_elements"],before["pending_references"],after,requirements)
    assert integrity["native_nodes_added"]==2 and integrity["native_ports_committed"]==0
    assert after["input_files"]==original["input_files"] and len(graph)==94838
    query_path=folder/"queries.json";assert run([str(binary),"--definition-query-records",str(native),str(spec),str(query_path)],folder,"queries",manifest,output)==0
    answers=read(query_path)["queries"];assert len(answers)==213 and all(r["status"]=="query_evaluated" for r in answers)
    answer_map={(r["owner_id"],r["field"]):r for r in answers}
    assert exact_json(answers[:209],read(Path(previous["current_query_path"]))["queries"])
    def target(identity,roles=None):
        if "local_role" in identity:return roles[identity["local_role"]]
        assert identity["resource"].startswith("sysml.library/");return native_identity(identity["resource"],identity["emf_fragment"])
    features=[]
    for row in ref["features"]:
        id=row["control"]["owner_id"];shape=row["after_insertion"]
        actual_relations=[index[e] for e in index[id]["properties"]["owned_relationship"] if index[e]["kind"]=="SysML::TypeFeaturing"]
        assert len(actual_relations)==len(shape["owned_type_featuring"])==1
        actual_effects=[]
        for relation in actual_relations:
            props=relation["properties"];actual_effects.append(dict(kind=relation["kind"].removeprefix("SysML::"),**{field:props[field] for field in ["is_implied","is_implied_included","feature_of_type","featuring_type","owning_related_element","owned_related_element"]}))
        expected_effects=[dict(kind=e["kind"],is_implied=e["is_implied"],is_implied_included=e["is_implied_included"],**{field:target(e[field]) for field in ["feature_of_type","featuring_type","owning_related_element"]},owned_related_element=[target(t) for t in e["owned_related_element"]]) for e in shape["owned_type_featuring"]]
        actual_types=[t["id"] for t in answer_map[(id,"featuring_type")]["targets"]];expected_types=[target(t) for t in shape["featuring_type"]]
        actual_owned=[t["id"] for t in answer_map[(id,"owned_type_featuring")]["targets"]]
        matched=exact_json(actual_effects,expected_effects) and actual_types==expected_types and actual_owned==[e["id"] for e in actual_relations] and index[id]["properties"]["is_implied_included"]==shape["is_implied_included"]
        assert matched
        features.append(dict(owner_id=id,actual_effects=actual_effects,expected_effects=expected_effects,actual_featuring=actual_types,expected_featuring=expected_types,physical_and_getter_match=matched))
    def actual_chain(id):
        node=index[id];props=node["properties"];effects=[]
        for child in props["owned_relationship"]:
            edge=index[child];ep=edge["properties"];kind=edge["kind"].removeprefix("SysML::")
            assert kind in ["FeatureChaining","Subsetting"]
            effects.append(dict(kind=kind,is_implied=ep["is_implied"],target=ep["chaining_feature" if kind=="FeatureChaining" else "subsetted_feature"]))
        return dict(kind=node["kind"].removeprefix("SysML::"),is_implied_included=props["is_implied_included"],effects=effects,**{field:[t["id"] for t in answer_map[(id,field)]["targets"]] for field in ["type","featuring_type","chaining_feature"]})
    comparisons=[];replaced_frames=0
    for row in old["chain_comparison"]:
        actual=actual_chain(row["owner_id"]);frames=row["expected_frames"]
        dependency_frames=[x for x in ref["wrappers"] if x["context"]==row["context"] and x["role"]==row["role"]]
        raw=frames
        if dependency_frames:
            assert row["role"] in ["initial_context","cross_chain"] and len(dependency_frames)==len(frames)
            roles={}
            if row["role"]=="initial_context":
                value=index[row["valuation_id"]+".implicit.value-binding.source"]
                parts=[index[i]["properties"]["chaining_feature"] for i in value["properties"]["owned_relationship"] if index[i]["kind"]=="SysML::FeatureChaining"]
                roles=dict(owner=row["binding_owner"],value=parts[0],value_result=parts[1])
            frames=[]
            for f in dependency_frames:
                s=f["shape"];expected=dict(kind=s["kind"],is_implied_included=s["is_implied_included"],effects=[dict(kind=e["kind"],is_implied=e["is_implied"],target=target(e["target"],roles)) for e in s["effects"]],**{field:[target(t,roles) for t in s[field]] for field in ["type","featuring_type","chaining_feature"]})
                frames.append(dict(ordinal=f["ordinal"],shape=expected));replaced_frames+=1
        matching=all(exact_json(actual,f["shape"]) for f in frames)
        assert matching
        comparisons.append(dict(owner_id=row["owner_id"],context=row["context"],role=row["role"],actual=actual,expected_frames=frames,exact_match_every_same_stage_frame=matching,retained_raw_frames=raw,exact_match_every_raw_frame=all(exact_json(actual,f["shape"]) for f in raw)))
    assert replaced_frames==3 and sum(len(r["expected_frames"]) for r in comparisons)==83 and len(comparisons)==44
    end_types=old["end_type_comparison"];assert all(r["exact_match"] for r in end_types)
    replay=folder/"replay.jsonl";assert run([str(binary),"--definition-plan-records",str(native),str(plan),str(replay)],folder,"replay",manifest,output)==0
    again=read(replay);assert exact_json(after["inspection"],again["inspection"]);replay_integrity=assess_successful_wave(graph,after["inspection"]["pending_references"],again,requirements)
    assert replay_integrity["native_nodes_added"]==replay_integrity["native_ports_committed"]==0
    fresh=folder/"fresh-queries.json";assert run([str(binary),"--definition-query-records",str(replay),str(spec),str(fresh)],folder,"fresh-queries",manifest,output)==0
    assert exact_json(read(fresh)["queries"],answers)
    manifest.update(status="same_stage_first_library_featuring_verified",integrity=integrity,replay_integrity=replay_integrity,features=features,required_features=2,physical_features_matching=2,chain_comparison=comparisons,required_chains=44,same_stage_chains_matching=44,required_reference_frames=83,same_stage_reference_frames_matching=83,original_raw_chains_matching=42,original_raw_reference_frames_matching=80,retained_end_types_matching=76,query_outcomes=dict(collections.Counter(r["status"] for r in answers)),fresh_queries_exact=True,constructor_replay_exact=True,previous_candidate_preserved=sha(record)==previous["current_record_sha256"],current_record_path=str(native),current_record_sha256=sha(native),current_query_path=str(query_path),current_query_sha256=sha(query_path),replay_path=str(replay),replay_sha256=sha(replay),fresh_query_path=str(fresh),fresh_query_sha256=sha(fresh),remaining="Enclosing Feature/Step/Expression, bounds, applicable validation, duplicate connector discrepancy, Closed publication and full context qualification remain open. Owned-cross formal discrepancy is unchanged.")
    save(output,manifest);print("Same-stage wrapper projections:44/44,83/83 frames; physical library producers:2/2; strict families:0/34",flush=True)

if __name__=="__main__":main()
