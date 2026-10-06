"""Focused batch and boundary checks with preserved consumer/executable witnesses."""
from pathlib import Path
import argparse,json,re,subprocess,time
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save

def main():
    parser=argparse.ArgumentParser();parser.add_argument("phase",choices=["focused","build","regression"]);parser.add_argument("--revision",required=True);args=parser.parse_args()
    previous=read(EV/"canonical-chain-result-resolved-sealed-progress.json")
    paths=list(previous["consumer_sha256"])+["crates/mercurio-sysml/src/language_frontend/lowering/emit/general_value_binding.rs",str(Path(__file__).relative_to(ROOT))]
    witness={p:sha(ROOT/p) for p in paths}
    binary=ROOT/"target/release/deps/mercurio_sysml-a53e5a7d9fbe8855.exe"
    plans={
      "focused":[(["cargo","test","-p","mercurio-sysml","--release","--lib","definition_general_value_binding_","--","--nocapture"],5)]+[([str(binary),name,"--test-threads=1"],count) for name,count in [
        ("definition_reference_binding_batch_",5),("definition_result_feature_defaults_",5),("definition_chain_specialization_",4),("definition_general_value_inputs_",3)]],
      "build":[(["cargo","test","-p","mercurio-tools","--bin","audit_release_compile","--release","cached_definition_","--","--test-threads=1"],13),(["cargo","build","-p","mercurio-tools","--bin","audit_release_compile","--release"],None)],
      "regression":[([str(binary),"definition_","--test-threads=1"],527)]}
    prefix="general-value-bindings-"+args.revision+"-"+args.phase;output=EV/(prefix+"-run.json");assert not output.exists()
    manifest=dict(schema="dev.mercurio.general-value-bindings-checks.v1",qualification_certificate=False,status="running",input_sha256=witness,runner_sha256=sha(__file__),runs=[])
    save(output,manifest)
    for number,(command,count) in enumerate(plans[args.phase]):
        log=EV/(prefix+"-"+str(number)+".log");start=time.monotonic()
        with log.open("w",encoding="utf-8") as handle:code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
        text=log.read_text(encoding="utf-8");count_ok=count is None or bool(re.search(rf"test result: ok\. {count} passed; 0 failed;",text))
        manifest["runs"].append(dict(command=command,exit_code=code,required_test_count=count,required_count_observed=count_ok,elapsed_seconds=round(time.monotonic()-start,3),log_path=str(log),log_sha256=sha(log)))
        manifest["inputs_unchanged"]=all(sha(ROOT/p)==h for p,h in witness.items());save(output,manifest)
        print(args.phase,number,"exit",code,"required",count,"count_ok",count_ok,flush=True)
        if code or not count_ok or not manifest["inputs_unchanged"]:
            manifest["status"]="failed";save(output,manifest);print(text[-6500:],flush=True);raise SystemExit(1)
    if args.phase=="build":binary=ROOT/"target/release/audit_release_compile.exe"
    manifest.update(status="passed",binary_path=str(binary),binary_sha256=sha(binary));save(output,manifest)

if __name__=="__main__":main()
