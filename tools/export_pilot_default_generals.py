"""Export resolved plain-Type defaults and observe native-provider dependency controls."""
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
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotDefaultGeneralProbe.java"
OUTPUT = PROFILE / "default-general-pilot-controls.json"

def digest(data):
    return hashlib.sha256(data).hexdigest()

EXPECTED = {"Type","Classifier","Class","DataType","Structure","Behavior","Function","Predicate","Metaclass","Definition","AttributeDefinition","EnumerationDefinition","PortDefinition","ConjugatedPortDefinition"}
EXPECTED_SELECTOR_SHA256 = '2cdf3be8c4703c4604f8eefd258daa27eb4628bb221b38c740528739d76d2953'
def validate(doc):
    if hashlib.sha256(json.dumps(doc["methods"],sort_keys=True,separators=(",",":")).encode()).hexdigest()!=EXPECTED_SELECTOR_SHA256: raise ValueError("Changed complete typed selector program")
    rows=doc["bindings"]
    if len(rows)!=len(EXPECTED) or {r["kind"] for r in rows}!=EXPECTED: raise ValueError("Changed resolved plain-Type binding inventory")
    required={"getDefaultSupertype":"org.omg.sysml.adapter.TypeAdapter#getDefaultSupertype","addDefaultGeneralType":"org.omg.sysml.adapter.TypeAdapter#addDefaultGeneralType","getInheritedMemberships":"org.omg.sysml.adapter.TypeAdapter#getInheritedMemberships","getGeneralTypesOf":"org.omg.sysml.util.TypeUtil#getGeneralTypesOf"}
    for row in rows:
        if any(row["dispatch"].get(k)!=v for k,v in required.items()):raise ValueError("Changed shared algorithm dispatch")
        if not row["qualified_name"] or row["specialization"] not in {"Specialization","Subclassification"}:raise ValueError("Invalid plain-Type mapping")
    body=doc["methods"]["org.omg.sysml.adapter.TypeAdapter#getDefaultSupertype"]
    expression=body["statements"][0]["expression"]
    if len(body["statements"])!=1 or expression.get("symbol")!="org.omg.sysml.adapter.TypeAdapter#getDefaultSupertype" or expression.get("arguments")!=[{"kind":"STRING_LITERAL","type":"java.lang.String","value":"base"}]:raise ValueError("Changed resolved base selector")
    controls=doc["controls"]
    if len(controls)!=7*len(EXPECTED) or {(c["kind"],c["mode"]) for c in controls}!={(k,m) for k in EXPECTED for m in range(7)}:raise ValueError("Incomplete or duplicate controls")
    if EXPECTED.intersection(doc["excluded_bindings"]):raise ValueError("Conflicting provider inventory")

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    def git(*command):
        with tempfile.TemporaryDirectory(prefix="plain-type-git-") as directory:
            config=Path(directory)/"gitconfig"
            config.write_text('[safe]\n\tdirectory = '+PILOT.resolve().as_posix()+'\n',encoding="utf-8")
            return subprocess.check_output(["git","-C",str(PILOT),*command],env=dict(os.environ,GIT_CONFIG_GLOBAL=str(config)))
    revision = git("rev-parse", "HEAD").decode().strip()
    if revision != PIN:
        raise ValueError("Changed Pilot revision")
    runtime = PILOT / "org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    reference = json.loads((PROFILE / "xtext-prediction-nfa.experimental.json").read_text(encoding="utf-8"))
    if digest(runtime.read_bytes()) != reference["provenance"]["runtime_sha256"]:
        raise ValueError("Changed pinned runtime")
    suffix = ".exe" if os.name == "nt" else ""
    with tempfile.TemporaryDirectory(prefix="default-general-") as temp:
        raw = Path(temp) / "raw.json"
        subprocess.run([str(args.java_bin / ("javac"+suffix)), "-encoding", "UTF-8", "-cp", str(runtime), "-d", temp, str(HELPER), str(ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotStepStrategyProbe.java")], check=True, timeout=60)
        subprocess.run([str(args.java_bin / ("java"+suffix)), "-cp", temp+os.pathsep+str(runtime), "dev.mercurio.pilot.PilotDefaultGeneralProbe", str(raw), str(runtime), str(PILOT/"org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/TypeAdapter.java")], check=True, timeout=60)
        doc = json.loads(raw.read_text(encoding="utf-8"))
    prefix = "org.omg.sysml.logic/src/main/java/org/omg/sysml/"
    paths = {p.relative_to(PILOT).as_posix() for p in (PILOT/(prefix+"adapter")).glob("*Adapter.java")}
    paths.update(prefix+"util/"+kind+".java" for kind in ["ImplicitGeneralizationMap","TypeUtil","SysMLLibraryUtil"])
    paths.add("org.omg.sysml/model/SysML.ecore")
    hashes = {}
    for path in sorted(paths):
        content = (PILOT/path).read_bytes().replace(b"\r\n",b"\n")
        pinned = git("show",PIN+":"+path).replace(b"\r\n",b"\n")
        if content != pinned: raise ValueError("Changed source: "+path)
        hashes[path] = digest(content)
    doc = {"schema_version":1, "scope":"Resolved TypeAdapter default-selector tree and dispatch-driven bindings; actual Pilot TypeUtil general-type and inherited-membership stage controls. Library resolution is an explicit supplied dependency. Metadata-derived bases, conditional subtype algorithms and global lookup are not qualified.", **doc,
        "provenance":{"pilot_revision":PIN,"runtime_sha256":digest(runtime.read_bytes()),"source_sha256":hashes,"helper_sha256":digest(HELPER.read_bytes()),"driver_sha256":digest(Path(__file__).read_bytes()),"tree_helper_sha256":digest((ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotStepStrategyProbe.java").read_bytes()),"java_sha256":digest((args.java_bin/("java"+suffix)).read_bytes()),"javac_sha256":digest((args.java_bin/("javac"+suffix)).read_bytes()),"java_version":subprocess.check_output([str(args.java_bin/("java"+suffix)),"-version"],stderr=subprocess.STDOUT).decode().strip()}}
    validate(doc)
    lines = ["// Generated by tools/export_pilot_default_generals.py; do not edit.",
             "pub(super) const DEFAULT_GENERALS: &[(&str, &str, &str)] = &["]
    for row in doc["bindings"]:
        lines.append("    ("+", ".join(json.dumps(row[key]) for key in ["kind","qualified_name","specialization"])+"),")
    lines.append("];\n")
    generated = ROOT/"crates/mercurio-sysml/src/language_frontend/lowering/emit/default_generals_generated.rs"
    native = "\n".join(lines)
    if args.check:
        if generated.read_text(encoding="utf-8") != native: raise ValueError("Stale default general dispatch")
    else: generated.write_text(native,encoding="utf-8",newline="\n")
    rendered = json.dumps(doc,indent=2,sort_keys=True)+"\n"
    if args.check:
        if OUTPUT.read_text(encoding="utf-8") != rendered: raise ValueError("Stale default-general controls")
    else: OUTPUT.write_text(rendered,encoding="utf-8",newline="\n")
    print(len(doc["bindings"]),"resolved defaults;",len(doc["controls"]),"independent general-type controls")

if __name__ == "__main__": main()
