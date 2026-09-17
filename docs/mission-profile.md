# Bounded thermal mission profile

Load [MercurioMissions.sysml](../crates/mercurio-sysml/resources/profiles/MercurioMissions.sysml) with your model, or place its package before your model package in one source file. It is an authored profile, not automatically installed into the standard library. Use the fully qualified annotation names below inside the analysis definition.

```sysml
@Mercurio::Missions::Clock {
    maxTime = 10.0;
    fixedStep = 0.5;
    sampleInterval = 0.5;
    maxSteps = 100;
}
@Mercurio::Missions::InitialStimulus {
    subject = "chamber";
    trigger = "start";
}
@Mercurio::Missions::Termination {
    onAllSatisfied = true;
    onAnyViolated = true;
    onBlocked = true;
}
```

Clock values are finite literal seconds: maxTime is nonnegative; fixedStep and sampleInterval are positive. Optional maxSteps and changeLoopLimit are positive integers. Invalid or unknown fields fail with mission.metadata.invalid. Numeric arithmetic in metadata is not evaluated. InitialStimulus names exactly one analysis subject by ID or unique label and replaces inferred initial events with one event at the start. Scheduled stimulus sequences are outside this profile.

Termination flags default to false. Requirement stopping uses the same sampled evaluator as the final requirement report. Empty, unsupported or unevaluated requirements cannot establish satisfaction. A violation takes precedence over all-satisfied; a configured execution-blocked stop takes precedence over requirement outcomes. Final-state completion takes precedence when all subjects have reached machine-level final states. Execution health, stop reason and requirement verdict are separate fields.

The thermal fixture starts at 20 degrees with a target of 80 and a five-second deadline. A heat rate of 10 violates the deadline; 20 satisfies it with a three-second witness. With the policy above both runs stop at five seconds, because this evaluator requires an exact deadline sample. An earlier final-state stop leaves that deadline unevaluated. There is no interpolation or proof claim.

The bounded timing syntax supports literal seconds and milliseconds (including 2[s] and 2000[ms]). An after trigger measures time since entering its source state; an at trigger measures mission time. An already-due absolute trigger fires without another clock increment. Unsupported timed expressions are rejected with transition.time_unsupported. The scheduler retains deterministic phase order: scripted events and immediate transitions before due timed transitions; transitions within a phase retain the projected order. This is a bounded implementation contract, not a claim of complete SysML execution semantics.

Reports carry trace_schema mercurio.simulation.trace.v1, a configuration stamp with the effective clock, step budget and stopping policy, and mission_summary with execution health, termination, recorded end time, frame count and verdict counts. Evidence strength is sampled_simulation. Legacy traces may lack the configuration stamp. Numbers use Rust f64 and serde_json serialization; no cross-host canonical digest or tolerance qualification is claimed. The stamp does not contain a full source-content hash.

Regression coverage runs the textual model, applies the Termination annotation through checked authoring, and runs again. The native MCP probe persists the edit to a proposal worktree, reopens it and compares the trace while checking that the original source remains unchanged.
