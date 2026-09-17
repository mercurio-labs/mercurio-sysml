# Release sample audit

This audit measures native compilation and Pilot acceptance on the same sample
source sets. A passing run is not certification of full OMG conformance or
proof that both compilers produce equivalent semantic graphs.

Build the strict native probe from this workspace:

    cargo build --locked -p mercurio-tools --bin audit_release_compile

Obtain clean, matching tagged checkouts of Systems-Modeling/SysML-v2-Release and
Systems-Modeling/SysML-v2-Pilot-Implementation. Download the matching Jupyter
kernel ZIP from the Pilot release and extract its fat JAR. For the 2026-08 release,
the JAR is jupyter-sysml-kernel-0.62.0-all.jar.

Run with Python 3.10+ and JDK 17+:

    python -B tools/audit_release_samples.py --release-root <release-checkout> --pilot-root <pilot-checkout> --pilot-jar <kernel-all.jar> --out <fresh-output-directory>

The default discovers every .sysml and .kerml file under sysml/src and kerml/src.
It includes examples, training and validation sources; it does not claim to
compile the normative standard-library sources or the separate Xpect suites.
The report fingerprints all 310 release sample files for release 2026-08,
all input dependencies, Pilot's textual library, the JAR, native resources,
source revisions, audit code, and the native binary. Dirty upstream checkouts
and native standard-library environment overrides are rejected.

Each source set comprises the target, sibling model files, and transitive
dependencies from crates/mercurio-tools/corpus/pilot_corpus.seed.json. All
source sets are recorded in corpus.json. Both engines get exactly the same
file set. Sets with different support files are isolated in different JVMs.
Pilot validates the smaller targets first so a large target cannot prevent
results for all the other targets being recorded.

The native probe uses the production strict parser, resolver and lowering
entry points with the shipped embedded baseline. Target syntax errors,
support-file syntax errors, and resolution/lowering errors are distinct.
An error in a support file blocks that source set: these cascades must not
be mistaken for independent syntax failures in every affected target.
A native pass means KIR was produced without a strict compilation error;
it does not mean all metamodel well-formedness constraints were checked.

Pilot runs the actual Xtext resource validator with CheckMode.ALL, as its
interactive process does, using the release's textual standard library.
Warnings are retained but do not fail a case. The older diagnostics exporter
only reads resource errors and does not establish this semantic validation.

The audit exits nonzero for verdict mismatches, crashes, timeouts, or missing
results. A pair of rejected files is recorded separately from a pair of
accepted files. The raw JSONL and logs remain beside audit.json and audit.md.
The report explicitly leaves full specification conformance and semantic
equivalence unestablished.

Use --paths-file for an explicitly labeled subset, --workers for the number of
Pilot JVMs, --java-timeout for the limit per source set (default 300 seconds),
and --native-timeout for the native corpus limit (default 3600 seconds).

For a rerun, --pilot-cache <previous-output-directory> reuses only completed
groups with the same exact inputs and matching input, library, JAR, Pilot
revision and Java-shim fingerprints. A changed dependency set reruns that
group. Incomplete groups rerun. Native compilation always reruns.
Use a fresh output directory even when reusing Pilot results.

## Candidate specification extraction

The existing extractors can use a candidate lock without changing the shipped
runtime lock. Set MERCURIO_PILOT_LOCK to an absolute path to a JSON lock using
the same schema as crates/mercurio-sysml/resources/pilot.lock.json. Its commit
must match the clean candidate Pilot checkout. Unset it after extraction.

    cargo run --locked -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_grammar -- --pilot-root <pilot-checkout> --profile-id <candidate-id> --out <grammar.extract.json>
    cargo run --locked -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_metamodel -- --pilot-root <pilot-checkout> --profile-id <candidate-id> --out <metamodel.extract.json>
    cargo run --locked -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_validators -- --pilot-root <pilot-checkout> --profile-id <candidate-id> --out <validators.extract.json>

The environment override selects a different immutable pin; it does not bypass
the clean-checkout or commit checks. Candidate extraction does not promote the
default standard library or regenerate compiler mappings.

## Verification

    cargo test --locked -p mercurio-tools --bin audit_release_compile --lib
    python -B -m unittest discover -s tools -p test_audit_release_samples.py
    cargo test --locked -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_grammar --bin extract_pilot_metamodel --bin extract_pilot_validators

The 2026-08 evidence and limitations are in conformance/2026-08/README.md.
