# 2026-08 implementation checkpoint

Immediate goal: the [definition-driven core preview](definition-core-preview.md), using the [six-item acceptance checklist](remaining-gap-checklist.md). Full 2026-08 semantic qualification is deferred; its original [completion criteria](full-qualification-checklist.md) and evidence are preserved. The candidate remains unpromoted.

This is a candidate implementation, **not a qualified 2026-08 release or a full
SysML/KerML conformance claim**. The original [audit](../2026-08/README.md) is an
immutable before-change baseline. The work is on `codex/sysml-v2-compliance-2026-08`
in separate outer, SysML and Foundation worktrees from local `develop`.

The [remaining-gap checklist](remaining-gap-checklist.md) is the authoritative
completion plan and verification cadence. This page retains historical evidence.

## Completed implementation

- Native parsing/lowering for release transitions, event payloads, send and
  assignment actions, control nodes, loops, conditional actions, allocations,
  anonymous enumerations and messages, and KerML metadata and short names.
- Scoped imports, public membership/re-export lookup, inherited feature lookup,
  implicit definition/usage defaults, conjugated ports, nested namespace ownership,
  and distinct IDs for anonymous owners. Mapping overlays emit actual metaclasses.
- Generated facts from clean Pilot commit
  `692170b71867353b8f90341e61556f49a5beb0e5` (tag `2026-08`), with source/runtime
  digests: grammar, Ecore metamodel, 12 SysML definition defaults, 9 KerML definition
  defaults, 14 usage defaults, 4 additional view/concern definition defaults, and all 22
  leading all-types/single-type predicates with their extracted exclusion guards.
- Native execution of the extracted usage type-family predicates, including the
  conditional Usage, OccurrenceUsage, ConnectionUsage and ActionUsage checks.
  These now traverse declared/default typing, subsetting, redefinition and
  inherited feature types, and remove redundant supertypes. Computed conjugation,
  cross features and general expression-result typing remain open. The coverage
  overlay traces all 113 annotated Pilot validator methods and explicitly records
  28 partial implementations and 85 untraced methods.
- Shared KerML/SysML parsing and neutral expression IR now retain classification,
  identity, eager Boolean operators, conditional/coalescing values, indexing,
  units, named arguments, expansion/lambda bodies and lexical parameter binding.
  KerML initializers and function/expr/bool result bodies are no longer discarded.
  Multiplicity, short names and initial/default value flags are retained.
  New expression forms fail explicitly when runtime evaluation is unsupported.
- Scientific exponents preserve their numeric values and following unit expressions;
  malformed exponents are rejected. Initializer self-references retain their own
  qualified identity even when another source declares the same name, while
  lambda parameters still shadow outer features. These repairs came from the
  detailed Pilot comparison, rather than sample-acceptance checks alone.
- Individual declarations use occurrence metaclasses with `is_individual`; accept
  actions, assertions and default interface ends retain their actual metaclasses.
  Explicit accept payload names survive lowering. Import-all preserves private
  imported memberships, inherited typing uses the target's lexical scope, and
  action typing no longer guesses a type from a similarly named definition.
- The Pilot oracle retains ordered reference sequences, including empty lists
  and repeated targets. A bounded structural initializer projection compares
  ordered operators, literals, references and invocations; unsupported cases
  remain unassessed.
- Strict semantic comparison reports coverage and preserves raw source elements,
  attributes and sequence order. Unsupported tolerance actions fail explicitly.
  Existing non-profile comparison APIs retain their behavior.
- Pilot diagnostics now execute `CheckMode.ALL`; merely reading resource errors
  previously omitted semantic checks. Batch diagnostics reject unequal support
  sets, preventing unrelated cases from changing name resolution.

The candidate standard library is exported from the pinned 0.62.0 runtime and
contains 17,166 KIR elements. The 10,262 named source IDs and public memberships
are retained. The currently shipped/default release bundle has **not** been
promoted to 2026-08. Explicit candidate-library paths were used for corpus checks.

## Earlier expression checkpoint

The retained implementation compiles all **310/310** release sample source sets,
up from **156/310** in the original audit. Pilot accepted the same 310 source sets
with `CheckMode.ALL` in that audit. The Release and Pilot sources are byte-identical;
sibling files and the seven recorded cross-folder dependencies remain unchanged.
This result establishes acceptance for those inputs, not preservation of every
semantic construct or agreement on every invalid model.

Twelve type controls use identical support sets in both engines, unique namespaces,
and the target file last. Six positive cases pass; six negative cases fail with
the expected Pilot type-rule identifier. The repaired legacy Pilot diagnostics
also reports six passes and six failures. `evidence.json` records the final
native results, pinned Pilot controls, fixture hashes and verification results.

A strict same-context `PartTest.sysml` comparison exposed unresolved differences:
43 native versus 382 Pilot source elements, 41 paired mismatches, 2 native-only,
341 Pilot-only, and no exact pairs in that exploratory run. These are **raw
representation differences**, not 341 established missing semantics: Pilot
materializes relationships and expressions as elements where native KIR often
uses properties or expression IR. Reference-ID and scalar/array conventions also
differ. No exclusions or tolerances were added to make this pass. A concrete
nested-package ownership defect found by this comparison was fixed afterward;
the exploratory comparison is not a post-fix semantic qualification result.

