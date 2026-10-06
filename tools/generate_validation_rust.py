"""Generate direct Rust validation code from the bounded, typed validation IR.

The backend emits Rust control flow and expressions, never a reconstructed AST
or runtime evaluator. Semantic model reads and value/diagnostic contracts stay
in explicit handwritten Rust helpers. Java is unnecessary for this backend.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
PROFILE = "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
INPUT = PROFILE + "/validation-rules.extract.json"
OUTPUT = "crates/mercurio-sysml/src/validation_rules_generated.rs"
MANIFEST = PROFILE + "/validation-rust-generation.json"
BOOL, STRING, NULL = "boolean", "java.lang.String", "null"
INT = "int"
LIST = "org.eclipse.emf.common.util.EList<org.omg.sysml.lang.sysml.Type>"
INSTANCE = "java.lang.Class.isInstance(java.lang.Object)"
PRIMITIVES = {BOOL, STRING, NULL, INT, LIST}
JAVA_NAME = re.compile(r"[A-Za-z_$][A-Za-z0-9_$]*(?:\.[A-Za-z_$][A-Za-z0-9_$]*)+\Z")
JAVA_MEMBER = re.compile(r"[A-Za-z_$][A-Za-z0-9_$]*\Z")


class GenerationError(ValueError):
    pass


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rust_string(value: str) -> str:
    if not isinstance(value, str):
        raise GenerationError("Rust string literal requires a string")
    result = ['"']
    escapes = {'"': '\\"', '\\': '\\\\', '\n': '\\n', '\r': '\\r', '\t': '\\t', '\0': '\\0'}
    for character in value:
        code = ord(character)
        if 0xD800 <= code <= 0xDFFF:
            raise GenerationError("Unpaired surrogate cannot appear in a Rust UTF-8 string")
        if character in escapes:
            result.append(escapes[character])
        elif 0x20 <= code <= 0x7E:
            result.append(character)
        else:
            result.append("\\u{" + format(code, "x") + "}")
    result.append('"')
    return "".join(result)


def require_string(value, context):
    if not isinstance(value, str) or not value:
        raise GenerationError(f"Missing {context}")
    rust_string(value)
    return value


def exact_fields(value, fields, context):
    if not isinstance(value, dict) or set(value) != set(fields):
        actual = set(value) if isinstance(value, dict) else set()
        raise GenerationError(f"Malformed {context}; expected fields {sorted(fields)}, got {sorted(actual)}")


def unique_strings(values, context):
    if not isinstance(values, list):
        raise GenerationError(f"Expected list for {context}")
    for item in values:
        require_string(item, context)
    if len(set(values)) != len(values):
        raise GenerationError(f"Duplicate {context}")
    return set(values)


def all_nodes(value):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from all_nodes(child)
    elif isinstance(value, list):
        for child in value:
            yield from all_nodes(child)


class Backend:
    def __init__(self, document):
        if document.get("schema_version") != 1 or document.get("source_format") != "typed-validation-rules":
            raise GenerationError("Expected version 1 typed-validation-rules")
        allowed_document = {"schema_version", "source_format", "source", "types", "dependencies", "rules", "provenance", "inlined_helpers"}
        if set(document) - allowed_document:
            raise GenerationError("Unsupported typed rule document fields")
        self.document = document
        dependencies = document.get("dependencies")
        self.rules = document.get("rules")
        if not isinstance(dependencies, list) or not isinstance(self.rules, list) or not self.rules:
            raise GenerationError("Missing rule or dependency inventory")
        self.dependencies = {}
        self.types = document.get("types")
        if not isinstance(self.types, dict) or not self.types:
            raise GenerationError("Missing resolved type hierarchy")
        for name, info in self.types.items():
            if not isinstance(name, str) or not isinstance(info, dict) or set(info) != {"kind", "supertypes"} or info["kind"] not in {"JvmGenericType", "JvmEnumerationType", "JvmAnnotationType", "JvmPrimitiveType", "JvmVoid"} or not isinstance(info["supertypes"], list) or not all(isinstance(parent, str) for parent in info["supertypes"]):
                raise GenerationError(f"Malformed resolved type metadata {name!r}")
        self.known_types = set(PRIMITIVES) | set(self.types)
        self.enum_types = {name for name, info in self.types.items() if info["kind"] == "JvmEnumerationType"}
        for dependency in dependencies:
            exact_fields(dependency, {"symbol", "receiver_type", "result_type"}, "getter dependency")
            symbol = require_string(dependency["symbol"], "getter symbol")
            if symbol in self.dependencies:
                raise GenerationError(f"Duplicate getter dependency {symbol}")
            receiver = self.contract_type(dependency["receiver_type"], allow_primitive=False)
            result = self.contract_type(dependency["result_type"], allow_primitive=True)
            if result == NULL:
                raise GenerationError("Getter cannot declare null as its result type")
            self.dependencies[symbol] = dependency

        ids, names = [], []
        for rule in self.rules:
            exact_fields(rule, {"id", "name", "declaring_type", "parameter", "source", "dependencies", "body"}, "rule")
            if rule["id"] != require_string(rule["declaring_type"], "declaring type") + "#" + require_string(rule["name"], "rule name"):
                raise GenerationError("Rule identity differs from declaring type and name")
            ids.append(require_string(rule.get("id"), "rule identity"))
            names.append(require_string(rule.get("name"), "rule name"))
            parameter = rule.get("parameter")
            exact_fields(parameter, {"name", "type"}, "rule parameter")
            require_string(parameter["name"], "parameter name")
            type_ = self.contract_type(parameter["type"], allow_primitive=False)

        unique_strings(ids, "rule identity")
        unique_strings(names, "short rule name")
        aliases = ids + names
        if len(set(aliases)) != len(aliases):
            raise GenerationError("Ambiguous full/short rule dispatch names")
        self.lines = []
        self.locals = {}
        self.counter = 0
        self.current_rule = "<document>"
        self.used_dependencies = set()
        self.rule_dependencies = set()

    def contract_type(self, type_, allow_primitive):
        if type_ in PRIMITIVES:
            if allow_primitive:
                return type_
            raise GenerationError(f"Expected model object contract type, got {type_}")
        if not isinstance(type_, str) or JAVA_NAME.fullmatch(type_) is None or type_ not in self.types:
            raise GenerationError(f"Unsupported or unknown contract type {type_!r}")
        if not self.object_type(type_) and not (allow_primitive and type_ in self.enum_types):
            raise GenerationError(f"Unsupported model value contract type {type_!r}")
        return type_

    def assignable(self, actual, expected):
        pending, seen = [actual], set()
        while pending:
            current = pending.pop()
            if current == expected:
                return True
            if current in seen:
                continue
            seen.add(current)
            pending.extend(self.types.get(current, {}).get("supertypes", []))
        return False

    def fail(self, message):
        raise GenerationError(f"{self.current_rule}: {message}")

    def type_of(self, expression):
        if not isinstance(expression, dict):
            self.fail("Malformed expression")
        type_ = expression.get("type")
        if type_ not in self.known_types:
            self.fail(f"Unknown expression type {type_!r}")
        return type_

    def expect(self, actual, expected):
        if actual != expected:
            self.fail(f"Type mismatch: expected {expected}, got {actual}")

    def reference(self, type_, nullable=True):
        return (nullable and type_ == NULL) or self.object_type(type_) or type_ in self.enum_types

    def object_type(self, type_):
        return type_ not in PRIMITIVES and type_ not in self.enum_types and self.assignable(type_, "org.eclipse.emf.ecore.EObject")

    def expr(self, expression):
        type_ = self.type_of(expression)
        kind = expression.get("kind")
        common = {"kind", "type"}
        if kind == "local":
            exact_fields(expression, common | {"name"}, "local expression")
            name = expression["name"]
            if name not in self.locals:
                self.fail(f"Unbound local {name!r}")
            variable, declared = self.locals[name]
            self.expect(type_, declared)
            return f"{variable}.clone()"
        if kind == "int":
            exact_fields(expression, common | {"value"}, "integer expression")
            self.expect(type_, INT)
            if type(expression["value"]) is not int or not 0 <= expression["value"] <= 2147483647:
                self.fail("Only nonnegative Java int literals are supported")
            return f"Value::Int({expression['value']}_i32)"
        if kind == "feature_literal":
            exact_fields(expression, common | {"identity"}, "Ecore feature literal")
            self.expect(type_, "org.eclipse.emf.ecore.EReference")
            identity = require_string(expression["identity"], "feature token")
            if re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*::[A-Za-z_][A-Za-z_0-9]*", identity) is None:
                self.fail("Invalid Ecore feature token")
            return "Value::object(" + rust_string(identity) + ", " + rust_string(type_) + ")"
        if kind == "length":
            exact_fields(expression, common | {"operand"}, "collection length")
            self.expect(type_, INT); self.expect(self.type_of(expression["operand"]), LIST)
            return "Value::Int(i32::try_from((" + self.expr(expression["operand"]) + ").list()?.len()).map_err(|_| \"collection length exceeds Java int\")?)"
        if kind == "int_eq":
            exact_fields(expression, common | {"left", "right"}, "integer equality")
            self.expect(type_, BOOL)
            for key in ("left", "right"): self.expect(self.type_of(expression[key]), INT)
            return "Value::Bool((" + self.expr(expression["left"]) + ").integer()? == (" + self.expr(expression["right"]) + ").integer()?)"
        if kind == "is_instance":
            exact_fields(expression, common | {"symbol", "operand", "class_name"}, "model instance check")
            self.expect(type_, BOOL)
            symbol = expression["symbol"]
            if symbol != INSTANCE or symbol not in self.rule_dependencies or symbol not in self.dependencies:
                self.fail("Unbound instance check service")
            self.contract_type(expression["class_name"], allow_primitive=False)
            if not self.object_type(self.type_of(expression["operand"])):
                self.fail("Instance check requires a model object")
            self.used_dependencies.add(symbol)
            return "Value::Bool(instance_of(" + self.expr(expression["operand"]) + ", " + rust_string(expression["class_name"]) + ", model)?)"
        if kind == "exists":
            exact_fields(expression, common | {"collection", "parameter", "predicate"}, "exists expression")
            self.expect(type_, BOOL); self.expect(self.type_of(expression["collection"]), LIST)
            parameter = expression["parameter"]
            exact_fields(parameter, {"name", "type"}, "closure parameter")
            self.expect(parameter["type"], "org.omg.sysml.lang.sysml.Type")
            name = require_string(parameter["name"], "closure parameter")
            if name in self.locals: self.fail("Shadowed closure parameter")
            collection = self.expr(expression["collection"])
            variable = f"item_{self.counter}"; self.counter += 1
            previous = self.locals.copy()
            self.locals[name] = (variable, parameter["type"])
            predicate = self.boolean(expression["predicate"])
            self.locals = previous
            return "Value::Bool((|| -> Result<bool, String> { for " + variable + " in (" + collection + ").list()? { if " + predicate + " { return Ok(true); } } Ok(false) })()?)"
        if kind == "null":
            exact_fields(expression, common, "null expression")
            self.expect(type_, NULL)
            return "Value::Null"
        if kind == "bool":
            exact_fields(expression, common | {"value"}, "Boolean expression")
            self.expect(type_, BOOL)
            if type(expression["value"]) is not bool:
                self.fail("Boolean literal must contain a Boolean")
            return "Value::Bool(" + str(expression["value"]).lower() + ")"
        if kind == "string":
            exact_fields(expression, common | {"value"}, "string expression")
            self.expect(type_, STRING)
            return "Value::String(" + rust_string(expression["value"]) + ".into())"
        if kind == "enum":
            exact_fields(expression, common | {"name"}, "enum expression")
            if type_ not in self.enum_types or not isinstance(expression["name"], str) or JAVA_MEMBER.fullmatch(expression["name"]) is None:
                self.fail("Invalid typed enumeration literal")
            return "Value::Enum { type_name: " + rust_string(type_) + ".into(), name: " + rust_string(expression["name"]) + ".into() }"
        if kind == "get":
            exact_fields(expression, common | {"symbol", "receiver"}, "getter expression")
            symbol = expression["symbol"]
            if symbol not in self.dependencies or symbol not in self.rule_dependencies:
                self.fail(f"Undeclared getter dependency {symbol!r}")
            self.expect(type_, self.dependencies[symbol]["result_type"])
            receiver = self.expr(expression["receiver"])
            receiver_type = self.type_of(expression["receiver"])
            expected_receiver = self.dependencies[symbol]["receiver_type"]
            if not self.object_type(receiver_type) or not self.assignable(receiver_type, expected_receiver):
                self.fail(f"Incompatible getter receiver {receiver_type}; expected {expected_receiver}")
            self.used_dependencies.add(symbol)
            return "read_getter(" + receiver + ", " + rust_string(symbol) + ", model)?"
        if kind in {"not", "and", "or", "identity_eq", "identity_ne"}:
            return "Value::Bool(" + self.boolean(expression) + ")"
        self.fail(f"Unsupported expression kind {kind!r}")

    def boolean(self, expression):
        self.expect(self.type_of(expression), BOOL)
        kind = expression.get("kind")
        if kind == "not":
            exact_fields(expression, {"kind", "type", "operand"}, "not expression")
            return "!(" + self.boolean(expression["operand"]) + ")"
        if kind in {"and", "or"}:
            exact_fields(expression, {"kind", "type", "left", "right"}, "Boolean binary expression")
            left, right = self.boolean(expression["left"]), self.boolean(expression["right"])
            operator = "&&" if kind == "and" else "||"
            return "(" + left + ") " + operator + " (" + right + ")"
        if kind in {"identity_eq", "identity_ne"}:
            exact_fields(expression, {"kind", "type", "left", "right"}, "identity expression")
            left, right = self.expr(expression["left"]), self.expr(expression["right"])
            lt, rt = self.type_of(expression["left"]), self.type_of(expression["right"])
            if not self.reference(lt) or not self.reference(rt):
                self.fail("Identity comparison requires object, enum, or null operands")
            if lt != NULL and rt != NULL and ((lt in self.enum_types) != (rt in self.enum_types) or (lt in self.enum_types and lt != rt)):
                self.fail("Incompatible object/enum identity comparison")
            return ("!" if kind == "identity_ne" else "") + "identity_equal(" + left + ", " + right + ")?"
        if kind == "bool":
            # Validate the complete literal before emitting its primitive value.
            self.expr(expression)
            return str(expression["value"]).lower()
        return "(" + self.expr(expression) + ").boolean()?"

    def line(self, indentation, text):
        self.lines.append("    " * indentation + text)

    def body(self, statements, indentation):
        if not isinstance(statements, list):
            self.fail("Statement body must be a list")
        for statement in statements:
            if not isinstance(statement, dict):
                self.fail("Malformed statement")
            operation = statement.get("op")
            if operation == "let":
                exact_fields(statement, {"op", "name", "type", "value"}, "let statement")
                name = require_string(statement["name"], "local name")
                if name in self.locals:
                    self.fail(f"Duplicate or shadowing immutable local {name!r}")
                value = self.expr(statement["value"])
                self.expect(statement["type"], self.type_of(statement["value"]))
                variable = f"local_{self.counter}"
                self.counter += 1
                self.locals[name] = (variable, statement["type"])
                self.line(indentation, f"let {variable} = {value};")
            elif operation == "block":
                exact_fields(statement, {"op", "body"}, "block statement")
                self.line(indentation, "{")
                previous = self.locals.copy()
                self.body(statement["body"], indentation + 1)
                self.locals = previous
                self.line(indentation, "}")
            elif operation == "if":
                exact_fields(statement, {"op", "condition", "then", "else"}, "if statement")
                condition = self.boolean(statement["condition"])
                self.line(indentation, "if " + condition + " {")
                previous = self.locals.copy()
                self.body(statement["then"], indentation + 1)
                self.locals = previous.copy()
                self.line(indentation, "} else {")
                self.body(statement["else"], indentation + 1)
                self.locals = previous
                self.line(indentation, "}")
            elif operation == "discard":
                exact_fields(statement, {"op", "value"}, "discarded helper result")
                self.expect(self.type_of(statement["value"]), BOOL)
                self.line(indentation, "let _ = " + self.expr(statement["value"]) + ";")
            elif operation == "diagnostic":
                self.diagnostic(statement, indentation)
            else:
                self.fail(f"Unsupported statement operation {operation!r}")

    def diagnostic(self, statement, indentation):
        exact_fields(statement, {"op", "severity", "message", "code", "subject", "feature", "index", "data"}, "diagnostic statement")
        if statement["severity"] not in {"error", "warning"}:
            self.fail("Unsupported diagnostic severity")
        values = {}
        for key in ["message", "subject", "feature", "code"]:
            expression = statement[key]
            values[key] = self.expr(expression)
            type_ = self.type_of(expression)
            if key in {"message", "code"}:
                self.expect(type_, STRING)
                values[key] = "(" + values[key] + ").string()?"
            else:
                if type_ != NULL and not self.object_type(type_):
                    self.fail(f"Diagnostic {key} requires object or null")
                values[key] = "(" + values[key] + ").optional_identity()?"
        index = statement["index"]
        if index is not None and (type(index) is not int or not -(2**63) <= index < 2**63):
            self.fail("Diagnostic index must be null or a signed 64-bit integer")
        if not isinstance(statement["data"], list):
            self.fail("Diagnostic data must be a list")
        data = []
        for expression in statement["data"]:
            rendered = self.expr(expression)
            self.expect(self.type_of(expression), STRING)
            data.append("(" + rendered + ").string()?")
        # Preserve Xtend/Java diagnostic argument evaluation order, independently
        # of the field declaration order in handwritten RuleDiagnostic.
        for key in ["message", "subject", "feature", "code"]:
            variable = f"diagnostic_{self.counter}_{key}"
            self.line(indentation, "let " + variable + " = " + values[key] + ";")
            values[key] = variable
        self.counter += 1
        self.line(indentation, "diagnostics.push(RuleDiagnostic {")
        self.line(indentation + 1, "severity: " + rust_string(statement["severity"]) + ".into(),")
        for key in ["message", "subject", "feature", "code"]:
            self.line(indentation + 1, key + ": " + values[key] + ",")
        self.line(indentation + 1, "index: " + ("None" if index is None else f"Some({index}_i64)") + ",")
        self.line(indentation + 1, "data: vec![" + ", ".join(data) + "],")
        self.line(indentation, "});")

    def generate(self, input_sha256, generator_sha256):
        self.lines = [
            "// @generated by tools/generate_validation_rust.py; do not edit.",
            "// Regenerate: python -B tools/generate_validation_rust.py",
            "// Typed IR SHA-256: " + input_sha256,
            "// Generator SHA-256: " + generator_sha256,
            "// Semantic getters and shared value contracts are handwritten Rust.",
            "use super::{SemanticModel, Value, RuleDiagnostic, require_getters, read_getter, identity_equal};",
            "",
            "pub(super) fn evaluate(",
            "    name: &str,",
            "    subject: &str,",
            "    model: &impl SemanticModel,",
            ") -> Result<Vec<RuleDiagnostic>, String> {",
            "    match name {",
        ]
        if any(node.get("kind") == "is_instance" for node in all_nodes(self.document)):
            self.lines.insert(6, "use super::instance_of;")
        rule_metadata = []
        ordered = sorted(self.rules, key=lambda rule: rule["id"])
        for index, rule in enumerate(ordered):
            function = f"rule_{index:04d}"
            self.lines.append("        " + rust_string(rule["id"]) + " | " + rust_string(rule["name"]) + " => " + function + "(subject, model),")
            rule_metadata.append({"id": rule["id"], "name": rule["name"], "function_name": function, "entrypoint": "fn " + function + "(", "dependencies": sorted(unique_strings(rule.get("dependencies"), "per-rule getter dependency"))})
        self.lines.extend([
            '        _ => Err(format!("unknown generated validation rule {name}")),',
            "    }", "}", "",
        ])
        all_used = set()
        for rule, metadata in zip(ordered, rule_metadata):
            self.current_rule = rule["id"]
            self.rule_dependencies = set(metadata["dependencies"])
            if not self.rule_dependencies <= set(self.dependencies):
                self.fail("Unknown declared getter dependency")
            self.used_dependencies = set()
            self.counter = 0
            parameter = rule["parameter"]
            self.locals = {parameter["name"]: ("subject_value", parameter["type"])}
            self.lines.extend([
                "fn " + metadata["function_name"] + "(",
                "    subject: &str,",
                "    model: &impl SemanticModel,",
                ") -> Result<Vec<RuleDiagnostic>, String> {",
                "    require_getters(" + rust_string(rule["id"]) + ", &[",
                *["        " + rust_string(symbol) + "," for symbol in metadata["dependencies"]],
                "    ], model)?;",
                "    let subject_value = Value::object(subject, " + rust_string(parameter["type"]) + ");",
                "    let mut diagnostics = Vec::new();",
            ])
            self.body(rule.get("body"), 1)
            if self.used_dependencies != self.rule_dependencies:
                self.fail("Unused declared getter dependencies: " + repr(sorted(self.rule_dependencies - self.used_dependencies)))
            all_used.update(self.used_dependencies)
            self.lines.extend(["    Ok(diagnostics)", "}", ""])
        if all_used != set(self.dependencies):
            raise GenerationError("Global dependency inventory contains unused entries")
        self.lines.append("pub(super) fn type_family_rules() -> &'static [(&'static str, &'static str)] { &[")
        for rule in ordered:
            if "org.omg.sysml.util.FeatureUtil.getAllTypesOf(org.omg.sysml.lang.sysml.Feature)" in rule["dependencies"]:
                self.lines.append("    (" + rust_string(rule["parameter"]["type"]) + ", " + rust_string(rule["name"]) + "),")
        self.lines.extend(["] }", ""])
        return "\n".join(self.lines), rule_metadata


def generate(document, input_sha256, generator_sha256, output_path=OUTPUT):
    if any(not isinstance(value, str) or re.fullmatch(r"[0-9a-f]{64}", value) is None for value in [input_sha256, generator_sha256]):
        raise GenerationError("Generation provenance requires lowercase SHA-256 digests")
    code, rules = Backend(document).generate(input_sha256, generator_sha256)
    encoded = code.encode("utf-8")
    manifest = {"schema_version": 1, "backend": "rust-source", "input_sha256": input_sha256,
                "generator_sha256": generator_sha256, "output_path": output_path,
                "output_sha256": hashlib.sha256(encoded).hexdigest(), "rules": rules}
    if document.get("inlined_helpers"):
        manifest["inlined_helpers"] = document["inlined_helpers"]
    return encoded, (json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def repository_path(root, value):
    path = PurePosixPath(value)
    if not isinstance(value, str) or path.is_absolute() or ".." in path.parts or "\\" in value or ":" in value:
        raise GenerationError("Expected portable repository-relative generation path")
    result = root.joinpath(*path.parts).resolve()
    if not result.is_relative_to(root.resolve()):
        raise GenerationError("Generation path escapes the repository")
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--input", default=INPUT)
    parser.add_argument("--out", default=OUTPUT)
    parser.add_argument("--manifest", default=MANIFEST)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    source, output, manifest = [repository_path(args.root, value) for value in [args.input, args.out, args.manifest]]
    document = json.loads(source.read_text(encoding="utf-8"))
    code, metadata = generate(document, digest(source), digest(Path(__file__)), args.out)
    if args.check:
        if not output.is_file() or output.read_bytes() != code or not manifest.is_file() or manifest.read_bytes() != metadata:
            raise GenerationError("Stale generated Rust validator source or generation provenance")
        print(f"Verified direct Rust generation for {len(document['rules'])} selected validation rules")
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        manifest.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(code)
        manifest.write_bytes(metadata)
        print(f"Generated direct Rust source for {len(document['rules'])} selected validation rules")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
