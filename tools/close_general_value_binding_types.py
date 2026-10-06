"""Close actual-library type prerequisites shared by all new value-binding ends.

Native registered demands select targets. No Pilot target or lifecycle flag is
fed into resolution. The bounded stage does not qualify terminal contexts.
"""
from pathlib import Path
import argparse, collections, json
from run_value_result_library_dependency_bundle import ROOT, EV, read, sha, save, run, port_descriptor
from verify_value_result_library_dependency_bundle import observe_sources
from audit_cached_native_dependencies import assess_successful_wave, exact_json


def main():
    parser = argparse.ArgumentParser(); parser.add_argument("--revision", required=True)
    args = parser.parse_args()
    stage_path = EV / "general-value-bindings-canonical-controls-native-run.json"; stage = read(stage_path)
    assert stage["status"] == "native_value_binding_stage_verified_reference_comparison_required" and stage["inputs_unchanged"]
    assert all(sha(path) == digest for path, digest in stage["input_sha256"].items())
    preflight = ROOT / "target/general-value-bindings-canonical-controls-comparison/native-getters.json"
    answers = read(preflight)["queries"]
    requirements = [row for row in answers if row["status"] == "dependency_required"]
    assert len(requirements) == 76 and all(row["field"] == "type" for row in requirements)
    queries = [dict(owner_id=row["owner_id"], field=row["field"]) for row in requirements]
    assert len({row["owner_id"] for row in queries}) == 76
    record = Path(stage["current_record_path"]); assert sha(record) == stage["current_record_sha256"]
    original = read(record); binary = Path(stage["runs"][0]["command"][0])
    folder = ROOT / ("target/general-value-bindings-" + args.revision + "-type-closure"); folder.mkdir(exist_ok=False)
    spec = folder / "queries-spec.json"; save(spec, dict(inspection_queries=queries))
    output = EV / ("general-value-bindings-" + args.revision + "-type-closure-run.json"); assert not output.exists()
    inputs = observe_sources(original); inputs.update({str(ROOT / path): digest for path, digest in stage["consumer_sha256"].items()})
    for path in [stage_path, preflight, record, binary, spec, Path(__file__), ROOT / "tools/audit_cached_native_dependencies.py",
                 ROOT / "tools/run_value_result_library_dependency_bundle.py", ROOT / "tools/verify_value_result_library_dependency_bundle.py"]:
        inputs[str(path)] = sha(path)
    manifest = dict(schema="dev.mercurio.general-value-binding-type-closure.v1", qualification_certificate=False, status="running",
        input_sha256=inputs, runs=[], waves=[], required_end_type_reads=76, reached_library_ports=[],
        strict_contexts_qualified=0, strict_obligations_qualified=0, strict_families_qualified=0, candidate_promoted=False)
    save(output, manifest); completed = set(); reached = {}
    for number in range(1, 5):
        demands = {json.dumps(row["dependency"]["prerequisite"], sort_keys=True): row["dependency"]["prerequisite"] for row in requirements}
        assert demands and not completed.intersection(demands)
        model = read(record); before = model["inspection"]
        index = {node["id"]: node for node in before["constructed_elements"]}
        pending = {(row["owner_id"], row["field"]): row for row in before["pending_references"]}
        for demand in demands.values():
            assert demand["kind"] == "read_field" and (demand["owner_id"], demand["field"]) in pending
            descriptor = port_descriptor(demand["owner_id"], demand["field"], index, pending)
            reached[(demand["owner_id"], demand["field"])] = descriptor
        plan = folder / ("wave-" + str(number) + "-plan.json"); save(plan, dict(requirements=list(demands.values())))
        native = folder / ("wave-" + str(number) + "-native.jsonl")
        code = run([str(binary), "--definition-plan-records", str(record), str(plan), str(native)], folder, "wave-" + str(number), manifest, output)
        if code:
            manifest.update(status="native_registered_dependency_rejected", failure=read(native) if native.exists() else None)
            save(output, manifest); print("Native type dependency rejected; retained original value-binding candidate", flush=True); return
        after = read(native)
        integrity = assess_successful_wave(before["constructed_elements"], before["pending_references"], after, list(demands.values()))
        assert integrity["native_nodes_added"] == 0 and after["input_files"] == original["input_files"]
        query_path = folder / ("wave-" + str(number) + "-queries.json")
        code = run([str(binary), "--definition-query-records", str(native), str(spec), str(query_path)], folder, "wave-" + str(number) + "-queries", manifest, output)
        assert code == 0
        answers = read(query_path)["queries"]
        assert len(answers) == 76 and {(row["owner_id"], row["field"]) for row in answers} == {(row["owner_id"], row["field"]) for row in queries}
        manifest["waves"].append(dict(number=number, requirements=list(demands.values()), integrity=integrity,
            record_path=str(native), record_sha256=sha(native), query_path=str(query_path), query_sha256=sha(query_path)))
        completed.update(demands); record = native
        requirements = [row for row in answers if row["status"] == "dependency_required"]
        manifest.update(reached_library_ports=list(reached.values()), getter_outcomes=dict(collections.Counter(row["status"] for row in answers)))
        save(output, manifest)
        if not requirements:
            if not all(row["status"] == "query_evaluated" for row in answers):
                manifest.update(status="precise_native_type_semantic_gap", unqualified_reads=[row for row in answers if row["status"] != "query_evaluated"])
                break
            manifest["status"] = "native_end_type_reads_available_reference_comparison_required"
            break
    else:
        manifest["status"] = "bounded_dependency_wave_limit_reached"
    manifest.update(current_record_path=str(record), current_record_sha256=sha(record), current_query_path=str(query_path), current_query_sha256=sha(query_path),
        original_candidate_preserved=sha(Path(stage["current_record_path"])) == stage["current_record_sha256"],
        boundary="Registered library reads only. Original graph flags are preserved; no new structural nodes, complete-context claims or certificates. Independent target/getter comparison and canonical replay remain required.")
    save(output, manifest)
    print("Shared end types:", manifest["getter_outcomes"], "; actual library ports", len(reached), "; strict families 0/34", flush=True)


if __name__ == "__main__":
    main()
