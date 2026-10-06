"""Seal current expression-stage evidence without claiming family qualification."""
from pathlib import Path
import json,hashlib,collections
ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT/"docs/conformance/2026-08-support"
EV=BASE/"definition-pipeline-evidence"
def read(p):return json.loads(Path(p).read_text(encoding="utf-8"))
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2)+"\n",encoding="utf-8")
def main():
    names={phase:EV/("value-result-expression-contribution-repaired-"+phase+"-run.json")
           for phase in ["focused","build","regression","execution","query"]}
    runs={phase:read(p) for phase,p in names.items()}
    for phase in ["focused","build","regression"]:
        run=runs[phase]
        assert run["status"]=="passed" and run["inputs_unchanged"],phase
        assert sha(run["binary_path"])==run["binary_sha256"],phase
        assert sha(ROOT/("tools/run_value_result_expression_contribution_repair_checks.py"))==run["runner_sha256"]
        assert all(sha(ROOT/p)==h for p,h in run["input_sha256"].items()),phase
        assert all(r["exit_code"]==0 and r["required_count_observed"] and sha(r["log_path"])==r["log_sha256"] for r in run["runs"])
    assert [r["required_test_count"] for r in runs["focused"]["runs"]]==[4,1]
    assert [r["required_test_count"] for r in runs["build"]["runs"]]==[13,None]
    assert [r["required_test_count"] for r in runs["regression"]["runs"]]==[506]
    execution=runs["execution"];queries=runs["query"]
    assert execution["status"]=="physical_bundle_compared" and execution["expressions_matched"]==52
    assert queries["status"]=="stage_compared" and queries["stage_getters_matched"]==8
    for run in [execution,queries]:
        assert run["inputs_unchanged"] and all(sha(p)==h for p,h in run["input_sha256"].items())
        assert all(sha(ROOT/p)==h for p,h in run["consumer_sha256"].items())
    record=Path(execution["current_record_path"])
    assert sha(record)==execution["current_record_sha256"]==queries["current_record_sha256"]
    query=read(queries["current_query_path"]);assert sha(queries["current_query_path"])==queries["current_query_sha256"]
    assert len(query["queries"])==4462 and set(queries["query_outcomes"]) <= {"query_evaluated","dependency_required"}
    comparison_path=EV/"value-result-expression-contribution-repaired-structural-comparison.json"
    comparison=read(comparison_path);rows=comparison["expressions"]
    assert len(rows)==56 and sum(row.get("exact_ordered_match",False) for row in rows)==52
    assert sum(row["status"]=="link_negative_not_qualified" for row in rows)==4
    assert sum(len(row.get("expected",[])) for row in rows)==71
    identity_map=read(EV/"value-result-lifecycle-shared-services-source-identities.json")["canonical_resource_fragment_to_native_id"]
    assert len(identity_map)==388 and all(row["owner_id"] in identity_map.values() for row in rows)
    model=read(record);inspection=model["inspection"]
    assert len(model["input_files"])==119
    assert model["element_count"]==len(inspection["constructed_elements"])==93888
    assert model["pending_reference_count"]==len(inspection["pending_references"])==18865
    nodes={node["id"]:node for node in inspection["constructed_elements"]}
    assert all(nodes[row["owner_id"]]["properties"].get("is_implied_included",False) is False for row in rows)
    integrity=read(EV/"value-result-invocation-input-integrity.json")
    release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert len(integrity["library_sha256"])==94 and all(sha(release/p)==h for p,h in integrity["library_sha256"].items())
    assert len(integrity["frozen_acceptance_sha256"])==4 and all(sha(BASE/p)==h for p,h in integrity["frozen_acceptance_sha256"].items())
    ref_path=EV/"value-result-expression-contribution-resolved-reference-run.json";reference=read(ref_path)
    assert reference["status"]=="reference_observed" and reference["inputs_unchanged"]
    assert sha(reference["output_path"])==reference["output_sha256"]
    assert all(sha(p)==h for p,h in reference["input_sha256"].items())
    additional=[comparison_path,Path(queries["current_query_path"]),
                EV/"value-result-expression-contribution-repaired-stage-getter-comparison.json",
                EV/"value-result-expression-contribution-repaired-cold-reference-comparison.json",
                EV/"value-result-expression-contribution-repaired-spec.json",ref_path,
                Path(reference["output_path"]),EV/"value-result-invocation-input-integrity.json",
                EV/"value-result-lifecycle-shared-services-source-identities.json",Path(__file__)]
    files=list(names.values())+additional
    progress=dict(schema="dev.mercurio.expression-contribution-progress.v1",qualification_certificate=False,
                  status="bounded_shared_stage_verified",consumer_sha256=execution["consumer_sha256"],
                  input_sha256={str(p):sha(p) for p in files},
                  fixed_contexts=26,fixed_original_source_identities=388,fixed_queries=4462,
                  required_expressions=56,eligible_expressions=52,expressions_matched=52,
                  required_link_negative_expressions=4,link_negative_expressions_qualified=0,
                  required_kinds=8,kind_counts=dict(collections.Counter(row["owner_kind"] for row in rows)),
                  ordered_physical_effects_matched=71,retained_chain_getters_matched=8,
                  native_definition_controls_passed=506,focused_controls_passed=5,tooling_controls_passed=13,
                  current_record_path=str(record),current_record_sha256=sha(record),
                  current_query_path=queries["current_query_path"],current_query_sha256=queries["current_query_sha256"],
                  native_nodes=93888,pending_references=18865,integrity=execution["integrity"],
                  query_outcomes=queries["query_outcomes"],cold_reference_outcomes=queries["cold_reference_outcomes"],
                  library_sha256=integrity["library_sha256"],frozen_acceptance_sha256=integrity["frozen_acceptance_sha256"],
                  complete_native_contexts=0,reviewed_obligations_closed=0,value_services_qualified=0,
                  strict_families_qualified=0,release_gates_qualified=0,candidate_promoted=False,
                  implementation="Imported default/instantiation programs and Ecore contracts drive shared native selection, explicit typed member/reference/chain prerequisites and atomic specialization construction. Scheduler, readiness, transactional storage and admission algorithms are explicit handwritten Rust.",
                  completed_stage="Ordered expression default contributions and prerequisite result/member structure across every eligible original expression; all completion flags stay false.",
                  remaining_dependencies=["value/result/self-result connectors and their full semantic dependencies",
                    "complete featuring and parameter transformations","bound and variability algorithms",
                    "scoped validation, including all required negative outcomes and Ecore mutations",
                    "Closed final publication","fresh whole-context persistence",
                    "all 26 independent terminal comparisons","remaining B1/B2/B3 obligations, 310 samples and final semantic/timing qualification"],
                  boundary="52/52 component-stage matches are not complete contexts, services, strict families or release support. Four link-negative expressions retain their required terminal obligations.",
                  celebration="Pending first accepted strict family certificate.")
    out=EV/"value-result-expression-contribution-progress.json";assert not out.exists();save(out,progress)
    print("Sealed: 52/52 expressions, 71 ordered effects, 8/8 retained chain getters; 506 native, 5 focused, 13 tooling controls.")
    print("Strict closure remains: 0/26 contexts, 0/10 B1 obligations, 0/1 services, 0/34 families, 0/5 gates.")
if __name__=="__main__":main()
