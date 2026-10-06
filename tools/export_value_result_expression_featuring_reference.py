"""Resolve physical featuring observations from the immutable full Pilot cache.

This exports one relationship projection, not expression lifecycle qualification.
UUIDs bridge identities within the same immutable observation only; original
EMF fragments and canonical native containment establish cross-runtime identity.
"""
from pathlib import Path
import argparse, gzip, hashlib, json, collections
from audit_value_result_provider_plan import native_identity
ROOT = Path(__file__).resolve().parents[1]
EV = ROOT / "docs/conformance/2026-08-support/definition-pipeline-evidence"
OUTPUT = EV / "value-result-expression-featuring-full-cache-observations.json"

def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def extract():
    cache_path = EV.parent / "value-result-reference-controls.json.gz"
    spec_path = EV / "value-result-value-binding-bundle-spec.json"
    identity_path = EV / "value-result-lifecycle-shared-services-source-identities.json"
    inventory_path = EV / "value-result-invocation-input-integrity.json"
    cache = json.loads(gzip.decompress(cache_path.read_bytes()))
    spec, identity, inventory = read(spec_path), read(identity_path), read(inventory_path)
    provenance = cache["provenance"]
    assert provenance["pilot_commit"] == "692170b71867353b8f90341e61556f49a5beb0e5"
    assert all(sha(ROOT / p) == h for p, h in provenance["input_sha256"].items())
    release = ROOT.parent / "target/upstream/SysML-v2-Release"
    assert provenance["library_source_sha256"] == inventory["library_sha256"]
    assert len(inventory["library_sha256"]) == 94
    assert all(sha(release / p) == h for p, h in inventory["library_sha256"].items())
    assert all(sha(EV.parent / p) == h for p, h in inventory["frozen_acceptance_sha256"].items())
    runtime = ROOT.parent / "target/support-2026-08/pilot-pinned-2026-08/org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    java, javac = Path("D:/dev/jdks/jdk21/bin/java.exe"), Path("D:/dev/jdks/jdk21/bin/javac.exe")
    assert sha(runtime) == provenance["runtime_sha256"]
    assert sha(java) == provenance["java_sha256"] and sha(javac) == provenance["javac_sha256"]
    assert spec["required_expressions"] == 56 and len(spec["expressions"]) == 56
    assert spec["fixed_contexts"] == 26 and spec["fixed_libraries"] == 94
    assert identity["nodes_mapped"] == 388
    native_sources = {}
    for key, value in identity["canonical_resource_fragment_to_native_id"].items():
        resource, fragment = key.split("#", 1)
        normalized = resource.replace("\\", "/").rsplit("/", 1)[-1] + "#" + fragment
        assert normalized not in native_sources
        native_sources[normalized] = value
    controls = {row["id"]: row for row in cache["controls"]}
    assert len(controls) == 26
    rows, unsupported, negative = [], [], []
    for control in spec["expressions"]:
        case = controls[control["context"]]
        if case["model_status"] != "exported" or case["validation_status"] != "accepted":
            assert control["full_reference_outcome"] != "accepted"
            negative.append(dict(context=control["context"], owner_id=control["owner_id"],
                required_outcome=control["full_reference_outcome"], status="required_negative_context_not_qualified"))
            continue
        observation = case["result_observations"]
        matches = [q for q in observation["queries"] if q["emf_fragment"] == control["emf_fragment"]
                   and q["resource"] == control["context"] + ".kerml"]
        assert len(matches) == 1 and matches[0]["kind"] == control["owner_kind"].rsplit("::", 1)[-1]
        query = matches[0]
        assert query["id"] == control["cached_full_expression_identity"]
        assert native_sources[query["resource"] + "#" + query["emf_fragment"]] == control["owner_id"]
        witnesses = {w["id"]: w for w in observation["stored_witnesses"]}
        assert len(witnesses) == len(observation["stored_witnesses"])
        witness = witnesses[query["id"]]
        assert witness["stored_attributes"]["isImpliedIncluded"] is True
        model = case["model"]
        by_name = {e["qualified_name"]: e for e in model["elements"]}
        assert len(by_name) == len(model["elements"])
        by_uuid = {e["properties"]["element_id"]: e for e in model["elements"] if "element_id" in e["properties"]}
        source = by_uuid[witness["stored_attributes"]["elementId"]]
        assert source["kind"] == query["kind"]
        targets = collections.defaultdict(list)
        for relation in model["relationships"]:
            if relation["relation"] == "featuring_type":
                targets[relation["source"]].append(relation["target"])
        target_native = {}
        for w in witnesses.values():
            uid = w["stored_attributes"].get("elementId")
            node = by_uuid.get(uid)
            if node is None:
                continue
            if w["resource"].startswith("sysml.library/"):
                native = native_identity(w["resource"], w["emf_fragment"])
            else:
                native = native_sources.get(w["resource"] + "#" + w["emf_fragment"])
            if native is not None:
                target_native[node["qualified_name"]] = native
        expected, physical = [], []
        sequence = source["properties"]["reference_sequences"]["owned_relationship"]
        assert sequence["ordered"] is True and sequence["derived"] is False
        for relationship_name in sequence["targets"]:
            relation = by_name[relationship_name]
            if relation["kind"] != "TypeFeaturing":
                continue
            selected = targets[relationship_name]
            assert len(selected) == 1
            target = target_native.get(selected[0])
            if target is None:
                unsupported.append(dict(context=control["context"], owner_id=control["owner_id"], target=selected[0]))
                continue
            expected.append(target)
            physical.append(dict(kind="TypeFeaturing", target_id=target, is_implied=relation["properties"].get("is_implied", False)))
        rows.append(dict(context=control["context"], owner_id=control["owner_id"], kind=query["kind"],
            source_identity=dict(resource=query["resource"], emf_fragment=query["emf_fragment"], cached_id=query["id"]),
            expected=expected, physical=physical, reference_owner_complete=True))
    assert len(rows) == 48 and len(negative) == 8 and not unsupported
    inputs = {str(p): sha(p) for p in [cache_path, spec_path, identity_path, inventory_path, runtime, java, javac, Path(__file__)]}
    inputs.update({str(ROOT/p):h for p,h in provenance["input_sha256"].items()})
    return dict(schema="dev.mercurio.cached-full-expression-featuring.v2", qualification_certificate=False,
        reference_stage="physical featuring projection after full context transform/validation; enclosing lifecycle is not claimed",
        required_expressions=56, fixed_contexts=26, fixed_libraries=94, expressions=rows,
        required_negative_expressions=negative, unmapped_targets=unsupported, input_sha256=inputs,
        library_sha256=inventory["library_sha256"], pilot_commit=provenance["pilot_commit"],
        identity_policy="Exact original EMF fragments and canonical containment; generated UUIDs bridge only within the same cached observation")

def main():
    parser=argparse.ArgumentParser(); parser.add_argument("--check", action="store_true"); args=parser.parse_args()
    result=extract()
    if args.check:
        assert read(OUTPUT) == result, "Cached expression featuring projection or provenance changed"
    else:
        assert not OUTPUT.exists(), "Archive the unwitnessed projection before replacing it"
        OUTPUT.write_text(json.dumps(result, indent=2)+"\n", encoding="utf-8")
    print("Resolved 48/56 original expression identities; 8 required negative-context expressions remain unqualified; 94 library hashes match.")
if __name__ == "__main__": main()