The [earlier semantic and performance assessment](../2026-08-assessment/README.md)
is retained as an immutable before-expression-change baseline. The subsequent
[expression candidate assessment](../2026-08-expression-assessment/README.md)
found two constants with lost exponents/units and an initializer reference bound
to an unrelated same-named feature. Those defects were repaired.

The [post-audit assessment](../2026-08-post-audit-assessment/README.md) reruns all
**310 same-context native comparisons** and **48 fresh-process timing trials**
across 73 targets with the corrected compiler. Only unchanged Pilot semantic
exports are reused, with source-set, runtime, helper-class and export hashes
verified. Completion is not semantic parity. Raw snapshots, source-level triage,
timing phases, ranges and frozen source/binary provenance are retained with each
assessment. The earlier inventory maintenance is disclosed in its original run;
the post-audit source snapshot includes the corrected 113-method inventory.

The current [expression evidence](expression-evidence.json) records **310/310**
native corpus acceptance and **18 Pilot-backed type controls**: nine valid models
pass and nine invalid models fail with the expected type-rule codes in both
engines. Pilot's additional diagnostics remain visible; this is not complete
invalid-model diagnostic parity. The earlier twelve-control `evidence.json`
remains the prior checkpoint rather than being overwritten.

Verification: **368 SysML core tests**, **499 Foundation core tests**, four
validator-extractor regressions, and five expression-projection regressions pass.
Foundation and SysML WASM checks and the complete SysML workspace/all-target check
also pass after the audit repairs. Native/WASM host adapters, plugins, the product
API and strict dependency boundaries passed earlier in this worktree, before the
final lexer/private resolver repairs; those final repairs change no public API or
dependency. These downstream checks establish build compatibility, not complete
UI/runtime or release qualification.

The inventory extractor now recognizes annotated Xtend overrides and explicit
return types, and rejects unrecognized annotated declarations instead of scanning
onward into a helper. Its corrected inventory contains 42 KerML and 71 SysML
annotated method declarations (including three overrides), not 113 independent
constraints. The corrected extraction passes a fresh `--check` against the clean
pinned Pilot source. The coverage ledger remains incomplete: 22 partial methods
and 91 untraced methods; the prior evidence files retain their original counts.

## Earlier typing and authoring checkpoint

- Single-type predicates now cover enumeration, calculation, constraint,
  requirement, concern, analysis, verification, use case, rendering, viewpoint,
  view and metadata usages. The 38 new Pilot controls cover accepted models,
  wrong/multiple types, inherited typing and redundant supertypes. All controls
  parse in both engines; 24 are accepted and 14 are rejected with a shared Pilot
  type-rule code. The retained fixture ledger records Pilot's additional errors.
- Typed lambda parameters resolve inherited and nested members in their lexical
  scope. Missing members on known types are rejected; untyped parameters remain
  dynamic. Parameter shadowing has explicit regressions.
- Source and KIR authoring preserve default versus initial values, lambda
  parameter qualifiers/types/multiplicity/defaults, operator precedence, quoted
  member names and multiple declared types. KIR authoring accepts structured
  expression IR and translates opaque element IDs back to qualified source names.
- Metadata shorthand retains its declared type. Verification usages emit
  `VerificationCaseUsage`. Named `action ... assign ... := ...` declarations emit
  `AssignmentActionUsage` and retain both operands. Emitted KIR keeps all explicit
  declared types, including secondary types.
- Type validation now takes implicit defaults from the declaration being compiled.
  It no longer borrows the default of a same-named declaration in a support file.
  A source-order regression covers the action/part collision exposed by the
  preliminary detailed audit of the asynchronous messaging examples.
- The Pilot library exporter now reads inherited feature types and adapter-cached
  implicit supertypes through Pilot's own utilities. The separately imported
  `stdlib.inherited-types.kir.json` candidate has 17,166 KIR elements from 10,262
  named Pilot elements and 57,831 exported relationships. The earlier candidate
  library and benchmark evidence remain unchanged.

The current [typing evidence](typing-evidence.json) records 310/310 release
sample compilations and 56 fresh Pilot/native control comparisons: 33 valid models
pass and 23 invalid models fail with a shared expected type-rule code. All 377
SysML core tests, 499 Foundation core tests and five expression-projection tests
pass. Foundation/SysML WASM, all SysML workspace targets, strict boundaries,
native/WASM host adapters, plugins and the product API all pass against this
candidate. These are build and bounded semantic checks, not full qualification.
The earlier 368-test record above remains historical.

The [final typing and authoring assessment](../2026-08-typing-assessment/README.md)
completes all 310 detailed same-context comparisons and 48/48 successful timing
trials across 73 targets. It retains raw differences and reports 273 named
metaclass differences, 173 direct Boolean differences and incomplete expression
coverage. Median process times span 0.729–3.742 seconds for native and
8.083–16.547 seconds for Pilot across the eight groups. These compare current
workflows, not equivalent semantic work or a qualified compiler speedup.

## Declaration and execution follow-through

The current work adds these repairs beyond the frozen typing checkpoint:

- Library packages retain `LibraryPackage`, `is_standard` and authoring syntax.
  Generated emission overrides preserve 27 actual Ecore metaclasses that inherited
  mappings previously reduced to generic kinds.
