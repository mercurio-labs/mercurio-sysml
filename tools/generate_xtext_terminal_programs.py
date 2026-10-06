"""Compile every pinned Xtext terminal into a shared native recognition program.

Recognition is deliberately separate from generated ANTLR DFA token arbitration,
parser-rule hidden-token policy, and value conversion. Unknown constructs fail.
"""
import argparse
import hashlib
import json
from pathlib import Path

from export_pilot_grammar_structure import validate_structure

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"


def units(text):
    encoded = text.encode("utf-16-le")
    return [encoded[i] | encoded[i + 1] << 8 for i in range(0, len(encoded), 2)]


def nodes(value):
    if isinstance(value, dict):
        if "kind" in value:
            yield value
        for child in value.values():
            yield from nodes(child)
    elif isinstance(value, list):
        for child in value:
            yield from nodes(child)


def qualify_committed_decisions(programs, terminals):
    """Prove single-character entry decisions and greedy exits for this pin.

    Overlapping FIRST/FOLLOW or nullable alternatives require imported lookahead
    decisions; never silently assign a handwritten branch order to new grammar.
    """
    all_units = (1 << 65536) - 1
    first, nullable = [], []
    for row in programs:
        op = row['op']
        if op == 'literal': bits, empty = 1 << row['units'][0], False
        elif op == 'range': bits, empty = ((1 << (row['right']-row['left']+1))-1) << row['left'], False
        elif op == 'not_characters': bits, empty = all_units ^ sum(1 << u for u in row['units']), False
        elif op == 'until': bits, empty = all_units, False
        elif op == 'call': bits, empty = first[row['child']], nullable[row['child']]
        elif op == 'sequence':
            bits, empty = 0, True
            for child in row['children']:
                if empty: bits |= first[child]
                empty = empty and nullable[child]
        else:
            bits, empty = 0, False
            for child in row['children']:
                if nullable[child] or bits & first[child]:
                    raise ValueError('Terminal choice requires imported lookahead: '+row['source_id'])
                bits |= first[child]
        if row['cardinality'] in ('?', '*', '+') and empty:
            raise ValueError('Nullable terminal decision body: '+row['source_id'])
        row['body_first_ranges'] = []
        start = None
        for unit in range(65537):
            present = unit < 65536 and bool(bits & (1 << unit))
            if present and start is None: start = unit
            if not present and start is not None:
                row['body_first_ranges'].append([start,unit-1]); start = None
        first.append(bits)
        nullable.append(empty or row['cardinality'] in ('?', '*'))
    if any(nullable[row['program']] for row in terminals):
        raise ValueError('Nullable terminal root cannot advance source scanning')
    pending = [(row['program'],0) for row in terminals]
    seen = {}
    while pending:
        index, follow = pending.pop()
        if index in seen and follow | seen[index] == seen[index]: continue
        follow |= seen.get(index,0); seen[index] = follow
        row = programs[index]
        if row['cardinality'] in ('?', '*', '+') and first[index] & follow:
            raise ValueError('Terminal exit requires imported lookahead: '+row['source_id'])
        body_follow = follow | (first[index] if row['cardinality'] in ('*','+') else 0)
        if row['op'] == 'call': pending.append((row['child'],body_follow))
        elif row['op'] == 'choice': pending.extend((child,body_follow) for child in row['children'])
        elif row['op'] == 'sequence':
            for child in reversed(row['children']):
                pending.append((child,body_follow))
                body_follow = first[child] | (body_follow if nullable[child] else 0)


