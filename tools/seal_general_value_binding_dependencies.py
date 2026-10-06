"""Seal one library dependency strategy without promoting getters to qualification."""
from pathlib import Path
import argparse,collections
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from verify_value_result_library_dependency_bundle import compare,observe_sources
from audit_cached_native_dependencies import assess_successful_wave,exact_json
PREFIX="general-value-binding-dependencies-"


def checked(path,status):
    proof=read(path);assert proof["status"]==status and proof["inputs_unchanged"]
    assert all(sha(name)==digest for name,digest in proof["input_sha256"].items())
    for row in proof["runs"]:assert row["exit_code"]==0 and sha(row["log_path"])==row["log_sha256"]
    return proof


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--closure-revision",required=True);parser.add_argument("--closure-reference-revision",required=True);args=parser.parse_args()
    native_path=EV/(PREFIX+"result-dispatch-native-run.json");native=checked(native_path,"bounded_library_dependency_stage_verified_with_explicit_getter_disagreements")
    closure_path=EV/(PREFIX+args.closure_revision+"-closure-run.json");closure=checked(closure_path,"native_library_read_closure_discovered")
    reference_paths=[EV/(PREFIX+"initial-reference-run.json"),EV/(PREFIX+"result-dispatch-library-reference-run.json"),EV/(PREFIX+args.closure_reference_revision+"-closure-reference-run.json")]
    references=[checked(path,"reference_observed") for path in reference_paths]
    observations=[];files=[native_path,closure_path,*reference_paths]
    for proof in references:
        output=Path(proof["output_path"]);assert sha(output)==proof["output_sha256"];files.append(output);observations.extend(read(output)["observations"])
    assert len(observations)==232+closure["additional_ports"]
    assert len({(row["root"]["owner_id"],row["root"]["field"]) for row in observations})==len(observations)
    checks=[]
    for phase,count in [("focused",8),("build",13),("regression",521)]:
        path=EV/("general-value-provider-result-dispatch-fixture-"+phase+"-run.json");proof=read(path)
        assert proof["status"]=="passed" and proof["inputs_unchanged"] and all(sha(ROOT/name)==digest for name,digest in proof["input_sha256"].items())
        assert sha(proof["binary_path"])==proof["binary_sha256"]
        assert all(row["exit_code"]==0 and row["required_count_observed"] and sha(row["log_path"])==row["log_sha256"] for row in proof["runs"])
        assert sum(row["required_test_count"] or 0 for row in proof["runs"])==count
        files.append(path);checks.append(proof)
    build=checks[1];binary=Path(build["binary_path"])
    record=Path(closure["current_record_path"]);query=Path(closure["query_path"])
    assert sha(record)==closure["current_record_sha256"] and sha(query)==closure["query_sha256"]
    model=read(record);rows=read(query)["queries"];assert len(rows)==224
    roots=native["requested_library_ports"]+[dict(kind="read_field",owner_id=row["owner_id"],field=row["field"]) for row in closure["ports"]]
    assert len(roots)==8+closure["additional_ports"] and len({(row["owner_id"],row["field"]) for row in roots})==len(roots)
    roots.sort(key=lambda row:(row["owner_id"],row["field"]))
    compared=compare(observations,model["inspection"]["constructed_elements"],rows)
    library=[row for row in compared if row["strategy"]=="library_reference"];getters=[row for row in compared if row["strategy"]=="provider_getter"]
    assert len(library)==len(roots) and len(getters)==224 and all(row["exact_ordered_match"] for row in library)
    assert {(row["owner_id"],row["field"]) for row in library}=={(row["owner_id"],row["field"]) for row in roots}
    mismatch=[dict(row,qualification_disposition="unqualified; same-stage and normative assessment required") for row in getters if not row["exact_ordered_match"]]
    folder=ROOT/("target/"+PREFIX+args.revision+"-seal");folder.mkdir(exist_ok=False)
    plan=folder/"requirements.json";save(plan,dict(requirements=roots));spec=folder/"queries-spec.json";save(spec,dict(inspection_queries=[dict(owner_id=row["owner_id"],field=row["field"]) for row in rows]))
    output=EV/(PREFIX+args.revision+"-sealed-progress.json");assert not output.exists()
    files.extend([record,query,binary,plan,spec,Path(__file__),ROOT/"tools/verify_value_result_library_dependency_bundle.py",ROOT/"tools/audit_cached_native_dependencies.py"])
    integrity_path=EV/"value-result-invocation-input-integrity.json";integrity=read(integrity_path);release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert len(integrity["library_sha256"])==94 and all(sha(release/name)==digest for name,digest in integrity["library_sha256"].items())
    assert all(sha(EV.parent/name)==digest for name,digest in integrity["frozen_acceptance_sha256"].items());files.append(integrity_path)
    inputs=observe_sources(model);inputs.update({str(ROOT/name):digest for name,digest in build["input_sha256"].items()});inputs.update({str(path):sha(path) for path in files})
    manifest=dict(schema="dev.mercurio.general-value-binding-dependency-checkpoint.v1",qualification_certificate=False,status="running",input_sha256=inputs,consumer_sha256=build["input_sha256"],runs=[],
      library_ports_matched=len(library),library_ports_required=len(roots),getters_matched=sum(row["exact_ordered_match"] for row in getters),getters_required=224,unqualified_getters=len(mismatch),
      focused_query_outcomes=dict(collections.Counter(row["status"] for row in rows)),native_controls_passed=521,focused_controls_passed=8,tooling_controls_passed=13,
      strict_contexts_qualified=0,strict_obligations_qualified=0,strict_services_qualified=0,strict_families_qualified=0,release_gates_qualified=0,candidate_promoted=False,
      fixed_contexts=26,fixed_resources=119,fixed_libraries=94,required_valuations=45,eligible_valuations=41,current_record_path=str(record),current_record_sha256=sha(record),current_query_path=str(query),current_query_sha256=sha(query))
    save(output,manifest);replay=folder/"replay.jsonl";code=run([str(binary),"--definition-plan-records",str(record),str(plan),str(replay)],folder,"replay",manifest,output);assert code==0
    restored=read(replay);assert restored["requested_requirements"]==roots
    replay_integrity=assess_successful_wave(model["inspection"]["constructed_elements"],model["inspection"]["pending_references"],restored,[])
    assert replay_integrity["native_nodes_added"]==replay_integrity["native_ports_committed"]==0
    assert exact_json(restored["inspection"]["constructed_elements"],model["inspection"]["constructed_elements"]) and restored["inspection"]["pending_references"]==model["inspection"]["pending_references"]
    fresh=folder/"fresh-queries.json";code=run([str(binary),"--definition-query-records",str(replay),str(spec),str(fresh)],folder,"fresh-queries",manifest,output);assert code==0
    assert exact_json(read(fresh)["queries"],rows)
    comparison=EV/(PREFIX+args.revision+"-sealed-comparison.json");save(comparison,dict(qualification_certificate=False,library_ports=library,provider_getters=getters,unqualified_comparisons=mismatch,reference_getter_mutations=observations))
    original=read(EV/"general-value-provider-valuation-envelope-sealed-progress.json");assert sha(original["current_record_path"])==original["current_record_sha256"]
    manifest.update(status="bounded_original_library_read_closure_verified_with_unqualified_getters",graph_unchanged_on_replay=True,fresh_queries_exact=True,original_candidate_preserved=True,
      replay_path=str(replay),replay_sha256=sha(replay),fresh_query_path=str(fresh),fresh_query_sha256=sha(fresh),comparison_path=str(comparison),comparison_sha256=sha(comparison),
      native_nodes=len(model["inspection"]["constructed_elements"]),pending_references=model["pending_reference_count"],
      remaining_dependencies=["Canonical chain-result specialization admission for three positive result-type getters", "All independent getter disagreements and the required source linking negatives", "General/default/initial value bindings, bound specialization and complete superclass/Expression/Feature lifecycle", "Both original Ecore mutations, applicable validation, terminal publication/persistence/comparison and strict certificates"],
      boundary="This seals only the original actual-library read closure and the exact indicated getter matches. Extraction, evaluated queries and raw/staged getter disagreement do not qualify behavior, contexts or families.")
    save(output,manifest);print("Sealed library read closure:",len(library),"/",len(roots),"; getter matches",manifest["getters_matched"],"/224;",manifest["focused_query_outcomes"],"; strict families 0/34",flush=True)

if __name__=="__main__":main()