- References to usages now use the IDs actually emitted for those usages, including
  cross-source initializer/allocation targets. String literals and free text are
  not rewritten as IDs.
- `IncludeUseCaseUsage` retains its explicit or referenced form, default type and
  `ReferenceSubsetting` relationship. Four fresh Pilot controls cover three valid
  forms and an invalid target. Source and KIR authoring round trips are tested.
- Usage variability and composition derive from ownership and typing using pinned
  Pilot source facts. Fifteen Pilot observations check eight properties each.
  Both languages preserve `ordered` and `nonunique`; authoring emits their suffix
  positions. Sixteen additive field contracts come from the extracted metamodel
  and explicit selection; strict KIR validation remains enabled.
- Library-index cache keys include semantic properties and language mappings.
  Cross-language and in-place-edit regressions prevent stale inherited features
  when the same library allocation is reused. The full suite first exposed an
  intermittent unresolved `duration`; its failed log and isolated recheck remain
  in the evidence directory, followed by the corrected complete rerun.
- The shared Foundation evaluator executes `if`, `??`, `implies`, `xor`, `&`, `|`
  and one-dimensional sequence `#`. A fresh executable Pilot oracle supplies 34
  results, tested through both SysML and KerML compilation (68 comparisons).
  Branching and coalescing are lazy; eager Boolean operators evaluate both inputs.
  Indexing is one-based and produces an empty sequence outside the bounds.
  The JSON numeric contract also accepts integral arithmetic results as indexes;
  it does not distinguish every integer/rational type or model collection shape.
- Requirement evidence checks every referenced observation before evaluation, so a
  skipped branch cannot hide a missing trace value. This evidence policy is more
  conservative than ordinary expression evaluation. Missing bindings, fractional
  indexes, wrong Boolean types and malformed IR retain explicit errors.

Current verification: **310/310 sample compilations**, **60 fresh native/Pilot
validation controls** (36 accepted, 24 rejected with the expected shared code),
**68 execution comparisons**, **393 SysML tests** and **503 Foundation tests**.
Native/WASM host adapters, plugins, the API, workspace targets, WASM kernels and
strict dependency boundaries pass. All **48 timing trials** succeed; each engine
accepts all 219 target executions across 73 targets and three repetitions.
[Declaration evidence](declaration-evidence.json) retains fixture and log hashes,
the earlier failed cache-sensitive test run and its corrected full rerun.

Execution fixtures and Pilot provenance are in
`crates/mercurio-tools/corpus/release-2026-08/execution-controls.pilot.json`.
`PilotExpressionAudit.java` evaluates parsed expressions through Pilot's execution
API and records unresolved values as errors. These bounded execution checks do
not establish arbitrary function, lambda, unit or model-object execution.

The validator ledger now records **23 partial methods and 90 untraced methods**.
The inclusion rule is an additional partial method; its computed feature-chain
cases remain open. Historical counts and reports above are unchanged evidence.

## Derived-flag repair follow-through (2026-09-18)

The declaration assessment exposed a regression from 173 to 268 direct Boolean
mismatches. Its source snapshot and results remain unchanged historical evidence.
The follow-up candidate repairs:

- Missing allocation/interface ancestry and the conditional base/binary connection
  default. The selection rule and library anchors are extracted from pinned Pilot
  source and checked against two- and three-ended interface observations.
- Merge/fork/join/decision/message and structured-action default types, including
  the more specific if-else type observed in the release source oracle.
- SysML untyped end declarations incorrectly emitted as KerML Feature. Their
  ReferenceUsage metaclass now comes from DefaultReferenceUsage in Pilot grammar.
- Variant featuring/owning type distinctions, variable-end constant flags, and
  message abstraction.
- Variation definition identity, abstract definition flags omitted by inherited
  emission templates, and abstract/variation authoring reconstruction.
- Anonymous action bodies previously parsed as references named action. Reserved
  anonymous usage keywords retain their metaclass; quoted names remain references.

The complete corrected suite passes **397 SysML tests and 503 Foundation tests**.
The focused fixture has 16 declarations with explicit Pilot property observations
and no Pilot validation diagnostics. Partial-library recovery remains supported:
generated parents are added as implicit semantics rather than unresolved explicit
source references. The earlier failing full run is retained separately.

The [frozen follow-up assessment](../2026-08-derived-flags-assessment/README.md)
completed all **310/310** comparisons and **48/48** timing trials. Direct Boolean
differences decreased from **268 to 83**; additional Boolean differences decreased
from **328 to 86**. Metaclass differences remain **121**. All **310 strict sample
compilations**, **60 validation controls**, and **70 candidate-library release
regressions** pass. Native group medians are 0.709–4.550 seconds versus Pilot's
8.083–19.266 seconds; these measure different current validation workloads and do
not establish equivalent semantic work. Source and binary integrity was verified
before subsequent repairs. Full qualification remains open.

## Conditional defaults and sequence execution (2026-09-18)

The next candidate preserves the types of untyped occurrences, both inherited
binary-connection types, and action/state transition defaults selected from the
resolved source metaclass. Inherited source features resolve across support
files. Named guarded successions retain TransitionUsage, and parenthesized
connection ends now retain their ReferenceSubsetting objects without inventing
declared names for anonymous ends. Flow definition
base/binary defaults come from the pinned adapter and implicit-generalization map.

