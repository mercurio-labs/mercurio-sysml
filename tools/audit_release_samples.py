"""Audit ALL release samples with strict native compilation and Pilot CheckMode.ALL.
Measures acceptance parity, not complete OMG conformance or semantic equivalence.
"""
import argparse
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

SYSML = Path(__file__).resolve().parents[1]

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()

def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")

def discover(root):
    return sorted(p.relative_to(root).as_posix()
                  for folder in ("sysml/src", "kerml/src")
                  for p in (root / folder).rglob("*")
                  if p.is_file() and p.suffix in (".sysml", ".kerml"))

def corpus(root, paths, dependencies):
    """Identical source sets: siblings plus transitive declared dependencies."""
    cases = []
    for target in paths:
        inputs, pending = set(), [target]
        while pending:
            relative = pending.pop()
            if relative in inputs:
                continue
            if not (root / relative).is_file():
                raise ValueError(f"missing dependency: {relative}")
            inputs.add(relative)
            siblings = [p.relative_to(root).as_posix() for p in (root / relative).parent.iterdir()
                        if p.is_file() and p.suffix in (".sysml", ".kerml")]
            pending.extend(p for p in siblings + dependencies.get(relative, []) if p not in inputs)
        ordered = sorted(inputs - {target}) + [target]
        cases.append({"relative_path": target, "input_files": [str(root / p) for p in ordered]})
    return cases

def read_results(path):
    if not path.exists():
        return {}
    values = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            value = json.loads(line)
            name = value["relative_path"]
            if name in values:
                raise ValueError(f"duplicate result: {name}")
            values[name] = value
    return values

def execute(command, log, timeout):
    try:
        with log.open("w", encoding="utf-8") as output:
            run = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT, timeout=timeout)
        return {"exit_code": run.returncode}
    except subprocess.TimeoutExpired:
        return {"error": f"timeout after {timeout}s"}
    except OSError as error:
        return {"error": str(error)}

def complete_results(cases, output, run):
    results = read_results(output)
    for case in cases:
        name = case["relative_path"]
        if name not in results:
            results[name] = {"relative_path": name, "status": "infrastructure_error",
                             "error": f"missing result; process={run}"}
    return results

def batch_key(group):
    return json.dumps(sorted(group, key=lambda c: c["relative_path"]), sort_keys=True)


