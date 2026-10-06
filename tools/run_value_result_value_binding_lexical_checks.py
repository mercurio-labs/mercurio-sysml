"""Focused checks and witness manifests for the shared lexical dependency and completed-read batch."""
import argparse, hashlib,json,subprocess,time,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
EV=ROOT/"docs/conformance/2026-08-support/definition-pipeline-evidence"
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2)+"\n",encoding="utf-8")
def main():
    parser=argparse.ArgumentParser();parser.add_argument("phase",choices=["focused","build","regression"]);parser.add_argument("--revision",required=True);args=parser.parse_args()
    old=json.loads((EV/"value-result-expression-contribution-progress.json").read_text(encoding="utf-8"))
    paths=list(old["consumer_sha256"])+["crates/mercurio-sysml/src/language_frontend/lowering/emit/binding_dependencies.rs","crates/mercurio-sysml/src/language_frontend/lowering/emit/reference_binding.rs"]
    paths.append("crates/mercurio-sysml/src/language_frontend/lowering/emit/expression_featuring.rs")
    paths.append("crates/mercurio-sysml/src/language_frontend/lowering/emit/multiplicity_featuring.rs")
    paths.append("crates/mercurio-sysml/src/definition_document_cached_dependencies_tests.rs")
    witness={p:sha(ROOT/p) for p in paths}
    plans={
      "focused":[(["cargo","test","-p","mercurio-sysml","--release","--lib","definition_completed_reference_jobs_","--","--nocapture"],1)]+[
          ([str(ROOT/"target/release/deps/mercurio_sysml-a53e5a7d9fbe8855.exe"),name,"--test-threads=1"],1) for name in [
            "definition_cached_dependencies_resume_only_remaining_native_ports",
            "definition_reference_index_independent_links_pending_retry_and_cycles",
            "definition_value_result_provider_plan_preserves_work_and_reuses_nested_roots"]],
      "build":[(["cargo","test","-p","mercurio-tools","--bin","audit_release_compile","--release","cached_definition_","--","--test-threads=1"],13),(["cargo","build","-p","mercurio-tools","--bin","audit_release_compile","--release"],None)],
      "regression":[([str(ROOT/"target/release/deps/mercurio_sysml-a53e5a7d9fbe8855.exe"),"definition_","--test-threads=1"],516)]}
    prefix="value-result-value-binding-lexical-"+args.revision+"-"+args.phase;out=EV/(prefix+"-run.json");assert not out.exists()
    record=dict(schema="dev.mercurio.value-binding-lexical-check-run.v1",qualification_certificate=False,status="running",input_sha256=witness,runner_sha256=sha(__file__),runs=[])
    save(out,record)
    for number,(command,count) in enumerate(plans[args.phase]):
      log=EV/(prefix+"-"+str(number)+".log");start=time.monotonic()
      with log.open("w",encoding="utf-8") as handle:code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
      content=log.read_text(encoding="utf-8");count_ok=count is None or bool(re.search(rf"test result: ok\. {count} passed; 0 failed;",content))
      record["runs"].append(dict(command=command,exit_code=code,required_test_count=count,required_count_observed=count_ok,elapsed_seconds=round(time.monotonic()-start,3),log_path=str(log),log_sha256=sha(log)))
      record["inputs_unchanged"]=all(sha(ROOT/p)==h for p,h in witness.items());save(out,record)
      print(args.phase,number,"exit",code,"required",count,"count_ok",count_ok,flush=True)
      if code or not count_ok or not record["inputs_unchanged"]:
        record["status"]="failed";save(out,record);print(content[-6000:],flush=True);raise SystemExit(1)
    binary=ROOT/("target/release/audit_release_compile.exe" if args.phase=="build" else "target/release/deps/mercurio_sysml-a53e5a7d9fbe8855.exe")
    record.update(status="passed",binary_path=str(binary),binary_sha256=sha(binary));save(out,record)
if __name__=="__main__":main()
