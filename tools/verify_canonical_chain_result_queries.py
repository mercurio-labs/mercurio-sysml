"""Current native query consumers over the sealed valuation/library dependency matrix."""
from pathlib import Path
import argparse,collections
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from verify_value_result_library_dependency_bundle import observe_sources,compare
from audit_cached_native_dependencies import exact_json
PREFIX="canonical-chain-result-"


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--build-revision",required=True);parser.add_argument("--closure-revision");args=parser.parse_args()
    checkpoint_path=EV/"general-value-binding-dependencies-result-closure-sealed-progress.json";checkpoint=read(checkpoint_path)
    assert checkpoint["status"]=="bounded_original_library_read_closure_verified_with_unqualified_getters" and checkpoint["library_ports_matched"]==23
    historical={}
    for path,digest in checkpoint["consumer_sha256"].items():
        source=ROOT/path
        if sha(source)!=digest:source=ROOT/"target/r40-chain-result-source-before"/path
        assert sha(source)==digest;historical[str(source)]=digest
    build_path=EV/("general-value-provider-"+args.build_revision+"-build-run.json");build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"] and all(sha(ROOT/path)==digest for path,digest in build["input_sha256"].items())
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    record=Path(checkpoint["current_record_path"]);baseline=Path(checkpoint["current_query_path"])
    assert sha(record)==checkpoint["current_record_sha256"] and sha(baseline)==checkpoint["current_query_sha256"]
    extra_files=[];extra_observations=[]
    if args.closure_revision:
        closure_path=EV/("general-value-binding-dependencies-"+args.closure_revision+"-closure-run.json");closure=read(closure_path)
        assert closure["status"]=="native_library_read_closure_discovered" and closure["inputs_unchanged"]
        assert all(sha(path)==digest for path,digest in closure["input_sha256"].items())
        record=Path(closure["current_record_path"]);assert sha(record)==closure["current_record_sha256"]
        reference_path=EV/("general-value-binding-dependencies-"+args.closure_revision+"-closure-reference-run.json");reference=read(reference_path)
        assert reference["status"]=="reference_observed" and reference["inputs_unchanged"] and all(sha(path)==digest for path,digest in reference["input_sha256"].items())
        observation_path=Path(reference["output_path"]);assert sha(observation_path)==reference["output_sha256"]
        extra_observations=read(observation_path)["observations"];assert len(extra_observations)==closure["additional_ports"]
        extra_files=[closure_path,reference_path,observation_path]
    record_sha=sha(record)
    model=read(record);previous={(row["owner_id"],row["field"]):row for row in read(baseline)["queries"]};assert len(previous)==224
    unknown=[row for row in previous.values() if row["status"]=="unavailable"];assert len(unknown)==3 and all(row["field"]=="type" for row in unknown)
    comparison_before=Path(checkpoint["comparison_path"]);assert sha(comparison_before)==checkpoint["comparison_sha256"]
    observations=read(comparison_before)["reference_getter_mutations"];assert len(observations)==247;observations+=extra_observations
    folder=ROOT/("target/"+PREFIX+args.revision);folder.mkdir(exist_ok=False)
    spec=folder/"queries-spec.json";save(spec,dict(inspection_queries=[dict(owner_id=owner,field=field) for owner,field in sorted(previous)]))
    output=EV/(PREFIX+args.revision+"-native-run.json");assert not output.exists()
    inputs=observe_sources(model);inputs.update(historical);inputs.update({str(ROOT/path):digest for path,digest in build["input_sha256"].items()})
    inputs.update({str(path):sha(path) for path in [*extra_files,checkpoint_path,comparison_before,build_path,binary,record,baseline,spec,Path(__file__),ROOT/"tools/verify_value_result_library_dependency_bundle.py",ROOT/"tools/audit_cached_native_dependencies.py"]})
    manifest=dict(schema="dev.mercurio.canonical-chain-result-consumer.v1",qualification_certificate=False,status="running",input_sha256=inputs,consumer_sha256=build["input_sha256"],runs=[],
      required_getters=224,required_valuations=45,eligible_valuations=41,fixed_contexts=26,fixed_resources=119,fixed_libraries=94,strict_contexts_qualified=0,strict_obligations_qualified=0,strict_families_qualified=0,release_gates_qualified=0,candidate_promoted=False)
    save(output,manifest);queries=folder/"queries.json";code=run([str(binary),"--definition-query-records",str(record),str(spec),str(queries)],folder,"focused-queries",manifest,output);assert code==0
    rows=read(queries)["queries"];assert len(rows)==224 and {(row["owner_id"],row["field"]) for row in rows}==previous.keys()
    indexed={(row["owner_id"],row["field"]):row for row in rows}
    manifest.update(status="canonical_chain_result_queries_observed",focused_query_outcomes=dict(collections.Counter(row["status"] for row in rows)),
      prior_chain_result_reads=[indexed[(row["owner_id"],row["field"])] for row in unknown],newly_evaluated_chain_results=sum(indexed[(row["owner_id"],row["field"])]["status"]=="query_evaluated" for row in unknown),
      regressed_reads=[row for row in rows if previous[(row["owner_id"],row["field"])]["status"]=="query_evaluated" and row["status"]!="query_evaluated"],current_record_path=str(record),current_record_sha256=sha(record),current_query_path=str(queries),current_query_sha256=sha(queries));save(output,manifest)
    compared=compare(observations,model["inspection"]["constructed_elements"],rows);library=[row for row in compared if row["strategy"]=="library_reference"];getters=[row for row in compared if row["strategy"]=="provider_getter"]
    assert len(library)==23+len(extra_observations) and len(getters)==224 and all(row["exact_ordered_match"] for row in library)
    comparison=EV/(PREFIX+args.revision+"-comparison.json");save(comparison,dict(qualification_certificate=False,comparisons=compared,unqualified_comparisons=[row for row in getters if not row["exact_ordered_match"]],reference_getter_mutations=observations))
    manifest.update(library_ports_matched=len(library),getters_matched=sum(row["exact_ordered_match"] for row in getters),unqualified_getters=sum(not row["exact_ordered_match"] for row in getters),comparison_path=str(comparison),comparison_sha256=sha(comparison));save(output,manifest)
    if manifest["newly_evaluated_chain_results"]!=3 or manifest["regressed_reads"]:
        manifest["status"]="canonical_chain_result_consumer_dependencies_remain";save(output,manifest);raise SystemExit(1)
    fresh=folder/"fresh-queries.json";code=run([str(binary),"--definition-query-records",str(record),str(spec),str(fresh)],folder,"fresh-queries",manifest,output);assert code==0
    assert exact_json(read(fresh)["queries"],rows) and sha(record)==record_sha
    manifest.update(status="bounded_canonical_chain_result_queries_verified",fresh_queries_exact=True,candidate_unchanged=True,fresh_query_path=str(fresh),fresh_query_sha256=sha(fresh),
      boundary="Three canonical chain-result query consumers and retained library ports are verified at this native stage. Raw/staged reference disagreements, bindings, full lifecycle, negatives and qualification remain open.")
    save(output,manifest);print("Canonical chain-result queries:",manifest["newly_evaluated_chain_results"],"/3; native matrix",manifest["focused_query_outcomes"],"; raw Pilot matches",manifest["getters_matched"],"/224; strict families 0/34",flush=True)

if __name__=="__main__":main()