The shared neutral evaluator incorporates the reviewed scalar/sequence changes
from Foundation commit `5633b3f5d616fc8d6d23214121ad2a93708cfc76`, preserving this
branch's newer structured IR. Tuples concatenate result sequences, null produces
no result, integer arithmetic preserves integer precision, and remainder, ranges,
selected sequence/numeric functions and lazy Boolean operations execute in the
shared core. Range tokenization preserves both dots. Explicit runtime bounds and
remaining numeric-equality differences are recorded in
`resources/kernel/execution-decisions.overlay.json`.

Fresh Pilot execution observations cover 26 expressions: **19 matching cases**,
**two mixed-numeric equality differences**, and **five rejected/unevaluated oracle
cases**. The matching cases are compiled and executed through both SysML and
KerML (**38 successful comparisons**). The two differences are asserted separately
in each language (**four recorded differences**, not passes). Pilot's
`EvaluationUtil.equal` delegates to Java object equality; the native evaluator
retains numeric-value equality. This is an unresolved qualification difference,
not a hidden comparator normalization. Large integer literals and three qualified
numeric function calls have no successful Pilot execution result in this probe.

The flow-end check rejects a source-owned third end with Pilot's
`validateFlowDefinitionConnectionEnds` code. Two positive and one negative control
agree with Pilot. The ledger moves to **24 partially traced methods and 89
untraced methods**; inherited flow-end collection and complete diagnostic
locations remain open. The first control-manifest attempts selected the same
negative target repeatedly; they are retained separately, and corrected per-target
results and hashes are checked in beside the three fixtures. All 310 release
corpus manifests were independently checked for correct target-last ordering.

The final anonymous-end fix passes **402 SysML tests**, **75 release regressions
with the actual candidate library**, and the SysML WASM check. The Foundation
changes pass **513 tests** and its WASM check; native host-adapter, REST and plugin
consumer checks pass. The frozen optimized candidate compiles **310/310 release
samples** and agrees with **63 Pilot-backed validation controls** (38 accepted,
25 rejected). The [frozen semantic and timing assessment](../2026-08-conditional-sequences-assessment/README.md)
completes **310/310 comparisons and 48/48 timing trials**, with source and binary
integrity verified afterward. [Combined evidence](conditional-sequence-evidence.json)
retains every compilation/control result and verification hashes. Previous
assessment snapshots remain immutable historical evidence.

The [semantic change record](conditional-sequence-semantic-change.json) shows
direct Boolean differences falling **83 to 32**, and additional Boolean differences
**86 to 34**. The projection now pairs **6,616** named declarations; **18,456/22,218**
reference fields agree and **4,250** missing counterparts remain outside that
denominator. Metaclass differences rise **121 to 126**: guarded succession `S`
is repaired, while six previously discarded named interface ends are now present
and reveal ReferenceUsage/PortUsage differences. Pilot's `InterfaceEnd` grammar
returns PortUsage; those ends still require a compiler fix. Ordered initializer
trees remain 186 equal, seven different and 310 unassessed; 17 of 503 declarations
still lack corresponding initializer IR. Strict graph equality remains zero.

Median process times are **0.700-6.492 seconds native** versus **7.686-17.775 seconds
Pilot**, across the eight measured groups. [Changes from the preceding snapshot](conditional-sequence-timing-change.json)
include the native vehicle median increasing from 4.550 to 6.492 seconds (+42.7%).
Three trials and uncontrolled host/cache conditions do not establish statistical
regression bounds. These timings compare the two current workflows; native still
implements less validation than Pilot, so this is not an equivalent-work speedup.

A [read-only source review](empty-validator-methods-review.json) identifies five
annotated Pilot methods with empty active bodies. The frozen 24-partial/89-untraced
ledger remains unchanged; this finding does not qualify their metaclasses or
inherited validators. **Full support and shipped/default promotion remain open.**

## Interface-end metaclass follow-through (2026-09-18)

The conditional/sequence snapshot exposed six previously omitted interface ends,
on top of 18 existing ReferenceUsage/PortUsage differences. The parser now follows
Pilot's `InterfaceEnd` rule for both binary and n-ary connector parts, preserving
PortUsage together with its ReferenceSubsetting object. Ordinary connection ends
retain ReferenceUsage. Named ends keep declared names; anonymous ends stay unnamed.

A fresh fixture covers ten interface ends and two ordinary connection ends.
Pilot accepts it without diagnostics and reports all 12 expected metaclasses.
The initial fixture used reserved words as unquoted names; its rejected oracle
attempt is retained separately and excluded from semantic evidence. Provenance and
observations are in `crates/mercurio-tools/corpus/release-2026-08/interface-ends.pilot.json`.
The full suite passes 403 tests, and the corrected fixture passes in all 76
candidate-library regressions. WASM and all-targets workspace checks pass. The
[final frozen assessment](../2026-08-interface-ends-assessment/README.md) completes
**310/310 sample compilations, 64 validation controls (39 accepted, 25 rejected),
310/310 detailed comparisons and 48/48 timing trials**. Its
[combined evidence](interface-end-evidence.json) verifies all 468 source files and
three optimized binaries; Foundation sources are unchanged from the verified
513-test snapshot. The preceding snapshot remains unchanged before-fix evidence.

