"""Verify fixed source reads and phase-aligned chain getter outputs."""
from pathlib import Path
import json,hashlib,collections,subprocess,time
from audit_value_result_ownership import compare
from audit_value_result_provider_plan import native_identity
ROOT=Path(__file__).resolve().parents[1]
EV=ROOT/"docs/conformance/2026-08-support/definition-pipeline-evidence"
def read(p):return json.loads(Path(p).read_text(encoding="utf-8"))
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2)+"\n",encoding="utf-8")
def main():
    execution=EV/"value-result-chain-transformation-execution-run.json";prior=read(execution)
    assert prior["status"]=="physical_bundle_compared" and prior["contexts_matched"]==4 and prior["inputs_unchanged"]
    build_path=EV/"value-result-chain-transformation-initial-build-run.json";build=read(build_path)
    record=Path(prior["current_record_path"]);assert sha(record)==prior["current_record_sha256"]
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    reference_path=EV/"value-result-chain-transformation-resolved-reference-observations.json"
    ref_run=read(EV/"value-result-chain-transformation-resolved-reference-run.json")
    assert sha(reference_path)==ref_run["output_sha256"]
    names=["value-result-lifecycle-reference-read-spec.json","value-result-ownership-reference-spec.json",
           "value-result-ownership-reference-observations.json","value-result-lifecycle-shared-services-source-identities.json"]
    files=[record,execution,build_path,binary,reference_path,Path(__file__)]+[EV/name for name in names]
    inputs={str(p):sha(p) for p in files}
    folder=ROOT/"target/value-result-chain-transformation"
    queries=folder/"queries.json";out=EV/"value-result-chain-transformation-query-run.json"
    assert not queries.exists() and not out.exists()
    command=[str(binary),"--definition-query-records",str(record),str(EV/names[0]),str(queries)]
    log=folder/"queries.log";start=time.monotonic()
    manifest=dict(schema="dev.mercurio.explicit-chain-stage-queries.v1",qualification_certificate=False,status="running",
        fixed_contexts=26,fixed_queries=4462,required_phase_getters=8,strict_families_qualified=0,
        input_sha256=inputs,consumer_sha256=build["input_sha256"],command=command)
    save(out,manifest)
    with log.open("w",encoding="utf-8") as handle:
        code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
    manifest.update(exit_code=code,elapsed_seconds=round(time.monotonic()-start,3),log_sha256=sha(log))
    if code:
        manifest["status"]="failed";save(out,manifest);raise SystemExit(code)
    native=read(queries);assert len(native["queries"])==4462
    matrix=compare(read(EV/names[1]),read(EV/names[2]),native,read(EV/names[3]),read(EV/names[0]))
    save(EV/"value-result-chain-transformation-cold-reference-comparison.json",matrix)
    by_key={(row["owner_id"],row["field"]):row for row in native["queries"]}
    stage=[]
    for context in read(reference_path)["observations"]:
        for ref in context["parameter_getters"]:
            field={"inheritedMembership":"inherited_membership","type":"type"}[ref["field"]]
            actual=by_key[(context["control"]["parameter_id"],field)]
            assert ref["status"]=="reference_observed"
            expected=[]
            for endpoint in ref["endpoints"]:
                assert endpoint["resource"].startswith("sysml.library/"),"Explicit mapping review needed for a generated source endpoint"
                expected.append(dict(id=native_identity(endpoint["resource"],endpoint["emf_fragment"]),kind="SysML::"+endpoint["kind"]))
            observed=[dict(id=t["id"],kind=t["kind"]) for t in actual.get("targets",[])]
            stage.append(dict(context=context["control"]["context"],owner_id=context["control"]["parameter_id"],field=field,
                native_status=actual["status"],actual=observed,expected=expected,
                exact_ordered_match=actual["status"]=="query_evaluated" and observed==expected))
    assert len(stage)==8
    save(EV/"value-result-chain-transformation-stage-getter-comparison.json",dict(
        schema="dev.mercurio.explicit-chain-stage-getter-comparison.v1",qualification_certificate=False,
        required_getters=8,getters=stage,comparison_boundary="Same explicit additional-member and two constraint-producer stage; no whole-model qualification."))
    manifest.update(status="stage_compared",stage_getters_matched=sum(row["exact_ordered_match"] for row in stage),
        current_record_path=str(record),current_record_sha256=sha(record),current_query_path=str(queries),current_query_sha256=sha(queries),
        query_outcomes=dict(collections.Counter(row["status"] for row in native["queries"])),
        cold_reference_outcomes=dict(collections.Counter(row["comparison"] for row in matrix["ports"])),
        cold_reference_boundary="Original cold getter reference retained; transformed chain contexts require the independent phase-specific comparison.",
        inputs_unchanged=all(sha(p)==h for p,h in inputs.items()))
    save(out,manifest)
    print("phase getters",manifest["stage_getters_matched"],"/8; fixed matrix",manifest["query_outcomes"],flush=True)
    assert manifest["inputs_unchanged"] and manifest["stage_getters_matched"]==8
if __name__=="__main__":main()
