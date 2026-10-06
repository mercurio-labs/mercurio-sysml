"""Verify fixed reads and independent featuring projections at the shared batch boundary."""
from pathlib import Path
import json,hashlib,collections,subprocess,time,argparse
from audit_value_result_ownership import compare
from audit_value_result_provider_plan import native_identity
ROOT=Path(__file__).resolve().parents[1]
EV=ROOT/"docs/conformance/2026-08-support/definition-pipeline-evidence"
def read(p):return json.loads(Path(p).read_text(encoding="utf-8"))
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2)+"\n",encoding="utf-8")
def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--build-revision",required=True);parser.add_argument("--execution-revision",required=True);args=parser.parse_args()
    execution=EV/("value-result-expression-featuring-"+args.execution_revision+"-execution-run.json");prior=read(execution)
    assert prior["status"]=="physical_projection_compared" and prior["expressions_matched"]==48 and prior["inputs_unchanged"]
    build_path=EV/("value-result-expression-featuring-"+args.build_revision+"-build-run.json");build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"]
    record=Path(prior["current_record_path"]);assert sha(record)==prior["current_record_sha256"]
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    reference_path=EV/"value-result-chain-transformation-resolved-reference-observations.json"
    ref_run=read(EV/"value-result-chain-transformation-resolved-reference-run.json")
    assert sha(reference_path)==ref_run["output_sha256"]
    names=["value-result-lifecycle-reference-read-spec.json","value-result-ownership-reference-spec.json",
           "value-result-ownership-reference-observations.json","value-result-lifecycle-shared-services-source-identities.json"]
    featuring_reference=EV/"value-result-expression-featuring-full-cache-observations.json"
    baseline_path=EV/"value-result-reference-binding-progress.json";baseline=read(baseline_path)
    baseline_queries=Path(baseline["current_query_path"]);assert sha(baseline_queries)==baseline["current_query_sha256"]
    files=[record,execution,build_path,binary,reference_path,featuring_reference,baseline_path,baseline_queries,Path(__file__)]+[EV/name for name in names]
    inputs={str(p):sha(p) for p in files}
    folder=ROOT/("target/value-result-expression-featuring-"+args.revision+"-queries");folder.mkdir(exist_ok=False)
    queries=folder/"queries.json";out=EV/("value-result-expression-featuring-"+args.revision+"-query-run.json")
    assert not queries.exists() and not out.exists()
    command=[str(binary),"--definition-query-records",str(record),str(EV/names[0]),str(queries)]
    log=folder/"queries.log";start=time.monotonic()
    manifest=dict(schema="dev.mercurio.expression-featuring-stage-queries.v1",qualification_certificate=False,status="running",
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
    save(EV/("value-result-expression-featuring-"+args.revision+"-cold-reference-comparison.json"),matrix)
    by_key={(row["owner_id"],row["field"]):row for row in native["queries"]}
    before_by_key={(row["owner_id"],row["field"]):row for row in read(baseline_queries)["queries"]}
    assert by_key.keys()==before_by_key.keys()
    newly_unavailable=[dict(owner_id=row["owner_id"],field=row["field"],status=row["status"],error=row.get("error"))
        for key,row in by_key.items() if row["status"]=="unavailable" and before_by_key[key]["status"]!="unavailable"]
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
    featuring=[]
    for ref in read(featuring_reference)["expressions"]:
        actual=by_key[(ref["owner_id"],"featuring_type")]
        observed=[target["id"] for target in actual.get("targets",[])]
        featuring.append(dict(context=ref["context"],owner_id=ref["owner_id"],native_status=actual["status"],actual=observed,expected=ref["expected"],
            exact_ordered_match=actual["status"]=="query_evaluated" and observed==ref["expected"]))
    assert len(featuring)==48
    projection_path=EV/("value-result-expression-featuring-"+args.revision+"-getter-comparison.json")
    save(projection_path,dict(schema="dev.mercurio.expression-featuring-getter-comparison.v1",qualification_certificate=False,
        accepted_context_projections=featuring,required_expressions=56,complete_native_contexts=0,
        boundary="Only physical featuring and its resolved getter; complete context lifecycle remains open."))
    save(EV/("value-result-expression-featuring-"+args.revision+"-stage-getter-comparison.json"),dict(
        schema="dev.mercurio.explicit-chain-stage-getter-comparison.v1",qualification_certificate=False,
        required_getters=8,getters=stage,comparison_boundary="Retained explicit chain getter controls after shared expression featuring construction; no whole-model qualification."))
    manifest.update(status="stage_read_regression" if newly_unavailable else "stage_compared",newly_unavailable_reads=newly_unavailable,stage_getters_matched=sum(row["exact_ordered_match"] for row in stage),
        featuring_getters_matched=sum(row["exact_ordered_match"] for row in featuring),
        featuring_comparison_path=str(projection_path),featuring_comparison_sha256=sha(projection_path),
        current_record_path=str(record),current_record_sha256=sha(record),current_query_path=str(queries),current_query_sha256=sha(queries),
        query_outcomes=dict(collections.Counter(row["status"] for row in native["queries"])),
        cold_reference_outcomes=dict(collections.Counter(row["comparison"] for row in matrix["ports"])),
        cold_reference_boundary="Original cold getter reference retained; transformed chain contexts require the independent phase-specific comparison.",
        inputs_unchanged=all(sha(p)==h for p,h in inputs.items()))
    save(out,manifest)
    print("phase getters",manifest["stage_getters_matched"],"/8; featuring",manifest["featuring_getters_matched"],"/48; fixed matrix",manifest["query_outcomes"],flush=True)
    assert manifest["inputs_unchanged"] and manifest["stage_getters_matched"]==8 and manifest["featuring_getters_matched"]==48 and not newly_unavailable
if __name__=="__main__":main()
