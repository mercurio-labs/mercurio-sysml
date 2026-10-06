"""Import resolved reduction definitions and verify native category admission."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from export_pilot_feature_redefinitions import ROOT, PROFILE, PILOT, PIN, require
HELPER=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotImplicitReductionExporter.java"
TREE_HELPER=HELPER.with_name("PilotResultConstructionExporter.java")
OUTPUT=PROFILE/"implicit-reduction.extract.json"
CONSTRUCTION=PROFILE/"specialization-construction.extract.json"
GENERATED=ROOT/"crates/mercurio-sysml/src/language_frontend/lowering/emit/implicit_reduction_generated.rs"
SOURCES=["adapter/TypeAdapter.java","util/TypeUtil.java"]

def sha(data): return hashlib.sha256(data).hexdigest()

DEFINITION_SHA="1aca1dfd6fe30a9a1fe119fc03b602a64fcd0510c02011d10f2922b8ce5a6d2a"
SHAPES={"empty","explicit_same","explicit_narrower","implicit_narrower","unrelated","explicit_redefinition","explicit_subsetting","implicit_redefinition","cross_category","owner_cycle","general_cycle","explicit_self","ordered"}
SHAPES |= {kind+"_"+shape for kind in ("FeatureTyping","Subclassification","ReferenceSubsetting","CrossSubsetting","ConjugatedPortTyping") for shape in ("unrelated","explicit_same","explicit_narrower","implicit_narrower")}

def render(doc):
    require(sha(json.dumps(doc["definitions"],sort_keys=True,separators=(",",":")).encode())==DEFINITION_SHA,"Changed resolved reduction definition")
    require(len(doc["cases"])==len(SHAPES) and {r["shape"] for r in doc["cases"]}==SHAPES,"Incomplete reduction controls")
    require(set(doc["kinds"])=={"Specialization","Subsetting","Redefinition","FeatureTyping","Subclassification","ReferenceSubsetting","CrossSubsetting","ConjugatedPortTyping"},"Unassessed reduction kinds")
    require(all(type(v) is int and v>=0 for v in doc["kinds"].values()) and len(set(doc["kinds"].values()))==8,"Invalid reduction kind ordering")
    require(set(doc["kinds"])==set(json.loads(CONSTRUCTION.read_text(encoding="utf-8"))["kinds"]),"Reduction/construction category inventory disagreement")
    require(all(r["remaining"]==r["after_closed"] for r in doc["cases"]),"Reduction queue remains open")
    return "// Generated pinned reduction kind order; semantic traversal is handwritten.\n"+"pub(super) fn rank(kind: &str) -> Option<usize> { match kind {\n"+"".join("    "+json.dumps(k)+" => Some("+str(v)+"),\n" for k,v in sorted(doc["kinds"].items()))+"    _ => None,\n} }\n"

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
        subprocess.run([str(java),"-cp",folder+os.pathsep+str(jar),"dev.mercurio.pilot.PilotImplicitReductionExporter",str(jar),str(raw),*[str(p) for p in sources]],check=True,timeout=60)
        doc=json.loads(raw.read_text())
    doc["schema"]="dev.mercurio.implicit-reduction.v1"
    doc["scope"]="Redundancy reduction over a precomputed pending snapshot; controlled complete ancestry, not complete transformation."
    doc["toolchain"]={name:subprocess.check_output([str(exe),"-version"],stderr=subprocess.STDOUT,text=True).strip() for name,exe in [("java",java),("javac",javac)]}
    doc["provenance"]={"construction_sha256":sha(CONSTRUCTION.read_bytes()),"pilot_revision":PIN,"runtime_sha256":expected,"source_sha256":hashes,"helper_sha256":sha(HELPER.read_bytes()),"tree_helper_sha256":sha(TREE_HELPER.read_bytes()),"driver_sha256":sha(Path(__file__).read_bytes())}
    generated=render(doc)
    serialized=json.dumps(doc,indent=2,sort_keys=True)+"\n"
    if args.check: require(OUTPUT.read_text()==serialized and GENERATED.read_text()==generated,"Stale reduction export")
    else:
        OUTPUT.write_text(serialized,encoding="utf-8")
        GENERATED.write_text(generated,encoding="utf-8")
    print(f"Observed {len(doc['cases'])} reduction contexts")
if __name__=="__main__": main()
