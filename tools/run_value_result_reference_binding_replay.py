"""Replay the verified reference binding graph with the repaired current consumer.

Historical construction and current replay are distinct evidence stages. This
runner cannot turn either stage into enclosing expression/family qualification.
"""
from pathlib import Path
import argparse,json,subprocess,time
from run_value_result_reference_binding_batch import ROOT,EV,read,sha,save,compare
from audit_cached_native_dependencies import assess_successful_wave

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--build-revision",required=True);args=parser.parse_args()
    execution_path=EV/"value-result-reference-binding-namespace-context-execution-run.json";execution=read(execution_path)
    assert execution["status"]=="physical_bundle_compared" and execution["expressions_matched"]==15 and execution["inputs_unchanged"]
    historical_path=EV/"value-result-reference-binding-namespace-context-historical-witness.json";historical=read(historical_path)
    assert sha(execution_path)==historical["execution_run_sha256"]
    archived=historical["archived_witnesses"]
    def historical_hash(path,expected):
        archive=archived.get(str(path))
        return sha(archive["archived_path"] if archive else path)==expected and (archive is None or archive["sha256"]==expected)
    assert all(historical_hash(p,h) for p,h in execution["input_sha256"].items())
    assert all(historical_hash(ROOT/p,h) for p,h in execution["consumer_sha256"].items())
    build_path=EV/("value-result-reference-binding-"+args.build_revision+"-build-run.json");build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"]
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    record=Path(execution["current_record_path"]);assert sha(record)==execution["current_record_sha256"]
    spec_path=EV/"value-result-reference-binding-context-spec.json";spec=read(spec_path)
    roots=spec["native_roots"];assert len(roots)==1 and len(roots[0]["owner_ids"])==15 and spec["required_expressions"]==16 and spec["fixed_contexts"]==26
    reference_run_path=EV/"value-result-reference-binding-context-reference-resolved-run.json";reference_run=read(reference_run_path)
    assert reference_run["status"]=="reference_observed" and reference_run["inputs_unchanged"]
    reference_path=Path(reference_run["output_path"]);assert sha(reference_path)==reference_run["output_sha256"]
    assert all(sha(p)==h for p,h in reference_run["input_sha256"].items())
    identity_path=EV/"value-result-lifecycle-shared-services-source-identities.json"
    checkpoint_path=EV/"value-result-expression-contribution-progress.json";checkpoint=read(checkpoint_path);query_path=Path(checkpoint["current_query_path"])
    assert sha(query_path)==checkpoint["current_query_sha256"]
    inventory_path=EV/"value-result-invocation-input-integrity.json";inventory=read(inventory_path)
    release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert len(inventory["library_sha256"])==94 and all(sha(release/p)==h for p,h in inventory["library_sha256"].items())
    assert len(inventory["frozen_acceptance_sha256"])==4 and all(sha(EV.parent/p)==h for p,h in inventory["frozen_acceptance_sha256"].items())
    folder=ROOT/("target/value-result-reference-binding-"+args.revision+"-replay");folder.mkdir(exist_ok=False)
    plan=folder/"requirements.json";save(plan,dict(requirements=roots));output=folder/"replayed.jsonl";log=folder/"replay.log"
    run_path=EV/("value-result-reference-binding-"+args.revision+"-replay-run.json");assert not run_path.exists()
    files=[execution_path,historical_path,build_path,binary,record,spec_path,reference_run_path,reference_path,identity_path,checkpoint_path,query_path,inventory_path,plan,Path(__file__)]
    inputs={str(p):sha(p) for p in files};command=[str(binary),"--definition-plan-records",str(record),str(plan),str(output)]
    manifest=dict(schema="dev.mercurio.reference-binding-current-replay.v1",qualification_certificate=False,status="running",input_sha256=inputs,consumer_sha256=build["input_sha256"],command=command,
                  complete_native_contexts=0,strict_families_qualified=0,candidate_promoted=False,
                  boundary="Historical original construction and current native replay are separate stages; enclosing transformations, validation and terminal outcomes remain open.")
    save(run_path,manifest);start=time.monotonic()
    with log.open("w",encoding="utf-8") as handle:code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
    manifest.update(exit_code=code,elapsed_seconds=round(time.monotonic()-start,3),log_path=str(log),log_sha256=sha(log),inputs_unchanged=all(sha(p)==h for p,h in inputs.items()))
    if code:manifest["status"]="failed";save(run_path,manifest);raise SystemExit(code)
    prior=read(record)["inspection"];native=read(output)
    integrity=assess_successful_wave(prior["constructed_elements"],prior["pending_references"],native,roots)
    assert integrity["native_nodes_added"]==0 and integrity["native_ports_committed"]==0
    assert native["inspection"]["constructed_elements"]==prior["constructed_elements"]
    assert native["inspection"]["pending_references"]==prior["pending_references"]
    rows=compare(native["inspection"]["constructed_elements"],read(reference_path),read(identity_path)["canonical_resource_fragment_to_native_id"],read(query_path))
    assert len(rows)==16;matched=sum(row.get("exact_ordered_match",False) for row in rows);assert matched==15
    comparison_path=EV/("value-result-reference-binding-"+args.revision+"-replay-comparison.json")
    save(comparison_path,dict(schema="dev.mercurio.reference-binding-replay-comparison.v1",qualification_certificate=False,required_expressions=16,expressions=rows))
    manifest.update(status="native_replay_verified",integrity=integrity,expressions_matched=matched,graph_unchanged=True,output_path=str(output),output_sha256=sha(output),comparison_path=str(comparison_path),comparison_sha256=sha(comparison_path))
    save(run_path,manifest);assert manifest["inputs_unchanged"]
    print("Current native replay: 15/15 bindings, graph and pending references unchanged; no enclosing/family qualification.",flush=True)
if __name__=="__main__":main()
