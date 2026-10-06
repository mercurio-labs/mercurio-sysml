"""Run isolated same-source Pilot semantic comparisons and fresh-process timings.
Run from mercurio-sysml. Never interprets graph-export time as compiler time.
"""
import argparse, collections, gzip, hashlib, json, os, platform, statistics, subprocess, time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def write(path, value):
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def run(command, log, timeout, env=None):
    start = time.perf_counter()
    with Path(log).open("w", encoding="utf-8") as stream:
        try:
            result = subprocess.run([str(v) for v in command], stdout=stream, stderr=subprocess.STDOUT,
                                    timeout=timeout, env=env)
            return {"returncode": result.returncode, "wall_ms": (time.perf_counter()-start)*1000}
        except subprocess.TimeoutExpired:
            return {"returncode": None, "timeout": True, "wall_ms": (time.perf_counter()-start)*1000}


def archive(path):
    path = Path(path)
    data = path.read_bytes()
    zipped = path.with_suffix(path.suffix + ".gz")
    zipped.write_bytes(gzip.compress(data, compresslevel=5, mtime=0))
    if gzip.decompress(zipped.read_bytes()) != data:
        raise ValueError("archive verification failed")
    path.unlink()  # only this runner's verified generated artifact
    return {"path": str(zipped), "uncompressed_sha256": hashlib.sha256(data).hexdigest()}


def oracle_ready(directory):
    # Timings precede graph export. Only the producer's completion record proves
    # that the export writer has closed, including on a resumed assessment.
    outcome = directory / "oracle-execution.json"
    if not outcome.exists():
        return False
    try:
        return (read(outcome)["returncode"] == 0
                and (directory / "pilot-export.json").exists()
                and (directory / "pilot-semantic-timings.json").exists())
    except (ValueError, OSError):
        return False


