"""Import resolved compatibility definitions; verify bounded native dependency admission."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import tempfile
from export_pilot_feature_redefinitions import ROOT, PROFILE, PILOT, PIN, require
HELPER=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotCompatibilityExporter.java"
TREE_HELPER=HELPER.with_name("PilotResultConstructionExporter.java")
OUTPUT=PROFILE/"compatibility.extract.json"
GENERATED=ROOT/"crates/mercurio-sysml/src/language_frontend/lowering/emit/compatibility_generated.rs"
SOURCE_ONLY=["adapter/TypeAdapter.java","util/ElementUtil.java","adapter/ElementAdapter.java","adapter/NamespaceAdapter.java","adapter/FeatureAdapter.java","adapter/ConnectorAdapter.java"]
SOURCES=SOURCE_ONLY+["adapter/InvariantAdapter.java","adapter/ExpressionAdapter.java","util/ConnectorUtil.java","util/TypeUtil.java","util/FeatureUtil.java","delegate/invocation/Feature_isFeaturedWithin_InvocationDelegate.java"]

def sha(data): return hashlib.sha256(data).hexdigest()

def normalize_capture_ids(definitions):
    """Alpha-rename javac's process-specific capture IDs, preserving identity."""
    captures = {}
    def visit(value, key=None):
        if isinstance(value, dict):
            return {k: visit(value[k], k) for k in sorted(value)}
        if isinstance(value, list):
            return [visit(item) for item in value]
        if key == "type" and isinstance(value, str):
            def capture(match):
                token = match.group(0)
                if token not in captures: captures[token] = "capture#" + str(len(captures))
                return captures[token]
            return re.sub(r"capture#\d+", capture, value)
        return value
    return visit(definitions)

DEFINITION_SHA="9d78704d2f127466077661a35bed9eed242633f489177b74642701cb5ac23113"
SHAPES={"unrelated","same_context","narrower_context","wider_context","different_context","unfeatured_super","owned_feature","direct_specialization","variable_owner","first_variable","nested_positive","nested_negative","nested_cycle","nested_diamond","global_positive","global_negative","global_empty","nested_global"}
EXPRESSION_CONTRIBUTIONS_SHA="6402e32b94fcc4477fbccf365814ecfc7243df06237c512266bd7eba867b9d4f"
def expression_default_bindings(doc, expression_kinds):
    definitions=doc.get("expression_default_definitions",{})
    require(set(definitions)=={"getDefaultSupertype","addDefaultGeneralType","invariant_selector"},"Incomplete resolved expression defaults")
    selector=definitions["getDefaultSupertype"]
    try:
        ret=selector["children"][0];call=ret["children"][0];key=call["children"][1]
        require(selector["kind"]=="BLOCK" and len(selector["children"])==1 and ret["kind"]=="RETURN" and len(ret["children"])==1
                and call["kind"]=="METHOD_INVOCATION" and call["symbol"]=="org.omg.sysml.adapter.TypeAdapter#getDefaultSupertype(java.lang.String)"
                and call["type"]=="java.lang.String" and len(call["children"])==2
                and key=={"kind":"STRING_LITERAL","type":"java.lang.String","value":"base","children":[]},"Changed resolved expression selector")
    except (KeyError,IndexError,TypeError) as exc: raise ValueError("Changed resolved expression selector") from exc
    require(sha(json.dumps(definitions["addDefaultGeneralType"],sort_keys=True,separators=(",",":")).encode())==EXPRESSION_CONTRIBUTIONS_SHA,"Changed resolved expression contributions")
    require(sha(json.dumps(definitions['getDefaultSupertype'],sort_keys=True,separators=(",",":")).encode())=="ce9bc480314eb859ab5edbd1bba755c4f1a1397adcde7d608620f55a3fe68feb","Changed resolved expression selector tree")
    require(sha(json.dumps(definitions['invariant_selector'],sort_keys=True,separators=(",",":")).encode())=="54e95ec9af4b808b95ee6f3ae0376f705c6f5c6d646a4450ee58cf9ecfdbe72c","Changed resolved expression selector tree")
    data=doc.get("expression_defaults",{});bindings=data.get("bindings",{});excluded=data.get("excluded",{});cases=data.get("cases",[])
    require(set(bindings).isdisjoint(excluded) and set(bindings)|set(excluded)==expression_kinds,"Incomplete expression default dispatch")
    negated_bindings=data.get("negated_bindings",{});selectors=data.get("selectors",{})
    require(set(selectors)==set(bindings) and all(selectors[k]==("org.omg.sysml.adapter.InvariantAdapter" if k in negated_bindings else "org.omg.sysml.adapter.ExpressionAdapter") for k in bindings),"Unassessed expression selector dispatch")
    expected={(kind,context,composite,negated) for kind in bindings for context in ("detached","value","package") for composite in (False,True) for negated in ((False,True) if kind in negated_bindings else (False,))}
    require(len(cases)==len(expected) and {(r["kind"],r["context"],r["composite"],r["negated"]) for r in cases}==expected,"Incomplete expression default controls")
    require(all(r["generals"]==[{"kind":"Subsetting","target":(negated_bindings if r["negated"] else bindings)[r["kind"]]}] for r in cases),"Unassessed expression default contribution")
    result_case=data.get("library_result_case",{})
    require(isinstance(result_case.get("source"),str) and result_case.get("targets")==["libraryResult"] and result_case.get("result_generals")==[{"kind":"Redefinition","target":"libraryResult"}],"Unassessed expression library result")
    require(result_case.get("reference_binding")==[{"membership":"OwningMembership","featuring":["A"],"ends":2,"related":["y","$result"]}],"Unassessed reference binding placement")
    require(result_case.get("binding_defaults",[{}])[0].get("end_redefined")==[[],[]],"Missing reference binding semantic evidence")
    inherited=data.get("inherited_binding_case",{})
    require(inherited.get("binding_defaults",[{}])[0].get("end_redefined")==[["left"],["right"]],"Missing inherited binding semantic evidence")
    result_defaults=data.get("result_defaults",[])
    require(len(result_defaults)==24 and {(r["owner_kind"],r["mask"]) for r in result_defaults}=={(k,m) for k in ("Expression","FeatureReferenceExpression","Function") for m in range(8)},"Incomplete result default controls")
    require(all(len(r["generals"])==1 and r["generals"][0]["kind"]=="Subsetting" for r in result_defaults),"Unassessed result default contributions")
    binding_defaults=data.get("binding_defaults",[])
    require(len(binding_defaults)==len(expression_kinds) and {r["owner_kind"] for r in binding_defaults}==expression_kinds,"Incomplete expression-owned binding controls")
    require(all(r["generals"]==[{"kind":"Subsetting","target":"Links::selfLinks"}] for r in binding_defaults),"Unassessed expression-owned binding defaults")
    return bindings

