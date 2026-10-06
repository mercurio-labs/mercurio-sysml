"""Import resolved result-constructor definitions; generate bounded native dispatch."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from export_pilot_feature_redefinitions import ROOT, PROFILE, PILOT, PIN, require
HELPER=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotResultConstructionExporter.java"
TREE_HELPER=HELPER.with_name("PilotOperandProbe.java")
OUTPUT=PROFILE/"result-construction.extract.json"
GENERATED=ROOT/"crates/mercurio-sysml/src/language_frontend/lowering/emit/result_construction_generated.rs"
SOURCES=["util/TypeUtil.java","adapter/InvocationExpressionAdapter.java","adapter/FeatureReferenceExpressionAdapter.java"]
def sha(data): return hashlib.sha256(data).hexdigest()

# Bounded, reviewed lowering: any resolved AST change requires reassessment.
# This is not a general Java translator. Ecore owns native storage and inverses.
DEFINITION_SHA = "b424a0a1e988137fdd755fb2a9c3d643e37760e1fd74b628ba9b6bb7c1f2ea78"
def construction_program(definitions):
    require(sha(json.dumps(definitions,sort_keys=True,separators=(",",":")).encode())==DEFINITION_SHA,"Changed resolved result-construction definition")
    def walk(node):
        yield node
        for child in node["children"]: yield from walk(child)
    one=list(walk(definitions["TypeUtil#addResultParameterTo/1"]))
    two=list(walk(definitions["TypeUtil#addResultParameterTo/2"]))
    def result_type(nodes, symbol):
        matches=[n["type"].rsplit(".",1)[-1] for n in nodes if n["kind"]=="METHOD_INVOCATION" and n.get("symbol")==symbol]
        require(len(matches)==1,"Ambiguous constructor operation")
        return matches[0]
    direction=[n["symbol"].split("#")[1].lower() for n in two if n["kind"]=="MEMBER_SELECT" and n.get("type")=="org.omg.sysml.lang.sysml.FeatureDirectionKind"]
    require(len(direction)==1,"Ambiguous result direction")
    return {"membership":result_type(two,"org.omg.sysml.lang.sysml.SysMLFactory#createReturnParameterMembership()"),"result":result_type(one,"org.omg.sysml.lang.sysml.SysMLFactory#createFeature()"),"direction":direction[0]}

def render(doc):
    require(sha(json.dumps(doc["definitions"],sort_keys=True,separators=(",",":")).encode())==DEFINITION_SHA,"Changed resolved result-construction definition")
    bindings=doc["bindings"]
    require(bool(bindings) and all(provider in {"InvocationExpressionAdapter#addAdditionalMembers/0","FeatureReferenceExpressionAdapter#addAdditionalMembers/0"} for provider in bindings.values()),"Unassessed result member provider")
    require(len(doc["cases"])==4*len(bindings) and {(c["class"],c["context"]) for c in doc["cases"]}=={(kind,context) for kind in bindings for context in ("empty","ordinary_member","existing_return","empty_return")},"Incomplete result construction controls")
    require(all(c["idempotent"] is True for c in doc["cases"]),"Result construction is not idempotent")
    kinds=" | ".join(json.dumps(kind) for kind in sorted(bindings))
    program=construction_program(doc["definitions"])
    require(doc["program"]==program,"Changed constructor program")
    return "// Generated from resolved pinned constructor definitions; do not edit.\n" + "pub(super) fn supported(kind: &str) -> bool { matches!(kind, "+kinds+") }\n" + "".join("pub(super) const "+key.upper()+": &str = "+json.dumps(value)+";\n" for key,value in program.items())


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
        subprocess.run([str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",folder,str(HELPER),str(TREE_HELPER)],check=True,timeout=60)
        subprocess.run([str(java),"-cp",folder+os.pathsep+str(jar),"dev.mercurio.pilot.PilotResultConstructionExporter",str(jar),str(raw),*[str(p) for p in sources]],check=True,timeout=60)
        doc=json.loads(raw.read_text())
    doc["schema"]="dev.mercurio.result-construction.v1"
    doc["scope"]="Owned result construction for exact resolved member providers; full transformation, result typing and binding remain unsupported. ConstructorExpression is excluded because its provider additionally computes typing."
    doc["toolchain"]={name:subprocess.check_output([str(exe),"-version"],stderr=subprocess.STDOUT,text=True).strip() for name,exe in [("java",java),("javac",javac)]}
    doc["provenance"]={"pilot_revision":PIN,"runtime_sha256":expected,"source_sha256":hashes,"helper_sha256":sha(HELPER.read_bytes()),"tree_helper_sha256":sha(TREE_HELPER.read_bytes()),"driver_sha256":sha(Path(__file__).read_bytes())}
    doc["program"]=construction_program(doc["definitions"])
    generated=render(doc)
    serialized=json.dumps(doc,indent=2,sort_keys=True)+"\n"
    if args.check: require(OUTPUT.read_text()==serialized and GENERATED.read_text()==generated,"Stale result constructor export")
    else:
        OUTPUT.write_text(serialized,encoding="utf-8")
        GENERATED.write_text(generated,encoding="utf-8")
    print(f"Resolved {len(doc['bindings'])} expression bindings; {len(doc['cases'])} independent result-construction controls")
if __name__=="__main__": main()
