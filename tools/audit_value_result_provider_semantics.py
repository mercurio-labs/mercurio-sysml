"""Audit cached native provider getters against independently executed EMF getters.

This is component evidence, not a complete context/family certificate. The cached
graph's historical producer and the current query executable have separate pins.
"""
import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

from audit_value_result_provider_plan import native_identity

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "docs/conformance/2026-08-support/definition-pipeline-evidence"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read(name):
    return json.loads((EVIDENCE / name).read_text(encoding="utf-8"))


def compare_queries(spec, independent, native):
    requests = spec["inspection_queries"]
    observations = independent["observations"]
    answers = native["queries"]
    require(len(requests) == len(observations) == len(answers) > 0, "Query denominator differs")
    require(native["status"] == "read_only_diagnostic"
        and native["publication"] == "not_attempted"
        and native["semantic_qualification"] == "not_assessed"
        and native["qualification_certificate"] is False, "Native diagnostic became qualification")
    require(len({(q["owner_id"], q["field"]) for q in requests}) == len(requests), "Duplicate query request")
    rows = []
    for request, observation, answer in zip(requests, observations, answers):
        key = (request["owner_id"], request["field"])
        require(key == (observation["root"]["owner_id"], observation["root"]["field"])
            == (answer["owner_id"], answer["field"]), "Query order or identity differs")
        require(observation["status"] == "reference_observed", "Independent getter was not observed")
        require(observation["provider_completion_before"] is False
            and observation["provider_completion_after"] is False, "Independent provider was prepared")
        resolved = observation["feature_declaring_kind"] + "/" + observation["feature"]
        require(any(feature.endswith("#//" + resolved)
                    for feature in observation["root"]["feature_candidates"]), "Independent imported feature differs")
        row = {"owner_id": key[0], "independent_owner_name": observation["owner"]["qualified_name"],
               "field": key[1], "resolved_feature": resolved, "native_context_complete": False}
        if answer["status"] == "query_evaluated":
            try:
                expected = [(native_identity(endpoint["resource"], endpoint["emf_fragment"]), endpoint["kind"])
                            for endpoint in observation["endpoints"]]
            except ValueError as error:
                row.update(status="independent_identity_unavailable", dependency=str(error))
            else:
                actual = [(endpoint["id"], endpoint["kind"].split("::")[-1]) for endpoint in answer["targets"]]
                if actual == expected:
                    row.update(status="exact_ordered_match", endpoint_count=len(actual))
                else:
                    row.update(status="semantic_mismatch", expected_endpoint_count=len(expected),
                        actual_endpoint_count=len(actual), expected=expected, actual=actual)
        elif answer["status"] == "dependency_required":
            require(answer.get("dependency"), "Typed native prerequisite absent")
            row.update(status="typed_dependency_required", dependency=answer["dependency"])
        elif answer["status"] == "unavailable":
            require(answer.get("error"), "Unsupported native algorithm reason absent")
            row.update(status="native_algorithm_unavailable", dependency=answer["error"])
        else:
            raise ValueError("Unknown native query result")
        rows.append(row)
    return {"counts": dict(sorted(Counter(row["status"] for row in rows).items())), "queries": rows}



