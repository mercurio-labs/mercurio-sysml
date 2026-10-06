"""Exact, ordered ownership-getter evidence. This never qualifies a lifecycle."""
import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path
from audit_value_result_provider_plan import native_identity

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "docs/conformance/2026-08-support/definition-pipeline-evidence"
PREFIX = "value-result-ownership"

def require(condition, message):
    if not condition:
        raise ValueError(message)

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def read(name):
    return json.loads((EVIDENCE / name).read_text(encoding="utf-8"))

def compare(spec, reference, native, identities, requests):
    ports = spec["ports"]
    require(spec["fixed_contexts"] == 26 and spec["source_contexts_parsed"] == 25
        and spec["mutations_applied"] is False, "Frozen context contracts changed")
    require(len(ports) == len(reference["observations"]) == 34
        and len({(p["owner_id"], p["field"]) for p in ports}) == 34, "All34 ownership ports are required")
    require(Counter(p["strategy_group"] for p in ports) == {
        "bound_expression_ownership": 16, "multiplicity_range_dispatch_and_featuring": 4,
        "invocation_parameter_ownership": 4, "registered_chain_producer_readiness": 10},
        "Shared algorithm scope changed")
    require(native["qualification_certificate"] is False and native["status"] == "read_only_diagnostic"
        and native["publication"] == "not_attempted" and native["semantic_qualification"] == "not_assessed",
        "Read outcomes cannot be promoted to full support")
    require(len(native["queries"]) == len(requests["inspection_queries"]) == 4462,
        "Full native read denominator changed")
    key = lambda q: (q["owner_id"], q["field"])
    require([key(q) for q in native["queries"]] == [key(q) for q in requests["inspection_queries"]],
        "Fixed native query order changed")
    answers = {key(q): q for q in native["queries"]}
    require(len(answers) == 4462, "Duplicate native query")
    mapping = identities["canonical_resource_fragment_to_native_id"]
    require(identities["nodes_mapped"] == len(mapping) == 388
        and identities["source_contexts_mapped"] == 25, "Canonical source map changed")
    def endpoint_id(endpoint):
        identity = endpoint["resource"].replace(chr(92), "/") + "#" + endpoint["emf_fragment"]
        if identity in mapping:
            return mapping[identity]
        if endpoint["resource"].startswith("sysml.library/"):
            return native_identity(endpoint["resource"], endpoint["emf_fragment"])
        return None
    require(len(reference["initial_source_node_counts"]) == len(reference["final_source_node_counts"]) == 25
        and sum(reference["initial_source_node_counts"].values()) == 388,
        "Independent original source environment changed")
    rows = []
    for port, observed in zip(ports, reference["observations"]):
        require(observed["root"] == port and observed["status"] == "reference_observed",
            "Independent observation identity, order or outcome changed")
        require(observed["provider_completion_before"] is False and observed["provider_completion_after"] is False,
            "Prepared upstream completion flags")
        require(endpoint_id(observed["owner"]) == port["owner_id"]
            and observed["owner"]["kind"] == port["owner_kind"].split("::")[-1], "Canonical owner differs")
        resolved = observed["feature_declaring_kind"] + "/" + observed["feature"]
        require(any(f.endswith("#//" + resolved) for f in port["feature_candidates"]),
            "Getter is not the imported feature")
        require(observed["source_nodes_after"] >= observed["source_nodes_before"]
            and observed["owner_relationships_after"] >= observed["owner_relationships_before"],
            "Unexpected upstream destructive getter")
        answer = answers[key(port)]
        expected_ids = [endpoint_id(t) for t in observed["endpoints"]]
        expected_kinds = [t["kind"] for t in observed["endpoints"]]
        if answer["status"] != "query_evaluated":
            outcome = answer["status"]
        elif any(t is None for t in expected_ids):
            outcome = "reference_endpoint_unrepresented"
        elif [(t["id"], t["kind"].split("::")[-1]) for t in answer["targets"]] != list(zip(expected_ids, expected_kinds)):
            outcome = "semantic_mismatch"
        else:
            outcome = "exact_ordered_match"
        row = dict(context=port["context"], owner_id=port["owner_id"], field=port["field"],
            algorithm=port["strategy_group"], comparison=outcome, expected_endpoint_count=len(expected_ids),
            reference_source_nodes_added=observed["source_nodes_after"]-observed["source_nodes_before"],
            complete_native_context=False)
        if outcome == "semantic_mismatch":
            row.update(expected=list(zip(expected_ids,expected_kinds)), actual=answer["targets"])
        if answer["status"] != "query_evaluated":
            row["dependency"] = answer.get("prerequisite", answer.get("message", answer.get("error")))
        rows.append(row)
    return dict(schema="dev.mercurio.shared-ownership-getter-comparison.v1", qualification_certificate=False,
        counts=dict(Counter(r["comparison"] for r in rows)),
        by_algorithm={a:dict(Counter(r["comparison"] for r in rows if r["algorithm"]==a))
            for a in sorted({r["algorithm"] for r in rows})}, ports=rows,
        reference_source_nodes_added=sum(r["reference_source_nodes_added"] for r in rows),
        complete_native_contexts=0, strict_families_qualified=0, candidate_promoted=False,
        boundary="Exact ordered getters are component evidence. All26 terminal contexts, native construction effects, transformation, validation, publication and persistence remain separate requirements.")

def audit(check=False):
    reference_run = read(PREFIX+"-reference-run.json")
    native_run = read(PREFIX+"-post-construction-queries-run.json")
    for run in [reference_run,native_run]:
        require(run["exit_code"] == 0 and run["inputs_unchanged"] is True, "Execution failed or inputs changed")
        for path, digest in run["input_sha256"].items():
            require(sha(ROOT/path) == digest, "Pinned run input changed: "+path)
    require(sha(EVIDENCE/(PREFIX+"-reference-observations.json")) == reference_run["output_sha256"],
        "Independent observations changed")
    require(sha(EVIDENCE/(PREFIX+"-post-construction-queries.json")) == native_run["output_sha256"],
        "Native observations changed")
    for path,digest in native_run["consumer_sha256"].items():
        require(sha(ROOT/path) == digest, "Native consumer changed: "+path)
    result = compare(read(PREFIX+"-reference-spec.json"), read(PREFIX+"-reference-observations.json"),
        read(PREFIX+"-post-construction-queries.json"),read("value-result-lifecycle-shared-services-source-identities.json"),
        read("value-result-lifecycle-reference-read-spec.json"))
    result["evidence_sha256"] = {n:sha(EVIDENCE/n) for n in [
        PREFIX+"-reference-run.json",PREFIX+"-post-construction-queries-run.json",
        "value-result-lifecycle-shared-services-source-identities.json","value-result-lifecycle-reference-read-spec.json"]}
    output = EVIDENCE/(PREFIX+"-comparison.json")
    text = json.dumps(result,indent=2)+"\n"
    if check:
        require(output.read_text(encoding="utf-8") == text, "Stale ownership comparison")
    else:
        output.write_text(text,encoding="utf-8")
    require(not result["counts"].get("semantic_mismatch",0), "Independent semantic mismatch")
    print(json.dumps(result["by_algorithm"],sort_keys=True))
    return result

if __name__ == "__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check",action="store_true")
    audit(parser.parse_args().check)
