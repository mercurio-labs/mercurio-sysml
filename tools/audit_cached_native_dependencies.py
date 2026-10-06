"""Verify fixed retained-native waves without awarding qualification credit."""
import argparse
import hashlib
import json
import struct
from pathlib import Path

BASE = Path("docs/conformance/2026-08-support/definition-pipeline-evidence")
CACHE = Path("target/value-result-fresh-structure/constructor-records.jsonl")
PREFIX = "value-result-cached-dependency-"

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def read(name):
    return json.loads((BASE / (PREFIX + name)).read_text(encoding="utf-8"))

def require(condition, message):
    if not condition:
        raise ValueError(message)

def exact_json(left, right):
    """Preserve JSON types and IEEE-754 bits, including signed zero."""
    if type(left) is not type(right):
        return False
    if isinstance(left, float):
        return struct.pack(">d", left) == struct.pack(">d", right)
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(exact_json(left[k], right[k]) for k in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(exact_json(a,b) for a,b in zip(left,right))
    return left == right

def assess_successful_wave(nodes, pending, result, requirements):
    inspection = result["inspection"]
    graph = inspection["constructed_elements"]
    require(result["element_count"] == len(graph) and len(graph) >= len(nodes), "Node count changed")
    require([n["id"] for n in graph[:len(nodes)]] == [n["id"] for n in nodes], "Existing graph order changed")
    index = {n["id"]: n for n in graph}
    require(len(index) == len(graph), "Duplicate output node")
    new_ids = set(index) - {n["id"] for n in nodes}
    commits = {(p["owner_id"], p["field"]) for p in result["committed_reference_fields"]}
    require(len(commits) == len(result["committed_reference_fields"]), "Duplicate committed port")
    require(commits <= {(p["owner_id"],p["field"]) for p in pending}, "Unregistered committed port")
    for before, after in zip(nodes, graph):
        require(before.keys() == after.keys(), "Existing record shape changed")
        require(all(exact_json(value,after[key]) for key,value in before.items() if key!="properties"),
                "Existing identity, metaclass or record metadata changed")
        bp, ap = before["properties"], after["properties"]
        require(bp.keys() <= ap.keys(), "Existing property removed")
        for field, value in ap.items():
            if field in bp and exact_json(value, bp[field]):
                continue
            if (before["id"], field) in commits:
                targets = value if isinstance(value, list) else [value]
                require(all(t in index for t in targets), "Committed endpoint missing")
            elif field == "owned_relationship" and field in bp:
                require(exact_json(value[:len(bp[field])], bp[field]) and set(value[len(bp[field]):]) <= new_ids,
                        "Construction changed existing containment order")
            else:
                raise ValueError(f"Undeclared mutation {before['id']}.{field}")
    remaining = [p for p in pending if (p["owner_id"], p["field"]) not in commits]
    require(inspection["pending_references"] == remaining
            and result["pending_reference_count"] == len(remaining), "Remaining registry changed")
    for job in requirements:
        if job["kind"] == "read_field":
            require((job["owner_id"],job["field"]) in commits, "Requested read not committed")
    return dict(native_nodes_added=len(new_ids), native_ports_committed=len(commits),
                pending_references_retained=len(remaining))

def assess():
    scope, spec, run = read("scope.json"), read("plan-spec.json"), read("execution-run.json")
    require(scope["required_read_port_count"] == 59 and scope["required_construction_count"] == 30
            and scope["fixed_reference_read_count"] == 4462 and scope["fixed_context_count"] == 26,
            "Frozen scope changed")
    require(sha(CACHE) == scope["input_sha256"] == spec["input_sha256"], "Constructor cache changed")
    original = json.loads(CACHE.read_text(encoding="utf-8"))
    nodes, pending = original["inspection"]["constructed_elements"], original["inspection"]["pending_references"]
    require(len(nodes) == 93708 and len(pending) == 18929 and len(original["input_files"]) == 119,
            "Input denominator changed")
    require(run["inputs_unchanged"] and not run["qualification_certificate"], "Run provenance changed")
    for name, digest in run["input_sha256"].items():
        require(sha(Path(name)) == digest, f"Input changed: {name}")
    for name, digest in run["consumer_sha256"].items():
        require(sha(Path(name)) == digest, f"Consumer changed: {name}")
    output = Path(run["output_path"])
    require(sha(output) == run["output_sha256"], "Output changed")
    batches = spec["batches"]
    require(len(batches) == 12 and len(batches[0]["requirements"]) == 89, "Wave denominator changed")
    require(batches[0]["requirements"] == [{"kind": "read_field", **p} for p in scope["required_read_ports"]]
            + scope["required_constructions"], "Mixed wave changed")
    for suffix in ["native-regression-run.json", "tool-tests-run.json", "build-run.json"]:
        manifest = read(suffix)
        require(manifest["exit_code"] == 0 and manifest["inputs_unchanged"], f"Unverified {suffix}")
        for path, digest in manifest["input_sha256"].items():
            require(sha(Path(path)) == digest, f"Stale {suffix}: {path}")
        logname = suffix.replace("-run.json", ".log")
        require(sha(BASE / (PREFIX + logname)) == manifest["log_sha256"], f"Log changed: {logname}")
    native = read("native-regression-run.json")
    require(native["required_test_count"] == native["passed_test_count"] == 488,
            "Current native regression denominator changed")
    require("definition_document::cached_dependencies_tests::definition_cached_dependencies_preserve_rational_storage_on_json_round_trip"
            in native["passed_tests"], "Numeric bit-fidelity test not verified")
    require(read("build-run.json")["binary_sha256"] == sha(Path(run["command"][0])), "Driver changed")
    results = []
    with output.open(encoding="utf-8") as stream:
        for batch, line in zip(batches, stream, strict=True):
            result = json.loads(line)
            require(result["batch_name"] == batch["name"] and result["requested_requirements"] == batch["requirements"],
                    "Wave identity or roots changed")
            require(result["publication"] == "not_attempted" and result["semantic_qualification"] == "not_assessed"
                    and result["qualification_certificate"] is False, "Execution claims qualification")
            summary = {"batch": batch["name"], "status": result["status"], "requested_jobs": len(batch["requirements"])}
            if result["status"] == "blocked":
                require("inspection" not in result and "model" not in result, "Failed wave returned a partial model")
                summary.update(discarded_commits=result["attempted_reference_commits_discarded"], error=result["error"])
            else:
                require(result["status"] == "dependency_inspection", "Unknown outcome")
                inspection = result["inspection"]
                require(inspection["linking"] == "requested_plan_completed"
                        and inspection["semantic_validation"] == inspection["transformation_completion"] == "not_assessed",
                        "Wave implies completion")
                summary.update(assess_successful_wave(nodes, pending, result, batch["requirements"]))
            results.append(summary)
    return {"schema": "dev.mercurio.cached-native-wave-integrity.v1", "qualification_certificate": False,
            "fixed_requested_reads": 59, "fixed_requested_constructions": 30, "input_resources": 119,
            "complete_native_contexts": 0, "strict_families_qualified": 0, "release_gates_qualified": 0,
            "batches": results,
            "boundary": "Requested native execution, semantic comparison, complete context verification and qualification remain separate. Negative controls and context isolation need their reviewed acceptance stages."}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    report = assess()
    destination = BASE / (PREFIX + "integrity.json")
    if args.check:
        require(read("integrity.json") == report, "Integrity report stale")
    else:
        destination.write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
    print(json.dumps({"waves_checked": len(report["batches"]), "native_contexts": 0, "strict_families": 0}))
if __name__ == "__main__":
    main()