def audit_root_queries(native_run, graph, native_prefix):
    run = read(native_prefix + "-root-queries-run.json")
    require(run["exit_code"] == 0 and run["inputs_unchanged"]
        and run["source_model_sha256"] == sha(graph)
        and run["binary_sha256"] == native_run["binary_sha256"]
        and run["consumer_sha256"] == native_run["consumer_sha256"], "Current root query provenance differs")
    spec_path = EVIDENCE / (native_prefix + "-root-query-spec.json")
    output_path = EVIDENCE / (native_prefix + "-root-queries.json")
    require(sha(spec_path) == run["query_spec_sha256"] and sha(output_path) == run["output_sha256"],
        "Current root query inputs/outputs changed")
    original = read("value-result-provider-plan-actual-spec.json")["cases"][0]["dependency_roots"]
    spec = read(spec_path.name)["inspection_queries"]
    require(spec == original and len(spec) == 9, "Frozen provider root requests changed")
    reference_run = read("value-result-provider-plan-pilot-run.json")
    require(reference_run["inputs_unchanged"] and all(r["exit_code"] == 0 for r in reference_run["runs"]),
        "Independent root read execution failed")
    for path, digest in reference_run["input_sha256"].items():
        require(sha(path) == digest, "Independent root read input changed: " + path)
    reference_path = EVIDENCE / "value-result-provider-plan-pilot-observations.json"
    require(sha(reference_path) == reference_run["observations_sha256"], "Independent root read observations changed")
    observations = read(reference_path.name)["observations"]
    by_key = {(o["root"]["owner_id"], o["root"]["field"]): o for o in observations}
    require(len(by_key) == len(observations) == 9, "Independent root denominator differs")
    native = read(output_path.name)
    require(native["status"] == "read_only_diagnostic" and native["publication"] == "not_attempted"
        and native["semantic_qualification"] == "not_assessed" and native["qualification_certificate"] is False
        and native["constructed_elements_read"] == 93320
        and native["unfinished_reference_descriptors_retained"] == 18876
        and len(native["input_files"]) == 94 and len(native["queries"]) == 9,
        "Current root graph was pruned or promoted")
    verified = []
    for request, answer in zip(spec, native["queries"]):
        key = (request["owner_id"], request["field"])
        require(key == (answer["owner_id"], answer["field"]) and key in by_key, "Root query identity/order differs")
        observed = by_key[key]
        require(observed["status"] == "reference_observed"
            and not observed["provider_completion_before"] and not observed["provider_completion_after"],
            "Independent root provider was prepared")
        endpoint = observed["endpoint"]
        expected = [(native_identity(endpoint["resource"], endpoint["emf_fragment"]), endpoint["kind"])]
        require(answer["status"] == "query_evaluated", "Current provider root getter is not implemented")
        actual = [(target["id"], target["kind"].split("::")[-1]) for target in answer["targets"]]
        require(actual == expected, "Current provider root endpoint differs")
        verified.append({"owner_id": key[0], "field": key[1], "endpoint_id": actual[0][0],
                         "verification": "current_native_ordered_getter_and_independent_endpoint"})
    return {"current_native_provider_roots_verified": 9, "queries": verified,
        "query_run_sha256": sha(EVIDENCE / (native_prefix + "-root-queries-run.json")),
        "boundary": "Historical full-graph construction and scheduler execution are retained; current readonly native getter execution is independently revalidated. This does not replay or qualify complete provider lifecycles."}

def validate_direction_controls(data):
    require(data["schema"] == "dev.mercurio.direction-operation-reference.v1"
        and data["qualification_certificate"] is False
        and data["native_context_qualification"] == "not_assessed"
        and data["operation"] == "Type.directionOf(Feature)", "Direction component became qualification")
    rows = data["observations"]
    ordinary = {(owner, feature) for owner in ["G", "H", "C", "D"] for feature in ["i", "o", "b", "u"]}
    cycles = {("A", "i"), ("B", "i")}
    require(len(rows) == 18 and {(row["owner"], row["feature"]) for row in rows} == ordinary | cycles,
        "Direction control denominator or identity changed")
    for row in rows:
        cycle = (row["owner"], row["feature"]) in cycles
        require(row["supplied_materialized_operation_input"] is cycle
            and row["completion_before"] is cycle and row["completion_after"] is cycle,
            "Direction control lifecycle boundary changed")
        require(row["value"] in [None, "in", "out", "inout"], "Direction control value is not an enum literal")
    return {"controls": 18, "unprepared_controls": 16, "supplied_materialized_cycle_controls": 2,
        "native_contexts_qualified": 0}


def audit_direction_reference(native_prefix):
    run = read(native_prefix + "-direction-pilot-run.json")
    require(run["qualification_certificate"] is False and run["inputs_unchanged"]
        and all(row["exit_code"] == 0 for row in run["runs"]) and run["observed_queries"] == 18
        and run["state_changes"] == 0, "Independent direction operation failed or changed lifecycle")
    for path, digest in run["input_sha256"].items():
        require(sha(path) == digest, "Independent direction operation input changed: " + path)
    upstream = ROOT.parent / "target/upstream/SysML-v2-Release"
    require(len(run["pinned_library_source_sha256"]) == 94, "Independent direction environment changed")
    for path, digest in run["pinned_library_source_sha256"].items():
        require(sha(upstream / path) == digest, "Direction reference library changed: " + path)
    observation = EVIDENCE / (native_prefix + "-direction-controls.json")
    require(sha(observation) == run["observations_sha256"], "Independent direction observations changed")
    result = validate_direction_controls(read(observation.name))
    focused = read(native_prefix + "-focused-run.json")
    require(focused["exit_code"] == 0 and focused["inputs_unchanged"]
        and any(row["exit_code"] == 0 and "definition_provider_completion" in row["command"] for row in focused["runs"]),
        "Native direction batch is not verified")
    require(focused["consumer_sha256"].get(str(observation.relative_to(ROOT)).replace("\\", "/")) == sha(observation),
        "Native direction controls were not compiled from the independent cache")
    for path, digest in focused["consumer_sha256"].items():
        require(sha(ROOT / path) == digest, "Native direction consumer changed: " + path)
    result.update(independent_run_sha256=sha(EVIDENCE / (native_prefix + "-direction-pilot-run.json")),
        native_focused_run_sha256=sha(EVIDENCE / (native_prefix + "-focused-run.json")),
        boundary="All18 independent results are asserted by native controls; supplied materialized cycles cannot qualify source or provider lifecycle.")
    return result