[All 24 observed interface-end metaclass differences are resolved](interface-end-semantic-change.json)
across five files. The overall metaclass count falls **126 to 102**, while direct
and additional Boolean differences remain **32 and 34**. Reference agreement is
**18,448/22,246**, with **4,222** missing counterparts; the previous snapshot was
18,456/22,218 with 4,250 missing counterparts. More fields now have counterparts,
but some explicit default types and implicit relationships differ. In particular,
several referenced ends still emit generic Ports::Port instead of their concrete
referenced port definition. This metaclass repair does not establish type or
relationship parity.

Final native process medians are **0.816-7.218 seconds**, versus Pilot's
**8.942-21.289 seconds**. The vehicle medians are 7.218 and 21.289 seconds.
[Per-group changes and ranges](interface-end-timing-change.json) retain the increase
from the preceding vehicle medians of 6.492 and 17.775 seconds. Changes in both
engines and uncontrolled host/cache conditions prevent attributing the difference
to this repair alone. The complete report retains phase times and all trials.

**At this checkpoint, full support remained incomplete.** The validator ledger
recorded 24 partial methods and 89 untraced methods. The following checkpoint
records subsequent changes; expression, relationship-projection and promotion
gates remain open.


## Referenced typing and validation follow-through (2026-09-18)

The existing develop-based compliance worktrees now inherit minimal concrete
types through connector-end references, including aliases, inherited features,
multiple types and separate support files. Mixed named/anonymous binary ends
parse independently. Pilot observations verify 14 ends across seven connectors.

Four additional Pilot reference-kind methods are mechanically extracted and
executed for perform, exhibit, assert and satisfy usages, with PerformAction's
exclusions retained. Qualified references preserve their owned ReferenceSubsetting
relationships and implicit names. Assertion negation and asserted/negated
satisfaction prefixes retain their grammar meaning and `is_negated` field.
Named succession flows retain SuccessionFlowUsage and their extracted default
Flows::SuccessionFlow type. Qualified reference authoring retains targets and
negation; generated child identities no longer force whole-file rewrites solely
because an anonymous owner's source position moved.

The [new assessment](../2026-08-reference-typing-assessment/README.md) and
[combined evidence](reference-typing-evidence.json) verify **310/310 samples**,
**76 controls (47 accepted, 29 rejected as expected)**, **407 SysML tests**,
**80 candidate-library regressions**, **513 Foundation tests**, WASM compilation,
and the all-targets workspace check. The 483 source files, three optimized
binaries and assessment tools were frozen before 310 semantic comparisons and
48 fresh timing trials. Unchanged Pilot source exports were reused only after
source-set, helper, runtime and export digest verification.

The [aggregate semantic change](reference-typing-semantic-change.json) records
**102 metaclass differences** and
**32/34 direct/additional Boolean differences**.
Reference agreement is **18,482/22,252**,
with **4,226 missing counterparts**.
Correctly anonymous usages can leave the named-declaration projection; absence
from its difference list is not itself a proof of equivalence. Full raw reports
and incomplete expression observations remain retained.

Native process medians span **1.220-8.757 seconds**, versus
Pilot **10.655-29.909 seconds**. The
[timing comparison](reference-typing-timing-change.json) retains every group and
its range. Host load and OS caches are uncontrolled, and the engines still
perform different semantic workloads; these are not conformance-equivalent
compiler speedups.

The validator ledger moves from 24 partial/89 untraced to **28 partial/85 untraced**.
Computed basic-feature chains, complete diagnostic multiplicity/location,
inherited flow ends, further authoring/execution semantics, lossless projection
and bundle qualification remain open. **Full 2026-08 support is not complete,
and the candidate has not been promoted to the shipped/default baseline.**


## Declaration metaclass preservation (2026-09-18)

Extension keywords now retain the grammar-defined Definition/Usage metaclasses,
including stacked prefixes, while explicit language declarations keep their
ordinary kind. Prefixes survive authoring through the shared language-extension
representation. Anonymous extension-prefixed connectors render their ordered
endpoint tuples and body members as valid source. KerML `type` declarations retain their bodies and specialization
links, and `assoc struct` retains AssociationStructure. Construct identities are
generated from the pinned grammar, with native routing in curated overlays.

Pilot validates both new controls and independently records their metaclasses,
members, typing and specialization. These repairs do not qualify complete semantic
metadata execution, derived base-type application, or every anonymous KerML Type
and type-relationship alternative. Symbolic conjugation parses, but conjugation
and union/intersection/difference relationship objects remain unmaterialized.

The [assessment](../2026-08-metaclass-preservation-assessment/README.md) and
[combined evidence](metaclass-preservation-evidence.json) verify **310/310 samples**,
**78 controls (49 accepted, 29 rejected as expected)**, **409 SysML tests**,
**82 candidate-library regressions**, **513 Foundation tests**, WASM compilation,
and the all-targets workspace check. Source files and three optimized binaries
are frozen. All 310 detailed semantic comparisons and 48 fresh-process timing
trials completed; unchanged Pilot graphs were reused only after provenance checks.

The [semantic comparison](metaclass-preservation-semantic-change.json) records
metaclass differences changing from **102 to 56**.
**52/54 direct/additional Boolean differences** remain. The increase from 32/34
consists of 20 newly assessed `is_variable` and 20 `may_time_vary` discrepancies
that previously had no direct native value; see the [scope audit](metaclass-preservation-boolean-scope-audit.json).
Reference agreement is **18,504/22,271**,
with **4,227 missing counterparts**.
Corrected kinds and newly preserved bodies change matching coverage; these
projection counts do not establish lossless graph equivalence.

