"""Finish the complete stored type-reference bundle in blocking library resources.

Only resolved Ecore FeatureTyping/type and registered Xtext references authorize
this bounded prefetch. No sample names or observed Pilot targets select work.
"""
from pathlib import Path
import argparse, collections, json
from run_value_result_library_dependency_bundle import ROOT, EV, read, sha, save, run, port_descriptor
from verify_value_result_library_dependency_bundle import observe_sources
from audit_cached_native_dependencies import assess_successful_wave


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", default="general-value-bindings-canonical-controls-type-closure-run.json")
    parser.add_argument("--revision", default="complete-type-bundle")
    args = parser.parse_args()
    baseline_path = EV / args.baseline; baseline = read(baseline_path)
    assert baseline["status"] in ["bounded_dependency_wave_limit_reached", "native_complete_type_reference_bundle_assessed"] and baseline["inputs_unchanged"]
    archive_rows = read(ROOT / "target/r41-type-bundle-before/witnesses.json")
    for path, digest in baseline["input_sha256"].items():
        if sha(path) != digest:
            assert Path(path).name == "close_general_value_binding_type_bundle.py"
            assert any(row["sha256"] == digest and sha(row["archived_path"]) == digest for row in archive_rows)
    stage_path = EV / "general-value-bindings-canonical-controls-native-run.json"; stage = read(stage_path)
    record = Path(baseline["current_record_path"]); assert sha(record) == baseline["current_record_sha256"]
    binary = Path(stage["runs"][0]["command"][0]); current = read(record); model = current["inspection"]
    blocked = read(baseline["current_query_path"])["queries"]
    resources = {row["dependency"]["prerequisite"]["owner_id"].split(".")[2] for row in blocked if row["status"] == "dependency_required"}
    assert resources and all("/sysml.library/" in bytes.fromhex(encoded).decode("utf-8").replace(chr(92), "/") for encoded in resources)
    ports = [row for row in model["pending_references"] if row["owner_id"].split(".")[2] in resources
             and (row["owner_kind"], row["field"]) in [("SysML::FeatureTyping", "type"),
                 ("SysML::Subsetting", "subsetted_feature"), ("SysML::FeatureChaining", "chaining_feature")]]
    assert 0 < len(ports) <= 64
    assert all(row["ecore_target"] in ["Feature", "Type"] for row in ports)
    roots = [dict(kind="read_field", owner_id=row["owner_id"], field=row["field"]) for row in ports]
    index = {node["id"]: node for node in model["constructed_elements"]}
    pending = {(row["owner_id"], row["field"]): row for row in model["pending_references"]}
    reached = baseline["reached_library_ports"] + [port_descriptor(row["owner_id"], row["field"], index, pending) for row in roots]
    assert len(reached) == len({(row["owner_id"], row["field"]) for row in reached})
    folder = ROOT / ("target/general-value-bindings-" + args.revision); folder.mkdir(exist_ok=False)
    plan = folder / "requirements.json"; save(plan, dict(requirements=roots))
    original_queries = [dict(owner_id=row["owner_id"], field=row["field"]) for row in blocked if row["owner_id"].endswith((".implicit.value-binding.end.0", ".implicit.value-binding.end.1"))]
    assert len(original_queries) == 76
    spec = folder / "queries-spec.json"; save(spec, dict(inspection_queries=original_queries + [dict(owner_id=row["owner_id"], field=row["field"]) for row in reached]))
    source_spec = EV / ("general-value-bindings-" + args.revision + "-reference-spec.json"); assert not source_spec.exists()
    save(source_spec, dict(qualification_certificate=False, source_files=current["input_files"], ports=[dict(row, anchor_id=row["owner_id"],
        anchor_kind=row["owner_kind"], anchor_fragment=row["emf_fragment"], getter_path=[], feature_name=row["pending_contract"]["feature_id"].rsplit("/", 1)[-1]) for row in reached]))
    output = EV / ("general-value-bindings-" + args.revision + "-run.json"); assert not output.exists()
    inputs = {row["archived_path"]: row["sha256"] for row in archive_rows}; inputs.update(observe_sources(current)); inputs.update({str(ROOT / path): digest for path, digest in stage["consumer_sha256"].items()})
    for path in [baseline_path, stage_path, record, binary, plan, spec, source_spec, Path(__file__), ROOT / "tools/close_general_value_binding_types.py",
                 ROOT / "tools/audit_cached_native_dependencies.py", ROOT / "tools/run_value_result_library_dependency_bundle.py"]:
        inputs[str(path)] = sha(path)
    manifest = dict(schema="dev.mercurio.general-value-binding-complete-type-bundle.v1", qualification_certificate=False, status="running",
        input_sha256=inputs, runs=[], required_end_type_reads=76, newly_requested_library_ports=len(ports), reached_library_ports=reached, strict_families_qualified=0, candidate_promoted=False)
    save(output, manifest)
    native = folder / "native.jsonl"
    code = run([str(binary), "--definition-plan-records", str(record), str(plan), str(native)], folder, "native", manifest, output)
    if code:
        manifest.update(status="precise_native_dependency_failure", failure=read(native) if native.exists() else None)
        save(output, manifest); return
    after = read(native)
    integrity = assess_successful_wave(model["constructed_elements"], model["pending_references"], after, roots)
    assert integrity["native_nodes_added"] == 0 and integrity["native_ports_committed"] == len(ports) and after["input_files"] == current["input_files"]
    query_path = folder / "queries.json"
    code = run([str(binary), "--definition-query-records", str(native), str(spec), str(query_path)], folder, "queries", manifest, output); assert code == 0
    answers = read(query_path)["queries"]; assert len(answers) == 76 + len(reached)
    manifest.update(status="native_complete_type_reference_bundle_assessed", integrity=integrity,
        end_type_outcomes=dict(collections.Counter(row["status"] for row in answers[:76])),
        remaining_dependencies=[row for row in answers[:76] if row["status"] != "query_evaluated"],
        current_record_path=str(native), current_record_sha256=sha(native), current_query_path=str(query_path), current_query_sha256=sha(query_path),
        boundary="Registered stored type/feature references in the blocking resources are resolved, preserving graph flags and original candidate. End types, independent endpoint comparisons and final lifecycle/validation remain separate acceptance requirements.")
    save(output, manifest); print("Complete type bundle:", manifest["end_type_outcomes"], "; actual library reads ", len(reached), "; strict families 0/34", flush=True)


if __name__ == "__main__":
    main()
