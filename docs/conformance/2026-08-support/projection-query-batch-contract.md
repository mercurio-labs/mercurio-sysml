# Shared projection-query batch contract

This is the fixed implementation contract. Its completed bounded verification is recorded separately in [projection-query-evidence.json](projection-query-evidence.json); this contract itself is not qualification evidence. Scope: the existing Type.feature, ownedFeature/endFeature and parameter/result projections used by computed redefinition. Keep the pinned definitions, selected delegate identities, canonical ownership, ordering, exclusion behavior, typed prerequisites and existing unsupported lifecycle errors.

## Audited native boundary

`stored_children` builds an identity index for each call. `stored_membership_endpoint` builds another index per endpoint read. Before this batch, owned/effective Feature and positional projections called these standalone readers, and effective/positional queries called standalone general-type readers. Those paths discarded the enclosing immutable query view. The new query variants propagate the caller view through these projections, result traversal and chain-equivalence reads. Additional-member readiness, strategy selection and some general-type producer internals still use their existing graph-facing services. This is source inspection, not a measured attribution of total elapsed time.

| Shared path | Imported inputs | Consumer entry points | Required batch change |
|---|---|---|---|
| Owned Features/ends | Ecore subset, setting delegate, ancestry and opposite contracts | definition_owned_features, definition_end_features | Carry the existing query index through ordered children and every endpoint read |
| Effective Features/ends | Resolved general-type edges; inherited membership/visibility/redefinition contracts | definition_effective_features, definition_positional_ends | Keep one query through general inputs, inherited traversal, membership projection and canonical declaring-owner reads |
| Parameter selection | Resolved parameter collection/ignored-parameter policy and Ecore direction/ownership | definition_parameter_collection, definition_relevant_parameters, definition_positional_parameters | Share the same immutable query with both owned and effective collections; preserve positional and duplicate/chain selection |
| Result selection | Imported ReturnParameterMembership and result/general-type contracts | definition_result_parameter, generated_result_parameter | Share checked navigation and general inputs; preserve first-result ordering and existing unresolved/lifecycle errors |

## Fixed verification boundary

Run focused existing independent Pilot controls for ends, parameters/results and computed redefinitions while editing. Add controls proving caller-view consumption and transparency on malformed/deferred endpoints, duplicates, ordering, edits and persistence. Keep constructor, conjugation/chaining and added-member exclusions explicit. No inferred is_implied_included flag, empty-scope fallback, sample-specific branch or changed library identity is allowed.

At the completed batch boundary, run the native definition regression and candidate-pipeline controls. Assess the same hash-bound actual resource closure once. Complete sample/Pilot comparisons and comparable timing remain milestone/final qualification work. Passing projection controls cannot close EnumLiteralDeclaration, EnumRule or a global dependency without their complete contracts.

## Coverage boundary

| Behavior | Imported | Native consumed | Verification / unsupported remainder |
|---|---|---|---|
| Owned Feature and end selection | Existing resolved Ecore contracts | Shared indexed child/endpoint projection | Existing independent end controls plus query-view guards; full Type lifecycle remains open |
| Effective Feature/end order | Existing resolved general/membership contracts | Shared query and bounded acyclic inheritance service | Independent end/parameter controls; cyclic positional generalization remains explicitly unsupported |
| Parameter/result selection | Resolved bounded policy and Ecore direction/ownership | Same caller query for owned/effective collections and first-result traversal | Independent parameter/result controls; conjugation/chaining, constructor and unfinished added-member lifecycles remain explicit boundaries |
| Actual pinned resource closure | Pinned source inputs | Candidate parse/construct/link pipeline | Fresh diagnostic assessment in the evidence record; requires completed linking, validation and persistence before qualification |
| EnumLiteralDeclaration / EnumRule | Reviewed full inventories | Bounded native carriers | Neither family is qualified; actual trigger/View and global parser/model-construction contracts remain required |

A successful diagnostic closure would still need detailed independent model comparison and comparable timing before release acceptance. No family certificate, completion flag or promotion follows from projection controls alone.
