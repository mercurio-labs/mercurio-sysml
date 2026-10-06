"""Record the full-library result of native Ecore reference-contract validation."""
import collections
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SUPPORT = ROOT.parent / "target/support-2026-08"
EVIDENCE = ROOT / "docs/conformance/2026-08-support/library-ecore-reference-evidence.json"
DIRECT = {
    "owner", "owning_membership", "member_element", "membership_owning_namespace",
    "members", "owned_membership", "features", "type", "featuring_type",
    "chaining_feature",
}


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    raw = SUPPORT / "pilot-stdlib-ordered.json"
    prior = SUPPORT / "stdlib.ordered-relations.kir.json"
    candidate = SUPPORT / "stdlib.ecore-contracts.kir.json"
    ecore = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-effective.extract.json"
    importer = ROOT / "crates/mercurio-tools/src/bin/import_pilot_stdlib.rs"
    raw_rows = read(raw)["relationships"]
    relationships = collections.Counter(row["relation"] for row in raw_rows)
    if set(relationships) != DIRECT | {"specializes"}:
        raise ValueError("Unaccounted Pilot library relationship family")
    features = {feature["id"]: feature for feature in read(ecore)["features"]}
    owned_id = next(id for id, feature in features.items()
                    if id.endswith("//Namespace/ownedMembership"))
    namespace_id = next(id for id, feature in features.items()
                        if id.endswith("//Membership/membershipOwningNamespace"))
    if features[owned_id]["opposite"] != namespace_id or features[namespace_id]["opposite"] != owned_id:
        raise ValueError("Pinned Ecore membership opposites changed")
    owned = {(row["source"], row["target"]) for row in raw_rows
             if row["relation"] == "owned_membership"}
    namespace = {(row["target"], row["source"]) for row in raw_rows
                 if row["relation"] == "membership_owning_namespace"}
    if owned != namespace or len(owned) != relationships["owned_membership"]:
        raise ValueError("Pilot membership references are not reciprocal")
    previous_elements = read(prior)["elements"]
    candidate_elements = read(candidate)["elements"]
    if previous_elements != candidate_elements:
        raise ValueError("Ecore contract import changed the ordered candidate elements")
    evidence = {
        "scope": "Ten direct Ecore library references checked at native import for class-compatible sources and targets, lower/upper bounds, ordering and uniqueness; TypeUtil-derived specializes remains separate. This verifies the pinned library import boundary, not all native construction or mutation paths.",
        "raw_export_sha256": sha(raw),
        "effective_ecore_sha256": sha(ecore),
        "importer_sha256": sha(importer),
        "prior_candidate_sha256": sha(prior),
        "candidate_sha256": sha(candidate),
        "candidate_elements_unchanged": True,
        "candidate_element_count": len(candidate_elements),
        "direct_reference_rows": sum(relationships[name] for name in DIRECT),
        "direct_reference_rows_by_relation": {name: relationships[name] for name in sorted(DIRECT)},
        "reciprocal_memberships": len(owned),
        "reciprocal_ecore_feature_ids": [owned_id, namespace_id],
        "derived_specialization_rows": relationships["specializes"],
        "qualification": "partial G07; other reference families, opposites, native mutation and sample graphs remain open",
    }
    EVIDENCE.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(f"Verified {evidence['direct_reference_rows']} direct Ecore reference rows; candidate elements unchanged")


if __name__ == "__main__":
    main()
