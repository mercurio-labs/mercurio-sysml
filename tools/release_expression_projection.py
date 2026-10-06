"""Explicit expression-tree observations, never a graph-parity waiver.

Compare ordered value trees only for the supported forms below. Unsupported,
missing, ambiguous and cyclic references fail closed. Equality is scoped to the
tree, not typing, evaluation, multiplicity, or other implicit relationships.
"""


class Unassessed(ValueError):
    pass


BINARY = {
    "add": "+", "subtract": "-", "multiply": "*", "divide": "/",
    "power": "**", "equal": "==", "not_equal": "!=", "less": "<",
    "less_equal": "<=", "greater": ">", "greater_equal": ">=",
    "and": "and", "or": "or",
}


def literal(value):
    # bool is an int subclass in Python; never allow true == 1 to pass.
    family = ("boolean" if isinstance(value, bool) else "number"
              if isinstance(value, (int, float)) else "null" if value is None
              else "string" if isinstance(value, str) else None)
    if family is None:
        raise Unassessed("non-scalar literal")
    return ["literal", family, value]


def native_tree(expr, mapping):
    if not isinstance(expr, dict):
        raise Unassessed("missing native expression")
    kind = expr.get("kind")
    recurse = lambda child: native_tree(child, mapping)
    if kind == "literal":
        return literal(expr["value"])
    if kind == "path":
        segments = expr.get("segments", [])
        if len(segments) != 1 or not isinstance(segments[0], dict):
            raise Unassessed("native chained or unresolved path")
        target = segments[0].get("feature")
        if target not in mapping:
            raise Unassessed("native reference has no observed unique target pair")
        return ["reference", mapping[target]]
    if kind == "unary":
        operator = {"negate": "-", "not": "not"}.get(expr.get("op"))
        if operator is None:
            raise Unassessed("unknown native unary operator")
        return ["operator", operator, [recurse(expr["expr"])]]
    if kind == "binary":
        if expr.get("op") not in BINARY:
            raise Unassessed("unknown native binary operator")
        return ["operator", BINARY[expr["op"]], [recurse(expr["left"]), recurse(expr["right"])]]
    if kind == "operation":
        return ["operator", expr["operator"], [recurse(x) for x in expr["operands"]]]
    if kind == "call":
        function = expr["function"]
        if function not in mapping:
            raise Unassessed("native function has no observed unique target pair")
        return ["call", mapping[function], [recurse(x) for x in expr["args"]]]
    raise Unassessed("native expression form: " + str(kind))


def references(fields, key):
    row = fields.get("reference_sequences", {}).get(key)
    if not isinstance(row, dict) or not isinstance(row.get("targets"), list):
        raise Unassessed("missing ordered Pilot reference: " + key)
    values = row["targets"]
    if not all(isinstance(value, str) for value in values):
        raise Unassessed("Pilot reference outside exported closure: " + key)
    return values


def pilot_tree(target, elements, direct, active=frozenset()):
    if target in active:
        raise Unassessed("cyclic Pilot expression")
    element = elements.get(target)
    if element is None:
        raise Unassessed("Pilot expression outside snapshot")
    kind = element["kind"].split("::")[-1]
    fields = direct(element)
    recurse = lambda child: pilot_tree(child, elements, direct, active | {target})
    if kind in ("LiteralBoolean", "LiteralInteger", "LiteralRational", "LiteralString"):
        if "value" not in fields:
            raise Unassessed("missing Pilot literal value")
        return literal(fields["value"])
    if kind == "NullExpression":
        return literal(None)
    if kind == "FeatureReferenceExpression":
        referent = references(fields, "referent")
        if len(referent) != 1:
            raise Unassessed("non-singleton Pilot referent")
        return ["reference", referent[0]]
    if kind in ("OperatorExpression", "InvocationExpression"):
        args = [recurse(child) for child in references(fields, "argument")]
        if kind == "OperatorExpression":
            if "operator" not in fields:
                raise Unassessed("missing Pilot operator")
            return ["operator", fields["operator"], args]
        function = references(fields, "function")
        if len(function) != 1:
            raise Unassessed("non-singleton Pilot function")
        return ["call", function[0], args]
    raise Unassessed("Pilot expression form: " + kind)


def compare(expression, targets, mapping, elements, direct):
    reasons = []
    native = pilot = None
    try:
        native = native_tree(expression, mapping)
    except Unassessed as error:
        reasons.append(str(error))
    try:
        if len(targets) != 1:
            raise Unassessed("non-singleton initializer target")
        pilot = pilot_tree(targets[0], elements, direct)
    except Unassessed as error:
        reasons.append(str(error))
    return {"status": "unassessed" if reasons else "equal" if native == pilot else "different",
            "native_tree": native, "pilot_tree": pilot, "reasons": reasons}
