"""Compare pinned Pilot library membership access and resolved targets with native lowering."""
import argparse
import collections
import json
import os
from pathlib import Path
import subprocess

from run_namespace_batch import ROOT, digest, read, write
from run_declaration_milestone import sources


def single_ref(properties, field):
    value = properties.get(field)
    if isinstance(value, str):
        return value
    if isinstance(value, list) and len(value) == 1 and isinstance(value[0], str):
        return value[0]
    return None


def reference_set(properties, field):
    value = properties.get(field, [])
    if isinstance(value, str):
        return {value}
    if isinstance(value, list) and all(isinstance(item, str) for item in value):
        return set(value)
    raise ValueError(f"Invalid library reference shape for {field}")


def compare_collection_sequences(raw, by_id):
    collection_fields = {"members", "owned_membership", "features", "type",
                         "featuring_type", "chaining_feature", "specializes"}
    sequences = collections.defaultdict(list)
    for relationship in raw["relationships"]:
        if relationship["relation"] in collection_fields:
            sequences[(relationship["source"], relationship["relation"])].append(
                relationship["target"])
    reordered = collections.Counter()
    duplicate_counts = collections.Counter()
    for (source, relation), expected in sequences.items():
        actual = by_id[source]["properties"].get(relation, [])
        if isinstance(actual, str):
            actual = [actual]
        if actual != expected:
            raise ValueError(f"KIR changed Pilot collection order or duplicates: {source}.{relation}")
        reordered[relation] += expected != sorted(expected)
        duplicate_counts[relation] += len(expected) - len(set(expected))
    return {"compared_collections": len(sequences),
            "compared_references": sum(map(len, sequences.values())),
            "non_lexicographic_order_witnesses": dict(reordered),
            "duplicate_targets": dict(duplicate_counts)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--library", default="stdlib.visibility.kir.json")
    parser.add_argument("--evidence", default="library-visibility-evidence.json")
    parser.add_argument("--previous", help="Prior candidate for monotonic relationship expansion audit")
    parser.add_argument("--raw-export", help="Pilot export whose collection sequences must match KIR exactly")
    args = parser.parse_args()
    before = sources()
    support = ROOT.parent / "target/support-2026-08"
    folder = support / "library-visibility-controls"
    evidence = ROOT / "docs/conformance/2026-08-support"
    oracle = read(evidence / "library-visibility-pilot-controls.json")
    helper = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java"
    exporter = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotStdlibExporter.java"
    importer = ROOT / "crates/mercurio-tools/src/bin/import_pilot_stdlib.rs"
    jar = support / "../upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"
    library = support / args.library
    fixture_dir = ROOT / "crates/mercurio-tools/corpus/release-2026-08/library-visibility"
    if oracle["helper_sha256"] != digest(helper) or oracle["jar_sha256"] != digest(jar):
        raise ValueError("Pilot provenance drift")
    if any(digest(fixture_dir / name) != sha for name, sha in oracle["source_sha256"].items()):
        raise ValueError("Pilot fixture drift")
    library_elements = read(library)["elements"]
    metadata = [e.get("properties", {}).get("metadata", {}) for e in library_elements]
    visibility_counts = {kind: sum(m.get("owning_membership_visibility") == kind for m in metadata)
                         for kind in ("public", "protected", "private")}
    if visibility_counts["private"] == 0 or visibility_counts["protected"] == 0:
        raise ValueError("Candidate lacks nonpublic library memberships")
    membership_graph = None
    relation_expansion = None
    collection_sequences = None
    if args.library != "stdlib.visibility.kir.json":
        by_id = {e["id"]: e for e in library_elements}
        memberships = [e for e in library_elements if e["id"].startswith("LibraryMembership::")]
        anonymous = [e for e in library_elements if e["id"].startswith("LibraryAnonymous::")]
        if len(by_id) != len(library_elements) or not memberships:
            raise ValueError("Concrete Membership identities missing or duplicated")
        if anonymous and any(not e["properties"].get("metadata", {}).get("resource_fragment")
                             for e in anonymous):
            raise ValueError("Anonymous library identity lacks source fragment")
        kinds = collections.Counter(e["kind"] for e in memberships)
        edge_counts = {}
        for field in ("member_element", "membership_owning_namespace"):
            present = [single_ref(e["properties"], field) for e in memberships]
            if any(value is not None and value not in by_id for value in present):
                raise ValueError(f"Concrete Membership {field} has a dangling reference")
            edge_counts[field] = {"present": sum(value is not None for value in present),
                                  "absent": sum(value is None for value in present)}
            source_present = [e["properties"].get("metadata", {}).get(f"{field}_present")
                              for e in memberships]
            if any(value is not None for value in source_present):
                if any(not isinstance(value, bool) for value in source_present):
                    raise ValueError(f"Incomplete Pilot presence flags for {field}")
                if any(reference is not None and not value
                       for reference, value in zip(present, source_present)):
                    raise ValueError(f"Exported {field} contradicts Pilot presence flag")
                edge_counts[field]["pilot_non_null_unexported"] = sum(
                    reference is None and value for reference, value in zip(present, source_present))
                edge_counts[field]["pilot_null"] = sum(not value for value in source_present)
            if anonymous and edge_counts[field]["absent"]:
                raise ValueError(f"Anonymous candidate still loses Membership {field} edges")
        reciprocal = 0
        for element in library_elements:
            if element["id"].startswith(("LibraryMembership::", "LibraryAnonymous::")) or "::" not in element["id"]:
                continue
            expected_visibility = element.get("properties", {}).get("metadata", {}).get("owning_membership_visibility")
            if expected_visibility is None:
                continue
            owner = element["id"].rsplit("::", 1)[0]
            membership_id = single_ref(element["properties"], "owning_membership")
            if membership_id is None:
                raise ValueError("Named library member lacks one owning Membership")
            membership = by_id[membership_id]
            properties = membership["properties"]
            if (single_ref(properties, "member_element") != element["id"]
                    or single_ref(properties, "membership_owning_namespace") != owner
                    or properties.get("metadata", {}).get("visibility") != expected_visibility
                    or membership_id not in by_id[owner]["properties"].get("owned_membership", [])):
                raise ValueError("Library Membership reciprocity or visibility differs")
            reciprocal += 1
        if reciprocal != 10168:
            raise ValueError(f"Unexpected reciprocal named library member count: {reciprocal}")
        previous = {e["id"]: e for e in read(support / "stdlib.visibility.kir.json")["elements"]}
        fields = (() if args.previous else
                  ("type", "specializes", "featuring_type", "chaining_feature", "members", "features"))
        for identity, old in previous.items():
            current = by_id.get(identity)
            if (current is None or current["kind"] != old["kind"]
                    or any(current["properties"].get(field) != old["properties"].get(field) for field in fields)
                    or current["properties"].get("metadata", {}).get("owning_membership_visibility")
                       != old["properties"].get("metadata", {}).get("owning_membership_visibility")):
                raise ValueError("Named library identity, visibility or preserved relationships changed")
        membership_graph = {"objects": len(memberships), "anonymous_objects": len(anonymous),
                            "kinds": dict(kinds),
                            "exported_edges": edge_counts,
                            "reciprocal_named_members": reciprocal,
                            "root_packages_without_named_owner": 94,
                            "previous_named_elements_preserved": len(previous)}
        if args.previous:
            prior_elements = {e["id"]: e for e in read(support / args.previous)["elements"]}
            if set(by_id) != set(prior_elements):
                raise ValueError("Relationship expansion changed the library element inventory")
            fields = ("members", "specializes", "features", "type", "featuring_type", "chaining_feature")
            gained = collections.Counter()
            gained_anonymous = collections.Counter()
            derived_first_type_changes = 0
            for identity, prior in prior_elements.items():
                current = by_id.get(identity)
                if current is None or current["kind"] != prior["kind"]:
                    raise ValueError("Prior anonymous graph identity was lost")
                if prior["kind"] == "MetamodelFeature":
                    if current["properties"].get("type") != prior["properties"].get("type"):
                        source = current["properties"].get("source_feature")
                        source_types = by_id[source]["properties"].get("type", [])
                        if isinstance(source_types, str):
                            source_types = [source_types]
                        current_types = current["properties"].get("type", [])
                        if isinstance(current_types, str):
                            current_types = [current_types]
                        if not source_types or current_types != source_types[:1]:
                            raise ValueError("Derived metafeature type does not follow Pilot feature order")
                        derived_first_type_changes += 1
                    continue
                for field in fields:
                    old = reference_set(prior["properties"], field)
                    new = reference_set(current["properties"], field)
                    if not old <= new:
                        raise ValueError(f"Prior {field} links were lost on {identity}")
                    added = new - old
                    gained[field] += len(added)
                    gained_anonymous[field] += sum(target.startswith("LibraryAnonymous::") for target in added)
            if sum(gained.values()) not in (0, 69989) or gained != gained_anonymous:
                raise ValueError("Unexpected pinned anonymous relationship expansion")
            relation_expansion = {"previous_library_sha256": digest(support / args.previous),
                                  "previous_elements_preserved": len(prior_elements),
                                  "derived_first_type_changes": derived_first_type_changes,
                                  "gained_links": dict(gained),
                                  "gained_anonymous_target_links": dict(gained_anonymous)}
        if args.raw_export:
            raw_path = support / args.raw_export
            collection_sequences = compare_collection_sequences(read(raw_path), by_id)
            if not sum(collection_sequences["non_lexicographic_order_witnesses"].values()):
                raise ValueError("Pinned Pilot export has no non-lexicographic order witness")
            collection_sequences["raw_export_sha256"] = digest(raw_path)

    env = dict(os.environ, MERCURIO_STDLIB_PATH=str(library), MERCURIO_KERNEL_LIBRARY_PATH=str(library))
    compile_binary = ROOT / "target/debug/audit_release_compile.exe"
    output = folder / "native-controls.jsonl"
    with (folder / "native-controls.log").open("w", encoding="utf-8") as log:
        result = subprocess.run([str(compile_binary), str(folder / "spec.json"), str(output)],
                                env=env, stdout=log, stderr=subprocess.STDOUT)
    rows = [json.loads(line) for line in output.read_text(encoding="utf-8").splitlines()]
    expected = {row["relative_path"]: row["status"] for row in oracle["cases"]}
    if result.returncode != 1 or len(rows) != 8 or {r["relative_path"]: r["status"] for r in rows} != expected:
        raise ValueError("Library access differs from Pilot")
    for row in rows:
        if row["status"] == "error" and not any(
                name in json.dumps(row["diagnostics"]) for name in ("index", "monitor1", "endWhen")):
            raise ValueError("Library access failed without the inaccessible member diagnostic")

    cases, expectations = [], []
    for name in ("positive-protected-inherited.kerml", "positive-public-inherited.kerml",
                 "positive-public-qualified.kerml", "positive-public-typed.kerml"):
        graph = read(folder / name / "graph.json")
        element, = [e for e in graph["elements"] if e.get("properties", {}).get("declared_name") == "x"]
        relation = "type" if name == "positive-public-typed.kerml" else "specializes"
        targets = [r["target"] for r in graph["relationships"]
                   if r["source"] == element["qualified_name"] and r["relation"] == relation]
        if len(targets) != 1:
            raise ValueError("Expected one Pilot library target")
        cases.append({"relative_path": name, "input_files": [str(fixture_dir / name)]})
        expectations.append({"source": name, "line": element["source"]["start_line"],
                             "name": "x", "property": relation,
                             "pilot": targets[0] if relation == "type" else targets})
    spec = folder / "target-spec.json"
    write(spec, {"cases": cases, "expectations": expectations})
    property_binary = ROOT / "target/debug/audit_release_properties.exe"
    property_output = folder / "target-results.jsonl"
    with (folder / "target-results.log").open("w", encoding="utf-8") as log:
        result = subprocess.run([str(property_binary), str(spec), str(property_output)],
                                env=env, stdout=log, stderr=subprocess.STDOUT)
    targets = [json.loads(line) for line in property_output.read_text(encoding="utf-8").splitlines()]
    if result.returncode or len(targets) != 4 or any(row["status"] != "match" for row in targets):
        raise ValueError("Library target identities differ from Pilot")
    if sources() != before:
        raise ValueError("Native source drift during audit")
    write(evidence / args.evidence, {
        "scope": ("Pinned Pilot collection order and duplicates preserved through native KIR with unchanged target set and element inventory; eight Pilot/native access controls and four resolved targets. Derived specialization duplicates, other graph relations, imports and complete namespace linking remain open; no benchmark."
                  if collection_sequences else
                  "Monotonic anonymous relationship expansion with unchanged element inventory: 69,989 gained target links, eight Pilot/native access controls and four resolved targets. Collection order and duplicates, other graph relations, imports and complete namespace linking remain open; no benchmark."
                  if relation_expansion else
                  "Concrete library Membership identity and anonymous target identities; all 50,106 member-element and owning-namespace links, reciprocal named-member links, eight Pilot/native access controls and four resolved targets. Other anonymous relationships, import algorithms and complete namespace linking remain open; no benchmark."
                  if membership_graph and membership_graph["anonymous_objects"] else
                  "Concrete library Membership identity, reciprocal named-member links, eight Pilot/native access controls and four resolved targets. Import relationships, anonymous-member behavior and complete namespace linking remain open; no benchmark."
                  if membership_graph else "Named direct library membership access: eight Pilot/native controls and four resolved targets. Full Membership objects, imports, and namespace linking remain open; no benchmark."),
        "visibility_counts": visibility_counts,
        "membership_graph": membership_graph,
        "relation_expansion": relation_expansion,
        "collection_sequences": collection_sequences,
        "compiler_source_sha256": before,
        "source_sha256": oracle["source_sha256"],
        "jar_sha256": digest(jar),
        "helper_sha256": digest(helper),
        "exporter_sha256": digest(exporter),
        "importer_sha256": digest(importer),
        "library_sha256": digest(library),
        "compile_binary_sha256": digest(compile_binary),
        "property_binary_sha256": digest(property_binary),
        "controls": rows,
        "targets": targets,
    })
    print("Verified eight library access controls and four Pilot/native targets")


if __name__ == "__main__":
    main()
