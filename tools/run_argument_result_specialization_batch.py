"""Apply reviewed rules to every applicable receiver in the fixed V01 sources.

Native delegates and the scheduler choose all dependencies. No Pilot target is
supplied to construction. Comparison uses the retained independent type frames.
"""
from pathlib import Path
import collections,json
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from verify_value_result_library_dependency_bundle import observe_sources
from audit_cached_native_dependencies import assess_successful_wave,exact_json

def main():
    build_path=EV/"argument-result-specialization-checks-run.json";build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"]
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    binary=Path(build["cli_path"]);assert sha(binary)==build["cli_sha256"]
    previous_path=EV/"general-value-bindings-remaining-feature-bundle-assessment.json";previous=read(previous_path)
    record=Path(previous["current_record_path"]);assert sha(record)==previous["current_record_sha256"]
    original=read(record);model=original["inspection"]
    source_files={str(Path(p).resolve()).lower() for p in original["input_files"] if "/sysml.library/" not in p.replace(chr(92),"/")}
    binding_path=ROOT/"crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/argument-result-specialization-bindings.extract.json"
    kinds={"SysML::"+row["kind"] for row in read(binding_path)["bindings"]}
    receivers=sorted(node["id"] for node in model["constructed_elements"] if node["kind"] in kinds
        and str(Path(bytes.fromhex(node["id"].split(".")[2]).decode("utf-8")).resolve()).lower() in source_files)
    assert len(receivers)==2 and len(source_files)==25 and len(original["input_files"])==119
    requirements=[dict(kind="argument_result_specialization_batch",owner_ids=receivers)]
    folder=ROOT/"target/argument-result-specialization";folder.mkdir(exist_ok=False)
    plan=folder/"requirements.json";save(plan,dict(requirements=requirements))
    baseline_path=EV/"general-value-bindings-end-type-comparison.json";baseline=read(baseline_path)
    spec=folder/"queries-spec.json";save(spec,dict(inspection_queries=[dict(owner_id=row["owner_id"],field=row["field"]) for row in baseline["rows"]]
        +[dict(owner_id=owner,field="argument") for owner in receivers]))
    inputs=observe_sources(original);inputs.update({str(ROOT/p):h for p,h in build["input_sha256"].items()})
    inputs.update({str(p):sha(p) for p in [build_path,binary,previous_path,record,binding_path,plan,spec,baseline_path,Path(__file__),ROOT/"tools/audit_cached_native_dependencies.py"]})
    output=EV/"argument-result-specialization-native-run.json";assert not output.exists()
    manifest=dict(schema="dev.mercurio.argument-result-specialization-native-batch.v1",qualification_certificate=False,status="running",
        input_sha256=inputs,consumer_sha256=build["input_sha256"],runs=[],applicable_receivers=receivers,requested_requirements=requirements,
        required_end_type_reads=76,strict_contexts_qualified=0,strict_obligations_qualified=0,strict_families_qualified=0,candidate_promoted=False)
    save(output,manifest);native=folder/"native.jsonl"
    code=run([str(binary),"--definition-plan-records",str(record),str(plan),str(native)],folder,"native",manifest,output)
    if code:
        manifest.update(status="native_dependency_batch_rejected",failure=read(native) if native.exists() else None,original_candidate_preserved=sha(record)==previous["current_record_sha256"])
        save(output,manifest);print("R42 native batch rejected; precise dependency retained",flush=True);raise SystemExit(code)
    after=read(native);integrity=assess_successful_wave(model["constructed_elements"],model["pending_references"],after,requirements)
    assert after["input_files"]==original["input_files"]
    query_path=folder/"queries.json"
    assert run([str(binary),"--definition-query-records",str(native),str(spec),str(query_path)],folder,"queries",manifest,output)==0
    answers=read(query_path)["queries"];assert len(answers)==78
    comparison=[]
    for before,answer in zip(baseline["rows"],answers[:76],strict=True):
        assert (before["owner_id"],before["field"])==(answer["owner_id"],answer["field"])
        actual=[row["id"] for row in answer.get("targets",[])]
        comparison.append(dict(owner_id=answer["owner_id"],field=answer["field"],native_status=answer["status"],actual_targets=actual,
            expected_targets_by_ordinal=before["expected_targets_by_ordinal"],exact_ordered_match=answer["status"]=="query_evaluated" and all(actual==row for row in before["expected_targets_by_ordinal"])))
    replay=folder/"replay.jsonl"
    assert run([str(binary),"--definition-plan-records",str(native),str(plan),str(replay)],folder,"replay",manifest,output)==0
    again=read(replay);assert exact_json(after["inspection"],again["inspection"]) and again["committed_reference_fields"]==[]
    replay_integrity=assess_successful_wave(after["inspection"]["constructed_elements"],after["inspection"]["pending_references"],again,[])
    fresh=folder/"fresh-queries.json"
    assert run([str(binary),"--definition-query-records",str(replay),str(spec),str(fresh)],folder,"fresh-queries",manifest,output)==0
    assert exact_json(read(fresh)["queries"],answers)
    manifest.update(status="native_argument_result_stage_compared",integrity=integrity,replay_integrity=replay_integrity,
        graph_unchanged_on_replay=True,fresh_queries_exact=True,end_type_outcomes=dict(collections.Counter(row["status"] for row in answers[:76])),
        independently_matched_end_types=sum(row["exact_ordered_match"] for row in comparison),end_type_comparison=comparison,
        argument_query_outcomes=answers[76:],current_record_path=str(native),current_record_sha256=sha(native),current_query_path=str(query_path),current_query_sha256=sha(query_path),
        replay_path=str(replay),replay_sha256=sha(replay),fresh_query_path=str(fresh),fresh_query_sha256=sha(fresh),
        original_candidate_preserved=sha(record)==previous["current_record_sha256"],remaining_dependencies=["Five previously scheduled library crossing/specialization nodes need independent structure/replay assessment",
        "Complete expression/value-chain lifecycle and bounds","Both original Ecore mutations, applicable validation, Closed publication and every terminal context"],
        boundary="Native rule execution and focused end types are separate from complete operator behavior, every delegate, terminal contexts and strict qualification.")
    save(output,manifest);print("R42 end types matched",manifest["independently_matched_end_types"],"/76; strict families 0/34",flush=True)

if __name__=="__main__":main()
