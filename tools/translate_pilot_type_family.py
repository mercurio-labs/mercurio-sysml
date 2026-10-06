"""Bounded compile-time specialization of resolved, nonrecursive Xtend helpers.

Only side-effect-free literal/local helper arguments and a terminal unused Boolean
return are admitted. Predicate bodies are translated node by node, never matched
against method source text. Native services are explicit dependencies.
"""
from copy import deepcopy
from translate_pilot_validators import Compiler, GETTERS, MODEL, BOOL, STRING, NULL, EOBJECT, FEATURE

INT = "int"
LIST = "org.eclipse.emf.common.util.EList<org.omg.sysml.lang.sysml.Type>"
ALL_TYPES = "org.omg.sysml.util.FeatureUtil.getAllTypesOf(org.omg.sysml.lang.sysml.Feature)"
INSTANCE = "java.lang.Class.isInstance(java.lang.Object)"
LENGTH = "org.eclipse.xtext.xbase.lib.ArrayExtensions.length(java.lang.Object[])"
EXISTS = "org.eclipse.xtext.xbase.lib.IterableExtensions.exists(java.lang.Iterable,org.eclipse.xtext.xbase.lib.Functions$Function1)"
INT_EQ = "org.eclipse.xtext.xbase.lib.IntegerExtensions.operator_equals(int,int)"
SERVICES = {ALL_TYPES: (MODEL + "Feature", LIST), INSTANCE: (EOBJECT, BOOL)}


