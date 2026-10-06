"""Check deterministic source-derived contracts without corpus runs or benchmarks."""
import argparse
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PROFILE = Path("crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--from-check", help="Resume at a script basename after earlier checks passed; reports only the selected suffix")
    args = parser.parse_args()
    commands = [
        ["tools/export_pilot_grammar_structure.py", "--java-bin", args.java_bin, "--check"],
        ["tools/generate_release_enum_rules.py", "--check"],
        ["tools/generate_xtext_assignment_contracts.py", "--check"],
        ["tools/generate_xtext_terminal_programs.py", "--check"],
        ["tools/export_pilot_prediction.py", "--java-bin", args.java_bin, "--check"],
        ["tools/generate_xtext_fragment_programs.py", "--check"],
        ["tools/generate_xtext_partial_programs.py", "--check"],
        ["tools/generate_xtext_context_controls.py", "--check"],
        ["tools/run_comment_value_probe.py", "--java-bin", args.java_bin, "--check"],
        ["tools/run_feature_chain_probe.py", "--java-bin", args.java_bin, "--check"],
        ["tools/run_chain_link_probe.py", "--java-bin", args.java_bin, "--check"],
        ["tools/run_metadata_entry_probe.py", "--java-bin", args.java_bin, "--check"],
        ["tools/run_relationship_decision_probe.py", "--java-bin", args.java_bin, "--check"],
        ["tools/run_expanded_decision_probe.py", "--java-bin", args.java_bin, "--check"],
        ["tools/run_succession_entry_probe.py", "--java-bin", args.java_bin, "--check"],
        ["tools/run_expression_decision_probe.py", "--java-bin", args.java_bin, "--check"],
        ["tools/run_expression_model_probe.py", "--java-bin", args.java_bin, "--check"],
        ["tools/extract_ecore_semantics.py", "--check"],
        ["tools/export_pilot_ecore_effective.py", "--java-bin", args.java_bin, "--check"],
        ["tools/generate_ecore_ownership.py", "--check"],
        ["tools/generate_ecore_hierarchy.py", "--check"],
        ["tools/generate_ecore_model.py", "--check"],
        ["tools/generate_ecore_defaults.py", "--check"],
        ["tools/audit_ecore_defaults.py", "--java-bin", args.java_bin, "--check"],
        ["tools/audit_multiplicity_literals.py", "--check"],
        ["tools/check_structural_source_coverage.py", "--check"],
        ["tools/generate_namespace_grammar_contracts.py", "--grammar", PROFILE / "grammar.structure.extract.json",
         "--metamodel", PROFILE / "metamodel.extract.json", "--out", PROFILE / "namespace-grammar.extract.json",
         "--rust-out", "crates/mercurio-sysml/src/namespace_grammar_generated.rs", "--check"],
        ["tools/export_pilot_xtend.py", "--java-bin", args.java_bin, "--check"],
        ["tools/translate_pilot_validators.py", "--check"],
        ["tools/generate_validation_rust.py", "--check"],
        ["tools/check_validation_bindings.py", "--check"],
        ["tools/export_validator_inventory.py", "--java-bin", args.java_bin, "--check"],
        ["tools/classify_validator_inventory.py", "--check"],
        ["tools/export_pilot_xtend.py", "--java-bin", args.java_bin,
         "--scope", PROFILE / "bounded-translation-scope.json",
         "--out", PROFILE / "validators.bounded.ast.extract.json",
         "--work-dir", ROOT.parent / "target/support-2026-08/xtend-bounded", "--check"],
        ["tools/check_bounded_translation.py", "--check"],
        ["tools/run_validation_predicate_probe.py", "--java-bin", args.java_bin, "--check"],
    ]
    if args.from_check:
        matches = [i for i, command in enumerate(commands) if Path(command[0]).name == args.from_check]
        if len(matches) != 1:
            parser.error("--from-check must identify exactly one check script")
        commands = commands[matches[0]:]
    for command in commands:
        subprocess.run([sys.executable, "-B", *map(str, command)], cwd=ROOT, check=True)
    print(f"Selected source-contract suffix is current ({len(commands)} checks)" if args.from_check else "All structural and translated-validator source contracts are current")


if __name__ == "__main__":
    main()