def audit(check=False, native_prefix="value-result-provider-semantics"):
    scope = read("value-result-provider-semantics-batch-scope.json")
    contract = read("value-result-provider-semantics-read-contract.json")
    require(scope["fixed_contexts"] == 26 and scope["strict_families_required"] == 34
            and scope["complete_native_contexts"] == 0, "Acceptance denominator changed")
    graph = Path(scope["actual_model"])
    producer = read("value-result-parser-sharing-actual-run.json")
    require(producer["exit_code"] == 0 and producer["inputs_unchanged"], "Historical graph producer failed")
    require(sha(graph) == scope["actual_model_sha256"] == producer["inspection_sha256"]
            == contract["source_model_sha256"], "Frozen native graph changed")
    require(sha(EVIDENCE / "value-result-parser-sharing-actual-run.json") == scope["model_producer_run_sha256"],
            "Historical graph producer provenance changed")
    require(sha(ROOT / "docs/conformance/2026-08-support/value-result-obligation-bundle.json")
            == scope["obligation_bundle_sha256"], "Frozen acceptance bundle changed")
    native_run = read(native_prefix + "-native-run.json")
    require(native_run["exit_code"] == 0 and native_run["inputs_unchanged"], "Native cached query execution failed")
    require(native_run["source_model_sha256"] == sha(graph), "Native query graph differs")
    for path, digest in native_run["consumer_sha256"].items():
        require(sha(ROOT / path) == digest, "Current native query consumer changed: " + path)
    require(sha(ROOT / "target/release/audit_release_compile.exe") == native_run["binary_sha256"],
            "Current native query executable changed")
    native_path = EVIDENCE / (native_prefix + "-native-queries.json")
    require(sha(native_path) == native_run["output_sha256"], "Native queries changed")
    reference_run = read("value-result-provider-semantics-pilot-run.json")
    require(reference_run["inputs_unchanged"] and all(run["exit_code"] == 0 for run in reference_run["runs"]),
            "Independent execution failed")
    for path, digest in reference_run["input_sha256"].items():
        require(sha(path) == digest, "Independent input changed: " + path)
    upstream = ROOT.parent / "target/upstream/SysML-v2-Release"
    require(len(reference_run["pinned_library_source_sha256"]) == 94, "Independent resource closure changed")
    for path, digest in reference_run["pinned_library_source_sha256"].items():
        require(sha(upstream / path) == digest, "Pinned library changed: " + path)
    reference_path = EVIDENCE / "value-result-provider-semantics-pilot-observations.json"
    require(sha(reference_path) == reference_run["observations_sha256"], "Independent observations changed")
    query_path = EVIDENCE / "value-result-provider-semantics-query-spec.json"
    require(sha(query_path) == native_run["query_spec_sha256"], "Query spec changed")
    spec = read(query_path.name)
    independent = read(reference_path.name)
    native = read(native_path.name)
    require(len(spec["inspection_queries"]) == contract["read_queries"] == 115
            and len(contract["receivers"]) == 8, "Reviewed read matrix changed")
    require(native["constructed_elements_read"] == 93320
            and native["unfinished_reference_descriptors_retained"] == 18876
            and len(native["input_files"]) == 94, "Native graph was pruned")
    result = compare_queries(spec, independent, native)
    roots = audit_root_queries(native_run, graph, native_prefix)
    data = {"schema": "dev.mercurio.provider-semantic-component-comparison.v1",
            "qualification_certificate": False, "complete_native_contexts": 0, "strict_families_qualified": 0,
            "required_queries": 115, "receivers": 8, "constructed_elements": 93320,
            "unfinished_reference_descriptors_retained": 18876,
            "comparison": result, "current_root_reads": roots,
            "provenance": {"historical_graph_producer": scope["model_producer_run_sha256"],
                           "native_run": sha(EVIDENCE / (native_prefix + "-native-run.json")),
                           "independent_run": sha(EVIDENCE / "value-result-provider-semantics-pilot-run.json"),
                           "auditor": sha(Path(__file__))},
            "boundary": "Exact imported getter outputs are bounded component evidence; complete semantic dependency closure, validation, publication, persistence and all26 terminal contexts remain required."}
    if native_prefix == "value-result-provider-completion":
        data["direction_operation_controls"] = audit_direction_reference(native_prefix)
    output = EVIDENCE / (native_prefix + "-comparison.json")
    encoded = json.dumps(data, indent=2) + "\n"
    if check:
        require(output.read_text(encoding="utf-8") == encoded, "Provider semantic comparison is stale")
    else:
        output.write_text(encoded, encoding="utf-8")
    print(json.dumps({"queries": 115, "counts": result["counts"], "complete_native_contexts": 0}, indent=2))
    return data


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--native-prefix", choices=["value-result-provider-semantics","value-result-provider-completion"],
                        default="value-result-provider-semantics")
    args = parser.parse_args()
    audit(args.check, args.native_prefix)