def render(doc):
    require(sha(json.dumps(doc["definitions"],sort_keys=True,separators=(",",":")).encode())==DEFINITION_SHA,"Changed resolved compatibility definition")
    require(len(doc["cases"])==len(SHAPES) and {r["shape"] for r in doc["cases"]}==SHAPES,"Incomplete compatibility contexts")
    require(all(type(r["compatible"]) is bool and type(r["can_access"]) is bool for r in doc["cases"]),"Invalid compatibility observation")
    require(len(doc["context_cases"])==10 and {r["shape"] for r in doc["context_cases"]}=={"empty","same","narrower","wider","unrelated","source_context","target_context","multiple","duplicate","nested"},"Incomplete connector contexts")
    require(all(len(r.get("placements",[]))==2 and {p["owner"] for p in r["placements"]}=={"a","c"} for r in doc["context_cases"]),"Incomplete binding placements")
    require(all(p["membership"] in {"FeatureMembership","OwningMembership"} and isinstance(p["featuring"],list) for r in doc["context_cases"] for p in r["placements"]),"Invalid binding placement")
    require(len(doc["binding_stages"])==4 and {(r["owner"],r["target"]) for r in doc["binding_stages"]}=={(o,t) for o in ("A","B") for t in ("x","y")},"Incomplete transformed binding controls")
    require(all(len(r["ends"])==2 and r["connector"]["kind"]=="BindingConnector" for r in doc["binding_stages"]),"Invalid transformed binding shape")
    value_shapes={"none","ordinary","default","initial","missing_value","missing_result","first_default","first_empty","explicit_general","implied_general","directional"}
    require(len(doc.get("value_cases",[]))==len(value_shapes) and {c["shape"] for c in doc["value_cases"]}==value_shapes,
            "Incomplete bound-value controls")
    require(all(c["contributions"] in ([],[{"kind":"Subsetting","chain":["expression","result"]}]) for c in doc["value_cases"]),
            "Invalid bound-value contribution")
    classes=json.loads((PROFILE/"ecore-effective.extract.json").read_text())["classes"]
    by_name={c["name"]:c for c in classes}
    def expression_kind(name):
        return name=="Expression" or any(expression_kind(parent.rsplit("/",1)[-1]) for parent in by_name[name]["super_types"])
    expression_kinds={c["name"] for c in classes if not c["abstract"] and not c["interface"] and expression_kind(c["name"])}
    featuring=doc.get("expression_featuring_cases",[])
    require(all(isinstance(r.get("query"),list) and all(isinstance(t,str) for t in r["query"]) for r in featuring),"Missing expression featuring query observations")
    expected_featuring={(kind,shape) for kind in expression_kinds for shape in ("empty","one","two","duplicate","existing")}
    require(len(featuring)==len(expected_featuring) and {(r["kind"],r["shape"]) for r in featuring}==expected_featuring,"Incomplete expression featuring controls")
    require(all(r["implementation"] in ("org.omg.sysml.adapter.ExpressionAdapter","org.omg.sysml.adapter.FeatureAdapter") and isinstance(r["pending"],list)
                and all(x["target"] in ("a","b") and type(x["implied"]) is bool and type(x["adopted"]) is bool for x in r["stored"]) for r in featuring),"Unassessed expression featuring dispatch: "+repr(sorted({(r["kind"],r["implementation"]) for r in featuring if r["implementation"]!="org.omg.sysml.adapter.ExpressionAdapter"})))
    concrete={c["name"] for c in classes if not c["abstract"] and not c["interface"]}
    expected={(kind, complete, shape) for kind in concrete for complete in (False,True)
              for shape in ("empty","explicit","implied","mixed","nested")}
    observed=doc.get("inclusion_cases",[])
    require(len(observed)==len(expected) and {(c["kind"],c["complete"],c["shape"]) for c in observed}==expected,
            "Incomplete implied inclusion controls")
    require(all(c["issues"] in ([],["validateElementIsImpliedIncluded"]) for c in observed),
            "Invalid implied inclusion observation")
    for case in doc["binding_stages"]:
        document=case.get("stored_document",[])
        require(bool(document) and document[0]["path"]=="$" and document[0]["kind"]=="Namespace",
                "Incomplete transformed document")
        require(len({r["path"] for r in document})==len(document),"Invalid transformed document")
        rows=case.get("stored_subtree",[])
        require(bool(rows),"Incomplete stored binding subtree")
        paths=[row["path"] for row in rows]
        require(len(set(paths))==len(paths) and paths[0]=="$" and
                all(isinstance(row.get("attributes"),dict) and isinstance(row.get("references"),dict) for row in rows),
                "Invalid stored binding subtree")
    expression_bindings=expression_default_bindings(doc,expression_kinds)
    default_program="pub(super) fn expression_default_name(kind: &str, negated: bool) -> Option<&'static str> { match kind {\n" + "".join(json.dumps(k)+" => Some("+("if negated { "+json.dumps(doc["expression_defaults"]["negated_bindings"][k])+" } else { "+json.dumps(v)+" }" if k in doc["expression_defaults"]["negated_bindings"] else json.dumps(v))+"),\n" for k,v in sorted(expression_bindings.items())) + "_ => None, } }\n"
    admission="pub(super) fn expression_featuring_supported(kind: &str) -> bool { matches!(kind, " + " | ".join(json.dumps(k) for k in sorted(expression_kinds)) + ") }\n"
    return "// Generated admission guard for pinned resolved compatibility semantics.\n"+'pub(super) const DEFINITION_SHA: &str = "'+DEFINITION_SHA+'";\n'+admission+default_program


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
        subprocess.run([str(java),"-cp",folder+os.pathsep+str(jar),"dev.mercurio.pilot.PilotCompatibilityExporter",str(jar),str(raw),*[str(p) for p in sources if p.name not in {Path(name).name for name in SOURCE_ONLY}]],check=True,timeout=60)
        doc=json.loads(raw.read_text())
        doc["definitions"]=normalize_capture_ids(doc["definitions"])
    doc["source_only_inputs"]=SOURCE_ONLY  # Inspected source fingerprint; existing resolved tree unit stays stable.
    doc["schema"]="dev.mercurio.compatibility.v1"
    doc["scope"]="Type compatibility including shared redefinitions and featured-within dependencies; complete inputs, not complete implicit transformation."
    doc["toolchain"]={name:subprocess.check_output([str(exe),"-version"],stderr=subprocess.STDOUT,text=True).strip() for name,exe in [("java",java),("javac",javac)]}
    doc["provenance"]={"pilot_revision":PIN,"runtime_sha256":expected,"source_sha256":hashes,"helper_sha256":sha(HELPER.read_bytes()),"tree_helper_sha256":sha(TREE_HELPER.read_bytes()),"driver_sha256":sha(Path(__file__).read_bytes())}
    validator=PILOT/"org.omg.kerml.xtext/src/org/omg/kerml/xtext/validation/KerMLValidator.xtend"
    relative=validator.relative_to(PILOT).as_posix()
    data=validator.read_bytes().replace(b"\r\n",b"\n")
    require(data==subprocess.check_output(["git","-C",str(PILOT),"show",PIN+":"+relative]).replace(b"\r\n",b"\n"),"Modified pinned validator")
    doc["provenance"]["source_sha256"][relative]=sha(data)
    doc["provenance"]["effective_ecore_sha256"]=sha((PROFILE/"ecore-effective.extract.json").read_bytes())
    generated=render(doc)
    serialized=json.dumps(doc,indent=2,sort_keys=True)+"\n"
    if args.check: require(OUTPUT.read_text()==serialized and GENERATED.read_text()==generated,"Stale compatibility export")
    else:
        OUTPUT.write_text(serialized,encoding="utf-8")
        GENERATED.write_text(generated,encoding="utf-8")
    print(f"Observed {len(doc['cases'])} compatibility contexts")
if __name__=="__main__": main()