def build(document):
    validate_structure(document)
    grammars = {g["name"]: g for g in document["grammars"]}
    rules = {r["id"]: r for g in grammars.values() for r in g["rules"]}
    programs, indices, active = [], {}, set()

    def no_modifiers(node):
        fields = node["fields"]
        if fields.get("cardinality") or fields.get("predicated") or fields.get("firstSetPredicated"):
            raise ValueError("Unexpected character operand modifier: " + node["id"])

    def char_set(node):
        no_modifiers(node)
        if node["kind"] == "Keyword":
            value = units(node["fields"]["value"])
            if len(value) != 1:
                raise ValueError("Complement requires UTF-16 character operands")
            return value
        if node["kind"] == "Alternatives":
            return [u for child in node["fields"]["elements"] for u in char_set(child)]
        raise ValueError("Unsupported character complement: " + node["kind"])

    def compile_node(node):
        identity = node["id"]
        if identity in indices:
            return indices[identity]
        if identity in active:
            raise ValueError("Recursive terminal program: " + identity)
        active.add(identity)
        fields, kind = node["fields"], node["kind"]
        cardinality = fields.get("cardinality") or "one"
        if cardinality not in ("one", "?", "*", "+"):
            raise ValueError("Unsupported cardinality: " + cardinality)
        if fields.get("predicated") or fields.get("firstSetPredicated") or fields.get("guardCondition"):
            raise ValueError("Terminal predicates need an explicit execution contract: " + identity)
        row = {"source_id": identity, "cardinality": cardinality}
        if kind == "Keyword":
            literal = units(fields["value"])
            if not literal:
                raise ValueError("Empty terminal keyword")
            row.update(op="literal", units=literal)
        elif kind in ("Group", "Alternatives"):
            if not fields["elements"]:
                raise ValueError("Empty terminal group")
            row.update(op="sequence" if kind == "Group" else "choice", children=[compile_node(c) for c in fields["elements"]])
        elif kind == "CharacterRange":
            no_modifiers(fields["left"])
            no_modifiers(fields["right"])
            if fields["left"]["kind"] != "Keyword" or fields["right"]["kind"] != "Keyword":
                raise ValueError("Non-literal character range")
            left, right = units(fields["left"]["fields"]["value"]), units(fields["right"]["fields"]["value"])
            if len(left) != 1 or len(right) != 1 or left[0] > right[0]:
                raise ValueError("Invalid UTF-16 character range")
            row.update(op="range", left=left[0], right=right[0])
        elif kind == "NegatedToken":
            row.update(op="not_characters", units=sorted(set(char_set(fields["terminal"]))))
        elif kind == "UntilToken":
            terminal = fields["terminal"]
            no_modifiers(terminal)
            if terminal["kind"] != "Keyword" or not terminal["fields"]["value"]:
                raise ValueError("UntilToken currently requires a nonempty literal delimiter")
            row.update(op="until", units=units(terminal["fields"]["value"]))
        elif kind == "RuleCall":
            target = fields["rule"]["$ref"]
            if fields.get("arguments") or fields.get("explicitlyCalled") or rules[target]["kind"] != "TerminalRule":
                raise ValueError("Unsupported terminal rule call: " + identity)
            row.update(op="call", child=compile_node(rules[target]["fields"]["alternatives"]))
        else:
            raise ValueError("Unsupported terminal node: " + kind)
        active.remove(identity)
        index = len(programs)
        indices[identity] = index
        programs.append(row)
        return index

    terminals = []
    for rule in rules.values():
        if rule["kind"] != "TerminalRule":
            continue
        if rule["fields"]["annotations"]:
            raise ValueError("Terminal rule annotations require explicit classification")
        terminals.append({"id": rule["id"], "name": rule["fields"]["name"],
                          "fragment": rule["fields"]["fragment"], "program": compile_node(rule["fields"]["alternatives"]),
                          "source_nodes": [node["id"] for node in nodes(rule["fields"]["alternatives"])]})
    if len({row["name"] for row in terminals}) != len(terminals):
        raise ValueError("Terminal overrides require language-specific rule dispatch")

    qualify_committed_decisions(programs, terminals)

    def effective_rules(name, stack=()):
        if name in stack:
            raise ValueError("Cyclic inherited grammar")
        grammar = grammars[name]
        inherited = grammar["fields"]["usedGrammars"]
        if len(inherited) > 1:
            raise ValueError("Multiple grammar inheritance requires an explicit precedence contract")
        result = effective_rules(inherited[0]["$ref"], stack + (name,)) if inherited else {}
        result.update({r["fields"]["name"]: r for r in grammar["rules"]})
        return result

    def hidden(name):
        grammar = grammars[name]
        if grammar["fields"]["definesHiddenTokens"]:
            return [ref["$ref"] for ref in grammar["fields"]["hiddenTokens"]]
        inherited = grammar["fields"]["usedGrammars"]
        return hidden(inherited[0]["$ref"]) if inherited else []

    languages = {}
    for name in grammars:
        effective = effective_rules(name)
        keywords = sorted({n["fields"]["value"] for r in effective.values() if r["kind"] != "TerminalRule"
                           for n in nodes(r["fields"]["alternatives"]) if n["kind"] == "Keyword"})
        languages[name] = {"keywords": keywords, "terminals": [r["id"] for r in effective.values() if r["kind"] == "TerminalRule"],
                           "inherited_hidden_tokens": hidden(name)}
    # The shared native token stream has no rule-entry callback. Any rule-level
    # override or diverging grammar policy requires a new consumer, not fallback.
    policies = {tuple(row["inherited_hidden_tokens"]) for row in languages.values()}
    if len(policies) != 1:
        raise ValueError("Distinct hidden-token policies require language-specific lexing")
    for rule in rules.values():
        if rule["fields"].get("definesHiddenTokens"):
            raise ValueError("Rule hidden-token override requires a parser-context consumer")
    policy = next(iter(policies))
    for identity in policy:
        if identity not in rules or rules[identity]["kind"] != "TerminalRule":
            raise ValueError("Hidden-token reference is not a terminal")
    if {rules[identity]["fields"]["name"] for identity in policy} != {"WS", "ML_NOTE", "SL_NOTE"}:
        raise ValueError("Unassessed hidden-token adapter policy")
    occurrences = [{"id": n["id"], "value": n["fields"]["value"], "terminal_context": r["kind"] == "TerminalRule"}
                   for r in rules.values() for n in nodes(r["fields"]["alternatives"]) if n["kind"] == "Keyword"]
    if len(occurrences) != document["coverage"]["node_kinds"]["Keyword"]:
        raise ValueError("Incomplete keyword inventory")
    return {"schema": "dev.mercurio.xtext-terminal-program.v1", "offset_units": "utf-16-code-units",
            "token_arbitration": "separate lexer-decision.extract.json; this artifact only recognizes selected terminal bodies",
            "programs": programs, "terminals": terminals, "languages": languages, "keyword_occurrences": occurrences}


