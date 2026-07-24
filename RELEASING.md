# Releasing Mercurio SysML

Mercurio SysML is one release unit with `mercurio-sysml` as its public
crates.io entry point. `mercurio-sysml-resources`,
`mercurio-language-frontend`, `mercurio-kerml`, `mercurio-requirements`, and
`mercurio-simulation` are published implementation packages.
`mercurio-tools` and `mercurio-sysml-cli` remain excluded from crates.io
publication.

All publishable SysML packages use the version in `[workspace.package]`.
Internal and Foundation dependencies retain both a local `path` and a registry
`version`.

## Prerequisite

Publish the required Mercurio Foundation release first and confirm that all of
its packages are visible on crates.io. The `FOUNDATION_REF` in
`.github/workflows/crates-release.yml` identifies the Foundation source release
used to qualify the local path dependencies.

## Qualification

With the compatible `mercurio-foundation` repository checked out beside this
repository:

```powershell
cargo test --workspace --locked
cargo run -p mercurio-tools --bin check_repo_boundaries -- --manifest ..\mercurio-foundation\repo-boundaries.json --strict
cargo doc --workspace --no-deps --locked
cargo package --workspace --exclude mercurio-tools --exclude mercurio-sysml-cli --no-verify --locked
```

## First Release

Before pushing the first tag:

1. Create a crates.io API token with permission to publish new crates.
2. Add it to the `mercurio-sysml` GitHub repository as the
   `CARGO_REGISTRY_TOKEN` Actions secret.
3. Protect the optional `crates-io` GitHub environment if release approval is
   desired.
4. Merge the qualified release commit to `main`.
5. Create and push `sysml-v<version>`, for example `sysml-v0.85.0`.

The workflow is resumable and waits for registry-index propagation between
dependent packages. After the first release, configure crates.io Trusted
Publishing and replace the long-lived token step with the crates.io OIDC
action.

## Publish Order

1. `mercurio-sysml-resources`
2. `mercurio-language-frontend`
3. `mercurio-requirements`
4. `mercurio-kerml`
5. `mercurio-sysml`
6. `mercurio-simulation`
