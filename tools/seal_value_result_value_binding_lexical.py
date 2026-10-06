"""Seal positive lexical dependencies without hiding open provider/scope gaps."""
from pathlib import Path
import argparse,json
from run_value_result_value_binding_lexical import ROOT,EV,SPEC,read,sha,hash_matches,verify_inputs

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--replay-revision",required=True);parser.add_argument("--check-revision",required=True);args=parser.parse_args()
    spec=read(SPEC);verify_inputs(spec)
    checks={phase:EV/("value-result-value-binding-lexical-"+args.check_revision+"-"+phase+"-run.json") for phase in ["focused","build","regression"]}
    for phase,path in checks.items():
        value=read(path);assert value["status"]=="passed" and value["inputs_unchanged"]
        assert sha(ROOT/"tools/run_value_result_value_binding_lexical_checks.py")==value["runner_sha256"]
        assert all(sha(ROOT/p)==h for p,h in value["input_sha256"].items())
        assert sha(value["binary_path"])==value["binary_sha256"]
        assert all(run["exit_code"]==0 and run["required_count_observed"] and sha(run["log_path"])==run["log_sha256"] for run in value["runs"])
        assert value["runs"][0]["required_test_count"]==dict(focused=1,build=13,regression=516)[phase]
        if phase=="focused":assert len(value["runs"])==4 and all(run["required_test_count"]==1 for run in value["runs"])
    paths={"native":EV/("value-result-value-binding-lexical-"+args.revision+"-native-run.json"),
      "replay":EV/("value-result-value-binding-lexical-"+args.replay_revision+"-native-run.json"),
      "queries":EV/("value-result-value-binding-lexical-"+args.revision+"-query-run.json"),
      "reference":EV/"value-result-value-binding-lexical-initial-reference-run.json"}
    for phase,path in paths.items():
        value=read(path);assert value["inputs_unchanged"] and all(hash_matches(p,h) for p,h in value["input_sha256"].items())
        if "consumer_sha256" in value:assert all(sha(ROOT/p)==h for p,h in value["consumer_sha256"].items())
    native,replay,queries,reference=[read(paths[phase]) for phase in ["native","replay","queries","reference"]]
    assert native["status"]==replay["status"]=="native_reference_compared" and native["ports_matched"]==replay["ports_matched"]==34
    assert native["integrity"]["native_ports_committed"]==34 and native["integrity"]["native_nodes_added"]==0
    assert replay["graph_unchanged"] and replay["integrity"]["native_nodes_added"]==replay["integrity"]["native_ports_committed"]==0
    assert reference["status"]=="reference_observed" and reference["getter_outcomes"]==dict(reference_observed=34,reference_getter_failed=2)
    assert queries["status"]=="stage_compared" and not queries["newly_unavailable_reads"] and queries["featuring_getters_matched"]==48 and queries["stage_getters_matched"]==8
    assert len(queries["exposed_unsupported_provider_queries"])==18
    for value in [native,replay]:
        assert sha(value["current_record_path"])==value["current_record_sha256"] and sha(value["comparison_path"])==value["comparison_sha256"]
        rows=read(value["comparison_path"])["ports"];assert len(rows)==36 and sum(row.get("exact_ordered_match",False) for row in rows)==34
    assert sha(queries["current_query_path"])==queries["current_query_sha256"]
    history=[EV/("value-result-value-binding-lexical-"+name+"-run.json") for name in [
      "initial-native", "initial-query", "replay-native", "initial-negative", "stored-reference-replay-regression", "completed-read-replay-native"]]
    history += [ROOT/"target/r36-source-before/witnesses.json",ROOT/"target/r36-replay-regression-before/witnesses.json",ROOT/"target/r36-replay-verifier-before/witnesses.json"]
    assert all(path.exists() for path in history)
    for archive in history[-3:]:
        assert all(sha(value["archived_path"])==value["sha256"] for value in read(archive).values())
    files=list(paths.values())+list(checks.values())+history+[SPEC,Path(__file__),Path(native["comparison_path"]),Path(replay["comparison_path"])]
    progress=dict(schema="dev.mercurio.value-binding-lexical-progress.v1",qualification_certificate=False,status="bounded_positive_lexical_stage_verified",
      required_ports=36,positive_ports_matched=34,required_missing_name_ports_unqualified=2,fixed_contexts=26,fixed_resources=119,fixed_libraries=94,
      native_ports_committed=34,native_nodes_added=0,graph_unchanged_on_fresh_record_replay=True,native_controls_passed=516,focused_controls_passed=4,tooling_controls_passed=13,
      fixed_query_outcomes=queries["query_outcomes"],exposed_unsupported_provider_queries=queries["exposed_unsupported_provider_queries"],featuring_getters_matched=48,retained_chain_getters_matched=8,
      current_record_path=native["current_record_path"],current_record_sha256=native["current_record_sha256"],current_query_path=queries["current_query_path"],current_query_sha256=queries["current_query_sha256"],
      consumer_sha256=read(checks["build"])["input_sha256"],evidence_sha256={str(p):sha(p) for p in files},
      strict_contexts_qualified=0,strict_obligations_qualified=0,strict_services_qualified=0,strict_families_qualified=0,release_gates_qualified=0,candidate_promoted=False,
      remaining_dependencies=["Both missing-name controls still require a complete external/global scope contract; generic failure is not negative qualification", "18 typed/valued/directed Feature provider reads remain explicitly unsupported", "25 remaining typed dependencies include genuine missing names and transitive library superclass/subset references", "Complete expression/value lifecycle, validation, original Ecore mutations, Closed publication, persistence and terminal comparison"],
      historical_runs=[str(path) for path in history[:-3]],
      boundary="Exact positive lexical targets and validated completed-read reuse are verified. The first overbroad read gate, failed replay, unsupported negative scope failures and three repaired prior replay expectations remain recorded. Negative scope completion and the enclosing semantic lifecycle remain open.")
    output=EV/"value-result-value-binding-lexical-progress.json";assert not output.exists();output.write_text(json.dumps(progress,indent=2)+"\n",encoding="utf-8")
    print("Sealed 34 positive lexical references; 516 native controls; provider and negative scope gaps remain open; 0/34 strict families.")
if __name__=="__main__":main()
