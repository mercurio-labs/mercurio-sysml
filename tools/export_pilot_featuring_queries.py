"""Import resolved result-constructor definitions; generate bounded native dispatch."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from export_pilot_feature_redefinitions import ROOT, PROFILE, PILOT, PIN, require
HELPER=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotFeaturingQueryExporter.java"
TREE_HELPER=HELPER.with_name("PilotResultConstructionExporter.java")
OUTPUT=PROFILE/"featuring-queries.extract.json"
GENERATED=ROOT/"crates/mercurio-sysml/src/language_frontend/lowering/emit/featuring_queries_generated.rs"
SOURCES=["util/FeatureUtil.java","adapter/FeatureAdapter.java","delegate/setting/Feature_featuringType_SettingDelegate.java"]

def sha(data): return hashlib.sha256(data).hexdigest()

DEFINITION_SHA="efba13b461a36007ea370986fdeefabb360e2aeb63710fd25f99a527bdf4bd57"
def render(doc):
    require(sha(json.dumps(doc["definitions"],sort_keys=True,separators=(",",":")).encode())==DEFINITION_SHA,"Changed resolved featuring definition")
    require(len(doc["cases"])==10 and {r["shape"] for r in doc["cases"]}=={"empty","direct","duplicate","breadth","diamond","cycle","self","first_chain","value","computed_owner"},"Incomplete featuring contexts")
    delegate=next(key.split("#")[0] for key in doc["definitions"] if key.endswith("#basicGet"))
    return "// Generated from pinned resolved featuring definitions; do not edit.\n"+"pub(super) const DELEGATE: &str = "+json.dumps("org.omg.sysml.delegate.setting."+delegate)+";\n"

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--java-bin",type=Path,required=True);parser.add_argument("--check",action="store_true");args=parser.parse_args()
    require(subprocess.check_output(["git","-C",str(PILOT),"rev-parse","HEAD"],text=True).strip()==PIN,"Pilot pin changed")
    jar=PILOT/"org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    expected=json.loads((PROFILE/"xtext-prediction-nfa.experimental.json").read_text())["provenance"]["runtime_sha256"]
    require(sha(jar.read_bytes())==expected,"Pilot runtime changed")
    sources=[PILOT/"org.omg.sysml.logic/src/main/java/org/omg/sysml"/name for name in SOURCES]
    hashes={}
    for source in sources:
        relative=source.relative_to(PILOT).as_posix();data=source.read_bytes().replace(b"\r\n",b"\n")
        require(data==subprocess.check_output(["git","-C",str(PILOT),"show",PIN+":"+relative]).replace(b"\r\n",b"\n"),"Modified pinned definition")
        hashes[relative]=sha(data)
    suffix=".exe" if os.name=="nt" else ""
    java,javac=[args.java_bin/(name+suffix) for name in ("java","javac")]
    with tempfile.TemporaryDirectory(prefix="result-construction-") as folder:
        raw=Path(folder)/"raw.json"
        subprocess.run([str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",folder,str(HELPER),str(TREE_HELPER),str(TREE_HELPER.with_name("PilotOperandProbe.java"))],check=True,timeout=60)
        subprocess.run([str(java),"-cp",folder+os.pathsep+str(jar),"dev.mercurio.pilot.PilotFeaturingQueryExporter",str(jar),str(raw),*[str(p) for p in sources]],check=True,timeout=60)
        doc=json.loads(raw.read_text())
    doc["schema"]="dev.mercurio.featuring-queries.v1"
    doc["scope"]="Featuring delegate and breadth-first traversal; complete fixture inputs and one fixed ordinary owner provider, not complete feature transformation."
    doc["toolchain"]={name:subprocess.check_output([str(exe),"-version"],stderr=subprocess.STDOUT,text=True).strip() for name,exe in [("java",java),("javac",javac)]}
    doc["provenance"]={"pilot_revision":PIN,"runtime_sha256":expected,"source_sha256":hashes,"helper_sha256":sha(HELPER.read_bytes()),"tree_helper_sha256":sha(TREE_HELPER.read_bytes()),"driver_sha256":sha(Path(__file__).read_bytes())}
    generated=render(doc)
    serialized=json.dumps(doc,indent=2,sort_keys=True)+"\n"
    if args.check: require(OUTPUT.read_text()==serialized and GENERATED.read_text()==generated,"Stale result constructor export")
    else:
        OUTPUT.write_text(serialized,encoding="utf-8")
        GENERATED.write_text(generated,encoding="utf-8")
    print(f"Observed {len(doc['cases'])} featuring query contexts")
if __name__=="__main__": main()