Native process medians span **0.760-5.756s** versus
Pilot **8.854-17.348s**. The
[timing comparison](metaclass-preservation-timing-change.json) retains every group.
Host load and OS caches are uncontrolled, and semantic workloads remain unequal;
these are not conformance-equivalent compiler speedups.

The validator ledger remains **28 partial/85 untraced**. Full expression semantics,
remaining validators, implicit relationships, lossless projection and release
qualification remain open. **Full 2026-08 support remains incomplete; the candidate
has not been promoted to the shipped/default baseline.**

## Source semantic metadata bases (2026-09-18)

The native resolver applies source language-extension metadata `baseType` values
written as static metaclass references. Definitions specialize the base feature's
minimal types (or a classifier base directly); usages subset feature bases.
Inherited metadata, redefined default values, short-name imports, stacked
annotations and support-file ancestry use the same lexical indexes. Ordinary
metadata adds no semantic parents, and classifier bases are ignored for usages,
as in Pilot's FeatureAdapter. Base-feature anchors and adapter-policy guards are
mechanically extracted from the pinned Pilot; native adaptation remains separate.

The [assessment](../2026-08-semantic-metadata-assessment/README.md) and
[evidence](semantic-metadata-evidence.json) verify **310/310 samples**,
**80 controls (51 accepted, 29 rejected as expected)**, **410 SysML unit tests**,
**83 candidate-library regressions**, WASM and all-targets workspace checks.
Foundation source hashes are unchanged; its previous **513-test** evidence is
reused explicitly rather than reported as a fresh test run. The fixture retains
316 Pilot source elements and 2,549 outgoing relationships. Generated-rule drift
checking passes. Candidate sources and three optimized binaries are frozen.

All **310 detailed comparisons** and **48 fresh-process timing trials** completed.
The [semantic change report](semantic-metadata-semantic-change.json) records
direct Boolean differences **52 → 27** and additional Boolean
differences **54 → 29**. The
[value audit](semantic-metadata-variability-audit.json) rechecks all 40 variability
differences exposed by the preceding metaclass repair: **40 now agree**
and **0 remain different**. These counts use actual direct values in
both current raw snapshots, not disappearance from a projection.
There are **56 metaclass differences**; reference agreement is
**18,607/22,362** with
**4,136 missing counterparts**. Raw graphs and all differences remain
retained; this declaration projection does not establish lossless equivalence.

Native process medians span **0.847–10.291s**, versus
Pilot **9.908–26.031s**; the [timing change report](semantic-metadata-timing-change.json)
retains all groups and ranges. Each engine has three trials per group, with
uncontrolled host load and OS caches. Semantic workloads remain unequal, so
these are not conformance-equivalent speedups.

General metadata expression evaluation, explicit metadata application bodies,
imported-library metadata value preservation and complete anonymous relationship
projection remain unqualified. The validator ledger remains **28 partial/85
untraced**. **Full 2026-08 support is incomplete; the candidate is unpromoted.**

## Metadata pass performance refinement (2026-09-18)

The metadata pass now skips metaclass-ancestry checks for unannotated context
declarations. The [final assessment](../2026-08-semantic-metadata-optimized-assessment/README.md)
and [evidence](semantic-metadata-optimized-evidence.json) repeat **310 sample
compilations, 80 controls, 410 unit tests, 83 candidate regressions, 310 detailed
comparisons and 48 timing trials**; WASM and workspace checks pass. Foundation
sources remain unchanged and reuse the preceding 513-test evidence.

All raw aggregate metrics, named-declaration projection counts, ordered expression
projection results and detailed projected differences are identical to the
preceding metadata assessment. The [Boolean delta](semantic-metadata-boolean-delta.json)
records **50 resolved differences and no introduced differences** versus metaclass
preservation; the [raw-value audit](semantic-metadata-optimized-all-boolean-repairs.json)
confirms explicit native/Pilot equality for all 50. **27 direct and 29 additional
Boolean differences, 56 metaclass differences and 3,755 reference-field differences
remain**. Full 2026-08 support and default promotion remain open.

Final native process medians span **0.743–5.608s**, versus
Pilot **8.598–17.027s**. The
[timing change report](semantic-metadata-optimization-timing-change.json) retains
per-group medians/ranges before and after the refinement. Host load and OS cache
are uncontrolled; differences cannot be attributed solely to the code change,
and unequal semantic workloads prevent a conformance-equivalent speedup claim.

## Connector and inherited-feature checkpoint

The [connector assessment](../2026-08-connectors-assessment/README.md) and
[evidence](connectors-evidence.json) record **310/310** sample source sets,
**82** shared controls (53 accepted, 29 expected rejections), **416** unit tests,
**89** candidate regressions, WASM and all workspace target checks. The unchanged
Foundation source hashes were verified against its preceding 513-test run;
those Foundation tests were reused, not rerun. The assessment freezes 498 source
files and three optimized binaries.