def rust_string(value):
    return json.dumps(value, ensure_ascii=False)


def render(contract):
    output = ["// Generated by tools/generate_xtext_terminal_programs.py; do not edit.",
              "use super::{Cardinality, Node, Operation, Terminal};", "pub(super) const PROGRAM: &[Node] = &["]
    cardinality = {"one": "One", "?": "Optional", "*": "ZeroOrMore", "+": "OneOrMore"}
    for row in contract["programs"]:
        op = row["op"]
        if op in ("literal", "not_characters", "until"):
            value = {"literal": "Literal", "not_characters": "NotCharacters", "until": "Until"}[op]
            expression = f"Operation::{value}(&{row['units']})"
        elif op == "range":
            expression = f"Operation::Range({row['left']}, {row['right']})"
        elif op in ("sequence", "choice"):
            expression = f"Operation::{op.capitalize()}(&{row['children']})"
        else:
            expression = f"Operation::Call({row['child']})"
        ranges = ", ".join(f"({low}, {high})" for low, high in row["body_first_ranges"])
        output.append(f"    Node {{ op: {expression}, cardinality: Cardinality::{cardinality[row['cardinality']]}, first: &[{ranges}] }},")
    output += ["];", "pub(super) const TERMINALS: &[Terminal] = &["]
    for row in contract["terminals"]:
        output.append(f"    Terminal {{ name: {rust_string(row['name'])}, id: {rust_string(row['id'])}, program: {row['program']} }},")
    output.append("];")
    for language, row in contract["languages"].items():
        name = language.rsplit(".", 1)[1].upper()
        if name == "KERMLEXPRESSIONS":
            output += ["// Retain the inherited grammar inventory for source-coverage controls;",
                       "// production frontends consume the effective KerML/SysML tables below.",
                       "#[allow(dead_code)]"]
        output.append(f"pub(super) const {name}_KEYWORDS: &[&str] = &[")
        output.extend("    " + rust_string(keyword) + "," for keyword in row["keywords"])
        output.append("];")
    policy = next(iter(contract["languages"].values()))["inherited_hidden_tokens"]
    output.append("pub(super) const HIDDEN_TOKENS: &[&str] = &[")
    output.extend("    " + rust_string(identity) + "," for identity in policy)
    output.append("];")
    return "\n".join(output) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--grammar", type=Path, default=PROFILE / "grammar.structure.extract.json")
    parser.add_argument("--out", type=Path, default=PROFILE / "xtext-terminal-programs.json")
    parser.add_argument("--rust-out", type=Path, default=ROOT / "crates/mercurio-sysml/src/xtext_terminal_generated.rs")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    contract = build(json.loads(args.grammar.read_text(encoding="utf-8")))
    contract["provenance"] = {"grammar_sha256": hashlib.sha256(args.grammar.read_bytes()).hexdigest(),
                              "generator_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    for path, content in [(args.out, json.dumps(contract, indent=2, ensure_ascii=False) + "\n"), (args.rust_out, render(contract))]:
        if args.check:
            if path.read_text(encoding="utf-8") != content:
                raise ValueError("Stale terminal contract: " + str(path))
        else:
            path.write_text(content, encoding="utf-8", newline="\n")
    print(f"Verified {len(contract['terminals'])} terminal programs and {len(contract['keyword_occurrences'])} keyword occurrences")


if __name__ == "__main__":
    main()
