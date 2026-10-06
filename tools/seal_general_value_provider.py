"""Seal current bounded selector evidence; do not award obligation/family certificates."""
from pathlib import Path
import argparse,json
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--check-revision",required=True);args=parser.parse_args()
    progress=read(EV/("general-value-provider-"+args.revision+"-progress.json"))
    assert progress["status"]=="bounded_non_typing_valuation_dependency_stage_verified" and progress["provider_getters_matched"]==18
    assert progress["inputs_unchanged"] and progress["input_graph_unchanged"] and progress["input_pending_registry_unchanged"] and progress["fresh_query_replay_unchanged"]
    assert not progress["regressed_reads"] and progress["strict_families_qualified"]==0
    assert progress["fixed_query_outcomes"]==dict(query_evaluated=4451,dependency_required=11)
    witnesses={};archives=[ROOT/"target"/name/"witnesses.json" for name in ["r38-provider-source-before","r38-provider-fixture-before","r38-provider-ownership-before","r38-provider-control-before","r38-provider-binaries-before","r38-provider-negative-fixture-before","r38-provider-envelope-before"]]
    for archive in archives:
        for row in read(archive):
            assert sha(row["archived_path"])==row["sha256"]
            witnesses[(str(Path(row["path"])),row["sha256"])]=row["archived_path"]
    def supported_hash(path,digest):
        return sha(path)==digest or (str(Path(path)),digest) in witnesses
    controls=[]
    for phase,count in [("focused",3),("build",13),("regression",519)]:
        path=EV/("general-value-provider-"+args.check_revision+"-"+phase+"-run.json");proof=read(path)
        assert proof["status"]=="passed" and proof["inputs_unchanged"]
        assert all(sha(ROOT/rel)==digest for rel,digest in proof["input_sha256"].items())
        assert sha(proof["binary_path"])==proof["binary_sha256"]
        assert all(row["exit_code"]==0 and row["required_count_observed"] and sha(row["log_path"])==row["log_sha256"] for row in proof["runs"])
        assert proof["runs"][0]["required_test_count"]==count
        controls.append(path)
    assert all(sha(ROOT/rel)==digest for rel,digest in progress["consumer_sha256"].items())
    assert all(sha(path)==digest for path,digest in progress["input_sha256"].items())
    assert all(sha(path)==digest for path,digest in progress["evidence_sha256"].items())
    normative=EV/"general-value-provider-normative-disposition.json"
    for path,digest in read(normative)["input_sha256"].items():assert sha(path)==digest
    history=[EV/name for name in ["general-value-provider-initial-focused-run.json","general-value-provider-fixture-corrected-focused-run.json", "general-value-provider-fixture-corrected-build-run.json","general-value-provider-fixture-corrected-regression-run.json","general-value-provider-initial-native-run.json","general-value-provider-ownership-corrected-focused-run.json","general-value-provider-bounded-corrected-focused-run.json","general-value-provider-bounded-corrected-build-run.json","general-value-provider-bounded-corrected-regression-run.json","general-value-provider-ownership-corrected-native-run.json","general-value-provider-progress.json"]]
    for path in history:
        item=read(path)
        for rel,digest in item["input_sha256"].items():
            absolute=Path(rel) if Path(rel).is_absolute() else ROOT/rel
            assert supported_hash(absolute,digest),str(absolute)
        for row in item.get("runs",[]):assert sha(row["log_path"])==row["log_sha256"]
    output=EV/("general-value-provider-"+args.revision+"-sealed-progress.json");assert not output.exists()
    files=controls+archives+history+[normative,EV/("general-value-provider-"+args.revision+"-progress.json"),ROOT/"tools/run_general_value_provider_batch.py",ROOT/"tools/run_general_value_provider_checks.py",Path(__file__)]
    sealed=dict(progress);sealed.update(status="bounded_non_typing_valuation_dependency_stage_sealed",native_controls_passed=519,focused_controls_passed=7,tooling_controls_passed=13,
      historical_failures_retained=True,archived_consumer_witnesses_verified=True,evidence_sha256={str(path):sha(path) for path in files},
      remaining_dependencies=["General value-result chains and binary bindings across all 45 valuations, including the initial context", "Complete Expression/Feature/superclass lifecycle and normative bound-value specialization", "Both original Ecore mutations and every required terminal validation/publication/persistence outcome", "Both missing-name global scope contracts and all 11 dependent reads", "Full remaining obligations, 34 strict families, 310 samples and comparable final timing"])
    save(output,sealed)
    print("Sealed R38: 18/18 getter controls; 519 native, 7 focused, 13 tooling controls; strict qualification unchanged",flush=True)

if __name__=="__main__":main()
