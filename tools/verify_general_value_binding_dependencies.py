"""Atomic native dependency stage and explicit comparison of every fixed getter.

Getter disagreements and stage differences remain unqualified. This preserves
all acceptance inputs and completion flags and issues no strict certificate.
"""
from pathlib import Path
import argparse,collections
from run_value_result_library_dependency_bundle import ROOT,EV,IDENTITIES,read,sha,save,run,port_descriptor
from verify_value_result_library_dependency_bundle import observe_sources,compare
from audit_cached_native_dependencies import assess_successful_wave,exact_json
PREFIX="general-value-binding-dependencies-"


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--preflight-revision",required=True);parser.add_argument("--build-revision",required=True);parser.add_argument("--reference-revision",required=True);args=parser.parse_args()
    preflight_path=EV/("general-value-binding-"+args.preflight_revision+"-preflight-run.json");preflight=read(preflight_path)
    build_path=EV/("general-value-provider-"+args.build_revision+"-build-run.json");build=read(build_path)
    reference_path=EV/(PREFIX+args.reference_revision+"-reference-run.json");reference=read(reference_path)
    for proof,status in [(preflight,"dependency_bundle_preflight_observed"),(build,"passed"),(reference,"reference_observed")]:
        assert proof["status"]==status and proof["inputs_unchanged"]
    assert all(sha(path)==digest for path,digest in preflight["input_sha256"].items())
    assert all(sha(ROOT/path)==digest for path,digest in build["input_sha256"].items())
    assert all(sha(path)==digest for path,digest in reference["input_sha256"].items())
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    record=Path(preflight["current_record_path"]);assert sha(record)==preflight["current_record_sha256"]
    model=read(record);original=model["inspection"];index={node["id"]:node for node in original["constructed_elements"]}
    pending={(row["owner_id"],row["field"]):row for row in original["pending_references"]}
    roots=[];external=[]
    for row in preflight["required_dependencies"]:
        dependency=row["dependency"]["prerequisite"]
        if dependency["kind"]=="read_field" and "/sysml.library/" in bytes.fromhex(dependency["owner_id"].split(".")[2]).decode("utf-8").replace(chr(92),"/"):
            assert (dependency["owner_id"],dependency["field"]) in pending
            if dependency not in roots:roots.append(dependency)
        else:external.append(dependency)
    roots.sort(key=lambda row:(row["owner_id"],row["field"]));assert roots and len(roots)<=128
    for row in roots:port_descriptor(row["owner_id"],row["field"],index,pending)
    observation=Path(reference["output_path"]);assert sha(observation)==reference["output_sha256"]
    observations=read(observation)["observations"];assert len(observations)==227
    folder=ROOT/("target/"+PREFIX+args.revision+"-native");folder.mkdir(exist_ok=False)
    output=EV/(PREFIX+args.revision+"-native-run.json");assert not output.exists()
    plan=folder/"requirements.json";save(plan,dict(requirements=roots))
    spec=folder/"queries-spec.json";save(spec,dict(inspection_queries=[dict(owner_id=row["owner_id"],field=row["field"]) for row in preflight["queries"]]))
    inputs=observe_sources(model);inputs.update({str(ROOT/path):digest for path,digest in build["input_sha256"].items()})
    inputs.update({str(path):sha(path) for path in [preflight_path,build_path,reference_path,binary,record,observation,spec,plan,IDENTITIES,Path(__file__),ROOT/"tools/audit_cached_native_dependencies.py",ROOT/"tools/verify_value_result_library_dependency_bundle.py"]})
    manifest=dict(schema="dev.mercurio.general-value-binding-dependency-native.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],
      consumer_sha256=build["input_sha256"],required_valuations=45,eligible_valuations=41,fixed_contexts=26,fixed_resources=119,fixed_libraries=94,
      requested_library_ports=roots,external_dependencies=external,strict_contexts_qualified=0,strict_obligations_qualified=0,strict_families_qualified=0,release_gates_qualified=0,candidate_promoted=False)
    save(output,manifest);native=folder/"transformed.jsonl"
    code=run([str(binary),"--definition-plan-records",str(record),str(plan),str(native)],folder,"native",manifest,output)
    if code:manifest.update(status="native_dependency_stage_rejected",failure=read(native) if native.exists() else None);save(output,manifest);raise SystemExit(code)
    current=read(native);integrity=assess_successful_wave(original["constructed_elements"],original["pending_references"],current,roots)
    assert integrity["native_nodes_added"]==0 and current["input_files"]==model["input_files"]
    completed={(row["owner_id"],row["field"]) for row in current["committed_reference_fields"]}
    for key in completed:port_descriptor(*key,index,pending)
    manifest.update(status="native_library_stage_executed",integrity=integrity,current_record_path=str(native),current_record_sha256=sha(native));save(output,manifest)
    query_path=folder/"queries.json";code=run([str(binary),"--definition-query-records",str(native),str(spec),str(query_path)],folder,"focused-queries",manifest,output);assert code==0
    queries=read(query_path)["queries"];assert len(queries)==224 and {(row["owner_id"],row["field"]) for row in queries}=={(row["owner_id"],row["field"]) for row in preflight["queries"]}
    compared=compare(observations,current["inspection"]["constructed_elements"],queries)
    library=[row for row in compared if row["strategy"]=="library_reference"];getters=[row for row in compared if row["strategy"]=="provider_getter"]
    assert len(library)==3 and len(getters)==224 and all(row["exact_ordered_match"] for row in library)
    mismatches=[dict(row,qualification_disposition="unqualified; reconcile actual lifecycle stage and normative rule before closure") for row in getters if not row["exact_ordered_match"]]
    comparison_path=EV/(PREFIX+args.revision+"-comparison.json");save(comparison_path,dict(qualification_certificate=False,comparisons=compared,unqualified_comparisons=mismatches,reference_mutations=observations))
    previous={(row["owner_id"],row["field"]):row for row in read(preflight["query_path"])["queries"]}
    regressed=[row for row in queries if previous[(row["owner_id"],row["field"])]["status"]=="query_evaluated" and row["status"]!="query_evaluated"]
    assert not regressed
    manifest.update(status="dependency_stage_compared",library_ports_matched=3,getters_matched=sum(row["exact_ordered_match"] for row in getters),unqualified_getters=len(mismatches),
      getter_comparison_outcomes=dict(collections.Counter(("matched" if row["exact_ordered_match"] else row["native_status"]) for row in getters)),focused_query_outcomes=dict(collections.Counter(row["status"] for row in queries)),
      regressed_reads=regressed,comparison_path=str(comparison_path),comparison_sha256=sha(comparison_path),query_path=str(query_path),query_sha256=sha(query_path));save(output,manifest)
    replay=folder/"replay.jsonl";code=run([str(binary),"--definition-plan-records",str(native),str(plan),str(replay)],folder,"replay",manifest,output);assert code==0
    restored=read(replay);assert restored["requested_requirements"]==roots
    replay_integrity=assess_successful_wave(current["inspection"]["constructed_elements"],current["inspection"]["pending_references"],restored,[])
    assert replay_integrity["native_nodes_added"]==replay_integrity["native_ports_committed"]==0
    assert exact_json(restored["inspection"]["constructed_elements"],current["inspection"]["constructed_elements"]) and restored["inspection"]["pending_references"]==current["inspection"]["pending_references"]
    fresh_path=folder/"fresh-queries.json";code=run([str(binary),"--definition-query-records",str(replay),str(spec),str(fresh_path)],folder,"fresh-queries",manifest,output);assert code==0
    assert exact_json(read(fresh_path)["queries"],queries)
    manifest.update(status="bounded_library_dependency_stage_verified_with_explicit_getter_disagreements",graph_unchanged_on_replay=True,fresh_queries_exact=True,replay_path=str(replay),replay_sha256=sha(replay),
      fresh_query_path=str(fresh_path),fresh_query_sha256=sha(fresh_path),original_candidate_preserved=sha(record)==preflight["current_record_sha256"],
      remaining_dependencies=["Resolve every independent getter disagreement at the same lifecycle stage", "All general value bindings, bound specialization and complete receiver lifecycle", "Default/initial dispatch and applicable validators including both original Ecore mutations", "Both required negative scope outcomes and all strict qualification stages"],
      boundary="Only the compared library ports and exact indicated getters have component evidence. An evaluated getter or mismatch explained by possible stage difference is not semantic support or strict qualification.")
    save(output,manifest);print("Library ports:",len(completed),"committed, 3 independently matched; getters",manifest["getters_matched"],"/224;",manifest["focused_query_outcomes"],"; strict families 0/34",flush=True)

if __name__=="__main__":main()
