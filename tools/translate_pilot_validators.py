"""Compile a deliberately bounded, typed subset of resolved Pilot Xtend AST.

No source text or method-specific predicates are interpreted here. Getter bindings
are a reviewed boundary: adding a getter requires a native implementation, rather
than silently treating an unknown property as absent. The generated program keeps
short-circuit evaluation, lexical scopes, and ordered diagnostics explicit.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
RESOURCES = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
DEFAULT_INPUT = RESOURCES / "validators.ast.extract.json"
DEFAULT_OUTPUT = RESOURCES / "validation-rules.extract.json"
MODEL = "org.omg.sysml.lang.sysml."
STRING = "java.lang.String"
BOOL = "boolean"
NULL = "null"
EOBJECT = "org.eclipse.emf.ecore.EObject"
FEATURE = "org.eclipse.emf.ecore.EStructuralFeature"
GETTERS = {
    MODEL + "Import.getImportOwningNamespace()": (MODEL + "Import", MODEL + "Namespace"),
    MODEL + "Import.getVisibility()": (MODEL + "Import", MODEL + "VisibilityKind"),
    MODEL + "Element.getOwner()": (MODEL + "Element", MODEL + "Element"),
    MODEL + "Annotation.getOwnedAnnotatingElement()": (MODEL + "Annotation", MODEL + "AnnotatingElement"),
    MODEL + "Annotation.getOwningAnnotatingElement()": (MODEL + "Annotation", MODEL + "AnnotatingElement"),
    MODEL + "Annotation.getOwningAnnotatedElement()": (MODEL + "Annotation", MODEL + "Element"),
    MODEL + "Definition.isVariation()": (MODEL + "Definition", BOOL),
    MODEL + "Usage.isReference()": (MODEL + "Usage", BOOL),
}
OPERATORS = {
    "org.eclipse.xtext.xbase.lib.BooleanExtensions.operator_and(boolean,boolean)": "and",
    "org.eclipse.xtext.xbase.lib.BooleanExtensions.operator_or(boolean,boolean)": "or",
    "org.eclipse.xtext.xbase.lib.BooleanExtensions.operator_not(boolean)": "not",
    "org.eclipse.xtext.xbase.lib.ObjectExtensions.operator_tripleEquals(java.lang.Object,java.lang.Object)": "identity_eq",
    "org.eclipse.xtext.xbase.lib.ObjectExtensions.operator_tripleNotEquals(java.lang.Object,java.lang.Object)": "identity_ne",
}
DIAGNOSTIC_OWNER = "org.eclipse.xtext.validation.AbstractDeclarativeValidator"
AST_FIELDS = {
    "XNullLiteral": set(),
    "XBooleanLiteral": {"isTrue"},
    "XStringLiteral": {"value"},
    "XFeatureCall": {"explicitOperationCall", "feature", "featureCallArguments", "invalidFeatureIssueCode", "typeArguments"},
    "XMemberFeatureCall": {"explicitOperationCall", "explicitStatic", "feature", "invalidFeatureIssueCode", "memberCallArguments", "memberCallTarget", "nullSafe", "typeArguments"},
    "XUnaryOperation": {"feature", "invalidFeatureIssueCode", "operand", "typeArguments"},
    "XBinaryOperation": {"feature", "invalidFeatureIssueCode", "leftOperand", "reassignFirstArgument", "rightOperand", "typeArguments"},
    "XVariableDeclaration": {"name", "right", "type", "writeable"},
    "XtendVariableDeclaration": {"extension", "name", "right", "type", "writeable"},
    "XBlockExpression": {"expressions"},
    "XIfExpression": {"conditionalExpression", "else", "if", "then"},
}


class TranslationError(ValueError):
    pass


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def encode(document: dict) -> bytes:
    return (json.dumps(document, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


class Compiler:
    def __init__(self, document: dict):
        if document.get("schema_version") != 1 or document.get("source_format") != "raw-xtend":
            raise TranslationError("Expected version 1 raw-xtend AST")
        if document.get("parser") != "org.eclipse.xtend.core.XtendStandaloneSetup":
            raise TranslationError("Expected the actual Xtend parser and linker")
        self.document = document
        self.symbols = document.get("symbols", {})
        self.types = document.get("types", {})
        self.rule = "<document>"
        self.source = {}
        self.dependencies: set[str] = set()
        self.all_dependencies: set[str] = set()
        self.bindings: dict[str, tuple[str, str]] = {}

    def fail(self, node, message):
        span = node.get("span", {}) if isinstance(node, dict) else {}
        path = self.source.get("path", self.source.get("file", "<unknown source>"))
        line = span.get("start_line", self.source.get("span", {}).get("start_line", "?"))
        raise TranslationError(f"{path}:{line} {self.rule}: {message}")

    def known_type(self, name):
        return name in {BOOL, STRING, NULL, "void", EOBJECT, FEATURE} or name in self.types

    def assignable(self, actual, expected):
        if actual == expected:
            return True
        if actual == NULL:
            return expected not in {BOOL, "void"}
        pending = [actual]
        seen = set()
        while pending:
            current = pending.pop()
            if current == expected:
                return True
            if current in seen:
                continue
            seen.add(current)
            info = self.types.get(current, {})
            pending.extend(info.get("supertypes", []) if isinstance(info, dict) else info)
        return False

    def require_type(self, expression, expected, node):
        if not self.assignable(expression["type"], expected):
            self.fail(node, f"type mismatch: expected {expected}, got {expression['type']}")

    def typed(self, node, kind, type_, **fields):
        if not self.known_type(type_):
            self.fail(node, f"unknown type {type_}")
        resolved = node.get("type")
        # XNullLiteral is assigned Object by Xbase in some contexts. Its semantic
        # type is nevertheless the null type, assignable to any reference type.
        if kind != "null" and resolved is not None and resolved != type_:
            self.fail(node, f"resolved type mismatch: AST {resolved}, expression {type_}")
        return {"kind": kind, "type": type_, **fields}

    def symbol(self, node):
        reference = node.get("fields", {}).get("feature")
        if not isinstance(reference, dict) or set(reference) != {"$ref"}:
            self.fail(node, "missing resolved feature reference")
        identity = reference["$ref"]
        if identity in self.bindings:
            return identity, None
        symbol = self.symbols.get(identity)
        if not isinstance(symbol, dict):
            self.fail(node, f"unresolved call or binding {identity}")
        return identity, symbol

    def signature(self, symbol, node):
        owner = symbol.get("declaring_type")
        name = symbol.get("simple_name")
        params = symbol.get("parameter_types")
        if not isinstance(owner, str) or not isinstance(name, str) or not isinstance(params, list):
            self.fail(node, "incomplete resolved operation signature")
        return owner + "." + name + "(" + ",".join(params) + ")"

    def expressions(self, node):
        fields = node["fields"]
        return fields.get("memberCallArguments", fields.get("featureCallArguments", []))

    def check_fields(self, node):
        kind = node.get("kind")
        allowed = AST_FIELDS.get(kind)
        if allowed is not None:
            extra = set(node.get("fields", {})) - allowed
            if extra:
                self.fail(node, f"unsupported {kind} fields: {sorted(extra)}")

    def expression(self, node):
        if not isinstance(node, dict) or not isinstance(node.get("fields"), dict):
            self.fail(node, "invalid AST expression")
        self.check_fields(node)
        kind, fields = node.get("kind"), node["fields"]
        if fields.get("invalidFeatureIssueCode") or fields.get("typeArguments"):
            self.fail(node, "unresolved feature or explicit generic type arguments")
        if fields.get("reassignFirstArgument"):
            self.fail(node, "reassigning operators are outside the supported subset")
        if kind == "XNullLiteral":
            return self.typed(node, "null", NULL)
        if kind == "XBooleanLiteral":
            value = fields.get("isTrue", False)
            if not isinstance(value, bool):
                self.fail(node, "invalid Boolean literal")
            return self.typed(node, "bool", BOOL, value=value)
        if kind == "XStringLiteral":
            if not isinstance(fields.get("value"), str):
                self.fail(node, "invalid string literal")
            return self.typed(node, "string", STRING, value=fields["value"])
        if kind in {"XBinaryOperation", "XUnaryOperation"}:
            _, symbol = self.symbol(node)
            if symbol is None:
                self.fail(node, "operator bound to a local variable")
            signature = self.signature(symbol, node)
            operator = OPERATORS.get(signature)
            if operator is None or not symbol.get("static") or symbol.get("type") != BOOL:
                self.fail(node, f"unsupported resolved operator {signature}")
            if operator == "not":
                if kind != "XUnaryOperation":
                    self.fail(node, "unary operator in binary AST")
                operand = self.expression(fields["operand"])
                self.require_type(operand, BOOL, node)
                return self.typed(node, "not", BOOL, operand=operand)
            if kind != "XBinaryOperation":
                self.fail(node, "binary operator in unary AST")
            left, right = self.expression(fields["leftOperand"]), self.expression(fields["rightOperand"])
            if operator in {"and", "or"}:
                self.require_type(left, BOOL, node)
                self.require_type(right, BOOL, node)
            elif left["type"] in {BOOL, "void"} or right["type"] in {BOOL, "void"}:
                self.fail(node, "identity comparison requires reference or null operands")
            elif left["type"] == STRING or right["type"] == STRING:
                self.fail(node, "string object identity is outside the supported subset")
            elif not (self.assignable(left["type"], right["type"]) or self.assignable(right["type"], left["type"])):
                self.fail(node, "incompatible reference types in identity comparison")
            return self.typed(node, operator, BOOL, left=left, right=right)
        if kind not in {"XFeatureCall", "XMemberFeatureCall"}:
            self.fail(node, f"unsupported expression AST {kind}")
        if fields.get("nullSafe"):
            self.fail(node, "null-safe calls are outside the supported subset")
        if self.expressions(node):
            self.fail(node, "expression calls with arguments are outside the supported subset")
        identity, symbol = self.symbol(node)
        if symbol is None:
            name, type_ = self.bindings[identity]
            return self.typed(node, "local", type_, name=name)
        if symbol.get("kind") == "JvmEnumerationLiteral":
            self.static_receiver(node, symbol)
            type_ = symbol.get("type", symbol.get("declaring_type"))
            if self.types.get(type_, {}).get("kind") not in {"JvmEnumerationType", "enum"}:
                self.fail(node, f"unregistered enumeration type {type_}")
            return self.typed(node, "enum", type_, name=symbol["simple_name"])
        if symbol.get("kind") == "JvmField":
            self.static_receiver(node, symbol)
            value = symbol.get("constant_value")
            initializer = symbol.get("initializer")
            if initializer is not None:
                translated_initializer = self.expression(initializer)
                if translated_initializer.get("kind") != "string" or translated_initializer.get("value") != value:
                    self.fail(node, "constant differs from its parsed Xtend initializer")
            if not symbol.get("static") or not symbol.get("final") or not symbol.get("source_immutable") or not symbol.get("source_static") or symbol.get("type") != STRING or not isinstance(value, str):
                self.fail(node, f"unsupported nonconstant field {identity}")
            return self.typed(node, "string", STRING, value=value)
        signature = self.signature(symbol, node)
        binding = GETTERS.get(signature)
        if binding is None:
            self.fail(node, f"unknown getter dependency {signature}")
        receiver_type, result_type = binding
        if symbol.get("static") or symbol.get("type") != result_type:
            self.fail(node, f"getter signature differs from reviewed binding {signature}")
        receiver_node = fields.get("memberCallTarget")
        if receiver_node is None:
            self.fail(node, f"getter requires explicit model receiver: {signature}")
        receiver = self.expression(receiver_node)
        self.require_type(receiver, receiver_type, node)
        self.dependencies.add(signature)
        return self.typed(node, "get", result_type, symbol=signature, receiver=receiver)

    def static_receiver(self, node, symbol):
        target = node["fields"].get("memberCallTarget")
        if target is None:
            return
        if target.get("kind") != "XFeatureCall":
            self.fail(node, "static values require a declaring-type receiver")
        _, target_symbol = self.symbol(target)
        if target_symbol is None or target_symbol.get("identifier") != symbol.get("declaring_type") or target_symbol.get("kind") not in {"JvmGenericType", "JvmEnumerationType"}:
            self.fail(node, "static receiver differs from the resolved declaring type")
        if self.expressions(target):
            self.fail(node, "static receiver cannot have arguments")

    def statements(self, node, scoped=True):
        if node is None:
            return []
        self.check_fields(node)
        restore_scope = scoped or node.get("kind") == "XBlockExpression"
        previous = self.bindings.copy()
        try:
            if node.get("kind") == "XBlockExpression":
                result = []
                for item in node["fields"].get("expressions", []):
                    result.extend(self.statements(item, scoped=False))
                return result if scoped else [{"op": "block", "body": result}]
            fields = node.get("fields", {})
            kind = node.get("kind")
            if kind in {"XVariableDeclaration", "XtendVariableDeclaration"}:
                if fields.get("extension") or fields.get("writeable", False):
                    self.fail(node, "mutable variables are outside the supported subset")
                name = fields.get("name")
                if not isinstance(name, str) or any(name == x[0] for x in self.bindings.values()):
                    self.fail(node, f"invalid or shadowed local {name}")
                value = self.expression(fields.get("right"))
                type_ = value["type"]
                if fields.get("type") is not None:
                    self.fail(node, "explicitly typed local declarations are outside the supported subset")
                if type_ == "void":
                    self.fail(node, f"local inferred type mismatch: {type_} / {value['type']}")
                identity = node.get("id")
                if not isinstance(identity, str) or identity in self.bindings:
                    self.fail(node, f"duplicate or absent local identity {identity}")
                self.bindings[identity] = (name, type_)
                return [{"op": "let", "name": name, "type": type_, "value": value}]
            if kind == "XIfExpression":
                condition = self.expression(fields.get("if"))
                self.require_type(condition, BOOL, node)
                return [{"op": "if", "condition": condition,
                         "then": self.statements(fields.get("then")),
                         "else": self.statements(fields.get("else"))}]
            if kind in {"XFeatureCall", "XMemberFeatureCall"}:
                return [self.diagnostic(node)]
            self.fail(node, f"unsupported statement AST {kind}")
        finally:
            if restore_scope:
                self.bindings = previous

    def diagnostic(self, node):
        _, symbol = self.symbol(node)
        if symbol is None:
            self.fail(node, "local used as a diagnostic call")
        signature = self.signature(symbol, node)
        severity = symbol.get("simple_name")
        expected = [STRING, EOBJECT, FEATURE, STRING, STRING + "[]"]
        if symbol.get("type") != "void" or not symbol.get("varargs") or symbol.get("declaring_type") != DIAGNOSTIC_OWNER or severity not in {"error", "warning"} or symbol.get("parameter_types") != expected or symbol.get("static"):
            self.fail(node, f"unsupported diagnostic call {signature}")
        fields = node["fields"]
        if fields.get("nullSafe") or fields.get("memberCallTarget") is not None:
            self.fail(node, "diagnostics require the implicit validator receiver")
        if fields.get("typeArguments") or fields.get("invalidFeatureIssueCode"):
            self.fail(node, "unresolved or generic diagnostic call")
        args = self.expressions(node)
        if len(args) < 4:
            self.fail(node, "diagnostic requires message, subject, feature, and code")
        values = [self.expression(arg) for arg in args]
        for expression, type_ in zip(values[:4], expected[:4]):
            self.require_type(expression, type_, node)
        for expression in values[4:]:
            self.require_type(expression, STRING, node)
        for label, expression in [("message", values[0]), ("code", values[3]), *[("data", item) for item in values[4:]]]:
            if expression["type"] == NULL:
                self.fail(node, f"null diagnostic {label} is outside the supported subset")
        return {"op": "diagnostic", "severity": severity,
                "message": values[0], "subject": values[1], "feature": values[2],
                "code": values[3], "index": None, "data": values[4:]}

    def method(self, method):
        fields = method.get("fields", {})
        name = fields.get("name", method.get("name"))
        owner = method.get("declaring_type")
        if not isinstance(name, str) or not isinstance(owner, str):
            self.fail(method, "missing method identity")
        self.rule = owner + "#" + name
        self.source = method.get("source", {})
        if isinstance(self.source, str):
            self.source = {"path": self.source}
        if method.get("kind") != "XtendFunction":
            self.fail(method, "expected XtendFunction")
        if fields.get("createExtensionInfo") or fields.get("exceptions") or fields.get("typeParameters"):
            self.fail(method, "generic, creating, or throwing validators are outside the supported subset")
        parameters = fields.get("parameters", [])
        if len(parameters) != 1:
            self.fail(method, "validator subset requires exactly one model parameter")
        parameter = parameters[0]
        if parameter.get("fields", {}).get("extension") or parameter.get("fields", {}).get("varArg"):
            self.fail(parameter, "extension or vararg validator parameter is unsupported")
        parameter_type = parameter.get("type")
        if not self.known_type(parameter_type) or not self.assignable(parameter_type, EOBJECT):
            self.fail(parameter, f"unknown or non-model parameter type {parameter_type}")
        parameter_name = parameter.get("fields", {}).get("name")
        if not isinstance(parameter_name, str):
            self.fail(parameter, "missing parameter name")
        self.bindings = {parameter["id"]: (parameter_name, parameter_type)}
        self.dependencies = set()
        body_node = fields.get("expression")
        if body_node is None or body_node.get("type") != "void":
            self.fail(method, "validator must have a complete void body")
        self.check_annotations(method)
        body = self.statements(body_node)
        self.all_dependencies.update(self.dependencies)
        return {"id": self.rule, "name": name, "declaring_type": owner,
                "parameter": {"name": parameter_name, "type": parameter_type},
                "source": {**self.source, "span": method.get("span")},
                "dependencies": sorted(self.dependencies), "body": body}

    def check_annotations(self, method):
        # Explicit @Check modes need a scheduling contract before native use.
        info = method["fields"].get("annotationInfo")
        annotations = info.get("fields", {}).get("annotations", []) if isinstance(info, dict) else []
        if len(annotations) != 1 or annotations[0].get("kind") != "XAnnotation":
            self.fail(method, "expected one plain @Check annotation")
        fields = annotations[0]["fields"]
        if fields.get("annotationType") != {"$ref": "org.eclipse.xtext.validation.Check"} or fields.get("value") is not None or fields.get("elementValuePairs"):
            self.fail(method, "nondefault or non-Check annotations are outside the supported subset")

    def compile(self):
        methods = self.document.get("methods")
        if not isinstance(methods, list) or not methods:
            raise TranslationError("No selected Xtend methods")
        rules = [self.method(method) for method in methods]
        if len({rule["id"] for rule in rules}) != len(rules):
            raise TranslationError("Duplicate validator method identity")
        return {
            "schema_version": 1,
            "source_format": "typed-validation-rules",
            "source": self.document.get("provenance", {}),
            "types": self.types,
            "dependencies": [{"symbol": symbol, "receiver_type": GETTERS[symbol][0], "result_type": GETTERS[symbol][1]} for symbol in sorted(self.all_dependencies)],
            "rules": rules,
        }


def translate(document):
    if any(not m["fields"].get("annotationInfo", {}).get("fields", {}).get("annotations", []) for m in document.get("methods", [])):
        from translate_pilot_type_family import FamilyCompiler
        return FamilyCompiler(document).compile()
    return Compiler(document).compile()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, default=DEFAULT_INPUT)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    document = json.loads(args.input.read_text(encoding="utf-8"))
    # Enforce the exporter's complete-source/reference/coverage contract at the
    # command boundary. The reusable compiler above has no selected-rule names.
    from export_pilot_xtend import validate_structure
    validate_structure(document)
    result = translate(document)
    result["provenance"] = {"input_sha256": digest(args.input), "translator_sha256": digest(Path(__file__))}
    if result.get("inlined_helpers"):
        result["provenance"]["family_translator_sha256"] = digest(ROOT / "tools/translate_pilot_type_family.py")
    encoded = encode(result)
    if args.check:
        if not args.out.is_file() or args.out.read_bytes() != encoded:
            raise TranslationError(f"Stale typed validation artifact: {args.out}")
        print(f"Verified {len(result['rules'])} typed validation rules, {len(result['dependencies'])} explicit getter dependencies")
    else:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_bytes(encoded)
        print(f"Translated {len(result['rules'])} validation rules, {len(result['dependencies'])} explicit getter dependencies")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
