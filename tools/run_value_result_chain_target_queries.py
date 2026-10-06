"""Requery the fixed original chain matrix with the current native consumer."""
import argparse
import collections
import hashlib
import json
import subprocess
import time
from pathlib import Path
from audit_value_result_ownership import compare

ROOT=Path(__file__).resolve().parents[1]
EV=ROOT/"docs/conformance/2026-08-support/definition-pipeline-evidence"
def read(p):
    return json.loads(Path(p).read_text(encoding="utf-8"))
def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,value):
    Path(p).write_text(json.dumps(value,indent=2)+"\n",encoding="utf-8")
def run(revision="", baseline="value-result-chain-pending-closure-run.json"):
    prefix = "value-result-chain-target-" + (revision + "-" if revision else "")
    build_path=EV/f"{prefix}build-run.json"
    build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"]
    binary=ROOT/"target/release/audit_release_compile.exe"
    assert sha(binary)==build["binary_sha256"]
    assert all(sha(ROOT/p)==s for p,s in build["input_sha256"].items())
    previous_path=EV/baseline
    previous=read(previous_path)
    record=Path(previous["final_record_path"])
    assert sha(record)==previous["final_record_sha256"]
    files=[build_path,previous_path,binary,record,Path(__file__)]
    names=["value-result-lifecycle-reference-read-spec.json","value-result-ownership-reference-spec.json",
        "value-result-ownership-reference-observations.json","value-result-lifecycle-shared-services-source-identities.json"]
    files.extend(EV/name for name in names)
    inputs={str(p):sha(p) for p in files}
    query_path=ROOT/"target"/(prefix.rstrip("-"))/"queries.json"
    query_path.parent.mkdir(parents=True,exist_ok=True)
    output=EV/f"{prefix}query-run.json"
    assert not output.exists() and not query_path.exists()
    command=[str(binary),"--definition-query-records",str(record),str(EV/names[0]),str(query_path)]
    log=EV/f"{prefix}queries.log"
    start=time.monotonic()
    with log.open("w",encoding="utf-8") as handle:
        code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
    result=dict(schema="dev.mercurio.fixed-chain-native-query-run.v1",qualification_certificate=False,
        input_sha256=inputs,consumer_sha256=build["input_sha256"],command=command,exit_code=code,
        elapsed_seconds=round(time.monotonic()-start,3),log_sha256=sha(log),
        fixed_context_denominator=26,fixed_getter_denominator=4462,fixed_ownership_getters=34,
        complete_native_contexts=0,strict_families_qualified=0,candidate_promoted=False)
    if code:
        result["status"]="native_query_failure";save(output,result);raise SystemExit(code)
    spec,ref,ids,requests=map(read,[EV/names[1],EV/names[2],EV/names[3],EV/names[0]])
    native=read(query_path)
    assert len(native["queries"])==4462
    comparison=compare(spec,ref,native,ids,requests)
    save(EV/f"{prefix}query-comparison.json",comparison)
    result.update(status="fixed_queries_compared",
        current_record_path=str(record),current_record_sha256=sha(record),
        current_query_path=str(query_path),current_query_sha256=sha(query_path),
        query_outcomes=dict(collections.Counter(p["status"] for p in native["queries"])),
        ownership_outcomes=dict(collections.Counter(p["comparison"] for p in comparison["ports"])),
        inputs_unchanged=all(sha(p)==s for p,s in inputs.items()))
    assert result["inputs_unchanged"]
    save(output,result)
    print(result["query_outcomes"],result["ownership_outcomes"],flush=True)
if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision",default="")
    parser.add_argument("--baseline",default="value-result-chain-pending-closure-run.json")
    args=parser.parse_args()
    run(args.revision,args.baseline)
