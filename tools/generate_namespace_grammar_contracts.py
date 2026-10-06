"""Derive a bounded namespace contract from the complete upstream Xtext AST.

This is a projection, not a replacement grammar. Unassigned rule calls are
expanded only until a model assignment; assignment terminals remain explicit.
The full grammar artifact preserves syntax that this projection does not use.
"""
import argparse
import hashlib
import json
from pathlib import Path

from export_pilot_grammar_structure import validate_structure

GRAMMARS = {"sysml": "org.omg.sysml.xtext.SysML", "kerml": "org.omg.kerml.xtext.KerML"}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def ref(value):
    if not isinstance(value, dict) or set(value) != {"$ref"}:
        raise ValueError(f"Expected resolved model reference: {value!r}")
    return value["$ref"]


class Grammar:
    def __init__(self, document):
        self.rules = {}
        for grammar in document["grammars"]:
            for rule in grammar["rules"]:
                if rule["id"] in self.rules:
                    raise ValueError("Duplicate qualified rule " + rule["id"])
                self.rules[rule["id"]] = rule

    def rule(self, identity):
        try:
            return self.rules[identity]
        except KeyError:
            raise ValueError("Unresolved grammar rule " + identity) from None

    def metaclass(self, identity):
        rule = self.rule(identity)
        classifier = ref(rule["fields"]["type"]["fields"]["classifier"])
        if "#//" not in classifier:
            raise ValueError("Expected imported Ecore classifier: " + classifier)
        uri, name = classifier.rsplit("#//", 1)
        namespaces = {"https://www.omg.org/spec/SysML/20250201": "SysML",
                      "http://www.eclipse.org/emf/2002/Ecore": "Ecore"}
        if uri not in namespaces:
            raise ValueError("Unsupported classifier namespace " + uri)
        return namespaces[uri] + "::" + name

    def assignments(self, identity):
        """Keep grammar paths/cardinalities; stop traversal at assignments."""
        found = []

        def visit(node, stack, context):
            kind, fields = node["kind"], node["fields"]
            context = context + [{"kind": kind, "cardinality": fields.get("cardinality", ""),
                                  "predicated": fields.get("predicated", False),
                                  "first_set_predicated": fields.get("firstSetPredicated", False)}]
            if kind == "Assignment":
                found.append({"feature": fields["feature"], "operator": fields["operator"],
                              "terminal": fields["terminal"], "rule_path": stack,
                              "context": context, "span": node.get("span")})
            elif kind in ("Group", "Alternatives", "UnorderedGroup"):
                for child in fields["elements"]:
                    visit(child, stack, context)
            elif kind == "RuleCall":
                target = ref(fields["rule"])
                if target in stack:
                    raise ValueError("Recursive unassigned projection: " + " -> ".join(stack + [target]))
                visit(self.rule(target)["fields"]["alternatives"], stack + [target], context)
            elif kind == "Keyword":
                return
            else:
                raise ValueError(f"Unsupported namespace projection node {kind} in {stack[-1]}")

        visit(self.rule(identity)["fields"]["alternatives"], [identity], [])
        return found

    def direct_choices(self, identity):
        node = self.rule(identity)["fields"]["alternatives"]
        nodes = node["fields"]["elements"] if node["kind"] == "Alternatives" else [node]
        if any(n["kind"] != "RuleCall" or n["fields"].get("cardinality") for n in nodes):
            raise ValueError("Expected direct unassigned alternatives in " + identity)
        return [ref(n["fields"]["rule"]) for n in nodes]

    def enum(self, identity):
        node = self.rule(identity)["fields"]["alternatives"]
        nodes = node["fields"]["elements"] if node["kind"] == "Alternatives" else [node]
        result = []
        for node in nodes:
            if node["kind"] != "EnumLiteralDeclaration":
                raise ValueError("Unsupported enum branch in " + identity)
            fields = node["fields"]
            if fields["literal"]["kind"] != "Keyword":
                raise ValueError("Unsupported enum token in " + identity)
            result.append({"token": fields["literal"]["fields"]["value"],
                           "value": ref(fields["enumLiteral"]).rsplit("/", 1)[1]})
        if len({row["token"] for row in result}) != len(result):
            raise ValueError("Ambiguous enum token mapping in " + identity)
        return result


