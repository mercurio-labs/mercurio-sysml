"""Execute the retained chain bundle's independent pending reads.

The eight original roots and 34 ownership comparisons remain fixed. An existing
semantic mismatch stays visible while unrelated native dependencies are executed.
"""
import collections
import hashlib
import json
import subprocess
import time
from pathlib import Path
from audit_cached_native_dependencies import assess_successful_wave
from audit_value_result_ownership import compare

ROOT = Path(__file__).resolve().parents[1]
EV = ROOT / "docs/conformance/2026-08-support/definition-pipeline-evidence"
def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))
def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2)+"\n", encoding="utf-8")

def run():
    baseline_path = EV / "value-result-chain-closure-run.json"
    baseline = read(baseline_path)
    record, queries = Path(baseline["final_record_path"]), Path(baseline["final_query_path"])
    assert sha(record) == baseline["final_record_sha256"]
    assert sha(queries) == baseline["final_query_sha256"]
    contract_path = EV / "value-result-chain-closure-contract.json"
    contract = read(contract_path)
    assert contract["fixed_context_denominator"] == 26 and contract["required_getters"] == 8
    keys = {(p["owner_id"],p["field"]) for p in contract["ports"]}
    binary = ROOT / "target/release/audit_release_compile.exe"
    build_path = EV / "value-result-invocation-build-run.json"
    build = read(build_path)
    assert sha(binary) == build["binary_sha256"]
    assert all(sha(ROOT/p) == s for p,s in build["input_sha256"].items())
    spec_path = EV / "value-result-ownership-reference-spec.json"
    ref_path = EV / "value-result-ownership-reference-observations.json"
    ids_path = EV / "value-result-lifecycle-shared-services-source-identities.json"
    requests_path = EV / "value-result-lifecycle-reference-read-spec.json"
    spec, ref, ids, requests = map(read,[spec_path,ref_path,ids_path,requests_path])
    inputs = {str(p):sha(p) for p in [baseline_path,contract_path,build_path,binary,
        record,queries,spec_path,ref_path,ids_path,requests_path,Path(__file__),
        ROOT/"tools/audit_cached_native_dependencies.py",ROOT/"tools/audit_value_result_ownership.py"]}
    output = EV / "value-result-chain-pending-closure-run.json"
    assert not output.exists(),"Inspect an existing run instead of restarting it."
    work = ROOT / "target/value-result-chain-pending-closure"
    work.mkdir(parents=True,exist_ok=True)
    manifest = dict(schema="dev.mercurio.fixed-chain-pending-closure.v1",
        qualification_certificate=False,fixed_context_denominator=26,
        fixed_getter_denominator=4462,fixed_ownership_getters=34,
        required_getters=8,waves=[],status="running",input_sha256=inputs,
        consumer_sha256=build["input_sha256"],complete_native_contexts=0,
        strict_families_qualified=0,candidate_promoted=False)
    save(output,manifest)
    completed=set()
    for number in range(1,13):
        native=read(queries)
        comparison=compare(spec,ref,native,ids,requests)
        rows=[p for p in comparison["ports"] if (p["owner_id"],p["field"]) in keys]
        assert len(rows)==8
        manifest["current_getters_matched"]=sum(p["comparison"]=="exact_ordered_match" for p in rows)
        manifest["remaining"]=[p for p in rows if p["comparison"]!="exact_ordered_match"]
        print("Fixed chain bundle",manifest["current_getters_matched"],"/8",flush=True)
        answers={(p["owner_id"],p["field"]):p for p in native["queries"]}
        needed={}
        for row in rows:
            if row["comparison"]=="dependency_required":
                prerequisite=answers[(row["owner_id"],row["field"])]["dependency"]["prerequisite"]
                needed[json.dumps(prerequisite,sort_keys=True)]=prerequisite
        if not needed:
            manifest["status"]="all_fixed_getters_independently_matched" if not manifest["remaining"] else "precise_semantic_or_implementation_gap"
            save(EV/"value-result-chain-pending-closure-comparison.json",comparison)
            break
        if completed.intersection(needed):
            manifest["status"]="repeated_completed_requirement";break
        plan=work/f"wave-{number:02}-plan.json"
        save(plan,dict(requirements=list(needed.values())))
        target=work/f"wave-{number:02}.jsonl";log=work/f"wave-{number:02}.log"
        command=[str(binary),"--definition-plan-records",str(record),str(plan),str(target)]
        before=read(record)["inspection"];started=time.monotonic()
        with log.open("w",encoding="utf-8") as handle:
            code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
        wave=dict(number=number,requirements=list(needed.values()),command=command,
            exit_code=code,elapsed_seconds=round(time.monotonic()-started,3),
            input_record_sha256=sha(record),output_sha256=sha(target),log_sha256=sha(log))
        manifest["waves"].append(wave)
        print("Native dependency wave",number,code,flush=True)
        if code:
            manifest["status"]="native_dependency_bundle_rejected";manifest["failure"]=read(target);break
        wave["integrity"]=assess_successful_wave(before["constructed_elements"],
            before["pending_references"],read(target),list(needed.values()))
        next_queries=work/f"wave-{number:02}-queries.json"
        query_log=work/f"wave-{number:02}-queries.log"
        command=[str(binary),"--definition-query-records",str(target),str(requests_path),str(next_queries)]
        with query_log.open("w",encoding="utf-8") as handle:
            query_code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
        assert query_code==0
        wave.update(query_command=command,query_output_sha256=sha(next_queries),query_log_sha256=sha(query_log))
        completed.update(needed);record,queries=target,next_queries
        save(output,manifest)
    else:
        manifest["status"]="bounded_wave_limit_reached"
    manifest.update(final_record_path=str(record),final_record_sha256=sha(record),
        final_query_path=str(queries),final_query_sha256=sha(queries),
        inputs_unchanged=all(sha(p)==s for p,s in inputs.items()),
        query_outcomes=dict(collections.Counter(p["status"] for p in read(queries)["queries"])))
    assert manifest["inputs_unchanged"]
    save(output,manifest)
    print(manifest["status"],flush=True)
if __name__=="__main__":
    run()
