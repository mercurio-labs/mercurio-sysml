# Pilot Bootstrap

Mercurio extraction tools read the vendored SysML v2 Pilot Implementation
checkout but never write source changes into it. The pinned Pilot commit is
recorded in `resources/pilot.lock.json`; release extraction must use that clean
commit unless a task explicitly updates the lock and reviews the drift.

## Requirements

- JDK 17 or newer on `PATH` for Java shim increments.
- Maven wrapper from the Pilot checkout for rebuilding the interactive jar.
- A clean Pilot checkout at the commit in `resources/pilot.lock.json`.

## Build the Pilot Interactive Jar

From the Pilot checkout root:

```powershell
.\mvnw -pl org.omg.sysml.interactive -am package
```

The extraction tools look for
`org.omg.sysml.interactive\target\org.omg.sysml.interactive-*-all.jar`.
Generated Pilot `target\` files are build detritus and must not be committed.

## Dirty Checkout Policy

Release tools reject dirty Pilot checkouts by default:

```powershell
cargo run -p mercurio-tools --features legacy-pilot-tools --bin build_stdlib_release -- --pilot-root ..\external\SysML-v2-Pilot-Implementation
cargo run -p mercurio-tools --bin qualify_pilot_release
```

Use `--allow-dirty` for `build_stdlib_release` or `import_pilot_stdlib`, and
`--allow-dirty-pilot` for `qualify_pilot_release`, only for non-release debug
regeneration. A release bundle from a dirty Pilot checkout needs a written
waiver in the bundle provenance.

## Static Grammar Extraction

PX-1 starts with the static grammar extractor:

```powershell
cargo run -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_grammar -- --profile-id sysml-2.0-pilot-2026-04 --pilot-root ..\external\SysML-v2-Pilot-Implementation --out resources\metamodels\sysml-2.0-pilot-2026-04\grammar.extract.json
```

Use `--check` to compare a checked-in `grammar.extract.json` with a fresh
extraction. The check normalizes `source.extracted_at_utc`; all extracted
grammar facts must otherwise match. Use `--allow-dirty` only for local debug
captures from a dirty Pilot checkout.

## Static Metamodel Extraction

PX-2 extracts the abstract syntax from the Pilot Ecore files and records the
JSON schema inputs as cross-check provenance:

```powershell
cargo run -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_metamodel -- --profile-id sysml-2.0-pilot-2026-04 --pilot-root ..\external\SysML-v2-Pilot-Implementation --out resources\metamodels\sysml-2.0-pilot-2026-04\metamodel.extract.json
```

Use `--check` to compare a checked-in `metamodel.extract.json` with a fresh
extraction. The check normalizes `source.extracted_at_utc`; all extracted
metaclasses, generalizations, structural features, and containment features
must otherwise match. Use `--allow-dirty` only for local debug captures from a
dirty Pilot checkout.

Audit a generated metamodel extract with:

```powershell
cargo run -p mercurio-tools --features legacy-pilot-tools --bin audit_metamodel_extract -- --profile-id sysml-2.0-pilot-2026-04 --deny-warnings
```

The audit fails structural extract errors, unresolved metaclass/feature
references, PX-2 spot-check regressions, and uncovered `sysml_field_specs()`
gaps. Current field registry aliases, KIR shape overrides, and Mercurio-only
fields are classified in `mappings\field_specs.overlay.json`; every overlay
entry must carry a reason, and aliases/shape overrides cite Pilot features.

Generate the field registry sidecar with:

```powershell
cargo run -p mercurio-tools --features legacy-pilot-tools --bin generate_sysml_field_specs -- --profile-id sysml-2.0-pilot-2026-04
```

Use `--check` to compare a checked-in
`mappings\field_specs.generated.json` with a fresh generation. The check
normalizes `source.generated_at_utc`; all field entries, classifications, and
source file hashes must otherwise match.

`mercurio-sysml` consumes the checked-in generated field registry at build
time. Its build script reads each registry profile's
`mappings\field_specs.generated.json`, verifies the profiles agree on the
runtime field table, emits `OUT_DIR\sysml_field_specs.rs`, and uses that same
field table while materializing generated stdlib constants. After changing the
field overlay or generated sidecar, run:

```powershell
cargo check -p mercurio-sysml
cargo test -p mercurio-sysml semantic_profile::tests:: --lib
```

## Static Validator Extraction

PX-4 inventories the Pilot Xtend validators without attempting to translate
their logic:

```powershell
cargo run -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_validators -- --profile-id sysml-2.0-pilot-2026-04
```

Use `--check` to compare a checked-in `validators.extract.json` with a fresh
generation. The check normalizes `source.extracted_at_utc`; all issue
constants, active `@Check` signatures, validation comments, diagnostic call
sites, helper-call references, classifications, and `pending` ledger rows must
otherwise match. Use `--allow-dirty` only for local debug captures from a dirty
Pilot checkout.

The current Pilot validator inventory records 111 active checks, 467 issue
constants, and 150 direct diagnostic call sites. The expressions validator is
included in provenance and source hashing, but currently contributes no active
checks.

## Implicit Semantics Extraction

PX-3 observes the Pilot's implicit relationship materialization by running the
Java exporter twice over a corpus: first with `ElementUtil.transformAll(...,
false)`, then with implicit elements enabled. The Rust wrapper emits aggregate
counts plus bounded example rows so the checked-in artifact stays reviewable:

```powershell
cargo run -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_implicit_semantics -- --profile-id sysml-2.0-pilot-2026-04 --allow-dirty
```

Use `--check` to compare a checked-in `implicit_semantics.extract.json` with a
fresh run. The check normalizes `source.extracted_at_utc`; all corpus
selection, source hashes, aggregate implicit element/relationship counts, and
recorded examples must otherwise match.

The default corpus is `small` from
`crates\mercurio-tools\corpus\pilot_corpus.seed.json`. Current observed counts:
5 cases, 34,798 added input-model implicit elements, and 261,751 added
input-model-sourced relationships. The artifact records full aggregate buckets
and caps example rows per case with
`--max-recorded-elements-per-case` and
`--max-recorded-relationships-per-case`.

## Pilot Conformance Harness

PX-5 compares Mercurio and the Pilot over the same source corpus using the
Pilot diagnostics batch exporter:

```powershell
cargo run -p mercurio-tools --features legacy-pilot-tools --bin pilot_conformance_harness -- --corpus small --out target\pilot-conformance-small.json
```

The report records per-file parse verdict agreement plus diagnostic multiset
comparison: matched diagnostics, Mercurio-only diagnostics, and Pilot-only
diagnostics. The `compare_pilot_compile_errors` command remains available as a
compatibility name for the same report logic. The harness is jar-gated; use it
after the interactive jar exists and expect large corpora to run substantially
slower than static extractors. Java oracle runs are timeout-bounded; use
`--java-timeout-seconds N` to adjust the default 300 second limit for a single
Pilot exporter invocation.

The harness compiles Mercurio cases against the same `load_sysml_baseline()`
path used by production SysML compilation. The current `small` corpus smoke
passes with 5/5 parse verdict matches and zero diagnostic deltas.

## One-Command Drift Pipeline

PX-6 provides a single orchestrator for release-bump drift checks:

```powershell
cargo run -p mercurio-tools --features legacy-pilot-tools --bin pilot_release_bump -- --profile-id sysml-2.0-pilot-2026-04 --corpus small --out target\pilot-release-bump-drift.json
```

By default the command runs in check mode: all generated artifacts must match
fresh extraction. Pass `--write` to regenerate artifacts in-place for a reviewed
candidate bump. The command rejects dirty Pilot checkouts unless
`--allow-dirty` is supplied for local debug work.
Use `--java-timeout-seconds N` to pass a timeout through to the conformance
harness stage.

For environments without a responsive Pilot jar, static extraction/audit stages
can still be exercised with:

```powershell
cargo run -p mercurio-tools --features legacy-pilot-tools --bin pilot_release_bump -- --profile-id sysml-2.0-pilot-2026-04 --skip-jar-stages --allow-dirty --out target\pilot-release-bump-static-drift.json
```

The orchestrator writes both JSON and Markdown drift reports. The report lists
each extraction/audit/conformance stage, command status, duration, output tails,
and the current Mercurio SysML git status for review.
