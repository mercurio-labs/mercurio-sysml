# Simulation KIR Profile

This profile defines the canonical executable model consumed by
`mercurio-simulation-core`. SysML/KerML KIR is normalized by
`mercurio-sysml-simulation` before execution.

## State Nodes

Canonical state:

```json
{
  "id": "state.HeatedBed.Heating",
  "label": "Heating",
  "parent_state_id": null,
  "is_initial": false,
  "is_final": false,
  "is_orthogonal": false,
  "is_history": false,
  "entry_behavior": { "actions": [] },
  "exit_behavior": { "actions": [] },
  "do_behavior": {
    "rate_integration": {
      "rates": []
    }
  }
}
```

State IDs are stable executable identities. Nested states reference another
state in the same machine through `parent_state_id`. A machine must have one
unambiguous top-level initial state.

Orthogonal states set `is_orthogonal: true`; entering one enters all initial
children. History pseudo-states set `is_history: true` and must be nested under
the composite state whose shallow history they restore.

## Transition Nodes

Canonical transition:

```json
{
  "id": "transition.HeatedBed.heating_ready",
  "source": "state.HeatedBed.Heating",
  "target": "state.HeatedBed.Ready",
  "trigger": { "kind": "change", "value": "temperature >= targetTemp" },
  "guard": { "expression_ir": {} },
  "effects": []
}
```

`source` and `target` must reference states in the same machine. More than one
transition from the same source with the same trigger kind/value is rejected as
ambiguous until explicit priority semantics are added.

When a transition targets a history pseudo-state, the engine resolves the target
to the last direct child exited from the history state's parent composite. If no
history exists, the parent composite's default child is used.

## Trigger Representation

Supported trigger kinds:

- `event`
- `signal`
- `time`
- `after`
- `change`
- `completion`

Triggered transitions must include a non-empty `value`, except completion
transitions.

## Guard Representation

Canonical guards are either:

- `ExpressionIr(value)`: self-contained expression IR.
- `RuntimeFeature(id)`: compatibility handle to a runtime calculation feature.

Adapters may parse legacy guard strings, aliases, or feature references, but
the engine must see one of these two typed forms.

## Effect And Action Representation

The core effect model is typed:

```rust
enum SimulationEffect {
    Assign(AssignEffect),
    EmitSignal(SignalEffect),
    Log(LogEffect),
}
```

Rates are state behavior, not transition effects:

```json
{
  "kind": "rate_integration",
  "rates": [
    { "feature": "temperature", "rate_feature": "heatRate" },
    { "feature": "temperature", "rate_expr": { "kind": "binary" } },
    { "feature": "temperature", "rate_per_second": 2.3 }
  ]
}
```

Legacy transition `rate` effects are compatibility-only and should normalize
into state behavior or stay on the legacy overlay path.

## Analysis-Case Scenario Shape

`AnalysisCaseDefinition` is SysML-specific and belongs in
`mercurio-sysml-simulation`. Extraction produces a generic
`ConcurrentSimulationScenario`:

```text
sysml-simulation::scenario_from_analysis_case(...) -> SimulationScenario
simulation-core::run(...)
```

The core engine must not know how analysis cases are encoded in SysML KIR.

## Initial Values And Assumptions

Initial values are keyed by explicit `(subject_id, feature)` tuples. Adapters
must resolve authored names such as `bed.temperature` before execution. The
engine does not guess subject IDs.

## Trace Identity Rules

Trace channel IDs are rendered as `{subject_id}.{feature}`. State snapshots are
keyed by subject ID and contain active state IDs. Transition events carry the
canonical transition ID and a trigger rendering.

## Validation

`validate_simulation_model` rejects:

- missing machines
- machines without states
- missing or multiple top-level initial states
- state parent references outside the machine
- transition source/target references outside the machine
- missing trigger values for triggered transitions
- ambiguous transitions from the same source for the same trigger kind/value