def summary(cases):
    valid = [c for c in cases if c["native"]["status"] in ("ok", "error")
             and c["pilot"]["status"] in ("ok", "error")]
    return {
        "files": len(cases),
        "completed_comparisons": len(valid),
        "native_pass": sum(c["native"]["status"] == "ok" for c in cases),
        "pilot_pass": sum(c["pilot"]["status"] == "ok" for c in cases),
        "both_pass": sum(c["native"]["status"] == c["pilot"]["status"] == "ok" for c in valid),
        "both_reject": sum(c["native"]["status"] == c["pilot"]["status"] == "error" for c in valid),
        "pilot_accepts_native_rejects": sum(c["native"]["status"] == "error" and c["pilot"]["status"] == "ok" for c in valid),
        "native_accepts_pilot_rejects": sum(c["native"]["status"] == "ok" and c["pilot"]["status"] == "error" for c in valid),
        "native_parse_pass": sum(c["native"].get("parse_ok") is True for c in cases),
        "pilot_parse_pass": sum(c["pilot"].get("parse_ok") is True for c in cases),
        "infrastructure_failures": len(cases) - len(valid),
    }

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release-root", required=True, type=Path)
    parser.add_argument("--pilot-root", required=True, type=Path)
    parser.add_argument("--pilot-jar", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--pilot-cache", type=Path, help="Reuse identical, fingerprint-checked Pilot input groups")
    parser.add_argument("--java-timeout", type=int, default=300)
    parser.add_argument("--native-timeout", type=int, default=3600)
    parser.add_argument("--paths-file", type=Path, help="Explicit subset; default discovers ALL samples")
    args = parser.parse_args()
    release, pilot, jar, out = [p.resolve() for p in (args.release_root, args.pilot_root, args.pilot_jar, args.out)]
    if args.workers < 1:
        parser.error("--workers must be positive")
    out.mkdir(parents=True, exist_ok=True)
    if (out / "corpus.json").exists():
        raise ValueError("output already contains an audit; choose a fresh directory")
    for root in (release, pilot):
        if git(root, "status", "--porcelain"):
            raise ValueError(f"upstream checkout must be clean: {root}")
    overrides = [name for name in ("MERCURIO_KERNEL_LIBRARY_PATH", "MERCURIO_SYSML_LIBRARY_PATH",
                 "MERCURIO_SYSML_DELTA_LIBRARY_PATH", "MERCURIO_STDLIB_PATH", "MERCURIO_STDLIB_LOCATOR")
                 if name in os.environ]
    if overrides:
        raise ValueError("unset standard-library overrides before auditing: " + ", ".join(overrides))
    paths = discover(release)
    if args.paths_file:
        paths = [p.strip() for p in args.paths_file.read_text(encoding="utf-8-sig").splitlines() if p.strip()]
    if not paths or len(paths) != len(set(paths)):
        raise ValueError("empty or duplicate corpus")
    seed = SYSML / "crates/mercurio-tools/corpus/pilot_corpus.seed.json"
    dependencies = json.loads(seed.read_text())["support_dependencies"]
    cases = corpus(release, paths, dependencies)
    inventory = [{"path": p, "sha256": digest(release / p),
                  "pilot_source_identical": (pilot / p).exists() and digest(pilot / p) == digest(release / p)}
                 for p in paths]
    input_paths = sorted({Path(p) for c in cases for p in c["input_files"]})
    write_json(out / "corpus.json", {"cases": cases})
    source_lock = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "release": {"commit": git(release, "rev-parse", "HEAD"), "tag": git(release, "describe", "--tags", "--exact-match")},
        "pilot": {"commit": git(pilot, "rev-parse", "HEAD"), "tag": git(pilot, "describe", "--tags", "--exact-match"),
                  "jar_sha256": digest(jar), "jar_name": jar.name},
        "mercurio": {name: {"commit": git(root, "rev-parse", "HEAD"), "status": git(root, "status", "--porcelain")}
                     for name, root in (("sysml", SYSML), ("foundation", SYSML.parent / "mercurio-foundation"))},
        "scope": "all release samples" if not args.paths_file else "explicit subset",
        "input_policy": "same-directory .sysml/.kerml files plus transitive corpus seed dependencies",
        "samples": inventory,
        "inputs": [{"path": p.relative_to(release).as_posix(), "sha256": digest(p)} for p in input_paths],
        "stdlib": [{"path": p.relative_to(release).as_posix(), "sha256": digest(p)}
                   for p in sorted((release / "sysml.library").rglob("*")) if p.suffix in (".sysml", ".kerml")],
        "native_resources": [{"path": p.relative_to(SYSML).as_posix(), "sha256": digest(p)}
                             for p in sorted((SYSML / "crates/mercurio-sysml/resources").rglob("*.json"))],
        "audit_sources": {str(p.relative_to(SYSML)): digest(p) for p in (
            Path(__file__), SYSML / "crates/mercurio-tools/src/bin/audit_release_compile.rs",
            SYSML / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotValidationAudit.java", seed)},
    }
    write_json(out / "source-lock.json", source_lock)
    classes = out / "classes"
    java_source = SYSML / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotValidationAudit.java"
    subprocess.run(["javac", "-cp", str(jar), "-d", str(classes), str(java_source)], check=True)
    executable = SYSML / "target/debug" / ("audit_release_compile.exe" if os.name == "nt" else "audit_release_compile")
    if not executable.is_file():
        raise ValueError("build audit_release_compile first")
    source_lock["native_binary_sha256"] = digest(executable)
    write_json(out / "source-lock.json", source_lock)
    groups = defaultdict(list)
    for case in cases:
        groups[tuple(sorted(case["input_files"]))].append(case)
    cached_groups = {}
    if args.pilot_cache:
        cache = args.pilot_cache.resolve()
        previous = json.loads((cache / "source-lock.json").read_text())
        for field in ("pilot", "inputs", "stdlib"):
            if previous[field] != source_lock[field]:
                raise ValueError(f"Pilot cache fingerprint mismatch: {field}")
        shim = str(java_source.relative_to(SYSML))
        if previous["audit_sources"][shim] != source_lock["audit_sources"][shim]:
            raise ValueError("Pilot cache shim fingerprint mismatch")
        for spec_path in (cache / "pilot").glob("*/spec.json"):
            group = json.loads(spec_path.read_text())["cases"]
            results = read_results(spec_path.parent / "results.jsonl")
            if all(c["relative_path"] in results and results[c["relative_path"]]["status"] in ("ok", "error") for c in group):
                cached_groups[batch_key(group)] = (results, spec_path.parent)
        source_lock["pilot_cache"] = str(cache)
        write_json(out / "source-lock.json", source_lock)
    print(f"Auditing {len(cases)} files in {len(groups)} isolated input sets", flush=True)

    def native():
        output = out / "native.jsonl"
        run = execute([str(executable), str(out / "corpus.json"), str(output)], out / "native.log", args.native_timeout)
        return complete_results(cases, output, run)

    def oracle(index, group):
        directory = out / "pilot" / f"{index:04d}"
        directory.mkdir(parents=True)
        # Validate smaller targets first so a large target timeout cannot hide
        # results for the rest of its input set. All files are loaded up front.
        ordered = sorted(group, key=lambda c: (Path(c["input_files"][-1]).stat().st_size, c["relative_path"]))
        write_json(directory / "spec.json", {"cases": ordered})
        output = directory / "results.jsonl"
        cached = cached_groups.get(batch_key(group))
        if cached:
            results, origin = cached
            output.write_text("".join(json.dumps(results[c["relative_path"]]) + "\n" for c in group), encoding="utf-8")
            (directory / "run.log").write_text(f"Fingerprint-checked reuse from {origin}\n", encoding="utf-8")
            print(f"Pilot {index + 1}/{len(groups)}: reused {len(group)} files", flush=True)
            return results
        run = execute(["java", "-Xmx2g", "-cp", os.pathsep.join((str(classes), str(jar))),
                       "dev.mercurio.pilot.PilotValidationAudit", str(release / "sysml.library"),
                       str(directory / "spec.json"), str(output)], directory / "run.log", args.java_timeout)
        results = complete_results(group, output, run)
        print(f"Pilot {index + 1}/{len(groups)}: {len(group)} files, {Counter(r['status'] for r in results.values())}", flush=True)
        return results

    with ThreadPoolExecutor(max_workers=1) as native_pool, ThreadPoolExecutor(max_workers=args.workers) as pool:
        native_run = native_pool.submit(native)
        futures = [pool.submit(oracle, i, group) for i, group in enumerate(groups.values())]
        pilot_results = {}
        for future in as_completed(futures):
            pilot_results.update(future.result())
        native_results = native_run.result()
    joined = [{"path": p, "native": native_results[p], "pilot": pilot_results[p]} for p in paths]
    totals = summary(joined)
    categories = sorted({"/".join(c["path"].split("/")[:3]) for c in joined})
    by_category = {category: summary([c for c in joined if c["path"].startswith(category + "/")]) for category in categories}
    parity = (totals["infrastructure_failures"] == 0 and totals["pilot_accepts_native_rejects"] == 0
              and totals["native_accepts_pilot_rejects"] == 0)
    result = {"scope": source_lock["scope"], "summary": totals, "by_category": by_category,
              "acceptance_parity": parity, "full_specification_conformance": "not_established",
              "semantic_equivalence": "not_established", "cases": joined}
    write_json(out / "audit.json", result)
    rows = ["# SysML v2 release sample audit", "", f"Scope: {source_lock['scope']}.",
            "", f"Acceptance parity: **{'PASS' if parity else 'FAIL'}**.",
            "Full specification conformance and semantic equivalence are not established.",
            "", "| Corpus | Files | Native pass | Pilot pass | Pilot only accepts | Native only accepts | Infrastructure failures |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for category, v in [*by_category.items(), ("TOTAL", totals)]:
        rows.append(f"| {category} | {v['files']} | {v['native_pass']} | {v['pilot_pass']} | {v['pilot_accepts_native_rejects']} | {v['native_accepts_pilot_rejects']} | {v['infrastructure_failures']} |")
    rows += ["", "Native uses its shipped baseline and strict parser/resolver/lowering APIs. Pilot uses the release's textual standard library and CheckMode.ALL.",
             "Cases are folder-scoped with explicit dependencies; validation fixtures and alternate examples may intentionally fail.",
             "Warnings do not fail Pilot. Crashes, timeouts, and missing results are infrastructure failures, never parity.",
             "All raw diagnostics, hashes, source sets, and process logs are retained beside this report.",
             "", "## Differences", ""]
    for c in joined:
        if c["native"]["status"] != c["pilot"]["status"] or "infrastructure_error" in (c["native"]["status"], c["pilot"]["status"]):
            messages = [d["message"] for d in c["native"].get("diagnostics", [])]
            messages += [d["message"] for d in c["pilot"].get("diagnostics", []) if d.get("severity") == "ERROR"]
            message = (messages or [c["native"].get("error") or c["pilot"].get("error", "")])[0]
            rows.append(f"- {c['path']}: native={c['native']['status']}, Pilot={c['pilot']['status']}. {message}")
    (out / "audit.md").write_text("\n".join(rows) + "\n", encoding="utf-8")
    print(json.dumps(totals, indent=2), flush=True)
    return 0 if parity else 1

if __name__ == "__main__":
    sys.exit(main())
