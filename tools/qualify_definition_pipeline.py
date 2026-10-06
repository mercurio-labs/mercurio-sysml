"""Run and record bounded native definition-pipeline qualification.

This certificate qualifies named tested behavior only. It cannot close a whole
source family or release gate. Source drift invalidates it; stored shapes are
not equivalent to all stored lifecycle semantics, nor is emitted grammar support.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
OUTPUT = ROOT / "docs/conformance/2026-08-support/definition-pipeline-qualification.json"
REQUIRED_TESTS = {
    "definition_feature_type_queries_reject_unassessed_inputs_without_mutation",
    "definition_feature_types_match_pilot_and_roundtrip",
    "definition_association_defaults_match_resolved_controls_and_boundaries",
    "definition_absent_reference_rejects_unassessed_contributors",
    "definition_connector_participant_library_is_required_only_without_computed_redefinition",
    "definition_class_end_generalizations_match_pilot_source_queries",
    "definition_feature_owned_sources_match_pilot_and_roundtrip",
    "definition_feature_owned_defaults_reach_source_and_reject_unknown_owners",
    "definition_feature_owned_defaults_reject_ownership_cycles",
    "definition_feature_defaults_match_resolved_pilot_controls",
    "definition_feature_defaults_unlock_connector_library_inheritance",
    "definition_connector_defaults_match_resolved_pilot_controls",
    "definition_connector_default_provider_reaches_source_and_preserves_guards",
    "definition_connector_projections_match_complete_pilot_inputs",
    "definition_specialization_owner_delegates_follow_canonical_containment",
    "definition_ordinary_strategy_source_batch_matches_pilot",
    "definition_featuring_dispatch_matches_all_admitted_pilot_classes",
    "definition_usage_featuring_preserves_missing_semantic_dependencies",
    "definition_usage_featuring_matches_all_pilot_bindings_on_complete_inputs",
    "definition_variable_featuring_matches_independent_pilot",
    "definition_variable_featuring_missing_libraries_and_collisions_are_atomic",
    "definition_owning_type_featuring_matches_independent_pilot",
    "definition_owning_type_featuring_rejects_unqualified_branches_atomically",
    "definition_binding_construction_matches_independent_pilot",
    "definition_binding_construction_rolls_back_failed_adoption",
    "definition_chain_construction_matches_independent_pilot",
    "definition_chain_queries_read_canonical_storage_and_reject_invalid_edges",
    "definition_participant_contribution_matches_independent_pilot",
    "definition_participant_contribution_rejects_missing_dependencies",
    "definition_variation_typing_matches_independent_pilot",
    "definition_variation_typing_does_not_complete_usage_provider",
    "definition_variability_matches_complete_pilot_inputs",
    "definition_variability_preserves_missing_provider_boundary",
    "definition_specialization_matches_independent_pilot",
    "definition_specialization_preserves_unknown_provider_boundary",
    "definition_redefinition_scopes_reject_inaccessible_and_missing_targets",
    "definition_parameters_and_results_match_independent_pilot",
    "definition_invocation_explicit_arguments_suppress_computed_redefinitions",
    "definition_parameters_reject_missing_and_unassessed_dependencies",
    "definition_documents_publish_and_roundtrip_canonical_stored_models",
    "definition_documents_reject_missing_ambiguous_and_cyclic_links",
    "definition_type_scopes_preserve_unimplemented_dependency_boundaries",
    "definition_plain_inheritance_preserves_semantic_dependency_boundaries",
    "definition_library_provider_disagreements_remain_explicit",
    "generated_definition_defaults_match_resolved_pilot_branches",
    "definition_owned_end_query_checks_canonical_ownership_and_flags",
    "definition_individual_source_exposes_variability_dependency",
    "definition_feature_redefinitions_match_independent_pilot_contexts",
    "definition_positional_ends_match_independent_pilot_models",
    "definition_positional_ends_reject_unresolved_and_cyclic_dependencies",
    "definition_feature_redefinitions_preserve_positional_and_adapter_dependencies",
    "definition_default_dependencies_do_not_fall_back_or_hide_inherited_ends",
    "definition_document_public_pipeline_matches_independent_package_links",
    "definition_documents_match_independent_pilot_models_links_and_roundtrips",
    "stored_feature_transactions_publish_and_round_trip_atomically",
    "stored_feature_transactions_reject_unsupported_or_invalid_batches_without_changes",
    "publication_separates_partial_external_graphs_and_preserves_snapshots",
    "every_stored_contract_enforces_its_imported_value_shape",
    "fresh_containment_initialization_uses_ancestry_without_overwriting_values",
    "candidate_execution_failures_do_not_count_as_invalid_syntax",
    "closed_publication_requires_both_directions_of_present_stored_opposites",
    "closed_publication_rejects_opaque_interchange_extensions",
}
BOUNDARIES = [
    "Opt-in candidate API; legacy AST/profile emission is not migrated.",
    "Whole-document controls are bounded; emitted rules are not all qualified.",
    "Stored value-shape evidence is not full lifecycle support for 87 features.",
    "External resources, missing required/implicit semantics and unimplemented scopes remain open.",
    "Derived snapshots require explicit recomputation before supported stored edits can change their dependencies.",
    "Complete samples, release-wide semantic comparison and paired timing remain pending.",
]


def digest(content):
    return hashlib.sha256(content).hexdigest()


def inventory():
    source = json.loads((PROFILE / "ecore-semantics.extract.json").read_text(encoding="utf-8"))
    features = [e for e in source["elements"] if e["kind"] in ("EAttribute", "EReference")]
    def flag(row, name):
        value = row["source_attributes"].get(name, "false")
        return value is True or value == "true"
    stored = sorted(e["id"] for e in features if not flag(e, "derived"))
    derived = sorted(e["id"] for e in features if flag(e, "derived"))
    operations = sorted(e["id"] for e in source["elements"] if e["kind"] == "EOperation")
    if (len(stored), len(derived), len(operations)) != (87, 328, 70):
        raise ValueError("Pinned obligation inventory changed; review required")
    return {"stored_features": stored, "derived_features": derived, "operations": operations}


def inputs():
    foundation = ROOT.parent / "mercurio-foundation"
    files = {Path(__file__).resolve()}
    sources = set()
    # Include local build scripts, inherited manifests, non-Rust source fixtures
    # and complete resource trees. An allowlist of profile filenames misses
    # embedded kernel/library inputs used by the same native library run.
    for workspace in [ROOT, foundation]:
        for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rust-toolchain"]:
            candidate = workspace / name
            if candidate.is_file(): files.add(candidate)
        files.update(p for p in (workspace / ".cargo").rglob("*") if p.is_file())
        for crate in (workspace / "crates").iterdir():
            if not crate.is_dir(): continue
            for name in ["Cargo.toml", "build.rs"]:
                candidate = crate / name
                if candidate.is_file(): files.add(candidate)
            for name in ["src", "resources", "tests", "corpus"]:
                files.update(p for p in (crate / name).rglob("*") if p.is_file())
        sources.update(p for p in files if p.suffix == ".rs")
    # Unit tests also embed independent controls kept under docs. Capture those
    # exact inputs, rather than report prose/evidence outputs (a hash cycle).
    for source in sources:
        content = source.read_text(encoding="utf-8")
        for relative in re.findall(r'include_(?:str|bytes)!\(\s*"([^"\n]+)"', content):
            files.add((source.parent / relative).resolve())
        manifest = source.parent
        while not (manifest / "Cargo.toml").is_file():
            if manifest == manifest.parent: raise ValueError("No manifest for " + str(source))
            manifest = manifest.parent
        for relative in re.findall(r'include_(?:str|bytes)!\(\s*concat!\(\s*env!\("CARGO_MANIFEST_DIR"\),\s*"([^"\n]+)"', content):
            files.add((manifest / relative.lstrip("/")).resolve())
    for name in ["export_pilot_implicit_reduction.py", "test_implicit_reduction.py", "export_pilot_compatibility.py", "test_compatibility.py", "export_pilot_featuring_queries.py", "test_featuring_queries.py", "export_pilot_specialization_construction.py", "test_specialization_construction.py", "export_pilot_result_construction.py", "test_result_construction.py", "qualification_certificates.py", "structural_coverage_plan.py", "strategy_qualification.py", "test_qualification_certificates.py", "generate_ecore_model.py", "generate_xtext_partial_programs.py", "generate_xtext_fragment_programs.py", "export_pilot_definition_documents.py", "export_pilot_definition_defaults.py", "test_definition_defaults.py", "export_pilot_feature_redefinitions.py", "test_feature_redefinitions.py", "export_pilot_connector_defaults.py", "test_connector_defaults.py", "export_pilot_feature_defaults.py", "test_feature_defaults.py"]:
        candidate = ROOT / "tools" / name
        if candidate.exists(): files.add(candidate)
    files.add(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotDefinitionDocumentProbe.java")
    files.add(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotDefinitionDefaultExporter.java")
    files.add(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotFeatureRedefinitionExporter.java")
    files.add(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotConnectorDefaultExporter.java")
    files.add(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotFeatureDefaultExporter.java")
    files.add(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotFeaturingQueryExporter.java")
    files.add(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotCompatibilityExporter.java")
    files.add(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotImplicitReductionExporter.java")
    files.add(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotSpecializationConstructionExporter.java")
    files.add(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotResultConstructionExporter.java")
    files.add(ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotOperandProbe.java")
    files.add(ROOT / "tools/test_qualify_definition_pipeline.py")
    return {p.relative_to(ROOT.parent).as_posix(): digest(p.read_bytes()) for p in sorted(files)}


def check_test_output(output):
    passed = set(re.findall(r"^test ([^ ]+) \.\.\. ok$", output, re.MULTILINE))
    names = {name.rsplit("::", 1)[-1] for name in passed}
    if not REQUIRED_TESTS <= names:
        raise ValueError("Missing passing obligation tests: " + str(sorted(REQUIRED_TESTS - names)))
    result = re.search(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;", output)
    if result is None or int(result.group(1)) != len(passed):
        raise ValueError("Expected a complete, unfiltered passing library run")
    return sorted(passed)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--run", action="store_true")
    group.add_argument("--check", action="store_true")
    args = parser.parse_args()
    obligations = inventory()
    before = inputs()
    if args.check:
        certificate = json.loads(OUTPUT.read_text(encoding="utf-8"))
        if certificate["source_sha256"] != before or certificate["inventory"] != obligations:
            raise ValueError("Qualification certificate is stale")
        if certificate["boundaries"] != BOUNDARIES or certificate["status"] != "bounded_milestone_verified":
            raise ValueError("Changed qualification scope")
        log = ROOT / certificate["native_tests"]["log"]
        content = log.read_bytes()
        if digest(content) != certificate["native_tests"]["log_sha256"]:
            raise ValueError("Changed qualification test log")
        if check_test_output(content.decode("utf-8")) != certificate["native_tests"]["passed"]:
            raise ValueError("Changed qualification test results")
        build = ROOT / certificate["native_build"]["log"]
        if digest(build.read_bytes()) != certificate["native_build"]["log_sha256"] or certificate["native_build"]["exit_code"] != 0:
            raise ValueError("Changed native build evidence")
        print("Bounded qualification current; release gates remain open")
        return
    if not args.run:
        print(json.dumps({k:len(v) for k,v in obligations.items()}, indent=2))
        print("Inventory only; use --run to qualify the bounded milestone")
        return
    evidence_dir = ROOT / "docs/conformance/2026-08-support/definition-pipeline-evidence"
    evidence_dir.mkdir(exist_ok=True)
    tests_path = evidence_dir / "native-tests.log"
    build_path = evidence_dir / "native-build.log"
    def execute(command, path):
        with path.open("w", encoding="utf-8") as log:
            result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, timeout=1200)
        if result.returncode: raise ValueError("Qualification failed: " + str(path))
        return path.read_bytes()
    build = execute(["cargo", "check", "-p", "mercurio-sysml", "--lib"], build_path)
    tests = execute(["cargo", "test", "-p", "mercurio-sysml", "--lib"], tests_path)
    passed = check_test_output(tests.decode("utf-8"))
    if inputs() != before: raise ValueError("Qualification inputs changed during verification")
    document = {"schema":"dev.mercurio.definition-pipeline-qualification.v1", "status":"bounded_milestone_verified", "inventory":obligations,
        "boundaries":BOUNDARIES, "source_sha256":before,
        "native_build":{"command":"cargo check -p mercurio-sysml --lib", "exit_code":0, "log":build_path.relative_to(ROOT).as_posix(), "log_sha256":digest(build)},
        "native_tests":{"command":"cargo test -p mercurio-sysml --lib", "log":tests_path.relative_to(ROOT).as_posix(), "log_sha256":digest(tests), "passed":passed}}
    OUTPUT.write_text(json.dumps(document,indent=2)+"\n",encoding="utf-8")
    print(len(passed), "native tests passed; bounded qualification written")


if __name__ == "__main__": main()
