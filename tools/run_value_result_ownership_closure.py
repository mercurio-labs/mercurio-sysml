"""Run one fixed shared bounds bundle through existing native queries/scheduler.

This build-time diagnostic bridge supplies no semantic answers. Native typed
requirements alone drive the bounded worklist; each write wave is integrity
checked and the fixed independent observations determine the exit.
"""
import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path
from audit_cached_native_dependencies import assess_successful_wave
from audit_value_result_ownership import compare, read, require

ROOT=Path(__file__).resolve().parents[1]
EV=ROOT/"docs/conformance/2026-08-support/definition-pipeline-evidence"
WORK=ROOT/"target/value-result-ownership/bounds-closure"
BINARY=ROOT/"target/release/audit_release_compile.exe"
PREFIX="value-result-ownership"
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,d):p.write_text(json.dumps(d,indent=2)+"\n",encoding="utf-8")

def run():
    WORK.mkdir(parents=True,exist_ok=True)
    output=EV/(PREFIX+"-bounds-closure-run.json")
    require(not output.exists(),"Existing closure run requires inspection; do not restart it blindly")
    spec=read(PREFIX+"-reference-spec.json")
    reference=read(PREFIX+"-reference-observations.json")
    identities=read("value-result-lifecycle-shared-services-source-identities.json")
    query_spec_path=EV/"value-result-lifecycle-reference-read-spec.json"
    requests=json.loads(query_spec_path.read_text(encoding="utf-8"))
    build=read(PREFIX+"-build-run.json")
    require(sha(BINARY)==build["binary_sha256"],"Current native executable differs")
    record=ROOT/"target/value-result-ownership/bounds-linked.jsonl"
    query_path=EV/(PREFIX+"-bounds-queries.json")
    initial=read(PREFIX+"-bounds-dependency-run.json")
    require(sha(record)==initial["output_sha256"] and initial["exit_code"]==0,
        "Initial native dependency wave is not verified")
    old_record=Path(initial["command"][2])
    before=json.loads(old_record.read_text(encoding="utf-8"))["inspection"]
    after=json.loads(record.read_text(encoding="utf-8"))
    initial_integrity=assess_successful_wave(before["constructed_elements"],before["pending_references"],
        after,json.loads(Path(initial["command"][3]).read_text(encoding="utf-8"))["requirements"])
    inputs={str(p):sha(p) for p in [BINARY,record,query_path,query_spec_path,
        EV/(PREFIX+"-reference-spec.json"),EV/(PREFIX+"-reference-observations.json"),Path(__file__),
        ROOT/"tools/audit_cached_native_dependencies.py",ROOT/"tools/audit_value_result_ownership.py"]}
    manifest=dict(schema="dev.mercurio.fixed-native-query-closure-run.v1",qualification_certificate=False,
        fixed_context_denominator=26,fixed_getter_denominator=4462,bound_getters_required=20,
        input_sha256=inputs,consumer_sha256=build["input_sha256"],initial_dependency_integrity=initial_integrity,
        waves=[],status="running",complete_native_contexts=0,strict_families_qualified=0,candidate_promoted=False,
        boundary="Native query/scheduler execution and exact ordered getter comparison. Full transformation, validation, publication, persistence and all26 terminal outcomes remain open.")
    start=time.monotonic()
    completed=set()
    for number in range(1,17):
        native=json.loads(query_path.read_text(encoding="utf-8"))
        comparison=compare(spec,reference,native,identities,requests)
        rows=[r for r in comparison["ports"] if r["algorithm"] in (
            "bound_expression_ownership","multiplicity_range_dispatch_and_featuring")]
        require(len(rows)==20,"Bound/range scope changed")
        matched=sum(r["comparison"]=="exact_ordered_match" for r in rows)
        manifest["current_bounds_matched"]=matched
        print("Bounds/ranges:",matched,"/20 ordered getter matches",flush=True)
        if matched==20:
            manifest.update(status="all20_bound_getters_independently_matched",
                final_record_path=str(record),final_record_sha256=sha(record),
                final_query_path=str(query_path),final_query_sha256=sha(query_path))
            save(EV/(PREFIX+"-bounds-closure-comparison.json"),comparison)
            break
        require(not any(r["comparison"] in ("semantic_mismatch","unavailable","reference_endpoint_unrepresented")
            for r in rows),"Required native algorithm or independent endpoint mismatch")
        answers={(q["owner_id"],q["field"]):q for q in native["queries"]}
        needed={}
        for row in rows:
            if row["comparison"]=="dependency_required":
                requirement=answers[row["owner_id"],row["field"]]["dependency"]["prerequisite"]
                signature=json.dumps(requirement,sort_keys=True)
                require(signature not in completed,"Completed native job was requested again")
                needed[signature]=requirement
        require(needed,"Incomplete getter bundle has no typed requirement")
        plan=WORK/f"wave-{number:02}-plan.json";save(plan,{"requirements":list(needed.values())})
        target=WORK/f"wave-{number:02}.jsonl";log=WORK/f"wave-{number:02}.log"
        command=[str(BINARY),"--definition-plan-records",str(record),str(plan),str(target)]
        input_hash=sha(record);t=time.monotonic()
        print("Native wave",number,":",len(needed),"shared typed prerequisites",flush=True)
        with log.open("w",encoding="utf-8") as f:
            code=subprocess.run(command,cwd=ROOT,stdout=f,stderr=subprocess.STDOUT).returncode
        wave=dict(number=number,requirements=list(needed.values()),command=command,exit_code=code,
            elapsed_seconds=round(time.monotonic()-t,3),input_sha256=input_hash,
            output_sha256=sha(target),log_sha256=sha(log))
        manifest["waves"].append(wave)
        require(code==0,"Native wave rejected; preserve its concrete dependency failure")
        prior=json.loads(record.read_text(encoding="utf-8"))["inspection"]
        result=json.loads(target.read_text(encoding="utf-8"))
        wave["integrity"]=assess_successful_wave(prior["constructed_elements"],prior["pending_references"],
            result,list(needed.values()))
        require(sha(record)==input_hash,"Native wave changed its input")
        completed.update(needed);record=target
        query_path=WORK/f"wave-{number:02}-queries.json"
        c=[str(BINARY),"--definition-query-records",str(record),str(query_spec_path),str(query_path)]
        code=subprocess.run(c,cwd=ROOT,capture_output=True).returncode
        require(code==0,"Native query execution failed")
        wave.update(query_command=c,query_sha256=sha(query_path))
        manifest["status"]="running"
        save(output,manifest)
    else:
        manifest["status"]="bounded_wave_limit_remaining_dependencies_explicit"
    manifest["inputs_unchanged"]=all(sha(p)==h for p,h in inputs.items()) and all(
        sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    manifest["elapsed_seconds"]=round(time.monotonic()-start,3)
    require(manifest["inputs_unchanged"],"Consumer/input fingerprints changed")
    save(output,manifest)
    require(manifest["status"]=="all20_bound_getters_independently_matched","Bundle remains incomplete")
    print("Verified:",manifest["current_bounds_matched"],"/20; complete contexts0/26; strict families0/34",flush=True)

if __name__=="__main__":run()
