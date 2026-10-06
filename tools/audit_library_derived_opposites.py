"""Verify Ecore-derived library inverses without requiring another Pilot reload."""
import collections
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SUPPORT = ROOT.parent / "target/support-2026-08"
EVIDENCE = ROOT / "docs/conformance/2026-08-support/library-derived-opposites-evidence.json"


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def targets(properties, relation):
    value = properties.get(relation, [])
    return [value] if isinstance(value, str) else value


def compare(previous, candidate, raw, expected_counts=None):
    before = {element["id"]: element for element in previous["elements"]}
    after = {element["id"]: element for element in candidate["elements"]}
    if len(before) != len(previous["elements"]) or before.keys() != after.keys():
        raise ValueError("Candidate changed library element identity")
    expected = {"owned_element": collections.defaultdict(list),
                "owned_member_element": collections.defaultdict(list)}
    for edge in raw["relationships"]:
        if edge["relation"] == "owner":
            expected["owned_element"][edge["target"]].append(edge["source"])
        elif edge["relation"] == "owning_membership":
            expected["owned_member_element"][edge["target"]].append(edge["source"])
    added = collections.Counter()
    for identity, old in before.items():
        new = after[identity]
        if old["kind"] != new["kind"] or old.get("layer", 0) != new.get("layer", 0):
            raise ValueError(f"Changed library kind/layer: {identity}")
        old_properties = old["properties"]
        new_properties = {key: value for key, value in new["properties"].items()
                          if key not in expected}
        if identity.endswith("::OwningMembership.owned_member_element") and new["kind"] == "MetamodelFeature":
            if new_properties.pop("upper", None) != 1:
                raise ValueError("Ecore singular ownedMemberElement bound missing")
        if old_properties != new_properties:
            raise ValueError(f"Changed prior library property: {identity}")
        for relation, by_owner in expected.items():
            actual = targets(new["properties"], relation)
            if actual != by_owner.get(identity, []):
                raise ValueError(f"Derived Ecore opposite changed cardinality/order: {identity}.{relation}")
            added[relation] += len(actual)
    if expected_counts is None:
        expected_counts = {"owned_element": 45384, "owned_member_element": 45232}
    if added != expected_counts:
        raise ValueError(f"Unexpected derived Ecore opposite counts: {added}")
    return dict(added)


def main():
    raw_path = SUPPORT / "pilot-stdlib-ordered.json"
    previous_path = SUPPORT / "stdlib.ecore-contracts.kir.json"
    candidate_path = SUPPORT / "stdlib.derived-opposites.kir.json"
    ecore_path = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-effective.extract.json"
    importer_path = ROOT / "crates/mercurio-tools/src/bin/import_pilot_stdlib.rs"
    tests_path = SUPPORT / "derived-opposites-release-tests.log"
    raw = read(raw_path)
    previous = read(previous_path)
    candidate = read(candidate_path)
    added = compare(previous, candidate, raw)
    if "test result: ok. 154 passed; 0 failed" not in tests_path.read_text(encoding="utf-8"):
        raise ValueError("Focused 2026-08 candidate suite did not pass")
    evidence = {
        "scope": "Two source-checked Ecore-derived opposite fields on an unpromoted native library candidate; exact inverse cardinality and raw encounter order, no prior element/property loss. Pilot getter order and downstream semantic behavior remain unqualified.",
        "raw_export_sha256": digest(raw_path),
        "effective_ecore_sha256": digest(ecore_path),
        "importer_sha256": digest(importer_path),
        "previous_candidate_sha256": digest(previous_path),
        "candidate_sha256": digest(candidate_path),
        "elements_preserved": len(candidate["elements"]),
        "derived_inverse_references": added,
        "owned_member_element_upper_bound": 1,
        "focused_release_tests": {"passed": 154, "failed": 0, "log_sha256": digest(tests_path)},
        "qualification": "partial G07; derived order against Pilot getter, other opposites, mutation and sample graphs remain open",
    }
    EVIDENCE.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(f"Verified {sum(added.values())} derived Ecore inverse references with no prior properties changed")


if __name__ == "__main__":
    main()
