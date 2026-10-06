# 2026-08 semantic and performance assessment

Executed 2026-09-18T02:05:09.912186+00:00. This assesses
the candidate worktree; **it is not release qualification**. Source, library,
runtime and binary hashes are recorded alongside this report. The native compiler and Pilot runtime were held fixed during measurement.
The exporter records ordered reference sequences separately from the legacy edge
set. Source/build provenance identifies the exact helper used for this run.
The source snapshot, assessment scripts and measured compiler binaries were frozen for this run.

This run reuses only the preceding run's unchanged Pilot oracle exports, verified by source-set, runtime, helper and export hashes. All native comparisons and all timing trials were rerun. See `oracle-reuse-provenance.json`.

The [implementation evidence](../2026-08-support/typing-evidence.json) records
310/310 sample compilations, 56 fresh Pilot-backed positive/negative controls,
377 SysML tests, 499 Foundation tests and passing native/WASM consumer checks.
The candidate adds inherited/minimal typing, twelve additional extracted type
predicates, authoring round-trip fixes, metadata typing, verification and named
assignment metaclasses, and source-local implicit default selection. The repaired
library retains Pilot's inherited feature types and cached implicit supertypes.

A preliminary audit exposed a same-name cross-source implicit-type collision.
Its failed snapshot remains in `../../../../target/assessment-2026-08-typing/`;
this report uses a new frozen snapshot after the fix and regression test. No
failed result was replaced inside that earlier snapshot. All native comparisons
were rerun. Compared with the prior post-audit report, named metaclass differences
fell from 284 to 273, and 140 additional reference fields are now compared. These
bounded improvements do not close the open qualification gates.

## Detailed semantic comparison

All 310 targets were attempted in **96 isolated, identical source sets**. Release
and Pilot files were checked byte for byte. Native uses `../target/support-2026-08/stdlib.inherited-types.kir.json`;
Pilot uses commit `692170b71867353b8f90341e61556f49a5beb0e5` and the 0.62.0 runtime. The semantic
source oracle uses `CheckMode.ALL` with its precomputed index disabled. The
performance run uses the published index, as described below.

- Completed comparisons: **310/310**; failures/incomplete:
  **0** (listed in `semantic-summary.json`).
- Source elements: **11,221 native**, **247,516 Pilot**.
- Raw paired results: **0 exact**, **7,745 mismatched**.
- Unpaired: **3,476 native-only**, **239,771 Pilot-only**.

These are not counts of independent semantic bugs. Pilot materializes membership,
typing, redefinition and expression objects that native KIR often stores as
properties. Identifiers, scalar/list forms, derived/default fields, order and
duplicate relationships also differ. Anonymous objects on one source line can be
paired imperfectly. No tolerance or exclusion turns these differences into passes.

Separate triage matches only unique `(declared name, start line)` pairs within a
source file. It maps reference IDs through observed pairs in the same source
group and compares six reference fields as sets. It does not declare absent
fields, anonymous objects or missing expressions equivalent. Raw reports remain:

- **6,602** uniquely paired named declarations.
- **17,994/21,720** checked reference fields agree after the stated projection.
- **4,671** reference fields present on only one side are outside that denominator.
- **273** metaclass differences and
  **173** direct Boolean differences.
- **17/503** matched declarations with a Pilot `FeatureValue` lack initializer IR on that native declaration. Inspect alternate lowering before treating every case as lost semantics.
- **690** missing/different nonempty Pilot short-name or direction fields.
- Ambiguous named anchors excluded from declaration pairing: **1**; **197** Pilot named anchors have no native pair (not automatically lost semantics).
- **49** literal initializer values compared;
  **0** literal-value differences.

Examples requiring implementation review:

- `sysml/src/examples/Simple Tests/ConstraintTest.sysml:88`, `mass`: **metaclass** differs. Native `Feature`; Pilot `ReferenceUsage`.
- `kerml/src/examples/Simple Tests/Dependencies.kerml:11`, `Use`: **metaclass** differs. Native `Feature`; Pilot `Dependency`.
- `sysml/src/examples/Simple Tests/PartTest.sysml:24`, `B1`: **is_variable** differs. Native `True`; Pilot `False`.

`semantic-differences.json` records every triaged source/line/property difference;
`semantic-cases.csv` records per-file counts. Ordered initializer-tree observations
are in `semantic-summary.json` under `ordered_expression_projection`: supported
literal, reference, operator and invocation forms retain argument order and
multiplicity. Missing evidence, unsupported forms, ambiguous references and cycles
remain explicitly unassessed. Tree equality does not establish type, implicit
relationship or evaluation parity. Recorded ordered-tree observations are
`{"different": 5, "equal": 130, "unassessed": 368}`. Different trees may reflect lazy argument
wrappers or operator spellings (such as `^` versus `**`) in Pilot; inspect the
retained trees before classifying a structural difference as a semantic defect.

