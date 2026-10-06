"""Independent truth tables and fail-closed controls for typed Xtend translation."""
from copy import deepcopy
import json
from pathlib import Path
import unittest

from translate_pilot_validators import (
    BOOL, DEFAULT_INPUT, EOBJECT, MODEL, OPERATORS, RESOURCES, STRING,
    TranslationError, encode, translate,
)


def walk(value):
    if isinstance(value, dict):
        if "kind" in value:
            yield value
        for child in value.values():
            yield from walk(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk(child)


def method(document, name):
    return next(item for item in document["methods"] if item["name"] == name)


def rule(document, name):
    return next(item for item in translate(document)["rules"] if item["name"] == name)


def interpret(program, subject, getters):
    """Test-only IR interpreter; expected results come from separate controls."""
    emitted = []

    def expression(node, variables):
        kind = node["kind"]
        if kind == "null":
            return None
        if kind in {"bool", "string"}:
            return node["value"]
        if kind == "enum":
            return (node["type"], node["name"])
        if kind == "local":
            return variables[node["name"]]
        if kind == "get":
            receiver = expression(node["receiver"], variables)
            if receiver is None:
                raise AssertionError("null getter receiver: short-circuit semantics changed")
            return getters[(receiver, node["symbol"])]
        if kind == "not":
            return not expression(node["operand"], variables)
        left = expression(node["left"], variables)
        if kind == "and":
            return left and expression(node["right"], variables)
        if kind == "or":
            return left or expression(node["right"], variables)
        right = expression(node["right"], variables)
        if kind == "identity_eq":
            return left == right
        if kind == "identity_ne":
            return left != right
        raise AssertionError(kind)

    def execute(operations, variables):
        variables = variables.copy()
        for operation in operations:
            kind = operation["op"]
            if kind == "let":
                if operation["name"] in variables:
                    raise AssertionError("duplicate immutable local")
                variables[operation["name"]] = expression(operation["value"], variables)
            elif kind == "block":
                execute(operation["body"], variables)
            elif kind == "if":
                branch = "then" if expression(operation["condition"], variables) else "else"
                execute(operation[branch], variables)
            elif kind == "diagnostic":
                emitted.append({
                    "severity": operation["severity"],
                    "message": expression(operation["message"], variables),
                    "code": expression(operation["code"], variables),
                    "subject": expression(operation["subject"], variables),
                    "feature": expression(operation["feature"], variables),
                    "index": operation["index"],
                    "data": [expression(value, variables) for value in operation["data"]],
                })
            else:
                raise AssertionError(kind)

    execute(program["body"], {program["parameter"]["name"]: subject})
    return emitted


class TypedXtendTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original = json.loads(DEFAULT_INPUT.read_text(encoding="utf-8"))
        cls.independent = json.loads((RESOURCES / "namespace-checks.extract.json").read_text(encoding="utf-8"))

    def setUp(self):
        self.document = deepcopy(self.original)

    def assert_rejected(self, pattern):
        with self.assertRaisesRegex(TranslationError, pattern) as raised:
            translate(self.document)
        self.assertIn("KerMLValidator.xtend:", str(raised.exception))
        self.assertIn("#check", str(raised.exception))

    def test_actual_ast_compiles_deterministically_with_exact_dependencies(self):
        result = translate(self.document)
        self.assertEqual(encode(result), encode(translate(self.document)))
        self.assertEqual({item["name"] for item in result["rules"]}, {"checkAnnotation", "checkImport", "checkReferenceUsage", "checkEnumerationDefinition", "checkEnumerationUsage", "checkAnalysisCaseUsage", "checkVerificationCaseUsage", "checkUseCaseUsage", "checkRenderingUsage", "checkViewpointUsage", "checkMetadataUsage"})
        self.assertEqual(len(result["dependencies"]), 10)
        self.assertEqual(result["types"], self.document["types"])
        self.assertIn(MODEL + "Element", result["types"][MODEL + "Namespace"]["supertypes"])
        for item in result["rules"]:
            self.assertEqual(len(item["dependencies"]), 3 if item["name"] in {"checkImport", "checkAnnotation"} else (1 if item["name"] in {"checkReferenceUsage", "checkEnumerationDefinition"} else 2))
            self.assertIn("span", item["source"])

    def test_all_annotation_presence_masks_match_independent_legacy_extract(self):
        program = rule(self.document, "checkAnnotation")
        names = ["getOwnedAnnotatingElement", "getOwningAnnotatingElement", "getOwningAnnotatedElement"]
        for mask in range(8):
            with self.subTest(mask=mask):
                getters = {( "annotation", MODEL + "Annotation." + name + "()"): (f"element-{index}" if mask & (1 << index) else None) for index, name in enumerate(names)}
                actual = interpret(program, "annotation", getters)
                expected = self.independent["annotation"]["failures_by_mask"][mask]
                self.assertEqual([{"issue": item["code"], "message": item["message"]} for item in actual], expected)
                for item in actual:
                    self.assertEqual((item["severity"], item["subject"], item["feature"], item["index"], item["data"]), ("error", "annotation", None, None, []))

    def test_all_import_visibility_and_ownership_combinations(self):
        program = rule(self.document, "checkImport")
        # Includes invalid enum absence in addition to the three enum values.
        for namespace in (None, "namespace"):
            for owner in (None, "owner"):
                for visibility in (None, "PRIVATE", "PROTECTED", "PUBLIC"):
                    with self.subTest(namespace=namespace, owner=owner, visibility=visibility):
                        getters = {
                            ("import", MODEL + "Import.getImportOwningNamespace()"): namespace,
                            ("namespace", MODEL + "Element.getOwner()"): owner,
                            ("import", MODEL + "Import.getVisibility()"): None if visibility is None else (MODEL + "VisibilityKind", visibility),
                        }
                        actual = interpret(program, "import", getters)
                        expected_count = int(namespace is not None and owner is None and visibility != "PRIVATE")
                        self.assertEqual(len(actual), expected_count)
                        for item in actual:
                            self.assertEqual(item["code"], self.independent["import_top_level"]["issue"])
                            self.assertEqual(item["message"], self.independent["import_top_level"]["message"])
                            self.assertEqual(item["subject"], "import")

    def test_semantic_operator_mutation_changes_generated_predicate(self):
        before = translate(self.document)
        target = next(node for node in walk(method(self.document, "checkImport")) if node["kind"] == "XBinaryOperation" and node["fields"]["feature"]["$ref"].endswith("operator_and(boolean,boolean)"))
        target["fields"]["feature"]["$ref"] = "org.eclipse.xtext.xbase.lib.BooleanExtensions.operator_or(boolean,boolean)"
        after = translate(self.document)
        self.assertNotEqual(before["rules"], after["rules"])
        changed = next(item for item in after["rules"] if item["name"] == "checkImport")
        self.assertEqual(changed["body"][0]["condition"]["kind"], "or")

    def test_method_name_does_not_select_or_reconstruct_predicate(self):
        before = rule(self.document, "checkImport")["body"]
        target = method(self.document, "checkImport")
        target["name"] = "checkRenamed"
        target["fields"]["name"] = "checkRenamed"
        self.assertEqual(rule(self.document, "checkRenamed")["body"], before)

    def test_unknown_getter_dependency_fails_with_source_and_method(self):
        getter = MODEL + "Annotation.getOwnedAnnotatingElement()"
        self.document["symbols"][getter]["simple_name"] = "getUnknownProperty"
        self.assert_rejected("unknown getter dependency")

    def test_unresolved_call_fails(self):
        node = next(n for n in walk(method(self.document, "checkImport")) if n["kind"] == "XMemberFeatureCall")
        node["fields"]["feature"] = {"$ref": "missing.link"}
        self.assert_rejected("unresolved call or binding")

    def test_unknown_model_parameter_type_fails(self):
        method(self.document, "checkImport")["fields"]["parameters"][0]["type"] = "UnknownModelType"
        self.assert_rejected("unknown or non-model parameter type")

    def test_unknown_ast_statement_fails(self):
        method(self.document, "checkImport")["fields"]["expression"]["fields"]["expressions"][0]["kind"] = "XForLoopExpression"
        self.assert_rejected("unsupported statement AST")

    def test_mutable_local_fails(self):
        node = next(n for n in walk(method(self.document, "checkAnnotation")) if n["kind"] == "XtendVariableDeclaration")
        node["fields"]["writeable"] = True
        self.assert_rejected("mutable variables")

    def test_null_safe_getter_fails(self):
        node = next(n for n in walk(method(self.document, "checkImport")) if n["kind"] == "XMemberFeatureCall")
        node["fields"]["nullSafe"] = True
        self.assert_rejected("null-safe")

    def test_operator_reassignment_fails(self):
        node = next(n for n in walk(method(self.document, "checkImport")) if n["kind"] == "XBinaryOperation")
        node["fields"]["reassignFirstArgument"] = True
        self.assert_rejected("reassigning operators")

    def test_wrong_resolved_getter_type_fails(self):
        self.document["symbols"][MODEL + "Import.getVisibility()"]["type"] = STRING
        self.assert_rejected("getter signature differs")

    def test_primitive_identity_comparison_fails(self):
        node = next(n for n in walk(method(self.document, "checkImport")) if n["kind"] == "XBinaryOperation" and "triple" in n["fields"]["feature"]["$ref"])
        node["fields"]["leftOperand"] = {"id": "mutated", "kind": "XBooleanLiteral", "type": BOOL, "span": node["span"], "fields": {"isTrue": True}}
        self.assert_rejected("identity comparison requires reference")

    def test_value_equality_is_not_treated_as_identity(self):
        node = next(n for n in walk(method(self.document, "checkImport")) if n["kind"] == "XBinaryOperation" and "triple" in n["fields"]["feature"]["$ref"])
        old = node["fields"]["feature"]["$ref"]
        replacement = old.replace("operator_tripleNotEquals", "operator_notEquals").replace("operator_tripleEquals", "operator_equals")
        symbol = deepcopy(self.document["symbols"][old])
        symbol["simple_name"] = symbol["simple_name"].replace("tripleNotEquals", "notEquals").replace("tripleEquals", "equals")
        symbol["identifier"] = replacement
        self.document["symbols"][replacement] = symbol
        node["fields"]["feature"]["$ref"] = replacement
        self.assert_rejected("unsupported resolved operator")

    def test_diagnostic_warning_and_data_are_preserved(self):
        node = next(n for n in walk(method(self.document, "checkImport")) if n["kind"] == "XFeatureCall" and ".error(" in n["fields"]["feature"]["$ref"])
        identity = node["fields"]["feature"]["$ref"]
        warning = identity.replace(".error(", ".warning(")
        self.document["symbols"][warning] = {**self.document["symbols"][identity], "simple_name": "warning", "identifier": warning}
        node["fields"]["feature"]["$ref"] = warning
        node["fields"]["featureCallArguments"].append({"id": "data", "kind": "XStringLiteral", "type": STRING, "span": node["span"], "fields": {"value": "detail"}})
        compiled = rule(self.document, "checkImport")["body"][0]["then"][0]
        self.assertEqual(compiled["severity"], "warning")
        self.assertEqual(compiled["data"], [{"kind": "string", "type": STRING, "value": "detail"}])

    def test_unknown_ast_field_is_rejected(self):
        node = next(n for n in walk(method(self.document, "checkImport")) if n["kind"] == "XBinaryOperation")
        node["fields"]["newSemantics"] = True
        self.assert_rejected("unsupported XBinaryOperation fields")

    def test_changed_constant_cache_cannot_hide_source_initializer(self):
        symbol = next(s for s in self.document["symbols"].values() if s.get("simple_name") == "INVALID_IMPORT_TOP_LEVEL_VISIBILITY_MSG")
        symbol["constant_value"] = "not the source value"
        self.assert_rejected("constant differs")

    def test_nested_block_local_does_not_escape_its_scope(self):
        target = method(self.document, "checkAnnotation")["fields"]["expression"]["fields"]["expressions"]
        declaration = target[0]
        target[0] = {"id": "scoped", "kind": "XBlockExpression", "type": "void", "span": declaration["span"], "fields": {"expressions": [declaration]}}
        self.assert_rejected("unresolved call or binding")

    def test_disjoint_block_locals_and_outer_after_inner_keep_distinct_scopes(self):
        target = method(self.document, "checkImport")["fields"]["expression"]["fields"]["expressions"]
        span = target[0]["span"]
        def declaration(identity):
            return {"id": identity, "kind": "XtendVariableDeclaration", "type": "void", "span": span,
                    "fields": {"name": "shared", "right": {"id": identity + "/right", "kind": "XStringLiteral", "type": STRING, "span": span, "fields": {"value": identity}}, "type": None, "writeable": False, "extension": False}}
        for index in range(2):
            identity = f"nested-{index}"
            target.append({"id": identity, "kind": "XBlockExpression", "type": "void", "span": span, "fields": {"expressions": [declaration(identity + "/local")]}})
        target.append(declaration("outer-after-inner"))
        compiled = rule(self.document, "checkImport")
        self.assertEqual([op["op"] for op in compiled["body"][-3:]], ["block", "block", "let"])
        self.assertEqual(interpret(compiled, "import", {("import", MODEL + "Import.getImportOwningNamespace()"): None}), [])

    def test_null_diagnostic_message_code_and_data_are_rejected(self):
        for index, label in ((0, "message"), (3, "code"), (4, "data")):
            with self.subTest(label=label):
                self.document = deepcopy(self.original)
                node = next(n for n in walk(method(self.document, "checkImport")) if n["kind"] == "XFeatureCall" and ".error(" in n["fields"]["feature"]["$ref"])
                literal = {"id": "null-argument", "kind": "XNullLiteral", "type": "null", "span": node["span"], "fields": {}}
                arguments = node["fields"]["featureCallArguments"]
                if index == len(arguments):
                    arguments.append(literal)
                else:
                    arguments[index] = literal
                self.assert_rejected("null diagnostic " + label)

    def test_mutable_static_fields_cannot_be_folded_as_constants(self):
        symbol = next(s for s in self.document["symbols"].values() if s.get("simple_name") == "INVALID_IMPORT_TOP_LEVEL_VISIBILITY_MSG")
        symbol["final"] = False
        symbol["source_immutable"] = False
        self.assert_rejected("unsupported nonconstant field")

    def test_constant_without_source_immutability_evidence_is_rejected(self):
        symbol = next(s for s in self.document["symbols"].values() if s.get("simple_name") == "INVALID_IMPORT_TOP_LEVEL_VISIBILITY_MSG")
        del symbol["source_immutable"]
        self.assert_rejected("unsupported nonconstant field")

    def test_check_phase_requires_explicit_support(self):
        annotation = method(self.document, "checkImport")["fields"]["annotationInfo"]["fields"]["annotations"][0]
        annotation["fields"]["value"] = {"kind": "XStringLiteral", "type": STRING, "fields": {"value": "FAST"}}
        self.assert_rejected("nondefault or non-Check")

    def test_incomplete_method_body_is_rejected(self):
        method(self.document, "checkImport")["fields"]["expression"] = None
        self.assert_rejected("complete void body")

    def test_diagnostic_call_binding_is_not_selected_by_name_alone(self):
        symbol = next(s for s in self.document["symbols"].values() if s.get("simple_name") == "error")
        symbol["declaring_type"] = "example.UnrelatedReporter"
        self.assert_rejected("unsupported diagnostic call")


if __name__ == "__main__":
    unittest.main()
