# mercurio-sysml — Agent Orientation

KerML and SysML v2 language libraries. Owns the concrete language implementation: source parsing, semantic compilation, metamodel resources, and the SysML simulation engine. Language-neutral services live in `mercurio-foundation`.

---

## Crate and module architecture

| Package/module | Responsibility |
|----------------|----------------|
| `mercurio-sysml` | Only publishable package; SysML facade and all language modules |
| `mercurio_sysml::language_frontend` | Source text → parse tree → KIR lowering pipeline |
| `mercurio_sysml::kerml` | KerML parser, compiler, and baseline services |
| `mercurio_sysml::requirements` | Requirements traceability and coverage analysis |
| `mercurio_sysml::simulation` | SysML simulation adapter and concurrent engine |
| `mercurio_sysml::resources` | Embedded metamodel, mapping, rulepack, and stdlib resources |
| `mercurio-tools` | Non-publishable maintainer tooling |
| `mercurio-sysml-cli` | Non-publishable CLI entry point |

---

## Simulation Engine

The simulation engine is the most active development area. Before modifying it read `../docs/simulation-engine-plan.md`.

Key files:

```
crates/mercurio-sysml/src/behavior/simulation.rs   — run_analysis_case() -> CapabilityRunReport, run_concurrent_simulation(), SimulationTrace artifact payloads
crates/mercurio-sysml/src/behavior/capability.rs   — capability introspection
crates/mercurio-sysml/src/compile.rs               — SysML text → KirDocument
```

**What is implemented:** multi-subject concurrent simulation on a shared time axis, `accept after` time triggers, `accept when` change triggers, `accept <signal>` event triggers, linear rate effects, assign effects, guard evaluation via `runtime.evaluate()`, `assume constraint` initial values.

**What is NOT implemented** (do not claim these work):
- State `do` actions as continuous rate source
- Nonlinear ODE (currently Euler with constant rate, 1-second steps)
- Zero-crossing detection (guards evaluated post-step)
- Cross-subject signal routing
- Hierarchical/orthogonal state machines beyond one nesting level
- Binding connectors in KIR
- Activity execution (action def / control flow / data flow)
- Parametric constraint solving

---

## Public API for External Callers

```rust
use mercurio_sysml::{list_analysis_cases, run_analysis_case, SysmlEnvironment};
// Runtime comes from mercurio-foundation: Runtime::from_graph(graph)
// run_analysis_case(runtime, analysis_case_id, run_id) returns CapabilityRunReport.
// SimulationTrace is the payload of the simulation_trace artifact, not the
// public analysis-case return type.
```

---

## Build & Test

```powershell
cargo build
cargo test
```

Boundary checker (run after any `Cargo.toml` change):

```powershell
cargo run -p mercurio-tools --bin check_repo_boundaries -- --manifest ..\mercurio-foundation\repo-boundaries.json
cargo run -p mercurio-tools --bin check_repo_boundaries -- --manifest ..\mercurio-foundation\repo-boundaries.json --strict
```

---

## Key Constraints

- KIR `kind` values must match SysML v2/KerML metaclass names (e.g. `StateUsage`, `TransitionUsage`, `ActionUsage`). Do not invent proprietary extensions.
- Kernel crates must remain compilable to `wasm32-unknown-unknown` — no OS-level I/O or threading in non-tool crates.
- `mercurio-tools` is the only crate in this workspace allowed to carry dev/build tooling dependencies that would break WASM.

---

## Further Reading

- [docs/crates.md](docs/crates.md) — crate responsibilities
- [docs/simulation-kir-profile.md](docs/simulation-kir-profile.md) — KIR profile for simulation
- [docs/usage.md](docs/usage.md) — API usage examples
- [../docs/simulation-engine-plan.md](../docs/simulation-engine-plan.md) — simulation roadmap and known gaps
- [../docs/codex-python-simulation-api.md](../docs/codex-python-simulation-api.md) — active task spec (HTTP route exposure)
