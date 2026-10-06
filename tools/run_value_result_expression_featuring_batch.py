"""Execute the fixed shared featuring batch over all retained real libraries.

Accepted-context physical projections, negative outcomes, and whole lifecycle
qualification remain separate. The fixed value-binding bundle is not narrowed.
"""
from pathlib import Path
import argparse, collections, json, subprocess, time
from audit_cached_native_dependencies import assess_successful_wave, exact_json
from export_value_result_expression_featuring_reference import ROOT, EV, OUTPUT, read, sha, extract

def save(path, value):
    path.write_text(json.dumps(value,indent=2)+"\n",encoding="utf-8")

def compare(graph, reference):
    index = {node["id"]:node for node in graph}
    rows = []
    for observed in reference["expressions"]:
        owner = index[observed["owner_id"]]
        actual = []
        for relation_id in owner["properties"].get("owned_relationship",[]):
            relation = index[relation_id]
            if relation["kind"].rsplit("::",1)[-1] != "TypeFeaturing":
                continue
            assert relation["properties"]["feature_of_type"] == owner["id"]
            assert relation["properties"]["owning_related_element"] == owner["id"]
            target = relation["properties"]["featuring_type"]
            assert target in index
            actual.append(dict(kind="TypeFeaturing",target_id=target,is_implied=relation["properties"].get("is_implied",False)))
        rows.append(dict(context=observed["context"],owner_id=owner["id"],actual=actual,expected=observed["physical"],
            original_owner_complete=owner["properties"].get("is_implied_included",False),
            exact_ordered_match=actual==observed["physical"] and not owner["properties"].get("is_implied_included",False)))
    assert len(rows)==48
    return rows

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--build-revision",required=True);parser.add_argument("--replay-revision");args=parser.parse_args()
    build_path=EV/("value-result-expression-featuring-"+args.build_revision+"-build-run.json");build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"]
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    spec_path=EV/"value-result-value-binding-bundle-spec.json";spec=read(spec_path)
    assert spec["required_expressions"]==56 and len(spec["expressions"])==56
    assert spec["fixed_contexts"]==26 and spec["fixed_resources"]==119 and spec["fixed_libraries"]==94
    assert all(sha(p)==h for p,h in spec["input_sha256"].items())
    reference=extract();assert read(OUTPUT)==reference
    record=Path(spec["retained_record_path"]);assert sha(record)==spec["retained_record_sha256"]
    extra=[]
    if args.replay_revision:
        previous_path=EV/("value-result-expression-featuring-"+args.replay_revision+"-execution-run.json");previous=read(previous_path)
        assert previous["status"]=="physical_projection_compared" and previous["expressions_matched"]==48 and previous["inputs_unchanged"]
        record=Path(previous["current_record_path"]);assert sha(record)==previous["current_record_sha256"]
        assert all(sha(p)==h for p,h in previous["input_sha256"].items())
        extra.append(previous_path)
    original=read(record);prior=original["inspection"]
    assert len(original["input_files"])==119
    ids=sorted(row["owner_id"] for row in spec["expressions"] if not row["link_negative"])
    assert len(ids)==52 and len(set(ids))==52
    roots=[dict(kind="expression_featuring_batch",owner_ids=ids)]
    folder=ROOT/("target/value-result-expression-featuring-"+args.revision);folder.mkdir(exist_ok=False)
    plan=folder/"requirements.json";save(plan,dict(requirements=roots));target=folder/"transformed.jsonl";log=folder/"transformation.log"
    run_path=EV/("value-result-expression-featuring-"+args.revision+"-execution-run.json");assert not run_path.exists()
    files=[build_path,binary,spec_path,OUTPUT,record,plan,Path(__file__)]+extra
    inputs={str(p):sha(p) for p in files}
    inputs.update(reference["input_sha256"])
    command=[str(binary),"--definition-plan-records",str(record),str(plan),str(target)]
    manifest=dict(schema="dev.mercurio.expression-featuring-execution.v1",qualification_certificate=False,status="running",
        input_sha256=inputs,consumer_sha256=build["input_sha256"],command=command,
        fixed_contexts=26,fixed_resources=119,fixed_libraries=94,required_expressions=56,eligible_expressions=52,
        cached_full_expression_projections=48,required_link_negative_expressions=4,required_negative_context_expressions=8,
        complete_native_contexts=0,strict_families_qualified=0,candidate_promoted=False,
        boundary="One physical featuring projection, not completed expression/value transformation, validation, publication or terminal persistence qualification.")
    save(run_path,manifest);start=time.monotonic()
    with log.open("w",encoding="utf-8") as handle:
        code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
    manifest.update(exit_code=code,elapsed_seconds=round(time.monotonic()-start,3),log_path=str(log),log_sha256=sha(log),
        output_path=str(target),output_sha256=sha(target) if target.exists() else None,
        inputs_unchanged=all(sha(p)==h for p,h in inputs.items()) and all(sha(ROOT/p)==h for p,h in build["input_sha256"].items()))
    if code:
        manifest.update(status="native_bundle_rejected",failure=read(target) if target.exists() else log.read_text(encoding="utf-8")[-5000:])
        save(run_path,manifest);print("Native featuring bundle rejected:",str(manifest["failure"])[:5500],flush=True);raise SystemExit(code)
    native=read(target);assert len(native["input_files"])==119
    graph=native["inspection"]["constructed_elements"]
    integrity=assess_successful_wave(prior["constructed_elements"],prior["pending_references"],native,roots)
    index={node["id"]:node for node in graph};prior_index={node["id"]:node for node in prior["constructed_elements"]}
    assert all(index[row["owner_id"]]["properties"].get("is_implied_included",False) is False for row in spec["expressions"])
    assert all(exact_json(index[row["owner_id"]],prior_index[row["owner_id"]]) for row in spec["expressions"] if row["link_negative"])
    assert len(native["committed_reference_fields"])==0
    rows=compare(graph,reference);matched=sum(row["exact_ordered_match"] for row in rows)
    comparison=EV/("value-result-expression-featuring-"+args.revision+"-structural-comparison.json")
    save(comparison,dict(schema="dev.mercurio.expression-featuring-structural-comparison.v1",qualification_certificate=False,
        required_expressions=56,eligible_expressions=52,accepted_context_projections=rows,
        required_negative_expressions=reference["required_negative_expressions"],enclosing_transform_validation_publication="not_assessed"))
    manifest.update(status="physical_projection_compared",integrity=integrity,expressions_matched=matched,
        nodes=len(graph),pending_references=len(native["inspection"]["pending_references"]),
        current_record_path=str(target),current_record_sha256=sha(target),comparison_path=str(comparison),comparison_sha256=sha(comparison))
    if args.replay_revision:
        assert integrity["native_nodes_added"]==0 and integrity["native_ports_committed"]==0
        assert exact_json(graph,prior["constructed_elements"])
        assert native["inspection"]["pending_references"]==prior["pending_references"]
        manifest["graph_unchanged"]=True
    save(run_path,manifest)
    print("Physical featuring matched",matched,"/48; eligible original expressions 52/56;",integrity,flush=True)
    assert manifest["inputs_unchanged"] and matched==48
if __name__=="__main__":main()
