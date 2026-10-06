"""Pinned owned-cross-feature selection and binary crossing constructor observations."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
ROOT = Path(__file__).resolve().parents[1]
PIN = "692170b71867353b8f90341e61556f49a5beb0e5"
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotCrossFeatureProbe.java"
OUTPUT = ROOT / "docs/conformance/2026-08-support/cross-feature-pilot-controls.json"
def digest(data): return hashlib.sha256(data).hexdigest()
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin",type=Path,required=True)
    parser.add_argument("--check",action="store_true")
    args=parser.parse_args()
    pilot=ROOT.parent / "target/support-2026-08/pilot-pinned-2026-08"
    def git(*args): return subprocess.check_output(["git","-C",str(pilot),*args])
    if git("rev-parse","HEAD").decode().strip()!=PIN: raise ValueError("Unexpected Pilot revision")
    sources={}
    for name in ["util/FeatureUtil.java","util/TypeUtil.java","delegate/setting/Type_endFeature_SettingDelegate.java","delegate/setting/Type_feature_SettingDelegate.java","adapter/FeatureAdapter.java","adapter/TypeAdapter.java","delegate/setting/Feature_crossFeature_SettingDelegate.java"]:
        path="org.omg.sysml.logic/src/main/java/org/omg/sysml/"+name
        data=(pilot/path).read_bytes().replace(b"\r\n",b"\n")
        if data!=git("show",PIN+":"+path).replace(b"\r\n",b"\n"): raise ValueError("Changed pinned source "+path)
        sources[path]=digest(data)
    for path in ["org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerML.xtext", "org.omg.kerml.expressions.xtext/src/org/omg/kerml/expressions/xtext/KerMLExpressions.xtext"]:
        data=(pilot/path).read_bytes().replace(b"\r\n",b"\n")
        if data!=git("show",PIN+":"+path).replace(b"\r\n",b"\n"): raise ValueError("Changed pinned grammar "+path)
        sources[path]=digest(data)
    jar=pilot/"org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    provenance={"pilot_commit":PIN,"source_sha256":sources,"jar_sha256":digest(jar.read_bytes()),"helper_sha256":digest(HELPER.read_bytes()),"driver_sha256":digest(Path(__file__).read_bytes())}
    suffix=".exe" if os.name=="nt" else ""
    provenance["toolchain"]={name:subprocess.check_output([str(args.java_bin/(name+suffix)),"-version"],stderr=subprocess.STDOUT,text=True).strip() for name in ["java","javac"]}
    with tempfile.TemporaryDirectory(prefix="cross-feature-") as temp:
        subprocess.run([str(args.java_bin/("javac"+suffix)),"-encoding","UTF-8","-cp",str(jar),"-d",temp,str(HELPER)],check=True,timeout=60)
        output=Path(temp)/"result.json"
        subprocess.run([str(args.java_bin/("java"+suffix)),"-cp",temp+os.pathsep+str(jar),"dev.mercurio.pilot.PilotCrossFeatureProbe",str(output)],check=True,timeout=60)
        result=json.loads(output.read_text(encoding="utf-8"))
    if len(result["cross_feature_controls"])!=8 or len(result["explicit_controls"])!=6 or len(result["typing_controls"])!=6 or len(result["featuring_controls"])!=24 or len(result["specialization_controls"])!=12 or len(result["selection_controls"])!=72 or len(result["binary_controls"])!=8 or len(result["source_controls"])!=6 or len(result["ordered_selection_controls"])!=3: raise ValueError("Changed control inventory")
    if provenance["jar_sha256"]!=digest(jar.read_bytes()) or provenance["helper_sha256"]!=digest(HELPER.read_bytes()): raise ValueError("Changed probe inputs")
    result.update(schema="dev.mercurio.cross-feature-controls.v1",provenance=provenance,scope="Owned cross-feature selection and binary addCrossingSpecialization, and owned-cross specialization and binary featuring with complete supplied owner generalizations. Six type-query controls additionally cover incomplete Class-owned crossing ends without a typed library default; they do not qualify full generalization or real-library transformation. Six explicit source-crossing controls cover owned chain lengths one through three and both end flags; eight crossFeature presence controls cover exact Feature absence and the eligible binary prerequisite. Neither group qualifies complete validation or subclass adapters. Six source-stage projections qualify the binary construction step only; these observations do not qualify complete source transformation, unresolved general owner typing/featuring, n-ary construction or complete library semantics.")
    text=json.dumps(result,indent=2,sort_keys=True)+"\n"
    if args.check:
        if OUTPUT.read_text(encoding="utf-8")!=text: raise ValueError("Stale cross-feature observations")
    else: OUTPUT.write_text(text,encoding="utf-8",newline="\n")
    print("Pinned cross-feature controls: 75 selections, 8 binary contributions, 12 specialization controls, 24 binary featuring controls, 6 incomplete crossing-end typing controls, 6 explicit crossing controls, 8 cross-feature presence controls, and 6 source-stage projections")
if __name__=="__main__": main()