KerML connectors, bindings and successions now retain binary/n-ary ends, endpoint
reference subsettings, concrete metaclasses and ordered source/target relations.
Named SysML bindings and successions use the same endpoint infrastructure.
A fresh pinned Pilot oracle verifies all 15 selected named connectors' metaclass,
minimal type set and ordered source/target references. Conditional connector and
association defaults are generated from checked Pilot predicates and mappings.
The parser also retains anonymous return/redefinition headers and their types;
opaque relationship operands no longer masquerade as named feature declarations.
Expressions inherit the observed Evaluation type and evaluations subsetting,
so inherited library features resolve through both typing and feature ancestry.
A strict positive/negative regression covers inherited `that` redefinitions.

All **310** detailed comparisons completed. The named projection records
**20** metaclass differences (previously 56),
**18773/22501**
agreeing reference fields, **4076**
missing reference counterparts, **40** direct Boolean
differences and **30** additional Boolean
differences. There are **162** unmatched
Pilot named anchors and **16**
matched initializers lacking native expression IR. The raw comparison still has
**0 exact pairs**. These projections are triage, not a lossless
semantic equivalence result. [Aggregate changes](connectors-semantic-change.json)
retain the before/after denominators because new declarations change coverage.

All **48** fresh-process timing trials succeeded. Median process seconds for
three repetitions per engine/group are:

| Source group | Native | Pilot |
| --- | ---: | ---: |
| kerml/src/examples/Address Book Example/AddressBookModel.kerml | 0.765 | 8.672 |
| kerml/src/examples/Simple Tests | 1.755 | 11.288 |
| sysml/src/examples/Simple Tests | 4.731 | 13.557 |
| sysml/src/examples/Vehicle Example | 5.689 | 17.321 |
| sysml/src/training/02. Part Definitions/Part Definition Example.sysml | 0.748 | 8.636 |
| sysml/src/training/20. Assignment Actions/Assignment Example.sysml | 0.836 | 9.000 |
| sysml/src/training/23. State Definitions | 0.972 | 9.155 |
| sysml/src/training/32. Requirements | 0.988 | 9.255 |

Full phase timings and ranges are in the assessment; [timing changes](connectors-timing-change.json)
retain the preceding run. Host load and OS caches are uncontrolled, and the
engines perform unequal semantic work; these results do not establish a causal
or conformance-equivalent speedup.

Inherited and cross-file bound connector values, complete cross-feature graphs,
feature-chain identities, expression-result graphs, remaining validator methods
and full bundle qualification remain open. Crossing multiplicity is retained on
the endpoint but its complete owned cross-feature graph is not established.
The candidate is unpromoted and **full 2026-08 support is not complete**.

## Crossing-feature and variability correction checkpoint

The [corrected assessment](../2026-08-crossing-features-assessment/README.md) and
[evidence](crossing-features-evidence.json) retain **310/310** sample compilations,
**84** shared controls (55 accepted, 29 expected rejections), **419** unit tests,
**92** candidate regressions, and passing WASM/workspace checks. The unchanged
Foundation source hashes still match its preceding 513-test run; that result
was reused rather than rerun. Both additional control source sets pass fresh
Pilot `CheckMode.ALL` validation with no diagnostics.

The preceding connector assessment exposed twelve falsely marked end features
and one typed binding with incorrect variability. KerML owned crossing features
now retain their own name, owner and multiplicity separately from the end;
`is_end` is explicitly false and their types follow the owning end through the
shared type traversal. Named and anonymous crossing syntax are tested. Pilot's
Usage setting delegate adds a binding/succession exclusion to the adapter's
variability predicate; that exclusion is now mechanically extracted and applied.
Five selected declarations match a fresh hash-linked Pilot oracle on metaclass,
owner, type set and the relevant flags.

All **14 Boolean field discrepancies across 13 declarations** are verified repaired
by [raw-value comparisons](crossing-features-boolean-repairs.json), with no new
Boolean discrepancies relative to the connector checkpoint. The proof requires
both direct Boolean fields to exist and retain a unique declaration pair; it does
not count vanished fields or unmatched declarations as repairs. Direct/additional
Boolean differences are now **27/29**,
compared with 40/30 in the preceding run. **20** named
metaclass differences remain. The projection records
**18825/22565**
agreeing reference fields, **4090**
missing reference counterparts and **149**
Pilot named anchors without native pairs. Full raw differences are retained.

All **310** detailed comparisons and **48** fresh-process timing trials completed;
all **438** timed target verdicts pass. Three repetitions per engine/group, phase
measurements and ranges are in the assessment; the [before/after record](crossing-features-timing-change.json)
retains the preceding run. Host load and OS caches are uncontrolled and semantic
workloads differ, so these times do not establish conformance-equivalent speedups.

This repairs ownership, basic crossing typing and variability, not the complete
crossing semantics. CrossSubsetting objects, inherited/redefined crossings,
Cartesian-product featuring types, all implicit relationships, remaining validators
and bundle qualification remain open. An exploratory constant-end control in a
non-occurrence Association exposed the still-unimplemented KerML
[`validateFeatureIsVariable` negative case](crossing-feature-validator-gap.json); it is not counted among the passing
controls. The shipped baseline remains unchanged and **full support is incomplete**.

## Declaration and feature-legality checkpoint

