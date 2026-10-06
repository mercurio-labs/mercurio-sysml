"""Evidence-backed component closure; no strict behavior or family certificate."""
from pathlib import Path
import argparse
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save
PREFIX="canonical-chain-result-"


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--native-revision",required=True);parser.add_argument("--check-revision",required=True);args=parser.parse_args()
    native_path=EV/(PREFIX+args.native_revision+"-native-run.json");native=read(native_path)
    assert native["status"]=="bounded_canonical_chain_result_queries_verified" and native["inputs_unchanged"] and native["newly_evaluated_chain_results"]==3 and not native["regressed_reads"]
    assert all(sha(path)==digest for path,digest in native["input_sha256"].items())
    assert all(row["exit_code"]==0 and sha(row["log_path"])==row["log_sha256"] for row in native["runs"])
    proofs={};files=[native_path,Path(__file__)]
    for phase,count in [("focused",13),("build",13),("regression",522)]:
        path=EV/("general-value-provider-"+args.check_revision+"-"+phase+"-run.json");proof=read(path)
        assert proof["status"]=="passed" and proof["inputs_unchanged"] and proof["input_sha256"]==native["consumer_sha256"]
        assert all(sha(ROOT/name)==digest for name,digest in proof["input_sha256"].items())
        assert all(row["exit_code"]==0 and row["required_count_observed"] and sha(row["log_path"])==row["log_sha256"] for row in proof["runs"])
        assert sum(row["required_test_count"] or 0 for row in proof["runs"])==count and sha(proof["binary_path"])==proof["binary_sha256"]
        proofs[phase]=proof;files.append(path)
    for name in ["current_record","current_query","fresh_query","comparison"]:
        path=Path(native[name+"_path"]);assert sha(path)==native[name+"_sha256"];files.append(path)
    integrity_path=EV/"value-result-invocation-input-integrity.json";integrity=read(integrity_path);release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert len(integrity["library_sha256"])==94 and all(sha(release/name)==digest for name,digest in integrity["library_sha256"].items())
    assert all(sha(EV.parent/name)==digest for name,digest in integrity["frozen_acceptance_sha256"].items());files.append(integrity_path)
    archives=[]
    for folder in ["r40-chain-result-source-before","r40-chain-result-fixture-before","r40-super-before","r40-adopt-before"]:
        path=ROOT/"target"/folder/"witnesses.json";witnesses=read(path)
        assert all(sha(row["archived_path"])==row["sha256"] for row in witnesses);archives.append(dict(path=str(path),sha256=sha(path),witnesses=len(witnesses)));files.append(path)
    previous=read(EV/"general-value-provider-valuation-envelope-sealed-progress.json");assert sha(previous["current_record_path"])==previous["current_record_sha256"]
    progress=dict(native);progress.update(status="bounded_canonical_chain_result_consumer_stage_sealed",qualification_certificate=False,
      native_controls_passed=522,focused_controls_passed=13,tooling_controls_passed=13,evidence_sha256={str(path):sha(path) for path in files},historical_witness_archives=archives,
      original_candidate_preserved=True,strict_services_qualified=0,
      remaining_dependencies=["General result chains and binary value bindings across all 41 eligible valuations, with three default and one initial dispatch", "Bound specialization and complete superclass/Expression/Feature lifecycle", "Resolve raw/staged getter disagreements at the same lifecycle stage", "Both required negative source scopes, both original Ecore mutations and applicable validators", "Complete terminal publication/persistence/comparison and all strict family/release gates"],
      boundary="A native consumer and canonical stage controls are present for the three result reads. Getter availability and component controls do not establish full semantic support. Independent getter disagreements, lifecycle and qualification remain open.")
    output=EV/(PREFIX+args.revision+"-sealed-progress.json");assert not output.exists();save(output,progress)
    print("Sealed canonical result consumers: 3/3;",progress["focused_query_outcomes"],";",progress["getters_matched"],"/224 exact raw getter matches; 522 native, 13 focused, 13 tooling; strict families 0/34",flush=True)

if __name__=="__main__":main()
