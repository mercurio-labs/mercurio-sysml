# Mercurio SysML

KerML and SysML language libraries for Mercurio. These crates implement language services that compile source text to KIR and depend on the language-neutral Mercurio core libraries.

This repository owns the concrete SysML-family language implementation: source parsing, semantic compilation, metamodel resources, bundled standard libraries, and maintainer tooling. Language-neutral model storage and runtime services live in `mercurio-foundation`.

## Quick Example

```rust
use mercurio_sysml::SysmlEnvironment;

let env = SysmlEnvironment::latest_metamodel()?;
let document = env.compile_text(
    "package Demo { part def Vehicle; part vehicle : Vehicle; }",
    "demo.sysml",
)?;

println!("compiled {} KIR elements", document.elements.len());
```

## Metamodel Versions

```rust
use mercurio_sysml::{available_metamodels, latest_metamodel};

for metamodel in available_metamodels()? {
    println!("{} {:?}", metamodel.id, metamodel.status);
}

let latest = latest_metamodel()?;
println!("latest: {}", latest.id);
```

## Documentation

- [Usage](docs/usage.md)
- [Metamodels](docs/metamodels.md)
- [Crates](docs/crates.md)
- [Resources](docs/resources.md)

## Build

```powershell
cargo build
```