class Metamodel:
    def __init__(self, document):
        self.classes = {row["qualified_name"] for row in document["metaclasses"]}
        self.parents = {}
        self.fields = {}
        for row in document["generalizations"]:
            self.parents.setdefault(row["specific"], []).append(row["general"])
        for row in document["structural_features"]:
            key = (row["owner"], row["name"])
            if key in self.fields:
                raise ValueError("Duplicate Ecore structural feature " + str(key))
            self.fields[key] = row

    def ancestors(self, kind):
        seen, pending = set(), [kind]
        while pending:
            current = pending.pop()
            if current not in seen:
                seen.add(current)
                pending.extend(self.parents.get(current, []))
        return seen

    def field(self, kind, name):
        owners = self.ancestors(kind)
        candidates = [row for (owner, field), row in self.fields.items() if owner in owners and field == name]
        # A redeclaration on a more specific class shadows its inherited name.
        candidates = [row for row in candidates if not any(row["owner"] != other["owner"]
                      and row["owner"] in self.ancestors(other["owner"]) for other in candidates)]
        if len(candidates) != 1:
            raise ValueError(f"Missing or ambiguous Ecore field {kind}::{name}")
        return candidates[0]

    def bind(self, grammar, assignment):
        owner = grammar.metaclass(assignment["rule_path"][0])
        field = self.field(owner, assignment["feature"])
        operator = assignment["operator"]
        if operator not in ("=", "+=", "?="):
            raise ValueError("Unsupported assignment operator " + operator)
        if operator == "+=" and field["upper_bound"] == 1:
            raise ValueError("Additive assignment to singular Ecore field " + field["qualified_name"])
        if operator == "?=" and not (field["kind"] == "attribute" and str(field["target"]).endswith("EBoolean")):
            raise ValueError("Boolean assignment to non-Boolean Ecore field " + field["qualified_name"])
        terminal = assignment["terminal"]
        # The legacy structural extract qualifies builtin Ecore datatypes with
        # the importing package name. Normalize only the two builtin scalar
        # types used by this projection; classifier references remain exact.
        field_type = field["target"]
        if field_type in ("SysML::EString", "SysML::EBoolean") and field_type not in self.classes:
            field_type = "Ecore::" + field_type.rsplit("::", 1)[1]
        if terminal["kind"] == "CrossReference":
            if field["kind"] != "reference" or field["containment"]:
                raise ValueError("Cross-reference does not match Ecore field " + field["qualified_name"])
            classifier = ref(terminal["fields"]["type"]["fields"]["classifier"])
            target = "SysML::" + classifier.rsplit("#//", 1)[1]
            if field["target"] not in self.ancestors(target):
                raise ValueError("Cross-reference type does not conform to Ecore field " + field["qualified_name"])
        elif terminal["kind"] == "RuleCall" and operator != "?=":
            result_type = grammar.metaclass(ref(terminal["fields"]["rule"]))
            if result_type in self.classes:
                if field["kind"] != "reference" or not field["containment"]:
                    raise ValueError("Constructed object requires Ecore containment " + field["qualified_name"])
                if field["target"] not in self.ancestors(result_type):
                    raise ValueError("Constructed type does not conform to Ecore field " + field["qualified_name"])
            elif field["kind"] != "attribute" or field_type != result_type:
                raise ValueError("Datatype/enum rule does not match Ecore field " + field["qualified_name"])
        elif terminal["kind"] == "Keyword":
            expected = "Ecore::EBoolean" if operator == "?=" else "Ecore::EString"
            if field["kind"] != "attribute" or field_type != expected:
                raise ValueError("Keyword does not match Ecore field " + field["qualified_name"])
        elif terminal["kind"] != "RuleCall":
            raise ValueError("Unsupported assignment terminal " + terminal["kind"])
        assignment["field_contract"] = field


