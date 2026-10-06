"""Bounded transitive lexical closure for one fixed 224-getter dependency bundle.

Reads are discovered by native getters, restricted to original actual-library
ports, then applied atomically. No endpoint guesses, flags or qualification.
"""
from pathlib import Path
import argparse,collections
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run,port_descriptor
from verify_value_result_library_dependency_bundle import observe_sources
from audit_cached_native_dependencies import assess_successful_wave,exact_json
PREFIX="general-value-binding-dependencies-"


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--previous-revision",required=True);parser.add_argument("--consumer-native-revision");parser.add_argument("--build-revision",default="result-dispatch-fixture");args=parser.parse_args()
    if args.consumer_native_revision:
        previous_path=EV/("canonical-chain-result-"+args.consumer_native_revision+"-native-run.json");previous=read(previous_path)
        assert previous["status"]=="canonical_chain_result_consumer_dependencies_remain"
        previous=dict(previous,query_path=previous["current_query_path"],query_sha256=previous["current_query_sha256"])
    else:
        previous_path=EV/(PREFIX+args.previous_revision+"-native-run.json");previous=read(previous_path)
        assert previous["status"]=="bounded_library_dependency_stage_verified_with_explicit_getter_disagreements"
    assert previous["inputs_unchanged"]
    assert all(sha(path)==digest for path,digest in previous["input_sha256"].items())
    build_path=EV/("general-value-provider-"+args.build_revision+"-build-run.json");build=read(build_path);assert build["status"]=="passed"
    assert all(sha(ROOT/path)==digest for path,digest in build["input_sha256"].items())
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    record=Path(previous["current_record_path"]);assert sha(record)==previous["current_record_sha256"]
    query_path=Path(previous["query_path"]);assert sha(query_path)==previous["query_sha256"]
    model=read(record);initial=model["inspection"];index={node["id"]:node for node in initial["constructed_elements"]}
    pending={(row["owner_id"],row["field"]):row for row in initial["pending_references"]}
    rows=read(query_path)["queries"];assert len(rows)==224
    folder=ROOT/("target/"+PREFIX+args.revision+"-closure");folder.mkdir(exist_ok=False)
    output=EV/(PREFIX+args.revision+"-closure-run.json");assert not output.exists()
    spec=folder/"queries-spec.json";save(spec,dict(inspection_queries=[dict(owner_id=row["owner_id"],field=row["field"]) for row in rows]))
    inputs=observe_sources(model);inputs.update({str(ROOT/path):digest for path,digest in build["input_sha256"].items()})
    inputs.update({str(path):sha(path) for path in [previous_path,build_path,binary,record,query_path,spec,Path(__file__),ROOT/"tools/audit_cached_native_dependencies.py",ROOT/"tools/run_value_result_library_dependency_bundle.py"]})
    manifest=dict(schema="dev.mercurio.general-value-binding-library-closure.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],waves=[],consumer_sha256=build["input_sha256"],
      fixed_contexts=26,fixed_resources=119,fixed_libraries=94,required_valuations=45,eligible_valuations=41,strict_contexts_qualified=0,strict_obligations_qualified=0,strict_families_qualified=0,candidate_promoted=False)
    save(output,manifest);current_record=record;current=model;committed=set()
    for number in range(16):
        frontier=set();external=[]
        for row in rows:
            if row["status"]!="dependency_required":continue
            dependency=row["dependency"]["prerequisite"]
            if dependency["kind"]=="read_field" and "/sysml.library/" in bytes.fromhex(dependency["owner_id"].split(".")[2]).decode("utf-8").replace(chr(92),"/"):
                key=(dependency["owner_id"],dependency["field"]);assert key in pending and key not in committed,"Repeated or non-original library demand"
                frontier.add(key)
            else:external.append(dependency)
        if not frontier:
            manifest.update(status="native_library_read_closure_discovered",additional_ports=len(committed),ports=[port_descriptor(*key,index,pending) for key in sorted(committed)],
              external_dependencies=external,unsupported_getters=[row for row in rows if row["status"]=="unavailable"],focused_query_outcomes=dict(collections.Counter(row["status"] for row in rows)),
              current_record_path=str(current_record),current_record_sha256=sha(current_record),query_path=str(query_path),query_sha256=sha(query_path),
              boundary="Native library lexical closure only. Added ports still require independent comparison; complete semantic lifecycle, binding and validation remain unqualified.")
            save(output,manifest);print("Transitive library closure:",len(committed),"additional ports;",manifest["focused_query_outcomes"],flush=True);return
        assert len(committed|frontier)<=128,"Reviewed transitive read bound reached"
        roots=[dict(kind="read_field",owner_id=owner,field=field) for owner,field in sorted(frontier)]
        for owner,field in frontier:port_descriptor(owner,field,index,pending)
        plan=folder/("wave-"+str(number)+"-plan.json");save(plan,dict(requirements=roots));manifest["input_sha256"][str(plan)]=sha(plan)
        native=folder/("wave-"+str(number)+"-native.jsonl");code=run([str(binary),"--definition-plan-records",str(current_record),str(plan),str(native)],folder,"wave-"+str(number)+"-native",manifest,output)
        if code:manifest.update(status="native_library_closure_rejected",failure=read(native) if native.exists() else None);save(output,manifest);raise SystemExit(code)
        next_model=read(native);prior=current["inspection"];integrity=assess_successful_wave(prior["constructed_elements"],prior["pending_references"],next_model,roots)
        assert integrity["native_nodes_added"]==0 and next_model["input_files"]==model["input_files"]
        writes={(row["owner_id"],row["field"]) for row in next_model["committed_reference_fields"]};assert writes and not (writes&committed)
        for owner,field in writes:port_descriptor(owner,field,index,pending)
        committed.update(writes);query_path=folder/("wave-"+str(number)+"-queries.json")
        code=run([str(binary),"--definition-query-records",str(native),str(spec),str(query_path)],folder,"wave-"+str(number)+"-queries",manifest,output);assert code==0
        new_rows=read(query_path)["queries"];assert len(new_rows)==224
        before={(row["owner_id"],row["field"]):row for row in rows}
        assert not [row for row in new_rows if before[(row["owner_id"],row["field"])]["status"]=="query_evaluated" and row["status"]!="query_evaluated"]
        manifest["waves"].append(dict(number=number,requirements=roots,ports=[port_descriptor(owner,field,index,pending) for owner,field in sorted(writes)],integrity=integrity,
          native_path=str(native),native_sha256=sha(native),query_path=str(query_path),query_sha256=sha(query_path)))
        current_record,current,rows=native,next_model,new_rows;save(output,manifest)
        print("Native closure wave",number,"committed",len(writes),"ports",flush=True)
    manifest.update(status="native_library_closure_evaluation_bound_reached",boundary="No closure or support claimed.");save(output,manifest);raise SystemExit(1)

if __name__=="__main__":main()
