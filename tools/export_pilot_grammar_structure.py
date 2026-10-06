"""Export pinned raw Pilot Xtext grammars through Xtext's parser and linker.

Java is a development-time generator dependency only. Generated JSON is portable
and contains no clock values, absolute build paths, or JVM runtime requirement.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PILOT_REVISION = "692170b71867353b8f90341e61556f49a5beb0e5"
JAR_SHA256 = "b1ad9d64b1f0c75730facf25a5e2856bc9df2bb4bd39476df2fdf5ae68cd9350"
DEFAULT_OUTPUT = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/grammar.structure.extract.json"
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotGrammarExporter.java"
SOURCE_PATHS = (
    "org.omg.kerml.expressions.xtext/src/org/omg/kerml/expressions/xtext/KerMLExpressions.xtext",
    "org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerML.xtext",
    "org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext",
    "org.omg.sysml/model/SysML.ecore",
)
ECORE = "http://www.eclipse.org/emf/2002/Ecore"
SYSML = "https://www.omg.org/spec/SysML/20250201"


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def validate_structure(document: dict) -> None:
    """Reject truncation, dangling internal references, and coverage drift."""
    if document.get("schema_version") != 1 or document.get("source_format") != "raw-xtext":
        raise ValueError("Unsupported grammar structure schema")
    if document.get("parser") != "org.eclipse.xtext.XtextStandaloneSetup":
        raise ValueError("Expected the actual Xtext parser")
    if document.get("span_encoding") != "utf-16-code-units":
        raise ValueError("Source offsets must explicitly use Xtext UTF-16 units")
    identities: set[str] = set()
    references: list[str] = []
    counts: Counter[str] = Counter()
    inferred = 0

    def walk(value):
        nonlocal inferred
        if isinstance(value, list):
            for child in value:
                walk(child)
        elif isinstance(value, dict):
            if "$ref" in value:
                if set(value) != {"$ref"} or not isinstance(value["$ref"], str):
                    raise ValueError("Invalid reference shape")
                references.append(value["$ref"])
                return
            if "kind" in value:
                identity = value.get("id")
                if not isinstance(identity, str) or not identity or identity in identities:
                    raise ValueError(f"Missing or duplicate node identity: {identity}")
                identities.add(identity)
                counts[value["kind"]] += 1
                if not isinstance(value.get("fields"), dict):
                    raise ValueError(f"Missing structural fields: {identity}")
                span = value.get("span")
                if value.get("origin") == "xtext-inferred":
                    if span is not None or value["kind"] != "TypeRef":
                        raise ValueError(f"Unexpected inferred syntax node: {identity}")
                    inferred += 1
                elif value.get("origin") == "source":
                    if not isinstance(span, dict) or set(span) != {"offset", "length", "start_line", "end_line"}:
                        raise ValueError(f"Missing source span: {identity}")
                    if not all(isinstance(n, int) for n in span.values()) or span["offset"] < 0 or span["length"] < 0 or span["start_line"] < 1 or span["end_line"] < span["start_line"]:
                        raise ValueError(f"Invalid source span: {identity}")
                else:
                    raise ValueError(f"Missing node origin: {identity}")
                walk(value["fields"])
                if value["kind"] == "Grammar":
                    if "rules" in value["fields"] or not isinstance(value.get("rules"), list):
                        raise ValueError("Rules must appear once under grammars[].rules")
                    walk(value["rules"])
                return
            for child in value.values():
                walk(child)

    grammars = document.get("grammars")
    if not isinstance(grammars, list) or not grammars:
        raise ValueError("No grammars exported")
    for grammar in grammars:
        if grammar.get("kind") != "Grammar" or grammar.get("id") != grammar.get("name"):
            raise ValueError("Invalid grammar identity")
        walk(grammar)
    for reference in references:
        if reference not in identities and reference not in {ECORE, SYSML} and not reference.startswith((ECORE + "#//", SYSML + "#//")):
            raise ValueError(f"Dangling or unexported reference: {reference}")
    expected = {
        "node_kinds": dict(sorted(counts.items())),
        "nodes": sum(counts.values()),
        "references": len(references),
        "inferred_nodes_without_source_span": inferred,
        "unresolved_references": 0,
        "unsupported_nodes": 0,
    }
    if document.get("coverage") != expected:
        raise ValueError("Export coverage does not match the complete graph")



def validate_contexts(document: dict) -> None:
    """Check Xtext-resolved maps against the exported inheritance closure."""
    grammars = {g['id']: g for g in document['grammars']}
    contexts = document.get('resolution_contexts', {})
    if set(contexts) != set(grammars):
        raise ValueError('Missing grammar resolution contexts')
    rules = {r['id']: r for g in grammars.values() for r in g['rules']}

    def effective(identity, active):
        if identity in active or identity not in grammars:
            raise ValueError('Invalid grammar inheritance closure: ' + identity)
        result = {}
        for parent in grammars[identity]['fields'].get('usedGrammars', []):
            for name, target in effective(parent['$ref'], active | {identity}).items():
                if name in result and result[name] != target:
                    raise ValueError('Ambiguous inherited rule: ' + name)
                result[name] = target
        for rule in grammars[identity]['rules']:
            name = rule['fields']['name']
            result[name] = rule['id']
        return result

    def calls(value):
        if isinstance(value, dict):
            if value.get('kind') == 'RuleCall':
                yield value
            for child in value.values():
                yield from calls(child)
        elif isinstance(value, list):
            for child in value:
                yield from calls(child)

    def first_predicates(value):
        if isinstance(value, dict):
            if value.get('fields', {}).get('firstSetPredicated'):
                yield value['id']
            for child in value.values():
                yield from first_predicates(child)
        elif isinstance(value, list):
            for child in value:
                yield from first_predicates(child)

    for identity, context in contexts.items():
        expected = effective(identity, set())
        if context.get('effective_rules') != expected:
            raise ValueError('Incorrect effective rule selection: ' + identity)
        first_sets = context.get('first_set_predicates', {})
        required_first_sets = {node for target in expected.values() for node in first_predicates(rules[target])}
        if set(first_sets) != required_first_sets:
            raise ValueError('Incomplete contextual first-set predicates: ' + identity)
        for symbols in first_sets.values():
            if not isinstance(symbols, list) or not symbols:
                raise ValueError('Empty or invalid contextual first set')
            for symbol in symbols:
                if not isinstance(symbol, str) or not (symbol.startswith('keyword:') or symbol.startswith('call:')):
                    raise ValueError('Invalid contextual first-set symbol')
                if symbol.startswith('call:') and rules.get(expected.get(symbol[5:]), {}).get('kind') != 'TerminalRule':
                    raise ValueError('First set must reference an effective terminal')
        declared_calls = [call for target in expected.values() for call in calls(rules[target])]
        def node_index(value):
            if isinstance(value, dict):
                if 'id' in value and 'kind' in value:
                    yield value['id'], value
                for child in value.values(): yield from node_index(child)
            elif isinstance(value, list):
                for child in value: yield from node_index(child)
        visible_nodes = dict(item for target in expected.values() for item in node_index(rules[target]))
        for source, target in context.get('hoisted_predicates', {}).items():
            call, guard = visible_nodes.get(source), visible_nodes.get(target)
            if (call is None or call['kind'] != 'RuleCall' or call['fields'].get('predicated')
                    or call['fields'].get('firstSetPredicated') or guard is None
                    or not guard['fields'].get('predicated')):
                raise ValueError('Invalid contextual hoisted predicate identity')
        bindings = context.get('rule_calls', {})
        if set(bindings) != {call['id'] for call in declared_calls}:
            raise ValueError('Incomplete contextual rule calls: ' + identity)
        for local in grammars[identity]['rules']:
            for call in calls(local):
                if call['fields']['rule']['$ref'] != bindings.get(call['id']):
                    raise ValueError('Contaminated source rule call: ' + call['id'])
        for call in declared_calls:
            declaration = call['fields']['rule']['$ref']
            if declaration not in rules:
                raise ValueError('Unknown contextual rule declaration')
            name = rules[declaration]['fields']['name']
            target = declaration if call['fields'].get('explicitlyCalled') else expected.get(name)
            if bindings[call['id']] != target:
                raise ValueError('Incorrect contextual rule call: ' + call['id'])


def validate_sources(document: dict, root: Path) -> dict[str, str]:
    hashes = {}
    for source in [document["metamodel"], *(g["source"] for g in document["grammars"])]:
        relative = PurePosixPath(source["path"])
        if relative.is_absolute() or ".." in relative.parts or "\\" in source["path"] or ":" in source["path"]:
            raise ValueError("Source provenance must be a portable relative path")
        actual = digest(root.joinpath(*relative.parts))
        if source["sha256"] != actual:
            raise ValueError(f"Source changed during export: {relative}")
        hashes[str(relative)] = actual
    return dict(sorted(hashes.items()))


def encode(document: dict) -> bytes:
    return (json.dumps(document, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pilot-root", type=Path, default=ROOT.parent / "target/upstream/SysML-v2-Pilot-Implementation")
    parser.add_argument("--jar", type=Path, default=ROOT.parent / "target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar")
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--work-dir", type=Path, default=ROOT.parent / "target/support-2026-08/grammar-structure")
    parser.add_argument("--check", action="store_true", help="Regenerate and fail if the checked-in artifact differs")
    args = parser.parse_args(argv)
    revision = subprocess.check_output(["git", "-C", str(args.pilot_root), "rev-parse", "HEAD"], text=True).strip()
    if revision != PILOT_REVISION:
        raise ValueError(f"Wrong Pilot revision: {revision}")
    status = subprocess.check_output([
        "git", "-C", str(args.pilot_root), "status", "--porcelain", "--untracked-files=all"
    ], text=True).strip()
    if status:
        raise ValueError("Pinned Pilot checkout must be clean before exporting raw grammar sources")
    jar_hash = digest(args.jar)
    if jar_hash != JAR_SHA256:
        raise ValueError(f"Wrong pinned Xtext/Pilot JAR: {jar_hash}")
    helper_hash = digest(HELPER)
    input_hashes = {path: digest(args.pilot_root / path) for path in SOURCE_PATHS}
    classes = args.work_dir / "classes"
    classes.mkdir(parents=True, exist_ok=True)
    suffix = ".exe" if os.name == "nt" else ""
    subprocess.run([str(args.java_bin / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", str(args.jar), "-d", str(classes), str(HELPER)], check=True)
    raw_output = args.work_dir / "grammar.raw.json"
    subprocess.run([str(args.java_bin / ("java" + suffix)), "-Xmx2g", "-cp", str(classes) + os.pathsep + str(args.jar), "dev.mercurio.pilot.PilotGrammarExporter", str(args.pilot_root), str(raw_output)], check=True)
    if digest(HELPER) != helper_hash or digest(args.jar) != jar_hash:
        raise ValueError("Generator inputs changed during export")
    document = json.loads(raw_output.read_text(encoding="utf-8"))
    validate_structure(document)
    validate_contexts(document)
    sources = validate_sources(document, args.pilot_root)
    if sources != input_hashes:
        raise ValueError("Raw grammar inputs changed during export or their closure is incomplete")
    document["provenance"] = {
        "pilot_revision": revision,
        "jar_sha256": jar_hash,
        "exporter_sha256": helper_hash,
        "driver_sha256": digest(Path(__file__)),
        "sources_sha256": sources,
    }
    result = encode(document)
    if args.check:
        if not args.out.is_file() or args.out.read_bytes() != result:
            raise ValueError(f"Stale grammar artifact: {args.out}")
        print(f"Verified deterministic grammar artifact ({len(result)} bytes)")
    else:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_bytes(result)
        print(f"Wrote deterministic grammar artifact ({len(result)} bytes)")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
