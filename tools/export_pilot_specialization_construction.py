"""Import resolved result-constructor definitions; generate bounded native dispatch."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from export_pilot_feature_redefinitions import ROOT, PROFILE, PILOT, PIN, require
HELPER=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotSpecializationConstructionExporter.java"
TREE_HELPER=HELPER.with_name("PilotResultConstructionExporter.java")
OUTPUT=PROFILE/"specialization-construction.extract.json"
GENERATED=ROOT/"crates/mercurio-sysml/src/language_frontend/lowering/emit/specialization_construction_generated.rs"
SOURCES=["util/TypeUtil.java","adapter/FeatureReferenceExpressionAdapter.java","util/ExpressionUtil.java","delegate/setting/FeatureReferenceExpression_referent_SettingDelegate.java"]
def sha(data): return hashlib.sha256(data).hexdigest()

DEFINITION_SHA="c5921adf8294eaad6bfc357fb0d2786d3dd2bbca2aead91e7840a0cd79563d01"
def render(doc):
    require(sha(json.dumps(doc["definitions"],sort_keys=True,separators=(",",":")).encode())==DEFINITION_SHA,"Changed resolved specialization definition")
    kinds=doc["kinds"]
    require(len(kinds)==len(set(kinds))==8,"Changed specialization inventory")
    require(len(doc["cases"])==3*len(kinds) and {(r["kind"],r["ownership"]) for r in doc["cases"]}=={(kind,ownership) for kind in kinds for ownership in ("detached","owned","unowned_membership")},"Incomplete specialization contexts")
    result_cases=doc["result_cases"]
    require(len(result_cases)==12 and {(r["context"],r["contained"]) for r in result_cases}=={(c,o) for c in ("absent","feature","classifier","first_classifier","two_features","parameter_first") for o in (False,True)},"Incomplete result specialization contexts")
    def walk(node):
        yield node
        for child in node["children"]: yield from walk(child)
    provider="FeatureReferenceExpressionAdapter#addResultSubsetting"
    constructors=[n["symbol"] for n in walk(doc["definitions"][provider]) if n["kind"]=="METHOD_INVOCATION" and n.get("symbol", "").startswith("org.omg.sysml.lang.sysml.SysMLPackage#get")]
    require(len(constructors)==1,"Ambiguous specialization constructor")
    constructor=constructors[0].split("#get")[1].split("(")[0]
    constants="pub(super) const RESULT_PROVIDER: &str = "+json.dumps(provider.split("Adapter#")[0])+";\n"+"pub(super) const RESULT_KIND: &str = "+json.dumps(constructor)+";\n"
    require(doc["self_reference_feature"]=="Base::Anything::self","Changed self-reference library binding")
    require(len(doc["self_cases"])==3 and {r["context"] for r in doc["self_cases"]}=={"absent","feature","classifier"},"Incomplete self-reference controls")
    constants += "pub(super) const SELF_REFERENCE_FEATURE: &str = "+json.dumps(doc["self_reference_feature"])+";\n"
    return "// Generated from pinned resolved specialization insertion; do not edit.\n"+"pub(super) fn supported(kind: &str) -> bool { matches!(kind, "+" | ".join(json.dumps(k) for k in sorted(kinds))+") }\n"+constants

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
        subprocess.run([str(java),"-cp",folder+os.pathsep+str(jar),"dev.mercurio.pilot.PilotSpecializationConstructionExporter",str(jar),str(raw),*[str(p) for p in sources]],check=True,timeout=60)
        doc=json.loads(raw.read_text())
    doc["schema"]="dev.mercurio.specialization-construction.v1"
    doc["scope"]="Physical insertion of resolved pending specializations, separate from computing, reducing and ordering semantic contributions."
    doc["toolchain"]={name:subprocess.check_output([str(exe),"-version"],stderr=subprocess.STDOUT,text=True).strip() for name,exe in [("java",java),("javac",javac)]}
    doc["provenance"]={"pilot_revision":PIN,"runtime_sha256":expected,"source_sha256":hashes,"helper_sha256":sha(HELPER.read_bytes()),"tree_helper_sha256":sha(TREE_HELPER.read_bytes()),"driver_sha256":sha(Path(__file__).read_bytes())}
    generated=render(doc)
    serialized=json.dumps(doc,indent=2,sort_keys=True)+"\n"
    if args.check: require(OUTPUT.read_text()==serialized and GENERATED.read_text()==generated,"Stale result constructor export")
    else:
        OUTPUT.write_text(serialized,encoding="utf-8")
        GENERATED.write_text(generated,encoding="utf-8")
    print(f"Observed {len(doc['kinds'])} specialization kinds; {len(doc['cases'])} construction controls")
if __name__=="__main__": main()