The exporter includes every source EAttribute (including literal values and
operators), every stored non-container source EReference, and its existing
derived-reference whitelist. Stored and selected derived references additionally retain ordered target lists, including duplicates and empty lists. Every exported source element is retained by the
strict snapshot. This is not an exhaustive proof for every computed EMF reference
or OMG constraint. The bounded tree projection does not establish all expression forms, multiplicity semantics, or behavioral execution. Registry-query
fallbacks may retain direct properties without
establishing full derived-value coverage. Full compressed snapshots and raw diffs
remain under `../../../../target/assessment-2026-08-typing-final/groups/`;
`semantic-artifacts.json` records their paths and uncompressed SHA-256 digests.

## Performance assessment

**48/48** measured executions succeeded.
Eight groups cover 73 targets across both languages, small models, the two broad Simple Tests
suites, behavior/requirements training, and the specification vehicle example.
Each engine runs three times per group in fresh processes; engine order alternates
across trials and groups. Within each group both engines read/parse inputs once;
native compiles targets sequentially and Pilot validates loaded resources
sequentially, retaining their ordinary in-process caches. Below are median seconds. `performance.csv` includes
min/max; `performance-results.json` includes every phase and trial.

| Source group | Targets | Native process | Pilot process | Native model phase | Pilot model phase |
| --- | ---: | ---: | ---: | ---: | ---: |
| kerml/src/examples/Address Book Example | 1 | 0.729 | 8.083 | 0.348 | 0.414 |
| kerml/src/examples/Simple Tests | 25 | 1.148 | 10.669 | 0.761 | 2.677 |
| sysml/src/examples/Simple Tests | 35 | 2.708 | 13.292 | 2.333 | 5.322 |
| sysml/src/examples/Vehicle Example | 4 | 3.742 | 16.547 | 3.359 | 8.820 |
| sysml/src/training/02. Part Definitions | 1 | 0.744 | 8.416 | 0.358 | 0.570 |
| sysml/src/training/20. Assignment Actions | 1 | 0.784 | 9.450 | 0.403 | 1.181 |
| sysml/src/training/23. State Definitions | 2 | 0.866 | 9.052 | 0.485 | 1.183 |
| sysml/src/training/32. Requirements | 4 | 0.879 | 9.280 | 0.505 | 1.469 |

Process time includes startup and library loading through completed target
compilation/validation. Model phase means input reading/parsing plus native
compilation/validation or Pilot validation (including lazy resolution). It excludes
explicit initialization and library-load phases. Library resolution can still
occur lazily, so subtracting load time does not perfectly equalize preparation.
Graph materialization, export, comparison, compression and builds are excluded.

Native uses Cargo `--release` and prebuilt KIR. Pilot loads source libraries with
the pinned release's checked-in `.index.json` enabled, its normal configuration.
The OS file cache is warm/uncontrolled, not flushed; the JVM is fresh per trial.
This is not a warm incremental-edit or UI benchmark. Pilot's maximum heap is
3 GiB. Host: Windows 11, Intel i7-11850H (8 cores/16 threads), approximately 64 GiB
RAM. Versions and hashes are in `build-provenance.json`.

**These timings compare current workflows, not equivalent semantic work.** Pilot
runs its complete validator set; native validation and preservation are incomplete.
Lower native time is not a speedup claim for a fully conformant compiler. Three
trials provide descriptive medians/ranges, not confidence intervals or tail latency.

## Reproduce

From `mercurio-sysml`, after preparing the pinned sources/runtime and candidate
library in the [implementation checkpoint](../2026-08-support/README.md):

```powershell
cargo build --release --locked -j 1 -p mercurio-tools --features legacy-pilot-tools --bin compare_pilot_semantics --bin audit_release_compile
javac -cp ../target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar -d ../target/assessment-2026-08-typing-final/classes tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java
python -B tools/run_release_assessment.py --out ../target/assessment-2026-08-typing-final --stage prepare --stdlib ../target/support-2026-08/stdlib.inherited-types.kir.json
python -u -B tools/run_release_assessment.py --out ../target/assessment-2026-08-typing-final --stage semantics --stdlib ../target/support-2026-08/stdlib.inherited-types.kir.json
python -u -B tools/run_release_assessment.py --out ../target/assessment-2026-08-typing-final --stage performance --stdlib ../target/support-2026-08/stdlib.inherited-types.kir.json
python -B tools/summarize_release_assessment.py --root ../target/assessment-2026-08-typing-final
python -B tools/publish_release_assessment.py --root ../target/assessment-2026-08-typing-final --dest docs/conformance/2026-08-typing-assessment
```

The semantic stage exports up to three independent source sets concurrently; all
workers finish before sequential benchmark trials start. The runner checks hashes,
isolates source sets, retains timeout/failure records
and resumes completed semantic groups. Cached exports must declare the exact
requested support set. Use a new output directory when changing compiler inputs;
do not combine candidates.
