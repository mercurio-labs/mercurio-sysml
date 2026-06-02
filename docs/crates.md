# Crates

## `mercurio-sysml`

The public SysML facade. It owns:

- source parsing and recovering parse reports,
- semantic compilation to KIR,
- `SysmlLanguageModule` for registry-based hosts,
- `SysmlEnvironment` for metamodel-aware clients,
- metamodel discovery and resource resolution.

Use this crate when an application needs to accept SysML source text and produce
foundation KIR.

## `mercurio-kerml`

The kernel language facade used by the SysML baseline. It owns:

- kernel parsing,
- compilation to KIR,
- baseline loading,
- `KermlLanguageModule` registration.

SysML environments register this module alongside `SysmlLanguageModule` because
the SysML baseline is built on the kernel language model.

## `mercurio-language-frontend`

Shared frontend implementation used by the concrete language crates. It owns the
language-specific lowering pipeline, resolver, emitter, formatting, and mapping
support needed before KIR is handed to foundation services.

## `mercurio-tools`

Maintainer tooling for language resources. It owns audits, release generation,
corpus comparisons, and resource import/export utilities. Applications should not
depend on this crate for ordinary runtime behavior.
