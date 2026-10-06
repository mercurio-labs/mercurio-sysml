"""Reviewed native algorithm bindings for pinned Ecore documentation rules.

This preserves documentation as documentation and resolves declaration IDs.
It is not an OCL/Xtend translator; executable semantics live in named Rust.
"""
from pathlib import Path
import json, hashlib
ROOT=Path(__file__).resolve().parents[1]
DATA=ROOT/"crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

def main():
    source=DATA/"ecore-semantics.extract.json"; definitions=json.loads(source.read_text(encoding="utf-8"))
    elements={row["id"]:row for row in definitions["elements"]}
    def declaration(kind,name=None):
        rows=[row for row in elements.values() if row["name"]==(name or kind) and (name is None or row.get("owner","").endswith("#//"+kind))]
        assert len(rows)==1
        return rows[0]["id"]
    argument=declaration("InstantiationExpression","argument");result=declaration("Expression","result")
    reviews={
      "IndexExpression":("checkIndexExpressionResultSpecialization", "arguments->notEmpty() and not arguments->first().result.specializesFromLibrary('Collections::Collection') implies result.specializes(arguments->first().result)","Collections::Collection"),
      "SelectExpression":("checkSelectExpressionResultSpecialization", "arguments->notEmpty() implies result.specializes(arguments->first().result)",None),
    }
    bindings=[]
    for kind,(rule,expected,guard) in reviews.items():
        declaration_id=declaration(kind);node=elements[declaration_id]
        assert node["super_types"]==["#//OperatorExpression"]
        details=[child["attributes"]["value"] for annotation in node["annotations"] for child in annotation["children"]
                 if child.get("attributes",{}).get("key")=="documentation"]
        assert len(details)==1
        text=" ".join(details[0].split());assert expected in text,"Normative documentation changed; review the native binding"
        bindings.append(dict(kind=kind,classifier_id=declaration_id,normative_rule=rule,argument_feature_id=argument,result_feature_id=result,
            first_argument_ordinal=0,unless_specializes_library=guard,documentation=details[0],documentation_interpretation="documentation_only_not_executable",
            native_algorithm="handwritten argument_result_specialization::plan/materialize_batch",translation_status="reviewed_manual_algorithm_binding"))
    output=DATA/"argument-result-specialization-bindings.extract.json"
    output.write_text(json.dumps(dict(schema="dev.mercurio.argument-result-specialization-bindings.v1",qualification_certificate=False,
        source_sha256={str(source.relative_to(ROOT)):sha(source)},bindings=bindings,
        excluded={"CollectExpression":"No first-argument result specialization in this class documentation; separate operator/invocation behavior remains required."}),indent=2)+"\n",encoding="utf-8")
    rust=ROOT/"crates/mercurio-sysml/src/language_frontend/lowering/emit/argument_result_specialization_generated.rs"
    lines=["// Generated reviewed bindings from pinned Ecore declarations; execution is handwritten.",
      "pub(super) struct Rule { pub(super) unless_library: Option<&'static str>, pub(super) normative_rule: &'static str }",
      "pub(super) fn rule(kind: &str) -> Option<Rule> { match kind {"]
    for binding in bindings:
        guard="Some(\""+binding["unless_specializes_library"]+"\")" if binding["unless_specializes_library"] else "None"
        lines.append("    "+json.dumps(binding["kind"])+" => Some(Rule { unless_library: "+guard+", normative_rule: "+json.dumps(binding["normative_rule"])+" }),")
    lines.extend(["    _ => None,", "} }", "pub(super) const BINDINGS_SHA256: &str = "+json.dumps(sha(output))+";"])
    rust.write_text("\n".join(lines)+"\n",encoding="utf-8")
    print("Two resolved Ecore/native algorithm bindings generated; no semantic qualification claimed")

if __name__=="__main__":main()
