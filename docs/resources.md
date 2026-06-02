# Resources

Language resources live under `resources/`.

## Kernel Baseline

`resources/kernel/kerml-kernel.kir.json` is the prebuilt kernel baseline loaded
by the kernel language crate and merged into SysML environments.

## Metamodel Bundles

`resources/metamodels/sysml-2.0-metamodel-0.57.0/` contains the current
metamodel bundle:

- `metamodel.json`: version descriptor consumed by `available_metamodels`.
- `profile.json`: language profile metadata.
- `provenance.json`: generation source and traceability metadata.
- `mappings/`: seed files for lowering and semantic defaults.
- `stdlib/`: generated standard library KIR, rulepack, release lock, and source
  export files.

## Generated Files

Generated resources are checked in so clients can create environments without
running maintainer tooling. Regenerate them only when intentionally updating a
metamodel bundle, standard library release, or mapping rule set.
