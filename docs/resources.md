# Resources

Language resources are owned by the public crate under
`crates/mercurio-sysml/resources/`. Keeping them inside that package makes the
crates.io archive self-contained.

## Kernel baseline

`crates/mercurio-sysml/resources/kernel/kerml-kernel.kir.json` is the
prebuilt kernel baseline loaded by the `kerml` module and merged into SysML
environments.

## Metamodel bundles

`crates/mercurio-sysml/resources/metamodels/` contains the versioned metamodel
bundles. Each bundle carries its descriptor, language profile, provenance,
lowering mappings, generated standard-library KIR, rulepack, and release locks.

## Generated files

Generated resources are checked in so clients can create environments without
running maintainer tooling. Regenerate them only when intentionally updating a
metamodel bundle, standard-library release, or mapping rule set. Maintainer
tools continue to treat the canonical crate resource directory as the source of
truth.
