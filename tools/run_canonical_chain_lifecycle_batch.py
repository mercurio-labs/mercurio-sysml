"""Execute and compare the complete fixed adopted-chain dependency stage."""
from pathlib import Path
import collections
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from verify_value_result_library_dependency_bundle import observe_sources
from audit_cached_native_dependencies import assess_successful_wave,exact_json
from audit_canonical_chain_lifecycle import assess
from audit_value_result_provider_plan import native_identity
from run_value_result_chain_transformation import children,owned_features

def main():
    build_path=EV/"canonical-chain-lifecycle-checks-run.json";build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"] and build["native_controls"]==539
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    binary=Path(build["cli_path"]);assert sha(binary)==build["cli_sha256"]
    previous_path=EV/"argument-result-specialization-sealed-progress.json";previous=read(previous_path)
    record=Path(previous["current_record_path"]);assert sha(record)==previous["current_record_sha256"]
    original=read(record);model=original["inspection"];index={node["id"]:node for node in model["constructed_elements"]}
    invariant_path=EV/"value-result-invocation-input-integrity.json";invariant=read(invariant_path);release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert len(original["input_files"])==119 and len(invariant["library_sha256"])==94
    assert all(sha(release/p)==h for p,h in invariant["library_sha256"].items())
    assert all(sha(EV.parent/p)==h for p,h in invariant["frozen_acceptance_sha256"].items())
    bundle_path=EV/"value-result-general-value-provider-dependency-bundle.json";bundle=read(bundle_path)
    source_map_path=EV/"value-result-lifecycle-shared-services-source-identities.json";source_map=read(source_map_path)["canonical_resource_fragment_to_native_id"]
    references=[EV/"attached-chain-lifecycle-reference-observations.json",EV/"canonical-chain-lifecycle-reference-1-observations.json",EV/"canonical-chain-lifecycle-reference-2-observations.json"]
    ref_runs=[EV/"attached-chain-lifecycle-reference-run.json",EV/"canonical-chain-lifecycle-reference-run.json",EV/"binary-cross-featuring-reference-run.json"]
    cross_featuring_path=EV/"binary-cross-featuring-reference-observations.json";references.append(cross_featuring_path)
    for path in ref_runs:
        proof=read(path);assert proof["inputs_unchanged"] and all(sha(Path(p))==h for p,h in proof["input_sha256"].items())
        observed=proof.get("observations",[proof])
        for output in observed:assert sha(Path(output["output_path"]))==output["output_sha256"]
    normative_path=EV/"binary-cross-featuring-normative-review.json"
    assert read(normative_path)["qualification_certificate"] is False
    reference_values=read(references[0])["observations"];reference_cross=read(references[1])["observations"];reference_results=read(references[2])["observations"]
    selected=[];binding_owners=[]
    for value in bundle["valuations"]:
        if not value["eligible_for_binding_stage"]:continue
        prefix=value["valuation_id"]+".implicit.value-binding";owner=index[value["valuation_id"]]["properties"]["owning_related_element"]
        if prefix not in index:continue
        binding_owners.append(owner)
        for role,suffix in [("value_chain",".source"),("initial_context",".initial-context")]:
            id=prefix+suffix
            if id in index:selected.append(dict(owner_id=id,role=role,valuation_id=value["valuation_id"],context=value["context"],binding_owner=owner))
    results_by_expression={};generated_identities={};role_proofs=[]
    for row in reference_results:
        expression=source_map[row["chain"]["resource"]+"#"+row["chain"]["emf_fragment"]]
        parameter=source_map[row["parameter"]["resource"]+"#"+row["parameter"]["emf_fragment"]]
        first_in=next(node for node in owned_features(index,index[expression]) if node["properties"].get("direction")=="in")
        assert first_in["id"]==parameter
        source=owned_features(index,index[parameter])[0]
        assert source["kind"]=="SysML::"+row["source"]["kind"]=="SysML::Feature"
        generated_identities[row["source"]["resource"]+"#"+row["source"]["emf_fragment"]]=source["id"]
        returns=[edge for edge in children(index,index[expression],"owned_relationship") if edge["kind"]=="SysML::ReturnParameterMembership"]
        assert len(returns)==1 and len(children(index,returns[0],"owned_related_element"))==1
        result=children(index,returns[0],"owned_related_element")[0]
        generated_identities[row["result"]["resource"]+"#"+row["result"]["emf_fragment"]]=result["id"]
        for shape in row["attached_result_chain_lifecycle"]:
            assert shape["direct_chain"]==[row["parameter"],row["source"]]
        role_proofs.append(dict(expression_id=expression,first_in_parameter=parameter,source_target_feature=source["id"],result_feature=result["id"],boundary="Identity matched by actual canonical roles, independently of constructed chain endpoints; no generated EMF ordinal is guessed."))
        matches=[node for node in index.values() if node["kind"]=="SysML::Feature" and node["id"].startswith(expression+".") and node["id"].endswith(".implicit.chain-specialization.result-chain")]
        assert len(matches)==1
        selected.append(dict(owner_id=matches[0]["id"],role="result_chain",expression_id=expression,context=row["control"]["context"]));results_by_expression[expression]=row
    cross_by_chain={}
    for row in reference_cross:
        owner=row["control"]["owner_id"];relations=[index[id] for id in index[owner]["properties"]["owned_relationship"] if index[id]["kind"]=="SysML::CrossSubsetting"];assert len(relations)==1
        id=relations[0]["properties"]["crossed_feature"];selected.append(dict(owner_id=id,role="cross_chain",context="actual-Links-library",crossing_owner=owner));cross_by_chain[id]=row
    selected.sort(key=lambda row:row["owner_id"]);owners=[row["owner_id"] for row in selected]
    assert len(owners)==44 and len(set(owners))==44 and dict(collections.Counter(row["role"] for row in selected))==dict(value_chain=38,initial_context=1,result_chain=4,cross_chain=1)
    folder=ROOT/"target/canonical-chain-lifecycle";folder.mkdir(exist_ok=False)
    selection_path=folder/"selection.json";save(selection_path,dict(qualification_certificate=False,receivers=selected,fixed_contexts=26,fixed_libraries=94))
    plan=folder/"requirements.json";requirements=[dict(kind="chain_lifecycle_batch",owner_ids=owners)];save(plan,dict(requirements=requirements))
    baseline_path=EV/"argument-result-specialization-native-run.json";baseline=read(baseline_path)
    spec=folder/"queries-spec.json";queries=[dict(owner_id=id,field=field) for id in owners for field in ["type","featuring_type","chaining_feature"]]
    queries += [dict(owner_id=row["owner_id"],field=row["field"]) for row in baseline["end_type_comparison"]]
    cross_featuring_reference=read(cross_featuring_path)["observations"][0]
    cross_feature_row=cross_featuring_reference["cross_specializations"][0]["specific"];cross_feature=native_identity(cross_feature_row["resource"],cross_feature_row["emf_fragment"])
    queries.append(dict(owner_id=cross_feature,field="featuring_type"))
    save(spec,dict(inspection_queries=queries));assert len(queries)==209
    inputs=observe_sources(original);inputs.update({str(ROOT/p):h for p,h in build["input_sha256"].items()})
    inputs.update({str(release/p):h for p,h in invariant["library_sha256"].items()})
    inputs.update({str(EV.parent/p):h for p,h in invariant["frozen_acceptance_sha256"].items()})
    inputs.update({str(p):sha(p) for p in [build_path,binary,previous_path,record,bundle_path,source_map_path,invariant_path,ROOT/"tools/run_value_result_chain_transformation.py",*references,*ref_runs,normative_path,selection_path,plan,spec,baseline_path,Path(__file__),ROOT/"tools/audit_canonical_chain_lifecycle.py"]})
    output=EV/"canonical-chain-lifecycle-native-run.json";assert not output.exists()
    manifest=dict(schema="dev.mercurio.canonical-chain-lifecycle-native.v1",qualification_certificate=False,status="running",input_sha256=inputs,consumer_sha256=build["input_sha256"],runs=[],receivers=selected,requested_requirements=requirements,strict_contexts_qualified=0,strict_obligations_qualified=0,strict_families_qualified=0,candidate_promoted=False);save(output,manifest)
    native=folder/"native.jsonl";code=run([str(binary),"--definition-plan-records",str(record),str(plan),str(native)],folder,"native",manifest,output)
    if code:
        manifest.update(status="native_chain_lifecycle_rejected",failure=read(native) if native.exists() else None,original_candidate_preserved=sha(record)==previous["current_record_sha256"]);save(output,manifest);raise SystemExit(code)
    after=read(native);after_index={node["id"]:node for node in after["inspection"]["constructed_elements"]}
    default_ref=next(effect["target"] for row in reference_values for binding in row.get("bindings",[]) for shape in binding["chains"] for effect in shape["effects"] if effect["kind"]=="Subsetting")
    default=native_identity(default_ref["resource"],default_ref["emf_fragment"])
    integrity=assess(model["constructed_elements"],model["pending_references"],after,owners,default);assert after["input_files"]==original["input_files"]
    query_path=folder/"queries.json";assert run([str(binary),"--definition-query-records",str(native),str(spec),str(query_path)],folder,"queries",manifest,output)==0
    answers=read(query_path)["queries"];assert len(answers)==209
    answer_map={(row["owner_id"],row["field"]):row for row in answers};comparisons=[]
    def identity(row,roles):
        if "local_role" in row:return roles[row["local_role"]]
        if row["resource"].startswith("sysml.library/"):return native_identity(row["resource"],row["emf_fragment"])
        key=row["resource"]+"#"+row["emf_fragment"]
        return generated_identities[key] if key in generated_identities else source_map[key]
    def actual(id):
        node=after_index[id];prop=node["properties"];effects=[]
        for child in prop["owned_relationship"]:
            edge=after_index[child];ep=edge["properties"];kind=edge["kind"].removeprefix("SysML::")
            assert kind in ["FeatureChaining","Subsetting"]
            effects.append(dict(kind=kind,is_implied=ep["is_implied"],target=ep["chaining_feature" if kind=="FeatureChaining" else "subsetted_feature"]))
        return dict(kind=node["kind"].removeprefix("SysML::"),is_implied_included=prop["is_implied_included"],effects=effects,**{field:[target["id"] for target in answer_map[(id,field)].get("targets",[])] for field in ["type","featuring_type","chaining_feature"]})
    for receiver in selected:
        id=receiver["owner_id"];native_shape=actual(id);all_expected=[]
        if receiver["role"] in ["value_chain","initial_context"]:
            row=next(row for row in reference_values if row["control"]["valuation_id"]==receiver["valuation_id"])
            value_chain=index[receiver["valuation_id"]+".implicit.value-binding.source"]
            endpoints=[index[child]["properties"]["chaining_feature"] for child in value_chain["properties"]["owned_relationship"]]
            roles=dict(owner=receiver["binding_owner"],value=endpoints[0],value_result=endpoints[1])
            for ordinal,binding in enumerate(row["bindings"]):
                for shape in binding["chains"]:
                    if shape["local_role"]!=receiver["role"]:continue
                    wanted=dict(kind=shape["kind"],is_implied_included=shape["is_implied_included"],effects=[dict(kind=e["kind"],is_implied=e["is_implied"],target=identity(e["target"],roles)) for e in shape["effects"]],**{field:[identity(e,roles) for e in shape[field]] for field in ["type","featuring_type","chaining_feature"]})
                    all_expected.append(dict(ordinal=ordinal,shape=wanted,exact_match=exact_json(native_shape,wanted)))
        elif receiver["role"]=="result_chain":
            row=results_by_expression[receiver["expression_id"]]
            for ordinal,shape in enumerate(row["attached_result_chain_lifecycle"]):
                endpoints=[identity(e,{}) for e in shape["direct_chain"]]
                wanted=dict(kind=shape["kind"],is_implied_included=shape["is_implied_included"],effects=[dict(kind="FeatureChaining",is_implied=False,target=e) for e in endpoints]+[dict(kind=e["kind"],is_implied=e["is_implied"],target=identity(e["target"],{})) for e in shape["specializations"]],type=[identity(e,{}) for e in shape["type"]],featuring_type=[identity(e,{}) for e in shape["featuring_type"]],chaining_feature=endpoints)
                all_expected.append(dict(ordinal=ordinal,shape=wanted,exact_match=exact_json(native_shape,wanted)))
        else:
            row=cross_by_chain[id];roles=dict(chain=id)
            endpoints=[identity(e["chaining_feature"],roles) for e in row["ordered_chain"]]
            wanted=dict(kind=row["chain_kind"],is_implied_included=row["chain_completion_after_lifecycle"],effects=[dict(kind="FeatureChaining",is_implied=False,target=e) for e in endpoints]+[dict(kind=e["kind"],is_implied=e["is_implied"],target=identity(e["general"],roles)) for e in row["chain_specializations_after_lifecycle"]],type=[identity(e,roles) for e in row["chain_type_after_lifecycle"]],featuring_type=[identity(e,roles) for e in row["chain_featuring_after_lifecycle"]],chaining_feature=endpoints)
            all_expected.append(dict(ordinal=0,shape=wanted,exact_match=exact_json(native_shape,wanted)))
        assert all_expected
        comparisons.append(dict(**receiver,actual=native_shape,expected_frames=all_expected,exact_match_every_frame=all(frame["exact_match"] for frame in all_expected)))
    end_types=[dict(owner_id=row["owner_id"],exact_match=answer["status"]=="query_evaluated" and all([t["id"] for t in answer["targets"]]==want for want in row["expected_targets_by_ordinal"])) for row,answer in zip(baseline["end_type_comparison"],answers[132:208],strict=True)]
    cross_featuring_expected=[identity(e,{}) for e in cross_featuring_reference["cross_featuring_after_insertion"]]
    cross_featuring_answer=answers[208];cross_featuring_comparison=dict(owner_id=cross_feature,field="featuring_type",native_status=cross_featuring_answer["status"],actual=[e["id"] for e in cross_featuring_answer.get("targets",[])],expected=cross_featuring_expected)
    cross_featuring_comparison["exact_match"]=cross_featuring_answer["status"]=="query_evaluated" and cross_featuring_comparison["actual"]==cross_featuring_expected
    replay=folder/"replay.jsonl";assert run([str(binary),"--definition-plan-records",str(native),str(plan),str(replay)],folder,"replay",manifest,output)==0
    again=read(replay);assert exact_json(after["inspection"],again["inspection"]);replay_integrity=assess_successful_wave(after["inspection"]["constructed_elements"],after["inspection"]["pending_references"],again,[])
    fresh=folder/"fresh-queries.json";assert run([str(binary),"--definition-query-records",str(replay),str(spec),str(fresh)],folder,"fresh-queries",manifest,output)==0;assert exact_json(read(fresh)["queries"],answers)
    manifest.update(status="canonical_chain_lifecycle_compared",integrity=integrity,replay_integrity=replay_integrity,generated_source_identity_role_proofs=role_proofs,independently_compared_chain_fields=["kind","is_implied_included","ordered_effects","type","featuring_type","chaining_feature"],independent_reference_frames=sum(len(row["expected_frames"]) for row in comparisons),independently_matched_chains=sum(row["exact_match_every_frame"] for row in comparisons),required_chains=44,chain_comparison=comparisons,independently_matched_end_types=sum(row["exact_match"] for row in end_types),required_end_types=76,end_type_comparison=end_types,query_outcomes=dict(collections.Counter(row["status"] for row in answers)),binary_cross_featuring_comparison=cross_featuring_comparison,normative_cross_featuring_formal_discrepancy="binary-cross-featuring-normative-review.json",current_record_path=str(native),current_record_sha256=sha(native),current_query_path=str(query_path),current_query_sha256=sha(query_path),replay_path=str(replay),replay_sha256=sha(replay),fresh_query_path=str(fresh),fresh_query_sha256=sha(fresh),fresh_queries_exact=True,graph_unchanged_on_replay=True,original_candidate_preserved=sha(record)==previous["current_record_sha256"],boundary="Complete transformation of the exact admitted chain wrappers; enclosing Feature/Step/Expression, bounds, validation, duplicate connectors, publication and strict qualification remain open.")
    save(output,manifest);print("Canonical chains matched",manifest["independently_matched_chains"],"/44; end types",manifest["independently_matched_end_types"],"/76; strict families 0/34",flush=True)

if __name__=="__main__":main()
