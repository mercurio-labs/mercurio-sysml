"""Focused and batch-boundary verification of shared chain-target consumers."""
import argparse
import hashlib
import json
import re
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EV = ROOT / "docs/conformance/2026-08-support/definition-pipeline-evidence"
def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))
def save(path,value):
    Path(path).write_text(json.dumps(value,indent=2)+"\n",encoding="utf-8")

def run(phase, revision=""):
    prefix = "value-result-chain-target-" + (revision + "-" if revision else "")
    paths=list(read(EV/"value-result-invocation-build-run.json")["input_sha256"])
    paths += [
        "crates/mercurio-sysml/src/language_frontend/lowering/emit/feature_chain_targets.rs",
        "crates/mercurio-sysml/src/language_frontend/lowering/emit/feature_chain_members.rs",
        "crates/mercurio-sysml/src/language_frontend/lowering/emit/feature_chain_members_generated.rs",
        "docs/conformance/2026-08-support/definition-pipeline-evidence/value-result-chain-selection-registered-reference-controls.json",
        "docs/conformance/2026-08-support/definition-pipeline-evidence/value-result-chain-selection-registered-reference-run.json",
        "tools/test_generate_ecore_model.py",
    ]
    inputs={p:sha(ROOT/p) for p in paths}
    binary=ROOT/"target/release/audit_release_compile.exe"
    tests=ROOT/"target/release/deps/mercurio_sysml-a53e5a7d9fbe8855.exe"
    plans={
        "focused":[
            (["python","-B","tools/generate_ecore_model.py","--check"],None),
            (["python","-B","-m","unittest","discover","-s","tools","-p","test_generate_ecore_model.py"],21),
            (["cargo","test","-p","mercurio-sysml","--release","--lib","definition_chain_targets","--","--nocapture"],4),
        ],
        "build":[
            (["cargo","test","-p","mercurio-tools","--bin","audit_release_compile","--release","cached_definition_","--","--test-threads=1"],13),
            (["cargo","build","-p","mercurio-tools","--bin","audit_release_compile","--release"],None),
        ],
        "regression":[([str(tests),"definition_","--test-threads=1"],498)],
    }
    output=EV/f"{prefix}{phase}-run.json"
    assert not output.exists(),"Inspect existing results rather than silently restarting a batch."
    result=dict(schema="dev.mercurio.chain-target-check-run.v1",qualification_certificate=False,
        input_sha256=inputs,runner_sha256=sha(__file__),phase=phase,runs=[],status="running")
    save(output,result)
    for number,(command,expected) in enumerate(plans[phase]):
        log=EV/f"{prefix}{phase}-{number}.log"
        start=time.monotonic()
        with log.open("w",encoding="utf-8") as handle:
            code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
        text=log.read_text(encoding="utf-8")
        step=dict(command=command,exit_code=code,elapsed_seconds=round(time.monotonic()-start,3),
            log_path=str(log),log_sha256=sha(log),expected_tests=expected)
        if expected is not None:
            step["required_count_observed"]=(
                f"Ran {expected} tests" in text if command[0]=="python"
                else bool(re.search(rf"test result: ok\. {expected} passed; 0 failed;",text)))
        result["runs"].append(step)
        result["inputs_unchanged"]=all(sha(ROOT/p)==s for p,s in inputs.items())
        save(output,result)
        print(phase,number,code,"expected",expected,flush=True)
        if code or not result["inputs_unchanged"] or (expected is not None and not step["required_count_observed"]):
            result["status"]="failed";save(output,result);print(text[-5000:],flush=True);raise SystemExit(1)
    result["status"]="passed"
    if phase=="build":result["binary_sha256"]=sha(binary)
    if phase in ("focused","regression"):result["test_binary_sha256"]=sha(tests)
    save(output,result)
if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase",choices=["focused","build","regression"])
    parser.add_argument("--revision", default="")
    args=parser.parse_args()
    run(args.phase,args.revision)
