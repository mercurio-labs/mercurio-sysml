"""Regression boundary with the strengthened partial-state snapshot tracked."""
from pathlib import Path
import argparse,re,subprocess,time
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);args=parser.parse_args()
    prior=read(EV/"general-value-bindings-canonical-controls-focused-run.json");assert prior["status"]=="passed"
    paths=list(prior["input_sha256"])+["crates/mercurio-sysml/src/definition_document_pilot_tests.rs",str(Path(__file__).relative_to(ROOT))]
    inputs={p:sha(ROOT/p) for p in paths}
    binary=ROOT/"target/release/deps/mercurio_sysml-a53e5a7d9fbe8855.exe"
    command=[str(binary),"definition_","--test-threads=1"]
    prefix="general-value-bindings-"+args.revision+"-regression";output=EV/(prefix+"-run.json");assert not output.exists()
    manifest=dict(schema="dev.mercurio.general-value-bindings-checks.v1",qualification_certificate=False,status="running",input_sha256=inputs,runner_sha256=sha(__file__),runs=[])
    save(output,manifest);log=EV/(prefix+"-0.log");start=time.monotonic()
    with log.open("w",encoding="utf-8") as handle:code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
    text=log.read_text(encoding="utf-8");count_ok=bool(re.search(r"test result: ok\. 527 passed; 0 failed;",text))
    unchanged=all(sha(ROOT/p)==h for p,h in inputs.items())
    manifest.update(status="passed" if code==0 and count_ok and unchanged else "failed",inputs_unchanged=unchanged,binary_path=str(binary),binary_sha256=sha(binary))
    manifest["runs"].append(dict(command=command,exit_code=code,required_test_count=527,required_count_observed=count_ok,elapsed_seconds=round(time.monotonic()-start,3),log_path=str(log),log_sha256=sha(log)))
    save(output,manifest);print("native regression",manifest["status"],flush=True)
    if manifest["status"]!="passed":print(text[-5500:],flush=True);raise SystemExit(1)
if __name__=="__main__":main()
