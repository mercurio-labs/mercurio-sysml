"""Wrap raw Pilot execution observations with reproducible provenance and explicit decisions."""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
for name in ("pilot-root", "fixture", "raw", "jar", "helper", "class-file", "out"):
    p.add_argument("--" + name, type=Path, required=True)
p.add_argument("--check", action="store_true")
a = p.parse_args()
def git(*args):
    return subprocess.check_output(["git", "-C", str(a.pilot_root), *args], text=True).strip()
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
if git("status", "--porcelain"):
    raise ValueError("Pilot checkout must be clean")
fixture = json.loads(a.fixture.read_text(encoding="utf-8"))
rows = json.loads(a.raw.read_text(encoding="utf-8"))
if [(r["id"], r["expression"]) for r in rows] != [(r["id"], r["expression"]) for r in fixture]:
    raise ValueError("Execution results do not match fixture order and expressions")
source = "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/EvaluationUtil.java"
result = {"pilot_commit": git("rev-parse", "HEAD"), "fixture_sha256": digest(a.fixture), "runtime_sha256": digest(a.jar), "helper_sha256": digest(a.helper), "class_sha256": digest(a.class_file), "raw_sha256": digest(a.raw), "observations": rows, "sources": {source: digest(a.pilot_root / source)}}
# Retain the reviewed explicit decisions as data, never adjust oracle results.
existing = json.loads(a.out.read_text(encoding="utf-8"))
for key in ("known_divergences", "oracle_limitations", "status"):
    result[key] = existing[key]
if a.check:
    if result != existing:
        raise ValueError("Execution evidence drift")
else:
    a.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
print(f"{len(rows)} raw execution observations verified; errors and divergences retained")
