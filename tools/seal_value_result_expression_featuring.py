"""Seal the shared featuring component; never issue an enclosing certificate."""
from pathlib import Path
import argparse, json
from export_value_result_expression_featuring_reference import ROOT, EV, OUTPUT, read, sha, extract

def verified(path, status):
    value=read(path);assert value["status"]==status and value["inputs_unchanged"]
    if "consumer_sha256" in value:
        assert all(sha(p)==h for p,h in value.get("input_sha256",{}).items())
    assert all(sha(ROOT/p)==h for p,h in value.get("consumer_sha256",{}).items())
    return value

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--replay-revision",required=True);args=parser.parse_args()
    spec_path=EV/"value-result-value-binding-bundle-spec.json";spec=read(spec_path)
    assert spec["required_expressions"]==56 and spec["fixed_contexts"]==26 and spec["fixed_libraries"]==94
    assert read(OUTPUT)==extract()
    paths={phase:EV/("value-result-expression-featuring-"+args.revision+"-"+phase+"-run.json") for phase in ["focused","build","regression","execution","query"]}
    paths["replay"]=EV/("value-result-expression-featuring-"+args.replay_revision+"-execution-run.json")
    checks={phase:verified(paths[phase],"passed") for phase in ["focused","build","regression"]}
    for phase in checks:
        check=checks[phase];assert all(sha(ROOT/p)==h for p,h in check["input_sha256"].items())
        assert sha(ROOT/"tools/run_value_result_expression_featuring_checks.py")==check["runner_sha256"]
        assert sha(check["binary_path"])==check["binary_sha256"]
        assert all(run["exit_code"]==0 and run["required_count_observed"] and sha(run["log_path"])==run["log_sha256"] for run in check["runs"])
    assert checks["focused"]["runs"][0]["required_test_count"]==4
    assert checks["regression"]["runs"][0]["required_test_count"]==515
    assert checks["build"]["runs"][0]["required_test_count"]==13
    execution=verified(paths["execution"],"physical_projection_compared")
    replay=verified(paths["replay"],"physical_projection_compared")
    queries=verified(paths["query"],"stage_compared")
    assert execution["expressions_matched"]==replay["expressions_matched"]==48
    assert replay["graph_unchanged"] and replay["integrity"]["native_nodes_added"]==0 and replay["integrity"]["native_ports_committed"]==0
    assert execution["integrity"]["native_nodes_added"]==45 and execution["integrity"]["native_ports_committed"]==0
    assert execution["fixed_resources"]==119 and execution["fixed_libraries"]==94
    assert queries["featuring_getters_matched"]==48 and queries["stage_getters_matched"]==8 and not queries["newly_unavailable_reads"]
    assert queries["query_outcomes"]==dict(query_evaluated=4293,dependency_required=169)
    assert sha(execution["current_record_path"])==execution["current_record_sha256"]
    assert sha(replay["current_record_path"])==replay["current_record_sha256"]
    assert sha(queries["current_query_path"])==queries["current_query_sha256"]
    comparisons=[Path(execution["comparison_path"]),Path(replay["comparison_path"]),Path(queries["featuring_comparison_path"])]
    assert all(sha(value["comparison_path"])==value["comparison_sha256"] for value in [execution,replay])
    assert sha(queries["featuring_comparison_path"])==queries["featuring_comparison_sha256"]
    for path in comparisons:
        rows=read(path)["accepted_context_projections"];assert len(rows)==48 and all(row["exact_ordered_match"] for row in rows)
    historical_audit=EV/"value-result-expression-featuring-boundary-read-regression-audit.json";audit=read(historical_audit)
    assert audit["status"]=="initial_boundary_not_accepted_for_sealing" and len(audit["regressions"])==4
    assert all(sha(p)==h for p,h in audit["input_sha256"].items())
    archives=read(audit["historical_witnesses"])
    assert all(sha(witness["archived_path"])==witness["sha256"] for witness in archives.values())
    evidence_paths=list(paths.values())+comparisons+[spec_path,OUTPUT,historical_audit,Path(audit["historical_witnesses"]),Path(__file__)]
    progress=dict(schema="dev.mercurio.expression-featuring-progress.v1",qualification_certificate=False,status="bounded_shared_stage_verified",
        required_expressions=56,eligible_expressions=52,accepted_context_projections=48,projections_matched=48,
        required_link_negative_expressions=4,required_negative_context_expressions=8,fixed_contexts=26,fixed_resources=119,fixed_libraries=94,
        native_nodes_added=45,native_ports_committed=0,record_nodes=execution["nodes"],pending_references=execution["pending_references"],
        native_controls_passed=515,focused_controls_passed=4,tooling_controls_passed=13,
        featuring_getters_matched=48,retained_chain_getters_matched=8,fixed_query_outcomes=queries["query_outcomes"],
        graph_unchanged_on_fresh_record_replay=True,original_completion_flags_changed=False,
        current_record_path=execution["current_record_path"],current_record_sha256=execution["current_record_sha256"],
        current_query_path=queries["current_query_path"],current_query_sha256=queries["current_query_sha256"],
        consumer_sha256=checks["build"]["input_sha256"],evidence_sha256={str(p):sha(p) for p in evidence_paths},
        strict_contexts_qualified=0,strict_obligations_qualified=0,strict_services_qualified=0,strict_families_qualified=0,release_gates_qualified=0,candidate_promoted=False,
        handwritten_dependencies=["featuringType delegate algorithm", "ordinary Multiplicity namespace propagation", "typed scheduler and Ecore construction transaction"],
        remaining_dependencies=["cross-feature bound owning-end featuring strategy", "full Feature/Step/Expression and subclass lifecycle", "general value/result and initial/default bindings", "applicable validation, both original Ecore mutations, Closed publication, full-context fresh persistence and all 26 terminal comparisons"],
        retained_failures=["Initial golden comparator omitted the independent adopted field; preserved failed focused run", "Initial library matrix retained four unavailable reads because the default selector misclassified TypeFeaturing; archived source/executable witnesses and original query output retained"],
        boundary="Physical featuring and its native getter only. Extracted definitions, component verification and strict enclosing qualification remain separate.")
    output=EV/"value-result-expression-featuring-progress.json";assert not output.exists()
    output.write_text(json.dumps(progress,indent=2)+"\n",encoding="utf-8")
    print("Sealed featuring stage: 48/48 physical projections and getters; 515 native controls; 0/26 contexts, 0/34 families.")
if __name__=="__main__":main()