def build(document, metamodel):
    if document["provenance"]["pilot_revision"] != metamodel["source"]["pilot"]["commit"]:
        raise ValueError("Grammar/metamodel Pilot revision mismatch")
    model_sources = {row["path"]: row["sha256"] for row in metamodel["source"]["source_files"]}
    model_source = document["metamodel"]
    if model_sources.get(model_source["path"]) != model_source["sha256"]:
        raise ValueError("Grammar/metamodel Ecore source hash mismatch")
    grammar = Grammar(document)
    model = Metamodel(metamodel)
    languages = {}
    for language, name in GRAMMARS.items():
        rule_id = lambda rule: name + "::" + rule
        body = grammar.assignments(rule_id("RelationshipBody"))
        # An assignment defines where objects are stored. Keep both permitted
        # ownership paths; never flatten annotations into ordinary owned elements.
        slots = []
        for assignment in body:
            terminal = assignment["terminal"]
            if terminal["kind"] != "RuleCall" or assignment["operator"] != "+=":
                raise ValueError("Unsupported relationship body assignment")
            target = ref(terminal["fields"]["rule"])
            slots.append({"feature": assignment["feature"], "rule": target,
                          "metaclass": grammar.metaclass(target), "evidence": assignment})
        annotation_rule = rule_id("OwnedAnnotation")
        if not any(row["rule"] == annotation_rule for row in slots):
            raise ValueError("Relationship body has no owned annotation alternative")
        owned_element = None
        for slot in slots:
            if slot["feature"] != "ownedRelatedElement":
                continue
            if owned_element is not None:
                raise ValueError("Multiple owned related element arms")
            families = grammar.direct_choices(slot["rule"])
            owned_element = {
                "rule": slot["rule"],
                "families": [
                    {"rule": family, "elements": [
                        {"rule": choice, "metaclass": grammar.metaclass(choice)}
                        for choice in grammar.direct_choices(family)
                    ]}
                    for family in families
                ],
            }
        annotation_assignment = grammar.assignments(annotation_rule)
        if len(annotation_assignment) != 1:
            raise ValueError("Unsupported OwnedAnnotation construction")
        annotation_target = annotation_assignment[0]["terminal"]
        if annotation_target["kind"] != "RuleCall":
            raise ValueError("Unsupported annotation child expression")
        annotation_choices = grammar.direct_choices(ref(annotation_target["fields"]["rule"]))
        queries = {}
        for form in ("MembershipImport", "NamespaceImport"):
            queries[form] = {"rule": rule_id(form), "metaclass": grammar.metaclass(rule_id(form)),
                             "assignments": grammar.assignments(rule_id(form))}
        row = {"grammar": name,
               "relationship_body": {"rule": rule_id("RelationshipBody"), "slots": slots},
               "owned_related_element": owned_element,
               "owned_annotation": {"metaclass": grammar.metaclass(annotation_rule),
                                    "assignment": annotation_assignment[0],
                                    "elements": [{"rule": r, "metaclass": grammar.metaclass(r)} for r in annotation_choices]},
               "alias": {"rule": rule_id("AliasMember"), "metaclass": grammar.metaclass(rule_id("AliasMember")),
                         "assignments": grammar.assignments(rule_id("AliasMember"))},
               "visibility": grammar.enum(rule_id("VisibilityIndicator")), "queries": queries}
        if language == "sysml":
            row["expose_visibility"] = grammar.enum(rule_id("ExposeVisibilityKind"))
            for form in ("MembershipExpose", "NamespaceExpose"):
                queries[form] = {"rule": rule_id(form), "metaclass": grammar.metaclass(rule_id(form)),
                                 "assignments": grammar.assignments(rule_id(form))}
        assignments = body + annotation_assignment + row["alias"]["assignments"]
        assignments += [a for query in queries.values() for a in query["assignments"]]
        for assignment in assignments:
            model.bind(grammar, assignment)
        for query in queries.values():
            if query["metaclass"] not in model.classes:
                raise ValueError("Unknown query Ecore class " + query["metaclass"])
        languages[language] = row
    return {"schema": "dev.mercurio.namespace-grammar-contract.v1", "languages": languages}


def render_rust(contract):
    lines = ["// Generated by tools/generate_namespace_grammar_contracts.py; do not edit.",
             "// Source: pinned Xtext AST. Procedural semantics remain separate."]
    for language, row in contract["languages"].items():
        prefix = language.upper()
        kinds = [element["metaclass"] for element in row["owned_annotation"]["elements"]]
        lines.append(f"pub(crate) const {prefix}_ANNOTATING_KINDS: &[&str] = &[" + ", ".join(json.dumps(k) for k in kinds) + "];" )
        fields = [slot["feature"] for slot in row["relationship_body"]["slots"]]
        lines.append(f"pub(crate) const {prefix}_RELATIONSHIP_BODY_FIELDS: &[&str] = &[" + ", ".join(json.dumps(k) for k in fields) + "];" )
        if row["owned_related_element"] is not None:
            rules = [element["rule"].rsplit("::", 1)[1]
                     for family in row["owned_related_element"]["families"]
                     for element in family["elements"]]
            if len(rules) != len(set(rules)):
                raise ValueError("Duplicate OwnedRelatedElement choice")
            lines.append(f"pub(crate) const {prefix}_OWNED_ELEMENT_RULES: &[&str] = &[" + ", ".join(json.dumps(k) for k in rules) + "];" )
        for form, query in row["queries"].items():
            # Retain rule names in constant names; no derived heuristic keywords.
            lines.append(f"pub(crate) const {prefix}_{form.upper()}: &str = " + json.dumps(query["metaclass"]) + ";")
    visibility = contract["languages"]["sysml"]["expose_visibility"]
    if len(visibility) != 1 or visibility[0]["token"] != "expose":
        raise ValueError("Native expose token adapter requires one expose enum literal")
    lines.append("pub(crate) const EXPOSE_VISIBILITY: &str = " + json.dumps(visibility[0]["value"]) + ";")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("grammar", "metamodel", "out", "rust-out"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    document = json.loads(args.grammar.read_text(encoding="utf-8"))
    validate_structure(document)
    contract = build(document, json.loads(args.metamodel.read_text(encoding="utf-8")))
    contract["source"] = {"grammar_sha256": digest(args.grammar), "metamodel_sha256": digest(args.metamodel),
                          "generator_sha256": digest(Path(__file__)), "generator": Path(__file__).name}
    outputs = [(args.out, json.dumps(contract, indent=2, ensure_ascii=False) + "\n"),
               (args.rust_out, render_rust(contract))]
    for path, content in outputs:
        if args.check:
            if path.read_text(encoding="utf-8") != content:
                raise ValueError("Namespace grammar contract drift: " + str(path))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8", newline="\n")
    print("Verified namespace grammar contracts for SysML and KerML")


if __name__ == "__main__":
    main()
