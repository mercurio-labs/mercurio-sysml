"""Observe ordinary Feature default selection under pinned concrete Classifiers.

This build-time Pilot probe does not generate or qualify native implementation.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from export_pilot_feature_redefinitions import ROOT, PROFILE, PILOT, PIN, require

HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotClassifierFeatureDefaults.java"
OUTPUT = ROOT / "docs/conformance/2026-08-support/classifier-feature-default-pilot-controls.json"

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    require(subprocess.check_output(["git", "-c", "safe.directory="+str(PILOT), "-C", str(PILOT), "rev-parse", "HEAD"], text=True).strip()==PIN, "Changed Pilot revision")
    runtime = PILOT / "org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    reference = json.loads((PROFILE / "xtext-prediction-nfa.experimental.json").read_text(encoding="utf-8"))
    require(digest(runtime)==reference["provenance"]["runtime_sha256"], "Changed Pilot runtime")
    suffix = ".exe" if os.name=="nt" else ""
    java, javac = [args.java_bin/(name+suffix) for name in ("java", "javac")]
    with tempfile.TemporaryDirectory(prefix="classifier-feature-defaults-") as temp:
        raw = Path(temp)/"controls.json"
        subprocess.run([str(javac), "-encoding", "UTF-8", "-cp", str(runtime), "-d", temp, str(HELPER)], check=True, timeout=60)
        subprocess.run([str(java), "-cp", temp+os.pathsep+str(runtime), "dev.mercurio.pilot.PilotClassifierFeatureDefaults", str(raw)], check=True, timeout=90)
        doc = json.loads(raw.read_text(encoding="utf-8"))
    owners = doc["concrete_classifier_owners"]
    require(len(owners)==len(set(owners)) and len(doc["controls"])==32*len(owners), "Incomplete owner controls")
    doc["provenance"] = {
        "pilot_revision": PIN, "runtime_sha256": digest(runtime),
        "helper_sha256": digest(HELPER), "driver_sha256": digest(Path(__file__)),
        "ecore_representation_sha256": digest(PROFILE / "ecore-effective.extract.json"),
        "selector_representation_sha256": digest(PROFILE / "feature-defaults.extract.json"),
        "toolchain": {name: subprocess.check_output([str(exe), "-version"], stderr=subprocess.STDOUT, text=True).strip() for name,exe in [("java",java),("javac",javac)]}
    }
    serialized = json.dumps(doc, indent=2, sort_keys=True)+"\n"
    if args.check:
        require(OUTPUT.read_text(encoding="utf-8")==serialized, "Stale classifier owner controls")
    else:
        OUTPUT.write_text(serialized, encoding="utf-8")
    print(f"{len(owners)} concrete Classifier owners; {len(doc['controls'])} independent selector observations; resource/lifecycle qualification separate")

if __name__ == "__main__":
    main()