class FamilyCompiler(Compiler):
    def __init__(self, document):
        super().__init__(document)
        self.substitutions = {}
        self.inlining = False
        self.helpers = {}
        self.inlined = {}
        for method in document["methods"]:
            annotations = method["fields"].get("annotationInfo", {}).get("fields", {}).get("annotations", [])
            if not annotations:
                candidates = [key for key, symbol in self.symbols.items()
                              if symbol.get("kind") == "JvmOperation" and symbol.get("declaring_type") == method["declaring_type"]
                              and symbol.get("simple_name") == method["name"]]
                if len(candidates) != 1:
                    self.fail(method, "helper must have one unambiguous resolved signature")
                self.helpers[candidates[0]] = method

    def known_type(self, name):
        return name in {INT, LIST} or (isinstance(name, str) and name.startswith("java.lang.Class<") and name.endswith(">")) or super().known_type(name)

    def symbol(self, node):
        identity = node.get("fields", {}).get("feature", {}).get("$ref")
        if identity in self.substitutions:
            return identity, None
        return super().symbol(node)

    def expression(self, node):
        if not isinstance(node, dict):
            return super().expression(node)
        fields = node.get("fields", {})
        kind = node.get("kind")
        self.check_fields(node)
        if fields.get("nullSafe") or fields.get("typeArguments") or fields.get("invalidFeatureIssueCode"):
            self.fail(node, "unsupported null-safe, generic or unresolved family expression")
        if kind == "XNumberLiteral":
            if set(fields) != {"value"} or node.get("type") != INT or not fields["value"].isascii() or not fields["value"].isdigit():
                self.fail(node, "only decimal int literals are supported")
            value = int(fields["value"])
            if value > 2147483647:
                self.fail(node, "integer literal exceeds Java int")
            return self.typed(node, "int", INT, value=value)
        if kind == "XBlockExpression":
            items = fields.get("expressions", [])
            if set(fields) != {"expressions"} or len(items) != 1:
                self.fail(node, "expression block requires one pure expression")
            return self.expression(items[0])
        if kind == "XBinaryOperation" and fields.get("feature") == {"$ref": INT_EQ}:
            symbol = self.symbols[INT_EQ]
            if not symbol.get("static") or symbol.get("parameter_types") != [INT, INT] or fields.get("reassignFirstArgument"):
                self.fail(node, "invalid integer equality signature")
            left, right = self.expression(fields["leftOperand"]), self.expression(fields["rightOperand"])
            self.require_type(left, INT, node); self.require_type(right, INT, node)
            return self.typed(node, "int_eq", BOOL, left=left, right=right)
        if kind not in {"XFeatureCall", "XMemberFeatureCall"}:
            return super().expression(node)
        identity, symbol = self.symbol(node)
        args = self.expressions(node)
        if identity in self.substitutions:
            if args:
                self.fail(node, "substituted value cannot be called")
            return deepcopy(self.substitutions[identity])
        if symbol and symbol.get("kind") == "JvmGenericType" and not args:
            if not self.assignable(identity, EOBJECT):
                self.fail(node, "class literal must identify a model class")
            return self.typed(node, "class_literal", node["type"], class_name=identity)
        if identity == ALL_TYPES:
            if len(args) != 1 or not symbol.get("static") or symbol.get("parameter_types") != [MODEL + "Feature"] or symbol.get("type") != LIST:
                self.fail(node, "invalid native type collection signature")
            self.static_receiver(node, symbol)
            receiver = self.expression(args[0]); self.require_type(receiver, MODEL + "Feature", node)
            self.dependencies.add(identity)
            return self.typed(node, "get", LIST, symbol=identity, receiver=receiver)
        if identity == LENGTH:
            if args or not symbol.get("static") or symbol.get("parameter_types") != ["java.lang.Object[]"] or symbol.get("type") != INT:
                self.fail(node, "unsupported length overload")
            receiver = self.expression(fields["memberCallTarget"]); self.require_type(receiver, LIST, node)
            # Xbase's EList-to-array coercion preserves cardinality. No array
            # identity or mutation escapes this sole supported length use.
            return self.typed(node, "length", INT, operand=receiver)
        if identity == INSTANCE:
            if len(args) != 1 or symbol.get("static") or symbol.get("parameter_types") != ["java.lang.Object"]:
                self.fail(node, "unsupported instance check signature")
            required = self.expression(fields["memberCallTarget"])
            if required["kind"] != "class_literal":
                self.fail(node, "instance checks require a specialized class literal")
            value = self.expression(args[0]); self.require_type(value, EOBJECT, node)
            self.dependencies.add(identity)
            return self.typed(node, "is_instance", BOOL, symbol=identity, operand=value, class_name=required["class_name"])
        if identity == EXISTS:
            if len(args) != 1 or not symbol.get("static") or symbol.get("type") != BOOL or symbol.get("kind") != "JvmOperation" or symbol.get("parameter_types") != ["java.lang.Iterable<T>", "org.eclipse.xtext.xbase.lib.Functions$Function1<? extends java.lang.Object & super T, java.lang.Boolean>"]:
                self.fail(node, "invalid exists signature")
            collection = self.expression(fields["memberCallTarget"]); self.require_type(collection, LIST, node)
            closure = args[0]; cf = closure.get("fields", {})
            if closure.get("kind") != "XClosure" or set(cf) != {"declaredFormalParameters", "explicitSyntax", "expression"} or not cf["explicitSyntax"] or len(cf["declaredFormalParameters"]) != 1:
                self.fail(closure, "requires an explicit single-parameter closure")
            parameter = cf["declaredFormalParameters"][0]; pf = parameter["fields"]
            if set(pf) != {"annotations", "extension", "name", "parameterType"} or pf["annotations"] or pf["extension"] or pf["parameterType"] is not None:
                self.fail(parameter, "unsupported closure parameter")
            if closure.get("type") != "(" + MODEL + "Type)=>boolean":
                self.fail(closure, "closure must resolve Type to boolean")
            name = pf["name"]
            if any(name == binding[0] for binding in self.bindings.values()):
                self.fail(parameter, "shadowed closure parameter")
            previous = self.bindings.copy()
            self.bindings[parameter["id"]] = (name, MODEL + "Type")
            try:
                predicate = self.expression(cf["expression"]); self.require_type(predicate, BOOL, closure)
            finally:
                self.bindings = previous
            return self.typed(node, "exists", BOOL, collection=collection, parameter={"name": name, "type": MODEL + "Type"}, predicate=predicate)
        if symbol and "ecore_feature" in symbol:
            if args or symbol.get("parameter_types") != [] or symbol.get("declaring_type") != MODEL + "SysMLPackage" or symbol.get("type") != "org.eclipse.emf.ecore.EReference":
                self.fail(node, "invalid resolved Ecore feature token")
            receiver = fields.get("memberCallTarget", {})
            if receiver.get("fields", {}).get("feature") != {"$ref": MODEL + "SysMLPackage.eINSTANCE"} or self.expressions(receiver):
                self.fail(node, "feature token must use actual Ecore singleton")
            self.static_receiver(receiver, self.symbols[MODEL + "SysMLPackage.eINSTANCE"])
            return self.typed(node, "feature_literal", "org.eclipse.emf.ecore.EReference", identity=symbol["ecore_feature"])
        return super().expression(node)

    def statements(self, node, scoped=True):
        if node: self.check_fields(node)
        if node and node.get("kind") in {"XFeatureCall", "XMemberFeatureCall"}:
            identity, symbol = self.symbol(node)
            if identity in self.helpers:
                return self.inline_helper(node, identity, symbol)
        return super().statements(node, scoped)

    def inline_helper(self, node, identity, symbol):
        if node["fields"].get("invalidFeatureIssueCode") or self.inlining or node["kind"] != "XFeatureCall" or node["fields"].get("typeArguments") or symbol.get("static") or symbol.get("type") != BOOL:
            self.fail(node, "only nonrecursive implicit-receiver Boolean helper calls are supported")
        helper = self.helpers[identity]; fields = helper["fields"]
        if fields.get("typeParameters") or fields.get("exceptions") or fields.get("createExtensionInfo"):
            self.fail(helper, "unsupported helper declaration")
        params, args = fields["parameters"], self.expressions(node)
        if len(params) != len(args):
            self.fail(node, "helper arity mismatch")
        if any(p["fields"].get("extension") or p["fields"].get("varArg") for p in params):
            self.fail(helper, "unsupported helper parameter modifiers")
        values = [self.expression(arg) for arg in args]
        for parameter, value in zip(params, values):
            if value["kind"] not in {"local", "string", "class_literal", "feature_literal"}:
                self.fail(node, "specialization only admits pure local or literal arguments")
            if value["kind"] == "class_literal":
                if parameter["type"] != "java.lang.Class<?>":
                    self.fail(node, "class argument requires wildcard Class parameter")
            else:
                self.require_type(value, parameter["type"], node)
        body = fields["expression"]
        if body["kind"] != "XBlockExpression":
            self.fail(helper, "helper requires a complete block")
        items = body["fields"]["expressions"]
        if not items or items[-1]["kind"] != "XReturnExpression":
            self.fail(helper, "helper requires a terminal explicit return")
        old_bindings, old_substitutions = self.bindings.copy(), self.substitutions.copy()
        self.substitutions.update({p["id"]: v for p, v in zip(params, values)})
        self.inlining = True
        try:
            result = []
            for item in items[:-1]:
                result.extend(self.statements(item, scoped=False))
            terminal = items[-1]
            if set(terminal["fields"]) != {"expression"}:
                self.fail(terminal, "unsupported terminal return")
            value = self.expression(terminal["fields"]["expression"]); self.require_type(value, BOOL, terminal)
            result.append({"op": "discard", "value": value})
        finally:
            self.bindings, self.substitutions, self.inlining = old_bindings, old_substitutions, False
        self.inlined[helper["id"]] = {"id": helper["id"], "symbol": identity, "source": helper["source"], "span": helper["span"]}
        return [{"op": "block", "body": result}]

    def diagnostic(self, node):
        _, symbol = self.symbol(node)
        implicit = [STRING, FEATURE, STRING, STRING + "[]"]
        if symbol and symbol.get("parameter_types") == implicit:
            if not self.inlining or symbol.get("declaring_type") != "org.eclipse.xtext.validation.AbstractDeclarativeValidator" or not symbol.get("varargs") or symbol.get("type") != "void" or symbol.get("static") or symbol.get("simple_name") not in {"error", "warning"}:
                self.fail(node, "unsupported implicit diagnostic")
            if node["fields"].get("memberCallTarget") or node["fields"].get("typeArguments"):
                self.fail(node, "invalid implicit diagnostic receiver")
            values = [self.expression(arg) for arg in self.expressions(node)]
            if len(values) < 3:
                self.fail(node, "implicit diagnostic arity")
            for value, expected in zip(values[:3], implicit): self.require_type(value, expected, node)
            for value in values[3:]: self.require_type(value, STRING, node)
            if any(value["type"] == NULL for value in [values[0], values[2], *values[3:]]):
                self.fail(node, "null diagnostic text")
            return {"op": "diagnostic", "severity": symbol["simple_name"], "message": values[0], "subject": self.subject,
                    "feature": values[1], "code": values[2], "index": None, "data": values[3:]}
        return super().diagnostic(node)

    def method(self, method):
        parameter = method["fields"]["parameters"][0]
        self.subject = {"kind": "local", "type": parameter["type"], "name": parameter["fields"]["name"]}
        body = method["fields"]["expression"]
        if body is not None and body.get("type") == BOOL:
            items = body["fields"].get("expressions", [])
            if len(items) != 1 or items[0]["fields"].get("feature", {}).get("$ref") not in self.helpers:
                self.fail(method, "nonvoid check must be one complete unused helper call")
            method = deepcopy(method)
            # @Check invocation ignores this inferred Boolean result. The complete
            # helper's terminal expression is still evaluated in the generated block.
            method["fields"]["expression"]["type"] = "void"
        return super().method(method)

    def compile(self):
        helpers = {method["id"] for method in self.helpers.values()}
        rules = [self.method(method) for method in self.document["methods"] if method["id"] not in helpers]
        if len({rule["id"] for rule in rules}) != len(rules): self.fail({}, "duplicate rule")
        if set(self.inlined) != helpers: self.fail({}, "unreferenced helper export")
        bindings = {**GETTERS, **SERVICES}
        return {"schema_version": 1, "source_format": "typed-validation-rules", "source": self.document.get("provenance", {}),
                "types": self.types, "dependencies": [{"symbol": s, "receiver_type": bindings[s][0], "result_type": bindings[s][1]} for s in sorted(self.all_dependencies)],
                "rules": rules, "inlined_helpers": list(self.inlined.values())}