def wait_for_oracle(directory, timeout=7200):
    start = time.monotonic()
    while time.monotonic() - start < timeout:
        if oracle_ready(directory):
            return {"returncode": 0, "reused_export": True}
        outcome = directory / "oracle-execution.json"
        if outcome.exists():
            try:
                result = read(outcome)
                if result["returncode"] != 0:
                    return result
            except (ValueError, OSError):
                pass
        time.sleep(1)
    return {"returncode": None, "timeout": True, "waiting_for_oracle": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=Path("../target/assessment-2026-08-detailed"))
    parser.add_argument("--manifest", type=Path, default=Path("../target/release-audit-2026-08-final/corpus.json"))
    parser.add_argument("--pilot-root", type=Path, default=Path("../target/upstream/SysML-v2-Pilot-Implementation"))
    parser.add_argument("--jar", type=Path, default=Path("../target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"))
    parser.add_argument("--stdlib", type=Path, default=Path("../target/support-2026-08/stdlib.qualified-source.kir.json"))
    parser.add_argument("--reuse-oracle-only", action="store_true", help="Wait for exports from a separate oracle stage; never launch duplicate Java workers")
    parser.add_argument("--stage", choices=["prepare", "oracle", "semantics", "performance"], required=True)
    args = parser.parse_args()
    out, pilot, jar = args.out.resolve(), args.pilot_root.resolve(), args.jar.resolve()
    out.mkdir(parents=True, exist_ok=True)
    groups = collections.defaultdict(list)
    source_hashes = {}
    for case in read(args.manifest)["cases"]:
        inputs = []
        for original in case["input_files"]:
            path = Path(original)
            parts = path.parts
            index = next(i for i in range(len(parts)-1) if parts[i] in ("sysml", "kerml") and parts[i+1] == "src")
            relative = Path(*parts[index:]).as_posix()
            candidate = pilot / relative
            if digest(path) != digest(candidate):
                raise ValueError("Release/Pilot source differs: " + relative)
            source_hashes[relative] = digest(candidate)
            inputs.append(candidate.as_posix())
        groups[tuple(sorted(inputs))].append({"relative_path": case["relative_path"], "input_files": inputs})
    entries = []
    for inputs, cases in groups.items():
        group_id = hashlib.sha256("\n".join(inputs).encode()).hexdigest()[:12]
        directory = out / "groups" / group_id
        directory.mkdir(parents=True, exist_ok=True)
        spec = directory / "spec.json"
        write(spec, {"cases": cases})
        entries.append({"id": group_id, "spec": str(spec), "cases": [c["relative_path"] for c in cases],
                        "input_count": len(inputs), "input_bytes": sum(Path(p).stat().st_size for p in inputs)})
    entries.sort(key=lambda e: (-len(e["cases"]), e["id"]))
    provenance = {"pilot_commit": subprocess.check_output(["git", "-C", str(pilot), "rev-parse", "HEAD"], text=True).strip(),
                  "pilot_clean": not subprocess.check_output(["git", "-C", str(pilot), "status", "--porcelain"], text=True).strip(),
                  "library_index_sha256": digest(pilot/"sysml.library"/".index.json"), "jar_sha256": digest(jar), "stdlib_sha256": digest(args.stdlib), "stdlib_path": str(args.stdlib.resolve()), "manifest_sha256": digest(args.manifest),
                  "platform": platform.platform(), "processor": platform.processor(), "logical_cpus": os.cpu_count(),
                  "sources": source_hashes, "groups": entries}
    if not provenance["pilot_clean"]:
        raise ValueError("dirty Pilot checkout")
    engine_paths = [Path("target/release/audit_release_compile.exe"), Path("target/release/compare_pilot_semantics.exe"), out/"classes/dev/mercurio/pilot/PilotModelExporter.class"]
    if args.stage in ("semantics", "performance"):
        provenance["engine_sha256"] = {path.name: digest(path) for path in engine_paths}
    if args.stage != "prepare":
        provenance["oracle_sha256"] = digest(out/"classes/dev/mercurio/pilot/PilotModelExporter.class")
    previous_lock = out/"source-lock.json"
    if previous_lock.exists():
        previous = read(previous_lock)
        for key in ("pilot_commit", "jar_sha256", "stdlib_sha256", "manifest_sha256", "sources", "library_index_sha256"):
            if previous.get(key) != provenance[key]:
                raise ValueError("assessment inputs changed; use a new output directory: " + key)
        if "oracle_sha256" in previous:
            if args.stage != "prepare" and previous["oracle_sha256"] != provenance["oracle_sha256"]:
                raise ValueError("oracle helper changed; use a new output directory")
            provenance["oracle_sha256"] = previous["oracle_sha256"]
        if "engine_sha256" in previous:
            if args.stage in ("semantics", "performance") and previous["engine_sha256"] != provenance["engine_sha256"]:
                raise ValueError("assessment engine binaries changed; use a new output directory")
            provenance["engine_sha256"] = previous["engine_sha256"]
    write(previous_lock, provenance)
    if args.stage == "prepare":
        print(f"Prepared {len(entries)} isolated groups, {sum(len(g['cases']) for g in entries)} targets", flush=True)
        return
    env = os.environ.copy()
    env["MERCURIO_STDLIB_PATH"] = str(args.stdlib.resolve())
    env["MERCURIO_KERNEL_LIBRARY_PATH"] = str(args.stdlib.resolve())
    env["MERCURIO_PILOT_JAR"] = str(jar)
    java = ["java", "-Xmx3g", "-cp", str(out/"classes") + os.pathsep + str(jar), "dev.mercurio.pilot.PilotModelExporter", "--assessment", pilot/"sysml.library"]
    native = Path("target/release/audit_release_compile.exe").resolve()
    comparator = Path("target/release/compare_pilot_semantics.exe").resolve()
    if args.stage == "oracle":
        with ThreadPoolExecutor(max_workers=3) as pool:
            pending = []
            for group in entries:
                directory = out/"groups"/group["id"]
                export, timing = directory/"pilot-export.json", directory/"pilot-semantic-timings.json"
                if oracle_ready(directory):
                    continue
                # A previous failed or incomplete run must not signal completion
                # while its replacement is still writing the graph.
                (directory / "oracle-execution.json").unlink(missing_ok=True)
                pending.append((group, pool.submit(run, java+[group["spec"], export, timing], directory/"pilot-export.log", 900, env)))
            for group, future in pending:
                result = future.result()
                print(f"ORACLE {group['id']}: {result}", flush=True)
                write(out/"groups"/group["id"]/"oracle-execution.json", result)
        return
    if args.stage == "semantics":
        # Source sets are isolated. Export concurrency affects audit throughput only;
        # the separate performance stage starts after every worker has terminated.
        pool = ThreadPoolExecutor(max_workers=3)
        exports = {}
        for group in entries:
            directory = out/"groups"/group["id"]
            if (directory/"summary.json").exists(): continue
            export, timing = directory/"pilot-export.json", directory/"pilot-semantic-timings.json"
            if oracle_ready(directory): continue
            exports[group["id"]] = (pool.submit(wait_for_oracle, directory) if args.reuse_oracle_only
                                    else pool.submit(run, java+[group["spec"], export, timing], directory/"pilot-export.log", 900, env))
        summaries = []
        for number, group in enumerate(entries, 1):
            directory = out/"groups"/group["id"]
            export = directory/"pilot-export.json"
            timing = directory/"pilot-semantic-timings.json"
            checkpoint = directory/"summary.json"
            if checkpoint.exists():
                summaries.extend(read(checkpoint)); continue
            print(f"SEMANTICS group {number}/{len(entries)}: {len(group['cases'])} targets {group['cases'][0]}", flush=True)
            outcome = (exports[group["id"]].result() if group["id"] in exports
                       else {"returncode": 0, "reused_export": True})
            group_rows = []
            if outcome["returncode"] != 0 or not export.exists():
                group_rows = [{"relative_path": name, "status": "pilot_export_failed", "execution": outcome, "group": group["id"]} for name in group["cases"]]
            else:
                pilot_statuses = {c["relative_path"]: c["status"] for c in read(timing)["cases"]}
                def compare_case(item):
                    index, name = item
                    report_path = directory/f"compare-{index:03}.json"
                    result = run([comparator, "--pilot-root", pilot, "--profile-id", "sysml-2.0-pilot-2026-08",
                                  "--corpus-manifest", args.manifest.resolve(), "--relative-path", name,
                                  "--pilot-export", export, "--all-attributes", "--include-derived-properties", "--out", report_path],
                                 directory/f"compare-{index:03}.log", 300, env)
                    row = {"relative_path": name, "group": group["id"], "execution": result, "pilot_validation_status": pilot_statuses[name]}
                    if result["returncode"] != 0 or not report_path.exists():
                        row["status"] = "comparison_failed"
                    else:
                        report = read(report_path)["report"]
                        row.update({"status": "compared", "native_elements": report["mercurio_count"], "pilot_elements": report["pilot_count"],
                                    "exact_pairs": report["exact_match_count"], "mismatched_pairs": len(report["mismatches"]),
                                    "native_only": len(report["mercurio_only"]), "pilot_only": len(report["pilot_only"]), "coverage": report["coverage"],
                                    "native_only_kinds": dict(collections.Counter(e["kind"] for e in report["mercurio_only"])),
                                    "pilot_only_kinds": dict(collections.Counter(e["kind"] for e in report["pilot_only"]))})
                        row["artifact"] = archive(report_path)
                    print(f"  {name}: {row['status']}", flush=True)
                    return row
                with ThreadPoolExecutor(max_workers=3) as comparisons:
                    group_rows = list(comparisons.map(compare_case, enumerate(group["cases"])))
                write(directory/"export-archive.json", archive(export))
            write(checkpoint, group_rows)
            summaries.extend(group_rows)
            write(out/"semantic-summary.json", {"total": len(summaries), "cases": summaries})
        pool.shutdown(wait=True)
        write(out/"semantic-summary.json", {"total": len(summaries), "cases": summaries})
    else:
        # Deterministic coverage selection: both languages, small/medium/large projects,
        # training, validation, and the two broad Simple Tests source sets.
        needles = ["AddressBookModel.kerml", "kerml/src/examples/Simple Tests/ArgumentResolution.kerml",
                   "sysml/src/examples/Simple Tests/ActionTest.sysml", "SysML v2 Spec Annex A SimpleVehicleModel.sysml",
                   "Part Definition Example.sysml", "Assignment Example.sysml", "State Definition Example-1.sysml", "Requirement Definitions.sysml"]
        selected = []
        for needle in needles:
            match = next(g for g in entries if any(c.endswith(needle) for c in g["cases"]))
            if match not in selected: selected.append(match)
        measurements = []
        for group_index, group in enumerate(selected):
            directory = out/"performance"/group["id"]
            directory.mkdir(parents=True, exist_ok=True)
            for repetition in range(3):
                # Alternate ordering to reduce systematic order bias; each is a fresh process.
                engines = ["native", "pilot"] if (repetition + group_index) % 2 == 0 else ["pilot", "native"]
                for engine in engines:
                    target = directory/f"{engine}-{repetition}.json"
                    command = ([native, "--assessment", group["spec"], target] if engine == "native" else java+[group["spec"], "-", target])
                    print(f"PERFORMANCE {group['id']} {engine} repetition {repetition+1}/3", flush=True)
                    execution = run(command, directory/f"{engine}-{repetition}.log", 900, env)
                    measurement = {"group": group["id"], "cases": group["cases"], "engine": engine, "repetition": repetition,
                                   "execution": execution, "timings": read(target) if target.exists() else None}
                    measurements.append(measurement)
                    write(out/"performance-results.json", {"method": "fresh process, OS filesystem cache uncontrolled/warm, optimized native/prebuilt stdlib, Pilot CheckMode.ALL with published .index.json enabled, graph export excluded", "measurements": measurements})


if __name__ == "__main__":
    main()
