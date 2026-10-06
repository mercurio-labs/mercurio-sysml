"""Seal bounded reference binding evidence while retaining strict release gaps."""
from pathlib import Path
import json,collections
from run_value_result_reference_binding_batch import ROOT,EV,read,sha,save

def main():
    names={phase:EV/("value-result-reference-binding-boundary-"+phase+"-run.json") for phase in ["focused","build","regression","replay","query"]}
    runs={phase:read(path) for phase,path in names.items()}
    for phase in ["focused","build","regression"]:
        run=runs[phase]
        assert run["status"]=="passed" and run["inputs_unchanged"],phase
        assert sha(run["binary_path"])==run["binary_sha256"],phase
        assert all(sha(ROOT/p)==h for p,h in run["input_sha256"].items()),phase
        assert sha(ROOT/"tools/run_value_result_reference_binding_checks.py")==run["runner_sha256"]
        assert all(row["exit_code"]==0 and row["required_count_observed"] and sha(row["log_path"])==row["log_sha256"] for row in run["runs"])
    assert [r["required_test_count"] for r in runs["focused"]["runs"]]==[5,1]
    assert [r["required_test_count"] for r in runs["build"]["runs"]]==[13,None]
    assert [r["required_test_count"] for r in runs["regression"]["runs"]]==[511]
    replay=runs["replay"];queries=runs["query"]
    assert replay["status"]=="native_replay_verified" and replay["expressions_matched"]==15 and replay["graph_unchanged"]
    assert replay["integrity"]["native_nodes_added"]==0 and replay["integrity"]["native_ports_committed"]==0
    assert queries["status"]=="stage_compared" and queries["stage_getters_matched"]==8
    for run in [replay,queries]:
        assert run["inputs_unchanged"] and all(sha(p)==h for p,h in run["input_sha256"].items())
        assert all(sha(ROOT/p)==h for p,h in run["consumer_sha256"].items())
    execution_path=EV/"value-result-reference-binding-namespace-context-execution-run.json";execution=read(execution_path)
    historical_path=EV/"value-result-reference-binding-namespace-context-historical-witness.json";historical=read(historical_path)
    assert sha(execution_path)==historical["execution_run_sha256"]
    assert execution["status"]=="physical_bundle_compared" and execution["expressions_matched"]==15 and execution["inputs_unchanged"]
    archived=historical["archived_witnesses"]
    def historical_hash(path,expected):
        archive=archived.get(str(path))
        return sha(archive["archived_path"] if archive else path)==expected and (archive is None or archive["sha256"]==expected)
    assert all(historical_hash(p,h) for p,h in execution["input_sha256"].items())
    assert all(historical_hash(ROOT/p,h) for p,h in execution["consumer_sha256"].items())
    record=Path(execution["current_record_path"])
    assert sha(record)==execution["current_record_sha256"]==queries["current_record_sha256"]
    assert sha(replay["output_path"])==replay["output_sha256"]
    model=read(record);inspection=model["inspection"]
    assert len(model["input_files"])==119
    assert model["element_count"]==len(inspection["constructed_elements"])==94097
    assert model["pending_reference_count"]==len(inspection["pending_references"])==18843
    query=read(queries["current_query_path"])
    assert sha(queries["current_query_path"])==queries["current_query_sha256"]
    assert len(query["queries"])==4462 and set(queries["query_outcomes"])<={"query_evaluated","dependency_required"}
    comparison_path=Path(execution["comparison_path"]);comparison=read(comparison_path)
    assert sha(comparison_path)==execution["comparison_sha256"]
    rows=comparison["expressions"];assert len(rows)==16
    assert sum(row.get("exact_ordered_match",False) for row in rows)==15
    assert sum(row["status"]=="link_negative_not_qualified" for row in rows)==1
    assert sha(replay["comparison_path"])==replay["comparison_sha256"]
    assert read(replay["comparison_path"])["expressions"]==rows
    nodes={node["id"]:node for node in inspection["constructed_elements"]}
    expression_rows=read(EV/"value-result-expression-contribution-repaired-structural-comparison.json")["expressions"]
    assert len(expression_rows)==56 and all(nodes[row["owner_id"]]["properties"].get("is_implied_included",False) is False for row in expression_rows)
    inventory_path=EV/"value-result-invocation-input-integrity.json";inventory=read(inventory_path)
    release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert len(inventory["library_sha256"])==94 and all(sha(release/p)==h for p,h in inventory["library_sha256"].items())
    assert len(inventory["frozen_acceptance_sha256"])==4 and all(sha(EV.parent/p)==h for p,h in inventory["frozen_acceptance_sha256"].items())
    additional=[execution_path,historical_path,comparison_path,Path(replay["comparison_path"]),Path(queries["current_query_path"]),inventory_path,
                EV/"value-result-reference-binding-boundary-stage-getter-comparison.json",EV/"value-result-reference-binding-boundary-cold-reference-comparison.json",Path(__file__)]
    files=list(names.values())+additional
    progress=dict(schema="dev.mercurio.reference-binding-progress.v1",qualification_certificate=False,status="bounded_shared_stage_verified",
        consumer_sha256=runs["build"]["input_sha256"],input_sha256={str(p):sha(p) for p in files},
        fixed_contexts=26,fixed_libraries=94,fixed_resources=119,fixed_queries=4462,fixed_original_source_identities=388,
        required_reference_expressions=16,eligible_reference_expressions=15,expressions_matched=15,
        required_link_negative_expressions=1,link_negative_expressions_qualified=0,
        membership_counts=dict(collections.Counter(row["actual"]["membership"] for row in rows if row.get("exact_ordered_match"))),
        native_definition_controls_passed=511,focused_controls_passed=6,tooling_controls_passed=13,
        native_bundle_qualified=False,current_native_replay_verified=True,
        current_record_path=str(record),current_record_sha256=sha(record),current_query_path=queries["current_query_path"],current_query_sha256=queries["current_query_sha256"],
        native_nodes=94097,pending_references=18843,integrity=execution["integrity"],query_outcomes=queries["query_outcomes"],retained_chain_getters_matched=8,
        cold_reference_outcomes=queries["cold_reference_outcomes"],library_sha256=inventory["library_sha256"],frozen_acceptance_sha256=inventory["frozen_acceptance_sha256"],
        complete_native_contexts=0,reviewed_obligations_closed=0,value_services_qualified=0,strict_families_qualified=0,release_gates_qualified=0,candidate_promoted=False,
        reference_stage="Explicit input featuring computation followed by fresh reference connector/end lifecycle; cold reference observations remain preserved separately.",
        completed_stage="All 15 eligible actual-library reference bindings match ordered placement, endpoints and physical contributions; repaired current consumer proves idempotent replay without changing the retained graph.",
        implementation="Imported binary default programs and Ecore endpoint/ownership contracts drive shared native construction. Dependency preflight, scheduling, context/featuring algorithms and transactional admission are explicit handwritten Rust.",
        historical_construction_boundary="Initial full library execution and repaired current-consumer replay have separate exact source/executable witnesses. The former is historical; the latter verifies retained graph behavior.",
        remaining_dependencies=["complete superclass Feature/Step/Expression transformations","remaining value/result/self-result connectors and semantic dependencies","complete featuring, parameters, bounds and variability","all scoped validators and required negative outcomes","Closed final publication and fresh whole-context persistence","all 26 independent terminal comparisons","remaining B1/B2/B3 obligations, 310 samples and final semantic/timing qualification"],
        boundary="15/15 bounded stage matches are not complete contexts, services, strict families or full release support. The link-negative obligation remains open. Original expression completion flags stay false.",
        celebration="Pending first accepted strict family certificate.")
    out=EV/"value-result-reference-binding-progress.json";archive=ROOT/"target/r34-source-before/progress-before-seal.json";assert not archive.exists();archive.write_bytes(out.read_bytes());save(out,progress)
    print("Sealed R34: 15/15 bindings; 511 native, 6 focused, 13 tooling controls; 4293 evaluated reads/169 typed dependencies; 8/8 retained getters.",flush=True)
    print("Strict closure: 0/26 contexts, 0/10 B1 obligations, 0/1 services, 0/34 families, 0/5 gates.",flush=True)
if __name__=="__main__":main()
