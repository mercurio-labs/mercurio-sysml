"""Bounded shared-chain gate with immutable consumer and executable witnesses."""
from pathlib import Path
import argparse,re,subprocess,time
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--attempt",required=True);parser.add_argument("--focused-only",action="store_true");args=parser.parse_args()
    previous=read(EV/"argument-result-specialization-checks-run.json");paths=set(previous["input_sha256"])
    paths.update("crates/mercurio-sysml/src/language_frontend/lowering/emit/"+name for name in ["canonical_chain_lifecycle.rs","canonical_chain_lifecycle_tests.rs"])
    paths.add(str(Path(__file__).relative_to(ROOT)));witness={p:sha(ROOT/p) for p in sorted(paths)}
    folder=ROOT/("target/chain-lifecycle-checks-"+args.attempt);folder.mkdir(exist_ok=False);output=folder/"run.json"
    manifest=dict(schema="dev.mercurio.canonical-chain-lifecycle-checks.v1",qualification_certificate=False,status="running",input_sha256=witness,runs=[],strict_families_qualified=0);save(output,manifest)
    binary=ROOT/"target/release/deps/mercurio_sysml-a53e5a7d9fbe8855.exe"
    plans=[(["cargo","test","-p","mercurio-sysml","--release","--lib","definition_chain_lifecycle_","--","--nocapture","--test-threads=1"],6)]
    if not args.focused_only:
        plans += [([str(binary),"definition_","--test-threads=1"],539),(["cargo","test","-p","mercurio-tools","--bin","audit_release_compile","--release","cached_definition_","--","--test-threads=1"],13),(["cargo","build","-p","mercurio-tools","--bin","audit_release_compile","--release"],None)]
    for number,(command,count) in enumerate(plans):
        log=folder/("check-"+str(number)+".log");start=time.monotonic()
        with log.open("w",encoding="utf-8") as handle:code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
        text=log.read_text(encoding="utf-8");count_ok=count is None or bool(re.search(rf"test result: ok\. {count} passed; 0 failed;",text))
        manifest["runs"].append(dict(command=command,exit_code=code,required_test_count=count,required_count_observed=count_ok,elapsed_seconds=round(time.monotonic()-start,3),log_path=str(log),log_sha256=sha(log)))
        manifest["inputs_unchanged"]=all(sha(ROOT/p)==h for p,h in witness.items())
        if binary.exists():manifest.update(binary_path=str(binary),binary_sha256=sha(binary))
        save(output,manifest);print("Chain checks",number,"exit",code,"count_ok",count_ok,flush=True)
        if code or not count_ok or not manifest["inputs_unchanged"]:
            manifest["status"]="failed";save(output,manifest);print(text[-7500:],flush=True);raise SystemExit(1)
    manifest.update(status="passed",focused_controls=6,native_controls=6 if args.focused_only else 539)
    if not args.focused_only:manifest.update(tooling_controls=13,cli_path=str(ROOT/"target/release/audit_release_compile.exe"),cli_sha256=sha(ROOT/"target/release/audit_release_compile.exe"))
    save(output,manifest)

if __name__=="__main__":main()
