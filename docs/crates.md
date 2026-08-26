# Crate and module architecture

## Public crate

`mercurio-sysml` is the only publishable package in this workspace. It owns
the complete KerML/SysML language implementation and exposes focused modules:

- `language_frontend`: lowering, resolution, transpilation, and mapping support;
- `kerml`: KerML parsing, compilation, baseline loading, and language service;
- `requirements`: requirement traceability and analysis capabilities;
- `simulation`: SysML simulation adapter and execution APIs;
- `resources`: embedded versioned metamodel, mapping, rulepack, and standard-library data.

The existing root facade remains the recommended entry point for common SysML
parsing, compilation, authoring, analysis, and simulation workflows.

## Compatibility packages

The workspace retains `mercurio-language-frontend`, `mercurio-kerml`,
`mercurio-requirements`, `mercurio-simulation`, and
`mercurio-sysml-resources` as thin re-export shims. They preserve source-tree
compatibility for internal consumers while those consumers migrate to
`mercurio-sysml` modules. Every shim has `publish = false`; no
implementation or resource data lives in those packages.

## Maintainer packages

`mercurio-tools` owns audits, release generation, corpus comparisons, and
resource import/export utilities. `mercurio-sysml-cli` is the repository CLI.
Both are non-publishable and may consume the compatibility shims during the
migration.
