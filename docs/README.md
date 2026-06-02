# Mercurio SysML Documentation

Mercurio SysML is the concrete language repository for SysML-family source text.
It owns parsing, recovery, language-specific lowering, bundled metamodel
resources, and tools that maintain those resources.

Foundation owns the language-neutral model substrate. This repository owns the
SysML language service that turns source text into KIR for that substrate.

## Sections

- [Usage](usage.md): create a SysML environment, inspect metamodel versions, and compile source text.
- [Metamodels](metamodels.md): layout and versioning contract for bundled metamodel resources.
- [Crates](crates.md): intent of each crate in this repository.
- [Resources](resources.md): generated libraries, mappings, provenance, and release files.

## Boundary

Keep SysML-specific concepts here:

- parser and recovery behavior,
- concrete source grammar,
- metamodel descriptors and compatibility IDs,
- standard library bundles,
- language-specific lowering rules,
- conformance and comparison tooling.

Keep foundation language-neutral:

- KIR schema and validation,
- graph projection,
- runtime/query/session/package APIs,
- language-service traits and reports,
- host-facing contracts that can work with any language that emits KIR.
