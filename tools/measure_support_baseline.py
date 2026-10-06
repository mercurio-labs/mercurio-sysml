"""Measure pinned support dimensions without treating extraction as implementation."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
REPORT = ROOT / "docs/conformance/2026-08-support/measurement-baseline.json"


def measure(coverage, grammar, programs, ecore, candidate_programs):
    rows = coverage["coverage_checklist"]["rows"]
    active = [r for r in rows if r["applicability"] == "used_by_pin"]
    rule_ids = {r["id"] for g in grammar["grammars"] for r in g["rules"]}
    admitted, roots = set(programs["rules"]), set(programs["roots"])
    for program in programs["language_programs"].values():
        admitted.update(program["rules"])
        roots.update(program["roots"])
    if not admitted <= rule_ids or not roots <= admitted:
        raise ValueError("Admitted rules/roots must resolve into the pinned grammar inventory")
    candidate_rules, candidate_roots = set(), set()
    by_language = {}
    for language, program in candidate_programs["language_programs"].items():
        rules, entries = set(program["rules"]), set(program["roots"])
        if not rules <= rule_ids or not entries <= rules:
            raise ValueError("Candidate emitted rules/roots must resolve into the pinned grammar inventory")
        candidate_rules.update(rules)
        candidate_roots.update(entries)
        by_language[language] = len(rules)
    families = {}
    for domain in ["xtext", "ecore"]:
        selected = [r for r in active if r["domain"] == domain]
        families[domain] = {
            "required": len(selected),
            "partial_native_consumers": sum(r["native_support"] == "partial" and bool(r["native_consumers"]) for r in selected),
            "bounded_evidence": sum(r["verification_status"] == "bounded_evidence_only" for r in selected),
            "fully_qualified": sum(r["completion"] == "complete" for r in selected),
            "unassessed_ids": [r["id"] for r in selected if r["native_support"] == "unassessed"],
        }
    return {
        "schema": "dev.mercurio.support-measurement-baseline.v1",
        "pilot_revision": grammar["provenance"]["pilot_revision"],
        "overall_completion_percent": None,
        "meaning": "Separate source inventory, strict generated admission, bounded evidence and whole-family closure. Counts are unweighted; no combined support percentage is justified.",
        "inventory": {
            "grammars": len(grammar["grammars"]),
            "declared_rules": len(rule_ids),
            "ecore_elements_by_kind": dict(sorted(Counter(e["kind"] for e in ecore["elements"]).items())),
            "delegate_binding_records": len(ecore["delegate_bindings"]),
        },
        "strict_generated_admission": {
            "rule_count": len(admitted), "rule_denominator": len(rule_ids),
            "rule_percent": round(100 * len(admitted) / len(rule_ids), 2),
            "entry_root_count": len(roots),
            "rule_ids": sorted(admitted), "entry_root_ids": sorted(roots),
            "limitation": "Membership in strict generated programs is not full semantic verification, production reachability, or total native parser coverage. Handwritten parser support is not measured here.",
        },
        "candidate_emission": {
            "rule_count": len(candidate_rules), "declared_rule_count": len(rule_ids),
            "rules_by_language": dict(sorted(by_language.items())),
            "entry_root_count": len(candidate_roots),
            "rule_ids": sorted(candidate_rules), "entry_root_ids": sorted(candidate_roots),
            "declared_but_not_emitted_ids": sorted(rule_ids - candidate_rules),
            "limitation": "Emitted candidate programs are separate from strict fragment admission. Emission proves neither execution nor semantic support; omitted rules may be overridden, unreachable, or handled by terminal consumers. No completion percentage follows from this count.",
        },
        "families": families,
        "completion_gates": coverage["coverage_checklist"]["completion_accounting"],
        "unmeasured": [
            "Complete individually reviewed behavioral obligations and their verification outcomes",
            "Native consumer reachability and complete behavior per upstream rule/feature",
            "Current all-sample pipeline and release-wide semantic results",
            "Comparable release timing against a recorded acceptance criterion",
        ],
    }


def current():
    paths = [ROOT / "docs/conformance/2026-08-support/structural-source-coverage.json",
             PROFILE / "grammar.structure.extract.json", PROFILE / "xtext-fragment-programs.json",
             PROFILE / "ecore-semantics.extract.json", PROFILE / "xtext-finite-programs.experimental.json"]
    coverage, grammar, programs, ecore, candidate_programs = [json.loads(p.read_text(encoding="utf-8")) for p in paths]
    # Reject stale coverage/program provenance rather than measure mixed revisions.
    inputs = dict(coverage["inputs"])
    inputs.update(coverage["coverage_checklist"]["evidence_file_sha256"])
    for name, expected in inputs.items():
        if hashlib.sha256((ROOT / name).read_bytes()).hexdigest() != expected:
            raise ValueError("Stale coverage evidence: " + name)
    for emitted in (programs, candidate_programs):
        for name, expected in emitted["source_sha256"].items():
            if hashlib.sha256((PROFILE / name).read_bytes()).hexdigest() != expected:
                raise ValueError("Stale generated program source: " + name)
    result = measure(coverage, grammar, programs, ecore, candidate_programs)
    result["input_sha256"] = {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
    result["generator_sha256"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="Create the baseline once; never overwrite it")
    parser.add_argument("--check", action="store_true", help="Compare current measurements and provenance with the saved baseline")
    args = parser.parse_args()
    result = current()
    data = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.write:
        with REPORT.open("x", encoding="utf-8") as output:
            output.write(data)
    elif args.check:
        if json.loads(REPORT.read_text(encoding="utf-8")) != result:
            raise ValueError("Measurements or provenance changed since the frozen baseline")
    else:
        print(data)
    if args.write or args.check:
        print("Baseline: strict generated rules {rule_count}/{rule_denominator} ({rule_percent}%), roots {entry_root_count}".format(**result["strict_generated_admission"]))


if __name__ == "__main__":
    main()
