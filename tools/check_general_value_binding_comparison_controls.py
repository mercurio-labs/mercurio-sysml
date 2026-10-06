"""Adversarial controls for evidence comparison; no native model is changed."""
import copy, json
from pathlib import Path
from compare_general_value_bindings import compare, ROOT, EV, read, sha, save


def main():
    proof = EV / "general-value-bindings-fresh-result-witness-comparison.json"
    stage_path = EV / "general-value-bindings-canonical-controls-native-run.json"
    stage = read(stage_path); assert read(proof)["status"] == "native_value_binding_stage_compared_unqualified"
    record = Path(stage["current_record_path"])
    graph = read(record)["inspection"]["constructed_elements"]
    prior = read(Path(read(EV / "canonical-chain-result-resolved-sealed-progress.json")["current_record_path"]))["inspection"]["constructed_elements"]
    bundle = EV / "value-result-general-value-provider-dependency-bundle.json"
    reference = EV / "general-value-bindings-per-binding-reference-observations.json"
    identities = EV / "value-result-lifecycle-shared-services-source-identities.json"
    primary = Path(stage["query_path"])
    extra = ROOT / "target/general-value-bindings-canonical-controls-comparison/native-getters.json"
    result_path = ROOT / "target/general-value-bindings-fresh-result-witness-comparison/native-results.json"
    values = read(bundle)["valuations"]; refs = read(reference); ids = read(identities)["canonical_resource_fragment_to_native_id"]
    answers = read(primary)["queries"] + read(extra)["queries"]; results = read(result_path)["queries"]
    inputs = [proof, stage_path, record, bundle, reference, identities, primary, extra, result_path,
              Path(__file__), ROOT / "tools/compare_general_value_bindings.py"]
    witnesses = {str(path): sha(path) for path in inputs}
    def invoke(g=graph, r=refs, q=answers, result=results):
        return compare(g, prior, values, r, ids, result, q)
    baseline = invoke(); controls = []
    first = next(value for value in values if value["eligible_for_binding_stage"] and not value["is_default"])
    identity = first["valuation_id"] + ".implicit.value-binding"
    def altered_node(node_id, field, value):
        trial = list(graph)
        ordinal = next(i for i, node in enumerate(graph) if node["id"] == node_id)
        trial[ordinal] = copy.deepcopy(graph[ordinal]); trial[ordinal]["properties"][field] = value
        return trial
    def rejected(name, call):
        try:
            call()
        except (AssertionError, KeyError):
            controls.append(dict(name=name, outcome="rejected")); return
        raise AssertionError("Corruption was accepted: " + name)
    shortened = copy.deepcopy(refs); shortened["observations"].pop()
    rejected("missing fixed valuation cannot shrink denominator", lambda: invoke(r=shortened))
    source = next(node for node in graph if node["id"] == identity + ".source")
    reversed_graph = altered_node(source["id"], "owned_relationship", list(reversed(source["properties"]["owned_relationship"])))
    rejected("reordered normative value/result chain", lambda: invoke(g=reversed_graph))
    result_trial = copy.deepcopy(results)
    row = next(row for row in result_trial if row["owner_id"] == first["expression_id"])
    row["targets"][0]["id"] = first["owner_id"]
    rejected("result identity cannot be fabricated from physical chain", lambda: invoke(result=result_trial))
    bad_relation = copy.deepcopy(refs)
    frame = next(row for row in bad_relation["observations"] if row["control"]["valuation_id"] == first["valuation_id"])
    frame["bindings"][0]["effects"].append(dict(kind="UnknownDelegate", unassessed_relation=True))
    rejected("unknown upstream semantic effect", lambda: invoke(r=bad_relation))
    flags = invoke(g=altered_node(identity, "is_variable", True))
    changed = next(row for row in flags if row["valuation_id"] == first["valuation_id"])
    assert not changed["exact_stage_match"] and all(any(diff["path"] == "/connector/is_variable" for diff in frame["differences"]) for frame in changed["reference_frames"])
    controls.append(dict(name="fixed property flag mismatch retained", outcome="explicit_mismatch"))
    extra_binding = copy.deepcopy(refs)
    frame = next(row for row in extra_binding["observations"] if row["control"]["valuation_id"] == first["valuation_id"])
    frame["bindings"].append(copy.deepcopy(frame["bindings"][0]))
    changed = next(row for row in invoke(r=extra_binding) if row["valuation_id"] == first["valuation_id"])
    assert changed["reference_binding_count"] == len(changed["reference_frames"]) == 3 and not changed["binding_count_matches"] and not changed["exact_stage_match"]
    controls.append(dict(name="all duplicate Pilot frames retained", outcome="explicit_mismatch"))
    empty_query = copy.deepcopy(answers)
    query = next(row for row in empty_query if row["owner_id"] == identity + ".end.0" and row["field"] == "type")
    query.update(status="query_evaluated", targets=[]); query.pop("dependency", None)
    changed = next(row for row in invoke(q=empty_query) if row["valuation_id"] == first["valuation_id"])
    assert not next(row for row in changed["getters"] if row["owner_id"] == identity + ".end.0" and row["field"] == "type")["matches_every_reference"]
    controls.append(dict(name="empty result cannot replace required type", outcome="explicit_mismatch"))
    required = next(row for row in baseline if row["valuation_id"] == first["valuation_id"])
    assert any(row["native_status"] == "dependency_required" and not row["matches_every_reference"] for row in required["getters"])
    assert len([row for row in baseline if row["status"] == "required_link_negative_not_qualified"]) == 4
    controls.append(dict(name="required reads and negative controls remain unqualified", outcome="preserved"))
    assert len(controls) == 8 and all(sha(path) == digest for path, digest in witnesses.items())
    output = EV / "general-value-bindings-comparison-controls-run.json"; assert not output.exists()
    save(output, dict(schema="dev.mercurio.general-value-binding-comparison-controls.v1", qualification_certificate=False,
        status="passed", input_sha256=witnesses, inputs_unchanged=True, controls=controls, strict_families_qualified=0))
    print("8 adversarial comparison controls passed; native candidate unchanged", flush=True)


if __name__ == "__main__":
    main()
