"""Prepare and summarize a native candidate census; never launch Cargo or Pilot.

Preparation preserves the existing release source sets. This is a diagnostic
first-blocker census, not a conformance certificate or complete dependency audit.
"""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
import os
from pathlib import Path

from audit_release_samples import corpus, discover

ROOT = Path(__file__).resolve().parents[1]
LIMITATIONS = [
    "Construction, linking and publication are one aggregate stage in the current candidate API.",
    "Only the first blocker per source set is observed; downstream dependencies remain unknown.",
    "Original sibling/dependency source sets are preserved; standard library closure is not supplied, and complete global resource semantics remain unqualified.",
    "The candidate parses each resource with its own suffix-selected grammar, preserves separate roots and links the source set transactionally; this does not qualify complete global scoping or resource loading.",
    "Candidate execution has resource limits, including a 1 MiB source bound; those are not syntax rejections.",
    "Constructed models remain semantically unqualified; no Pilot comparison or timing qualification is performed.",
    "Prepared hashes and result identities do not attest that a binary was built from these sources or that an execution used them.",
]
STAGES = {"syntax", "lexical", "unsupported", "resource_limit", "artifact", "construction_or_linking"}


def read_bytes(path):
    path = Path(path).resolve()
    # Windows checkouts contain legitimate paths longer than MAX_PATH.
    if os.name == "nt" and not str(path).startswith("\\\\?\\"):
        path = Path("\\\\?\\" + str(path))
    return path.read_bytes()


def digest(path):
    return hashlib.sha256(read_bytes(path)).hexdigest()


def read_json(path):
    return json.loads(read_bytes(path).decode("utf-8-sig"))


def candidate_inputs():
    """Native source/resource fingerprint, not build or dependency attestation."""
    files = {Path(__file__), ROOT / "tools/audit_release_samples.py",
             ROOT / "crates/mercurio-tools/src/bin/audit_release_compile.rs"}
    for workspace in (ROOT, ROOT.parent / "mercurio-foundation"):
        for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rust-toolchain"):
            if (workspace / name).is_file():
                files.add(workspace / name)
        for crate in (workspace / "crates").iterdir():
            if not crate.is_dir():
                continue
            for name in ("Cargo.toml", "build.rs"):
                if (crate / name).is_file():
                    files.add(crate / name)
            for folder in ("src", "resources"):
                files.update(p for p in (crate / folder).rglob("*") if p.is_file())
    return sorted(files)


def prepare(corpus_path, release_root, dependencies_path, implementation_paths, allow_subset=False):
    release_root = Path(release_root).resolve()
    original = read_json(corpus_path)
    cases = original.get("cases", [])
    if not cases:
        raise ValueError("Empty census corpus")
    names = [case["relative_path"] for case in cases]
    if len(set(names)) != len(names):
        raise ValueError("Duplicate census targets")
    discovered = set(discover(release_root))
    if not set(names) <= discovered:
        raise ValueError("Census target is outside discovered release samples")
    if not allow_subset and set(names) != discovered:
        raise ValueError("Census does not contain every discovered release sample")
    dependencies = read_json(dependencies_path)["support_dependencies"]
    expected = corpus(release_root, names, dependencies)
    all_files = set()
    normalized = []
    for old, new in zip(cases, expected):
        supplied = [Path(p).resolve() for p in old["input_files"]]
        required = [Path(p).resolve() for p in new["input_files"]]
        if supplied != required:
            raise ValueError("Changed source grouping/order: " + old["relative_path"])
        for path in supplied:
            if not path.is_file():
                raise ValueError("Missing source input: " + str(path))
            path.relative_to(release_root)  # Reject escaped or linked external sources.
            all_files.add(path)
        normalized.append({"relative_path": old["relative_path"],
                           "input_files": [str(p) for p in supplied]})
    inventory = [{"path": str(p), "release_path": p.relative_to(release_root).as_posix(),
                  "sha256": digest(p), "bytes": len(read_bytes(p))} for p in sorted(all_files)]
    library_inventory = [{"path": str(p.resolve()), "release_path": p.relative_to(release_root).as_posix(),
                          "sha256": digest(p), "bytes": len(read_bytes(p))}
                         for p in sorted((release_root / "sysml.library").rglob("*"))
                         if p.is_file() and p.suffix in (".sysml", ".kerml")]
    provenance_paths = {Path(corpus_path).resolve(), Path(dependencies_path).resolve()}
    provenance_paths.update(Path(p).resolve() for p in implementation_paths)
    lock = {"schema": "dev.mercurio.definition-census-inputs.v1", "release_root": str(release_root),
            "case_count": len(normalized), "discovered_sample_count": len(discovered),
            "source_set_count": len({tuple(sorted(c["input_files"])) for c in normalized}),
            "target_language_counts": dict(sorted(Counter(Path(c["relative_path"]).suffix[1:] for c in normalized).items())),
            "library_inventory": library_inventory,
            "library_scope": "Recorded and hashed, not loaded by the candidate runner; resource closure remains unqualified",
            "scope": "explicit_subset" if set(names) != discovered else "all_discovered_release_samples",
            "cases": normalized, "inputs": inventory,
            "provenance_sha256": {str(p): digest(p) for p in sorted(provenance_paths)},
            "limitations": LIMITATIONS,
            "execution": {"program": "audit_release_compile", "mode": "--definition-candidate",
                          "semantic_qualification": "not_assessed"}}
    return {"cases": normalized}, lock


