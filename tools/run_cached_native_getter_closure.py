"""Bounded cached native getter closure, driven by a fixed batch contract.

Only native typed prerequisites authorize work. Reference observations are
comparison evidence and never supplied as model values or completion flags.
"""
import argparse
import collections
import hashlib
import json
import subprocess
import time
from pathlib import Path
from audit_cached_native_dependencies import assess_successful_wave
from audit_value_result_ownership import compare

ROOT = Path(__file__).resolve().parents[1]
EV = ROOT / "docs/conformance/2026-08-support/definition-pipeline-evidence"

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))

def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")

def run(contract_path):
    contract = read(contract_path)
    assert contract["qualification_certificate"] is False
    assert contract["fixed_context_denominator"] == 26
    baseline = read(EV / contract["baseline"])
    record, queries = Path(baseline["current_record_path"]), Path(baseline["current_query_path"])
    assert sha(record) == baseline["current_record_sha256"]
    assert sha(queries) == baseline["current_query_sha256"]
    binary = ROOT / "target/release/audit_release_compile.exe"
    build = read(EV / contract["build"])
    assert sha(binary) == build["binary_sha256"]
    assert all(sha(ROOT / path) == digest for path, digest in build["input_sha256"].items())
    spec_path = EV / "value-result-ownership-reference-spec.json"
    ref_path = EV / "value-result-ownership-reference-observations.json"
    ids_path = EV / "value-result-lifecycle-shared-services-source-identities.json"
    requests_path = EV / "value-result-lifecycle-reference-read-spec.json"
    spec, ref, ids, requests = map(read, [spec_path, ref_path, ids_path, requests_path])
    keys = [(row["owner_id"], row["field"]) for row in contract["ports"]]
    assert len(keys) == len(set(keys)) == contract["required_getters"]
    original = {(p["owner_id"], p["field"]): p for p in spec["ports"]}
    assert all(original[key] == port for key, port in zip(keys, contract["ports"]))
    work = ROOT / contract["work_directory"]
    work.mkdir(parents=True, exist_ok=True)
    output = EV / (contract["output_prefix"] + "-run.json")
    assert not output.exists(), "Inspect an existing run rather than restart it."
    files = [binary, record, queries, spec_path, ref_path, ids_path, requests_path,
             contract_path, Path(__file__), ROOT / "tools/audit_cached_native_dependencies.py",
             ROOT / "tools/audit_value_result_ownership.py", EV / contract["baseline"], EV / contract["build"]]
    inputs = {str(path): sha(path) for path in files}
    manifest = dict(schema="dev.mercurio.fixed-cached-getter-closure.v1",
        qualification_certificate=False, batch=contract["batch"],
        input_sha256=inputs, consumer_sha256=build["input_sha256"],
        fixed_context_denominator=26, fixed_getter_denominator=4462,
        fixed_ownership_getters=34, required_getters=len(keys),
        waves=[], status="running", complete_native_contexts=0,
        strict_families_qualified=0, candidate_promoted=False)
    completed = set()
    save(output, manifest)
    for number in range(1, contract["max_waves"] + 1):
        native = read(queries)
        comparison = compare(spec, ref, native, ids, requests)
        rows_by_key = {(p["owner_id"], p["field"]): p for p in comparison["ports"]}
        rows = [rows_by_key[key] for key in keys]
        matched = sum(row["comparison"] == "exact_ordered_match" for row in rows)
        manifest["current_getters_matched"] = matched
        manifest["remaining"] = [row for row in rows if row["comparison"] != "exact_ordered_match"]
        print(contract["batch"], matched, "/", len(keys), "exact ordered matches", flush=True)
        if matched == len(keys):
            manifest["status"] = "all_fixed_getters_independently_matched"
            save(EV / (contract["output_prefix"] + "-comparison.json"), comparison)
            break
        if any(row["comparison"] not in ("dependency_required", "exact_ordered_match") for row in rows):
            manifest["status"] = "precise_semantic_or_implementation_gap"
            save(EV / (contract["output_prefix"] + "-comparison.json"), comparison)
            break
        answers = {(q["owner_id"], q["field"]): q for q in native["queries"]}
        needed = {}
        for row in manifest["remaining"]:
            requirement = answers[row["owner_id"], row["field"]]["dependency"]["prerequisite"]
            needed[json.dumps(requirement, sort_keys=True)] = requirement
        if not needed or completed.intersection(needed):
            manifest["status"] = "repeated_completed_requirement"
            break
        plan = work / f"wave-{number:02}-plan.json"
        save(plan, dict(requirements=list(needed.values())))
        target = work / f"wave-{number:02}.jsonl"
        log = work / f"wave-{number:02}.log"
        command = [str(binary), "--definition-plan-records", str(record), str(plan), str(target)]
        before = read(record)["inspection"]
        start = time.monotonic()
        with log.open("w", encoding="utf-8") as handle:
            code = subprocess.run(command, cwd=ROOT, stdout=handle, stderr=subprocess.STDOUT).returncode
        wave = dict(number=number, requirements=list(needed.values()), command=command,
            exit_code=code, elapsed_seconds=round(time.monotonic()-start, 3),
            input_record_sha256=sha(record), output_sha256=sha(target), log_sha256=sha(log))
        manifest["waves"].append(wave)
        print("Native wave", number, code, wave["elapsed_seconds"], "seconds", flush=True)
        if code:
            manifest["status"] = "native_dependency_bundle_rejected"
            manifest["failure"] = read(target)
            break
        after = read(target)
        wave["integrity"] = assess_successful_wave(
            before["constructed_elements"], before["pending_references"], after, list(needed.values()))
        next_queries = work / f"wave-{number:02}-queries.json"
        query_log = work / f"wave-{number:02}-queries.log"
        query_command = [str(binary), "--definition-query-records", str(target), str(requests_path), str(next_queries)]
        with query_log.open("w", encoding="utf-8") as handle:
            query_code = subprocess.run(query_command, cwd=ROOT, stdout=handle, stderr=subprocess.STDOUT).returncode
        assert query_code == 0
        wave.update(query_command=query_command, query_output_sha256=sha(next_queries),
                    query_log_sha256=sha(query_log))
        completed.update(needed)
        record, queries = target, next_queries
        save(output, manifest)
    else:
        manifest["status"] = "bounded_wave_limit_reached"
    manifest.update(final_record_path=str(record), final_record_sha256=sha(record),
        final_query_path=str(queries), final_query_sha256=sha(queries),
        inputs_unchanged=all(sha(path) == digest for path, digest in inputs.items()),
        query_outcomes=dict(collections.Counter(q["status"] for q in read(queries)["queries"])))
    assert manifest["inputs_unchanged"]
    save(output, manifest)
    print("Bundle:", manifest["status"], flush=True)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("contract", type=Path)
    run(parser.parse_args().contract)
