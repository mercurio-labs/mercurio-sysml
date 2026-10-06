"""Observe pinned EMF required stored-feature multiplicities on fresh concrete objects."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
PILOT = ROOT.parent / "target/support-2026-08/pilot-pinned-2026-08"
PIN = "692170b71867353b8f90341e61556f49a5beb0e5"
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotRequiredFeatureProbe.java"
OUTPUT = PROFILE / "required-features-pilot-controls.json"

def digest(data):
    return hashlib.sha256(data).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    revision = subprocess.check_output(["git", "-C", str(PILOT), "rev-parse", "HEAD"], text=True).strip()
    if revision != PIN:
        raise ValueError("Changed Pilot revision")
    runtime = PILOT / "org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    reference = json.loads((PROFILE / "xtext-prediction-nfa.experimental.json").read_text(encoding="utf-8"))
    if digest(runtime.read_bytes()) != reference["provenance"]["runtime_sha256"]:
        raise ValueError("Changed pinned runtime")
    suffix = ".exe" if os.name == "nt" else ""
    with tempfile.TemporaryDirectory(prefix="required-features-") as temp:
        raw = Path(temp) / "raw.json"
        subprocess.run([str(args.java_bin / ("javac"+suffix)), "-encoding", "UTF-8", "-cp", str(runtime), "-d", temp, str(HELPER)], check=True, timeout=60)
        subprocess.run([str(args.java_bin / ("java"+suffix)), "-cp", temp+os.pathsep+str(runtime), "dev.mercurio.pilot.PilotRequiredFeatureProbe", str(raw)], check=True, timeout=60)
        doc = json.loads(raw.read_text(encoding="utf-8"))
    paths = {'org.omg.sysml/model/SysML.ecore'}
    hashes = {}
    for path in sorted(paths):
        content = (PILOT/path).read_bytes().replace(b"\r\n",b"\n")
        pinned = subprocess.check_output(["git","-C",str(PILOT),"show",PIN+":"+path]).replace(b"\r\n",b"\n")
        if content != pinned: raise ValueError("Changed source: "+path)
        hashes[path] = digest(content)
    doc = {"schema_version":1, "scope":"Actual EMF per-feature multiplicity validation of required nonderived/nonvolatile features on fresh concrete Pilot objects. Native unverified defaults/delegates remain separate from matching observations. Not whole-model validation.", "controls":doc,
           "provenance":{"pilot_revision":PIN,"runtime_sha256":digest(runtime.read_bytes()),"ecore_sha256":digest((PROFILE/"ecore-effective.extract.json").read_bytes()),"source_sha256":hashes,"helper_sha256":digest(HELPER.read_bytes()),"driver_sha256":digest(Path(__file__).read_bytes())}}
    rendered = json.dumps(doc,indent=2,sort_keys=True)+"\n"
    if args.check:
        if OUTPUT.read_text(encoding="utf-8") != rendered: raise ValueError("Stale required-feature controls")
    else: OUTPUT.write_text(rendered,encoding="utf-8",newline="\n")
    print(len(doc["controls"]),"EMF required-feature observations; native dependencies remain explicit")

if __name__ == "__main__": main()
