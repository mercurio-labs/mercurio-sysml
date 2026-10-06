"""Compare actual native provider ports with pinned independent observations.

This verifies a dependency-read component, never a full model, context or family.
Native execution must retain all 94 constructed resources and unfinished work.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "docs/conformance/2026-08-support/definition-pipeline-evidence"

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def read(name):
    return json.loads((EVIDENCE / name).read_text(encoding="utf-8"))

def require(condition, reason):
    if not condition:
        raise ValueError(reason)

def native_identity(resource, fragment):
    path = str(ROOT.parent / "target/upstream/SysML-v2-Release" / resource).replace("\\", "/")
    fields = {"ownedRelationship": "owned_relationship", "ownedRelatedElement": "owned_related_element"}
    matches = re.findall(r"@([A-Za-z_]+)\.(\d+)", fragment)
    require(matches and all(name in fields and str(int(number)) == number for name, number in matches), "Unsupported canonical ownership fragment")
    reconstructed = "//" + "/".join("@" + name + "." + number for name, number in matches)
    require(reconstructed == fragment, "Noncanonical endpoint fragment")
    return "definition.resource." + path.encode("utf-8").hex() + "".join("." + fields[name] + "." + number for name, number in matches)

def compare_ports(spec, observations, row, reference_ports):
    nodes = row["inspection"]["constructed_elements"]
    index = {node["id"]: node for node in nodes}
    require(len(index) == len(nodes) == row["element_count"], "Duplicate or missing native identity")
    expected_roots = {"definition.resource." + path.encode("utf-8").hex() for path in spec[0]["input_files"]}
    require(len(expected_roots) == 94 and {i for i in index if i.count(".") == 2} == expected_roots, "Constructed environment was pruned")
    require(len(spec) == 1 and len(spec[0]["input_files"]) == 94 and len(spec[0]["dependency_roots"]) == 9 and len(observations) == 9, "Frozen denominator changed")
    roots = {(r["owner_id"], r["field"]) for r in spec[0]["dependency_roots"]}
    observed_roots = {(r["root"]["owner_id"], r["root"]["field"]) for r in observations}
    require(len(roots) == 9 and roots == observed_roots, "Native/reference root identities differ")
    root_features = {(r["owner_id"], r["field"], r["feature_id"]) for r in reference_ports}
    require(len(root_features) == 9 and {(r["owner_id"], r["field"]) for r in reference_ports} == roots, "Resolved reference feature ports differ")
    require(root_features == {(r["root"]["owner_id"], r["root"]["field"], r["root"]["feature_id"]) for r in observations}, "Native/reference imported feature identities differ")
    committed = {(r["owner_id"], r["field"]) for r in row["committed_reference_fields"]}
    pending = row["inspection"]["pending_references"]
    require(len(pending) == row["pending_reference_count"], "Unfinished registry count changed")
    pending_ports = {(r["owner_id"], r["field"]) for r in pending}
    require(roots <= committed and not roots & pending_ports, "Requested native roots were not committed")
    verified = []
    for observation in observations:
        root = observation["root"]
        owner = index[root["owner_id"]]
        require(observation["status"] == "reference_observed" and not observation["provider_completion_before"]
            and not observation["provider_completion_after"], "Independent provider was prepared")
        require(owner["kind"].split("::")[-1] == observation["owner"]["kind"], "Owner metaclass mismatch")
        require(not owner["properties"].get("is_implied_included", False), "Native provider completion was fabricated")
        expected = native_identity(observation["endpoint"]["resource"], observation["endpoint"]["emf_fragment"])
        require(owner["properties"].get(root["field"]) == expected, "Native endpoint identity differs")
        require(expected in index and index[expected]["kind"].split("::")[-1] == observation["endpoint"]["kind"], "Endpoint metaclass differs")
        verified.append({"owner_id": root["owner_id"], "field": root["field"], "feature_id": root["feature_id"],
            "endpoint_id": expected, "independent_endpoint_name": observation["endpoint"]["qualified_name"],
            "verification": "exact_resource_canonical_ownership_and_metaclass", "native_context_complete": False})
    return {"nodes": len(nodes), "pending": len(pending), "committed": len(committed), "ports": verified}

def audit(check=False):
    run = read("value-result-parser-sharing-actual-run.json")
    require(run["exit_code"] == 0 and run["inputs_unchanged"], "Native execution failed or changed")
    for path, digest in run["consumer_sha256"].items():
        require(sha(ROOT / path) == digest, "Native consumer changed: " + path)
    require(sha(ROOT / "target/release/audit_release_compile.exe") == run["binary_sha256"], "Native executable changed")
    raw = EVIDENCE / "value-result-parser-sharing-actual-inspection.jsonl"
    require(sha(raw) == run["inspection_sha256"], "Native observation changed")
    spec_path = EVIDENCE / "value-result-provider-plan-actual-spec.json"
    require(sha(spec_path) == run["spec_sha256"], "Native input spec changed")
    spec = json.loads(spec_path.read_text(encoding="utf-8"))["cases"]
    require(len(spec) == 1 and len(spec[0]["input_files"]) == 94 and len(spec[0]["dependency_roots"]) == 9, "Frozen denominator changed")
    pin = read("value-result-provider-plan-actual-input.json")
    upstream = ROOT.parent / "target/upstream/SysML-v2-Release"
    require(len(pin["pinned_library_source_sha256"]) == 94, "Pinned environment changed")
    for path, digest in pin["pinned_library_source_sha256"].items():
        require(sha(upstream / path) == digest, "Pinned library changed: " + path)
    reference_run = read("value-result-provider-plan-pilot-run.json")
    require(reference_run["inputs_unchanged"] and all(r["exit_code"] == 0 for r in reference_run["runs"]), "Independent run not verified")
    for path, digest in reference_run["input_sha256"].items():
        require(sha(path) == digest, "Independent input changed: " + path)
    reference_path = EVIDENCE / "value-result-provider-plan-pilot-observations.json"
    require(sha(reference_path) == reference_run["observations_sha256"], "Independent observations changed")
    observations = json.loads(reference_path.read_text(encoding="utf-8"))["observations"]
    require(len(observations) == 9, "Independent port denominator changed")
    rows = [json.loads(line) for line in raw.read_text(encoding="utf-8").splitlines() if line.strip()]
    require(len(rows) == 1, "Native observation row count changed")
    row = rows[0]
    require(row["status"] == "dependency_inspection" and row["publication"] == "not_attempted"
        and row["semantic_qualification"] == "not_assessed", "Inspection became qualification or publication")
    comparison = compare_ports(spec, observations, row, read("value-result-provider-plan-pilot-spec.json")["ports"])
    data = {"schema": "dev.mercurio.native-provider-port-comparison.v1", "qualification_certificate": False,
        "native_provider_ports_verified": 9, "required_provider_ports": 9, "constructed_library_resources": 94,
        "constructed_elements": comparison["nodes"], "remaining_pending_reference_fields": comparison["pending"],
        "committed_reference_fields": comparison["committed"], "publication": "not_attempted",
        "complete_native_contexts": 0, "strict_families_qualified": 0, "ports": comparison["ports"],
        "input_sha256": {"auditor": sha(Path(__file__)), "native_run": sha(EVIDENCE / "value-result-parser-sharing-actual-run.json"),
            "native_observation": sha(raw), "independent_run": sha(EVIDENCE / "value-result-provider-plan-pilot-run.json"),
            "independent_observation": sha(reference_path)},
        "remaining": ["Provider semantic closure, defaults/prototypes/transformation and applicable validation remain required.",
            "Full source/library lifecycle, Closed publication, persistence and all26 terminal contexts remain unqualified."]}
    output = EVIDENCE / "value-result-parser-sharing-provider-comparison.json"
    encoded = json.dumps(data, indent=2) + "\n"
    if check:
        require(output.read_text(encoding="utf-8") == encoded, "Provider-port comparison evidence is stale")
    else:
        output.write_text(encoded, encoding="utf-8")
    print(json.dumps({key: data[key] for key in ["native_provider_ports_verified", "constructed_library_resources",
        "constructed_elements", "remaining_pending_reference_fields", "complete_native_contexts"]}, indent=2))

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    audit(parser.parse_args().check)