def validate_fingerprints(lock):
    for row in lock["inputs"] + lock["library_inventory"]:
        if digest(row["path"]) != row["sha256"]:
            raise ValueError("Changed census source: " + row["path"])
    for path, expected in lock["provenance_sha256"].items():
        if digest(path) != expected:
            raise ValueError("Changed census provenance: " + path)


def summarize(lock, rows):
    expected = {c["relative_path"]: c for c in lock["cases"]}
    observed = {}
    for row in rows:
        name = row["relative_path"]
        if name not in expected or name in observed:
            raise ValueError("Unexpected or duplicate census result: " + name)
        if row.get("input_files") != expected[name]["input_files"]:
            raise ValueError("Result source set differs from prepared census: " + name)
        if row.get("semantic_qualification") != "not_assessed":
            raise ValueError("Result makes unsupported semantic qualification claim: " + name)
        status, stage = row.get("status"), row.get("stage")
        if status == "blocked":
            if stage not in STAGES or not isinstance(row.get("error"), str) or not row["error"]:
                raise ValueError("Invalid candidate blocker: " + name)
        elif status == "constructed_unqualified":
            if stage is not None or not isinstance(row.get("element_count"), int) or row["element_count"] < 0:
                raise ValueError("Invalid constructed candidate result: " + name)
        elif status == "infrastructure_error":
            if stage is not None or not row.get("error"):
                raise ValueError("Invalid infrastructure result: " + name)
        else:
            raise ValueError("Unknown or qualified candidate status: " + name)
        observed[name] = row
    missing = sorted(set(expected) - set(observed))
    if missing:
        raise ValueError("Missing census results: " + ", ".join(missing))
    groups = defaultdict(list)
    for name, row in observed.items():
        if row["status"] != "constructed_unqualified":
            groups[(row.get("stage", "infrastructure"), row["error"])].append(name)
    return {"schema": "dev.mercurio.definition-census-summary.v1", "case_count": len(observed),
            "statuses": dict(sorted(Counter(r["status"] for r in rows).items())),
            "first_blocker_stages": dict(sorted(Counter(r["stage"] for r in rows if r["status"] == "blocked").items())),
            "candidate_parse_completed": sum(r["status"] == "constructed_unqualified" or r.get("stage") == "construction_or_linking" for r in rows),
            "candidate_syntax_rejections": sum(r.get("stage") == "syntax" for r in rows),
            "semantically_qualified_cases": 0,
            "reported_source_set_cache_hits": sum(r.get("source_set_cache_hit") is True for r in rows),
            "cache_scope": "Within-process exact ordered resource paths and text; no persistent qualification cache",
            "first_blocker_groups": [{"stage": stage, "message": message, "cases": sorted(names), "count": len(names)}
                                     for (stage, message), names in sorted(groups.items(), key=lambda pair: (-len(pair[1]), pair[0]))],
            "limitations": LIMITATIONS}


def write_new(path, value):
    with Path(path).open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    prep = commands.add_parser("prepare")
    prep.add_argument("--corpus", required=True, type=Path)
    prep.add_argument("--release-root", required=True, type=Path)
    prep.add_argument("--dependencies", type=Path, default=ROOT / "crates/mercurio-tools/corpus/pilot_corpus.seed.json")
    prep.add_argument("--out", required=True, type=Path)
    prep.add_argument("--allow-subset", action="store_true")
    report = commands.add_parser("summarize")
    report.add_argument("--lock", required=True, type=Path)
    report.add_argument("--results", required=True, type=Path)
    report.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    if args.command == "prepare":
        manifest, lock = prepare(args.corpus, args.release_root, args.dependencies, candidate_inputs(), args.allow_subset)
        if any((args.out / name).exists() for name in ("corpus.json", "input-lock.json")):
            raise ValueError("Census already prepared; choose a fresh output directory")
        args.out.mkdir(parents=True, exist_ok=True)
        write_new(args.out / "corpus.json", manifest)
        write_new(args.out / "input-lock.json", lock)
        print(f"Prepared {lock['case_count']} cases in {lock['source_set_count']} unique source sets; recorded {len(lock['library_inventory'])} library files without loading them. No native or Pilot execution performed")
    else:
        lock = read_json(args.lock)
        if lock.get("schema") != "dev.mercurio.definition-census-inputs.v1":
            raise ValueError("Unsupported census input schema")
        validate_fingerprints(lock)
        rows = [json.loads(line) for line in read_bytes(args.results).decode("utf-8").splitlines() if line.strip()]
        result = summarize(lock, rows)
        result["evidence_sha256"] = {"input_lock": digest(args.lock), "results": digest(args.results)}
        write_new(args.out, result)
        print(f"Summarized {result['case_count']} candidate cases; semantic qualification remains unassessed")


if __name__ == "__main__":
    main()
