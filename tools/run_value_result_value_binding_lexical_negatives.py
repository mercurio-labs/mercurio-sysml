"""Record both prescribed native missing-name failures without publishing a model."""
from pathlib import Path
import argparse, json
from run_value_result_value_binding_lexical import ROOT,EV,SPEC,read,sha,save,run_command,verify_inputs

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--native-revision",required=True);parser.add_argument("--reference-revision",default="initial");args=parser.parse_args()
    spec=read(SPEC);verify_inputs(spec)
    native_path=EV/("value-result-value-binding-lexical-"+args.native_revision+"-native-run.json");native=read(native_path)
    assert native["status"]=="native_reference_compared" and native["ports_matched"]==34 and native["inputs_unchanged"]
    record=Path(native["current_record_path"]);assert sha(record)==native["current_record_sha256"]
    build_path=EV/"value-result-expression-featuring-repaired-build-run.json";build=read(build_path)
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"] and all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    ref_path=EV/("value-result-value-binding-lexical-"+args.reference_revision+"-reference-run.json");ref=read(ref_path)
    observed_path=Path(ref["output_path"]);assert ref["status"]=="reference_observed" and sha(observed_path)==ref["output_sha256"]
    negatives=[p for p in spec["ports"] if p["required_outcome"]=="link_negative_unqualified"];assert len(negatives)==2
    observed={(p["root"]["owner_id"],p["root"]["field"]):p for p in read(observed_path)["observations"]}
    assert all(observed[(p["owner_id"],p["field"])]["status"]=="reference_getter_failed" for p in negatives)
    folder=ROOT/("target/value-result-value-binding-lexical-"+args.revision+"-negative");folder.mkdir(exist_ok=False)
    plan=folder/"requirements.json";save(plan,dict(batches=[dict(name=p["context"],requirements=[dict(kind="read_field",owner_id=p["owner_id"],field=p["field"])]) for p in negatives]))
    output=folder/"failures.jsonl";log=folder/"native.log";run_path=EV/("value-result-value-binding-lexical-"+args.revision+"-negative-run.json");assert not run_path.exists()
    files=[SPEC,native_path,record,build_path,binary,ref_path,observed_path,plan,Path(__file__),ROOT/"tools/run_value_result_value_binding_lexical.py"]
    inputs={str(p):sha(p) for p in files};command=[str(binary),"--definition-plan-records",str(record),str(plan),str(output)]
    manifest=dict(schema="dev.mercurio.value-binding-lexical-negative-run.v1",qualification_certificate=False,status="running",input_sha256=inputs,consumer_sha256=build["input_sha256"],command=command,required_missing_names=2,strict_families_qualified=0,complete_native_contexts=0)
    save(run_path,manifest);result=run_command(command,log);manifest.update(result)
    assert result["exit_code"]!=0 and output.exists()
    failures=[json.loads(line) for line in output.read_text(encoding="utf-8").splitlines() if line.strip()];assert len(failures)==2
    for port,failure in zip(negatives,failures,strict=True):
        assert failure["status"]=="blocked" and "inspection" not in failure and "model" not in failure
        assert failure["requested_requirements"]==[dict(kind="read_field",owner_id=port["owner_id"],field=port["field"])]
        failed=failure["failed_native_reference_reads"];assert failed and all(p["owner_id"]==port["owner_id"] and p["field"]==port["field"] for p in failed)
        assert all(p["pending_descriptors"] and all(d["spelling"]==port["spelling"] for d in p["pending_descriptors"]) for p in failed)
    manifest.update(status="prescribed_failure_controls_recorded",input_model_returned=False,failures=failures,output_path=str(output),output_sha256=sha(output),inputs_unchanged=all(sha(p)==h for p,h in inputs.items()),
        boundary="Both exact failing lexical ports are independently observed; complete negative-context parsing/transformation/validation/publication/persistence qualification is not claimed. An unsupported algorithm cannot satisfy an expected rejection.")
    save(run_path,manifest);assert manifest["inputs_unchanged"]
    print("Recorded 2 prescribed failing lexical ports; no partial model returned.")
    for failure in failures:print(failure["batch_name"],failure["error"],flush=True)
if __name__=="__main__":main()
