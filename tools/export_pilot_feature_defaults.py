"""Export one resolved Feature selector; dependent native algorithms are separate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

from export_pilot_feature_redefinitions import ROOT, PROFILE, PILOT, PIN, digest, require, HELPER as AST_HELPER

HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotFeatureDefaultExporter.java"
DOCUMENT_HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotDefinitionDocumentProbe.java"
OUTPUT = PROFILE / "feature-defaults.extract.json"
GENERATED = ROOT / "crates/mercurio-sysml/src/language_frontend/lowering/emit/feature_defaults_generated.rs"
PREFIX = "org.omg.sysml.adapter."
STRING = "java.lang.String"


def invocation(symbol, result, arguments=None, parameters=None, receiver=None):
    node = dict(kind="METHOD_INVOCATION", symbol=symbol, type=result, arguments=arguments or [], parameters=parameters or [])
    if receiver is not None:
        node["receiver"] = receiver
    return node


def identifier(name, type_name):
    return dict(kind="IDENTIFIER", name=name, type=type_name)


def expected_selector():
    def literal(value): return dict(kind="STRING_LITERAL",type=STRING,value=value)
    def choose(method,yes,no):
        return dict(kind="CONDITIONAL_EXPRESSION",type=STRING,condition=invocation(PREFIX+"FeatureAdapter#"+method,"boolean"),**{"true":yes,"false":no})
    tree=choose("hasStructureType",choose("isSubobject",literal("subobject"),literal("object")),choose("hasClassType",choose("isSuboccurrence",literal("suboccurrence"),choose("isPortion",literal("portion"),literal("occurrence"))),choose("hasDataType",literal("dataValue"),literal("base"))))
    return dict(kind="BLOCK",statements=[dict(kind="RETURN",expression=invocation(PREFIX+"TypeAdapter#getDefaultSupertype",STRING,[tree],[STRING]))])


def expected_participant():
    package="org.omg.sysml.lang.sysml.SysMLPackage"
    eclass="org.eclipse.emf.ecore.EClass"
    receiver=dict(kind="MEMBER_SELECT",name="eINSTANCE",type=package,receiver=identifier("SysMLPackage",package))
    redefinition=invocation(package+"#getRedefinition",eclass,receiver=receiver)
    declared=invocation(PREFIX+"TypeAdapter#isImplicitSpecializationDeclaredFor","boolean",[redefinition],[eclass])
    condition=dict(kind="PARENTHESIZED",type="boolean",expression=dict(kind="CONDITIONAL_AND",type="boolean",left=invocation(PREFIX+"FeatureAdapter#isAssociationEnd","boolean"),right=dict(kind="LOGICAL_COMPLEMENT",type="boolean",expression=declared)))
    effect=invocation(PREFIX+"TypeAdapter#addDefaultGeneralType","void",[dict(kind="STRING_LITERAL",type=STRING,value="participant")],[STRING])
    return dict(condition=condition,effect=effect)


def expected_association_selector():
    target=invocation(PREFIX+"AssociationAdapter#getTarget","org.omg.sysml.lang.sysml.Association")
    ends=invocation("org.omg.sysml.lang.sysml.Type#getOwnedEndFeature","org.eclipse.emf.common.util.EList<org.omg.sysml.lang.sysml.Feature>",receiver=target)
    size=invocation("java.util.List#size","int",receiver=ends)
    condition=dict(kind="NOT_EQUAL_TO",type="boolean",left=size,right=dict(kind="INT_LITERAL",type="int",value=2))
    def name(key): return invocation(PREFIX+"TypeAdapter#getDefaultSupertype",STRING,[dict(kind="STRING_LITERAL",type=STRING,value=key)],[STRING])
    return dict(kind="BLOCK",statements=[dict(kind="RETURN",expression=dict(kind="CONDITIONAL_EXPRESSION",type=STRING,condition=condition,**{"true":name("base"),"false":name("binary")}))])


def selected_key(mask,owner,composite,portion,owner_mask=0):
    owner_structure = owner == "Structure" or (owner == "Feature" and bool(owner_mask & 2))
    owner_class = owner in ("Class","Structure") or (owner == "Feature" and bool(owner_mask & 3))
    if mask & 2: return "subobject" if composite and owner_structure else "object"
    if mask & 1:
        if composite and owner_class: return "suboccurrence"
        if portion and owner_class: return "portion"
        return "occurrence"
    return "dataValue" if mask & 4 else "base"


def validate_typing(doc):
    methods={"typing_usage_definition":"3d5f90481d9dc39f3b729879246a17ee065705065e55b8ec157e283283f397bd","typing_dynamicInvoke":"12aea97d29a8e145d280f769726ff67bb82bdeddb7e603e6015d37465718a677","typing_getAllTypes":"062dd7b898ca44691d0d72d071d958e862b4e564d8ca47ea8be148bcc637ea39","typing_getFeatureTypes":"c6fd6f39284a3b2e2e91306b4f407611a78a5591107ff403af68154b49f1c679","typing_getTypes":"3c2069a3812583d4ee89aabc1f6b8b163611027801b80fa9c73309cdb99d7cb9","typing_removeRedundantTypes":"648997ba4cf82527f480e17c753968bf4e0f223c7662ed32db0d31a1326abfec"}
    for name,expected in methods.items():
        require(hashlib.sha256(json.dumps(doc["methods"][name],sort_keys=True,separators=(",",":")).encode()).hexdigest()==expected,"Changed resolved Feature typing algorithm: "+name)
    require(set(doc["typing_bindings"])=={"Feature","Usage","ReferenceUsage"},"Changed typing binding inventory")
    for binding in doc["typing_bindings"].values():
        require(binding=={name:PREFIX+"FeatureAdapter#"+name for name in ("getAllTypes","getTypes","getFeatureTypes")},"Changed Feature typing dispatch")
    edges={"direct":[["candidate","leaf"]],"chain":[["candidate","left"],["left","leaf"]],"diamond":[["candidate","left"],["candidate","right"],["left","leaf"],["right","leaf"]],"cycle":[["candidate","leaf"],["leaf","candidate"]]}
    rows=doc["inherited_typing_controls"]
    expected={(kind,mask,relation,shape) for kind in doc["typing_bindings"] for mask in range(16) for relation in ("Subsetting","Redefinition","ReferenceSubsetting") for shape in edges}
    require(len(rows)==576 and {(r["kind"],r["typing_mask"],r["relationship"],r["shape"]) for r in rows}==expected,"Changed inherited typing inventory")
    for row in rows:
        require(row["edges"]==edges[row["shape"]],"Changed inherited typing graph")
        require(row["types"]==["type"+str(bit) for bit in range(4) if row["typing_mask"] & (1<<bit)],"Inherited typing observation disagrees")
        require(row["projected_types"]==["type"+str(bit) for bit in range(4 if row["kind"]=="Feature" else 3) if row["typing_mask"] & (1<<bit)],"Classifier projection observation disagrees")
        require(row["default_supertype"]=="Base::things","Inherited typing must not change the owned-typing default selector")

def validate(doc):
    validate_typing(doc)
    validate_usage(doc)
    require(len(doc["annotation_default_controls"]) == 96 and len({c["source"] for c in doc["annotation_default_controls"]}) == 96, "Changed annotation default control inventory")
    require(doc["schema"] == "dev.mercurio.feature-defaults.v1" and doc["binding"] == "Feature", "Unassessed binding")
    require(doc["methods"]["selector"] == expected_selector(), "Changed resolved Feature selector AST")
    require(doc["methods"]["participant"] == expected_participant(), "Changed resolved participant suppression AST")
    require(doc["methods"]["association_selector"] == expected_association_selector(), "Changed resolved Association selector AST")
    require(set(doc["association_defaults"]) == {"Association","AssociationStructure"} and all(set(names) == {"base","binary"} and all(isinstance(name,str) and "::" in name for name in names.values()) for names in doc["association_defaults"].values()), "Changed Association default bindings")
    for kind in ("Association","AssociationStructure"):
        expected={"getDefaultSupertype":PREFIX+"AssociationAdapter#getDefaultSupertype","getSpecializationEClass":PREFIX+"ClassifierAdapter#getSpecializationEClass",**{name:PREFIX+"TypeAdapter#"+name for name in ("computeImplicitGeneralTypes","addDefaultGeneralType")}}
        require(doc["association_dispatch"][kind] == expected,"Changed Association inherited dependencies")
    expected={(kind,n,owned) for kind in ("Association","AssociationStructure") for n in range(4) for owned in (False,True)}
    require(len(doc["association_controls"]) == 16 and {(r["kind"],r["owned_ends"],r["package_owned"]) for r in doc["association_controls"]} == expected,"Changed Association inventory")
    for row in doc["association_controls"]:
        require(row["default_supertype"] == doc["association_defaults"][row["kind"]]["binary" if row["owned_ends"] == 2 else "base"],"Association observation disagrees")
    require(len(doc["association_source_controls"]) == 4 and all(c["accepted"] for c in doc["association_source_controls"]),"Changed Association source inventory")
    dispatch = {name:PREFIX+"FeatureAdapter#"+name for name in ("computeImplicitGeneralTypes","addDefaultGeneralType","getSpecializationEClass","hasStructureType")}
    dispatch["getDefaultSupertype"] = PREFIX+"FeatureAdapter#getDefaultSupertype"
    require(doc["methods"]["dispatch"] == dispatch, "Changed inherited Feature dependencies")
    require(set(doc["defaults"]) == {"base","object","subobject","occurrence","suboccurrence","portion","dataValue"} and all(isinstance(v,str) and "::" in v for v in doc["defaults"].values()), "Missing resolved default names")
    expected = {(m,o,c,p,om) for m in range(8) for o in ("detached","Package","Class","Structure","Feature") for om in range(8 if o == "Feature" else 1) for c in (False,True) for p in (False,True)}
    actual = {(r["typing_mask"],r["owner"],r["composite"],r["portion"],r["owner_typing_mask"]) for r in doc["controls"]}
    require(len(doc["controls"]) == 384 and actual == expected, "Changed control inventory")
    for row in doc["controls"]:
        key = selected_key(row["typing_mask"],row["owner"],row["composite"],row["portion"],row["owner_typing_mask"])
        require(row["default_supertype"] == doc["defaults"][key], "Pilot observation disagrees with resolved selector")

    require(len(doc["source_controls"]) == 2 and all(r["accepted"] and r["nodes"] and r["links"] for r in doc["source_controls"]), "Changed source control inventory")
    require(len(doc["end_general_controls"]) == 2 and all(r["accepted"] and len(r["general_queries"]) == 2 for r in doc["end_general_controls"]), "Changed end-general control inventory")
    require(len(doc["binding_connector_controls"]) == 4 and all(r["accepted"] and len(r["connector_projections"]) == 1 and len(r["general_queries"]) == 2 for r in doc["binding_connector_controls"]), "Changed BindingConnector source inventory")
    require(len(doc["connector_participant_controls"]) == 5 and all(r["accepted"] and len(r["connector_projections"]) == 1 for r in doc["connector_participant_controls"]) and sum(len(r["general_queries"]) for r in doc["connector_participant_controls"]) == 6, "Changed Connector participant inventory")
    require(len(doc["absent_reference_controls"]) == 5 and all(r["accepted"] and len(r["connector_projections"]) == 1 for r in doc["absent_reference_controls"]), "Changed absent-reference inventory")
    require(len(doc["feature_type_controls"]) == 8 and all(c["accepted"] and isinstance(c["types"],list) for c in doc["feature_type_controls"]),"Changed Feature type inventory")
    require(len(doc["association_type_controls"]) == 8 and all(c["accepted"] and set(c["association_types"]) == {"source_type","target_type","related_type"} for c in doc["association_type_controls"]),"Changed Association type inventory")

def validate_usage(doc):
    # Reviewed complete javac-resolved override: Transition payload and Succession
    # end effects precede the inherited Usage call. Ordinary namespace ownership
    # excludes both branches; signatures alone do not establish this fallback.
    tree=doc["methods"]["reference_default_contributions"]
    require(hashlib.sha256(json.dumps(tree,sort_keys=True,separators=(",",":")).encode()).hexdigest() == "75dc02001691a90a81175b579bd204946985359374e3320a89ffba75309cfe16", "Changed resolved ReferenceUsage default override")
    expected = {name:PREFIX+"FeatureAdapter#"+name for name in ("getDefaultSupertype","computeImplicitGeneralTypes","getSpecializationEClass","hasStructureType")}
    expected.update({name:PREFIX+"UsageAdapter#"+name for name in ("addDefaultGeneralType","addAdditionalMembers","isAddMultiplicity")})
    require(set(doc["usage_bindings"]) == {"Usage","BindingConnectorAsUsage","ReferenceUsage"},"Changed Usage binding inventory")
    for kind,binding in doc["usage_bindings"].items():
        dispatch=dict(expected)
        if kind == "ReferenceUsage": dispatch["addDefaultGeneralType"]=PREFIX+"ReferenceUsageAdapter#addDefaultGeneralType"
        require(binding["dispatch"] == dispatch,"Changed Usage dispatch")
        require(set(binding["defaults"]) == set(doc["defaults"]) and all(isinstance(v,str) and "::" in v for v in binding["defaults"].values()),"Changed Usage default bindings")
    shapes={"direct","short","multiple","inherited","diamond","private","protected","explicit","duplicate","chain_equal","chain_distinct"}
    ends=doc["reference_end_controls"]
    require(len(ends)==33 and {(r["shape"],r["position"]) for r in ends}=={(shape,pos) for shape in shapes for pos in range(3)} and all(r["kind"]=="ReferenceUsage" for r in ends),"Changed ReferenceUsage end inventory")
    parameter_shapes=(shapes-{"duplicate"})|{"result_first","no_result"}
    parameters=doc["reference_parameter_controls"]
    require(len(parameters)==48 and {(r["shape"],r["position"]) for r in parameters}=={(shape,pos) for shape in parameter_shapes for pos in range(4)} and all(r["kind"]=="ReferenceUsage" for r in parameters),"Changed ReferenceUsage parameter inventory")
    reference="org.omg.sysml.lang.sysml.ReferenceUsage"
    type_name="org.omg.sysml.lang.sysml.Type"
    element="org.omg.sysml.lang.sysml.Element"
    fallback=dict(locals=[dict(kind="VARIABLE",name="target",type=reference,initializer=invocation(PREFIX+"ReferenceUsageAdapter#getTarget",reference)),dict(kind="VARIABLE",name="type",type=type_name,initializer=invocation("org.omg.sysml.lang.sysml.Feature#getOwningType",type_name,receiver=identifier("target",reference)))],guard_operand=identifier("type",type_name),guard_type="org.omg.sysml.lang.sysml.TransitionUsage",else_effect=invocation(PREFIX+"FeatureAdapter#addRedefinitions","void",[identifier("skip",element)],[element],identifier("super",PREFIX+"UsageAdapter")))
    transition="org.omg.sysml.lang.sysml.TransitionUsage"
    cast=dict(kind="TYPE_CAST",type=transition,expression=identifier("type",type_name))
    query=invocation("org.omg.sysml.util.UsageUtil#getTransitionLinkFeatureOf","org.omg.sysml.lang.sysml.Feature",[cast],[transition],identifier("UsageUtil","org.omg.sysml.util.UsageUtil"))
    fallback["link_identity"]=dict(kind="EQUAL_TO",type="boolean",left=identifier("target",reference),right=query)
    rows=doc["reference_transition_controls"]
    require(len(rows)==6 and {(r["membership"],r["preceding"]) for r in rows}=={(m,p) for m in ("FeatureMembership","ParameterMembership","TransitionFeatureMembership") for p in (False,True)},"Changed transition link inventory")
    require(all(r["is_transition_link"]==(r["membership"]=="FeatureMembership" and not r["preceding"]) for r in rows),"Transition link observation disagrees")
    require(doc["methods"]["reference_redefinition_fallback"]==fallback,"Changed ReferenceUsage fallback")
    reference_dispatch={name:PREFIX+("ReferenceUsageAdapter#" if name=="addRedefinitions" else "FeatureAdapter#")+name for name in ("addRedefinitions","addFeatureWriteTypes","addComputedRedefinitions","isComputeRedefinitions","getRedefinedFeaturesWithComputed","getRelevantFeatures","getGeneralTypes","getEndRelevantFeatures","getParameterRelevantFeatures","getRelevantParameters","filterIgnoredParameters")}
    require(doc["reference_dispatch"]==reference_dispatch,"Changed ReferenceUsage dispatch")
    target=invocation(PREFIX+"UsageAdapter#getTarget","org.omg.sysml.lang.sysml.Usage")
    require(doc["methods"]["usage_contributions"] == [invocation(PREFIX+"UsageAdapter#addVariationTyping","void"),invocation(PREFIX+"FeatureAdapter#addDefaultGeneralType","void",receiver=identifier("super",PREFIX+"FeatureAdapter"))],"Changed Usage contributions")
    require(doc["methods"]["usage_multiplicity"] == dict(kind="BLOCK",statements=[dict(kind="RETURN",expression=invocation("org.omg.sysml.lang.sysml.Feature#isEnd","boolean",receiver=target))]),"Changed Usage multiplicity")
    require(doc["methods"]["usage_added_members"] == dict(condition=dict(kind="PARENTHESIZED",type="boolean",expression=invocation(PREFIX+"UsageAdapter#isAddMultiplicity","boolean")),effect=invocation("org.omg.sysml.util.TypeUtil#addMultiplicityTo","void",[target],["org.omg.sysml.lang.sysml.Type"],identifier("TypeUtil","org.omg.sysml.util.TypeUtil"))),"Changed Usage added members")
    expected={(kind,mask,owned,composite,portion) for kind in doc["usage_bindings"] for mask in range(8) for owned in (False,True) for composite in (False,True) for portion in (False,True)}
    require(len(doc["usage_source_controls"])==4 and all(c["accepted"] for c in doc["usage_source_controls"]),"Changed Usage source inventory")
    rows=doc["usage_controls"]
    require(len(rows)==192 and {(r["kind"],r["typing_mask"],r["package_owned"],r["composite"],r["portion"]) for r in rows}==expected,"Changed Usage control inventory")
    for row in rows:
        key=selected_key(row["typing_mask"],"Package" if row["package_owned"] else "detached",row["composite"],row["portion"])
        require(row["default_supertype"]==doc["usage_bindings"][row["kind"]]["defaults"][key],"Usage observation disagrees")

def render_usage(doc):
    lines=["pub(super) fn ordinary_feature_type_binding(kind: &str) -> bool {", "    matches!(kind, "+" | ".join(json.dumps(k) for k in sorted(doc["typing_bindings"]))+")", "}"]
    lines+=["pub(super) fn usage_default_name(kind: &str, structure: bool, class: bool, data: bool) -> Option<&'static str> {", "    match kind {"]
    for kind,binding in doc["usage_bindings"].items():
        n={k:json.dumps(v) for k,v in binding["defaults"].items()}
        lines.append(f"        {json.dumps(kind)} => Some(if structure {{ {n['object']} }} else if class {{ {n['occurrence']} }} else if data {{ {n['dataValue']} }} else {{ {n['base']} }}),")
    return lines+["        _ => None,", "    }", "}", "pub(super) fn ordinary_usage_binding(kind: &str) -> bool { usage_default_name(kind, false, false, false).is_some() }", "pub(super) fn usage_additional_members_required(is_end: bool) -> bool { is_end }", "pub(super) fn reference_redefinition_uses_feature(owner_is_transition: bool, is_transition_link: bool) -> bool { !(owner_is_transition && is_transition_link) }"]


def render(doc):
    validate(doc)
    n={k:json.dumps(v) for k,v in doc["defaults"].items()}
    return "\n".join([
        "// Generated by tools/export_pilot_feature_defaults.py; do not edit.",
        "// Exact Feature selector; guarded native services provide semantic predicate inputs.",
        "pub(super) fn feature_default_name(structure: bool, class: bool, data: bool, subobject: bool, suboccurrence: bool, portion: bool) -> &'static str {",
        f"    if structure {{ if subobject {{ {n['subobject']} }} else {{ {n['object']} }} }}",
        f"    else if class {{ if suboccurrence {{ {n['suboccurrence']} }} else if portion {{ {n['portion']} }} else {{ {n['occurrence']} }} }}",
        f"    else if data {{ {n['dataValue']} }} else {{ {n['base']} }}",
        "}",
        "pub(super) fn participant_required(association_end: bool, implicit_redefinition: bool) -> bool {",
        "    association_end && !implicit_redefinition",
        "}",
        "pub(super) fn association_default_name(kind: &str, ends: usize) -> Option<&'static str> {",
        "    match (kind, ends == 2) {",
        *[f"        ({json.dumps(kind)}, {str(binary).lower()}) => Some({json.dumps(names['binary' if binary else 'base'])})," for kind,names in doc["association_defaults"].items() for binary in (False,True)],
        "        _ => None,",
        "    }", "}", *render_usage(doc), ""])


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
    source = prefix + "adapter/FeatureAdapter.java"
    suffix = ".exe" if os.name == "nt" else ""
    java, javac = [args.java_bin / (name+suffix) for name in ("java","javac")]
    with tempfile.TemporaryDirectory(prefix="feature-defaults-") as temp:
        raw = Path(temp) / "raw.json"
        subprocess.run([str(javac),"-encoding","UTF-8","-cp",str(runtime),"-d",temp,str(AST_HELPER),str(DOCUMENT_HELPER),str(HELPER)],check=True,timeout=60)
        subprocess.run([str(java),"-cp",temp+os.pathsep+str(runtime),"dev.mercurio.pilot.PilotFeatureDefaultExporter",str(runtime),str(raw),str(PILOT/source),str(PILOT/(prefix+"adapter/AssociationAdapter.java")),str(PILOT/(prefix+"adapter/UsageAdapter.java")),str(PILOT/(prefix+"adapter/ReferenceUsageAdapter.java")),str(PILOT/(prefix+"delegate/invocation/Feature_typingFeatures_InvocationDelegate.java")),str(PILOT/(prefix+"delegate/setting/Usage_definition_SettingDelegate.java"))],check=True,timeout=90)
        doc = json.loads(raw.read_text(encoding="utf-8"))
    paths = {prefix+"util/UsageUtil.java",prefix+"adapter/ReferenceUsageAdapter.java",prefix+"adapter/UsageAdapter.java",prefix+"adapter/ConnectorAsUsageAdapter.java",prefix+"adapter/BindingConnectorAsUsageAdapter.java",source,"org.omg.sysml/model/SysML.ecore",prefix+"adapter/AssociationAdapter.java",prefix+"adapter/AssociationStructureAdapter.java",prefix+"adapter/ClassifierAdapter.java",prefix+"delegate/setting/Feature_type_SettingDelegate.java",prefix+"delegate/setting/Usage_definition_SettingDelegate.java"}
    paths.update(prefix+p for p in ["adapter/FeatureAdapter.java","adapter/TypeAdapter.java","util/ImplicitGeneralizationMap.java","util/TypeUtil.java","util/ElementUtil.java","util/FeatureUtil.java"])
    paths.update(prefix+p for p in ["delegate/invocation/Feature_typingFeatures_InvocationDelegate.java","delegate/setting/Association_associationEnd_SettingDelegate.java","delegate/setting/Association_relatedType_SettingDelegate.java","delegate/setting/Association_sourceType_SettingDelegate.java","delegate/setting/Association_targetType_SettingDelegate.java"])
    hashes = {}
    for path in sorted(paths):
        content = (PILOT/path).read_bytes().replace(b"\r\n",b"\n")
        require(content == subprocess.check_output(["git","-C",str(PILOT),"show",PIN+":"+path]).replace(b"\r\n",b"\n"),"Changed pinned source "+path)
        hashes[path] = digest(content)
    doc["provenance"] = dict(pilot_revision=PIN,runtime_sha256=runtime_hash,source_sha256=hashes,helper_sha256=digest(HELPER.read_bytes()),ast_helper_sha256=digest(AST_HELPER.read_bytes()),driver_sha256=digest(Path(__file__).read_bytes()),document_helper_sha256=digest(DOCUMENT_HELPER.read_bytes()),toolchain={name:subprocess.check_output([str(exe),"-version"],stderr=subprocess.STDOUT,text=True).strip() for name,exe in [("java",java),("javac",javac)]})
    doc["scope"] = "Resolved Feature and Association selectors, bounded Usage contributions and ReferenceUsage fallback guarded by complete transition-link identity. Factory selectors, source model observations and native semantic verification are separate evidence. End Usage multiplicity, transition effects, global resources and full transformation remain unqualified."
    generated = render(doc)
    serialized = json.dumps(doc,indent=2,sort_keys=True)+"\n"
    if args.check:
        require(OUTPUT.read_text(encoding="utf-8") == serialized and GENERATED.read_text(encoding="utf-8") == generated,"Stale Feature default artifacts")
    else:
        OUTPUT.write_text(serialized,encoding="utf-8",newline="\n")
        GENERATED.write_text(generated,encoding="utf-8",newline="\n")
    print("Feature 384; Association 16; Usage 192 selector controls; ReferenceUsage 33 end and 48 parameter controls; six transition-link identity controls; four Usage source models; 576 inherited typing controls")


if __name__ == "__main__":
    main()
