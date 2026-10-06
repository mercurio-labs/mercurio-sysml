# 2026-08 semantic and performance assessment

Executed 2026-09-17T21:40:14.443681+00:00. This assesses
the candidate worktree; **it is not release qualification**. Source, library,
runtime and binary hashes are recorded alongside this report. The native compiler and Pilot runtime were held fixed during measurement.
Exporter-only cache optimizations were introduced during the semantic run and
verified against the 35-target SysML Simple Tests group: 24,284 exported elements
and 198,690 relationships were unchanged after explicitly mapping per-process
UUID references. The helper history and equivalence counts are in
`build-provenance.json`; all performance trials use the final helper.

## Detailed semantic comparison

All 310 targets were attempted in **96 isolated, identical source sets**. Release
and Pilot files were checked byte for byte. Native uses the candidate library;
Pilot uses commit `692170b71867353b8f90341e61556f49a5beb0e5` and the 0.62.0 runtime. The semantic
source oracle uses `CheckMode.ALL` with its precomputed index disabled. The
performance run uses the published index, as described below.

- Completed comparisons: **310/310**; failures/incomplete:
  **0** (listed in `semantic-summary.json`).
- Source elements: **11,170 native**, **247,516 Pilot**.
- Raw paired results: **0 exact**, **7,635 mismatched**.
- Unpaired: **3,535 native-only**, **239,881 Pilot-only**.

These are not counts of independent semantic bugs. Pilot materializes membership,
typing, redefinition and expression objects that native KIR often stores as
properties. Identifiers, scalar/list forms, derived/default fields, order and
duplicate relationships also differ. Anonymous objects on one source line can be
paired imperfectly. No tolerance or exclusion turns these differences into passes.

Separate triage matches only unique `(declared name, start line)` pairs within a
source file. It maps reference IDs through observed pairs in the same source
group and compares six reference fields as sets. It does not declare absent
fields, anonymous objects or missing expressions equivalent. Raw reports remain:

- **6,547** uniquely paired named declarations.
- **17,504/21,331** checked reference fields agree after the stated projection.
- **4,714** reference fields present on only one side are outside that denominator.
- **374** metaclass differences and
  **171** direct Boolean differences.
- **82/497** matched declarations with a Pilot `FeatureValue` lack initializer IR on that native declaration. Inspect alternate lowering before treating every case as lost semantics.
- **714** missing/different nonempty Pilot short-name or direction fields.
- Ambiguous named anchors excluded from declaration pairing: **1**; **252** Pilot named anchors have no native pair (not automatically lost semantics).
- **29** literal initializer values compared;
  **0** literal-value differences.

Examples requiring implementation review:

- `sysml/src/examples/Simple Tests/ConstraintTest.sysml:24`, `massAnalysis`: **metaclass** differs. Native `AssertUsage`; Pilot `AssertConstraintUsage`.
- `sysml/src/examples/Simple Tests/ConjugationTest.sysml:15`, `p1`: **metaclass** differs. Native `ReferenceUsage`; Pilot `PortUsage`.
- `kerml/src/examples/Simple Tests/Dependencies.kerml:11`, `Use`: **metaclass** differs. Native `Feature`; Pilot `Dependency`.
- `sysml/src/examples/Simple Tests/PartTest.sysml:24`, `B1`: **is_variable** differs. Native `True`; Pilot `False`.
- `kerml/src/examples/Simple Tests/Expressions.kerml:14`, `b`: **initializer** differs. Native `None`; Pilot `['InputModel::kerml/src/examples/Simple Tests/Expressions.kerml::14::OperatorExpression']`.

KerML initializer loss is independently confirmed by `skip_expression_tail` in
the native parser. Assertion metaclasses and occurrence variability also require
correction. `semantic-differences.json` records every triaged source/line/property
difference; `semantic-cases.csv` records per-file counts.

### Coverage and oracle limits

The exporter includes every source EAttribute (including literal values and
operators), every stored non-container source EReference, and its existing
derived-reference whitelist. Every exported source element is retained by the
strict snapshot. This is not an exhaustive proof for every computed EMF reference
or OMG constraint. The projection does not establish nonliteral expression-tree
equivalence, multiplicity semantics, or behavioral execution. Registry-query
fallbacks may retain direct properties without
establishing full derived-value coverage. Full compressed snapshots and raw diffs
remain under `../../../../target/assessment-2026-08-detailed/groups/`;
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
| kerml/src/examples/Address Book Example | 1 | 0.985 | 10.464 | 0.369 | 0.604 |
| kerml/src/examples/Simple Tests | 25 | 0.975 | 12.081 | 0.576 | 3.286 |
| sysml/src/examples/Simple Tests | 35 | 1.772 | 13.664 | 1.388 | 5.596 |
| sysml/src/examples/Vehicle Example | 4 | 2.372 | 16.623 | 1.993 | 8.843 |
| sysml/src/training/02. Part Definitions | 1 | 0.788 | 10.487 | 0.358 | 0.806 |
| sysml/src/training/20. Assignment Actions | 1 | 0.705 | 8.990 | 0.328 | 1.249 |
| sysml/src/training/23. State Definitions | 2 | 0.810 | 10.161 | 0.389 | 1.344 |
| sysml/src/training/32. Requirements | 4 | 0.810 | 9.625 | 0.419 | 1.515 |

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
javac -cp ../target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar -d ../target/assessment-2026-08-detailed/classes tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java
python -B tools/run_release_assessment.py --stage prepare
python -u -B tools/run_release_assessment.py --stage semantics
python -u -B tools/run_release_assessment.py --stage performance
python -B tools/summarize_release_assessment.py
python -B tools/publish_release_assessment.py
```

The semantic stage exports up to three independent source sets concurrently; all
workers finish before sequential benchmark trials start. The runner checks hashes,
isolates source sets, retains timeout/failure records
and resumes completed semantic groups. Cached exports must declare the exact
requested support set. Use a new output directory when changing compiler inputs;
do not combine candidates.
