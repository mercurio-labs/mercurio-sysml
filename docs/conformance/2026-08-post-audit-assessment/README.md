# 2026-08 semantic and performance assessment

Executed 2026-09-18T00:25:18.224366+00:00. This assesses
the candidate worktree; **it is not release qualification**. Source, library,
runtime and binary hashes are recorded alongside this report. The native compiler and Pilot runtime were held fixed during measurement.
The exporter records ordered reference sequences separately from the legacy edge
set. Source/build provenance identifies the exact helper used for this run.
The source snapshot, assessment scripts and measured compiler binaries were frozen for this run. After measurement, UTF-8 punctuation was restored in 20 comment lines across two source files. Executable lines, line counts, all other captured sources, scripts and measured binaries stayed unchanged; exact before/after hashes and changed lines are retained in `build-provenance.json`.

This run reuses only the preceding run's unchanged Pilot oracle exports, verified by source-set, runtime, helper and export hashes. All native comparisons and all timing trials were rerun. See `oracle-reuse-provenance.json`.

## Detailed semantic comparison

All 310 targets were attempted in **96 isolated, identical source sets**. Release
and Pilot files were checked byte for byte. Native uses the candidate library;
Pilot uses commit `692170b71867353b8f90341e61556f49a5beb0e5` and the 0.62.0 runtime. The semantic
source oracle uses `CheckMode.ALL` with its precomputed index disabled. The
performance run uses the published index, as described below.

- Completed comparisons: **310/310**; failures/incomplete:
  **0** (listed in `semantic-summary.json`).
- Source elements: **11,213 native**, **247,516 Pilot**.
- Raw paired results: **0 exact**, **7,745 mismatched**.
- Unpaired: **3,468 native-only**, **239,771 Pilot-only**.

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
- **17,823/21,580** checked reference fields agree after the stated projection.
- **4,811** reference fields present on only one side are outside that denominator.
- **284** metaclass differences and
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
remain under `../../../../target/assessment-2026-08-post-audit/groups/`;
`semantic-artifacts.json` records their paths and uncompressed SHA-256 digests.

## Repairs verified from the preceding audit

`repair-verification.json` retains before/after source observations for three
concrete defects. The two release constants now retain values
`0.0072973525693` and `0.000544617021487` and their
`MeasurementReferences::one` unit references. The `Expressions::z` initializer
now references its own declaration instead of the unrelated `Circular::z`.

The bounded projection deliberately remains conservative: both constant trees
are still unassessed because external library objects are not paired by that
projection, and the `z` tree still differs because Pilot's lazy wrapper is retained.
The repair evidence verifies the specific values, identifiers and paired
self-reference; it does not turn those full trees into conformance passes.

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
| kerml/src/examples/Address Book Example | 1 | 0.666 | 8.310 | 0.293 | 0.422 |
| kerml/src/examples/Simple Tests | 25 | 1.026 | 10.788 | 0.623 | 2.730 |
| sysml/src/examples/Simple Tests | 35 | 2.144 | 13.438 | 1.754 | 5.530 |
| sysml/src/examples/Vehicle Example | 4 | 2.939 | 17.029 | 2.531 | 9.017 |
| sysml/src/training/02. Part Definitions | 1 | 0.685 | 8.532 | 0.306 | 0.648 |
| sysml/src/training/20. Assignment Actions | 1 | 0.716 | 9.159 | 0.337 | 1.234 |
| sysml/src/training/23. State Definitions | 2 | 0.813 | 9.431 | 0.389 | 1.233 |
| sysml/src/training/32. Requirements | 4 | 0.831 | 9.465 | 0.454 | 1.516 |

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
javac -cp ../target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar -d ../target/assessment-2026-08-post-audit/classes tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java
python -B tools/run_release_assessment.py --out ../target/assessment-2026-08-post-audit --stage prepare
python -u -B tools/run_release_assessment.py --out ../target/assessment-2026-08-post-audit --stage semantics
python -u -B tools/run_release_assessment.py --out ../target/assessment-2026-08-post-audit --stage performance
python -B tools/summarize_release_assessment.py --root ../target/assessment-2026-08-post-audit
python -B tools/publish_release_assessment.py --root ../target/assessment-2026-08-post-audit --dest docs/conformance/2026-08-post-audit-assessment
```

The semantic stage exports up to three independent source sets concurrently; all
workers finish before sequential benchmark trials start. The runner checks hashes,
isolates source sets, retains timeout/failure records
and resumes completed semantic groups. Cached exports must declare the exact
requested support set. Use a new output directory when changing compiler inputs;
do not combine candidates.
