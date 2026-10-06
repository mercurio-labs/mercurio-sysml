"""Verify a discovered native library bundle in one atomic wave and fresh replay.

The full fixed read matrix runs at this completed dependency-bundle boundary.
This tool awards bounded component evidence, never strict family qualification.
"""
from pathlib import Path
import argparse,collections,json
from run_value_result_library_dependency_bundle import ROOT,EV,PREFIX,read,sha,save,run,setup,IDENTITIES
from audit_cached_native_dependencies import assess_successful_wave,exact_json
from audit_value_result_provider_plan import native_identity


def observe_sources(model):
    original=read(EV/"value-result-value-binding-lexical-initial-reference-run.json")
    expected=original["input_sha256"];sources={str(Path(path)):sha(path) for path in model["input_files"]}
    for path,digest in sources.items():
        assert path in expected and expected[path]==digest,"Original source/library fingerprint changed: "+path
    return sources


def compare(observations,graph,queries):
    identities=read(IDENTITIES)["canonical_resource_fragment_to_native_id"]
    index={node["id"]:node for node in graph};query_index={(row["owner_id"],row["field"]):row for row in queries}
    def endpoint(row):
        resource=row["resource"].replace(chr(92),"/");fragment=row["emf_fragment"]
        id=native_identity(resource,fragment) if resource.startswith("sysml.library/") else identities[resource+"#"+fragment]
        return dict(id=id,kind="SysML::"+row["kind"])
    compared=[]
    for ref in observations:
        root=ref["root"];library=root["required_outcome"]=="positive_library_reference"
        if library:
            value=index[root["owner_id"]]["properties"].get(root["field"]);values=value if isinstance(value,list) else [value] if value is not None else []
            actual=[dict(id=id,kind=index[id]["kind"]) for id in values];status="query_evaluated"
        else:
            query=query_index[(root["owner_id"],root["field"])];status=query["status"]
            actual=[dict(id=row["id"],kind=row["kind"]) for row in query.get("targets",[])]
        expected=[endpoint(row) for row in ref["endpoints"]]
        compared.append(dict(owner_id=root["owner_id"],field=root["field"],strategy="library_reference" if library else "provider_getter",
          reference_status=ref["status"],native_status=status,expected=expected,actual=actual,
          exact_ordered_match=ref["status"]=="reference_observed" and status=="query_evaluated" and actual==expected,
          provider_completion_before=ref["provider_completion_before"],provider_completion_after=ref["provider_completion_after"],
          reference_source_nodes_before=ref["source_nodes_before"],reference_source_nodes_after=ref["source_nodes_after"]))
    return compared


