# SysML v2 / Pilot 2026-08 compliance audit

Date: 2026-09-17. Result: **release sample compilation parity fails;
full OMG tool conformance is not established**.

The audit was performed on branch codex/sysml-v2-compliance-2026-08 in isolated
outer, mercurio-sysml and mercurio-foundation worktrees created from local
develop. Local develop was ahead of origin/develop; no uncommitted work from
the original checkouts was copied or changed.

## Sources

| Source | Exact baseline |
| --- | --- |
| Mercurio SysML develop | ced2caa463f460f9d1c0a0741fd9b992543a259e |
| Mercurio Foundation develop | 0539a9d3294110f752850840b1fcad9115d2bcff |
| SysML-v2-Release tag 2026-08 | fb97b754f29588b8e9c7a35f370880cd15eb29e7 |
| SysML-v2-Pilot-Implementation tag 2026-08 | 692170b71867353b8f90341e61556f49a5beb0e5 |
| Published Pilot runtime | jupyter-sysml-kernel-0.62.0-all.jar |

The live GitHub release API identified 2026-08 as the latest published release
on the audit date. The Pilot release was published September 11 and the
specification/sample release September 12. The public web search cache was
older, so it was not used to select the baseline.

The current formal documents were downloaded directly from OMG:
[SysML 2.0](https://www.omg.org/spec/SysML/2.0/Language/PDF),
formal/2026-03-02, and
[KerML 1.0](https://www.omg.org/spec/KerML/1.0/PDF),
formal/2026-03-01. The latest release repository contains **SysML 2.1 Beta 2
and KerML 1.1 Beta 2**, dated August 2026 and labeled Release 2026-07 on their
cover pages. These are different baselines; the beta Pilot result is not a
claim of formal SysML 2.0 certification. PDF URLs and SHA-256 digests are in
[specifications.json](specifications.json).

All 310 sample source files in the release checkout are byte-identical to the
corresponding files in the tagged Pilot source checkout. Both upstream
checkouts remained clean. The vendored external checkout was not modified.

## Sample results

Every .sysml and .kerml file under sysml/src and kerml/src was included.
The engines received identical source sets: sibling model files plus transitive,
explicit cross-folder dependencies. Seven missing dependency entries were
added to the audit corpus seed after Pilot diagnosed their missing imports.

| Corpus | Files | Native compilation passes | Pilot validation passes |
| --- | ---: | ---: | ---: |
| KerML examples | 58 | 26 | 58 |
| SysML examples | 96 | 44 | 96 |
| SysML training | 100 | 63 | 100 |
| SysML validation | 56 | 23 | 56 |
| Total | **310** | **156** | **310** |

There are **154 Pilot-accepted source sets that native compilation rejects**.
All 310 comparisons completed; none remains skipped, crashed, or timed out.
The first Vehicle group hit a 300-second bound. The final run used 600 seconds,
validated smaller targets first, and completed all four Vehicle cases. The
Annex A model's validation took approximately 190 seconds in the successful run.

Native failure classification:

- **10 target syntax errors**: 300/310 target files parse successfully.
- **67 target resolution/lowering errors** after their source sets parse.
- **77 support-file syntax failures**: the target parses, but another file
  in its required source set does not. These are cascading failures, not 77
  additional independently unsupported target syntaxes.

Examples of direct syntax gaps are unnamed transitions with first/accept/then,
transition bodies, arrow invocation in assignment expressions, named flow
payloads, and KerML metadata annotation syntax. Representative sources include
StateTest.sysml, Assignment Example.sysml, 3d-Function-based Behavior-item.sysml,
MetadataTest.kerml, and the specification's SimpleVehicleModel.

Resolution failures include feature imports, implicit start/fork specializations,
inherited redefinitions, and expression/reference targets. Diagnostics preserve
the file and source span where available.

See [audit.md](audit.md) for the complete mismatch list,
[audit.json](audit.json) for every engine result,
[corpus.json](corpus.json) for exact input sets, and
[source-lock.json](source-lock.json) for file, library, runtime and source hashes.
Raw JSONL, JVM logs and initial-run evidence remain in the worktree's
target/release-audit-2026-08 and target/release-audit-2026-08-final directories.

## Semantic validation counterexample

A separate negative control parses in both engines:

    package Audit {
        part def Vehicle;
        attribute invalid: Vehicle;
    }

Native strict compilation produces KIR. Pilot rejects it with
validateAttributeUsageType_: an attribute must be typed by attribute definitions.
This demonstrates a gap in the audited native compilation path's enforcement
of well-formedness constraints. It does not inventory every validation capability
available through other Mercurio surfaces.

Positive, unresolved-name and malformed-syntax controls also behaved as expected.
[controls.json](controls.json) preserves their sources and both engines' results.
The controls are separate from the 310 upstream sample count.

## Specification coverage and drift

Clause 2 of both the formal and beta specifications requires abstract-syntax
and model-interchange conformance as the minimum for a conforming tool.
Textual notation and semantic conformance have additional obligations.
Compiling sample files alone cannot establish those claims.

| Conformance area | Evidence from this audit |
| --- | --- |
| Textual notation | Incomplete on the latest release corpus: 10 target parser failures |
| Abstract syntax / well-formedness | Negative type control diverges; complete constraint coverage is unestablished |
| Semantic interpretation / graph equivalence | Not established by acceptance parity; existing legacy semantic comparator does not build on this develop baseline |
| Model interchange | Not qualified by this audit |
| Graphical notation, execution, optional domain libraries | Not qualified by this audit |

Candidate grammar, Ecore and validator artifacts were mechanically extracted
from the clean 2026-08 Pilot pin using existing Rust extractors and a separate
[candidate-pilot.lock.json](candidate-pilot.lock.json). The shipped runtime pin
and default library remain unchanged pending a passing qualification.

Compared with the shipped sysml-2.0-pilot-2026-04 extraction:

- The extracted counts remain 257 metaclasses, 620 structural features and
  297 generalizations, with no changes to those extracted facts.
- There are 611 grammar construct rows and 724 grammar rule rows. Seven
  rule-call entries changed.
- There are 111 active validator checks and 467 issue constants. Four
  check records and one constant changed.
- The existing validator coverage ledger still has 111 pending mappings.
  Pending means coverage has not been traced, not proof that every check is absent.

[artifact-drift.json](artifact-drift.json) records the detailed differences.
Comparison ignores source line numbers and preserves duplicate records.
The static extraction does not capture all OCL, operation bodies or semantic
implementation changes; unchanged extracted fields do not prove equivalence.

The older compare_pilot_semantics tool fails to compile because it references
missing SemanticCompareProfile, tolerance, snapshot and coverage APIs.
The older diagnostics harness can also accept partial KIR and omits Pilot's
full validator. It is therefore not the basis of this result.

## Verification and next steps

The added audit has a strict native probe, a Pilot CheckMode.ALL validator shim,
a complete corpus runner, immutable fingerprints, diagnostic preservation,
and nonzero exit status for mismatches or incomplete comparisons.
Pilot caches are reusable only after input/library/JAR/shim fingerprint checks.

Completed checks:

- Default workspace build: cargo build --locked -j 1 passed.
- Strict repository boundary check passed for the present SysML/Foundation
  stack; the checker explicitly skipped absent AI/plugin workspace checks.
- Native audit and tool library tests passed (3).
- Existing grammar, metamodel and validator extractor tests passed (7).
- Python audit tests passed (4).
- Four real Pilot positive/negative controls completed.
- Full corpus audit completed and intentionally returned exit code 1 for
  the 154 acceptance mismatches.

The initial parallel workspace build exhausted disk space during PDB linking.
Only generated debug-symbol files and the incremental cache inside this audit
worktree were removed; the subsequent single-job build passed.

Recommended repair order is parser gaps first (including the 77 cascading
source-set failures), then resolution/lowering, then systematic validator
coverage and negative cases. Restore semantic graph comparison and qualify
model interchange before making a broader conformance claim.

See [the reproducible procedure](../../release-audit.md). This is an audit and
qualification failure record, not an implementation of the compiler repairs.

Final formatting removed extra blank lines at the end of two audit source files.
[Executed source snapshots](executed-audit-sources.json) preserve the exact UTF-8
text and fingerprints, including original line endings; the source lock is unchanged.