The [fresh assessment](../2026-08-declaration-validation-assessment/README.md) and
[implementation evidence](declaration-validation-evidence.json) record **310/310**
sample compilations, **92** shared controls (**59 accepted, 33 expected
rejections**), **425** full unit tests, **98** candidate-library regressions, and passing
WASM/workspace checks. All unit and candidate tests were rerun on the assessed source. The unchanged
Foundation source still matches its preceding 513-test run; it was not rerun.

Named and optional-keyword terminate actions preserve their actual
`TerminateActionUsage` kind, argument and body. Their implicit type is extracted
from both fresh Pilot declarations. Named and anonymous flow payloads use
`PayloadFeature`, retain explicit names and source positions, and do not invent
an explicit name for the anonymous form. Seven selected declarations match fresh
Pilot metaclass/type projections, with owner comparisons where owners have source
names. KerML prefix metadata no longer creates a spurious crossing feature;
independent named/anonymous crossings remain intact. Both new positive source
sets pass Pilot `CheckMode.ALL` with no diagnostics. Complete metadata membership
and evaluation remain open.

Two mechanically extracted branches of `KerMLValidator.checkFeature` now reject
variable features without an occurrence owning type, and reject variable portions.
The extracted parser postprocessor makes constant KerML features variable.
Six fresh shared controls cover two accepts and four expected rejections, checking
Pilot issue codes/messages and positive flags. The exact previously recorded
constant-end negative control is also rejected with `validateFeatureIsVariable`;
its separate regression is retained in the evidence and is not double-counted
among the 92 controls. Its earlier acceptance remains in the historical
[crossing gap record](crossing-feature-validator-gap.json).

Enabling the validator exposed malformed inherited standard-library IDs in two
valid time-varying samples. Inherited lookup now retains library IDs through the
existing normalization path instead of prefixing them as local feature IDs.
Nested portion/redefinition positives and a non-occurrence feature-owner negative
regression pass. Both original samples compile again. This does not establish
all computed owner or crossing relationships. The validator ledger now contains
**29 partially traced and 84 untraced methods**;
`checkFeature` is explicitly partial, not complete.

The complete source comparison verifies [all three targeted metaclass repairs](declaration-validation-repairs.json)
on retained unique raw declaration pairs. Two removed Boolean discrepancies are
also verified equal as direct Boolean fields in the same retained pairs. Named metaclass differences are
**17**, compared with 20 before this batch. Direct/additional
Boolean differences are **25/29**.
The record lists 0 new metaclass/Boolean differences for review;
missing fields and unmatched declarations are not counted as equal.
Reference fields agree in **18839/22559**
comparisons, with **4096** missing counterparts.
Full raw graphs, all 310 comparisons, and the [aggregate before/after record](declaration-validation-semantic-change.json)
remain available.

All 48 fresh-process timing trials completed with 438 successful target verdicts.
The assessment reports three trials per engine/group, median/range and phase
measurements; the [timing change record](declaration-validation-timing-change.json)
retains the preceding crossing-feature run. Host load and OS caches are uncontrolled,
and semantic workloads differ. These timings do not establish a conformance-equivalent
speedup. The candidate remains unpromoted and **full 2026-08 support is incomplete**.

## Remaining requirements for full support

Use the [single remaining-gap checklist](remaining-gap-checklist.md) for active
work, batch acceptance criteria and the three full-assessment gates. Full support
remains incomplete. Historical checkpoint counts above apply only to their frozen
sources; the checklist records subsequent focused work without claiming a new
full assessment.

## Reproduce in this worktree

Run from `mercurio-sysml`. Upstream clones and the runtime are in the outer
worktree's ignored `target/upstream` directory. The immutable corpus is at
`../target/release-audit-2026-08-final/corpus.json`; candidate artifacts and raw
execution logs are at `../target/support-2026-08`.

```powershell
cargo build --locked -j 1 -p mercurio-tools --features legacy-pilot-tools --bin audit_release_compile --bin compare_pilot_semantics --bin import_pilot_stdlib
$env:MERCURIO_STDLIB_PATH = (Resolve-Path ../target/support-2026-08/stdlib.inherited-types.kir.json).Path
$env:MERCURIO_KERNEL_LIBRARY_PATH = $env:MERCURIO_STDLIB_PATH
target/debug/audit_release_compile.exe ../target/release-audit-2026-08-final/corpus.json ../target/support-2026-08/native-rerun.jsonl
cargo test --locked -j 1 -p mercurio-sysml --lib
cargo check --locked -j 1 -p mercurio-sysml --target wasm32-unknown-unknown
cargo test --locked -j 1 --manifest-path ../mercurio-foundation/Cargo.toml -p mercurio-foundation --lib
```

The current `stdlib.inherited-types.kir.json` and the historical
`stdlib.qualified-source.kir.json` are provenance-checked **candidate inputs**,
not qualified release bundles. Keep them separate when reproducing their reports.

The generators accept `--check` and fail on dirty Pilot inputs or artifact drift.
Definition defaults use `defaults-export.json` and `defaults.sysml`; KerML defaults
use `kernel-defaults-export-34.json`, `kernel-defaults.kerml`, namespace
`ReleaseKernelDefaults`, and the nine `--kind` values recorded in the artifact.
Usage defaults use `occurrence-defaults-export-33.json`,
`occurrence-defaults.sysml`, and the additional namespace
`ReleaseOccurrenceDefaults::sequence`. Type predicates derive directly from
`SysMLValidator.xtend` and the fresh `metamodel.extract.json`.
