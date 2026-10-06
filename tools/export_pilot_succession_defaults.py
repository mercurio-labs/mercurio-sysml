"""Export the shared resolved Connector selector for two verified bindings."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

from export_pilot_feature_redefinitions import ROOT, PROFILE, PILOT, PIN, digest, require, HELPER as AST_HELPER

HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotSuccessionDefaultExporter.java"
OUTPUT = PROFILE / "succession-defaults.extract.json"
GENERATED = ROOT / "crates/mercurio-sysml/src/language_frontend/lowering/emit/succession_defaults_generated.rs"
PREFIX = "org.omg.sysml.adapter."
STRING = "java.lang.String"
CONNECTOR = "org.omg.sysml.lang.sysml.Connector"


def invocation(symbol, result, arguments=None, parameters=None, receiver=None):
    node = dict(kind="METHOD_INVOCATION", symbol=symbol, type=result, arguments=arguments or [], parameters=parameters or [])
    if receiver is not None:
        node["receiver"] = receiver
    return node


def identifier(name, type_name):
    return dict(kind="IDENTIFIER", name=name, type=type_name)


def expected_selector():
    def default(name):
        return invocation(PREFIX + "TypeAdapter#getDefaultSupertype", STRING, [dict(kind="STRING_LITERAL",type=STRING,value=name)], [STRING])
    def conditional(condition, yes, no):
        return dict(kind="CONDITIONAL_EXPRESSION",type=STRING,condition=condition,**{"true":yes,"false":no})
    ends = dict(kind="NOT_EQUAL_TO",type="boolean",left=identifier("numEnds","int"),right=dict(kind="INT_LITERAL",type="int",value=2))
    owned = invocation("org.omg.sysml.util.TypeUtil#getOwnedEndFeaturesOf", "java.util.List<org.omg.sysml.lang.sysml.Feature>", [identifier("target",CONNECTOR)], ["org.omg.sysml.lang.sysml.Type"], identifier("TypeUtil","org.omg.sysml.util.TypeUtil"))
    return dict(kind="BLOCK",statements=[
        dict(kind="VARIABLE",name="target",type=CONNECTOR,initializer=invocation(PREFIX+"ConnectorAdapter#getTarget",CONNECTOR)),
        dict(kind="VARIABLE",name="numEnds",type="int",initializer=invocation("java.util.List#size","int",receiver=owned)),
        dict(kind="RETURN",expression=conditional(invocation(PREFIX+"FeatureAdapter#hasStructureType","boolean"),conditional(ends,default("object"),default("binaryObject")),conditional(ends,default("base"),default("binary"))))])


def validate(doc, binding="Connector"):
    require(doc["schema"] == "dev.mercurio.connector-defaults.v1" and doc["binding"] == binding, "Unassessed binding")
    require(doc["methods"]["selector"] == expected_selector(), "Changed resolved Connector selector AST")
    dispatch = {name:PREFIX+"FeatureAdapter#"+name for name in ("computeImplicitGeneralTypes","addDefaultGeneralType","getSpecializationEClass","hasStructureType")}
    dispatch["getDefaultSupertype"] = PREFIX+"ConnectorAdapter#getDefaultSupertype"
    require(doc["methods"]["dispatch"] == dispatch, "Changed inherited Connector dependencies")
    require(set(doc["defaults"]) == {"base","binary","object","binaryObject"} and all(isinstance(v,str) and "::" in v for v in doc["defaults"].values()), "Missing resolved default names")
    expected = {(n,t,o) for n in range(4) for t in ("none","Class","Structure") for o in ("detached","Package","Class")}
    actual = {(r["owned_ends"],r["typing"],r["owner"]) for r in doc["controls"]}
    require(len(doc["controls"]) == 36 and actual == expected, "Changed control inventory")
    for row in doc["controls"]:
        key = ("binaryObject" if row["owned_ends"] == 2 else "object") if row["typing"] == "Structure" else ("binary" if row["owned_ends"] == 2 else "base")
        require(row["default_supertype"] == doc["defaults"][key], "Pilot observation disagrees with resolved selector")


def render(doc):
    for key,kind in [(None,"Succession"),("succession_as_usage","SuccessionAsUsage")]:
        binding=doc if key is None else doc[key]
        require(binding["methods"]["selector"]==expected_selector(),"Changed resolved succession selector")
        expected_dispatch={name:PREFIX+"FeatureAdapter#"+name for name in ("computeImplicitGeneralTypes","addDefaultGeneralType","getSpecializationEClass","hasStructureType")}
        expected_dispatch["getDefaultSupertype"]=PREFIX+"ConnectorAdapter#getDefaultSupertype"
        if kind=="SuccessionAsUsage":expected_dispatch["addDefaultGeneralType"]=PREFIX+"SuccessionAsUsageAdapter#addDefaultGeneralType"
        require(binding["methods"]["dispatch"]==expected_dispatch,"Changed succession dispatch")
        # Validate the shared selector separately from the concrete additional
        # override. Decision/merge specialization remains a native dependency.
        proxy=dict(binding,methods=dict(binding["methods"],dispatch={**expected_dispatch,"addDefaultGeneralType":PREFIX+"FeatureAdapter#addDefaultGeneralType"}))
        validate(proxy,kind)
    lines=["// Generated pinned resolved shared Connector selector and succession maps.","// Additional decision/merge/context/end algorithms are separate dependencies.","pub(super) fn default_for(kind:&str,owned_ends:usize,structure:bool)->Option<&'static str> {", "    match (kind,owned_ends==2,structure) {"]
    for key,kind in [(None,"Succession"),("succession_as_usage","SuccessionAsUsage")]:
        binding=doc if key is None else doc[key]
        for binary,structure,name in [("false","false","base"),("true","false","binary"),("false","true","object"),("true","true","binaryObject")]:lines.append(f'        ("{kind}",{binary},{structure}) => Some({json.dumps(binding["defaults"][name])}),')
    return "\n".join(lines+["        _ => None,", "    }", "}", ""])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin",type=Path,required=True)
    parser.add_argument("--check",action="store_true")
    args = parser.parse_args()
    require(subprocess.check_output(["git","-C",str(PILOT),"rev-parse","HEAD"],text=True).strip() == PIN,"Changed Pilot revision")
    runtime = PILOT / "org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    reference = json.loads((PROFILE / "xtext-prediction-nfa.experimental.json").read_text(encoding="utf-8"))
    runtime_hash = digest(runtime.read_bytes())
    require(runtime_hash == reference["provenance"]["runtime_sha256"],"Changed Pilot runtime")
    prefix = "org.omg.sysml.logic/src/main/java/org/omg/sysml/"
    source = prefix + "adapter/ConnectorAdapter.java"
    suffix = ".exe" if os.name == "nt" else ""
    java, javac = [args.java_bin / (name+suffix) for name in ("java","javac")]
    with tempfile.TemporaryDirectory(prefix="connector-defaults-") as temp:
        raw = Path(temp) / "raw.json"
        subprocess.run([str(javac),"-encoding","UTF-8","-cp",str(runtime),"-d",temp,str(AST_HELPER),str(HELPER)],check=True,timeout=60)
        subprocess.run([str(java),"-cp",temp+os.pathsep+str(runtime),"dev.mercurio.pilot.PilotSuccessionDefaultExporter",str(runtime),str(raw),str(PILOT/source)],check=True,timeout=90)
        doc = json.loads(raw.read_text(encoding="utf-8"))
    paths = {source,"org.omg.sysml/model/SysML.ecore"}
    paths.update(prefix+p for p in ["adapter/FeatureAdapter.java","adapter/TypeAdapter.java","util/ImplicitGeneralizationMap.java","util/TypeUtil.java","util/ElementUtil.java","adapter/SuccessionAdapter.java","adapter/SuccessionAsUsageAdapter.java","adapter/ConnectorAdapter.java"])
    hashes = {}
    for path in sorted(paths):
        content = (PILOT/path).read_bytes().replace(b"\r\n",b"\n")
        require(content == subprocess.check_output(["git","-C",str(PILOT),"show",PIN+":"+path]).replace(b"\r\n",b"\n"),"Changed pinned source "+path)
        hashes[path] = digest(content)
    doc["provenance"] = dict(pilot_revision=PIN,runtime_sha256=runtime_hash,source_sha256=hashes,helper_sha256=digest(HELPER.read_bytes()),ast_helper_sha256=digest(AST_HELPER.read_bytes()),driver_sha256=digest(Path(__file__).read_bytes()),toolchain={name:subprocess.check_output([str(exe),"-version"],stderr=subprocess.STDOUT,text=True).strip() for name,exe in [("java",java),("javac",javac)]})
    doc["scope"] = "Resolved shared Connector selector with separate Succession and SuccessionAsUsage dispatch/maps; override behavior is recorded as a separate semantic dependency. Controls use explicit complete input types and owned end memberships. Native owned-end collection, structure ancestry, default installation, resource lookup and transformation completeness are separate obligations."
    generated = render(doc)
    serialized = json.dumps(doc,indent=2,sort_keys=True)+"\n"
    if args.check:
        require(OUTPUT.read_text(encoding="utf-8") == serialized and GENERATED.read_text(encoding="utf-8") == generated,"Stale Connector default artifacts")
    else:
        OUTPUT.write_text(serialized,encoding="utf-8",newline="\n")
        GENERATED.write_text(generated,encoding="utf-8",newline="\n")
    print("1 shared selector; 2 succession bindings; 72 independent default-name observations; override semantics separate")


if __name__ == "__main__":
    main()
