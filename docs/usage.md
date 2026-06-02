# Usage

## Create A SysML Environment

`SysmlEnvironment` packages the selected metamodel resource, language registry,
and baseline KIR document. Clients normally use the latest metamodel convenience
constructor.

```rust
use mercurio_sysml::SysmlEnvironment;

let env = SysmlEnvironment::latest_metamodel()?;
let document = env.compile_text(
    "package Demo { part def Vehicle; part vehicle : Vehicle; }",
    "demo.sysml",
)?;

println!("compiled {} KIR elements", document.elements.len());
```

## Inspect Available Metamodels

Use the free functions when a host wants to list choices before constructing an
environment.

```rust
use mercurio_sysml::{available_metamodels, latest_metamodel};

for metamodel in available_metamodels()? {
    println!(
        "{}: SysML {}, metamodel {}, {:?}",
        metamodel.id,
        metamodel.sysml_version,
        metamodel.metamodel_version,
        metamodel.status,
    );
}

let latest = latest_metamodel()?;
println!("latest metamodel: {}", latest.id);
```

## Select A Specific Metamodel

Use `for_metamodel` when reproducibility matters and the application should not
silently move to a newer metamodel.

```rust
use mercurio_sysml::{SysmlEnvironment, SYSML_2_0_METAMODEL_057_ID};

let env = SysmlEnvironment::for_metamodel(SYSML_2_0_METAMODEL_057_ID)?;
assert_eq!(env.metamodel().id, SYSML_2_0_METAMODEL_057_ID);
```

## Register The Language Service Manually

Hosts that manage their own language registry can register the module directly.

```rust
use std::path::Path;

use mercurio_language_contracts::{LanguageRegistry, SemanticCompileStatus};
use mercurio_sysml::{SysmlLanguageModule, load_sysml_baseline};

let mut registry = LanguageRegistry::new();
registry.register(SysmlLanguageModule);

let baseline = load_sysml_baseline()?;
let report = registry.compile_path(
    Path::new("demo.sysml"),
    "package Demo { part def Vehicle; }",
    &baseline,
);

assert_eq!(report.status, SemanticCompileStatus::Ok);
let document = report.document.expect("successful compile returns KIR");
```
