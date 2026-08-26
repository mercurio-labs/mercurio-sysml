# Releasing Mercurio SysML

Mercurio SysML is one release unit and one crates.io package:
`mercurio-sysml`. The former implementation packages
`mercurio-sysml-resources`, `mercurio-language-frontend`, `mercurio-kerml`,
`mercurio-requirements`, and `mercurio-simulation` remain in the workspace
only as non-publishable compatibility shims. Their implementations live in
focused modules of `mercurio-sysml`. `mercurio-tools` and
`mercurio-sysml-cli` are also non-publishable.

The public crate uses the version in `[workspace.package]`. Its Foundation
dependency retains both a local `path` for paired development and a registry
`version` for the packaged crate.

## Prerequisite

Publish the matching `mercurio-foundation` release first and confirm that it
is visible on crates.io. `FOUNDATION_REF` in
`.github/workflows/crates-release.yml` identifies the Foundation source
release used to qualify the local path dependency.

## Qualification

With the compatible `mercurio-foundation` repository checked out beside this
repository:

```powershell
cargo test --workspace --locked
$env:MERCURIO_REPO_ROOT = "..\mercurio-foundation"
cargo run -p mercurio-tools --bin check_repo_boundaries -- --manifest ..\mercurio-foundation\repo-boundaries.json --strict
$env:RUSTDOCFLAGS = "-D warnings"
cargo doc -p mercurio-sysml --all-features --no-deps --locked
cargo package -p mercurio-sysml --locked
```

The release workflow also extracts the generated `.crate` archive and runs the
canonical crate's all-feature tests from that archive. This verifies that the
published artifact contains its language modules and versioned resources and
does not rely on sibling workspace packages.

## Release

Before pushing a tag:

1. Confirm `mercurio-foundation` at the matching version is published.
2. Merge the qualified release commit to `main`.
3. Create and push `sysml-v<version>`, for example `sysml-v0.86.0`.

The workflow publishes only `mercurio-sysml`. A manual dispatch can safely
resume a release; it exits successfully when that version is already present.