def retained_projections(queries):
    by_key={(row["owner_id"],row["field"]):row for row in queries};featuring=[];chain=[]
    for ref in read(EV/"value-result-expression-featuring-full-cache-observations.json")["expressions"]:
        row=by_key[(ref["owner_id"],"featuring_type")]
        actual=[target["id"] for target in row.get("targets",[])]
        featuring.append(dict(owner_id=ref["owner_id"],actual=actual,expected=ref["expected"],exact_ordered_match=row["status"]=="query_evaluated" and actual==ref["expected"]))
    for context in read(EV/"value-result-chain-transformation-resolved-reference-observations.json")["observations"]:
        for ref in context["parameter_getters"]:
            field={"inheritedMembership":"inherited_membership","type":"type"}[ref["field"]]
            row=by_key[(context["control"]["parameter_id"],field)]
            expected=[dict(id=native_identity(target["resource"],target["emf_fragment"]),kind="SysML::"+target["kind"]) for target in ref["endpoints"]]
            actual=[dict(id=target["id"],kind=target["kind"]) for target in row.get("targets",[])]
            chain.append(dict(owner_id=row["owner_id"],field=field,actual=actual,expected=expected,exact_ordered_match=ref["status"]=="reference_observed" and row["status"]=="query_evaluated" and actual==expected))
    assert len(featuring)==48 and len(chain)==8
    assert all(row["exact_ordered_match"] for row in featuring+chain)
    return featuring,chain


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--discovery-revision",required=True);parser.add_argument("--reference-revision",required=True);args=parser.parse_args()
    checkpoint,inventory,build,binary,record,model,pending,inputs=setup();inputs.update(observe_sources(model))
    discovery_path=EV/(PREFIX+args.discovery_revision+"-discovery-run.json");discovery=read(discovery_path)
    reference_path=EV/(PREFIX+args.reference_revision+"-reference-run.json");reference=read(reference_path)
    assert discovery["status"]=="library_lexical_dependency_bundle_discovered" and reference["status"]=="reference_observed"
    for prior in [discovery,reference]:
        assert prior["inputs_unchanged"] and all(sha(p)==h for p,h in prior["input_sha256"].items())
    observation=Path(reference["output_path"]);assert sha(observation)==reference["output_sha256"]
    folder=ROOT/("target/"+PREFIX+args.revision+"-verification");folder.mkdir(exist_ok=False)
    output=EV/(PREFIX+args.revision+"-verification-run.json");assert not output.exists()
    ports=discovery["ports"];assert 2<=len(ports)<=128
    roots=[dict(kind="read_field",owner_id=row["owner_id"],field=row["field"]) for row in ports]
    assert len({(row["owner_id"],row["field"]) for row in roots})==len(ports)
    plan=folder/"requirements.json";save(plan,dict(requirements=roots))
    fixed=EV/"value-result-lifecycle-reference-read-spec.json";baseline=Path(checkpoint["current_query_path"])
    assert sha(baseline)==checkpoint["current_query_sha256"]
    files=[discovery_path,reference_path,observation,plan,Path(__file__),fixed,baseline,
      EV/"value-result-expression-featuring-full-cache-observations.json",EV/"value-result-chain-transformation-resolved-reference-observations.json"]
    inputs.update({str(path):sha(path) for path in files})
    manifest=dict(schema="dev.mercurio.library-dependency-verification.v1",qualification_certificate=False,status="running",input_sha256=inputs,
      consumer_sha256=build["input_sha256"],fixed_contexts=26,fixed_resources=119,fixed_libraries=94,required_library_ports=len(ports),required_provider_getters=14,
      runs=[],strict_contexts_qualified=0,strict_obligations_qualified=0,strict_services_qualified=0,strict_families_qualified=0,release_gates_qualified=0,candidate_promoted=False)
    save(output,manifest)
    native=folder/"transformed.jsonl";code=run([str(binary),"--definition-plan-records",str(record),str(plan),str(native)],folder,"native",manifest,output)
    if code:manifest.update(status="atomic_library_bundle_rejected",failed_output=read(native) if native.exists() else None);save(output,manifest);raise SystemExit(1)
    current=read(native);original=model["inspection"];integrity=assess_successful_wave(original["constructed_elements"],original["pending_references"],current,roots)
    assert integrity["native_nodes_added"]==0 and integrity["native_ports_committed"]==len(ports)
    assert current["input_files"]==model["input_files"]
    manifest.update(status="atomic_library_bundle_executed",integrity=integrity,current_record_path=str(native),current_record_sha256=sha(native));save(output,manifest)
    queries_path=folder/"queries.json";code=run([str(binary),"--definition-query-records",str(native),str(fixed),str(queries_path)],folder,"fixed-queries",manifest,output);assert code==0
    queries=read(queries_path)["queries"];previous={(row["owner_id"],row["field"]):row for row in read(baseline)["queries"]}
    assert len(queries)==4462 and {(row["owner_id"],row["field"]) for row in queries}==previous.keys()
    regressions=[row for row in queries if previous[(row["owner_id"],row["field"])]["status"]=="query_evaluated" and row["status"]!="query_evaluated"]
    compared=compare(read(observation)["observations"],current["inspection"]["constructed_elements"],queries)
    library=[row for row in compared if row["strategy"]=="library_reference"];providers=[row for row in compared if row["strategy"]=="provider_getter"]
    assert len(library)==len(ports) and len(providers)==14
    featuring,chain=retained_projections(queries)
    comparison_path=EV/(PREFIX+args.revision+"-comparison.json");save(comparison_path,dict(qualification_certificate=False,ports=compared,featuring=featuring,chain=chain))
    manifest.update(status="library_bundle_compared",library_ports_matched=sum(row["exact_ordered_match"] for row in library),provider_getters_matched=sum(row["exact_ordered_match"] for row in providers),
      unqualified_provider_getters=[row for row in providers if not row["exact_ordered_match"]],regressed_reads=regressions,
      fixed_query_outcomes=dict(collections.Counter(row["status"] for row in queries)),featuring_getters_matched=48,chain_getters_matched=8,
      current_query_path=str(queries_path),current_query_sha256=sha(queries_path),comparison_path=str(comparison_path),comparison_sha256=sha(comparison_path));save(output,manifest)
    assert manifest["library_ports_matched"]==len(ports) and not regressions
    replay=folder/"replay.jsonl";code=run([str(binary),"--definition-plan-records",str(native),str(plan),str(replay)],folder,"replay",manifest,output);assert code==0
    restored=read(replay);assert restored["requested_requirements"]==roots
    replay_integrity=assess_successful_wave(current["inspection"]["constructed_elements"],current["inspection"]["pending_references"],restored,[])
    assert replay_integrity["native_nodes_added"]==replay_integrity["native_ports_committed"]==0
    assert exact_json(restored["inspection"]["constructed_elements"],current["inspection"]["constructed_elements"]) and restored["inspection"]["pending_references"]==current["inspection"]["pending_references"]
    manifest.update(status="bounded_library_dependency_stage_verified",graph_unchanged_on_replay=True,replay_path=str(replay),replay_sha256=sha(replay),
      boundary="Registered actual-library lexical closure and the indicated getter comparisons are verified. Unmatched provider getters, negative scope contracts and complete context lifecycle remain open.")
    save(output,manifest)
    for phase,count in [("focused",1),("build",13),("regression",516)]:
        proof=read(EV/("value-result-value-binding-lexical-replay-regression-repaired-"+phase+"-run.json"))
        assert proof["status"]=="passed" and proof["inputs_unchanged"] and all(sha(ROOT/p)==h for p,h in proof["input_sha256"].items())
        assert sha(proof["binary_path"])==proof["binary_sha256"] and all(run["exit_code"]==0 and run["required_count_observed"] and sha(run["log_path"])==run["log_sha256"] for run in proof["runs"])
        assert proof["runs"][0]["required_test_count"]==count
    progress=dict(manifest);progress.update(evidence_sha256={str(path):sha(path) for path in [output,discovery_path,reference_path,comparison_path]},
      native_controls_passed=516,focused_controls_passed=4,tooling_controls_passed=13,
      remaining_dependencies=["Both required missing-name scope contracts remain unqualified", "All unsupported provider reads and non-library semantic dependencies retained in discovery", "Complete FeatureValue/Expression transformations, validation, original Ecore mutations, Closed publication, persistence and terminal comparisons"])
    progress_path=EV/(PREFIX+"progress.json");assert not progress_path.exists();save(progress_path,progress)
    print("Sealed library ports",len(ports),"/",len(ports),"; provider getters",manifest["provider_getters_matched"],"/14;",manifest["fixed_query_outcomes"],"; 0/34 strict families",flush=True)

if __name__=="__main__":main()
