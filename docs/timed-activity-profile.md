# Bounded timed activity execution (M11 increment)

Opt in with the authored Mercurio::Missions profile. This is a sequential execution
subset, not the full M11 token engine and not a general SysML conformance claim.

- Put `@Mercurio::Missions::TimedActivity { completionFeature = "completed"; }`
  inside one inline activity (`action procedure`) on the analysis subject's part definition.
- Declare a unique Boolean attribute `completed : Boolean = false` on that definition.
- Give each leaf action `@Mercurio::Missions::Duration { seconds = 5.0; }`.
  Durations must be positive finite literal seconds; expressions are rejected.
- Connect all actions in one sequence with `first warmup then hold;` successions.
- Use the existing Clock and optional Termination metadata on the analysis definition.
- Express deadline requirements using the subject's completion attribute.

See the complete [textual fixture](../crates/mercurio-sysml/tests/fixtures/timed-activity.sysml).
It runs warmup (2 seconds), hold (5), then measure (1). Completion at 8 seconds
satisfies an 8-second deadline; changing warmup to 3 seconds violates it.
Stopping observation at 6 seconds produces an unevaluated deadline.

The adapter validates the whole sequence before execution and lowers it to the
existing language-neutral timed scheduler. There is no second clock or host-side
execution implementation. One active action is represented by its authored KIR ID
in each frame's states. Start/end events carry action labels in trigger and authored
action IDs in reason; transition_id remains reserved for succession transitions.
The final internal observation state and completion edge are not authored objects
and are removed from trace presentation. Source KIR is never mutated.

Completion sets the declared Boolean output to true. No physical measurement,
heating, arbitrary action body, or external call is implied by an action's name.
The completed result remains available until the clock or termination policy
ends the run, so the common sampled deadline evaluator can observe its deadline.
Execution health, requirement verdict, and the observation stop reason are separate.
Trace schema and evidence strength remain simulation.trace.v1 and sampled_simulation.

Rejected: multiple behavior bindings or subjects, mixed state machines and activities,
forks/joins, decisions/merges, cycles, disconnected actions, object/item flows,
typed/inherited/nested action bodies, assignments and calls inside actions,
missing/invalid durations or output declarations, InitialStimulus, additional
objectives/assumptions/calculations/constraint techniques. The profile performs no FMI or
external-solver work. Unsupported models fail with activity.timed.unsupported or
mission.metadata.invalid; they do not receive a successful timed trace.

The original M11 still includes general token flow, guarded decisions, forks/joins,
item propagation and richer animated flow semantics. Those remain open.
