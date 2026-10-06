"""Export pinned Pilot validator methods using Xtend's parser, linker and type system.

The selected method bodies are exported in full. This is an AST and binding
export, not a claim that arbitrary Xtend expressions can be translated.
Java/Xtend is a development-time generator dependency, never a native runtime.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PILOT_REVISION = "692170b71867353b8f90341e61556f49a5beb0e5"
JAR_SHA256 = "b1ad9d64b1f0c75730facf25a5e2856bc9df2bb4bd39476df2fdf5ae68cd9350"
XTEND_VERSION = "2.38.0"
XTEND_SHA256 = "aac6d80aaa36f9e897d45c76a6df39cd17764ba53ac86fff91f35252094cf4e0"
SOURCE = "org.omg.kerml.xtext/src/org/omg/kerml/xtext/validation/KerMLValidator.xtend"
SYSML_SOURCE = "org.omg.sysml.xtext/src/org/omg/sysml/xtext/validation/SysMLValidator.xtend"
SELECTION = {SOURCE: {"checkAnnotation", "checkImport"}, SYSML_SOURCE: {'checkUseCaseUsage', 'checkEnumerationUsage', 'checkOneType', 'checkRenderingUsage', 'checkReferenceUsage', 'checkViewpointUsage', 'checkMetadataUsage', 'checkEnumerationDefinition', 'checkAnalysisCaseUsage', 'checkVerificationCaseUsage'}}
METHODS = set().union(*SELECTION.values())
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotXtendExporter.java"
DEFAULT_OUTPUT = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/validators.ast.extract.json"


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def encode(document: dict) -> bytes:
    # Java immutable-map iteration order deliberately varies between JVMs.
    return (json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def validate_structure(document: dict, selection=None) -> None:
    selection = SELECTION if selection is None else selection
    selected_methods = set().union(*selection.values())
    if document.get("schema_version") != 1 or document.get("source_format") != "raw-xtend":
        raise ValueError("Unsupported Xtend AST schema")
    if document.get("parser") != "org.eclipse.xtend.core.XtendStandaloneSetup":
        raise ValueError("Expected actual Xtend parser and linker")
    if document.get("span_encoding") != "utf-16-code-units":
        raise ValueError("Xtend source offsets must use UTF-16 code units")
    methods = document.get("methods", [])
    if len(methods) != len(selected_methods) or {m.get("name") for m in methods} != selected_methods:
        raise ValueError("Incomplete or duplicated selected methods")
    symbols = document.get("symbols")
    types = document.get("types")
    if not isinstance(symbols, dict) or not isinstance(types, dict):
        raise ValueError("Missing resolved symbols or types")
    identities = set()
    references = []
    counts = Counter()

    def walk(value):
        if isinstance(value, list):
            for child in value:
                walk(child)
        elif isinstance(value, dict):
            if "$ref" in value:
                if set(value) != {"$ref"} or not isinstance(value["$ref"], str):
                    raise ValueError("Invalid reference shape")
                references.append(value["$ref"])
            elif "fields" in value:
                identity = value.get("id")
                if not isinstance(identity, str) or not identity or identity in identities:
                    raise ValueError("Missing or duplicated AST identity")
                identities.add(identity)
                if not isinstance(value.get("fields"), dict) or not isinstance(value.get("kind"), str):
                    raise ValueError("Incomplete AST node")
                counts[value["kind"]] += 1
                span = value.get("span")
                if not isinstance(span, dict) or set(span) != {"offset", "length", "start_line", "end_line"}:
                    raise ValueError("Missing AST source span")
                if not all(isinstance(n, int) for n in span.values()) or span["offset"] < 0 or span["length"] < 0 or span["start_line"] < 1 or span["end_line"] < span["start_line"]:
                    raise ValueError("Invalid AST source span")
                if value["fields"].get("invalidFeatureIssueCode") is not None:
                    raise ValueError("Unresolved feature issue in AST")
                walk(value["fields"])
            else:
                for child in value.values():
                    walk(child)

    for method in methods:
        if method.get("kind") != "XtendFunction" or method.get("name") not in selection.get(method.get("source"), set()):
            raise ValueError("Invalid selected method source")
        if method.get("id") != method.get("declaring_type", "") + "#" + method["name"]:
            raise ValueError("Invalid method identity")
        walk(method)
    for identity, symbol in symbols.items():
        if symbol.get("identifier") != identity or not isinstance(symbol.get("kind"), str):
            raise ValueError("Invalid JVM symbol identity")
        if "initializer" in symbol:
            walk(symbol["initializer"])
            initializer = symbol["initializer"]
            if "constant_value" in symbol and not all(symbol.get(flag) is True for flag in ("static", "final", "source_immutable", "source_static")):
                raise ValueError("Cannot fold mutable or non-static source field")
            if "constant_value" in symbol and initializer["kind"] == "XStringLiteral" and symbol["constant_value"] != initializer["fields"]["value"]:
                raise ValueError("Constant differs from parsed Xtend initializer")
    for reference in references:
        if reference not in identities and reference not in symbols:
            raise ValueError(f"Dangling JVM/AST reference: {reference}")
    coverage = {"node_kinds": dict(sorted(counts.items())), "nodes": sum(counts.values()), "selected_methods": len(methods), "unresolved_references": 0}
    if document.get("coverage") != coverage:
        raise ValueError("AST export coverage mismatch")


def clean_revision(root: Path) -> str:
    revision = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
    if revision != PILOT_REVISION:
        raise ValueError(f"Wrong Pilot revision: {revision}")
    status = subprocess.check_output(["git", "-C", str(root), "status", "--porcelain", "--untracked-files=all"], text=True).strip()
    if status:
        raise ValueError("Pinned Pilot checkout must be clean for Xtend export")
    return revision


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pilot-root", type=Path, default=ROOT.parent / "target/upstream/SysML-v2-Pilot-Implementation")
    parser.add_argument("--jar", type=Path, default=ROOT.parent / "target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar")
    parser.add_argument("--xtend-jar", type=Path, default=Path.home() / f".m2/repository/org/eclipse/xtend/org.eclipse.xtend.core/{XTEND_VERSION}/org.eclipse.xtend.core-{XTEND_VERSION}.jar")
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--work-dir", type=Path, default=ROOT.parent / "target/support-2026-08/xtend-structure")
    parser.add_argument("--scope", type=Path, help="Explicit bounded scope; requires a separate --out artifact")
    parser.add_argument("--check", action="store_true", help="Regenerate and reject stale checked-in AST")
    args = parser.parse_args(argv)
    selection = SELECTION
    scope_hash = None
    if args.scope:
        if args.out.resolve() == DEFAULT_OUTPUT.resolve():
            raise ValueError("Trial scope must use a separate --out artifact")
        scope = json.loads(args.scope.read_text(encoding="utf-8"))
        if scope.get("schema_version") != 1:
            raise ValueError("Unsupported scope schema")
        owners = {"org.omg.kerml.xtext.validation.KerMLValidator": SOURCE, "org.omg.sysml.xtext.validation.SysMLValidator": SYSML_SOURCE}
        selection = {source: set() for source in selection}
        identities = scope["checks"] + scope["helpers"]
        if len(identities) != len(set(identities)):
            raise ValueError("Duplicate scope method")
        for identity in identities:
            owner, name = identity.split("#")
            selection[owners[owner]].add(name)
        selection = {source: names for source, names in selection.items() if names}
        scope_hash = digest(args.scope)
    revision = clean_revision(args.pilot_root)
    if digest(args.jar) != JAR_SHA256:
        raise ValueError("Wrong pinned Pilot/Xtext JAR")
    if digest(args.xtend_jar) != XTEND_SHA256:
        raise ValueError("Wrong pinned Xtend 2.38.0 parser JAR")
    inputs = {source: digest(args.pilot_root / source) for source in selection}
    helper_hash = digest(HELPER)
    driver_hash = digest(Path(__file__))
    classes = args.work_dir / "classes"
    classes.mkdir(parents=True, exist_ok=True)
    suffix = ".exe" if os.name == "nt" else ""
    dependencies = os.pathsep.join(map(str, [args.jar, args.xtend_jar]))
    subprocess.run([str(args.java_bin / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", dependencies, "-d", str(classes), str(HELPER)], check=True)
    raw = args.work_dir / "validators.raw.json"
    selected_path = args.work_dir / "selection.json"
    selected_path.write_bytes(encode({source: sorted(names) for source, names in selection.items()}))
    subprocess.run([str(args.java_bin / ("java" + suffix)), "-Xmx2g", "-cp", str(classes) + os.pathsep + dependencies, "dev.mercurio.pilot.PilotXtendExporter", str(args.pilot_root), str(raw), str(selected_path)], check=True)
    document = json.loads(raw.read_text(encoding="utf-8"))
    validate_structure(document, selection)
    if clean_revision(args.pilot_root) != revision or inputs != {source: digest(args.pilot_root / source) for source in selection}:
        raise ValueError("Pilot sources changed during export")
    if digest(HELPER) != helper_hash or digest(Path(__file__)) != driver_hash or digest(args.jar) != JAR_SHA256 or digest(args.xtend_jar) != XTEND_SHA256:
        raise ValueError("Generator inputs changed during export")
    document["provenance"] = {
        "pilot_revision": revision, "jar_sha256": JAR_SHA256,
        "xtend_version": XTEND_VERSION, "xtend_jar_sha256": XTEND_SHA256,
        "exporter_sha256": helper_hash, "driver_sha256": driver_hash,
        "sources_sha256": inputs,
    }
    if args.scope:
        if digest(args.scope) != scope_hash:
            raise ValueError("Scope changed during export")
        document["provenance"]["scope_sha256"] = scope_hash
    result = encode(document)
    if args.check:
        if not args.out.is_file() or args.out.read_bytes() != result:
            raise ValueError(f"Stale Xtend AST artifact: {args.out}")
        print(f"Verified deterministic Xtend AST ({len(result)} bytes)")
    else:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_bytes(result)
        print(f"Wrote resolved Xtend AST ({len(result)} bytes)")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
