"""Mutation controls for actual helper AST translation and direct Rust emission."""
from copy import deepcopy
import json
from pathlib import Path
import unittest
from translate_pilot_validators import DEFAULT_INPUT, translate, TranslationError
from translate_pilot_type_family import ALL_TYPES, INSTANCE, LENGTH
from generate_validation_rust import generate, GenerationError


def walk(value):
    if isinstance(value, dict):
        yield value
        for child in value.values(): yield from walk(child)
    elif isinstance(value, list):
        for child in value: yield from walk(child)

class FamilyTests(unittest.TestCase):
    def setUp(self):
        self.doc = json.loads(DEFAULT_INPUT.read_text(encoding="utf-8"))
        self.helper = next(m for m in self.doc["methods"] if m["name"] == "checkOneType")

    def test_complete_family_is_inlined_and_uses_explicit_services(self):
        result = translate(self.doc)
        self.assertEqual(len(result["rules"]), 11)
        self.assertEqual(len(result["inlined_helpers"]), 1)
        families = [r for r in result["rules"] if ALL_TYPES in r["dependencies"]]
        self.assertEqual(len(families), 7)
        for rule in families:
            self.assertEqual(set(rule["dependencies"]), {ALL_TYPES, INSTANCE})
            kinds = {n.get("kind") for n in walk(rule["body"])}
            self.assertTrue({"length", "int_eq", "exists", "is_instance", "feature_literal"} <= kinds)

    def test_cardinality_mutation_changes_ir_and_rust(self):
        literal = next(n for n in walk(self.helper) if n.get("kind") == "XNumberLiteral")
        literal["fields"]["value"] = "2"
        result = translate(self.doc)
        code, _ = generate(result, "a" * 64, "b" * 64)
        self.assertEqual(code.count(b"Value::Int(2_i32)"), 7)
        self.assertNotIn(b"Value::Int(1_i32)", code)

    def test_changed_caller_message_is_not_hardcoded(self):
        for symbol in self.doc["symbols"].values():
            if symbol.get("simple_name") == "INVALID_ENUMERATION_USAGE_TYPE_MSG":
                symbol["constant_value"] = "changed upstream message"
                symbol["initializer"]["fields"]["value"] = "changed upstream message"
        code, _ = generate(translate(self.doc), "a" * 64, "b" * 64)
        self.assertIn(b"changed upstream message", code)

    def test_mutable_helper_local_rejected(self):
        node = next(n for n in walk(self.helper) if n.get("kind") == "XtendVariableDeclaration")
        node["fields"]["writeable"] = True
        with self.assertRaisesRegex(TranslationError, "mutable"): translate(self.doc)

    def test_nonterminal_return_rejected(self):
        items = self.helper["fields"]["expression"]["fields"]["expressions"]
        items.insert(0, deepcopy(items[-1]))
        with self.assertRaisesRegex(TranslationError, "unsupported statement"): translate(self.doc)

    def test_unknown_collection_service_rejected(self):
        node = next(n for n in walk(self.helper) if n.get("fields", {}).get("feature") == {"$ref": ALL_TYPES})
        node["fields"]["feature"] = {"$ref": "unknown.service"}
        with self.assertRaisesRegex(TranslationError, "unresolved"): translate(self.doc)

    def test_wrong_length_overload_rejected(self):
        self.doc["symbols"][LENGTH]["parameter_types"] = ["java.lang.String"]
        with self.assertRaisesRegex(TranslationError, "length overload"): translate(self.doc)

    def test_missing_final_return_rejected(self):
        self.helper["fields"]["expression"]["fields"]["expressions"].pop()
        with self.assertRaisesRegex(TranslationError, "terminal explicit return"): translate(self.doc)

    def test_closure_extra_field_rejected(self):
        closure = next(n for n in walk(self.helper) if n.get("kind") == "XClosure")
        closure["fields"]["invented"] = True
        with self.assertRaisesRegex(TranslationError, "single-parameter closure"): translate(self.doc)

    def test_invalid_ecore_receiver_rejected(self):
        caller = next(m for m in self.doc["methods"] if m["name"] == "checkEnumerationUsage")
        node = next(n for n in walk(caller) if n.get("fields", {}).get("feature", {}).get("$ref", "").endswith("getEnumerationUsage_EnumerationDefinition()"))
        node["fields"]["memberCallTarget"]["fields"]["feature"] = {"$ref": INSTANCE}
        with self.assertRaisesRegex(TranslationError, "Ecore singleton"): translate(self.doc)

    def test_generator_rejects_unbound_instance_service(self):
        result = translate(self.doc)
        result["dependencies"] = [d for d in result["dependencies"] if d["symbol"] != INSTANCE]
        with self.assertRaises(GenerationError): generate(result, "a" * 64, "b" * 64)

    def test_generator_rejects_shadowed_closure_local(self):
        result = translate(self.doc)
        rule = next(r for r in result["rules"] if ALL_TYPES in r["dependencies"])
        closure = next(n for n in walk(rule) if n.get("kind") == "exists")
        closure["parameter"]["name"] = rule["parameter"]["name"]
        with self.assertRaisesRegex(GenerationError, "Shadowed"): generate(result, "a" * 64, "b" * 64)

if __name__ == "__main__": unittest.main()
