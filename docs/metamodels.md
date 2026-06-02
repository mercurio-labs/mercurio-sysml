# Metamodels

The SysML repository treats a metamodel as a versioned resource bundle. A bundle
contains the descriptor, lowering mappings, provenance, and prebuilt KIR
libraries needed to compile source text consistently.

Current bundle:

- ID: `sysml-2.0-metamodel-0.57.0`
- Status: `latest`
- Descriptor: `resources/metamodels/sysml-2.0-metamodel-0.57.0/metamodel.json`

## Descriptor Contract

Each metamodel descriptor provides:

- `id`: stable bundle identifier used by clients.
- `display_name`: human-readable label.
- `sysml_version`: source language version.
- `kerml_version`: kernel language version.
- `metamodel_version`: upstream metamodel version represented by the bundle.
- `status`: `latest`, `supported`, or `deprecated`.
- `profile_path`: profile metadata.
- `mappings_path`: lowering and semantic mapping resources.
- `stdlib_path`: full standard library KIR.
- `sysml_delta_path`: SysML delta KIR merged with the kernel baseline.
- `provenance_path`: source and generation provenance.
- `legacy_ids`: compatibility aliases accepted by `metamodel_resource`.

## Client Contract

Use explicit IDs for reproducible applications:

```rust
use mercurio_sysml::{SysmlEnvironment, SYSML_2_0_METAMODEL_057_ID};

let env = SysmlEnvironment::for_metamodel(SYSML_2_0_METAMODEL_057_ID)?;
```

Use `latest_metamodel()` or `SysmlEnvironment::latest_metamodel()` for tools and
experiments that intentionally track the newest supported bundle.

## Adding A Version

Adding a metamodel version should be a directory-level change:

- add a new `resources/metamodels/<id>/` directory,
- include descriptor, profile, mappings, provenance, and generated KIR resources,
- mark only one descriptor as `latest`,
- preserve compatibility aliases only when the new bundle intentionally replaces
  an older identifier,
- add tests that list, select, load, and compile with the new version.
