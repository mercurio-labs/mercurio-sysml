# Sampled deadline requirements (M5 first slice)

The native analysis-case path now adds requirement_outcomes and the schema marker
mercurio.simulation.requirement_outcomes.v1 to simulation-trace artifact payloads.
Existing trace fields and mechanical run status remain unchanged.

The executable fixture is crates/mercurio-sysml/src/simulation/thermal-deadline.sysml.
It uses one analysis subject and a directly owned requirement with one require
constraint and an attribute named deadline with a literal Real value. For this
bounded profile, deadline is measured in seconds from simulation time zero.
A five-second deadline yields violated at heatRate 10 and satisfied at heatRate 20
(the first qualifying temperature sample is at three seconds).

The semantics are eventual satisfaction at a recorded sample no later than the
deadline. An exact deadline sample is required even when an earlier witness exists.
Absent deadline coverage, blocked/failed execution, missing expressions, unresolved
operands and unsupported expression forms yield unevaluated with a stable reason
code. There is no interpolation across missing samples. Comparisons and Boolean
combinations of finite numeric/Boolean literals and resolved feature paths are
supported; arithmetic, calls, multi-subject requirements and multiple nested
constraints are not evaluated in this first slice. Unknown operands are checked
even inside Boolean branches that would otherwise short-circuit.

Evidence is sampled_simulation, not a proof or exhaustive reachability result.
This does not complete M5: typed Mercurio::Missions metadata, explicit termination
reasons, final-state termination and full dropped-construct diagnostics remain.
M7 playback and the end-to-end failing-to-passing agent mission are also unfinished.
