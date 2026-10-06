> Historical evidence only. The active completion plan is [remaining-gap-checklist.md](remaining-gap-checklist.md). Earlier status and next-step statements below are snapshots.

# 2026-08 remaining-gap checklist

**This is the authoritative completion plan for this worktree. Full support is open.**
It replaces the remaining-work lists and implied next steps in older checkpoint
reports. Those reports remain historical evidence. Work stays on the existing
`codex/sysml-v2-compliance-2026-08` develop-based component worktrees.

Execute the source-driven S1–S3 batches below against the existing G01–G16
obligations, then S4/B5 qualification. Finish a coherent batch and its focused
tests before opening another. Fix a discovered regression in the active
batch; assign newly discovered gaps here instead of starting another report cycle.

<!-- structural-coverage:start -->
## Ecore/Xtext coverage checklist (authoritative)

Priority is demonstrated native support for the pinned 2026-08 Ecore and Xtext definitions.
The curated source is `structural-coverage-plan.json` in the candidate profile; edit that file,
then run `python -B tools/check_structural_source_coverage.py` to regenerate this checklist.
The linked [machine-readable checklist](structural-source-coverage.json) records every matched upstream ID,
native consumer, semantic dependency, evidence anchor and remaining obligation.
**Every used feature remains required and open. No exclusions are approved.** Rows with no active occurrences are marked not used in this pin and reopen automatically on drift. Partial means bounded consumers exist;
unassessed means native behavior has not been exhaustively mapped or qualified, not that parsing necessarily fails.
Imported definitions and generation tests never establish runtime support. The written specification is normative;
Pilot is independent behavioral evidence. Normative references still need to be attached to each qualification row.

| Feature | Imported instances | Native support | Required next work |
|---|---:|---|---|
| xtext.Action | 53 | unassessed | Generate object creation, current-object reassignment and assigned actions; qualify resulting object identities and ownership. |
| xtext.Alternatives | 261 | partial | Qualify full NamespaceBody and linking semantics; extend the native literal/feature-reference MultiplicityRange graph and derived getter to remaining literal forms and referenced-value delegates; compare direct and owned-chain MultiplicitySubset targets with Pilot and qualify general feature-chain linking; compare Flow and SuccessionFlow endpoint, `all` sufficiency, typed PayloadFeature with owned multiplicity and literal ValuePart graphs with Pilot, then implement nonliteral OwnedExpression construction, remaining payload-specialization/ValuePart alternatives and library redefinition semantics. Extend alternative choice, backtracking/predication and diagnostics across the full grammar. DisjoiningPart shorthand and delegate semantics remain separate gaps. |
| xtext.Annotation | 4 | unassessed | Classify each grammar rule annotation and implement its actual parser/generator effect or justify no runtime effect. |
| xtext.Assignment | 771 | partial | Implement =, += and ?= against resolved Ecore properties, including actions and object construction. |
| xtext.CharacterRange | 7 | unassessed | Implement character range boundaries and character encoding semantics in the shared terminal engine. |
| xtext.CrossReference | 71 | partial | Derive typed reference construction and connect to qualified scoping/linking services. |
| xtext.EnumLiteralDeclaration | 24 | partial | Qualify the generated mappings against Pilot model values in every context and connect all remaining enum rules to native model construction; the complete mapping is imported but only visibility, direction, PortionKind and RequirementConstraintKind have shared native consumers. |
| xtext.EnumRule | 14 | partial | Extend native parser and construction consumers from visibility, direction, PortionKind and RequirementConstraintKind to all remaining enum rules; qualify Pilot model values and context-specific grammar semantics, including the `[` filter visibility token. |
| xtext.Grammar | 3 | unassessed | Derive inherited/overridden rule selection and hidden-token policy per language; qualify the complete grammar closure. |
| xtext.Group | 639 | partial | Preserve ordered sequencing, group cardinality and predicates; handle any non-null guard conditions. |
| xtext.Keyword | 663 | unassessed | Derive keyword recognition and token-boundary interactions across all grammar contexts. |
| xtext.NegatedToken | 4 | unassessed | Implement terminal complement with correct EOF and character semantics. |
| xtext.ParserRule | 704 | partial | Generate parser/model-construction behavior for every rule, including fragment, wildcard and hidden-token attributes. |
| xtext.ReferencedMetamodel | 6 | partial | Use resolved namespace/classifier imports in every generated model binding. |
| xtext.RuleCall | 1724 | partial | Qualify full NamespaceBody calls, MultiplicityRange referenced-value semantics and generalized feature-chain linking; compare native Flow and SuccessionFlow endpoint/`all` sufficiency/typed payload with ordered multiplicity/literal ValuePart graphs with Pilot and implement nonliteral OwnedExpression plus remaining PayloadFeatureSpecializationPart/ValuePart rule calls. Natural literal and feature-reference MultiplicityRange bounds have owned Expression members and a native derived getter; Boolean, string and real bound syntax reaches Pilot's non-Natural transformation diagnostic. MultiplicitySubset consumes both direct-feature and owned-chain alternatives. Derive inherited rule lookup, explicit calls and arguments throughout the grammar while preserving return-object semantics. DisjoiningPart shorthand and delegate semantics remain open. |
| xtext.TerminalRule | 9 | unassessed | Implement terminal recognition and priorities from definitions, including fragments and hidden tokens. |
| xtext.TypeRef | 851 | partial | Use resolved Ecore type identities throughout parser model construction. |
| xtext.UntilToken | 2 | unassessed | Implement delimiter consumption and unterminated-input diagnostics. |
| xtext.cardinality | 4219 | unassessed | Drive optional/repeated parsing and construction from every cardinality; prevent empty-repeat loops. |
| xtext.predicates | 4219 | unassessed | Implement actual lookahead/first-set/guard behavior; retaining predicate flags is insufficient. |
| xtext.hidden_tokens | 707 | unassessed | Derive grammar-level and rule-level hidden-token behavior and overrides. |
| xtext.rule_parameters | 2428 | not_used_in_pin | No active occurrences in this pin; preserve the fields and reopen automatically if upstream begins using them. |
| ecore.inheritance | 175 | partial | Audit semantic_profile hierarchy consumers, which remain tied to registered release bundles; qualify abstract/interface construction and unknown/legacy metaclass boundaries. |
| ecore.types | 724 | partial | Extend source-driven reference typing beyond ten pinned library export fields to native construction and mutation; migrate primitive identities, enum/data types and operation types; remove legacy naming exceptions. |
| ecore.multiplicities | 542 | partial | Apply bounds to every relevant model property/operation and native mutation path, beyond the two admitted ownership pairs and ten pinned library import fields. |
| ecore.containment | 351 | partial | Qualify remaining relationship-body kinds and lexical scope/linking; migrate remaining ownership emitters and implement required cycle/removal behavior. |
| ecore.opposites | 351 | partial | Qualify Pilot getter order for derived Element.ownedElement; extend add/move/remove coverage beyond source recompilation and the two admitted ownership pairs, including other source-defined opposites and cycle behavior. |
| ecore.ordering | 542 | partial | Extend source-driven order preservation beyond the verified pinned library collections to all native construction, mutation and serialization paths; qualify unordered collection comparison separately. TypeUtil-derived specialization order and multi-type consumers remain separate semantic dependencies. |
| ecore.uniqueness | 542 | partial | Apply collection identity/value uniqueness across all actual properties and native mutation paths. Pinned library import now preserves the observed Feature.chainingFeature duplicate; seven TypeUtil-derived specializes duplicates need separate semantic classification. |
| ecore.defaults | 415 | partial | Qualify attached Pilot source-model default outcomes and serialized KIR, particularly 47 deferred isVariable delegate observations. Expose constructors override Import.isImportAll=false with true; native expose lowering already emits true. Qualify library-import values and direct non-source mutation paths; distinguish other derived/unsettable fields and custom model behavior. |
| ecore.property_flags | 415 | partial | Implement lifecycle/read-write/derived/proxy semantics of flags actually present; report irrelevant flags with evidence. |
| ecore.operations | 127 | unassessed | Map every operation to an executable native algorithm and ordered parameter/result semantics; signature export is not implementation. |
| ecore.annotations | 687 | unassessed | Classify annotation languages, constraints, documentation, subsets/redefinitions and semantic effects; do not execute documentation formulas by assumption. |
| ecore.delegate_bindings | 398 | unassessed | Resolve all setting/invocation dispatch and implement required algorithms, including default fallback behavior and Java dependencies. |
| ecore.packages_and_enums | 27 | partial | Apply package and data-type identities throughout native construction, connect the remaining Xtext enum rules to their model properties, and qualify all enum conversions against Pilot. Imported identities and values alone do not establish runtime support. |

### Implementation batches

- **S1 — Shared Ecore model services** (G05, G06, G07): Qualify remaining relationship-body kinds and lexical scope/linking; migrate remaining ownership emitters and reconcile semantic_profile hierarchy with release registration. Exit: Native consumers and focused controls cover hierarchy, types, bounds, ownership, collection semantics, defaults and property flags actually used.
- **S2 — Xtext-driven parsing and model construction** (G05, G08, G10, G12): KerML relationship-body dispatch consumes the 36 pinned OwnedRelatedElement choices; Namespace, Disjoining, direct and owned-chain MultiplicitySubset targets, Natural literal and feature-reference MultiplicityRange bounds, and Flow/SuccessionFlow FlowEnd, `all` sufficiency, typed PayloadFeature with ordered owned multiplicity, and literal ValuePart graphs now have focused native construction controls. Boolean, string and real MultiplicityRange syntax reaches the Pilot-matched non-Natural diagnostic. Qualify remaining Namespace and feature-chain linking semantics, referenced-value delegates, and Flow/SuccessionFlow nonliteral ValuePart, other payload-specialization/ValuePart forms and library redefinition semantics; then expand the shared grammar engine by family. Exit: Every observed grammar node and behavioral field has a native consumer and syntax/model-construction evidence.
- **S3 — Semantic dependencies and integration** (G05, G06, G07, G08, G09, G10, G11, G12, G13): Map scope/link/value conversion and every operation/delegate to shared native services; qualify required algorithms and document normative disagreements. Exit: No required dependency remains signature-only, unassessed or unsupported.
- **S4 — Milestone and release qualification** (G14, G15, G16): At substantial semantic milestones and final qualification, run all 310 samples in equivalent contexts, detailed Pilot comparisons and paired timing. Exit: All required behavior demonstrated; disagreements resolved/documented; comparable timings; native runtime and packaging qualified.

S1 is the active implementation batch; S2 and S3 follow dependency order rather than isolated sample fixes.
Existing G01–G16 semantic obligations remain required. M2/M3 retain full comparisons and timing.
Xtend remains secondary: assess correctness, direct-Rust effort, transitive Java/Xtend dependencies and maintenance benefit
for representative predicates, derivations and operations before selecting translation, native interpretation, handwritten Rust
or reference-only treatment. A general Xtend compiler is not a deliverable.
<!-- structural-coverage:end -->

## Structural batch history (2026-09-29)

**S1 requirement/textual ownership batch:** requirement-constraint memberships,
textual/invariant memberships and invariant result-expression memberships now use
both generated Ecore ownership pairs. Missing `owning_related_element` and
`owning_relationship` inverses are materialized through the shared attachment
algorithm. A top-level textual member creates a real anonymous resource Namespace
when needed; non-root missing owners fail instead of silently losing an endpoint.
Native graph controls cover both directions, ordered membership children, explicit
requirement memberships and resource namespace qualification. The invariant fixture
runs through KerML, whose grammar supplies this result-expression construction.
143 release regressions, 15 lowering golden controls and three ownership tests pass
with Java unavailable; 12 focused tooling tests, production compilation and source
freshness pass. Logs: outer `target/support-2026-08/membership-ownership-*`.
Direct ordinary KerML relationship-owned declarations, alias/import construction,
other implicit graph emitters and general reparenting/cycle semantics remain open.
No full sample comparison or benchmark was repeated.


**S1 typing hierarchy migration:** usage-family eligibility, required type checks,
owned-end checks, feature/usage flags, scalar validator applicability and the native
`Class.isInstance` binding now share resolved-EMF `metaclass_conforms`. Removed the
second runtime ancestry traversal and deserialization of the type-check extract's
legacy generalizations. Its predicates remain separately imported/implemented;
this does not expand Xtend translation or complete derived type collection.
All 142 release regressions and 40 focused Java-unavailable native controls pass;
48 focused Python checks, production compilation and provenance checks pass.
Logs are outer `target/support-2026-08/shared-typing-hierarchy-*`.
`semantic_profile` still uses registered release-bundle hierarchy and separate
case-insensitive compatibility behavior; reconcile that at the explicit profile
boundary rather than silently switching the default release to this candidate.


**S1 shared hierarchy:** `generate_ecore_hierarchy.py` generates native conformance
for all 175 resolved runtime Ecore classes. `relationship_declarations::metaclass_is`
now consumes it, replacing the handwritten runtime JSON ancestry closure. Unknown
classes fail closed; multiple inheritance is preserved. Other ancestry consumers
and abstract/interface construction checks remain open in the authoritative rows.
Three generator tests and the native hierarchy test pass. The final run passed all
142 existing 2026-08 regressions plus five focused ownership/hierarchy/legacy
controls with empty PATH and Java environment removed. Production compilation and
source-contract freshness pass. Logs: outer `target/support-2026-08/structural-hierarchy-*`.
No full sample comparison or timing assessment was repeated.


This expansion supports the existing G01-G16 batches; it does not replace their
completion gates or authorize another translator trial. The eleven-validator
trial stays bounded. Full comparisons and benchmarks remain at M2/M3.

- [x] Preserve complete Ecore XML containment and explicit attributes, including
  inheritance, types, bounds, containment, opposites, ordering, uniqueness,
  defaults, property flags, operations, annotations and delegate markers.
  The semantic export now retains `source_tree` and per-element
  `source_attributes`; omitted attributes remain omitted, not guessed defaults.
- [x] Inventory the existing resolved Xtext graph: 5,810 nodes across 18 kinds,
  including all serialized fields (predicates and cardinalities are fields).
  Reject newly observed kinds pending review. Grammar inheritance and references
  retain the existing exporter closure/link checks.
- [x] Generate [structural source coverage](structural-source-coverage.json) with
  input hashes, observed fields, all 398 delegate bindings, existing bounded
  consumers, and explicit external dependencies. Preservation, implementation and
  verification remain separate. Broad native coverage is **unassessed**, not zero
  and not complete. Ten Ecore preservation tests and three inventory tests pass.
- [x] Resolve effective runtime Ecore defaults and typed references with the actual
  EMF frontend. `ecore-effective.extract.json` exports 175 classes, 415 features
  and 70 operations. Cross-check all runtime class flags/inheritance and feature
  types, bounds, ordering, uniqueness, containment, opposites, derivation flags,
  declared defaults and ID flags against the structural extract. Explicitly record
  its legacy SysML naming of Ecore EString/EBoolean/EInt/EDouble; all other compared
  discrepancies fail. Real EMF tests verify omitted defaults, explicit changes and
  rejection of unresolved types. This is metadata qualification, not algorithm
  execution. Non-class classifiers and generic declarations remain preserved in
  the complete source tree; the separate legacy kerml.ecore is outside this export.
- [ ] Migrate native consumers to resolved Ecore type identities and effective
  defaults; remove legacy primitive naming normalizations once consumers migrate.
  Preserve the distinction between Ecore defaults and custom generated-model
  constructor behavior (for example EnumerationDefinition variation).
- [ ] Map each applicable source definition to a native consumer and focused
  evidence. Replace unassessed entries only with checked bindings and tests;
  signatures and source provenance alone never qualify algorithms as supported.
- [x] Add the first native ownership consumer: generated contracts for
  `Element.ownedRelationship <-> Relationship.owningRelatedElement` and
  `Relationship.ownedRelatedElement <-> Element.owningRelationship`, sourced from
  resolved EMF metadata. Declaration memberships and relationship-owned annotations
  now attach both endpoints through `emit/ecore_ownership.rs`. The handwritten
  algorithm validates endpoint types and field shapes before mutation, maintains
  insertion order and uniqueness, and rejects conflicting containers. Generator
  tests reject unsupported multiplicity/flags/opposites. The coverage inventory
  names the generated file, native algorithm, callers and tests separately.
  The legacy KIR kind `SysML::Requirements::VerifyRequirementUsage` has an
  explicit compatibility binding to the resolved Xtext rule
  `RequirementVerificationUsage`; its return metaclass is generated, not guessed.
  Unknown kinds still fail. Validation: four generator tests and three native
  ownership tests pass; the 142-test release regression run passed 141 and exposed
  the legacy-kind failure, now fixed and passing on focused rerun with the legacy
  emission control. Six final focused native tests pass with empty PATH and Java
  environment removed; production `cargo check` and deterministic freshness pass.
  Full semantic comparisons and benchmarks were not repeated.
  Canonicalizing the legacy emitted kind and its
  specialized membership remains separate semantic work.
  This covers initial attachment only: no general reparenting, whole-graph cycle
  detection, delegate algorithms, or migration of other emitters is claimed.
- [ ] In G05/G07, continue consuming structural definitions for relationship construction,
  containment and opposites; qualify direct relationship-owned elements and
  namespace lookup separately. Keep handwritten scoping/linking explicit.
- [ ] Expand native grammar consumption by coherent families: inheritance/rule
  selection, terminals, alternatives/groups/cardinalities, assignments/actions,
  predicates and cross-references. Qualify parsing AND resulting model structure.
- [ ] Account for value conversion, scoping, linking, implicit model construction,
  derived property algorithms, operation/delegate dispatch, collection semantics
  and inverse maintenance outside grammar syntax. Unknown Ecore extensions are
  preserved as source only; they must be reviewed before execution credit.

## Source-derived implementation method (2026-09-29)

**Required architecture (user direction):** use extensive build-time tools to
minimize trial and error, ship no runtime Java dependency, and clearly delineate
handwritten Rust wherever automatic translation does not cover the behavior.

- **Generation boundary:** pinned JVM/Xtext/Xtend/EMF tools may run during explicit
  builds, regeneration, CI and qualification. Requiring a JDK in the build
  environment is acceptable. These tools produce versioned, reproducible artifacts
  and Rust source; builds may also consume checked-in generated outputs. The shipped
  compiler/library must require no JVM, JAR, JNI bridge, Java subprocess or network
  fallback. Release qualification must inspect dependencies/package contents and
  run smoke controls with Java unavailable; source inspection alone is not that
  qualification. WASM remains part of the existing release gates.
- **Deterministic first:** resolve upstream symbols/types and derive supported
  behavior before implementing it. Unknown constructs or missing dependencies
  produce actionable generation failures with source locations. Use source
  mutation tests and focused boundary controls to qualify the translation; use
  Pilot execution for independent verification and unresolved integration behavior.
  Full comparisons and timing remain reserved for M2/M3.
- **Explicit implementation ownership:** generated Rust/artifacts, handwritten
  Rust infrastructure, handwritten semantic bindings and unsupported/unassessed
  behavior are distinct classifications. Generated files identify their inputs
  and regeneration command and are not hand edited. A handwritten binding records
  its upstream symbol, native module/entrypoint, reason/scope, focused tests,
  qualification status and limitations. A generated caller does not turn a
  handwritten callee into generated coverage. Missing implementations never
  silently become null/default values or invoke Pilot at runtime.
- **Visible remaining work:** retain separate resolved, translated, Rust-generated,
  bound and independently qualified states. Report transitive handwritten getter,
  helper and runtime dependencies. The current scoped binding registry is
  `validation-bindings.json` beside the profile's typed validator artifact; it
  identifies handwritten Rust support and all ten semantic service bindings separately from
  generated predicates and the test-only reference interpreter.
  Its build-time checker emits a boundary report and rejects stale/missing
  dependency declarations. The [generated implementation boundary report](validation-implementation-boundary.json)
  records eleven selected rules, all ten semantic service bindings, source hashes and pending
  qualification; 37 focused checker tests pass. This is selected-method coverage,
  not an inventory claiming all Pilot methods are handled.

For the remaining gates, derive implementation contracts from pinned source
artifacts before running Pilot experiments. Keep the G01–G16 checklist and M2/M3
milestones; this changes the implementation method, not the completion criteria.
The first structural and bounded validator slices are implemented below. Extend
the source-derived pipeline in coherent semantic batches, keeping native model
services and generated predicate coverage separately accountable.

1. **Preserve the grammar structure.** Extend the existing grammar extraction
   beyond keywords, return metaclasses and a rule-call graph. Prefer exporting
   the parsed Xtext model with the upstream Xtext parser at generation time;
   do not add a JVM requirement to the native runtime. Preserve ordered groups,
   alternatives, cardinalities, assignments and their operators, object actions,
   cross-reference types, enum literal mappings, predicates, terminals, and
   imported/inherited rule identity. Keep rule identity qualified by grammar:
   SysML and KerML have distinct `RelationshipBody` rules. Reject unsupported
   constructs instead of silently omitting them. Establish full input coverage
   before calling an extract complete.
2. **Join the Ecore contract.** Reuse the metamodel extract for inheritance,
   containment, opposites, bounds, ordering, defaults, subsets and redefinitions.
   Extend it to retain operations, constraints/derivation documentation and
   delegate bindings, identifying the Ecore resources actually used by the
   pinned runtime. Documentation containing formulas is source evidence, not
   automatically an executable constraint. A delegate name is not its algorithm.
3. **Generate and consume focused contracts.** Start with relationship bodies,
   memberships, imports and exposes: derive permitted children, constructed
   objects, ownership fields and enum mappings. Generate shared tables and
   regression expectations where supported; trace remaining handwritten rules
   to explicit source paths and native implementations. Do not undertake a
   wholesale parser replacement before this slice demonstrates value.
4. **Account for semantics outside the grammar.** Inventory parser
   postprocessors, scope/link providers, Xtend validators, Java adapters and
   setting/invocation delegates under their existing G05–G13 owners. Extract
   supported declarative tables and predicates; implement procedural algorithms
   with focused tests. Unknown branches stay open. Check Pilot behavior against
   normative requirements when the sources disagree.
5. **Use Pilot to qualify integration.** Reuse observations only when their
   source, library, tool/configuration and input hashes match. Run isolated
   controls for unresolved semantics and boundaries between generated contracts
   and handwritten processing; reserve full semantic comparisons and timing
   trials for M2/M3. Grammar extraction cannot establish linking correctness,
   complete derived graphs or performance.

Concrete initial contracts: `SysML.xtext:47` limits relationship bodies to
`ownedRelationship += OwnedAnnotation`; `KerML.xtext:51–62` additionally permits
ordinary `ownedRelatedElement`. `SysML.xtext:2370` maps the token `expose` to
`VisibilityKind::protected`. These should be extracted and tested directly.
No new conformance gate is marked complete by this method change.

**First structural extraction implemented:** the upstream Xtext parser now
exports all 727 rules, 5,810 model nodes and 3,461 resolved references, including
116 explicitly identified inferred type nodes. The independent Ecore inventory
retains 398 delegate bindings and all 70 operation signatures, with source hashes
and documentation kept separate from executable semantics. A namespace projection
joins assignments to the existing Ecore field contracts and generates native
constants for query metaclasses, expose visibility and relationship-body slots.
Native alias/import parsing now uses the proper language body parser: ordinary
KerML children retain definitions/features, and SysML rejects those children.
This establishes parse preservation only; collecting/resolving/emitting ordinary
KerML relationship children remains G05.d. The historical `positive-*.sysml`
relationship-owned-elements fixtures are negatives under the generated contract;
the control runner now derives this expectation from the body slots.

Reproduce with `tools/export_pilot_grammar_structure.py --java-bin <JDK/bin>
--check`, `tools/extract_ecore_semantics.py --check`, and
`tools/generate_namespace_grammar_contracts.py` with the checked-in grammar,
metamodel, namespace-contract and generated-Rust paths plus `--check`.
The corresponding three `tools/test_*` suites exercise malformed/unresolved
source, relocation, deterministic extraction, delegate metadata, cross-language
ownership, Ecore drift and native-generation changes. Twenty-five focused native
namespace tests and the default Windows stack regression pass. The extractor
suites pass 11 grammar, nine Ecore and eight namespace-contract tests. No full Pilot comparison or benchmark was repeated.

**First resolved Xtend translation implemented:** the actual Xtend 2.38 frontend
loads and links `checkImport`, `checkAnnotation`, `checkReferenceUsage` and
`checkEnumerationDefinition`, exporting their complete ASTs with
resolved operators/getters/constants, inferred types and source spans. A bounded
compiler emits typed validation IR, including the resolved type hierarchy. The
Rust backend emits direct predicates with lexical locals, native control flow,
short-circuit expressions and ordered diagnostic arguments. Production dispatch
calls this generated Rust; the JSON interpreter is compiled only under
`cfg(test)` as a reference. Ten model service bindings and shared value/diagnostic
support remain explicitly handwritten Rust. The old text-matching namespace
extractor remains historical independent evidence and no longer drives these
native checks. Unsupported syntax, calls, field mutability, diagnostic forms,
IR fields/types or incompatible receivers fail generation; missing native getter
implementations fail before predicate execution.

An independent oracle invokes the actual compiled Pilot methods on 12 import
combinations, all eight annotation ownership combinations and both Boolean
values for each of the two scalar checks. The native test
compares complete ordered diagnostics with all 24 observations, including
severity, message, issue code, subject, feature, data and the explicit
`INSIGNIFICANT_INDEX` representation. It also checks matching source/JAR/commit
provenance. These controls validate the predicates; they do not validate the
controlled getter implementations, EMF graph completeness, dispatch, or source
locations. The known Pilot annotation inverse-delegate difference remains open
under its existing normative exception. Four translated methods do not close G05.

**Architecture direction:** retain upstream frontends as generation-time tools,
export versioned resolved structure and typed behavior, then generate native
implementation and a coverage/dependency report. Grammar AST, Ecore metadata and
Xtend behavior are distinct inputs; none alone supplies scope providers, setting
or invocation delegate algorithms, model mutation, or parser postprocessing.
The test-only interpreter supplies an executable IR reference for the Rust source
backend. Generated source, interpreter and the same independent Pilot predicate
observations agree for all 24 controls. No JVM belongs in the native runtime.

**Complete textual validator inventory implemented:** the upstream Xtend frontend
now inventories all three SysML/KerML textual validator sources: 113 declared
`@Check` methods and 43 helpers, with 750 distinct resolved member dependencies.
All 113 checks resolve. The OWL validator is recorded as outside this textual
language scope. Package fragments and synthetic receiver nodes are identified
explicitly through Xtext APIs instead of being treated as unresolved operations
or assigned invented source locations. Constructor calls are included.

The [complete implementation inventory](validator-implementation-inventory.json)
separates eleven generated checks from 102 not yet translated, including five empty
upstream check bodies. Empty bodies are not promoted to full native coverage.
It retains direct calls, source-helper call chains (including cycles), unsupported
AST features, mutable locals, generic/null-safe calls and unbound model services.
External Java bodies and dynamic dispatch remain outside the source-helper graph.
The prior handwritten ledger's 29 partial/84 untraced statuses remain historical
native evidence, separate from source resolution and generated implementation.
A getter implemented for a selected import/annotation adapter does not qualify
other callers automatically. Input, exporter, JAR and generated-rule provenance
must agree, and missing/duplicated calls or checks fail the inventory checks.

Sixteen focused inventory tests pass, including four real Xtend source mutations:
a renamed check is discovered; a commented annotation no longer registers a check;
an unknown property remains an explicit resolution failure; invalid syntax fails
extraction. Deterministic regeneration and all source-contract checks pass. No
native behavior changed in this inventory batch, so full comparisons and timing
were not repeated. CI checks inventory/report freshness and the static tests;
the full mutation suite is enabled with `MERCURIO_INVENTORY_JAVA_BIN=<JDK/bin>`.

**Scalar translator batch implemented:** both SysML checks generate without new
IR operators. The exporter now resolves both validator sources together, retaining
actual constant origins and resource-specific inferred types. The two native
bindings explicitly read derived `is_reference` and definition `is_variation`;
missing reference flags fail closed. Flag derivation and validation traverse
alias/import body usages as well as ordinary declarations. Flag assignment retains
values for repeated feature representations and distinguishes metaclasses sharing
a qualified name. Partial-library fixtures derive only provable reference/composite
flags; they do not invent ancestry-dependent results. Enumeration definitions
now receive Pilot's implicit `isVariation=true` default; actual Pilot model
construction independently confirms that default. This default and flag derivation
remain handwritten Rust. Pilot's reference predicate reports
`validateAttributeUsageIsReferential`; generated diagnostics retain that observed
identifier despite the source comment naming the reference constraint.

**Frozen translator bound:** `bounded-translation-scope.json` selects eleven
whole validators: the four implemented checks plus `checkEnumerationUsage`,
`checkAnalysisCaseUsage`, `checkVerificationCaseUsage`, `checkUseCaseUsage`,
`checkRenderingUsage`, `checkViewpointUsage`, and `checkMetadataUsage`. The sole
helper is `checkOneType`. `checkAllTypes`, guarded callers and multi-helper callers
are explicitly outside this trial. The scope includes separate mandatory native
service, diagnostic, source mutation, runtime integration and no-Java gates.

The complete resolved AST for all twelve methods is exported separately in
`validators.bounded.ast.extract.json`. The [bounded progress report](bounded-translation-progress.json)
checks exact selection and rejects a family caller whose complete body is anything
other than the selected helper call. The current metric is **11/11 generated checks
(100% by count), 1/1 helpers resolved and generated by specialization**. This is neither a
schedule estimate nor an overall conformance percentage. The report deliberately
cannot declare the trial qualified solely from generated-check count.

**Type-family implementation delivered:** the complete `checkOneType` body is
translated node by node and specialized into each caller. Supported additions are
Java int literals/equality, the resolved EList-to-array length operation, a pure
single-argument `exists` closure, model `Class.isInstance`, literal/local helper
arguments, a terminal unused Boolean return, and implicit-subject diagnostics.
Diagnostic feature tokens are resolved from the actual pinned Ecore singleton at
build time. Other helper constructs fail closed; recursion, mutable locals and
early returns remain unsupported. There is no handwritten reconstruction of the
helper predicate. Production dispatch and applicable metaclasses are generated.

The seven selected contexts now bypass the old handwritten cardinality/type loop.
Native `all_types`, metaclass ancestry, value/service contracts and diagnostic
location mapping remain explicitly handwritten. Missing type metaclasses fail
closed. Native diagnostics retain the Ecore feature token in text and currently
use declaration spans. Complete derived type graphs and exact feature-source
locations are not claimed. The independent oracle executes actual Pilot callers
and helper with real model classes, controlled `FeatureAdapter.getAllTypes`, and
explicit validator `currentObject`: 28 new cases cover empty, matching, wrong and
multiple types. All 52 total observations agree with generated Rust and the
test-only interpreter. Twelve focused helper mutation/rejection tests pass.

The type-family library regression run passed **480/482** tests. The only failures
are the two already-recorded G05 cases below; no new failures appeared. Existing
single-type source controls still cover valid, multiple, redundant and inherited
types through native compilation. Export mutation tests pass 17/17; typed-family
mutation tests pass 12/12, the original translator 26/26, Rust backend 21/21,
binding checker 37/37 and bounded-scope checker 9/9. All 40 focused native tests
pass with an empty child PATH and Java/Pilot environment settings removed,
including the existing single-type source controls. Production `cargo check` and
all deterministic source-contract checks pass. These qualify the bounded native
path, not every shipped product/platform. Source tests and predicate
observations are separate evidence from specification compliance. Retained logs
are under `../target/support-2026-08/type-family-*`.

**Trial payoff and stop boundary:** source-driven generation across seven reused
callers is demonstrated. It did require substantial compiler/support work, so this
is not evidence of a net development-time saving. Keep the current supported
subset and explicit native services. Do not expand into `checkAllTypes`, general
Java/EMF execution, or other helper families merely to increase the count. Resume
G05–G16 semantic completion after the trial's focused regression/freshness checks;
full comparisons and benchmarks remain M2/M3. Specification compliance remains a
separate acceptance criterion from Pilot predicate agreement.

Scalar-batch regression evidence: the broad 481-test library run initially passed
465 and failed 16. After repairing flag propagation and partial-library derivation,
all 14 affected failures passed exact reruns, along with the 38 focused controls.
The whole library suite was not rerun after that repair. Two failures in unchanged
paths remain assigned to G05: `resolve_type_reference_prefers_local_definition_over_stdlib_alias`
(the zero-line synthetic-reference shortcut selects the library alias) and
`transpiles_package_and_usage_metatype_anchors` (its fixture lacks a resolvable
`SysML` import target). These remain open; the library suite is not claimed green.
Retained logs: `../target/support-2026-08/scalar-library-regressions.log`,
`scalar-repaired-regressions.json`, and `generated-validation-no-java.json`.

No general Xtend compiler is a completion requirement. Return to the remaining
G05–G16 semantic gaps after this bounded trial. Full comparisons/benchmarks stay M2/M3.

Reproduce all structural/validator artifact freshness and the bounded executable
oracle with `python -B tools/check_source_contracts.py --java-bin <JDK/bin>`.
Focused native filters are `validation_rules`, `annotation_checks`,
`translated_import_check`, `translated_scalar` and `release_2026_08_namespace`. All 38 focused tests
pass, including nine runtime/reference tests, the annotation/import binding
controls, two scalar controls and 25 namespace regressions. The test executable was freshly built and
run directly with an empty child PATH and Java/Pilot environment settings removed,
using `tools/check_native_validator_runtime.py --test-binary <built-test-executable>
--out <evidence.json>`. This qualifies the bounded execution path; it does not
certify all shipped products/targets or audit hardcoded external process paths.
The production library also passes `cargo check --locked -p mercurio-sysml --lib`.

Sixteen exporter tests pass, including actual SysML predicate mutation and
cross-source constant provenance. The Rust generator's 21 focused tests compile and execute generated code and
cover source-order diagnostics, lexical scope, escaping, short-circuiting,
receiver inheritance and unsupported-IR rejection. Twenty-six translator and
37 implementation-boundary tests pass. CI now checks translation, Rust generation
and binding freshness before the existing workspace tests. The generated Rust
and generation manifest retain input/output/generator hashes; the boundary report
verifies exact selected-rule/dependency coverage and test-only interpreter gating.
All source-contract checks pass. No full comparison or benchmark was repeated.
Whole-model support and translation/native qualification of the remaining checks
remain open. Source inventory and dependency classification are now complete for
the declared textual checks; the frozen eleven-check shared-family trial is specified above.

## Starting evidence and current state

**Current batch status:** G01–G04 and **M1 are complete**. G05 / B2a is active;
all later gates remain open. The candidate library remains unpromoted.

The current measurement baseline is the
[M1 declaration assessment](../2026-08-declaration-m1-assessment/README.md), completed
2026-09-28. It records 310/310 native and Pilot sample acceptances, 114 shared
controls, 513 Foundation / 443 SysML / 116 candidate tests, WASM/workspace/generator
checks, 310 detailed comparisons, and 48/48 successful timing trials. All 17
original metaclass pairs and all 240 original property rows are verified repaired.
The prior [declaration/feature-legality assessment](../2026-08-declaration-validation-assessment/README.md)
remains immutable historical evidence.

| Current measured observation | Assigned work |
| --- | --- |
| Zero named metaclass, direct/additional Boolean, short-name or direction discrepancies; all original pairs/rows retained | G01–G04 complete at M1 |
| 3,723 reference discrepancies; 4,073 reference fields with no counterpart | G06–G07; expressions/metadata as indicated by the owning object |
| 139 Pilot named anchors without native pairs, 1 ambiguous anchor, 1,718 native-only named anchors | G05 |
| 16/504 matched Pilot initializers without native initializer IR; ordered trees: 7 different, 188 equal, 309 unassessed | G08–G09 |
| 113 validator methods: 29 partial, 84 untraced; 467 extracted issue/message constants | G05–G13; exhaustive method assignment below |
| 6,036 missing additional-Boolean counterparts; large anonymous/implicit raw graph differences | G07 and the owning semantic batches |
| Runtime/bundle qualification and default-baseline promotion incomplete | G14–G16 |

M1 retains 6,660 unique named pairs. Of 22,630 checked reference fields, 18,907
agree under the documented projection; this remains a triage projection, not a
lossless graph-equivalence claim. Across eight benchmark groups, native median
process times are 0.71–5.03 seconds and Pilot medians are 8.66–18.90 seconds.
These compare current workflows with different semantic coverage, not a speedup
claim for a fully conformant compiler.

Counts overlap and are not independent defect counts. Every discrepancy needs a
repair or a testable, lossless representation explanation. Missing objects, fields,
unsupported expressions and ambiguous pairs remain unassessed until resolved.
Pilot agreement alone does not prove every normative SysML/KerML requirement.

## M1 recovery after workspace relocation (2026-09-28)

The existing compliance worktree moved to `D:/dev/repos/mercurio-labs/mercurio/worktrees/sysml-v2-compliance-2026-08`.
Its source changes and published historical reports survived, but the user
confirmed the ignored `target` artifacts were deleted. M1 therefore used fresh
artifacts; the earlier 513 Foundation / 443 SysML / 116 candidate passes and WASM
pass are historical observations, not restored log evidence.

Recovery is now repository tooling, not a script kept under `target`:
[inputs and library](../../../tools/restore_release_assessment_inputs.py),
[114 controls and 240 original property rows](../../../tools/run_declaration_milestone_controls.py),
and [milestone gates](../../../tools/run_declaration_milestone.py).
Pinned commits/JAR and all 310 source hashes match the retained baseline. Library
path/time provenance is freshly generated and gets a new digest. Ninety-three
control sources match retained hashes; the other 21 received fresh isolated Pilot
checks. All deleted raw Pilot graph exports were regenerated for this comparison. No historical report is overwritten.

Fresh recovery gates now pass: the release tools build; the imported library has
17,166 elements (SHA-256 `551e16553396b3d34c50442b13079994b48d3481eab5aba5f2ab84dd82975476`);
all 310 sample contexts compile; all 114 controls match expected acceptance and
required diagnostics; all 240 original property rows match. The 21 refreshed
Pilot controls include seven accepted and 14 rejected inputs. Fresh release-profile
suites pass 513 Foundation, 443 SysML and 116 candidate tests; WASM, workspace
all-targets and field-generator checks also pass. Two assessment handoff regressions
pass. The native comparison snapshot freezes 558 source files and three tool binaries;
the exact candidate library is also retained. All 310 comparisons, 17 original
metaclass repair assertions and 48 timing trials pass. Final source/binary/helper
verification passes, and compact evidence is published in the M1 report.

## Execution checklist

A checked item means its stated acceptance evidence exists. Passing a narrow
control never closes an entire validator method or semantic family. The milestone
items below independently gate full-comparison claims.

### B1 — Declarations, memberships and declared/derived flags (complete at M1)

- [x] **G01 — Requirement constraint representation, focused scope.** Emit real
  `ConstraintUsage` and `RequirementConstraintMembership`, membership kind and
  visibility, reciprocal ownership, distinct anonymous identities and actual
  `ReferenceSubsetting`. Preserve legacy role ID prefixes through a named overlay.
  Four regressions cover named/anonymous forms, qualified targets, Pilot's observed
  anonymous-reference shadowing, minimal types, and a wrong-type rejection.
  [Fresh Pilot controls](../../../crates/mercurio-tools/corpus/release-2026-08/requirement-constraint-pilot-results.json)
  retain the duplicate-effective-name warnings; warning parity and complete
  `checkRequirementConstraintMembership` are still G05/G11. M1 verifies the
  four original corpus metaclass repairs on retained pairs.
- [x] **G02 — Named KerML relationship declarations, focused scope complete.** Batch
  the 11 baseline cases listed below through shared parse/collect/emit paths.
  Preserve actual relationship metaclasses, names/short names, source locations,
  owner, ordered source/target endpoints and relationship-specific references.
  Test each relationship family with a valid named declaration, anonymous form,
  qualified endpoints and an invalid/missing endpoint where applicable. Compare
  the focused relationship projection with fresh Pilot controls; no placeholder
  `Feature` objects and no disappearance counted as a repair.
  [G02 evidence](relationship-declarations-evidence.json) verifies 11 original
  metaclass repairs and 67 ordered reference fields across five original files,
  retaining unique declaration pairs. The inline inverse endpoint also preserves
  its anonymous Feature, two FeatureChaining objects, ordered steps and type.
  Five focused tests, all 107 candidate tests, 310 corpus compilations and the
  construct-seed drift check pass. Nine fresh missing-endpoint controls reject;
  exact diagnostic-code parity and full relationship legality remain G06/G13.
- [x] **G03 — Textual representations and invariants, focused scope.** Fix the two listed cases
  together with their ownership/annotation links, language/body text and invariant
  expression/result representation. Focused tests must preserve body text and
  distinguish the representation from its annotated declaration; use Pilot
  positive/negative syntax controls and a parse/authoring round trip.
  [Evidence](textual-declarations-evidence.json) verifies both original metaclass
  repairs and four original representation bodies/owners. Complete nested
  expression graphs and diagnostic parity remain explicitly assigned to G08/G09.
- [x] **G04 — Declared names, direction and derived flags.** Resolve all 54 measured
  Boolean discrepancies and 186 short-name/direction rows in the linked difference
  inventory. Group controls by association/crossing ends, assignment-created
  features, temporary trigger payloads and control-structure usage. Trace defaults
  to pinned postprocessors/setting delegates, including explicit overrides.
  Assert actual fields and retained identities; never turn a missing field into
  equality. Add positive/negative flag interactions to the same focused suites.
  [Evidence](flag-name-evidence.json) and [all 240 verified rows](flag-name-repairs.json)
  cover every recorded tuple across 54 original files; 115 candidate tests pass.

**B1 exit:** G01–G04 accepted, relevant profile generators pass `--check`, candidate
regressions pass, then M1. Further structural or expression gaps discovered here
are assigned to G05–G13 with the affected source/line and property.

### B2 — Structural graph and its legality (B2a active)

- [ ] **G05 / B2a — Namespaces, membership and identity (9 validator methods).**
  Triage all 139 unmatched Pilot anchors, the ambiguous anchor and 1,718 native-only
  anchors into actual missing semantics, generated objects or pairing defects.
  Preserve declared versus derived names, visibility, imports/re-exports and
  distinguishability diagnostics. Test public/private/protected membership,
  nested/qualified imports, duplicate names, unnamed siblings and stable identity
  through authoring. Close all branches of the assigned namespace/base-usage
  validators using shared lookup rather than surface-specific checks.

**G05 execution order (one active checklist):**

- [x] **G05.a — Reconcile the M1 anchor inventory.** The
  [inventory](namespace-anchor-inventory.json) accounts for all 139 Pilot-only,
  1,718 native-only and one ambiguous named anchor. Its rows remain unassessed;
  a same-name candidate is evidence for investigation, not an equivalence waiver.
- [x] **G05.b — Source preservation and import prefix batch.** Preserve metadata
  prefixes and declaration locations in both parsers; retain locale comments and
  following declarations; share canonical Comment identity and package/comment
  annotation lookup. Preserve visibility, `import all`, aliases, comment bodies
  and locales through authoring. Execute the extracted `checkImport` top-level
  visibility predicate in shared resolution and register both import flags in
  the generated field contract. Focused acceptance evidence is linked below;
  this does not close annotation relationships or complete import graph semantics.
- [x] **G05.c — Canonical annotations and documentation.** Preserve named/short-name
  documentation, locale and exact source identity (including `Documentation Example.sysml:7`); anonymous comments and ordered multi-target annotations;
  actual Annotation relationships, reciprocal owner links and `checkAnnotation`
  ownership alternatives. Test both languages, nested owners, comment-about-comment,
  invalid targets and authoring round trips. Do not count a restored source anchor
  as complete graph equivalence.
  **Verified portion:** structured documentation headers now preserve long/short
  names, locale, raw editable body and source identity; emission normalizes body
  text like Pilot and creates `Documentation`, `OwningMembership`, documented/
  annotated element links and reciprocal owner documentation. Both languages and
  nested owners have round-trip/runtime checks. Eight Pilot/native controls and
  14 property rows include the original `Documentation Example.sysml:7` anchor.
  **Anonymous source documentation now verified:** `doc /*...*/` uses the same
  canonical member path, preserves exact spans and raw editable bodies, and retains
  distinct identities even on the same line. Empty, multiline and Unicode bodies
  match Pilot. Repeated localized text edits replace documentation inside its owner;
  legacy authoring doc values render as owned members instead of prefix syntax.
  **Explicit comment annotations now verified:** named/unnamed `comment` and
  locale forms preserve raw bodies and ordered typed targets through authoring.
  Each explicit target creates an actual comment-owned Annotation with its own
  source span and canonical source/target/ownership links; comments without
  `about` annotate their owning namespace. Quoted targets, comment-about-comment,
  comment-about-documentation and unresolved later targets are covered. Eight
  focused Pilot graph projections verify ownership, memberships and ordered
  endpoints; a deliberately reversed projection must fail.
  **Textual annotation batch verified:** bare block comments now retain distinct
  source identities and editable raw bodies. Comments, documentation and textual
  representations in explicit relationship bodies use OwnedAnnotation; programmatic
  AST docs use canonical ownership too. All eight ownership-presence combinations
  of the extracted `checkAnnotation` predicate are tested, including source-level
  self-annotation rejection. Textual annotations before constraint results preserve
  both members and the result. Twenty-four fresh Pilot/native controls, 26 selected
  reference-graph projections, eight prior annotation graphs, 67 prior source anchors,
  460 SysML / 48 authoring / 14 lexer / 16 candidate namespace tests and 310/310
  sample contexts pass. The pinned Pilot owning-annotation inverse defect is recorded
  with raw graphs and exact source hashes; it is not hidden as parity.
  **Alias/import body dependency verified:** eight isolated controls and twelve
  selected annotation graphs now preserve alias/import-owned documentation,
  comments and representations, including short-only aliases inside definitions.
  [Evidence](membership-bodies-graph-evidence.json) retains the pinned Pilot inverse
  exception and the wrong-owner negative control. Expose bodies now pass eight
  acceptance controls, twelve annotation graphs, wrong-owner rejection and 67 prior
  anchors ([evidence](expose-bodies-graph-evidence.json)). This closes the textual
  annotation scope of G05.c. Metadata annotating elements remain G12 and
  expression/lambda body coverage remains G08.
- [ ] **G05.d — Membership visibility, lookup and diagnostics.** Preserve alias/
  import/expose relationship bodies and their annotations, and use Namespace versus
  non-Namespace ownership (connectors are both Types and Relationships). Emit explicit
  owning/alias memberships and retain all visibility values through qualified,
  nested and recursive lookup/re-export. Prove private/protected names do not leak.
  Cover duplicate long/short names across owned, alias and inherited memberships,
  warning severity, same-element aliases and Pilot's excluded synthetic cases.
  Complete `checkNamespace` and the standard-library warning in `checkLibraryPackage`.
  **Current verified slice:** explicit owning/alias memberships and concrete named
  imports match [28 selected Pilot graphs](membership-identity-graph-evidence.json),
  including a wrong-target negative control. Source ownership order, visibility,
  alias long/short names and import membership identity are retained. Anonymous
  resource roots are distinct across files; authoring skips their unnamed path
  segment. Saved-view readers accept concrete Expose metaclasses. Scoped alias
  lookup replaces global alias bindings, including inherited feature aliases.
  [Eight wildcard visibility controls](namespace-wildcard-visibility-evidence.json)
  prove public aliases to private elements, `import all`, private-alias exclusion
  and exclusion of recursive descendants beneath private namespaces.
  **Additional verified slice:** [20 expose/import graphs](expose-identity-graph-evidence.json)
  now retain concrete membership targets and annotations; four implicit multiplicity
  memberships remain G06/G07. [Six direct-ownership graphs](relationship-owned-elements-identity-graph-evidence.json)
  verify KerML alias/import class and feature children, ordered containment and target
  preservation, with wrong-target controls. Recursive alias-body collection and
  definition-owned alias links have native regressions. The derived `owner` follows
  the [written specification](normative-ownership.json), distinct from `owningRelationship`.
  **Qualified local visibility now verified:** [eight Pilot/native controls](qualified-visibility-evidence.json)
  reject private/protected local members, private namespace prefixes and private
  aliases reached through qualification, while allowing lexical private access and
  public aliases to private elements. The same handwritten access service checks
  type, import, alias, specialization and structured expression references.
  [Normative clauses and boundaries](normative-visibility.json) distinguish this
  local declared-membership subset from complete namespace resolution. All eight
  prior wildcard/import-all controls still pass.
  **Inherited local visibility now verified:** [eight feature/alias controls](inherited-visibility-evidence.json)
  and [eight nested-type controls](inherited-types-evidence.json) match pinned Pilot
  acceptance. Shared native lookup admits inherited protected members, excludes
  private members, checks qualified paths as public, and preserves alias visibility
  independently of its target. Definition specializations and inheritance traversal
  now use that same lookup for nested general types. Native assertions verify the
  selected feature/type targets and members inherited through a nested general type.
  This is handwritten scoping over collected definitions, not generated Xtext behavior.
  **Named local import visibility now verified:** [eight Pilot/native controls](import-visibility-evidence.json)
  distinguish private and public re-exports, permit `import all` to reach private
  source members, and reject qualified access to a private imported membership.
  Short import bindings now resolve within their declaring namespace. An owned
  member takes precedence over a same-name import, and expose targets prefer a
  uniquely named local usage to an unrelated library suffix. Native controls assert
  the actual specialization target for every accepted case. This is still
  handwritten import and scope behavior; the new library candidate retains
  concrete Membership objects, but canonical library import identity is unqualified.
  **Imported feature references now verified:** [eight Pilot/native controls](import-feature-visibility-evidence.json)
  show that private imports cannot be recovered through the legacy feature or
  redefinition fallbacks in sibling packages, while local private imports,
  `import all`, public re-exports and public aliases to private features bind
  the expected targets. The import target resolver follows a public alias to
  its feature, then preserves that resolved identity through emission.
  **Named direct library member visibility now verified:** the pinned Pilot
  exporter records each named element's owning Membership visibility, and the
  cached native index applies it to direct, inherited and qualified lookup.
  [Eight Pilot/native controls and four target comparisons](library-visibility-evidence.json)
  cover public, protected and private library members; all 48 prior visibility
  controls still match. The candidate has 768 private and six protected named
  members. The newer candidate additionally retains concrete owning Membership
  objects and drives this visibility from their reciprocal links, with metadata
  fallback for older KIR. This does not qualify all library membership or import
  behavior.
  **Concrete library Membership graph now verified:** the build-time exporter
  retains [50,106 Membership-derived objects](library-membership-graph-evidence.json),
  including 8,482 `FeatureValue` objects. Stable source-fragment IDs and scalar
  Ecore cardinality preserve member, owning namespace and owning membership links.
  The audit checks reciprocal links and visibility for 10,168 named nonroot members,
  all 17,166 previously exported named elements, eight Pilot/native access controls
  and four resolved targets. The prior 48 visibility controls and 154 focused
  release tests pass against that candidate. Ninety-four root packages have no
  named owning namespace. The presence audit then identified 35,047 non-null
  `member_element` and 30,237 non-null owning-namespace references without export
  identities. The [new anonymous-identity candidate](library-anonymous-graph-evidence.json)
  adds 49,458 source-fragment-backed anonymous elements and preserves all 50,106
  of each Membership link in strict native KIR. Eight Pilot/native access controls
  and four resolved targets still match. Other anonymous relationships, library
  imports and their algorithms, full graph comparison and namespace linking remain
  open under G05/G07; the expanded candidate is unpromoted.
  **Next work, in order:** qualify remaining relationship-body element kinds and
  their lexical scope/linking; enforce remaining imported-library-member
  visibility, ambiguity, shadowing, redefinition-specific and expression-specific
  scopes; complete distinguishability and warning delivery. The concrete candidate
  library graph still needs canonical library import and anonymous-member behavior
  under G07/G14. The selected graph projections do not close those requirements.
- [ ] **G05.e — Remaining base-usage predicates.** Complete `checkElement` implied
  relationship consistency, Definition/Usage variation memberships and specialization,
  reference-usage referentiality, every `checkUsage` branch (including parameter/objective
  exceptions), and variant owning-namespace legality. Keep one positive and one
  negative control for each reachable branch; record derived invariants explicitly.
- [ ] **G05.f — Close or assign every inventoried identity.** Verify repairs against
  the original tuples; classify generated/derived names and the ambiguous `else`
  without relaxing pairing. Retain a concrete semantic owner for unresolved rows:
  expression/lambda members G08/G09, implicit transition payloads/effects G10,
  concern/rendering members G11/G12. G05 closes only when its remaining rows and
  all nine assigned methods have evidence, not when the source audit passes.

- [ ] **G06 / B2b — Types, feature relationships and multiplicity (36 methods).**
  Complete computed conjugation, CrossSubsetting, inherited crossings and
  Cartesian-product featuring types; minimal typing, redefinition/subsetting,
  association/connector/interface ends and multiplicity legality. Use fresh
  controls for inherited and local ends, conjugated ports, cyclic/invalid
  specialization, conflicting/minimal types and multiplicity bounds/order.
  Integrate every assigned validator branch while implementing its shared graph
  operation. Two existing `checkFeature` predicates remain partial until the
  other branches and computed owning-type cases pass.
- [ ] **G07 — Lossless graph comparison contract.** Preserve relationship object
  identity, ownership, multiplicity, ordered references and duplicates, including
  anonymous memberships, implicit typing and result objects. Account for every
  baseline reference difference and missing counterpart with a semantic owner.
  Extend projections only with documented transformations and tests showing that
  order, identity and cardinality cannot be silently lost. Keep raw snapshots and
  residual lists. No raw-byte-equality requirement, blanket exclusions or missing
  fields interpreted as defaults. Remaining expression/metadata obligations are
  closed by G08–G13 before M2.
  **Partial library graph evidence:** the unpromoted candidate retains 50,106
  Membership-derived objects, including `FeatureValue`, and verifies reciprocal
  identity, Ecore single-reference shape and visibility for 10,168 named nonroot
  members ([audit](library-anonymous-graph-evidence.json)). This is import and
  native consumption evidence for selected fields, not a lossless comparison of
  imports, derived opposites or the full sample graphs. The prior candidate lost
  35,047 non-null `member_element` and 30,237 non-null owning-namespace edges;
  stable anonymous identities restore all of those links. Other relation families
  still require source-driven cardinality, order and reciprocal graph qualification.
  **Anonymous collection targets now verified:** routing six library collection
  relations through those identities adds [69,989 links](library-anonymous-relations-evidence.json)
  with none lost: 34,970 namespace members, 20,056 featuring-type links, 14,467
  owned features and 496 specializations. The clean KIR preserves the same 116,730
  element IDs as the prior candidate; the Foundation normalizer no longer invents
  12,945 metamodel fields for anonymous owners. Eight Pilot/native access controls,
  four resolved targets, 154 focused release tests on the pre-cleanup graph, and
  a Foundation normalization test pass. This qualifies target identity for those
  selected relation targets; order and duplicates are qualified separately below,
  while full Ecore opposites remain open.
  **Ordered library collections now verified:** the exporter retains Pilot getter
  encounter order and duplicates, and the importer checks six pinned effective-Ecore
  collection contracts. The [ordered candidate audit](library-ordered-relations-evidence.json)
  compares 183,265 collections and 254,674 references exactly with persisted KIR,
  including 27,052 non-lexicographic order witnesses and one repeated
  `Feature.chainingFeature` target. Seven repeated `specializes` targets come from
  `TypeUtil.getSupertypesOf`, which appends owned-specialization generals and
  implicit generals to a list without deduplication. They are retained as Pilot
  derivation output, not treated as Ecore uniqueness violations; distinguishing
  the two contributing paths and native specialization behavior remains open. Restoring
  Pilot order changes 145 derived metafeature first-type selections; each now uses
  the source feature's first type, but multi-type behavior beyond that projection
  remains G06/G07. Three audit tests verify that reversed or collapsed sequences
  are rejected, the native library-index cache test retains ordered repeated
  specialization parents, and 154/154 focused release tests pass on the ordered candidate.
  Other anonymous relation families, full Ecore opposites and lossless sample graphs
  remain open.
  **Effective-Ecore reference contracts now checked at library import:** ten direct
  reference fields are checked against imported class inheritance, source and target
  types, lower/upper bounds, ordering and uniqueness. The full pinned export passes
  for [384,250 direct reference rows](library-ecore-reference-evidence.json); the
  116,730 candidate elements are unchanged. The Ecore-declared
  `Namespace.ownedMembership` / `Membership.membershipOwningNamespace` opposite is
  reciprocal for all 50,106 exported memberships, and import rejects a missing
  reverse edge. Four focused importer controls reject
  invalid source/target classes, overfull singular references, missing required
  Membership links and duplicate targets in unique collections. The 61,252
  TypeUtil-derived `specializes` rows are explicitly outside these direct Ecore
  contracts. Native construction/mutation, unexported reference families and
  reciprocal opposite behavior remain open.
  **Derived library opposites under qualification:** effective Ecore declares
  `Element.ownedElement` opposite `Element.owner` and
  `OwningMembership.ownedMemberElement` opposite `Element.owningMembership`.
  The native importer now materializes those missing inverse fields from the
  already-validated forward edges and applies Ecore's singular upper bound to
  `ownedMemberElement`. The [candidate audit](library-derived-opposites-evidence.json)
  verifies 45,384 `owned_element` and 45,232 `owned_member_element` references,
  exact inverse cardinality and forward-edge encounter order, 116,730 unchanged
  element IDs, and no prior property loss. Five focused importer tests, three
  audit fault tests and 154/154 release tests pass on the enriched candidate.
  The inverse-order comparison against
  Pilot's `ownedElement` getter and native update/removal behavior remain open.

**B2 exit:** focused graph/validator suites, relevant namespace/import/authoring
regressions and a corpus compilation check; full comparisons wait for M2.

### B3 — Expression preservation, typing and execution (17 methods)

- [ ] **G08 — Initializers and result graphs.** Resolve the 16 missing-IR cases and
  seven unequal trees, then classify and cover all 309 unassessed trees. Preserve
  FeatureValue, result/return/parameter memberships, ordered arguments, duplicate
  operands, named arguments and lambda body declarations. Test trigger payloads,
  lazy wrappers, nested lambdas and alternate operator spellings using targeted
  Pilot exports. G03 supplies invariant result-root IR and membership; complete
  nested operands, explicit return features and implicit binding objects here.
  Tree equality alone does not close typing or evaluation. The G05 documentation
  batch exposed a concrete additional dispatch case: `constraint c { true }`
  (with or without a preceding documentation member) does not take the dedicated
  result-expression path unless the `constraint` modifier is also present.
  Cover plain constraint result bodies here; requirement-owned `assume constraint`
  bodies with preceding documentation are preserved by G05's focused regression.
- [ ] **G09 — Shared expression semantics.** Complete remaining classification,
  identity, unit, higher-order, invocation and indexing forms, including expression
  result types and conversion/error behavior. Run shared-engine positive/negative
  tests and Pilot-backed value/type/diagnostic controls for each form; test native
  authoring round trips. Close all 17 assigned validator methods through the same
  semantic core consumed by execution and callers. Include the observed
  `validateBindingConnectorTypeConformance` warning in
  `invariant-invalid-result.kerml`; G03 records but does not close that diagnostic.

**B3 exit:** focused expression/typing/authoring tests and relevant Foundation
execution tests when changed; full comparisons wait for M2.

### B4 — Behavior, requirements and metadata, each with its legality

- [ ] **G10 / B4a — Behavior graph semantics (24 methods).** Close control-flow,
  assignment, accept/send/perform, loop/conditional, state/subaction/transition,
  succession and flow/end rules. Test valid and invalid parameter/endpoint roles,
  transition triggers/guards/effects, control-node branch cardinalities and nested
  ownership with shared Pilot controls. Reuse core expression/type services.
  Keep fixes in compiler/semantic services; simulation-engine internals remain
  outside this task unless separately authorized.
- [ ] **G11 / B4b — Requirements, constraints and cases (18 methods).** Close
  constraint typing and membership placement/kind, actor/stakeholder/subject roles,
  satisfaction, concern framing, objectives, analysis/verification/use-case
  references and include semantics. Cover valid/invalid ownership and target
  types, imported/inherited references, and diagnostics including G01's warnings.
  G01 representation tests do not establish these validator methods as complete.
- [ ] **G12 / B4c — Metadata and view semantics (9 methods).** Complete qualified
  metadata prefixes, memberships, expression evaluation, explicit application
  bodies and imported-library values, plus filtering/expose/rendering/viewpoint
  relationships. Test inherited/imported metaclasses, wrong metadata targets,
  body values, qualification, visibility and view/rendering membership legality,
  including relationship-owned MetadataUsage/MetadataFeature annotations, with fresh
  controls and authoring preservation tests.
- [ ] **G13 — Coverage closure.** Review every statement, helper/delegate dependency,
  reachable diagnostic branch and extracted issue/message mapping in all 113
  methods. Attach implementation and positive/negative control evidence per branch
  (warning severity/code included), or prove why a branch is unreachable under
  the pinned grammar/metamodel. An unsupported branch remains open. Ensure all
  467 inventory constants are accounted for without treating message constants
  as 467 independent constraints. Reconcile the pinned spec/grammar/metamodel
  crosswalk and semantic delegates beyond annotated validators; record and fix
  any normative obligations the Pilot oracle does not exercise. Generator drift
  checks and complete branch evidence are required before coverage is qualified.

**B4 exit:** each subsystem's focused suite and controls pass, G05–G13 close, then M2.

### B5 — Release candidate qualification

- [ ] **G14 — Bundle and integration.** Build/register the complete versioned
  library/profile bundle with source/runtime/generator provenance and reproducible
  imports. Qualify compile/query/validation/authoring/execution through shared
  native services, desktop-native callers and the browser WASM path, plus affected
  host/API adapters. Run the relevant workspace suites, Foundation tests and WASM
  build. Check dependency boundaries after any Cargo manifest change. Build success
  alone is insufficient; integration controls must exercise the shipped bundle.
- [ ] **G15 — Final M3 assessment.** Reproduce all 310 sample compilations in Pilot's
  same source contexts, all accumulated positive/negative controls, full detailed
  semantic comparison and paired performance assessment on the frozen candidate.
  Account for every residual with a tested lossless representation mapping; no
  semantic defect, missing evidence or unsupported required construct remains.
- [ ] **G16 — Promote the qualified baseline.** Only after G01–G15 and M1–M3 pass,
  switch the shipped/default registration to the qualified 2026-08 bundle and
  exercise the normal default-loading path. Record exact component/bundle hashes
  and final evidence. No publish, push or merge is implied by this local plan.

## Comparison and benchmark cadence

| Gate | Trigger | Checks |
| --- | --- | --- |
| M1 — Declaration milestone | G01–G04 closed | Full SysML unit suite and WASM check; 310 corpus compilations; accumulated controls; 310 detailed comparisons; 48 fresh timing trials |
| M2 — Semantic milestone | G05–G13 closed | Full affected suites and corpus/controls; 310 detailed comparisons using the completed projection; 48 timing trials |
| M3 — Release candidate | G14 ready and all implementation gaps closed | G15 frozen-source comparison/benchmark plus full bundle/integration qualification |

- [x] M1 passed on 2026-09-28; all G01–G04 gates and frozen comparisons/timings verified.
- [ ] M2 passed.
- [ ] M3 passed.

Within a batch, run only focused regressions, relevant generator drift checks and
small fresh Pilot controls needed to settle semantics. A corpus compile may be
warranted by shared parser/resolver changes. Do not rerun a full comparison or
benchmark after each fix. Use targeted profiling only for a concrete suspected
regression; it does not create another qualification checkpoint.

At each milestone, freeze/hash sources, candidate library, helper and binaries;
compare identical source sets and retain failures/unassessed cases. Reuse Pilot
exports only when their input/runtime/helper hashes match. Timing uses the existing
eight groups, three fresh-process trials per engine, alternating order, recording
medians/ranges and phase times. Record host/cache limitations and semantic workload
differences; do not claim conformance-equivalent speedup while workloads differ.
A failed milestone returns to its owning batch. Rerun affected checks after repair;
repeat the complete assessment only when a substantial correction or final gate
requires it. G16's default-loader change needs its focused smoke test, not a fourth
benchmark unless it changes the measured workload.

## Exact declaration targets and field inventory

These rows come from the last full
[semantic differences](../2026-08-declaration-validation-assessment/semantic-differences.json).
Paths below are relative to the pinned Pilot corpus. They are obligations to
recheck, not a claim that the current source still contains every old defect.

| Gap | Source | Line / name | Pilot metaclass |
| --- | --- | --- | --- |
| G01 | `sysml/src/examples/Simple Tests/RequirementTest.sysml` | 6 / `c1` | `ConstraintUsage` |
| G01 | `sysml/src/examples/Simple Tests/RequirementTest.sysml` | 16 / `c1` | `ConstraintUsage` |
| G03 | `sysml/src/examples/Simple Tests/TextualRepresentationTest.sysml` | 7 / `inOCL` | `TextualRepresentation` |
| G02 | `kerml/src/examples/Simple Tests/Classifiers.kerml` | 5 / `Super` | `Subclassification` |
| G02 | `kerml/src/examples/Simple Tests/Dependencies.kerml` | 11 / `Use` | `Dependency` |
| G02 | `kerml/src/examples/Simple Tests/Features.kerml` | 16 / `F` | `TypeFeaturing` |
| G02 | `kerml/src/examples/Simple Tests/Features.kerml` | 68 / `Redef` | `Redefinition` |
| G02 | `kerml/src/examples/Simple Tests/Features.kerml` | 45 / `Sub` | `Subsetting` |
| G02 | `kerml/src/examples/Simple Tests/Features.kerml` | 42 / `t1` | `FeatureTyping` |
| G02 | `kerml/src/examples/Simple Tests/Features.kerml` | 43 / `t2` | `FeatureTyping` |
| G02 | `kerml/src/examples/Simple Tests/Inverses.kerml` | 12 / `Invert` | `FeatureInverting` |
| G03 | `kerml/src/examples/Simple Tests/TextualRepresentation.kerml` | 6 / `x_constraint` | `Invariant` |
| G02 | `kerml/src/examples/Simple Tests/Types.kerml` | 17 / `Gen` | `Specialization` |
| G02 | `kerml/src/examples/Simple Tests/Types.kerml` | 25 / `c1` | `Conjugation` |
| G02 | `kerml/src/examples/Simple Tests/Types.kerml` | 26 / `c2` | `Conjugation` |
| G01 | `sysml/src/validation/08-Requirements/8-Requirements.sysml` | 111 / `fuelConstraint` | `ConstraintUsage` |
| G01 | `sysml/src/validation/08-Requirements/8-Requirements.sysml` | 129 / `fuelConstraint` | `ConstraintUsage` |

G04's 54 Boolean rows are: `is_variable` 18, `may_time_vary` 18,
`is_constant` 9, `is_end` 7, `is_composite` 1 and `is_reference` 1.
Its remaining field rows are `direction` 154 and `declared_short_name` 32.
Use the source/line/name/property tuples in the linked inventory; do not replace
these observed rows with a few representative tests and claim closure.

## Exhaustive validator assignment

The [generated method inventory](../../../crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/validators.extract.json)
remains the source of extracted facts; the
[coverage overlay](../../../crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/validator-coverage.overlay.json)
remains the source of implementation evidence. This table assigns work; it does
not upgrade any coverage status. All 113 distinct methods appear exactly once.
Names below include the `check` prefix; the column supplies the validator class.

| Batch | KerMLValidator methods | SysMLValidator methods |
| --- | --- | --- |
| B2a (9) | `checkElement`, `checkAnnotation`, `checkNamespace`, `checkImport`, `checkLibraryPackage` | `checkDefinition`, `checkReferenceUsage`, `checkUsage`, `checkVariantMembership` |
| B2b (36) | `checkSpecialization`, `checkType`, `checkClassifier`, `checkEndFeatureMembership`, `checkFeature`, `checkFeatureChaining`, `checkRedefinition`, `checkSubsetting`, `checkCrossSubsetting`, `checkDataType`, `checkClass`, `checkStructure`, `checkAssociation`, `checkBindingConnector`, `checkImplicitBindingConnectors`, `checkConnector`, `checkMultiplicityRange` | `checkAttributeDefinition`, `checkAttributeUsage`, `checkEnumerationDefinition`, `checkEnumerationUsage`, `checkEventOccurrenceUsage`, `checkOccurrenceDefinition`, `checkOccurrenceUsage`, `checkConjugatedPortDefinition`, `checkPortDefinition`, `checkPortUsage`, `checkConnectionUsage`, `checkFlowDefinition`, `checkFlowUsage`, `checkInterfaceDefinitionEnds`, `checkInterfaceUsageEnds`, `checkInterfaceUsage`, `checkAllocationUsage`, `checkDataType`, `checkClass` |
| B3 (17) | `checkParameterMembership`, `checkExpression`, `checkFunction`, `checkReturnParameterMembership`, `checkResultExpressionMembership`, `checkCollectExpression`, `checkFeatureChainExpression`, `checkFeatureReferenceExpression`, `checkInstantiationExpression`, `checkConstructionExpression`, `checkInvocationExpression`, `checkOperatorExpression`, `checkSelectExpression`, `checkIndexExpression`, `checkFeatureValue` | `checkCalculationUsage`, `checkOperatorExpression` |
| B4a (24) | `checkBehavior`, `checkFlow`, `checkFlowEnd` | `checkAcceptActionUsage`, `checkActionUsage`, `checkAssignmentActionUsage`, `checkTriggerInvocationExpression`, `checkControlNode`, `checkDecisionNode`, `checkForkNode`, `checkJoinNode`, `checkMergeNode`, `checkPerformActionUsage`, `checkSendActionUsage`, `checkForLoopActionUsage`, `checkIfActionUsage`, `checkWhileLoopActionUsage`, `checkExhibitStateUsage`, `checkStateDefinition`, `checkStateSubactionMembership`, `checkStateUsage`, `checkTransitionFeatureMembership`, `checkTransitionUsage`, `checkSuccession` |
| B4b (18) | — | `checkAssertConstraintUsage`, `checkConstraintUsage`, `checkActorMembership`, `checkFramedConcernUsage`, `checkRequirementConstraintMembership`, `checkRequirementDefinition`, `checkRequirementUsage`, `checkSatisfyRequirementUsage`, `checkStakeholderMembership`, `checkSubjectMembership`, `checkCaseDefinition`, `checkCaseUsage`, `checkObjectiveMembership`, `checkAnalysisCaseUsage`, `checkRequirementVerificationMembership`, `checkVerificationCaseUsage`, `checkIncludeUseCaseUsage`, `checkUseCaseUsage` |
| B4c (9) | `checkMetadataFeature`, `checkElementFilterMembership` | `checkExpose`, `checkRenderingUsage`, `checkViewDefinition`, `checkViewRenderingMembership`, `checkViewpointUsage`, `checkViewUsage`, `checkMetadataUsage` |

## Batch log

Keep one short entry per completed batch here. Update checkbox, evidence and
remaining obligation together; do not create another narrative checkpoint report
for a small fix. Full reports belong only to M1–M3.

| Batch/item | Result | Evidence / remaining obligation |
| --- | --- | --- |
| G01 focused implementation | 102 candidate tests passed; four requirement regressions included; 310 compilation preflight passed; construct seed current | `parser/release_2026_08_tests.rs` requirement tests and linked Pilot control JSON; raw logs in `target/support-2026-08/requirement-memberships-*` in the outer worktree. Full unit/WASM and corpus semantic repair verification deferred to M1. Membership legality remains G11. |
| G02 relationship declarations | 107 candidate tests passed, including five focused tests; 310 compilations passed; five original-case comparisons verify 11 metaclass repairs and 67 reference fields | [Evidence](relationship-declarations-evidence.json), [verified repairs](relationship-declarations-repairs.json), and fresh Pilot controls. Includes inline-chain identity/type/link preservation. No full comparison or benchmark; M1 remains pending G03/G04. |
| G03 textual declarations | 112 candidate tests passed, including five focused tests; 14 Foundation lexer and 47 authoring tests; 310/310 compilations; two original metaclass repairs and four language/body/owner matches | [Evidence](textual-declarations-evidence.json), [verified repairs](textual-declarations-repairs.json), seven fresh Pilot controls and construct seed current. Nested expression graph/type obligations remain G08/G09. No full comparison or benchmark; M1 remains pending G04. |
| G04 flags, names and directions | All 240 measured rows match across 54 original files; three focused and 115 candidate tests passed; strict boundaries passed | [Evidence](flag-name-evidence.json), [all verified rows](flag-name-repairs.json), three fresh Pilot controls, and audit failure-mode controls. Corpus-wide compilation/comparison and timing are reserved for M1 immediately after this batch. |
| M1 declaration milestone | 513 Foundation / 443 SysML / 116 candidate tests; WASM/workspace/generator; 310 native/Pilot acceptances; 114 controls; all 17 metaclass and 240 property repairs; 310 comparisons; 48 successful timing trials | [Frozen assessment and evidence](../2026-08-declaration-m1-assessment/README.md). Full support remains open. |
| G05.a–b source/import batch | 67/67 original source anchors restored (60 prefixed declarations and seven members following comments); 8/8 isolated Pilot/native import controls; eight focused tests with default and candidate libraries; 33 import, 14 lexer and 47 authoring regressions; 59 generated field contracts | [Reconciled inventory](namespace-anchor-inventory.json), [focused evidence](namespace-source-import-evidence.json), [fresh Pilot controls](namespace-import-pilot.json), and [reproducible runner](../../../tools/run_namespace_batch.py). Extracted top-level import visibility is covered. Annotation ownership, full membership/import graph semantics, remaining validator branches and identity triage stay open; no full comparison or benchmark rerun. |
| G05.c documentation headers/ownership | 8/8 Pilot/native controls; 14/14 Pilot property rows; 67/67 prior source anchors; nine namespace tests with default/candidate libraries; 14 lexer and 47 authoring regressions; 61 generated fields | [Evidence](documentation-header-evidence.json), [Pilot controls](documentation-pilot-controls.json), [audit](../../../tools/audit_documentation_batch.py). Structured documentation headers and ownership verified; anonymous legacy documentation and complete Annotation semantics remain open. No full comparison or benchmark. |
| G05.c anonymous documentation and editing | 16/16 Pilot/native controls; 32/32 documentation properties; 67/67 prior anchors; 454 SysML, 47 authoring, 14 lexer and 11 candidate namespace tests; 310/310 sample compilations | [Evidence](anonymous-documentation-evidence.json), [Pilot controls](anonymous-documentation-pilot-controls.json), [audit runner](../../../tools/audit_anonymous_documentation_batch.py). Anonymous documentation keeps raw text, source identity and canonical ownership. Repeated edits remain localized; documentation before assumption results preserves both. Annotation relationships and programmatic AST documentation emission remain open; the observed plain-constraint result-dispatch gap is assigned to G08. No full semantic comparison or benchmark. |
| G05.c explicit comment/Annotation graphs | 8/8 Pilot/native controls; 8/8 source-anchored graph comparisons; reversed-order rejection; 67/67 prior anchors; 455 SysML, 47 authoring, 14 lexer and 12 candidate namespace tests; workspace all-targets and 67 field contracts | [Graph evidence](annotation-graph-evidence.json), [Pilot controls](annotations-pilot-controls.json), [audit runner](../../../tools/audit_annotation_batch.py). Ordered explicit comment targets and namespace ownership are verified. Relationship-owned annotations, complete `checkAnnotation`, bare regular comments and programmatic documentation emission remain open. No full comparison or benchmark. |
| G05.c textual annotation ownership and syntax | 24/24 fresh Pilot/native controls; 26 selected reference-graph checks with an explicit Pilot inverse exception; eight prior annotation graphs and 67 prior anchors; 460 SysML / 48 authoring / 14 lexer / 16 candidate namespace tests; 310/310 sample contexts; workspace and 69 field contracts | [Completion evidence](annotation-completion-evidence.json), [owned annotations](owned-annotations-graph-evidence.json), [bare comments](bare-comments-graph-evidence.json), [runner](../../../tools/verify_annotation_completion.py). Complete emitted-Annotation ownership predicates, self-annotation rejection, canonical legacy docs, and textual annotations before constraint results. Alias/import/expose bodies remain the concrete G05.d dependency. No full comparison or benchmark. |
| S1/S2 direct relationship ownership and G05 expose targets | 148 release tests; final seven relationship regressions; 6 direct-ownership and 20 expose/import Pilot graphs; 12 annotation graphs, 8 acceptance controls and 67 prior anchors; wrong-target/owner rejection; 15 source-contract tests | Native collector retains direct class/feature declarations and recursive bodies. Shared Ecore attachment preserves both ownership levels and ordering; shared identity finalization covers alias/import/body references. Full native suite passed 489 with the candidate override plus the release-locator test separately before the final bounded reference/owner edits. Those final edits passed the listed focused regressions. G05.d visibility, remaining body kinds, implicit memberships and full structural qualification remain open. No benchmarks. |
| G05.d qualified local membership visibility | 150 release tests; 8 fresh Pilot/native qualified controls plus 8 wildcard/import-all regressions; 4 additional native reference-role controls; 11 coverage-accounting tests and current source hashes | Shared handwritten qualified-access service checks local membership paths before binding. It preserves lexical private access and public aliases to private elements, and reports the source span of inaccessible paths. Inherited/imported/library membership visibility, protected inheritance and specialized expression scopes remain open. No full comparison or benchmark. |

| G05.d inherited local features, aliases and nested types | 16 inherited Pilot/native acceptance controls plus 16 qualified/wildcard regressions; five focused inherited tests with target assertions; 494 native tests passed with the candidate, both release-selector tests passed separately after a Windows path assertion correction; 11 coverage tests and all deterministic source contracts | [Feature/alias evidence](inherited-visibility-evidence.json), [nested-type evidence](inherited-types-evidence.json), [normative scope](normative-visibility.json). Shared handwritten lookup distinguishes lexical, inherited and visible membership access, preserves public aliases to private targets, and resolves nested definition specializations and their inherited members. Imported/library visibility, ambiguity/shadowing, redefinition/expression-specific scope rules and complete namespace linking remain open. No full sample comparison or benchmark. |

| G05.d named local import scope and visibility | 8 new Pilot/native controls plus 32 prior visibility controls; 153 release tests; 28 membership/import and 20 expose Pilot graph matches with wrong-target rejection; 11 coverage-accounting tests and deterministic source contracts | [Import evidence](import-visibility-evidence.json) and [single coverage plan](../../../crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/structural-coverage-plan.json). Owner-scoped binding closes the demonstrated private-import leak across packages; qualification checks the import membership rather than the target. A same-name owned member and local expose target retain their correct graph identities. Library membership visibility, full imported-member paths and namespace linking remain open. No full sample comparison or benchmark. |

| G05.d imported feature-reference scope | 8 new Pilot/native feature controls plus 40 previous visibility controls; 154 release tests, 496 candidate-library native tests, 11 coverage tests and deterministic source contracts | [Imported-feature evidence](import-feature-visibility-evidence.json) and [coverage plan](../../../crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/structural-coverage-plan.json). Shared feature lookup and legacy fallback now honor declaring import scope; public alias imports retain their resolved feature identity. Complete library membership visibility, further import/redefinition/expression contexts and global namespace linking remain open. No full sample comparison or benchmark. |

| G05.d / G07 anonymous library identity and relationships | 50,106 Membership-derived and 49,458 anonymous target objects; all 50,106 member-element and owning-namespace links; 69,989 additional anonymous collection targets with zero prior links lost; 8 pinned Pilot/native controls and 4 resolved targets; 154/154 release tests on the expanded graph; focused cache and Foundation normalization tests | [Membership graph](library-anonymous-graph-evidence.json), [relation expansion](library-anonymous-relations-evidence.json) and [audit runner](../../../tools/audit_library_visibility.py). Build-time Pilot export assigns stable source-fragment IDs; strict native KIR preserves the selected links without extra anonymous-owner metafeatures, and native visibility consumes the Membership graph. Collection order and duplicates, other anonymous relations, library imports and Membership algorithms, full graph comparison and G05/G07 remain open. No full sample comparison or benchmark. |
| G07 ordered library collections | Exact sequence match for 183,265 Pilot collections and 254,674 persisted KIR references; 27,052 non-lexicographic witnesses and eight duplicate targets retained; 154/154 release tests, three audit fault tests and the focused native index-cache test pass | [Ordered candidate evidence](library-ordered-relations-evidence.json), [audit runner](../../../tools/audit_library_visibility.py) and [Ecore contract](../../../crates/mercurio-tools/src/bin/import_pilot_stdlib.rs). The exporter retains getter order and duplicate rows, the importer checks six pinned effective-Ecore collection contracts, and the native library index retains repeated specialization parents in order. TypeUtil's list derivation retains seven repeated `specializes` targets; its source-path decomposition and downstream semantic behavior, other relation families, full Ecore opposites and lossless sample graphs remain open. No full sample comparison or benchmark. |
| G07 direct library reference contracts | 384,250 rows in ten Ecore-mapped relations pass native class, target type, lower/upper bound, order and uniqueness checks; all 50,106 exported Namespace/Membership opposite pairs reciprocal; 116,730 candidate elements unchanged; four focused importer controls pass | [Reference evidence](library-ecore-reference-evidence.json) and [importer](../../../crates/mercurio-tools/src/bin/import_pilot_stdlib.rs). The 61,252 TypeUtil-derived specialization rows are classified separately. Other reference families, native construction/mutation, remaining opposites and sample graph comparison remain open. |
| G07 derived library opposites | 45,384 `owned_element` and 45,232 `owned_member_element` references added to a separate 116,730-element candidate from pinned Ecore opposites; no prior property loss; five importer tests, three audit fault tests and 154/154 release tests pass | [Derived-opposite evidence](library-derived-opposites-evidence.json), [importer](../../../crates/mercurio-tools/src/bin/import_pilot_stdlib.rs) and [audit](../../../tools/audit_library_derived_opposites.py). Ecore singular bound applied to `ownedMemberElement`; raw forward-edge order is preserved. Pilot `ownedElement` getter order, other opposite families, native update/removal and full sample graph comparison remain open. |
| G07 native getter and source edits | The Foundation derived getter now consumes an explicit ordered `owned_element` collection and derives `owner` from its inverse before legacy convenience relations; 13 focused derived tests pass. A same-file sibling-package lookup error was fixed; SysML add, move and remove now recompile with both Ecore ownership pairs reciprocal and no stale definition. | [Native source-edit regression](../../../crates/mercurio-sysml/src/authoring.rs) and `target/support-2026-08/source-mutation-opposites-test.log` in the outer worktree. Foundation changes are in the sibling component worktree. Pilot getter order, non-source mutation paths, other opposite families and cycles remain open. |
| S1/S2 KerML relationship-body choices | The pinned Xtext `OwnedRelatedElement` alternatives now generate 36 rule choices and the native parser checks its handwritten AST-to-rule adapter against them. Package, function and connector children retain reciprocal Ecore ownership, extending the prior class/feature cases. Eight generator controls and 155/155 candidate-library release tests pass. | [Grammar projection](../../../tools/generate_namespace_grammar_contracts.py), [native adapter](../../../crates/mercurio-sysml/src/namespace_grammar.rs) and [model-construction control](../../../crates/mercurio-sysml/src/parser/release_2026_08_tests.rs); log in outer `target/support-2026-08/relationship-body-choice-release-tests.log`. `Namespace`, `Multiplicity`, `Disjoining`, `Flow` and `SuccessionFlow` still lack concrete parser/model construction; full grammar alternatives, linking and Pilot semantic comparison remain open. No benchmark was repeated. |
| S1/S2 direct KerML Disjoining | The pinned `Disjoining` construct is selected into the generated seed. Native parsing and lowering now construct its two type endpoints, direct and named forms, and nested relationship-body ownership. The generated seed check, deterministic source contracts, focused ownership controls and all 503 native library tests pass. | [Parser](../../../crates/mercurio-sysml/src/kerml/parser/relationships.rs), [endpoint mapping](../../../crates/mercurio-sysml/resources/kernel/relationship-declarations.overlay.json), [construction control](../../../crates/mercurio-sysml/src/kerml/mod.rs). `DisjoiningPart` shorthand, delegate algorithms, feature-chain variants and Pilot semantic comparison remain open. Four direct relationship-body forms still lack concrete construction: `Namespace`, `Multiplicity`, `Flow`, `SuccessionFlow`. |
| S1/S2 KerML Namespace construction | The pinned `Namespace` construct is selected into the generated seed. Named and anonymous source forms inside a relationship body emit distinct Namespace objects; anonymous names remain internal. A focused control now exercises all three pinned `NamespaceBody` arms—member, alias and import—with preserved reference and owner links. It passes registered Ecore field checks; the construct seed, source contracts and all 504 native library tests passed before the added three-arm assertions. | [Parser](../../../crates/mercurio-sysml/src/kerml/parser.rs), [construct mapping](../../../crates/mercurio-sysml/resources/kernel/kerml-constructs.overlay.json), [construction control](../../../crates/mercurio-sysml/src/kerml/mod.rs). Complete member/linking semantics and Pilot comparison remain open; `Multiplicity`, `Flow`, `SuccessionFlow` still lack direct construction. |
| S1/S2 KerML MultiplicitySubset | The pinned `MultiplicitySubset` arm parses named and shorthand direct references plus dotted owned feature chains. The shared relationship resolver binds each chain step; the shared chain emitter constructs an owned `Feature` and ordered `FeatureChaining` relationships below the owned `Subsetting`. Focused controls verify both target forms, reciprocal ownership, ordered step IDs, registered Ecore fields and strict rejection of an unresolved final step. All 506 native library tests and deterministic source contracts pass. | [Parser](../../../crates/mercurio-sysml/src/kerml/parser.rs), [relationship emitter](../../../crates/mercurio-sysml/src/language_frontend/lowering/relationship_declarations.rs), [construction control](../../../crates/mercurio-sysml/src/kerml/mod.rs). General feature-chain linking and Pilot semantic comparison remain open. |
| S1/S2 KerML MultiplicityRange bounds | The pinned range arm parses one or two integer/infinity literal or feature-reference bounds and emits ordered owned `OwningMembership → Expression` objects before body members; feature references own a `Membership` cross-reference to the resolved feature. The owning memberships carry reciprocal namespace/member/owned-member links. Native `bound`, `lowerBound` and `upperBound` derivation now filters owned members by the generated Ecore `Expression` hierarchy and follows Pilot's first-two-member rule. Bounded native `valueOf` and `hasBounds` algorithms handle direct nonnegative EInt/infinity literals and missing-lower cases; an unevaluated feature reference returns Pilot's `-2` sentinel. Focused local, qualified, mixed-bound, unresolved-target, EInt-overflow, ordering, algorithm and registered-field controls pass. | [Parser](../../../crates/mercurio-sysml/src/kerml/parser.rs), [resolver](../../../crates/mercurio-sysml/src/language_frontend/lowering/resolve.rs), [range emitter](../../../crates/mercurio-sysml/src/language_frontend/lowering/emit/multiplicity_ranges.rs), [native derived algorithms](../../../crates/mercurio-sysml/src/kerml/multiplicity.rs), [construction controls](../../../crates/mercurio-sysml/src/kerml/mod.rs). The build-time pinned Pilot probe confirmed the direct feature-reference graph; the pinned Java setting/invocation delegates supplied the getter, `valueOf` and `hasBounds` algorithms. Other literal-expression forms, feature-reference evaluation, and full Pilot comparison remain open. |
| S1/S2 KerML Flow/SuccessionFlow graphs | Named and anonymous concrete `Flow`/`SuccessionFlow` declarations retain TypeBody members and reciprocal RelationshipBody ownership. The `from … to …` and `all <end> to <end>` arms construct ordered `EndFeatureMembership → FlowEnd → FeatureMembership → FlowFeature → Redefinition` graphs; dotted ends add `ReferenceSubsetting`, with longer prefixes reusing owned feature-chain construction. The `all` arm sets the Ecore-backed `is_sufficient` field. Payload typing accepts the pinned named `of item : Type`, anonymous `of : Type`, and direct `of Type` forms. The typed payload can own multiplicity before or after typing, reusing the shared `OwningMembership → MultiplicityRange →` bound-expression graph and preserving relationship order. Literal Flow ValuePart forms (`=`, `:=`, `default =`) own a `FeatureValue → LiteralExpression` graph with Ecore-backed flags and reciprocal owning-membership/member links. Native getters derive `flowEnd`, `payloadFeature`, `payloadType`, and the ValuePart expression. Focused endpoint, chain, sufficiency, payload-order, bound-reference, literal-value and rejection controls pass with strict registered-field validation. | [Parser](../../../crates/mercurio-sysml/src/kerml/parser.rs), [emitter](../../../crates/mercurio-sysml/src/language_frontend/lowering/emit/flows.rs), [shared range emitter](../../../crates/mercurio-sysml/src/language_frontend/lowering/emit/multiplicity_ranges.rs), [derived getters](../../../crates/mercurio-sysml/src/kerml/flow.rs), [construction controls](../../../crates/mercurio-sysml/src/kerml/mod.rs). Pinned Pilot probes confirmed the `FeatureValue` membership and typed payload multiplicity graph. Nonliteral OwnedExpression construction still needs operand features and library bindings; other PayloadFeature specialization and ValuePart forms, Natural typing/`Transfers::Transfer::payload` redefinition semantics, and full Pilot comparison remain open. |
| S1 Ecore constructor defaults | Fresh pinned Pilot factory construction observed 1,200 inherited explicit-default properties across 167 concrete classes. 1,151 match Ecore; `MembershipExpose` and `NamespaceExpose` override `Import.isImportAll=false` to `true`, which native expose lowering already emits; 47 detached `Feature.isVariable` getter calls require their delegate context and remain unqualified. Native source edits recompile through the shared default applicator; all 511 native library tests passed after that applicator was added. | [Constructor evidence](ecore-defaults-constructor-evidence.json), [reproducible audit](../../../tools/audit_ecore_defaults.py), [native default applicator](../../../crates/mercurio-sysml/src/language_frontend/lowering/ecore_defaults.rs). Attached Pilot source-model values, library import and direct non-source mutation still require qualification. No full comparison or benchmark was repeated. |
| S2 KerML non-Natural multiplicity literals | The pinned grammar's `LiteralExpression` choices admit Boolean, string and real values in `MultiplicityBounds`; Pilot parsed all three and reported `Must have a Natural value` during transformation. Native parsing now accepts the syntax, preserves quoted string contents including `..`, and reports the same semantic rejection. Eight focused multiplicity tests pass. | [Pilot control](multiplicity-literal-bounds-evidence.json), [fixture](../../../crates/mercurio-tools/corpus/release-2026-08/multiplicity-literal-bounds/invalid-nonnatural.kerml), [reproducible audit](../../../tools/audit_multiplicity_literals.py), [parser](../../../crates/mercurio-sysml/src/kerml/parser.rs) and [native rejection](../../../crates/mercurio-sysml/src/language_frontend/lowering/emit/multiplicity_ranges.rs). These invalid forms do not count as successfully constructed bounds; referenced-value evaluation and full grammar qualification remain open. No full comparison or benchmark was repeated. |
| S1/S2 resolved enum keywords | All 14 pinned Xtext enum rules and 24 literal declarations resolve to seven Ecore enums and their numeric values. Native SysML/KerML modifier recognition consumes generated visibility and direction keywords; `PortionKind` emits `snapshot`/`timeslice` values on `OccurrenceUsage`, and requirement membership kinds resolve generated `assume`/`require` literals. Focused generation fault checks, native strict-KIR controls and the requirement membership ownership test pass; 81 candidate fields are registered. The full 514-test native suite passed before the final requirement-kind wiring. | [Generated contract](../../../crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/enum-rules.extract.json), [generator](../../../tools/generate_release_enum_rules.py), [native consumer](../../../crates/mercurio-sysml/src/enum_grammar.rs), [requirement consumer](../../../crates/mercurio-sysml/src/language_frontend/lowering/emit/requirement_memberships.rs). Remaining enum rule consumers, full package/data-type identity, and Pilot value comparisons remain open. No full sample comparison or benchmark was repeated. |

**Next action: S1/S2 complete MultiplicityRange referenced-value semantics, nonliteral Flow ValuePart expression construction, and remaining PayloadFeature specialization/ValuePart forms, then G05.d lexical linking and visibility**, followed by
G05.e–G05.f above. Keep the M1 report and frozen source/library snapshot immutable.
Use focused checks for these batches. Full comparisons and benchmarks wait until
M2 after G05–G13; full 2026-08 support and candidate promotion remain open.

## 2026-09-30 contextual dispatch milestone record

## First end-to-end milestone and current boundary

Pinned Xtext enum rules are exported as a resolved mapping to seven Ecore enums (14 rules, 24 declarations). Generated Rust drives native visibility/direction modifier recognition and `PortionKind`/requirement kind construction; focused identity-failure, parsing, strict-KIR and membership tests pass. The remaining enum-rule contexts, Pilot value comparison and full grammar-driven object construction are **unsupported/unverified**. The same pattern must become a shared rule engine, not a series of hand-selected enum cases. [Contract](../../../crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/enum-rules.extract.json), [generator](../../../tools/generate_release_enum_rules.py), [native consumer](../../../crates/mercurio-sysml/src/enum_grammar.rs).

The next bounded path resolves every pinned Xtext assignment and action against effective Ecore, then generates a native containment-slot lookup consumed by RelationshipBody attachment. Four generator and one Rust focused tests pass. The [resolved contract](../../../crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/xtext-assignment-contracts.json) marks each row `semantic_verified=false`; unassigned-call current-object flow and scope/link algorithms remain explicit dependencies. The Ecore model service imports all 415 features; production currently validates four ownership references and 24 observed-default attributes, then reconciles 51 library attribute descriptors. Its shape/bounds/uniqueness checks are handwritten Rust driven by the pinned table.

The [terminal contract](../../../crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/xtext-terminal-programs.json) now generates 73 native recognition operations from all nine pinned terminal rules and retains 663 keyword occurrences. SysML and KerML parsers consume the shared recognizer for quoted-name/string boundaries; five focused native and seven generator tests pass. Token arbitration, incomplete exponents, overlapping comments, hidden-token policy, numeric grouping and other parser contexts remain open. Recognizing every terminal definition is not full lexical integration.

The [rule programs](../../../crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/xtext-fragment-programs.json) contain nine roots and 21 dependencies. The two production expression roots are now the actual pinned LiteralExpression and NullExpression: all five literal alternatives execute typed unassigned rule calls; two simple actions construct LiteralInfinity and NullExpression; attributes bind and validate through Ecore. RealValue executes its imported lexical sequence/choice/cardinality before converting the assembled value, without converting individual digit components to EInt. Source-derived numeric-language partitions reject overlapping integer/real alternatives. Identification remains in production; MemberPrefix and BasicFeaturePrefix remain executor-only. OwnedMultiplicityRange now drives production bound-tree construction; the SysML MultiplicityExpressionMember root is executor-only. The artifact lists 697 other rules outside this scope.

The containment milestone follows pinned OwnedMultiplicityRange -> MultiplicityBounds -> membership/expression rules -> Ecore-bound assignments -> native KIR. The shared interpreter preserves ordered child instances, resolves reciprocal containment slot identities and records qualified feature references as typed unresolved spellings. The production emitter consumes generated classes/attribute values and verifies reference syntax against inputs supplied by the existing Rust resolver. KIR identity allocation, scope/link lookup, derived membership properties and ownership attachment remain explicit handwritten services. The EString datatype matcher follows imported qualification groups, cardinalities and names, including global and quoted spellings.

Sixteen focused native controls pass, including ordered equal-valued children with distinct identities, inverse ownership, typed unresolved references, malformed syntax, transactional rollback and strict compiled KIR targets. Fourteen generator controls pass, including containment target/write-policy drift and unsupported datatype carriers. All 555 native library tests and 12 coverage-checklist tests pass. Fragment/terminal generation and coverage freshness checks pass. This batch does not rerun full Pilot comparisons or benchmarks; the candidate remains unpromoted.

Explicit boundaries remain: current-type flow now permits checked subtype actions and individual operand captures, but complete recursive action contexts remain unsupported. The new production consumer reparses normalized bound text from the handwritten parser; original-source whitespace/punctuation fidelity is not qualified. Numeric datatype execution consumes one legacy Number token; hidden-token policy and general lexical prediction remain unqualified. Boolean/signed-32 EInt/finite EDouble conversion is handwritten. String/name escape spelling is preserved without converter equivalence. Reference spelling/type capture does not establish scoping, linking, proxy loading or uniqueness after alias resolution. Other constructors, mutation/inverse maintenance, anonymous/short-only definition ASTs and normative/Pilot qualification remain open.

**Operand-construction milestone:** the native Ecore table now preserves all 328 imported setting-delegate records, including URI, binding status and candidate implementation classes. A named handwritten Rust service checks the operand binding and creates private ParameterMembership -> input Feature -> FeatureValue -> Expression through checked stored ownership. Its append behavior matches [actual pinned Pilot observations](operand-construction-pilot-controls.json) for all seven concrete invocation classes. The production invariant-result emitter uses it for complete literal/unary/eager-binary trees. Four focused native controls and five Ecore-generator tests pass, including order, distinct equal-valued operands, wrong types, collision/ownership rejection, failure atomicity, and strict nested-expression compilation. All 559 native library tests and 12 coverage-checklist tests pass after integration; generated Ecore and coverage freshness checks pass. Complete sample comparisons and benchmarks were not rerun, and the candidate remains unpromoted.

[Normative evidence](normative-operands.json) distinguishes this from full invocation semantics. Pilot's operand is an append-only parsing extension whose read facade is empty; it is neither stored containment nor the normative argument derivation. No disagreement is claimed for the observed construction subset. Function/type binding, parameter redefinition, result specialization, argument derivation, control-function bodies, reparenting and general list mutation remain unqualified. This dependency is Java adapter code; explicit handwritten Rust is the bounded implementation decision, not Xtend translation.

**Capture-action milestone:** the same program artifact now holds all 26 exact operand-capturing actions separately from its nine executable rule roots. Native construction executes each action's imported class/feature contract through the handwritten operand service. Five eager binary families in production (additive, multiplicative, exponentiation, equality and relational) select actions from adjacent imported operator assignments and retain the exact source-action identity in metadata. Individual action execution does not add enclosing-rule coverage: 697 rules remain outside this engine.

The generator propagates possible current types through admitted acyclic calls, sequential subtype actions, branch/optional joins and repetition fixed points; subsequent assignments must resolve to one Ecore feature identity across every reachable type. Seventeen generator tests, 17 native fragment tests and five operand-service tests pass. Controlled action-loop tests establish left association and rollback; all 26 source action contracts are checked against the existing Pilot wrapper-graph observations. Full pinned precedence, recursive-rule and predicate behavior are not claimed from these controls. All 561 native library tests and 12 coverage-checklist tests pass after integration; fragment generation, formatting and coverage freshness checks pass. No full sample comparison or benchmark was rerun.

**Recursive execution and resolution batch:** model-rule recursion now uses fixed-point FIRST/nullable analysis and rejects non-consuming cycles. Native execution rejects repeated calls at the same input position and more than 64 active rule calls; operation-specific stack frames fix a debug-build stack overflow found by the depth stress test. These controlled tests add no pinned rule roots: nine roots/30 rules remain admitted.

The upstream exporter previously loaded sibling languages together, allowing SysML overrides to contaminate inherited base calls. Each grammar now resolves in its own Xtext resource set. The artifact preserves source definitions and independent effective-rule/call maps: base 108/164, KerML 282/636, SysML 549/1,246. All four annotations are Override; neutral compilation rejects context-dependent calls; context-aware compilation is described below. Thirteen export tests (including actual Java relocation, malformed syntax and unresolved links) and 20 fragment generator tests pass. This corrects imported resolution; it does not qualify runtime inheritance. All 562 native library tests, 31 Xtext generator tests, 12 coverage tests, eight namespace tests and four ownership-generator tests pass; affected artifact freshness and formatting checks pass.

**Contextual dispatch batch:** production execution selects separately generated KerML and SysML programs. Upstream effective-rule/call maps specialize every admitted call before FIRST, recursion and type-flow checks; inherited root aliases are preserved and Override signatures checked. The shared literal AST adapter stays neutral only while every dependency program is identical across both languages. This replaces the blanket context-dispatch rejection for explicitly selected contexts.

Generated controls retain pinned inheritance, Override, call and ownership contracts while replacing ExpressionBody bodies with distinguishable literals. Native execution constructs the correct typed child and inverse containment, handles inherited root aliases and rejects sibling-language syntax. These are controlled dispatch semantics, not qualification of Pilot's full expression bodies. All 23 fragment-generator and 19 native fragment tests pass, including production identification/literal/multiplicity integration. No new pinned rule roots are claimed.

**Next implementation batch:** extend shared lexical/value adaptation for the KerML REGULAR_COMMENT dependency and predicate prediction for SysML CalculationBodyPart, then expand complete expression families. Keep source precedence, action construction, control-function body wrapping and normative argument derivation as separate gates. Full sample comparisons and benchmarks remain deferred; the candidate remains unpromoted.

JSON interchange consumes all 415 Ecore feature declarations through inherited lookup: 64 attributes and 351 references. Generated contracts supply names, enum/type domains, bounds and uniqueness; handwritten document-local linking resolves forward/exchange IDs and checks target classes. Importer/exporter v2 preserves canonical `owned_relationship`, `owned_related_element` and `related_element` separately instead of collapsing aliases. Ecore flags and opposite identities drive explicit graph consistency, single-container and cycle checks. External references retain their IDs with unresolved diagnostics; invalid payloads are preserved with errors. Focused controls cover mixed-class round trips, negative values/targets/shapes, identity collisions and contradictory graphs. Missing required values, embedded objects, inverse maintenance, derived containment/delegates and all-path persistence/mutation integration remain open. These checks do not establish complete model validation.

The JSON import report's `persistable_document()` revalidates current Ecore values and graph facts before native KIR normalization and strict validation. `sysml_json_field_registry()` supplies inherited, exact-kind shapes to Foundation's generic registry API. JSON → KIR serialization/reload → JSON controls preserve scalar/list attributes, class-dependent reference shapes and nonunique ordered references. Other persistence/loader paths still need this registry; no candidate has been promoted.

Ecore defaults now have a generated native source-model applicator and a library-import promotion path for marked Pilot exports with complete observed values. The exact `692170b` pinned tag and interactive jar were built using a process-scoped JDK selector workaround. A separate pristine pinned checkout supplies library inputs because Maven modified tracked library files in the build checkout. The refreshed export contains 109,826 elements and 445,502 relationships; an exporter guard rejects the earlier root-only result. The unpromoted native KIR passes strict persisted validation, and a field-by-field audit matches all 658,561 attached Pilot getter values. The generated Ecore service corrects 51 library attribute descriptors before persistence, including the three plural attributes. Foundation KIR now distinguishes scalar lists from references and scopes same-named fields by Ecore descendant kind. [Audit evidence](ecore-library-observations-evidence.json). Mutation/delegate behavior and full library qualification remain open.



## Checkpoint before complete-model Ecore handoff

# 2026-08 native-support critical path

**Full support remains open.** This is the single active plan for the existing compliance worktree. The [machine-readable coverage checklist](structural-source-coverage.json) retains every pinned Xtext/Ecore ID, native consumer, semantic dependency, verification anchor and full remaining obligation. [Prior batch evidence](remaining-gap-history.md) is historical, not an alternate plan.

The written 2026-08 specification is normative; pinned Pilot behavior is an independent comparator. Importing a definition, consuming it in native code and verifying its semantics are separate gates. Java is permitted for build-time extraction and evidence; production parsing and model services remain native.

## Architecture audit

| Boundary | Present | Missing for qualification |
|---|---|---|
| Xtext | Complete structural extract; 771 statically resolved assignments, 53 actions, generated enum/relationship slots and nine terminals; native rule executor drives named-definition Identification and all LiteralExpression alternatives plus NullExpression; OwnedMultiplicityRange builds typed containment/reference trees used by native KIR | Expand fragment execution to all rule families; resolve token arbitration, hidden-token policy, predicates, feature-capturing/subtype-changing actions and reference construction |
| Ecore | All 415 effective feature declarations in a generated native service; JSON consumes attribute/reference identities, types, bounds and explicit graph constraints; bounded defaults and kind-scoped scalar lists | Apply that service across every constructor, mutation and persistence path; implement inverse maintenance, operation/delegate algorithms and annotation effects |
| Xtend and external semantics | Pinned source exports and bounded validator translation trials | Per-rule decision and typed dependency binding; scoping/linking and delegates stay named handwritten Rust until supported by evidence |
| Qualification | Focused source freshness and native graph controls | Complete sample pass, normative disagreements, Pilot semantic comparison and comparable timing |

Extraction coverage is broad but execution coverage is uneven. The handwritten boundary currently includes native SysML/KerML parsers, scope/link resolution and many model emitters; the critical path moves their common syntax and structural semantics behind generated contracts without hiding external algorithms.

The bounded Xtend trial already translated `checkOneType` into generated Rust across seven callers and matched 52 controlled Pilot predicate observations. Its compiler/support cost did not demonstrate a net time saving. Retain that subset; assess only another rule with reuse and simple typed dependencies. Keep native `all_types`, Ecore ancestry, diagnostic locations and broader Java/EMF services explicitly handwritten. [Trial evidence](remaining-gap-history.md#source-derived-implementation-method-2026-09-29).

## Ordered critical path

1. **Close whole-body prediction.** All five first-set predicates and the hoisted CalculationBody predicate now come from upstream frontend services, with exact source identities/signatures and native probe controls. Eighteen decisions still exceed deterministic analysis bounds and block complete expression/body closures. A separate 100,000-state/100,000,000-configuration run exhausted a fixed 2 GiB heap in SysML NFA closure and wrote no artifact. The finite-graph experiment below avoids eager expansion but is not ready for adoption. Predicate visibility and imported caller FOLLOW traversal now have bounded native evidence. Visible guards now reuse native source-bound probes in focused controls. Repeated guard identities are resolved and a bounded converged-conflict resolver has frontend controls. Complete experimental definition, feature-prefix and assignment models now have focused evidence. Imported entry-decision dependency proofs close the two nullable-sequence generation blockers. Next qualify broader predicate/context conflicts and complete grammar families before production admission. Do not turn its rejected cases into guessed branches or remove production failure sites.
2. **Admit complete grammar families.** Use closed prediction dependencies to expand strict generated execution beyond the current 34 roots/135 rules. Verify full accepted and malformed rule inputs, model construction, context inheritance, token arbitration and hidden-token policy before production integration. The 555 exported decisions are import evidence only.
3. **Integrate complete native construction and semantic services.** Move admitted complete grammar families through the production pipeline. Apply the shared Ecore contracts across constructors, mutation and persistence; complete scope/link selection, required-value checks, inverses, operation/delegate algorithms and validation. Imported signatures and controlled-scope tests do not close these gates.
4. **Qualify category milestones.** Compare complete native models against pinned Pilot and the written specification, including malformed inputs and unresolved semantic dependencies. Keep handwritten Rust boundaries and disagreements explicit. Retain the bounded Xtend subset; expand it only after a useful-rule evaluation shows a benefit.
5. **Final release qualification.** Process every in-scope sample through the required pipeline, perform detailed semantic comparison and paired timing, then assess the completion checklist. The candidate stays unpromoted and full support stays open while any required behavior is unimplemented.

## Current implementation boundary

**Finite prediction graph milestone (experimental, not production admission):** The pinned frontend now exports 718 contextual decisions: the prior 573 plus 145 repeated-entry sites. The finite graph remains 2,373 base states, 7,565 KerML states and 14,077 SysML states under a fixed 512 MiB Java heap. All predicate identities bind, including both repeated ActionBodyItem guards, using whole-rule equality, occurrence counts and unique source positions; changed structure or positions reject binding. Explicit handwritten Rust supplies traversal, predicate visibility, precise call returns and imported global FOLLOW handling. Existing native zero-width probes consume the resolved guards. A bounded ordered-conflict resolver handles alternatives with an identical continuation/return context and guards on every earlier alternative; other conflicts remain unsupported. Two independent pinned-frontend controls verify this priority boundary.

The opt-in test path now connects pinned Xtext/ANTLR artifacts → resolved graph and guard identities → generator-checked programs → native rule execution → typed containment construction. Graph decisions bind **before** ambiguity analysis, with source signatures, cardinalities, alternative counts and provenance checked. Mandatory nullable calls carry explicit internal entry-decision dependencies; removing an imported dependency restores the enclosing sequence's mandatory failure. This closes both experimental sequence-generation blockers without test-side deletion of failure markers. The separate `xtext-finite-programs.experimental.json` emits 273 KerML and 540 SysML rules with zero unresolved prediction-site markers; these counts are generation evidence, not semantic qualification.

All 45 accepted-input branch controls and seven rejected-input CST branch observations remain focused evidence. **Eleven complete model controls** (seven part definitions, two KerML functions with feature prefixes, and two assignment actions with simple/qualified targets) match Pilot's ordered containment classes and declared names; **five malformed complete inputs** reject in both implementations. Linking, derived values, all scalar/reference values, diagnostics/recovery equivalence and complete rule-family support remain unqualified. Canonical strict failure markers and the 34 roots/135 rules admission boundary are unchanged; the graph is test-only and the candidate remains unpromoted. Reproduce the experimental programs with `python -B tools/generate_xtext_partial_programs.py --finite-graph --check` and the upstream graph with `python -B tools/evaluate_pilot_nfa.py --java-bin <jdk-bin> --check`.

**Finite-program milestone verification:** 615 native library tests passed. A subsequent strengthened focused control passed and explicitly refuses to count unsupported/budget/runtime errors as malformed-input rejection. All 46 generator tests and six finite-export tests pass; canonical and finite-program freshness, upstream finite-graph reproduction, structural coverage freshness and production `cargo check` pass. No full sample comparison or benchmark was run. A concurrent test relink briefly hit a Windows executable lock; retrying after the full suite completed succeeded.

**Resolved predicate milestone (current):** Xtext `FirstSetComputer` exports all five predicates per independently resolved grammar context. Source signatures, ANTLR decisions and native probes consume those sets without per-rule token lists. Native prediction preserves the distinction between first-token tests and complete syntactic predicates; changed/missing imports and wrong terminal targets reject generation. Thirty-seven independent Pilot branch controls now include TerminateNode and containing CalculationDefinition forms. Xtext GrammarAccessExtensions supplies the hoisted guard identity; native generation rejects missing or changed mappings rather than reconstructing a rule-specific guard. Focused checks pass: 41 native executor tests, 26 construction tests, 44 generator tests, 12 grammar-contract tests (three optional integration tests skipped), and 12 coverage tests. Actual grammar export and Pilot probe reproducibility were checked separately. The strict executor still admits 34 roots/135 rules; complete body closure remains open.

**Milestone verification:** All 608 native library tests passed (zero failures or skips) after the predicate and represented-name integrations. All source-contract stages are current: raw grammar reproducibility passed separately; the remaining chain passed after refreshing a stale ownership-table grammar fingerprint and resuming its final 17 checks. The ownership implementation was byte-for-byte unchanged apart from that fingerprint. This includes 37 independent Pilot branch controls, 26 represented-name controls, existing construction controls, and the bounded Xtend trial. Focused Python checks passed 44 generator tests, 12 grammar-contract tests (three optional integration tests skipped), and 12 structural coverage tests. No full sample comparison or timing qualification was performed at this milestone.

**Represented-name boundary:** Pinned KerML 1.1 Beta 2 section 8.2.2.3 (PDF page 102; hash in `../2026-08/specifications.json`) requires quote removal and escape decoding. Generated Name values now reuse the explicit handwritten escape decoder; raw lexical reference spelling stays unchanged. Twenty-six parsed Pilot controls compare native represented names to `ElementUtil.unescapeString`, covering both contexts and every escape. Raw Pilot declaredName stores lexical quotes; that intermediate representation is not the normative represented name. Escaped declared names and short names also reach production compiled SysML and KerML models in focused tests. Complete qualified-name lookup, authoring normalization and semantic name validity remain separate obligations.

**Latest category batch:** Configurable deterministic ANTLR analysis bounds resolve six additional decisions: ConnectorDeclaration, FunctionBodyPart, IfNode, PackageMember, PayloadParameter and PerformedActionUsage. Twenty-eight independent generated-Pilot-parser branch controls cover these six families plus SendNode, whose visible-predicate non-LL(*) retry follows the pinned frontend algorithm under the same deterministic bounds. Timeout fallback remains disabled; retry options are restored before analyzing another decision. JSON object keys are canonicalized without reordering predicate arrays. Complete-root admission remains 34 roots/135 rules; this is bounded branch evidence, not full model or specification qualification. Remaining analysis gaps stay explicit and the candidate is unpromoted.

Strict contextual admission now includes `OwnedFeatureTyping`, `OwnedSubsetting`, `OwnedRedefinition`, `OwnedReferenceSubsetting` and `OwnedCrossSubsetting` in both languages. The complete rule closures use imported definitions and unchanged strict prediction checks. Ninety independent Pilot controls verify 50 accepted models (qualified names, quoted names and feature chains) and 40 rejected inputs. Native materialization compares model shape, reference spelling after controlled binding, imported defaults and reciprocal containment. The enclosing `FeatureSpecializationPart` closures are also admitted. Twenty-eight additional Pilot controls invoke real containing Feature/ReferenceUsage rules and compare 17 accepted native clause models and 11 rejections, including comma lists, language-specific keyword forms, multiplicity containment, order and uniqueness. A named handwritten `NonuniqueValueConverter` binding maps the grammar keyword to false; generation rejects changed signatures and the oracle pins the upstream converter source. `FeatureDeclaration` is now admitted in both contexts. Forty containing-rule Pilot controls compare 21 accepted complete declaration models and 19 rejections, including names/short names, sufficiency, specialization lists, multiplicity flags, and supported relationship tails while preserving language-specific restrictions. KerML `TypeDeclaration` and `ClassifierDeclaration` add 32 containing-rule Pilot controls (22 accepted, ten rejected), including specialization requirements, anonymous classifiers, conjugation, multiplicity and type relationship tails. Shared fragment entry now initializes and validates the caller classifier even for entirely optional bodies; subtype preservation and wrong-caller rejection have focused controls. SysML `DefinitionDeclaration` and `OccurrenceDefinitionPrefix` add 22 Pilot controls through PartDefinition (15 accepted, seven rejected), including abstract/variation/individual, metadata repetitions and the contained empty multiplicity for individual definitions. Generated construction now consumes five effective primitive defaults without explicit literals, preserving existing assignments. Enum defaults remain excluded because constructor/delegate overrides require separate qualification; library observation promotion and the candidate are unchanged. These are focused executor roots; production grammar integration, actual scope selection and required semantic transformations remain open.

| Category | Imported | Native consumption and evidence | Still required |
|---|---|---|---|
| Language contexts | Xtext-resolved effective rules and calls, separately exported for base/KerML/SysML | Production entry points select context-specialized programs. Controlled inherited calls and Override identities construct the correct typed child with inverse ownership; wrong-language syntax, signature drift and recursion fail. | Qualify actual overridden expression bodies; hidden-token policies. |
| Xtext rule execution | All pinned definitions; 34 roots/135 unique rules admitted to the bounded executor | Identification, all literal alternatives/NullExpression and multiplicity containment/reference trees reach production through explicit adapters. Model evaluation uses heap frames with cycle/progress checks and explicit frame/work budgets; syntax probes retain independent bounds. | The other 592 rules; complete expression precedence/predicates, lexical arbitration, converters and model construction. |
| Actions and semantic adapters | All 53 actions; 26 exact operand captures; 328 setting-delegate records | All 26 individual captures have native controls; five eager binary families use them in production. Handwritten operand append constructs Ecore wrappers and matches pinned Pilot controls for seven concrete classes. | Enclosing grammar support, control-function bodies, normative argument derivation and the other delegates. |
| Ecore model service | All 415 effective features, types, flags, bounds, defaults and opposite identities | JSON interchange/persistence checks, bounded inverse construction and defaults consume contracts. Unpromoted library audit matches 658,561 observed getter values. | Every construction/mutation/persistence path; required-value enforcement, operation/delegate algorithms and complete validation. |
| Xtend and qualification | Bounded trial and independent Pilot evidence retained | Existing translated predicate controls remain bounded; handwritten dependencies stay explicit. | Remaining semantic services, complete samples, detailed semantic comparison and paired timings. |

Decision discovery covers every parser Alternatives node and every optional group/call/assignment/cross-reference/keyword node in each resolved grammar context. All **573 contextual sites** are accounted for: **555 exported decisions and 18 explicitly unsupported**. Of these, 276 successful exports are optional-entry gates. Whole-rule structural equality, occurrence counts and source positions now distinguish 31 formerly ambiguous repeated decision identities; altered enclosing structure or occurrence binding fails closed. This is import coverage, not implemented-language coverage. Upstream analysis is limited to 10,000 states and 10,000,000 configuration additions; overflow rejects the whole decision. Timer-triggered fallback is disabled. FunctionBodyPart now completes deterministic analysis at the larger bound; no timer fallback is used. Single-body repeated entry gates remain outside this discovery increment; repeated Alternatives retain prior support.

The new end-to-end path is pinned KerML `MetadataFeatureDeclaration` → resolved Xtext representation → signature-checked optional-entry automaton → native fragment execution → Ecore-checked FeatureTyping containment and typed deferred reference. **17 independent Pilot controls** compare acceptance, name fields, child classes and unresolved type spelling, with syntax-only probe agreement. **One representation difference remains explicit:** raw Pilot stores lexical quotes in declaredName; Mercurio represents names without quotes and decodes escapes as required by the normative KerML clause. The 26 normalized Pilot name controls above verify this bounded conversion; complete qualified-name lookup remains open. This fragment is executor-only: complete metadata bodies, production integration and reference resolution are separate requirements. Across language programs, admitted execution is **34 roots/135 unique rules**; the neutral subset remains 14/39.

The complete pinned SysML `EmptySuccessionMember` now also runs through generated native rules and Ecore-checked construction. **11 independent Pilot controls** match acceptance, root class, ordered containment classes and multiplicity integers; native checks verify inverse fields and unchanged caller state. Its dependency closure contains 26 rules, adding eight previously unadmitted rules across language programs. This is executor-only support for the succession member, not complete action-body parsing or endpoint resolution.

**40 fragment-generator and 38 native fragment tests pass.** Existing 48 reference-member model cases and 66 expression decision/token cases remain covered. An additional 72 independent Pilot endpoint decisions across Specialization, Conjugation, Disjoining, FeatureInverting, Subsetting and Redefinition match native decision execution. These six complete relationship rules still fail generation at the recursive body dependency; they are not added to admitted rule/model coverage. Build-time artifacts reproduce; production execution stays native and the candidate remains unpromoted.

Resolved syntactic-predicate ASTs now preserve guarded syntax and occurrence cardinality in both source and native signatures; changed enclosing predicates invalidate contextual binding. The 19 explicit decision gaps comprise 18 deterministic analysis limits and one missing structural match. All five first-set predicates now have context-specific sets computed by Xtext itself. EnumerationBody and TargetParameter decisions now export syntax gates and ordered predicate transitions. The shared native evaluator has controlled evidence for origin rewind, zero-width probes, fallback order, missing-dependency rejection and recursion/cycle bounds. Neither complete rule closure is newly qualified: their recursive model dependencies remain blocked. Arbitrary Java conditions and unsupported Boolean predicate combinations are rejected. Importing predicate syntax does not qualify its runtime decision dependency.

Experimental expression execution now uses an isolated `xtext-partial-programs.json` artifact generated from the same pinned resolved definitions. It emits 273 KerML and 540 SysML dependency rules, with respectively 4 and 15 unresolved prediction sites preserved as mandatory native errors. These are **emitted dependency counts, not supported-rule counts**; admitted complete-root coverage remains 34/135. Strict generation still rejects unresolved predictions, and structural/type/provenance/progress checks are not relaxed. Native controls execute six actual OwnedExpression paths in each context (integer, Boolean, typed reference, parentheses, addition and multiplication precedence), checking values, reference spelling and staged operand structure. This is focused native evidence, not independent Pilot semantic qualification. Operand wrapper materialization is now exercised for bounded eager expressions as described below; complete scope resolution, function binding/argument derivation, complete expressions and production integration remain open. Optional/speculative/syntax-probe paths must propagate unsupported-site errors. The artifact is test-only and never enables production fallback.

The bounded construction path is pinned `OwnedExpression` → contextual partial program → native capture/containment tree → the existing handwritten operand-append delegate → Ecore-checked native graph. **92 independent raw Pilot controls** cover 84 accepted trees and eight rejected inputs across both languages. Arithmetic, logical and conditional wrappers now come from the grammar; the materializer has no operator allowlist. Native construction consumes imported defaults and reciprocal containment. Forty reference cases compare raw shape/spelling after explicit binding to a controlled native scope, with both Xtext target narrowing and Ecore endpoint types checked. Additional native rejection controls cover wrong target types, unresolved/ambiguous resolution, empty identities and collisions with constructed identities. Input trees and external endpoints remain unchanged. **Pilot scope resolution is not compared**: scope selection is an explicit caller dependency, and complete scoping/linking qualification remains open. Control-expression raw construction is covered here; function binding, derived arguments and runtime evaluation are not. No new complete root is admitted and the partial artifact remains test-only. Every marked unsupported prediction site (4 KerML, 15 SysML) is exercised as an error in model and syntax-probe execution. Evidence: `expression-model-pilot-controls.json`; reproducible with `tools/run_expression_model_probe.py --java-bin <jdk>/bin --check`.

Construction now records and replays a shared source-order journal across ordinary containment and operand captures, fixing classification/cast wrapper ordering without rule-specific reordering. Additional controls cover classification, casts, extent, feature chains, indexing and sequences. Stored graph construction uses Ecore class/feature checks rather than a class allowlist. **Named external dependency:** FeatureChainExpression and IndexExpression Java constructors initialize operator values not supplied by Ecore defaults; a handwritten native initializer implements these two cases and the evidence pins their source hashes. Other specialized operator constructors remain explicit errors. This does not implement derived type/target algorithms. Missing, duplicate and invalid construction events are rejected. Complete expression closure and production integration remain open.

Invocation, positional/named argument, constructor, metadata-access and chained-invocation construction now have focused raw Pilot controls. Reference evidence enumerates Ecore noncontainment reference features, including redefinition and feature-chaining slots, rather than special-casing member/type fields. The shared executor now creates the caller's current classifier at resolved model-fragment calls, including assignment-free argument lists; failed optional attempts do not leak objects and existing subtypes are preserved. This closes the empty constructor-result construction gap. **Named-argument scope resolution and derived argument binding remain unqualified**: the test resolver supplies explicit endpoints and does not implement Pilot's contextual scope service.

Construction and linking are now separate phases: all stored containment and operand-delegate wrappers exist before any scope callback. References are typed/validated and linked into a local graph, which is returned only after every link succeeds. The endpoint API now distinguishes external objects from IDs in the completed local graph. Both paths receive the same Xtext/Ecore checks; forward and cyclic noncontainment references are supported without weakening external identity-collision rejection. Focused tests parse `x+y` in both contexts and verify internal endpoints, missing/wrong-type targets and late-failure rollback. Scope selection in those tests remains controlled, not independently Pilot-qualified. The resolver receives that complete graph, owner/feature identity and its checked parent namespace. A shared handwritten parent-namespace service uses imported Ecore Namespace ancestry and reciprocal ownership contracts; the 92-case raw Pilot corpus now compares parent namespace classes using pinned `NamespaceUtil.getParentNamespaceOf`. Invalid containment and callbacks occurring before construction completes have focused controls. Actual `T(x=U(x=x))` parses in both contexts; its controlled resolver uses completed graph containment and earlier callee bindings to distinguish `T.x`, `U.x` and lexical `x`. The callback also receives a handwritten non-expression namespace selector using imported classifier ancestry and the same checked containment traversal. Independent pinned Pilot controls now compare exact non-expression namespace identities for every object in all 84 accepted expression trees, attached through FeatureValue/Feature/OwningMembership to a Package. Native controls separately cover detached roots. This verifies bounded namespace selection, not complete scope lookup. **Still unqualified:** relative namespace rules, parameter visibility/inheritance, argument/result derivations and integration with the complete native lookup service. Parent traversal and controlled parameter binding are not full scope support.

Model-rule execution now uses explicit heap-backed frames rather than recursive model calls. It preserves shared assignment completions, construction order, committed choices, optional rollback, cycle checks and unsupported-site errors. Budgets are **256 active model-rule frames and 100,000 evaluation steps**, with cleanup on failure; syntax-only/lexical probes remain separately bounded and are not claimed stackless. Controls verify 80 nested model calls, rejection at 300, step exhaustion and unchanged caller state. This is not unbounded recursion support, nor admission of the partial rule graph into production. Existing qualified adapters use the new shared executor; the partial expression graph remains test-only.

Owned-result lookup now has a named handwritten native service consuming imported Ecore ancestry, direction domains and reciprocal containment. It preserves membership order, selects the first owned member, applies Behavior/Step parameter eligibility and follows the final resolved feature-chain target. Sixty independent Pilot controls cover five owner classes, three membership classes and four direction states; native controls additionally cover ordered results, unresolved chains and broken ownership. The inherited-result traversal now composes that query with mandatory member-readiness and ordered-general-type services. Thirty-two independent Pilot controls verify first-result selection across ordered diamonds and legal cycles; explicit native failures cover absent prerequisites, missing/wrong-type endpoints and duplicate graph identities. The traversal uses a heap stack and visits each type once. Native general-type collection now reads owned specialization relationships, filters by their resolved specific endpoint, appends required implicit-general inputs, and then the final feature-chain target. General/specific slot dispatch is generated from the pinned Ecore `redefines` annotations, including transitive redefinition chains; ambiguous branches fail explicitly and derived aliases still require delegates. The 32 inherited-result controls now traverse these native relationships. The generated materializer now applies the same annotation dispatch to reference writes, checks the effective target type and rejects duplicate aliases before callbacks. Ten independent Pilot reflective-setter controls verify both general/specific reads and concrete storage slots across five specialization classes. This does not admit additional grammar roots or implement derived-property setters. **Still required:** complete additional-member and implicit-general providers, unresolved endpoint binding, transformation and relative-scope integration. Supplying controlled prerequisites does not qualify those services or full `Expression.result`.

**Next batch, in order:** implement relative namespace result dependencies (transformation/additional members, inherited result parameters, instantiated-type inputs, redefinition-based argument ordering and value expressions) and integrate actual scope lookup and remaining expression classes beyond the controlled linking/construction bridge; qualify the actual dependencies of the newly bound predicate automata, keeping engine controls separate from complete-rule evidence; resolve the actual definition/predicate dependencies behind KerML `FunctionBodyPart`/`FeatureElement` and SysML `ActionBodyItem` (deterministic analysis limits); verify complete expression closures; migrate qualified families into production consumers. Never silently accept disabled alternatives. The earlier optional metadata-entry gap is closed for the focused fragment. Keep scalar recognizers outside parser-DFA binding. Hidden-token arbitration, malformed-token recovery, broad lexical equivalence, type-changing fragment cycles, scalar-rule predicates and parameter guards remain open. Deferred references, control-function wrapping and normative derivation remain separate semantic gates.

[Detailed milestone evidence and prior integration results](remaining-gap-history.md#2026-09-30-contextual-dispatch-milestone-record) are historical. This ordered plan and the coverage table below remain authoritative. Full corpus comparisons and benchmarks stay deferred until a substantial execution milestone; the candidate remains unpromoted.



## Archived milestone detail before typed import consolidation (2026-10-01)

## Current milestone

**Union implementation and independent evidence:** generated Namespace.membership input contracts now drive a public, read-only native evaluator, including Type.inheritedMembership. All eight actual pinned Pilot getter observations match exact native order and duplicate removal. Inter-subset ordering is explicitly handwritten and guarded against unknown imported contracts; it is not inferred from the union annotation. Canonical reciprocal ownership now derives ownedMembership using imported subset/type contracts and proves empty importedMembership when no Import exists. Closed canonical package imports now derive through the shared scope service with first-occurrence declaration order, membership identity, visibility/import-all and whole-member conflict hiding. Other inputs must be explicit. Evaluation requires resolved, typed targets, rejects duplicate identities/malformed inputs, ignores stale union snapshots and leaves storage unchanged. Filtered/external imports and inherited/type scopes remain dependencies; stale snapshots and incomplete canonical import targets reject. Four additional actual Pilot canonical package graphs now verify the public JSON-import-to-union path: ordinary imports, import-all private membership, duplicate imports and public re-export. Exact ordered owned/imported/union identities match, snapshots reconcile, and input graphs remain unchanged. These are independent behavioral controls for the bounded bridge, not complete scope qualification. Persistence separately checks membership consistency when inputs are complete. Reproduce the independent controls with `python -B tools/export_pilot_union.py --java-bin D:/dev/jdks/jdk21/bin --check`. The normative reference remains [normative-union.json](normative-union.json); no whole-family qualification is claimed. This batch passes 39 native tests (26 JSON/persistence and 13 Ecore) and 12 coverage-plan checks.

**Subset implementation:** 190 pinned annotations resolve to 224 direct edges. Generated transitive contracts now drive shared native explicit-model checks and public JSON persistence: contradictory present subset/superset values reject without mutation, including inherited contracts and absent intermediate snapshots. Missing values remain unknown. Derivation, union evaluation, update maintenance and independent Pilot/normative qualification remain open; this does not close the annotation family. Verification passes 35 native tests (24 JSON/persistence and 11 Ecore), seven generator tests and 12 coverage-plan checks. The JSON regression exposed and corrected a fixture that omitted its source from relatedElement; duplicate-reference preservation remains tested.

**Annotation assessment:** the [annotation inventory](ecore-annotation-coverage.json) now classifies all 1,674 pinned annotations and resolves subset/redefinition references. The coverage row is corrected from unassessed to partial because native redefinition dispatch already exists; this is an assessment correction, not newly implemented behavior. Remaining annotation work is explicit: 190 subsets, one union, 280 opposite-role labels, delegate algorithms and complete redefinition semantics. Documentation formulas remain non-executable text. The frozen measurement baseline is preserved; any increase in families with recorded evidence must be attributed to this audit.

**Current end-to-end path:** pinned grammar programs preserve qualified names and chain groups into the AST. Native library lookup consumes Ecore membership/redefinition contracts, reciprocal ownership, effective-name dispatch and visibility defaults. Namespace and feature indexes share explicit inheritance composition. Canonical package imports now reuse the candidate linker's graph scope algorithm in production: namespace and membership imports, public re-exports, import-all and whole-membership conflict hiding feed ordinary native type lookup. Canonical import endpoint types are validated. Unsupported filters, unresolved endpoints and inherited target scopes remain explicit errors. Materialized libraries and root anchoring retain compatibility adapters.

**Bounded evidence:** the previous accumulated milestone passed 667 native library tests and 12 coverage-integrity checks (target/library-explicit-inheritance-regression-final.log). This import bridge adds a public compilation regression for stable identities, long/short names, visibility, import-all, re-exports, alternate-name collision hiding, invalid/unresolved endpoints and record-order independence. This batch passes 49 focused native tests (nine index, five external-reference and 35 construction/linking tests) and 12 coverage-integrity checks; the previous full-suite count is not a claim that the new tree received release-wide qualification. The reused service retains its existing 34 Pilot linking models/75 targets and documented normative disagreements; the production bridge adds native integration evidence, not new independent Pilot observations.

**Remaining critical dependencies:** global resources, complete imports (filters, inherited targets, open/external graphs), computed generalizations and name delegates, materialized-only snapshot reconciliation, incomplete/external ownership, complete qualified membership access, protected inheritance, type-aware lookup/redefinition filtering and remaining declaration-name consumers. The native owned-member bridge requires exactly one child while Pilot selects the first. Strict grammar admission remains 38 roots/141 rules. Next: close scope dependencies using shared services, then qualify complete grammar families. The candidate remains unpromoted; full sample/Pilot comparisons and timing remain pending.

Reproduce focused chain-link observations with `python -B tools/run_chain_link_probe.py --java-bin <jdk-bin> --check`.

The shared **QualifiedName production adapter** now serves alias targets, SysML metadata type/about references and both languages' metadata prefixes. It executes the admitted contextual rule and preserves represented segments, source spans and enclosing-rule boundaries. Admission remains 38 roots/141 unique rules. Complete AliasMember/Import strict admission is blocked by KerML FeatureElement, SysML UsageElement and SysML ActionBodyItem prediction dependencies. **Next shared migration:** map callers of legacy `parse_qualified_name` to their actual upstream rules before moving them: that helper conflates QualifiedName with dotted FeatureChain and expression syntax. A wholesale substitution produced regressions and was withdrawn; no permissive fallback was added to the generated adapter. Broad regression testing also exposed a one-megabyte stack overflow. Final usage-AST assembly and Documentation assembly now execute outside the large recursive parser frames, and optional reference spans are heap-backed. Both existing nested-body and new nested-generated-alias stack controls pass without raising the stack limit. Nine qualified-reference tests and all 651 native library tests pass after the frame refactor, including both one-megabyte stack controls. Twelve coverage-integrity checks also pass. This is native regression evidence, not full sample/Pilot/timing qualification.

Complete **Comment, Documentation and TextualRepresentation** rules now drive both public language parsers, replacing duplicated header recognition. Comment adds two admitted roots and four unique rules (38 roots/141 rules total). Twelve independent Pilot Comment objects verify raw fields, ordered Annotation children, reference types and unresolved spellings; the expanded oracle has 30 objects across the annotation and literal forms. Generated cross-references now retain exact source spans. A shared handwritten AST adapter projects ordered targets to existing native ownership/linking services; no general scope qualification follows from grammar admission. Focused public tests verify escaped/qualified names, repeated targets, inclusive source spans, malformed headers, language-specific keywords, bare-comment authoring identity and adjacent declarations. Existing namespace tests cover native compilation, target resolution, ownership and authoring round trips. Candidate remains unpromoted; full sample comparison and timing remain pending. Batch verification: 67 Xtext tests, 35 construction/linking tests, 25 namespace regressions, one focused target-span/context test, 46 generator tests and 12 coverage-integrity tests pass. The 30-object oracle reproduces via `tools/run_comment_value_probe.py --java-bin <jdk-bin> --check`.

The candidate's named handwritten **package-scope service over generated Ecore graphs** now resolves namespace and membership imports, recursive imports, import-all visibility, public re-exports, aliases and lexical/qualified package members. Imported Xtext reference types distinguish a Membership endpoint from its member Element; names, flags, containment and visibility come from the generated model. Thirty-four models loaded through Pilot's actual Xtext resources independently verify 75 reference targets, including duplicate imports, diamond re-exports and short-only declarations/aliases. Observations compare target kind, namespace path and a declared long-or-short display label; stored names are not conflated with that label. This is a bounded scope milestone, not general scoping qualification.

The native linker now follows an explicit pending-field dependency stack. This lets multiple unresolved imports use outer namespaces without manufacturing retry cycles. Active imports are excluded while resolving their own targets, following the pinned lazy-linking algorithm. Genuine dependency cycles and missing dependencies still fail transactionally. No runtime Java is introduced.

Normative basis: the clean `2026-08` release's `doc/1-Kernel_Modeling_Language.pdf`, sections 7.2.5.2ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Â¦Ãƒâ€šÃ‚Â¡ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¦ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã¢â‚¬Å“7.2.5.4 (printed pages 23ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Â¦Ãƒâ€šÃ‚Â¡ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¦ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã¢â‚¬Å“25), distinguishes imported memberships, recursive ownership traversal, visibility and conflict hiding. The implementation also pin-checks Pilot scope-provider and namespace/name utility sources. **Declared-name package conflict hiding is now implemented**: derive complete imported membership sets, deduplicate by membership identity, hide whole memberships on long/short-name collisions with owned or other imported memberships, and apply imported metaclass compatibility. Hidden imports allow lookup to continue in the outer lexical scope. Filtered imports, inherited/type scopes, computed Feature names, global resources and public production integration remain open. The candidate stays unpromoted.

**Recorded normative disagreement:** recursive `import all` includes private owned sub-namespaces under KerML 8.3.2.4.2 and 8.3.2.4.6 (printed pages 125 and 132). Pilot's `NamespaceAdapter` / `NamespaceImport.importedMemberships` derives the target correctly, but `KerMLScope.resolveRecursive` only traverses public namespaces and its actual Xtext reference remains unresolved. Native lookup follows the written rule. `scope_disagreement_controls` records both independent Pilot observations; a dedicated native regression checks the derived target path. This case is separate from the 34 matching linking controls.

**Additional recorded disagreements:** three controls show Pilot selecting an imported member that must be hidden under KerML 7.2.5.4 / Namespace.importedMemberships, including a collision through a different short name. A fourth shows its `Membership_isDistinguishableFrom` delegate reporting a collision between unrelated metaclasses; the pinned delegate explicitly leaves that check unimplemented. KerML 8.3.2.4.3 (printed page 127) requires the check. Native uses imported Ecore ancestry and narrows candidates by the imported Xtext reference type. `membership_conflict_controls` retains Pilot's actual target, delegate result and independently resolved EMF metaclass compatibility. These are normative exceptions, not matching Pilot observations.

**Resolved-name milestone:** Pilot's own invocation selector now exports 350 effective-name/short-name bindings across all 175 resolved EClasses. Generated Rust dispatch invokes explicit handwritten algorithms for declared names, explicit redefinition chains and conjugated-port derivation through checked PortConjugation references. Twenty-seven independent name observations match. Twelve new graph controls verify explicit redefinition chains, first-target ordering and declared-name overrides across Feature, PartUsage and AttributeUsage. A contained Redefinition contributes only when its imported specific endpoint identifies the owning Feature. Unnamed Features without a resolved explicit chain still require computed redefinitions. Missing targets, invalid target types and cyclic derivations reject. Ordinary anonymous definitions no longer block unrelated lookup, verified through two additional real Pilot linking models. The source-only LifeClass delegate is not selected by this pinned runtime model. Imported selection, algorithm consumption and these bounded observations remain separate accomplishments.

The [normative name ledger](normative-names.json) records clean pinned specification hashes, KerML printed pages 112/163 and SysML section 8.3.12.2 (printed page 293). These confirm the implemented declared/conjugated rules independently of Pilot.

**Hidden-token consumer milestone:** generated inherited hidden-terminal identities and terminal bodies now drive native lexing through a language-neutral Foundation callback. Native execution of 758 upstream ANTLR lexer decision states replaces handwritten comment precedence, closing the discovered ML_NOTE/SL_NOTE overlap false acceptance. The exporter exhaustively evaluates special transitions over EOF and every UTF-16 code unit; unused upstream array suffixes are preserved but cannot be targeted. There are 397 independent upstream prediction controls and 16 Pilot parse/hidden-span controls across KerML and SysML. Non-Xtext Unicode whitespace rejects; semantic regular comments remain tokens. These controls do not qualify all visible-token construction or diagnostics.

**Numeric consumer milestone:** imported RealValue syntax now determines native Number carrier spans, and exported lexer decisions select decimal/exponent components in source scanning and parser lookahead. Resolved upstream keyword identities replace longest-prefix keyword guessing. All fifteen independent Pilot numeric controls match native source acceptance; twelve non-range controls also verify component spelling/identity and committed exponent failures. Focused source controls verify fractional/scientific forms, spans and range punctuation. Foundation only validates callback byte boundaries and preserves the existing token interface. Numeric carrier admission and public punctuation representation remain explicit handwritten adapters. This is bounded lexical evidence, not complete visible-token or grammar-family qualification.

**Committed terminal-body milestone:** all nine pinned terminal families now feed native source scanning through generated programs, including identifier and semantic-comment spans. Build-time FIRST/FOLLOW proofs establish disjoint choices, nonnullable decision bodies and greedy exits through rule-call contexts; changes requiring additional lookahead fail generation. Native optional/repeated groups commit after entry rather than backtracking to a shorter prefix. This removes the handwritten SL_NOTE carriage-return exception. All **1,323 actual Pilot terminal-method observations** match native consumed lengths or rejection. Source controls preserve raw values, trivia, Unicode/multiline spans and adjacent comment boundaries. Keyword/punctuation carrier adaptation and complete language-specific arbitration remain separate, open obligations.

**Language-specific keyword milestone:** native parsing now consumes both upstream lexer decisions: 758 SysML states and 646 KerML states, matching 2,002 independent prediction observations. Parser lookahead no longer chooses the longest keyword from its local rule context. Complete punctuation tokens cannot match shorter keywords; trivia prevents token joining. Shared word carriers are interpreted by language, so KerML `part`/`def` names reach generated Identification assignments while remaining SysML keywords. Normalized `specializes` and `:>` retain distinct keyword identities through their source spans. This qualifies bounded generated-parser behavior; it does not promote every legacy public parser path.

**Default-aware scope milestone:** package-scope reads now use shared generated Ecore defaults for omitted visibility, recursion and import-all attributes. Explicit values are validated and retained; reads do not materialize properties or change unset state. All 34 independent Pilot linking models/75 target observations still match, with every native scope callback also checked against a graph omitting default-valued scope properties. Forty-nine known constructor/delegate dependencies remain explicit blockers rather than inferred defaults, including expose import-all overrides and contextual Usage variability. Absent enum policies and derived/volatile values also require qualified services.

**Resolved converter milestone:** the actual injected Xtext services export 175 context/rule/datatype bindings. Native scalar execution now requires those generated bindings before invoking explicit handwritten algorithms, replacing type-only selection. Of 667 independent converter observations, 665 match and two non-finite Double outcomes remain unsupported. Raw string conversion preserves quoted spelling; semantic unescaping and represented-name decoding remain distinct operations. This is converter evidence, not complete grammar/model qualification. A further **36 LiteralReal parser-to-model controls** match native complete-consumption acceptance and stored real values across both languages. Pilot rejects separated numeric components during conversion (for example, `1 . 5`), while preserving surrounding trivia. Exported diagnostics retain this distinction; diagnostic equivalence and normative whitespace policy remain open.

**Required-feature assessment milestone:** imported Ecore lower bounds now drive a shared non-mutating assessment exposed by `SysmlJsonImportReport::required_feature_diagnostics`. Missing stored values, invalid shapes/bounds and incompatible local reference targets produce violations. Qualified defaults and canonical identity are read without changing storage. Derived/volatile features, unresolved default policies and external targets remain explicitly unverified, even when a derived value was serialized. Partial-model persistence remains independent; this is not complete validation or delegate implementation. Independent EMF validation now covers all 1,536 fresh-concrete-class required stored-feature cases: 1,477 outcomes match and 59 remain unverified. Four constructor-provided expression operators exposed false missing-value diagnoses; generated dependency guards correct those diagnoses without pretending the initializer algorithms are implemented. Explicit values still receive normal checks.

**Resolved constant-constructor milestone:** Java's compiler API now parses and resolves the four pinned specialized operator constructors, including inherited field symbols and String constant values. A bounded translator accepts only `super(); operator = OPERATOR_EDEFAULT = constant;` and emits native data; it does not interpret unrelated constructors or delegates. Shared generated values replace two handwritten cases and add CollectExpression/SelectExpression initialization. Native construction, default reads and required-feature assessment consume the table. Independent runtime getter observations match all four values; required-feature totals are now **1,481 matching and 55 unverified**. This consumes Pilot implementation semantics, not additional Ecore-defined behavior; complete expression and normative qualification remain open.

**Stored-containment mutation milestone:** `SysmlJsonImportReport::move_contained_element` now consumes a shared transactional Ecore service. Imported containment and opposite contracts govern movement within and between the two stored ownership families. Both endpoints update together, destination order is preserved, repeated same-owner moves are no-ops, and failed type/bounds/cycle checks leave the original graph unchanged. Incomplete/external old ownership and explicit derived/volatile properties reject pending resolution/recomputation services. Focused persistence regressions pass. Independent EMF snapshots now match six valid sequential moves through forward and inverse writes, including duplicate no-op and cross-family movement. The generated owner setter also rejects an ancestor cycle. General mutation integration and performance remain open.

**Recorded mutation-level discrepancy:** the pinned generated owner setters use `EcoreUtil.isAncestor` and reject recursive ownership. Direct mutation of their containment lists bypasses that check and commits a cycle. The oracle retains both operations and their resulting graphs. Mercurio's transactional move rejects either route and keeps the original graph. The direct-list operation is not counted as matching evidence; this records a mutation API difference, not a claim that a complete Pilot validation pipeline accepts the cyclic model.

**Next shared dependency batch:** qualify complete grammar families and their public parser/model bridge, including source-carrier and converter equivalence. Computed redefinitions, inherited/type scopes, automatic conjugated model synthesis and lazy original-port resolution remain required semantic dependencies. Keep candidate admission gated on complete families and public pipeline verification.

Pinned Xtext/ANTLR ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¾ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ resolved finite graph ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¾ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ generator-checked programs ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¾ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ native parsing ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¾ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ shared Ecore construction has **22 accepted complete-model controls across 14 entry rules**. These compare containment classes/order, upstream explicitly set stored attributes, controlled typed reference identities and reciprocal ownership. Five malformed controls reject; unsupported/runtime errors cannot count as syntax rejection. Unresolved controlled references fail without mutating the parsed input.

The controls include part definitions, functions, assignments, packages, actions, calculations, states, constraints, requirements, ports, items, connections, interfaces and enumerations. This is representative evidence, not complete family qualification. Generated IDs, unset/default-only attributes, derived values, actual scope selection and diagnostics/recovery equivalence remain outside this comparison.

All 18 finite syntax guards (two KerML, 16 SysML) now bind during generation; identity, predicate kind, FIRST/syntax distinction and signature changes fail generation. Explicitly guarded single-body occurrences use their imported zero-width gate instead of a redundant global entry decision. Conservative FIRST reachability prunes impossible paths while retaining nullable returns, missing-FOLLOW errors and fixed work/depth limits. A regression caught and fixed precise-return pruning. All 52 independent bound branch observations still match.

| Area | Imported/generated | Native evidence | Remaining qualification |
|---|---|---|---|
| Xtext | 718 parser decisions; 758 SysML/646 KerML lexer states; 273 KerML/540 SysML emitted rules | 2,002 lexer predictions; 1,323 terminal-body observations; 16 hidden-token controls; 52 parser branches; 22 model controls; five malformed controls | Full families, lexical policies, general context conflicts, production admission |
| Ecore | 415 effective feature declarations | Shared constructor checks explicit fields, typed endpoints and reciprocal containment | All model paths; defaults, required values and executable semantics |
| Scoping/linking | Typed references and explicit service interfaces | 34 real Pilot linking controls/75 matching reference targets; membership identity, declared-name conflict hiding, recursive imports and dependency-stack linking; five documented exception controls | Filter evaluation, inherited/type scopes, computed Feature names, global resources and production integration |
| Xtend/delegates/operations | Pinned signatures, bounded translation and 350 selected name bindings | Generated name dispatch; 27 independent name observations; explicit handwritten algorithms | Computed redefinitions and other executable dependencies; signatures alone are unsupported |
| Release | Pinned samples and assessment infrastructure | Focused controls for this milestone | Full sample pipeline, semantic comparison and paired timing |

Current containment qualification batch: **three focused mutation tests pass**. Independent upstream extraction reproduces all eight steps: six valid moves, one rejected inverse-setter cycle, and one separately recorded direct-list cycle discrepancy. Complete ownership-field snapshots are checked against native results for matching operations. No full corpus comparison or timing assessment was run; full release support remains open.

Reproduce ownership mutation observations with `python -B tools/export_pilot_containment_moves.py --java-bin <jdk-bin> --check`.

Reproduce constant assignments with `python -B tools/export_pilot_constructor_constants.py --java-bin <jdk-bin> --check` and generated native values with `python -B tools/generate_constructor_constants.py --check`.

Reproduce required-feature observations with `python -B tools/export_pilot_required_features.py --java-bin <jdk-bin> --check`, and constructor guards with `python -B tools/generate_required_feature_dependencies.py --check`.

Reproduce converter bindings with `python -B tools/export_pilot_value_converters.py --java-bin <jdk-bin> --check` and native dispatch with `python -B tools/generate_value_converter_dispatch.py --check`.

Reproduce candidate programs with `python -B tools/generate_xtext_partial_programs.py --finite-graph --check`; reproduce upstream evidence with `python -B tools/evaluate_pilot_nfa.py --java-bin <jdk-bin> --check`. Lexer prediction: `python -B tools/export_pilot_lexer_decision.py --java-bin <jdk-bin> --check` (or `--validate` for offline structure/tooling verification). Name dispatch: `python -B tools/export_pilot_name_delegates.py --java-bin <jdk-bin> --check` and `python -B tools/generate_name_dispatch.py --check`. Native filters: `xtext_` and `operand_construction::tests`.



## Materialized inheritance checkpoint before plain defaults (2026-10-01)

**Materialized inheritance milestone:** the imported `Element.isImpliedIncluded` flag now selects complete stored-graph inheritance. Native consumers use canonical ownership, Ecore subset/endpoint contracts and defaults to derive memberships, including stored implied generalizations. An iterative redefinition closure and both written `Type.removeRedefinedFeatures` predicates filter inherited Features. Complete unnamed Features with no redefinition now yield no names; named redefinition targets supply effective names. This consumes the graph's completeness assertion; computing missing implied relationships remains a separate dependency.

**Independent evidence:** eight union-composition controls and 27 canonical Pilot graphs match native results. Nine new complete-graph controls provide no inherited snapshots and verify full inheritance, direct/transitive/cyclic redefinition filtering, stored implied edges and anonymous/derived names. The exporter checks fixture completeness against actual Pilot computed redefinitions and implicit generals without replacing those services. Public JSON evaluation reconciles supplied snapshots and rejects incomplete or malformed inputs without mutation. Six public source-compilation scenarios verify surviving names, filtered-name rejection and record-order independence.

**Verification:** all **683 native library tests pass** (`target/materialized-inheritance-milestone.log`), including 33 JSON/model tests, 35 construction/linking tests and the six-scenario public compilation control. All 15 coverage-integrity tests pass and all 35 matching Pilot observations reproduce, with two disagreements recorded separately. Regression elapsed time is not a comparative benchmark. Reproduce the oracle with `python -B tools/export_pilot_union.py --java-bin D:/dev/jdks/jdk21/bin --check`. The frozen baseline and unpromoted candidate are preserved. Full sample qualification and comparative timing remain pending.


## Plain-Type default and canonical-library checkpoint (2026-10-01)

**Plain-Type implicit-default milestone:** actual pinned upstream default selection now generates native bindings for Type, Classifier, Class, DataType and Structure. The public native model evaluator consumes those bindings with explicit typed library-name-to-ID mappings, computes missing defaults, and derives inherited memberships through the existing shared services on a private graph copy. It preserves the original graph and completeness flags. This is implementation beyond importing a default table; The public model API now also resolves generated default names through canonical standard-library roots in the graph, using shared membership visibility and effective-name services. Ordinary compilation and external resource loading still need integration.

**Independent evidence:** 20 actual Pilot controls verify ordered general-type contributions and final memberships for the five metaclasses, including explicit/default overlap and self-default exclusion. Library lookup remains supplied in the independent Pilot controls. Three additional native integration tests exercise automatic in-graph bindings, public re-exports, owned effective names and rejection of private, ambiguous, missing or malformed dependencies; this does not independently qualify upstream global lookup. Native negative controls cover missing/wrong targets, incompatible metaclasses, incomplete ownership, metadata, unsupported subclasses, contradictory flags and private staging identity collisions. Existing materialized inheritance, Feature filtering and effective-name behavior remain covered by 35 matching Pilot controls and two separately documented disagreements.

**Verification:** the latest focused batch passes **38 JSON/model tests and 35 construction/linking tests**; the three library-binding tests also pass after tightening root ownership validation. The last full milestone passed **685 native library tests** (`target/plain-type-default-milestone.log`), including 35 public JSON/model tests and 35 construction/linking tests. All 15 coverage-integrity tests pass; the five generated bindings and 20 new Pilot controls reproduce exactly. This supersedes the preceding 683-test milestone. Regression elapsed time is not a comparative benchmark. Reproduce the new binding/oracle artifacts with `python -B tools/export_pilot_default_generals.py --java-bin D:/dev/jdks/jdk21/bin --check`. [Normative default requirements and boundaries](normative-plain-defaults.json) distinguish the written rules, imported defaults, native computation and supplied library dependency. The frozen baseline and unpromoted candidate are preserved; no full sample comparison or benchmark is claimed.


## Feature-chaining checkpoint before common document publication (2026-10-01)

**Feature-chaining inheritance milestone:** the shared native inheritance service now implements the written Feature.supertypes rule: Type contributions come first, followed by the final chaining Feature. Canonical ownership, imported Ecore ancestry, subset contracts and typed endpoints drive this named handwritten algorithm. Both recursive imports and complete stored-graph inheritance consume it; public source compilation uses the same service for library linking. Earlier plain-Type default computation and automatic canonical in-graph library binding remain available.

**Independent evidence:** nine actual pinned Pilot factory graphs verify final-target selection, explicit-parent order, recursive imports, cycles, duplicate parents, visibility, PartUsage dispatch and conjugation. No inherited snapshots or substituted upstream semantic getters are used. The exporter checks absence of unmaterialized implicit generals and redefinitions. Public native compilation verifies visible/hidden names and exact linked targets across all nine graphs; native negative controls cover missing, mistyped and malformed endpoints, contradictory ownership/snapshots, incomplete graphs and declaration order. These are operation/integration controls, not complete chain-declaration parsing, chain legality or navigation qualification.

**Verification:** focused model and public-compilation controls pass, all 15 coverage-integrity tests pass, and the nine new chaining observations plus existing 35 union observations and two documented disagreements reproduce. The full native regression suite passes **691/691 tests** (`target/feature-chaining-milestone.log`), superseding the previous 685-test full milestone. The 268.23-second regression duration is not a comparative benchmark. Reproduce focused upstream evidence with `python -B tools/export_pilot_union.py --java-bin D:/dev/jdks/jdk21/bin --check`. [Normative inheritance and chaining evidence](normative-inheritance.json) separates imported contracts, handwritten algorithms and remaining obligations. The frozen baseline and unpromoted candidate are preserved. Full sample qualification and comparative timing remain pending.



## Archived cumulative milestone reports, 2026-10-02

These reports are historical evidence and superseded next-step proposals, not an additional active completion plan. Current dependencies and measurements remain in remaining-gap-checklist.md.

## Current milestone

**Latest binary cross-featuring batch:** One atomic native stage now constructs TypeFeaturing from the other effective end's resolved types. **24 new independent Pilot controls match**, covering directly owned/inherited ends, zero/one/two types, both owner positions and existing featuring suppression; persistence and rollback pass. **157 focused native tests pass.** [Evidence](normative-binary-crossing.json). Metrics remain **155 whole-model matches (+0), 26/30 basic binding models, 3/4 bounded obligations, 0/34 families, 0/5 gates**. All cross stages still require resolved generalization inputs; real Links and nonbinary products remain unqualified.

**Next bounded integration contract:** (1) Split type-query dependencies from whole-generalization construction, retaining exact adapter/dependency guards. (2) Schedule cross specialization, featuring and crossing as transactional contributions during lazy linking, without asserting `is_implied_included`; prove each materialized contribution before ordinary providers consume it. (3) Run focused positive/negative source controls and the actual Base + ScalarValues + Links closure. Do not count constructed output as semantic qualification. Nonbinary Cartesian products remain a separate explicit branch; the completed binary stages do not close it.

**Latest owned-cross specialization batch:** One atomic native stage now adds FeatureTyping from resolved owning-end types and Subsetting from redefined ends' cross-features, consuming shared queries and imported Ecore contracts. **12 new independent Pilot stage controls match**, with idempotence, persistence and atomic rejection; **156 focused native tests pass**. Pilot fixture preconditions are checked explicitly after correcting missing relationship endpoints. This stage requires resolved owner generalizations and does not assert transformation completeness. [Evidence](normative-binary-crossing.json). Metrics: **155 matching whole models (+0), 26/30 basic binding models, 3/4 bounded obligations, 0/34 families, 0/5 gates**. Next critical dependencies: separate type-query dependencies from crossing construction, implement cross-feature featuring, then schedule the stages transactionally in lazy linking. Actual Links remains unqualified.

**Latest binary-crossing batch:** Native owned-cross-feature selection now matches **75 independent Pilot controls**. One transactional binary constructor reuses Ecore chain/specialization setters; **8 factory controls and 6 source-stage projections** verify target order, canonical ownership, suppression, persistence and atomic rejection. The stored `cross_feature` delegate consumes the constructed chain. **155 focused native tests pass.** [Evidence and required boundaries](normative-binary-crossing.json). Metrics remain **155 whole-model matches (+0), 26/30 basic binding models, 3/4 bounded obligations, 0/34 families, 0/5 gates**. Next: compute owned-cross-feature typing/subsetting and featuring, then integrate those contributions with lazy linking/general-type providers. Binary construction alone does not qualify the real Links closure; nonbinary and compound-chain contexts remain explicit dependencies.

**Prior shared ownership/scope batch:** Ordinary Feature default selection now consumes imported annotation/multiplicity classes and admits Classifier/DataType owners, verified by **96 independent Pilot source selector controls** with persistence. Inherited-first redefinition lookup no longer filters candidates using sibling redefinitions in the declaring type; two new controls verify exact chain and redefinition targets. The actual **Base + ScalarValues library closure constructs and links 144 native elements**. This is construction, not semantic qualification. Adding Links now reaches owned cross-feature contribution semantics instead of the earlier spurious redefinition cycle. [Evidence and boundaries](normative-feature-owned-members.json). Metrics remain **155 matching whole models (+0), 26/30 basic binding models, 3/4 bounded obligations, 0/34 families, 0/5 gates**. **152 focused native tests pass.** Next: implement the shared owned-cross-feature contribution and its required typed/featuring dependencies (owner types, inherited end redefinitions, cross-feature specialization), then qualify the actual Links closure.

**Prior shared parser/linker batch:** Imported `REGULAR_COMMENT` recognition now participates in prediction, verified against eight independent documented KerML/SysML roots. Ordered FeatureChaining references use Ecore ownership/subset contracts and one named handwritten scope service: eleven independent controls verify nine resolved target paths through persistence and two hidden/missing-target rejections. Real release Base and Links now pass parsing; inherited filtering now defers required explicit redefinitions to the transactional linker. Construction still blocks on Feature default-contribution and external/type-scope dependencies. The candidate audit rejects empty libraries and explicitly labels constructed models unqualified. [Evidence and remaining boundaries](normative-library-prediction-chain-scope.json). **151 definition-focused tests, 54 parser tests (overlapping), and two candidate-audit tests pass.** Metrics: **155 matching Pilot models (+0), 26/30 basic binding models, 3/4 bounded obligations, 0/34 families, 0/5 release gates**. Next: qualify the ordinary Feature provider against real documented library ownership and complete the remaining semantic-library closure; do not substitute reduced fixtures for that closure.

**Prior expression batch:** The imported connector default selector now supports fixed, untyped binary BindingConnectors under ordinary OwningMembership in all **32 concrete Expression classes**, verified by **32 independent Pilot controls (+32)**. Two source projections verify connector/end defaults, featuring, ordered endpoints, positional redefinition and persistence. Shared queries consume recognized materialized contributions without setting completion flags. Unknown implied contributions and unsupported ownership/variability remain errors. The inherited-end fixture verifies selection, not the real library's transitive participant specialization. [Evidence](compatibility-evidence.json). Metrics remain **155 matching Pilot models (+0)**, **26/30 basic ordinary binding models**, **3/4 bounded strategy obligations**, **0/34 whole families**, **0/5 release gates**. **149 focused native tests and 95 tooling tests passed.**

**Next expression batch contract:** (1) Qualify actual semantic-library closure for reference binding/result defaults, especially SelfLink ends and transitive participant specialization, then compose reference binding with result specialization. Controlled library projections do not discharge this dependency. The actual release Links.kerml (tag 2026-08, matching the pinned Pilot test library) adds sameThing.self crossing, inherited association ends, multiple redefinitions and typed multiplicities; its hash and dependency chain are recorded in compatibility-evidence.json. This source audit is not pipeline qualification. (2) Add ResultExpressionMembership and valuation binding stages, with self-reference fallback and initial/default-value contexts explicit. (3) Qualify one atomic expression lifecycle against a whole-source Pilot comparison. Multiplicity-bound featuring remains a separate context. The shared reference binding/default/featuring construction and effective-end queries are now verified within the bounded contexts above; no completion flags or whole-model credit follow from these stages alone.

**Common document/model pipeline:** the opt-in public `definition_document::parse_and_link` API now consumes complete pinned KerML/SysML RootNamespace programs, constructs canonical Ecore storage, resolves supported local references through the shared transactional linker, and publishes only after the common closed-model check succeeds. Syntax, lexical, unsupported-execution, resource-limit and artifact failures remain distinct. Production execution uses Rust and embedded artifacts; the candidate is unpromoted and the existing compiler is unchanged.

**Independent behavior:** 90 actual Pilot whole-document parses and lazy-linked models match 1403 canonical nodes, effective scalar/default attributes, ordered containment and 2595 effective reference slots, including automatic endpoints and property-redefinition views. Eight malformed documents reject. Canonical persistence/reload preserves the observations; all 34 prior independent package-link controls still pass through the public path. Controls now cover ordinary specialization, multiple generals, forward aliases, direct and qualified type members, owned-name shadowing, subsetting, conjugation, port conjugation, composition policy and variation ownership. These controls qualify their stated scope, not every reachable grammar rule. The finite candidate emits 275 KerML and 542 SysML contextual rules; strict admission remains 141/727. Emission is not verified coverage.

**Common structural boundary:** construction, JSON persistence and supported transactional edits share imported contracts and publication validation. Value-shape tests cover the 87 stored feature declarations. `changeable`, `unsettable` and `resolveProxies` are now preserved by native contracts; unsupported delegate, ownership, identity and stale-derived-value writes reject atomically. Partial interchange preserves opaque extensions; closed candidate publication requires pinned classes, resolved endpoints and both directions of present stored opposites. Missing inverse slots reject; derived opposites remain separate semantic obligations. Missing required values and executable derived semantics remain separate assessments.

**Qualification status:** 157 focused native tests pass for the current implementation; the current regression is recorded in normative-binary-crossing.json. Shared Usage defaults and the guarded ReferenceUsage fallback add four complete Pilot source comparisons, bringing the matching total to 148. The shared BindingConnector provider adds three further matching models (151 total); one invalid binary control is excluded and recorded separately. The bounded endpoint/binding lifecycle adds four complete document matches (155 total). Basic whole-model coverage is 26/30 ordinary bindings; the required redefinition-context matrix remains open. The bounded certificate is refreshed. The earlier 756-test full-library run is prior evidence, not a current full run; complete samples, release semantics and timing remain unqualified.

**Explicit boundary:** the legacy AST/profile emitter still contains singleton/list aliases and legacy metaclass projections; migrate whole families to the candidate constructor before applying closed canonical publication there. The new API computes bounded plain-Type general inputs and materializes selected implied relationships transactionally. Complete transformation, every scope origin/global resource, all delegates and complete language validation remain unqualified. The existing bounded inheritance/chaining and plain-Type default services remain available. No benchmark or complete sample qualification is claimed.

**Type/feature batch:** the shared constructor distinguishes raw grammar-fragment construction from complete-document postprocessing. Ordinary specialization/conjugation scope origins and direct owned type members use imported Ecore ancestry. `definition_document::reference_targets` reads canonical references and bounded membership/import views through imported property redefinitions. Usage defaults and automatic relationship endpoints are explicitly handwritten postprocessing dependencies, with upstream source fingerprints and [written-specification anchors](normative-document-types.json).

| Obligation | Imported/native implementation | Verification and remaining boundary |
|---|---|---|
| Declarations, direct type members and ordinary specializations | Pinned grammar programs; shared Ecore construction and local scope dispatch | Independent positive/negative documents and model round trips; whole-family closure remains open |
| Usage composition and automatic relationship endpoints | Imported policy, ancestry, containment and redefinitions; named Rust postprocessors | Context, explicit ref/direction/end, variation ownership, constant feature and conjugated-name controls |
| Effective relationship views | Imported redefinition dispatch plus bounded stored ownership queries | All 2595 observed slots match before/after persistence; other derived getters reject |
| Plain KerML inherited lookup | Imported defaults for Type, Classifier, Class, DataType and Structure feed the shared inheritance walker; explicit in-graph standard-library roots | Ten independent whole-document controls match; preserves incomplete flags and rejects missing providers |
| Definition default providers | Resolved javac trees and metaclass dispatch generate 26 policies; handwritten Ecore owned-end and library services supply dependencies | 196 runtime contexts and eleven new source models; individual variability and inherited Flow ends remain open |
| Ordinary Feature names, positional ends and inheritance filtering | Resolved strategy dispatch and selectors feed one ordered native redefinition query | 30/79 ordinary-context bindings with 180 controls; 28 end-selection adapters and two empty selectors with 810 end contexts; six further source models. Parameter/result now has 1200 bounded runtime controls and eight source integrations; constructor and broader inherited-first origins remain open |
| Library aliases and public re-exports | Canonical namespace lookup and pending-field scheduling | Two Pilot exported-index provider disagreements recorded separately; equivalence remains unqualified |
| Global resources, conditional general types and full validation | Separate algorithms remain required | Unqualified; candidate remains unpromoted |

**Plain-Type inheritance milestone:** pinned grammar â†’ canonical Ecore model â†’ generated default-general table â†’ shared inherited-membership traversal â†’ transactional local linking â†’ public model and persistence. Required defaults are resolved lazily from explicit standard LibraryPackages; no synthetic completeness assertion is used. Visibility, diamonds, conjugation, forward references and lexical/import fallback have focused evidence. [Normative/default-provider boundaries](normative-plain-defaults.json) distinguish ten matching documents from two provider disagreements.

**Definition-default milestone:** five resolved Java method bodies (including the empty added-member hook) and runtime dispatch now drive native programs for 26 Definition metaclasses. Their imported representation, consumers, 196 runtime controls, eleven source integrations and remaining dependencies are separated in [the evidence record](normative-definition-defaults.json). The individual source now parses and links with shared Multiplicity naming. Its full-model observation remains separate because Usage.mayTimeVary still differs; it is not counted as matching. Flow uses owned ends in Pilot but `flowEnd` in the written constraint; inherited-end equivalence remains unqualified.

**Shared Feature milestone:** resolved selector trees → native strategy dispatch → one ordered redefinition query → effective names and inherited-membership filtering. [Coverage and evidence](normative-feature-redefinitions.json) distinguish 30 consumed ordinary-context bindings from 49 unsupported strategies. Generated owned/effective end selection now feeds shared generalization, Ecore ownership and inheritance services. Twenty-eight end-selection adapters and two empty selectors match 810 positional contexts; the same native service exposes Type.endFeature. Six further source models match before/after persistence (64 total). Cyclic/conjugated end ancestry, end chain equivalence and unbound providers still reject. Individual Multiplicity naming works; its retained model comparison exposes the separate Usage variability dependency.

**Parameter/result milestone:** eight resolved selector bodies now drive bounded parameter/result selection. Shared native ownership, generalization and inheritance services match 1,200 independent Function-owned controls; 30 explicit invocation controls pass in the full regression run. Eight additional ActionDefinition/CalculationDefinition source models match through persistence (72 total). Return direction uses a named handwritten document postprocessor tied to pinned upstream sources. Cross-category generals, chain equivalence, constructors and other unbound providers remain excluded.

**Inherited-reference increment:** pinned grammar/Ecore reference construction now reaches the handwritten inherited-first Redefinition scope through shared generalization and membership queries. The active feature is excluded from its own target-filter computation. Four additional source models match (76 total); private, missing, self-only and owned-only targets reject. [Normative scope and remaining exclusions](normative-redefinition-scope.json) distinguish the bounded common behavior from broader Pilot/specification lookup differences.

**Specialization prerequisite:** the public native `definition_document::specializes` query now connects imported Type/relationship contracts and generated generalization providers to a cycle-safe handwritten traversal. All 82 independent Pilot pairs across four topologies match. Missing Usage providers remain errors; the retained `Usage.isVariable` disagreement is unchanged. [Scoped evidence](normative-specialization.json) separates this prerequisite from unimplemented variability.

**Variability decision:** the public native `definition_document::may_time_vary` query now consumes the imported delegate binding and shared ownership, library and specialization services. All 32 independently observed complete-generalization controls match, including JSON reload and no-mutation checks. This is a named handwritten algorithm matching the written rule. It does not set completeness flags or qualify ordinary source Usage generalizations; matching document count stays 76. [Exact scope and remaining provider dependencies](normative-variability.json).

**Variation typing provider:** eight source cases now execute the partial variation-typing provider through native specialization and agree with independently invoked Pilot `addVariationTyping`. Part/item/attribute/action variants of definitions and usages are covered. The same eight models pass full observation and persistence comparisons (84 total). Positive paths do not imply complete Usage generalizations; unrelated answers still reject. [Bounded contribution evidence](normative-variation-typing.json).

**Participant contribution:** the imported Feature participant binding now generates the native library name. A named handwritten provider uses Ecore ownership and Subsetting contracts to prove specialization for bare Association/AssociationStructure ends. Six Pilot contexts verify two positive contributions and four non-applicable cases; six additional full models match (90 total). Shared library lookup now resolves direct public names without unnecessary inherited-scope evaluation. [Boundaries and evidence](normative-participant-generalization.json).

**Ordered-chain deduplication:** positional end and parameter redefinitions now share canonical direct chaining-feature comparison. Equal nonempty chains with distinct wrapper identities collapse to the first target; reversing feature order keeps targets distinct. Independent Pilot matrices now cover **990 end cases and 1,440 parameter/result cases** (420 additional cases). All 90 source models still match. Recursive chain construction and bound-value/crossing transformation remain separate. [Scoped evidence](normative-chain-equivalence.json).

**Next ordered implementation batches:**

1. **Qualify complete document families through the common candidate path.** Next semantic batch: complete the Feature/Usage generalization pipeline, then qualify the connected Usage.isVariable/mayTimeVary bridge on ordinary source inputs. Variation typing now supplies bounded positive paths. Bare association-end participant contributions now have bounded evidence. Direct implicit-chain deduplication is now shared and verified. Next ordered provider dependencies: shared Feature bound-value/crossing construction and connector implicit-state handling; OccurrenceUsage classification/portion extras; subtype-specific default selectors. Each provider must reject unimplemented branches and get independent runtime evidence before source integration. Four bounded inherited-first reference controls now match; multiple pending redefinitions, conflicting generals, and cross-resource origins remain unqualified. Ordinary, bounded end, and Function-owned parameter/result dependencies now share the ordered Feature service. Reuse imported adapter strategy dispatch and Ecore ownership; retain errors for constructor, assignment and unbound strategy dependencies. Then bind Usage generalization and mayTimeVary providers to the same semantic queries. Resolve the Flow inherited-end selector boundary against the written rule. Definition default policy and owned-end derivation are now connected; semantic metadata and other adapter-added members remain required dependencies. Preserve explicit unsupported errors until those providers have evidence. Expand independent positive/negative model controls by shared predictor, converter, construction and scope dependencies. Keep syntax rejection distinct from unsupported execution. Migrate whole qualified compiler families rather than adding isolated AST adapters.
2. **Complete shared stored-model lifecycle behavior.** The finite inventory is 87 stored features. Extend transactional ownership/opposites, deletion and reference maintenance; preserve or invalidate computed views explicitly. Migrate remaining import/mutation/publication callers and qualify required-value policies. Shape checks alone do not close this batch.
3. **Close derived-semantic dependency groups.** Inventory 328 derived features and 70 operations separately from parameters. Bind each to shared typed queries or a named handwritten Rust algorithm with explicit dependencies and evidence. Prioritize namespace/name/link services, general/type/redefinition computation, then expression/delegate and metadata/filter services. Keep the eleven-validator Xtend trial bounded; no general compiler is required.
4. **Qualify release behavior.** Finish reviewed behavioral obligations and gate certificates, then process all samples through equivalent native/Pilot contexts, detailed semantic comparison and paired timing. The new bounded milestone certificate does not qualify the release.

**Preserved boundaries:** 1,674 annotations are classified; 190 subset annotations resolve to 224 direct edges and generate transitive snapshot checks. Missing derived values remain unknown. Union order is a guarded handwritten policy, not inferred from annotation text. Automatic subset/union maintenance, complete redefinition semantics and delegate computation remain open. Documentation formulas remain text. Production stays native.

**Normative disagreements remain explicit:** recursive import-all through private namespaces follows the written rule although Pilot's Xtext scope lookup differs from its own derived membership result; imported-name conflict hiding and distinguishability controls also have recorded disagreements. The historical record retains exact sources, controls, earlier milestones and reproduction commands. Shared-Feature aliases also expose a difference: the written membership-based filtering predicate removes both aliases, while Pilot retains them. Stored implied Specializations are another recorded disagreement: native recursive inheritance excludes them under Type.supertypes(true), while Pilot traverses them. See [inheritance evidence](normative-inheritance.json) and [union evidence](normative-union.json) and [archived milestone detail](remaining-gap-history.md). No whole-family qualification is claimed.

**Latest run metrics (2026-10-02, shared binding provider):** matching whole source models **151 (+3)**; complete basic-model bindings **26/30 (+0)**; bounded strategy obligations **3/4 (+0)**; whole families **0/34 (+0)**; release gates **0/5 (+0)**. A single resolved selector now drives separately verified Connector and BindingConnector maps. Added 36 imported selector controls; native qualification covers nine explicit-two-owned-end generalization contexts and 27 admission rejections. Four source structures and eight end queries match Pilot, but the missing-reference source violates binary validity and is excluded from the matching-model total. **112 focused native tests and 117 tooling tests pass**. See [binding provider evidence](binding-provider-evidence.json). Next: shared redundant implicit-general removal (including target-excluded cycles), physical insertion/full end transformation, then membership selection and post-insertion transformation. Binary validation, inherited-end cases and other implicit providers remain open. Candidate remains unpromoted.

**Shared producer milestone:** resolved adapter/getter identities now admit 32 stored-variable and 47 Usage semantic-getter bindings to the shared producer. Usage calls the handwritten may_time_vary service through its imported delegate contract; a stored is_variable flag cannot override it. All 47 classes match two independent Pilot contexts with explicitly complete generalization inputs (94 cases: 45 variable, 49 fixed). The missing-library boundary rejects atomically for 45 classes; the two constant-false exclusions require no library. Neither runtime fixture claims complete native Usage generalization providers, and production never synthesizes completeness flags. This qualifies scoped producer dispatch, not whole classes or ordinary source Usage semantics.

**Next shared batch, in order:**
1. Finish the fixed ordinary-redefinition strategy contract: candidate parse/link/publication/persistence verification for every applicable binding, with explicit treatment of grammar-unreachable classes. The current controlled matrix alone cannot close this obligation. The integration inventory contains accepted observations for 27/30 bindings, with complete basic-model matches for 26/30. FlowEnd, Multiplicity and Usage still lack source observations. BindingConnectorAsUsage now has bounded package-owned source qualification; typed and other owning contexts remain open. SuccessionAsUsage still needs its specialized generalizations before source projections can be qualified. Basic declarations do not qualify the explicit redefinition context matrix. All 30 have declared return sites, so no grammar-unreachable exception is established. Prioritize assignment-created versus delegated-return construction paths, then obtain batched independent source observations.
2. Implement complete implicit-generalization and typed library-resolution dependencies shared by six of seven remaining algorithm groups. Next concrete provider contract: Feature bound-value specialization must resolve the first owned FeatureValue, exclude default values, transform its Expression, obtain its result, construct the ordered expression/result chain, and contribute Subsetting only when owned specializations are empty and direction is absent. Initial-value binding additionally needs Base::things::that and Occurrences::Occurrence::startShot plus featuring. The shared chain constructor exists. Seven expression providers now also have a shared owned-result constructor and readable result query, with 28 Pilot controls and two source/persistence controls. Feature-reference result subsetting now uses shared implied-specialization insertion (24 insertion plus 12 producer controls); derived specific endpoints use canonical containment. Featuring projection and breadth-first closure now have ten independent controls and source/persistence integration; unknown providers and external TypeFeaturing inverses reject. Next: shared connector context compatibility, then feature-reference binding/self-reference handling, invocation result typing/binding and complete expression transformation. Pending-contribution selection/reduction remains separate. Implementing the final Boolean predicate alone does not qualify this provider. The 49 remaining bindings group as Action 19, Expression 20, Requirement objectives 4, Payload 1, Reference 1 (non-transition redefinition fallback now consumed; specialized defaults and transition behavior remain open), Rendering 1, and shared Accept/Send/Terminate additions 3. Keep exact resolved dispatch identities in the imported artifact.
3. Complete expression/multiplicity/cross-feature featuring, ordered closure, connector compatibility and transformation using the same services. Objective discovery is a separate four-binding batch.
4. Review and populate the frozen behavioral contracts, using their mapped existing evidence; the family/dependency/release certificate checker is implemented and remains fail-closed for unreviewed contracts. Then qualify complete samples and paired timings at substantial milestones. No generic MMF, persistence-backend or broad OCL expansion is part of this critical path.



## Candidate lifecycle batches through required-value dispatch (archived 2026-10-02)

Implemented first dependency slice: typed scalar/chain reads, Feature types, generalization prerequisites and cross-specialization retry. Eight independent source-order controls match Pilot chained targets (four positive, four missing-target negatives) without custom resolvers or supplied complete flags. All 166 focused native tests and 62 qualification-tooling tests pass. [Scoped evidence](query-prerequisite-evidence.json). This does not close the real-source integration obligation. The `sameThing.self` pending-link cycle is now resolved by a bounded scoped chain-context consumer: eight reciprocal-subsetting controls match Pilot source targets (four positive, four negative), and Base + ScalarValues + Links constructs/links 269 elements. [Scoped chain evidence](scoped-chain-query-evidence.json). This is construction/linking evidence only; complete library model semantics, validation and the strategy integration obligation remain unqualified. Next, carry scoped context through the remaining connected queries and implement contribution completion and validation over the supplied resource environment. Fresh scoped-chain regression: 167 native tests pass; the two complete real-library input sets retry successfully. The separate-resource batch now preserves individual roots and per-unit grammars, with 10/12 independent Pilot resource-link controls matching and two explicit duplicate-export disagreements. Base + ScalarValues + Links constructs/links 271 elements as three native resources; semantic validation remains not assessed. One batch-boundary regression passes 171 native tests, four audit-tool tests and 14 census-tool tests. [Batch evidence](resource-source-set-evidence.json). Exact resource-path/text assessment caching avoids repeated sibling-source work within a run. The prepared older census lock is historical and must be refreshed before execution. Other query families and general job consolidation remain open. Source-order permutation, forward references, dependency cycles, idempotent edits, rollback and unchanged cached Pilot observations are the focused acceptance checks.

The shared Type/Definition default-contribution batch now uses the imported five plain-Type and 26 Definition selector contracts, qualified library resolution, existing immutable-snapshot reduction and one atomic insertion transaction. The 196 cached Definition cases exercise insertion and replay after persistence; unassessed implied contributions, missing libraries and late batch failures reject without publication. The candidate audit has an opt-in `--definition-type-defaults` stage that reports contributions separately from transformation and validation. [Bounded batch evidence](type-default-contribution-evidence.json). Existing Base/ScalarValues/Links owners require no new defaults; Feature/Association and other adapter contributions remain outside this producer. The subsequent Feature batch composes that stage with the supplied libraries; no owner is marked complete by either producer.

The ordinary non-end Feature default producer now admits only the exact recomputed implied Subsetting, so type/redefinition queries and insertion replay remain usable after persistence. Eight cached source controls and two untyped/data-typed controls exercise this path; malformed targets, specific endpoints, nested relationships and other categories still reject atomically. The existing specialized result-parameter provider remains separate. The candidate audit composes Type and Feature producers with `--definition-defaults`. Base + ScalarValues adds two defaults; the real-library-backed source control adds four including the library defaults. Base + ScalarValues + Links reaches an explicit **Feature-owned end context** dependency at `Links::selfLinks::thisThing`; the Feature batch publishes no partial contributions on this failure. [Bounded Feature evidence](feature-default-contribution-evidence.json). The following Feature-owned end batch addresses that context and its replay admission; required validation remains a separate obligation. Association default production, other adapter categories and final source-set semantic qualification remain open.

The Feature-owned end batch now reuses the imported positional-end strategy for exact Feature ends under an exact Feature owner. Canonical owner/membership checks remain required; only recomputed default Subsetting and positional Redefinition targets are admitted on replay. Eight new independent Pilot source controls cover direct Association typing, a subsetted owner, an extra end and explicit redefinition in both source orders (18 ordered queries). Four wrong-position/owner/membership/typing controls reject atomically. [Feature-owned end evidence](feature-owned-end-contribution-evidence.json). The real Base + ScalarValues + Links set now passes both contribution stages with six inserted relationships and 277 elements. The audit verifies unchanged Type/Feature replay after KIR persistence on all three source sets. Explicit compound and constructed binary crossing wrappers admit only their exact imported untyped default, preserving ordered chain identity; wrong defaults still reject. This does not execute Association/default and other adapter transformations, required derived-value validation, or complete semantic validation. The next bounded batch is to inventory and dispatch the actual required-value/delegate validation dependencies for this small library closure, while keeping other contribution categories explicit.

The required-value consumer batch now separates candidate assessment from partial abstract-syntax persistence. Imported Ecore redefinitions select canonical required endpoints while retaining both inherited and narrower value/type constraints. Existing native reference delegates run before any required derived value is assessed; serialized caches cannot satisfy missing prerequisites. One binding-checked handwritten identity projection serves Membership and OwningMembership, including redefined owning-member endpoints and explicit Element identity. The candidate audit reports invalid and unverified required values separately, after unchanged contribution replay; neither a zero invalid count nor an extracted delegate grants full semantic validation. [Required-value batch evidence](required-value-consumer-evidence.json). The real Base + ScalarValues + Links closure now has zero invalid required values and 497 unverified required-value instances, down from 701 before identity projection. These are scoped declaration instances, not a release completion percentage or independent Pilot qualification of the new projections. The next shared batch is ownership-origin/libraryNamespace and conjugation queries, then annotation and chained-owner projections; expression/result/multiplicity and import providers remain explicit dependencies. Complete contribution transformation and final validation remain open.


### Construction/linking cost batch — 2026-10-03

Pure native phase/reference observers now expose resource construction, linking and publication without introducing clocks or I/O into the language crate. Immutable indexed containment and membership reads reuse checked identities inside a reference query or endpoint read pass; the index ends before graph writes, deferred resolution or retry. Ordered children, Ecore ownership/opposite/type guards and ambiguous-import behavior are preserved. Fresh evidence: **211 definition tests (+2), two focused trace/index controls and seven audit-tool tests pass**. [Cost evidence](construction-cost-evidence.json) records the exact source and diagnostic inputs.

The independent phase-budget probe parsed all sixteen resources and constructed 12,288 unpublished elements. It exhausted its ninety-second linking budget with 34 completed reference attempts; four Collections imports accounted for approximately eighty seconds. This is cost localization, not a successful full-resource comparison or a speedup benchmark. **Next bounded batch:** profile repeated membership-name evaluation in import selection, then share immutable query state across that algorithm while preserving pending dependencies, ordering, ambiguity and invalidation. Do not repeat unbounded closure assessments. Metrics remain **155 models (+0), 50/79 ordinary bindings (+0), 7/9 bounded obligations, 0/34 families, 0/5 release gates**.

### Shared import/name-query batch — 2026-10-03

One immutable native identity/name view now spans reference selection, recursive import expansion and collision checks. Successful name derivations are reused; failures are reevaluated. Retries, graph edits and persistence reloads start fresh. Generated Ecore ownership/type/visibility and effective-name dispatch still drive the consumers; traversal and collision selection remain explicit handwritten algorithms. Fresh checks: **213 definition tests (+2), two cache lifecycle controls, ten audit-tool tests, fifteen certificate tests and fourteen plan tests pass**. [Shared-query evidence](shared-import-query-evidence.json) binds sources, pinned resource contents and the bounded diagnostic.

The fifty-five-second linking probe completed **134 reference attempts**, versus 23 in the earlier name-only probe. It still timed out after constructing 12,288 unpublished elements across sixteen resources; attempts include retries and do not count as newly linked/qualified fields. **Next fixed batch:** replace pairwise import-name collision enumeration with an ordered name index, preserving first-error behavior, candidate order, alternate-name whole-membership hiding, endpoint validation and visibility/cycle exclusions. Verify against existing cached Pilot collision controls plus focused boundary/error-order controls; use a bounded closure retry at the batch boundary. Avoid cross-edit semantic caches and unbounded reruns. Metrics remain **155 models (+0), 50/79 ordinary bindings (+0), 7/9 bounded obligations, 0/34 families, 0/5 release gates**. No new language support or timing gate is claimed.


### Current bounded cost milestone — 2026-10-03

Pure phase/query tracing, immutable identity/name reuse and ordered import-name collision indexing are implemented. The index preserves candidate/other order, alternate-name whole-membership hiding and early missing/deferred endpoint precedence over later name failures. The first candidate retains its original validation scan; subsequent candidates inspect only matching-name positions. Recursive owned subnamespace traversal shares the same immutable view. Fresh checks: **215 definition tests (+2), two collision-boundary controls, ten audit-tool tests, fifteen certificate tests and fourteen plan tests pass**. [Current evidence](import-name-index-evidence.json) binds native sources, cached Pilot controls and exact pinned resource contents; [earlier cost milestones](remaining-gap-history.md) are historical.

The fifty-five-second linking probe again completed **134 reference attempts (+0)** and timed out with 12,288 unpublished constructed elements. Removing pairwise enumeration did not improve measured closure progress. Remaining nested membership/Feature-typing queries take roughly two to three seconds. **Next fixed batch:** share one immutable read/query view through standard-library default binding, root/ancestor discovery and connected general-type/inherited-scope consumers. In particular, root discovery currently rebuilds a full graph index separately for each LibraryPackage. Use indexed containment traversal and successful path reuse confined to that view; preserve ownership/type/error/pending checks and create fresh views after edits/retries. Verify cached source/default/inheritance controls plus focused missing-root, ambiguous-root, reciprocal-ownership and invalidation cases before one bounded closure retry. No cross-edit cache, unbounded assessment or new persistence backend.

Metrics remain **155 verified models (+0), 50/79 ordinary bindings (+0), 7/9 bounded obligations, 0/34 families, 0/5 release gates**. This cost batch closes no additional language-support obligation; full support and all release gates remain open.


### Current bounded cost milestone — 2026-10-03

Library root/ancestor/path lookup now consumes the same immutable identity/name view as connected general-type, inherited-scope and expression-default queries. Indexed parent traversal retains the imported Ecore reciprocal/type/cycle checks. Only successfully resolved local library paths are reused; errors, external/deferred targets and construction requests are not cached. Edits, retries and reloads start fresh. Fresh checks: **217 definition tests (+2), two library-query boundary controls, ten audit-tool tests, fifteen certificate tests and fourteen plan tests pass**. Controls cover missing/ambiguous/nested roots, broken ownership, deferred aliases, edits and persistence; existing cached Pilot default/source/scope controls remain passing. [Current evidence](library-query-evidence.json) separates imported definitions, native consumers, handwritten algorithms and bounded semantic verification. Earlier cost notes are in [history](remaining-gap-history.md).

The fifty-five-second linking probe completed **153 reference attempts (+19 from 134)** and timed out with 12,288 unpublished elements across sixteen resources. This is modest diagnostic progress; attempts include prerequisite queries/retries and do not count as distinct linked or qualified fields. The longest completed query was a nested Collections expression-membership lookup; the in-flight query was a Performances Feature redefinition. **Next fixed batch:** instrument that connected internal type/generalization/name/prerequisite path with caller-owned diagnostic boundaries, identify repeated keys and dominant evaluator, then extend immutable view/index reuse into the measured slice. Reuse existing cached Pilot observations and add focused dependency-order/invalidation controls. Obtain a focused cost control before another closure retry; do not choose another optimization from source inspection alone, weaken pending/unsupported guards, add cross-edit caches or run unbounded assessments.

Metrics remain **155 verified models (+0), 50/79 ordinary bindings (+0), 7/9 bounded obligations, 0/34 families, 0/5 release gates**. This batch closes no additional language-support obligation. Complete sample comparison, semantic qualification and Pilot timing assessment remain required at the integration/final milestone; the candidate is unpromoted.


### Current bounded cost milestone — 2026-10-03

Nested dependency tracing measured inherited-membership work at **28.5 seconds inclusive / 15.0 seconds exclusive** in the prior bounded probe. Recursive inheritance now shares the caller's immutable identity/name/library view for parent public/protected scope and recursive traversal. Imported Ecore guards, visibility, order, exclusions, supplied general-type inputs and deferred/error boundaries remain intact. Trace evidence did not justify another general-type cache. The core observer has no clock or I/O; diagnostic timing remains in tools. Fresh checks: **219 definition tests (+2), two focused result/inheritance controls, five trace-analyzer tests, ten audit-tool tests, fifteen certificate tests and fourteen plan tests pass**. [Current evidence](dependency-query-evidence.json) binds current consumers, frozen historical baseline and measured outcomes. Earlier cost notes are in [history](remaining-gap-history.md).

The same fifty-five-second linking probe completed **156 reference attempts (+3 from 153)**; status **timed_out**. Attempts include retries/prerequisites and earn no semantic credit. Nested inclusive times overlap; the changed tracing exposes work previously charged to uninstrumented subcalls. **Next fixed batch:** use the slowest completed and in-flight dependency frames to isolate the remaining inherited-scope/redefinition cost, establish a focused repeated-input cost control, then change only its shared algorithm. Preserve immutable-view invalidation and unsupported guards; no broad qualification rerun until this resource lifecycle completes or a concrete semantic dependency is reported.

Metrics remain **155 verified models (+0), 50/79 ordinary bindings (+0), 7/9 bounded obligations, 0/34 families, 0/5 release gates**. No additional language-support obligation closes. Complete sample comparison, semantic qualification and Pilot timing assessment remain required; the candidate is unpromoted.


### Current bounded cost milestone — 2026-10-03

A separate trace isolated **28.1 seconds in redefinition filtering, including 8.4 seconds in closure traversal**. The dominant filter now consumes the caller's immutable identity index for membership endpoints, target lookup, owned relationships and explicit-redefinition prerequisite traversal. Imported Ecore ownership/type/opposite guards, prerequisite order, owned/sibling suppression and survivor order remain unchanged. No result cache or completeness flag was added. Fresh checks: **221 definition tests (+2), two focused filter tests, ten audit-tool tests, fifteen certificate tests and fourteen plan tests pass**. The focused controls cover fixed expected results and the prior algorithm, padding, reversed storage, malformed/deferred targets, owned suppression, skipped targets and cycles. [Current evidence](indexed-redefinition-evidence.json) binds the measured pre-change source and current native consumer. Earlier cost notes are in [history](remaining-gap-history.md).

The same fifty-five-second linking probe completed **158 reference attempts (+2 from 156)**; status **timed_out**. Attempts include retries/prerequisites and earn no semantic credit. This is a bounded cost diagnostic, not release qualification or the final Pilot benchmark. **Next fixed batch:** isolate repeated `definition_redefined_features` calls inside closure/owned-feature filtering and the general-type evaluator. The current probe records 12.7 seconds of completed closure work and 17.9 seconds of general-type exclusive work. Reuse the immutable view for their canonical ownership/relationship reads, with a focused matched-query cost control before another resource retry. Preserve pending/unsupported guards. If the resource lifecycle completes, assess its actual semantic result before broad sample qualification.

Metrics remain **155 verified models (+0), 50/79 ordinary bindings (+0), 7/9 bounded obligations, 0/34 families, 0/5 release gates**. No additional language-support obligation closes. Complete sample comparison, semantic qualification and Pilot timing assessment remain required; the candidate is unpromoted.


### Current bounded cost milestone — 2026-10-03

The shared redefinition evaluator now consumes one immutable index for owned relationships, canonical Feature ownership, stored membership endpoints, recursive closures and owned-feature filtering. General-type result-parameter classification uses the same structural ownership reader. Imported Ecore type/opposite/subset guards, first-only reads, duplicate order and unsupported positional/write strategies remain intact. No result cache or completeness flag was added. Fresh checks: **223 definition tests (+2), two focused view controls, ten audit-tool tests, fifteen certificate tests and fourteen plan tests pass**. [Current evidence](redefinition-view-evidence.json) distinguishes imported contracts, native consumers, handwritten dependencies and bounded verification. Earlier notes are in [history](remaining-gap-history.md).

A matched-query control on **12,017 elements / 32 queries** measured the prior native path at **291.701 ms** and the shared view at **1.493 ms**. This is a debug synthetic control with a preexisting caller index and equal checked results, not a release or Pilot speedup. The frozen sixteen-resource fifty-five-second linking probe completed **160 reference attempts (+2 from 158)**; status **timed_out**. Attempts include prerequisites/retries and earn no semantic credit. **Next fixed batch:** use the current slowest and in-flight frames to isolate the remaining shared general-type/closure dependencies. Establish a matched focused control before extending index reuse into that evaluator. If the resource lifecycle completes, assess the actual semantic result before broad sample qualification. Preserve pending/unsupported guards and immutable invalidation.

Metrics remain **155 verified models (+0), 50/79 ordinary bindings (+0), 7/9 bounded obligations, 0/34 families, 0/5 release gates**. No additional language-support obligation closes. Complete sample comparison, semantic qualification and Pilot timing assessment remain required; the candidate is unpromoted.



### Current bounded cost milestone — 2026-10-03

Canonical full redefinition closures now reuse successful results within one immutable reference-query view. The prior trace showed the same closure evaluated five times within one reference attempt, costing about **5.5 seconds**. Keys require pointer-identical canonical objects and unique identities; cloned inputs keep uncached validation. Errors and prerequisites are not cached, explicit pending checks still run, and edits/retries/reloads create fresh views. Filtering-specific exclusions and visibility are outside the cached full closure. Imported Ecore guards, generated strategy selection and unsupported boundaries remain intact. Fresh checks: **225 definition tests (+2), two focused cache controls, ten audit-tool tests, fifteen certificate tests and fourteen plan tests pass**. Five matched requests yield one evaluation; clone rejection, uncached failures, endpoint repair, cyclic edit, reload and reversed storage are verified. [Current evidence](redefinition-cache-evidence.json) preserves exact prior/current provenance. Earlier shared-view cost controls and milestones are in [history](remaining-gap-history.md).

The frozen sixteen-resource probe now **terminates naturally with a concrete semantic blocker**, rather than timing out. All sixteen resources parse; linking reports its guarded rejection in **45.917 seconds**, after **166 reference attempts (+6 from 160; +8 across this run)**. There are **4,846 closure requests / 761 actual evaluations** in this immutable-view trace. The process exits 1 with candidate status `blocked`; this is not successful linking, publication or semantic qualification. The error is **`Intersecting.intersecting_type` requires a scope-origin service while resolving `OrderedCollection`**. [Exact result](definition-pipeline-evidence/performances-redefinition-cache-results.jsonl) is bound in the current capsule.

**Next fixed semantic batch:** audit all pinned Type-owned intersection/union/disjoining cross-reference origins using the structured grammar/Ecore representations, resolved Pilot scope provider and written specification. Implement one shared definition-directed origin policy, explicitly labeling its handwritten scoping algorithm. Cover lexical/inherited/imported context, identity/order, malformed/detached ownership and dependency scheduling with focused independent controls. Retain guards where required semantics remain unsupported. Run the existing resource pipeline only at the completed batch boundary; defer broad sample/Pilot/timing qualification until a complete candidate lifecycle is available.

Metrics remain **155 verified models (+0), 50/79 ordinary bindings (+0), 7/9 bounded obligations, 0/34 families, 0/5 release gates**. No additional language-support obligation closes. Complete sample comparison, semantic qualification and Pilot timing assessment remain required; the candidate is unpromoted.



### Current bounded semantic milestone — 2026-10-03

Five stored Type-relationship reference fields from six resolved Xtext grammar occurrences now select a native nearest-containing-Namespace origin through generated admission data. The imported Ecore target contracts remain authoritative. The scoping algorithm is explicitly handwritten Rust, reviewed against the pinned generic Element fallback and NamespaceUtil; Specialization retains its separate origin rule. The written namespace name-resolution and visibility rules agree for this bounded behavior. **33 independent Pilot source controls (21 positive / 12 negative) match** across lexical, nested shadowing, inherited, imported, qualified, missing, wrong-kind and private contexts, including both standalone Disjoining endpoints. Native checks preserve canonical endpoint identity through serialization and reversed storage. Owned feature-chain transformation, complete Type set semantics and validation remain required and unsupported by this batch. [Scope evidence](type-relationship-scope-evidence.json) connects imported definitions, generated selection, the native consumer, semantic dependencies and verification.

Fresh checks: **226 native definition tests (+1), three generator drift tests, ten audit-tool tests and exact Pilot cache reproduction pass**. Updated bounded certificates retain the strict existing acceptance contracts. This does not close a whole language family or qualify another whole Pilot model.

The fixed sixteen-resource assessment parses all sixteen inputs and passes the previous Intersecting origin rejection. It is stopped at the unchanged **55-second linking budget**, after **179 reference attempts (+13)**, with 178 completed traced attempts. There is no new semantic rejection, published model, or completed linking result; the next in-flight reference is a Collections Membership endpoint. [Assessment supervision](definition-pipeline-evidence/performances-type-relationship-supervision.json) and [query analysis](definition-pipeline-evidence/performances-type-relationship-analysis.json) preserve this boundary. These are diagnostic timings, not a Pilot performance comparison. Do not repeatedly rerun this closure or infer semantic qualification from reference attempts.

**Next coherent batch:** compose existing expression default/result algorithms in MultiplicityRange and FeatureValue contexts using the actual frozen library environment. Prepare cached upstream controls and focused native tests for the shared dependency contract first. Assess the full resource closure only after this connected semantic batch, with an explicitly reviewed diagnostic budget; retain unresolved dependencies rather than manufacturing models or completion flags.

Metrics remain **155 matching whole models (+0), 50/79 ordinary bindings (+0), 7/9 bounded obligations, 0/34 families and 0/5 release gates**. Complete sample semantics and comparable Pilot timings remain required. Candidate unpromoted; no defensible overall percentage.



### Current bounded semantic milestone — 2026-10-03

The seven imported owned-result constructor bindings now share a whole-document additional-member stage. It clones/stages once, derives stable identities from canonical owners, reuses generated constructor selection and Ecore ownership contracts, and commits only after closed structural publication succeeds. A late collision leaves the entire input unchanged. The opt-in candidate audit `--definition-defaults` runs this native stage before default contributions and checks idempotent replay after persistence. The existing plain parse path remains unchanged. This orchestration is explicitly handwritten Rust; the imported semantic constructors select applicable bindings and construction types/direction.

All **28 cached independent Pilot constructor contexts across seven bindings** match through the shared batch. Existing/empty return memberships suppress construction; missing results, ordinary members, canonical identity, reversed storage, persistence and unsupported-provider separation are verified. Parsed literal multiplicities with reference/operator FeatureValue expressions exercise the public API, with literal results still explicitly unresolved. Fresh checks: **229 native definition tests (+3), eleven audit-tool tests (+1) and five generator checks pass**. [Batch evidence](owned-expression-result-batch-evidence.json) separates imported rules, native consumption, stage verification and remaining semantics. This does not qualify literal result inheritance or complete expression transformation.

**Concrete next prerequisite:** reference-valued multiplicity bounds currently fail before this result stage because the linker has no native MultiplicityRange generalization provider. The pinned MultiplicityAdapter uses a shared owner-sensitive classifier/feature/base selector for both Multiplicity and MultiplicityRange and supplies an empty relevant-Feature list. Export/resolve this small semantic program and qualify all applicable bindings, ownership contexts and imported library names in one focused batch. Preserve crossing/featuring/transformation dependencies separately. Then compose actual library defaults with inherited literal result queries; do not create stand-in result objects or set completion flags.

The prior sixteen-resource probe remains the latest resource assessment: all inputs parse, 179 reference attempts, stopped at the unchanged linking diagnostic budget with no published model. No repeated full resource/Pilot comparison or benchmark was run for this stage-only batch. Reserve the next resource assessment for completion of the connected Multiplicity/default/result dependencies.

Metrics remain **155 matching whole models (+0), 50/79 ordinary bindings (+0), 7/9 bounded obligations, 0/34 families and 0/5 release gates**. Full sample semantic comparison and comparable Pilot timings remain required. Candidate unpromoted; no defensible overall percentage.


### Current bounded semantic milestone — 2026-10-03

A resolved typed MultiplicityAdapter program now generates the owner-sensitive defaults for both Multiplicity and MultiplicityRange. The generated selector preserves Classifier-before-Feature precedence and consumes the pinned implicit library map. An explicitly handwritten canonical Element.owner projection uses imported Ecore containment/inverses; shared native library lookup, generalization and transactional contribution services consume the selected names. Metadata-derived bases, malformed ownership, missing libraries, conjugation/crossing/chaining/valuation and unassessed partial states retain explicit guards. This implements bounded default/generalization inputs, not complete Multiplicity featuring, typing, evaluation or validation.

All **20 independent Pilot selector/empty-relevant-feature controls** match across both bindings, five owner categories and two membership kinds. The exact pinned **Base.kerml and ScalarValues.kerml** texts are cached with verified release revision and source hashes. In both resource orders, actual library source plus reference-valued bounds now parses/constructs/links; the candidate constructs three owned expression results, contributes a physical MultiplicityRange Subsetting to actual Base::naturals, persists/reverses graph storage and replays both stages with zero new insertions. No stand-in declarations/results or completion flags are used. [Multiplicity evidence](multiplicity-defaults-evidence.json) separates imported program, native consumer, selector verification and the narrower actual-source boundary.

Fresh checks: **232 native definition tests (+3), four generator drift checks, eleven audit-tool tests and exact Pilot cache reproduction pass**. This clears the prior missing MultiplicityRange provider for this actual two-library source path. It does not qualify numeric bounds, featuring, a complete expression transformation, or another complete Pilot sample.

**Next connected batch:** qualify inherited literal result queries/defaults against the actual Performances resource environment, composing the existing default and result algorithms. The prior frozen sixteen-resource probe remains unqualified at its diagnostic timeout (179 reference attempts). Inspect its current query/dependency evidence and choose one connected semantic/cost boundary before one milestone reassessment; do not repeat broad probes per small fix. Retain unsupported result typing, featuring, valuation and validation boundaries explicitly. Multiplicity typing/featuring remain required later lifecycle work.

Metrics remain **155 matching whole models (+0), 50/79 ordinary bindings (+0), 7/9 bounded obligations, 0/34 families and 0/5 release gates**. Complete sample semantic comparison and comparable Pilot timings remain required. Candidate unpromoted; no defensible overall percentage.


### Current bounded semantic milestone — 2026-10-03

The current sixteen-resource candidate completed a diagnostic assessment in **121.406 seconds**, with **191/191 reference attempts finished**, then rejected a concrete standalone Subsetting inside an OwningMembership. It did not time out, publish a model or reach default/result contribution qualification. The linking diagnostic allowance increased from 55 to 180 seconds; this is not a comparable benchmark or a speedup claim. [Frozen assessment](performances-multiplicity-milestone-evidence.json) retains the pre-fix binary/source identity and exact source set.

One shared bounded batch now distinguishes an owned relationship declaration from direct Feature typing/generalization. Imported Ecore ancestry selects Specialization, Disjoining and FeatureInverting descendants, while canonical membership containment keeps their declarations out of the enclosing Feature's direct ownedTyping/ownedSpecialization inputs. The explicit handwritten scope dependency now follows the relationship's immediate container, which may be an OwningMembership rather than a Type. FeatureTyping's non-expression origin and Redefinition's inherited-target exclusion remain separate semantics. Implied partial declarations, malformed membership and unassessed relationship algorithms retain guards; no completion flags are inferred.

**50 cached independent Pilot projection controls** match for ten concrete relationship classes across five explicit typing contexts. Three actual pinned-grammar source controls verify two standalone endpoint positives and a local Redefinition negative. Native positives parse, construct, link, preserve canonical membership/endpoint identities and persist. Factory graphs also reverse storage and preserve read-only queries. This is enclosing-Feature projection and bounded source-scope evidence, not ten complete relationship implementations. [Batch evidence](relationship-membership-evidence.json) separates imported contracts, native consumers and these verification boundaries.

Fresh native regression: **235 definition tests pass (+3)**, eleven audit tests, 33 certificate tests and fourteen structural-plan tests pass; both Pilot caches reproduce exactly. Verified models **155 (+0)**, ordinary Feature bindings **50/79 (+0)**, bounded obligations **7/9 (+0)**, whole families **0/34 (+0)**, release gates **0/5 (+0)**. The candidate stays unpromoted. No new whole-resource or sample semantic qualification is claimed.

**Next connected batch:** finish actual Performances closure and inherited literal result/default composition. The frozen resource assessment predates this membership correction; its wrapper guard has focused native evidence, but the complete source set remains unqualified. Reuse the fixed source inventory and cached strategy controls, preserve explicit unsupported dependencies, and make the next resource assessment at a connected semantic batch boundary. Multiplicity featuring, typing, evaluability and validation remain separate required work.


### Current bounded semantic milestone — 2026-10-03

The fixed sixteen-resource assessment passed the previous standalone-membership guard and terminated at **inherited Feature filtering's missing additional-member stage**: 138.390 seconds, 191 completed reference attempts, no timeout or published model. [Frozen assessment](performances-membership-milestone-evidence.json) predates the correction below; its unchanged diagnostic budgets are not a complete qualification benchmark.

One connected native batch now carries an explicit AdditionalMembers prerequisite from inherited filtering to the transactional linker. It runs the existing resolved, generated owned-result producer, attaches canonical return/result membership, rebuilds the lookup index and retries the active field. Shared readiness then lets parameter/effective-feature/filter consumers inspect the assessed stored member. All seven imported providers reuse this strategy. Construction remains demand-driven; partial implied states, unsupported constructor producers, malformed graphs and final closed-publication checks retain their boundaries. No completion flags are inferred. [Batch evidence](additional-member-dependency-evidence.json) separates the existing imported producer, its new native scheduler consumer and focused verification.

**238 native definition tests pass (+3)**; eleven audit, 33 certificate and fourteen structural-plan tests pass. Focused controls verify typed read-only prerequisites, seven-provider/four-context readiness, transactional scheduling, retries, rollback, unsupported/repeated requests and zero-insertion replay. Pilot's own Xtext library provider also supplies **eight cached literal/null result observations against sixteen actual pinned library resources**, with real EObject identities, canonical ReturnParameterMembership/out results and no observed resource diagnostics. The cache reproduces and four acceptance controls pass. These are prepared independent reference observations; **native real-library matches remain zero** until the candidate closure completes and identities compare.

Release metrics are unchanged: models **155 (+0)**; ordinary Feature bindings **50/79 (+0)**; bounded obligations **7/9 (+0)**; whole families **0/34 (+0)**; release gates **0/5 (+0)**. Candidate remains unpromoted.

**Next connected batch:** exercise the scheduled producer through automatic linking of the fixed real-library cohort, then compose inherited literal result/default queries and compare the eight cached Pilot observations. Account for any further unsupported producer, typing, featuring or evaluability dependency explicitly; numeric evaluation, validation and final full sample/timing qualification remain required.


### Current bounded semantic milestone — 2026-10-03

The fixed sixteen-resource assessment now demonstrates **automatic additional-member scheduling and retry**: canonical owned return/result construction grows the graph from 12,288 to 12,290 elements. It terminates at the next dependency, a directed operand parameter's default/generalization context, after 150.391 seconds and 192 completed reference attempts. No model is published. [Frozen assessment](performances-additional-member-milestone-evidence.json) predates the parameter correction below; this failed diagnostic run is not a qualification benchmark.

The connected native parameter batch reuses imported Ecore ownership/ancestry, the resolved parameter predicate, the existing generated default selector and shared positional redefinition/reduction algorithms. Fixed directed exact Features under Behavior/Step-compatible owners now supply read-only general/default queries and canonical physical contributions. Added-member and owner-field dependencies remain typed. Canonical valuations are preserved; expression evaluation and full transformation are separate obligations. No completion flags are fabricated. [Parameter evidence](parameter-defaults-evidence.json) separates imported definitions, native consumers and semantic verification.

**241 native definition tests pass (+3)**; eleven audit, 33 certificate and fourteen structural-plan tests pass. All **540/540 cached Pilot parameter-selector controls** match across six owner classes, two membership kinds, three directions, five typings and three valuation states, with reversed storage and persistence. Native parsed-source controls compose positional redefinition, inherited default reduction, physical insertion and zero-insertion replay. Those small source controls declare test dependencies; they do not establish actual-library qualification. The existing **eight cached Pilot literal/null results against sixteen actual libraries still have zero qualified native matches**.

Release metrics remain: whole models **155 (+0)**; ordinary Feature bindings **50/79 (+0)**; bounded obligations **7/9 (+0)**; whole families **0/34 (+0)**; release gates **0/5 (+0)**. Candidate remains unpromoted.

**Next connected milestone:** compose this parameter/default strategy through the actual library-backed candidate lifecycle, qualify inherited literal/null results against the eight cached Pilot identities, and record any further shared algorithm dependencies. Nonconstant evaluation, featuring, complete validation and final sample/timing qualification remain required. Reuse reference observations and focused tests; reassess the real source set once at the next coherent batch boundary.



## 2026-10-03 prior completion narrative

# 2026-08 native-support critical path

**Full support remains open; no defensible overall completion percentage is established.** This is the single active plan. The [machine-readable checklist](structural-source-coverage.json) tracks imported identities, native consumers, dependencies and evidence. [Historical milestones](remaining-gap-history.md) do not define additional plans.

The written pinned specification is normative; Pilot is an independent behavioral reference. Import, native implementation and semantic verification are separate gates. Production stays native; Java is build-time tooling. The candidate remains unpromoted.

## Completion measurement

The [frozen measurement baseline](measurement-baseline.json) records exact input hashes and rule identities: 141/727 strict generated rules (19.39%), 38 roots, partial consumers and bounded evidence in 21/21 active Xtext families and 12/13 Ecore families. These dimensions are not an overall support percentage. Run `python -B tools/measure_support_baseline.py` for a current snapshot; `--check` detects changes since the baseline and `--write` refuses to overwrite the frozen record.

The generated checklist now reports **0/34 whole feature families qualified**, one unused family, and **0/5 release gates qualified**. These are coarse closure counts, not a percentage of implemented behavior. The five gates are behavioral-inventory review, native pipeline, complete samples, Pilot semantics and timing assessment; each has explicit acceptance criteria and a current blocker in the curated plan. Gate deletion, duplication and unsupported closure claims fail validation. An identity fingerprint makes changes to the selected upstream inventory visible.

**Existing measurement mechanism:** the first reviewed strategy contract has four stable obligations with hash-bound certificate validation: three close against fresh named test outcomes; candidate integration across all applicable bindings remains open. Contract changes, missing obligations, stale source/observations/logs, missing tests and open dependencies reject closure. These are bounded strategy obligations, not the release behavioral denominator. Whole-family, dependency and release certificates now have a checked format: every subject needs reviewed imported/native/semantic obligations, exact verification contexts and passing evidence, with fresh inputs and closed dependencies. Sample, semantic and timing gates enforce their own complete reports and one shared frozen sample manifest. The enum-literal family contract is now reviewed with integration still pending; the other 46 contracts remain unreviewed. No release acceptance requirement is relaxed. Expand the reviewed obligation inventory by complete algorithm groups, not individual sample additions.

## Required report after every run

Report a compact metric after every manual or scheduled continuation, including runs with no verified gain. This is the user's reporting preference as of 2026-10-01 and supersedes the earlier instruction to stay quiet when nothing changed. The existing continuation automation reads this plan.

Use the preceding completed run as the delta baseline. Report verified matching Pilot document models, the scoped semantic coverage changed by the batch, whole-family closure and release-gate closure. Keep denominators and context boundaries explicit; newly imported definitions, generated rules and passing test counts do not count as newly verified behavior. Record inventory additions separately from coverage gains. Do not invent an overall completion percentage while the behavioral denominator remains unreviewed.

Reporting baseline established on 2026-10-01:

| Metric | Baseline | Scope |
|---|---:|---|
| Matching Pilot document models | 58 | Full observations and persistence for these controls only; disagreements and unsupported models excluded |
| Ordinary Feature adapter bindings verified | 30/79 | Ordinary contexts only, supported by 180 Pilot runtime observations; positional and specialized strategies remain open |
| Whole feature families qualified | 0/34 | Partial consumers do not close a family |
| Release gates qualified | 0/5 | Gate acceptance and certificate requirements remain unchanged |

Example: `Verified models 58 (+0); ordinary Feature bindings 30/79 (+0); families 0/34 (+0); release gates 0/5 (+0). Tests this run: <actual result or not run>. Next: <one remaining dependency>.`

State whether test and comparison evidence is fresh for the current sources. If tests were not run, say so and identify any carried-forward results as prior evidence. At this reporting-only baseline, no tests were rerun; the last implementation qualification recorded 714 passing native library tests and 32 passing tooling tests. Update the current milestone and its evidence when implementation changes these counters; keep this initial baseline fixed so later reports can show both run-to-run and cumulative progress. Report failures or blockers even when coverage does not change.

## Reassessed critical path â€” 2026-10-02

**Diagnosis:** the imported definitions and existing native algorithms remain the foundation. The immediate integration defect in the Feature/library path is lost dependency information: nested Feature/type/default queries turn unresolved pending references into generic errors before the transactional linker can schedule them. Adding one construction enum variant per failure has not completed the semantic lifecycle. The public candidate also explicitly reports `semantic_validation: not_assessed`; construction success cannot close validation obligations.

**Correct the measurement:** 141/727 is strict fragment admission, not candidate parser breadth. The whole-document candidate emits 718 distinct pinned rule identities across 275 KerML and 542 SysML entries; shared identities count once. Nine declared identities are outside that emission inventory, including separately handled terminals. These are emission counts, not verified parsing or semantics. `tools/measure_support_baseline.py` now reports both paths separately; the original frozen baseline is retained.

**Current evidence:** 155 matching bounded Pilot whole models; 26/30 basic binding models; ordinary strategy 3/4 obligations. The separately scoped crossing lifecycle contract closes 4/5 obligations using existing stage evidence and leaves real-source integration open. Thus 7/9 obligations across these two bounded contracts are closed; this is evidence mapping, not new behavior or a release percentage. Whole families remain 0/34 and release gates 0/5. All 47 family/dependency/gate contracts still need their behavioral obligation review; no overall percentage is justified. Prior native evidence was 166 focused passing tests, with 62 qualification-tooling tests for the dependency batch; the scoped-chain milestone below supersedes this native evidence with 167 focused passing tests. This reassessment changes planning and measurement, not native behavior. Fresh reassessment checks: 49 tooling tests pass (15 strategy, 14 structural, 14 census, 6 measurement); native tests were not rerun. The [corrected measurement](definition-pipeline-evidence/reassessment-measurement-2026-10-02.json) and [prepared census lock](definition-pipeline-evidence/census-2026-10-02/input-lock.json) preserve the exact current inventories. The census has been prepared, not executed.

| Order | Bounded deliverable | Exit evidence |
|---|---|---|
| 1. Freeze scope and reuse evidence | Preserve the pinned 310-sample inventory (252 SysML, 58 KerML), 96 existing sibling/dependency source sets and 94 library sources with hashes. In one review pass, map the 47 final contracts to explicit behavioral obligations and existing named evidence. Keep partial strategy closures separate from full-family closure. | Missing sources, duplicates, changed grouping and stale inputs reject. Each required subject is mapped to an imported definition, native consumer, semantic dependency and verification context, or explicitly open. No new certificate framework is required. |
| 2. Compose existing semantic queries | Introduce one internal result protocol for ready values, registered pending fields/contributions, unsupported behavior and invalid models. Migrate the connected slice: reference reads â†’ chain targets â†’ Feature types/generalizations/defaults â†’ inherited scope â†’ crossing specialization/featuring. Reuse the existing transactional linker and named algorithms. | Combined source controls run in both declaration orders with real dependency scheduling, no custom resolver or fabricated completion flags. Cycles, missing nonpending fields, unsupported adapters and failed edits remain distinct and publish nothing. A real-library retry produces either completion or a concrete unsupported-algorithm trace; moving the first generic error alone earns no closure. |
| 3. Complete the library-backed candidate lifecycle | Add resource-aware source identities and linking; preserve the actual source-set/library environment instead of concatenating the whole library into each sample. Implement resource loading, contribution completion and validation as separate bounded sub-batches, then compose them with persistence. Resolve the real Base + ScalarValues + Links closure before expanding its library dependency closure. | Parse, construct, link, complete, validate and persist are separately reported. Combined lifecycle observations match cached Pilot controls and the written specification. The scoped strategy integration obligation closes only on this evidence. No unconditional `is_implied_included` or semantic-success stamp. |
| 4. Close categories using a candidate census | Run one native-only, provenance-bound census at the integration milestone. Group first blockers by shared algorithm and affected source sets; rerun affected groups during edits. Complete remaining stored lifecycle policies, delegate/operation strategies, and Xtext lexical/prediction/construction categories in ranked batches. | Every batch has a fixed applicable binding/context set and either closes its reviewed obligations or names a concrete remaining dependency. Parse success, structural success and semantic success stay separate. A first-blocker census is a lower bound, not a complete failure inventory. |
| 5. Qualify the release | Use the candidate pipeline for every pinned sample with equivalent resources; perform detailed independent Pilot model/diagnostic comparison and comparable timing. Promote only after all required gates close. | All required Ecore/Xtext behavior and samples verified; native behavior verified against the written specification, with Pilot disagreements documented; recording a disagreement alone cannot close an obligation. No silent exclusions. The legacy sample compiler cannot supply candidate qualification. |

### Immediate implementation batch contract

The connected Feature prerequisite and separate-resource construction/linking batches now have bounded proof. The next runtime batch is **complete contribution/validation lifecycle for the library-backed candidate**, using the same shared query and transactional implementation. Use a small sibling module for query/job identities where that reduces duplication; do not rewrite the parser, implement a general semantic engine, or add a new persistence backend. Only fields in the actual pending registry may be deferred. Stage readiness belongs to the evaluation context, not persistent model flags. Keep existing unsupported metadata, nonbinary and compound-chain behavior explicit until its own algorithm is implemented and verified.

The later Ecore lifecycle batch is bounded by the actual pin: 87 stored declarations, 328 derived declarations and 70 operations. Group them by resolved algorithm and context. The imported flags contain no `unsettable=true` and only one `changeable=false`; do not expand this release into all possible EMF behavior. Existing required-value validation must call implemented derived queries before their behavior can count as verified.

The candidate now constructs/links separate resources, contributes bounded Type/Feature defaults, and verifies unchanged replay after KIR persistence. [Resource evidence](resource-source-set-evidence.json), [Type defaults](type-default-contribution-evidence.json), [Feature defaults](feature-default-contribution-evidence.json), [Feature-owned ends](feature-owned-end-contribution-evidence.json), and [required-value dispatch](required-value-consumer-evidence.json) retain their precise scope and unsupported contexts. Prior batch detail is archived in [remaining-gap-history.md](remaining-gap-history.md).

**Latest verified batch:** resolved Ecore `modelLevelEvaluable` metadata generates the complete typed signature and nine dynamic branch candidates. Most-specific native dispatch implements LiteralExpression, NullExpression and MetadataAccessExpression constant-true operation bodies; six other algorithms remain explicitly unsupported. Six source controls and seven detached factories compare independent Pilot attribute/operation results. Persistence, cached-value rejection and unsupported-branch reporting are verified. [Evaluability evidence](evaluability-evidence.json). The 195-test definition regression passes (+2), with 12 Ecore generator tests (+2). Base + ScalarValues + Links retains 277 elements and unchanged replay, with zero invalid and **28 unverified required-value instances**, down from 56; all concern expression results. Matching Pilot whole models remain 155 (+0), whole families 0/34 and release gates 0/5. Evaluability does not implement numeric evaluation, result construction or full semantic validation.

**Latest resource batch:** pinned Xtext frontends import 94 library sources and 582 import declarations; a shared package-prefix consumer selects 16 real Performances dependency resources (121,245 bytes, zero unknown explicit import prefixes). This is source selection, not semantic or implicit/global closure. Nine resources parse natively; KerML.kerml reaches the fixed 100,000-step execution budget before construction/linking. The first-blocker assessment is recorded in [resource evidence](performances-resource-evidence.json). Fresh checks: 196 definition tests, 78 tooling tests and 10 audit-tool tests pass. Verified Pilot models remain 155 (+0), families 0/34 and release gates 0/5. The 28 expression-result gaps belong to the prior small cohort and were not reassessed on the expanded source set.

**Latest native parser milestone:** the same frozen 16-resource source set now parses completely (**16/16, +7**). A shared execution policy credits only new consumed-token progress, retains an absolute work ceiling and existing cycle/depth limits, and reuses successful immutable-program predicate decisions within one parse. Fresh checks pass: 57 parser tests (including large valid input, bounded no-progress work, cleanup and cache isolation), 196 definition tests, 10 audit-tool tests and 15 certificate tests. [Parser-work evidence](parser-work-evidence.json) binds the exact candidate binary, pinned grammar programs, source manifest and outcomes. Construction/linking now stops at **default generalization requires semantic metadata assessment**. No document is published; no semantic validation or release gate is qualified. Matching Pilot models remain 155 (+0), families 0/34 and gates 0/5.

**Latest shared metadata batch:** imported Ecore ancestry, delegate bindings and reciprocal ownership now drive owner-specific `ownedAnnotation`/annotating endpoint projections and metadata selection. Six default-provider paths consume the shared empty-metadata-base assessment; unrelated metadata, documentation and aliases no longer trigger a global rejection. Nine independent Pilot factory controls match identity/order observations; persistence, reversed storage, invalid/missing targets, stale derived values and applied-metadata rejection are verified. A source control verifies inherited linking in both declaration orders. Fresh checks: **199 definition tests (+3), 15 certificate tests, 14 plan tests and 10 audit-tool tests pass**. [Metadata evidence](metadata-selection-evidence.json) records the written specification, per-feature verification boundaries and remaining dependencies. The owning-annotating endpoint consumer still needs an independent fallback control; metadata metaclass/baseType evaluation and pending-target scheduling remain unqualified.

The optimized assessment parses all 16 frozen resources and clears the previous global metadata blocker. Construction/linking now stops at **computed redefinitions require an unsupported adapter strategy for Invariant**. This is a first-blocker observation, not a complete failure inventory. No document is published or promoted. Matching Pilot document models remain 155 (+0), families 0/34 and release gates 0/5; nine factory controls are scoped semantic evidence, not nine additional whole models.

**Latest expression redefinition batch:** a build-time resolved typed Java-tree export and bounded reviewed selector contract drive twenty concrete expression bindings through the shared native Feature query. All **120 ordinary Pilot factory controls** match ordered redefinitions/names, reversed storage, closed publication and persistence; **60 guard-classification controls** match with invalid/missing enum checks. Transition-guard redefinition, FeatureValue owning-Type semantics and positional expression algorithms remain explicitly unsupported. This is specialized bounded generation, not a general Java/Xtend translator. Fresh checks: **201 definition tests (+2), 11 focused expression tests, 5 generator tests, 15 certificate tests and 14 plan tests pass**; the cache reproduces exactly. [Expression evidence](expression-redefinitions-evidence.json) preserves the scope and remaining dependencies. Scoped ordinary binding coverage is **50/79 (+20)**; matching whole models remain **155 (+0)**, families **0/34**, gates **0/5** and bounded obligations **7/9** unchanged.

The single optimized reassessment parses all sixteen resources and clears the Invariant redefinition guard. Its next first blocker is **expression defaults require detached, package or FeatureValue contexts; performance/positional contexts remain open**. No linked document is published and semantic validation remains not assessed. The remaining 29 bindings group into six resolved algorithm combinations, including 19 shared ActionUsage bindings; these are audit groups, not qualified behavior.

**Latest expression conditional-default batch:** the resolved six-method program now drives an ordered list of base/owned-performance/subperformance/enclosed-performance defaults across thirteen base providers. Shared Ecore ownership/ancestry and the existing exact-Feature typing query supply explicitly handwritten predicate dependencies. **182/196 new Pilot default-stage controls** match native public generalization/specialization queries, atomic contribution insertion, persistence and reversed graph storage. The remaining **14 composite-Step cases** reject unsupported Step typing without mutation. A parsed behavior/expr source verifies both declaration orders and atomic failure on a late identity collision. Fresh checks: **203 definition tests (+2), 13 focused expression tests, 5 generator tests, 15 certificate tests and 14 plan tests pass**. [Conditional-default evidence](expression-context-defaults-evidence.json) records each boundary. This is not complete expression transformation or validation. Whole models remain **155 (+0)**, ordinary bindings **50/79 (+0)**, families **0/34**, gates **0/5**, bounded obligations **7/9**.

**Normative disagreement:** PDF constraints/semantics on pages 220/292 name singular `ownedPerformance`, `subperformance` and `enclosedPerformance`; the same PDF's library definitions on pages 344/354 and the pinned release library/Pilot use plural names. Native linking consumes the actual pinned declarations. The naming inconsistency remains explicit and earns no full normative qualification.

The single optimized assessment parses all sixteen frozen resources, clears the expression-default context blocker, and now stops at **type/inherited scope service is required: no document generalization provider for Step**. Semantic qualification remains not assessed; no linked document is published or promoted. This is the next first blocker, not a complete failure inventory.

**Latest shared Step batch:** the resolved typed Step selector generates exclusive default ordering for the exact Step binding. Imported Ecore ancestry/ownership and handwritten shared type traversal supply the semantic prerequisites. **56/56 independent default-stage controls and 6/6 typing controls match**, including owned-versus-aliased payloads, inheritance cycles, diamond duplication and incomplete implicit-default typing. Parsed package/step source reaches the provider in both declaration orders; physical contributions, public queries, persistence and replay pass without changing completion flags. Overriding adapters, directed/end steps, applied metadata, valuation/chaining/crossing and unsupported partial states remain explicit. The fourteen previously rejected composite-Step expression cases now match: **196/196** in that fixed matrix. [Step evidence](step-strategy-evidence.json) separates import, native consumption and verification.

Fresh checks: **206 definition tests (+3), 3 focused Step tests, 5 generator tests, 15 certificate tests and 14 plan tests pass**. The pinned cache and generated selector reproduce exactly; the checked certificates remain **7/9**, whole families **0/34**, release gates **0/5**, whole models **155 (+0)** and ordinary bindings **50/79 (+0)**. The corrected reference fixture resets library lookup between stored and implicit-default typing observations; contaminated initial observations earn no qualification. Naming and mutual-type-reduction normative disagreements remain open.

**Batch-boundary assessment:** one optimized run of the same sixteen-resource Performances closure parses all resources and clears the Step provider blocker. Construction/linking now stops at **no document generalization provider for Behavior**. This is a first-blocker observation; semantic qualification remains not assessed and no linked document is published or promoted.

**Latest resolved plain-Type batch:** the imported typed TypeAdapter base selector and runtime method dispatch now generate admission and names for **14 shared non-Feature bindings (+9)**. The audit retains **24 excluded overriding bindings** with explicit method identities. **98/98 Pilot query-stage controls** match, adding 78 observations beyond the prior 20-control matrix: ordinary/default overlap, self defaults, private/protected inheritance and stored-implied incomplete states. Shared handwritten traversal, metadata admission, local linking and canonical contribution insertion consume the generated table. All bindings pass physical contribution replay and persistence against supplied factory dependencies; parsed Behavior/Function/Predicate/Metaclass source reaches public specializes queries and contribution consumers in both declaration orders. Completion flags remain unchanged. [Provider evidence](plain-type-provider-evidence.json) preserves import/consumer/verification boundaries.

The independent partial-state controls confirm Pilot returns both stored and recomputed copies of a default while incomplete; native keeps that ordered query behavior and deduplicates inherited membership identities. Canonical containment and explicit lookup aliases are separate in the replay fixture, preserving conjugated-port effective-name rules. Factory controls do not establish actual Ports closure or conjugated-port origination. Function result connectors, applied metadata, custom occurrence/association providers and complete validation remain open.

Fresh checks: **207 definition tests (+1), 5 focused provider tests, 5 generator tests, 15 certificate tests and 14 plan tests pass**; cached observations and native generated dispatch reproduce exactly. Matching whole models remain **155 (+0)**, ordinary bindings **50/79 (+0)**, bounded obligations **7/9**, families **0/34** and gates **0/5**. No release percentage is established.

**Prior plain-Type batch-boundary assessment:** the one optimized sixteen-resource Performances assessment parses all resources and clears the missing Behavior provider. Construction/linking now stops at **parameter supertype category requires normative assessment**. This first-blocker observation earns no linked-document or semantic-validation credit; the candidate remains unpromoted.

**Completed bounded mixed-parameter batch:** four resolved typed method trees and thirty runtime strategy bindings now drive the owned-parameter predicate and shared native collection consumers. All **1,164/1,164 cached Pilot stage controls** match owned/all collections, result lookup and ordered redefinitions in original/reversed storage and serialized reload; a parsed Function-specializes-Class source verifies both declaration orders without mutating completion flags. The blanket general-category rejection is replaced by the assessed generic collection rules. Missing/deferred inputs, added-member, chaining/conjugation and specialized invocation/constructor dependencies remain guarded. [Parameter evidence](mixed-parameter-evidence.json) delineates imported, consumed, verified and handwritten behavior.

The normative parameter/result constraints require a narrower set of redefinitions than Pilot's generic effective-feature and recursive result algorithms. This difference is explicitly open for assessment; these factory observations earn no whole-model validation credit. Fresh checks: **209 definition tests (+2), five generation drift controls and exact Pilot cache reproduction pass**. Whole models remain **155 (+0)**, ordinary bindings **50/79 (+0)**, bounded obligations **7/9**, families **0/34**, release gates **0/5**. The new optimized assessment parsed all sixteen resources but was stopped after the announced thirty-minute CPU budget in construction/linking, without a linked model or semantic diagnostic. It earns no actual-resource support or timing-gate credit; it does not establish that the prior blocker cleared in the real environment.

**Immediate bounded prerequisite — actual Performances closure and inherited literal results:** both Multiplicity selector bindings now consume the resolved owner-sensitive program, and the real Base/ScalarValues source path links reference-valued bounds and replays result/default contributions. Next compose inherited literal result queries with the actual Performances defaults/resource environment. The sixteen-resource closure remains unqualified: its latest frozen assessment demonstrates automatic additional-member scheduling and retry, then stops at directed parameter defaults. The shared fixed-parameter strategy now has focused query/contribution/replay proof, but complete source-set linking and validation remain open. Treat selection, contribution, result typing, featuring and validation as separate obligations; no stand-in result objects or completion flags.

**Next fixed semantic batch:** qualify expression results against the actual required library/resource environment. Resolved literal default programs name `Performances::literalIntegerEvaluations`; the three-library cohort omits Performances and its imports. Establish that real closure first, then compose existing default/result algorithms in MultiplicityRange and FeatureValue contexts with focused stage observations. Do not manufacture stand-ins, result objects or completion flags to reduce the cohort counter. Nonconstant evaluability and validation remain separate dependencies. [Prior import/multiplicity evidence](import-multiplicity-projection-evidence.json) retains the projection boundaries.

**Next ordered dependency batches in that closure:**

| Order | Shared dependency | Unverified declaration instances | Completion boundary |
|---|---|---:|---|
| Done (bounded cohort) | Annotation/Documentation targets and chained ownership | 0 (46 resolved) | Imported ownership/reference contracts drive shared projections; source, persistence, malformed-input and normative/Pilot disagreement controls. Annotation/chaining invariants remain separate. |
| Done (bounded cohort) | Imported-element projection | 0 (3 resolved) | NamespaceImport/MembershipImport compose canonical targets through the supplied resource closure; missing/external/private dependencies remain explicit. |
| Done (constant branches) | Literal/null/metadata-access evaluability | 0 (28 resolved in cohort) | Three native operation bodies; six other branch algorithms and actual evaluation remain open. |
| Done (bounded empty branch) | Owner-specific metadata selection for defaults | Nine factory controls; broader metadata scope open | Imported annotation delegates and canonical membership drive six consumers. Actual metadata-derived bases, owning-endpoint fallback control and pending scheduling remain open. |
| Done (ordinary matrix) | Expression-family computed redefinitions | 120 factory controls across 20 bindings | Detached/Package/Class ordinary contexts are verified; transition guards, FeatureValue owning-Type contexts and positional algorithms remain open. |
| Done (bounded default stage) | Expression defaults in ordinary owning-Type contexts | 196/196 stage controls match | Conditional lists and public queries are verified through the shared exact-Step type query. Directed/end/positional and complete lifecycle behavior remain open. |
| Done (bounded exact Step strategy) | Step default/general-type provider and inherited Feature typing | 56 default-stage and 6 typing controls; 14 expression contexts newly match | Generated exclusive selector plus canonical payload/owner/type prerequisites, source and replay. Overridden adapters, directed/end/positional behavior, pending states and full lifecycle remain open. |
| Done (bounded shared query stage) | Resolved non-Feature plain-Type providers | 14 bindings, 98 controls; 24 overriding bindings excluded from this strategy | Generated shared admission/names, independent queries, source/replay/persistence. Full Function/conjugated-port lifecycle, actual resource dependencies and validation remain open. |
| Done (bounded shared query stage) | Positional parameter collections and mixed general-Type categories | 30 bindings; 1,164/1,164 stage controls match | Generated owner predicate plus shared ordered collections/results/redefinitions and parsed-source replay. Physical implicit relationships, added-member/chaining/conjugation, specialized invocation/constructor services and full normative validation remain open. |
| 1b | Expression results and actual resource closure | 28 (prior small cohort) | Verify actual default/result inheritance or construction after the library set links; no stand-ins or completion flags. |
| Done (identity projections) | Multiplicity bound and upper-bound derivation | 0 (34 resolved) | Owned expression identities derive from canonical ordered memberships. Numeric evaluation, result typing and multiplicity invariants remain open. |
| 5 | Complete contribution and validation lifecycle, then candidate census | Additional release scope remains unmeasured | Close reviewed integration obligations only after all required adapters/constraints execute. Full sample semantics and timing remain final gates. |

### Execution rules

- Keep one native implementation stream and one independent evidence/control stream with the same fixed batch contract. Separate tool-only inventory work is safe in parallel; avoid concurrent edits to the semantic core.
- Reuse pinned observations. Re-export a complete strategy's controls only when its inputs, exporter, schema or toolchain change, or genuinely new cases are needed.
- Run affected focused tests per edit and one regression pass at the batch boundary. Run the small real-library closure once after the connected batch, not after every guard adjustment. Full sample comparison and timings remain milestone/final work.
- Rank work by shared dependencies and source sets unlocked, using the census when available. Do not invent a speedup or completion estimate from test/rule counts.
- Use the existing certificate validators. Map evidence once into reviewed contracts; do not repeatedly expand bookkeeping instead of completing behavior. No whole-family requirement is weakened by a bounded strategy certificate.
- Keep the eleven-validator Xtend trial bounded. Extend translation only for a named high-value dependency after comparing correctness, effort and maintenance with handwritten Rust. No general Xtend compiler on this release's critical path.

Prior implementation details are in [remaining-gap-history.md](remaining-gap-history.md); the precise crossing boundaries and prior passing logs are in [normative-binary-crossing.json](normative-binary-crossing.json). The candidate remains unpromoted.

### Current family qualification attempt — 2026-10-03

The selected whole family is **xtext.EnumLiteralDeclaration**, with **24 declarations, 14 enum rules and 18 direct call sites**. Its first complete obligation inventory is now reviewed in the existing qualification contract: imported identities/fields, native callers, enum-rule semantics, and integrated candidate publication/validation/persistence. The acceptance scope includes every required caller context; no family certificate is issued. [Review](enum-family-review.md), [inventory](enum-family-inventory.json) and [evidence](enum-family-evidence.json) distinguish passing bounded observations from open integration requirements.

**72/72 independent Pilot selected-rule controls match** (positive, quoted rejection and keyword/identifier boundary for every declaration). **34/34 raw caller model values match**, covering all eighteen direct enum assignments, including secondary import visibility and portion/control-node carriers. Native canonical construction, closed structural publication, reload and invalid enum-value rejection pass for **22/24 primary carriers**. Secondary-carrier publication and complete model constraints remain unqualified. The written pinned specifications supply the literal meanings; Ecore integer values are representation evidence. No production algorithm changed in this audit; the generic candidate already consumes all mappings.

**Two real integrated blockers remain required:** TriggerFeatureKind passes the corrected ReferenceUsage transition-link identity dispatch and now requires **TransitionUsage generalization/contribution/added-member lifecycle**; ExposeVisibilityKind reaches the missing **ViewUsage generalization provider**. Both have explicit failing-source controls. The checker also still requires its complete native_parser/model_construction dependency contracts; those have not been narrowed, removed or marked complete. The three named integrated qualification tests remain pending, so passing failure-boundary tests cannot close the fourth obligation.

Fresh checks: **244 definition tests pass (+3)**; five observation guards, 33 certificate and fourteen plan tests pass. Reviewed active qualification targets are **1/46 (+1)**, with four reviewed family obligations and zero closed family obligations. This is improved measurement, not increased full support. Release metrics remain whole models **155 (+0)**; ordinary Feature bindings **50/79 (+0)**; bounded strategy obligations **7/9 (+0)**; whole families **0/34 (+0)**; gates **0/5 (+0)**. Candidate remains unpromoted.

**Next family-directed implementation batch:** implement the full TransitionUsage/ViewUsage contribution and added-member lifecycle with actual resources with actual semantic dependencies; reuse the cached enum controls to finish integrated caller publication, validation and persistence. Keep the actual-library closure work linked to these same shared query services. No stand-in targets, substituted failure controls or reduced qualification criteria.

**Latest shared multiplicity batch:** resolved admission and member-construction definitions now drive one native producer across **45 concrete Usage bindings**. **4,050/4,050 Pilot admission/construction/replay controls**, serialization/reversed storage, rollback, and **seven generator checks** pass. The candidate linker consumes this producer through its typed additional-member dependency; the document API also exposes an atomic batch. **249 definition tests (+2)** pass at the batch boundary; one additional focused native source/persistence test passes. The created element is bare `Multiplicity`: bounds, featuring, full validation and complete transformation remain required. The source path `end item i; end item j[2];` creates one missing bare multiplicity, preserves the declared range, and replays unchanged after serialization/reversed storage. TransitionUsage's custom lifecycle and SuccessionAsUsage's separate no-op dispatch are explicitly excluded from this producer. [Evidence](multiplicity-construction-evidence.json). Whole families **0/34 (+0)**; matching whole models **155 (+0)**; ordinary Feature queries **69/79 (+0)**; gates **0/5 (+0)**. No certificate or candidate promotion.

**Latest shared Action batch:** resolved ActionUsage and AcceptActionUsage policy now generates membership/default-map/lifecycle selection for **22 bindings**. **198/198 Pilot membership selectors** and **114/114 ordinary redefinition/naming queries** match; nineteen additional ordinary bindings are verified, including persistence and reordered storage. **247 definition tests (+3)** and **eight generator tests** pass. Ordinary Feature query coverage is now **69/79 (+19)**; this does not qualify positional/constructor contexts or complete Action transformation. [Evidence](action-policy-evidence.json) and [normative constraints](action-policy-normative.json).

**Current family result: xtext.EnumLiteralDeclaration remains open.** The trigger source now passes ReferenceUsage and Accept trigger selection, then requires the full **TransitionUsage generalization/contribution/added-member lifecycle**. Expose still requires the **ViewUsage lifecycle**. Whole families **0/34 (+0)**; matching whole models **155 (+0)**; primary canonical enum carriers **22/24 (+0)**; gates **0/5 (+0)**. No family certificate is issued; candidate remains unpromoted.

**Ordered family closure path:** finish the enum family's shared TransitionUsage/ViewUsage lifecycle and actual library prerequisites; verify all required direct caller contexts; close the strict native_parser/model_construction dependency contracts when their complete acceptance is met; qualify EnumLiteralDeclaration, then EnumRule and Ecore packages/enums using their own complete reviewed inventories. Continue through the remaining families recorded in [qualification results](family-qualification-results.json). Their existing acceptance stays intact; selector import and bounded passing controls never close a family. The native_parser/model_construction dependencies span multiple grammar/model categories, so family-directed progress can precede a certificate without increasing the whole-family count.

## 2026-10-04 expression-context milestone: previous critical-path narrative

# 2026-08 native-support critical path

**Fully qualified families: 0/34. Release gates: 0/5.** These are certificate counts, not an estimate of implemented behavior. This file is the single completion plan. The [structured coverage matrix](structural-source-coverage.json) is its detailed feature/evidence appendix; [family results](family-qualification-results.json) retain each family and its open dependencies. [Earlier narrative](remaining-gap-history.md) is historical.

The pinned written specification is normative; Pilot is an independent behavioral reference. Import, native consumption and semantic verification are separate accomplishments. Keep production native, permit upstream Java only at build time, and preserve the unpromoted candidate. No exclusions, relaxed acceptance or narrower global contracts are approved.

## Verified milestone

The [contribution batch](instantiation-contribution-evidence.json) now connects the two resolved operator/trigger adapter programs to the shared native default/generalization producer and physical materializer. Across all six concrete bindings, typed effects preserve **both relationship kind and target**, imported classifier-group ordering, self suppression, and canonical identity/ordered-chain duplicate suppression. This extends bounded default-recipe coverage from **2/8 to 8/8 instantiation classes**, alongside the already verified 8/8 getters. Dispatch is generated from the resolved definitions; effect composition, canonical chaining/navigation and graph edits are explicitly handwritten Rust.

The fresh reference batch contains **228 independent contribution cases and eight equivalence cases**. Native checks compare 912 contribution contexts and 32 equivalence contexts, plus 80 physical-construction/idempotence/abstract-syntax persistence contexts. They include a TriggerInvocationExpression targeting a Feature, where FeatureTyping and Subsetting must remain distinct effects to the same endpoint. Typed chain prerequisites retry after their exact field changes; malformed endpoints reject. No completion flags are written. Detached/package contexts and supplied completed library targets are bounded controls, not full inherited transformation, valid callable models or cold-library qualification.

The contribution batch passed 360 native definition tests. Its unchanged 24-library-resource plus caller assessment parsed all 25 inputs, then rejected positional parameters at an unadmitted adapter boundary after 579.109 seconds. This is a diagnostic duration, not a Pilot benchmark or resource qualification. The [assessment](enum-trigger-states-instantiation-contribution-assessment.json) is tied to that optimized executable.

The next [expression positional batch](expression-parameter-evidence.json) imports the resolved `ExpressionAdapter#getGeneralTypes` override and admits its ordinary delegation branch across 20 concrete bindings. Native checks match 960 independent query controls in 3,840 original/persisted/reversed contexts; guard contexts still reject. Shared positional traversal remains handwritten Rust. The full 361-test native regression, optimized build and 11 audit/pipeline checks pass. The unchanged-resource outcome is recorded in [the prior assessment](enum-trigger-states-expression-parameter-assessment.json); full resource semantic qualification remains open. The [normative review](library-instantiation-normative-review.json) keeps invalid-domain library/null-kind controls distinct from full semantic qualification. **No family or release gate closes solely from this milestone.**

The shared [expression-context contribution milestone](expression-context-contribution-evidence.json) consumes valid directions across20 bindings and all six bounded owner contexts. Generated roles/selectors drive native effects; canonical projection and graph edits remain handwritten. It passed364 native tests,11 audit/pipeline checks and the focused independent matrix, physical persistence, rejection and retry controls. Full inherited lifecycle, normative validation and cold resources remain open.

## Ordered implementation and qualification

1. **Complete the inherited lifecycle and actual enum resource pipeline.** The [expression-context batch](expression-context-contribution-evidence.json) now admits Ecore-valid directions and matches960 cached controls in3,840 original/persisted/reversed contexts, plus120 physical/persistence,720 rejection and20 typed retry contexts. All364 native definition tests and11 audit/pipeline checks pass. Completed/end, receiver-owned valuation/crossing, metadata, conjugation and full callable validation remain explicit dependencies. The [normative review](expression-context-contribution-normative-review.json) documents the pinned PDF/Pilot library-role naming disagreement. Assess the unchanged required resource set once at this boundary; prioritize its exact shared blocker. No family closes from component evidence alone.
2. **Qualify the complete inherited contribution contexts.** The shared typed recipe batch is implemented and bounded tests pass across all six operator/trigger bindings. Extend independent controls and native proofs to owning/value/multiplicity/direction/end/metadata/conjugation contexts and required inherited computed contributions. Preserve typed prerequisites and transaction boundaries. Do not treat the bounded recipe as a full `computeImplicitGeneralTypes` lifecycle.
3. **Finish the global parser/model contracts and remaining definition consumers.** Use the structured matrix to finish every pinned Xtext construct and Ecore structure/semantics. Import/resolution alone never closes an obligation. Where definitions supply no algorithm, name the handwritten Rust dependency, implement it, and verify it independently. Keep MMF expansion, alternate persistence backends and a general OCL/Xtend compiler outside this release's critical path.
4. **Close families against the strict checker.** Start with `xtext.EnumLiteralDeclaration`, `xtext.EnumRule` and `ecore.packages_and_enums`; proceed family by family. A family closes only when all required contexts, global dependency contracts and independent controls have fresh evidence and an accepted certificate. Preserve the existing bounded obligations and certificate requirements; do not turn a larger binding count into closure.
5. **Run final release qualification.** Process every required sample through native parsing, construction, linking, validation, publication and round-trip persistence; perform the detailed Pilot semantic comparison and a comparable timing assessment. Document normative disagreements and resolve required behavior. Only then close all 34 families and five release gates.

## Coverage and unsupported behavior

| Category | Imported | Native consumption | Verification / still unsupported |
| --- | --- | --- | --- |
| Instantiation getter dispatch, common membership selector | Resolved eight bindings and common operation | All eight getters; canonical Ecore reads | Bounded common and library controls pass; callable validity and full lifecycle remain open |
| Operator/trigger library names, enum roles and constructor defaults | 25 resolved trees, ordered packages, three roles | Generated recipes plus shared typed canonical lookup | Six bindings match cached observations; actual global resources and invalid-domain normative validation remain open |
| Instantiation default/generalization contributions | Common resolved recipes, two adapter programs and four insertion/equivalence helpers | Typed composition and physical construction across all eight classes | Detached/package and mixed relationship effects verified; complete contexts and inherited lifecycle remain open |
| Ordinary expression positional parameters/results | Resolved general-type override and 20 reflected bindings | Generated non-guard admission plus shared canonical positional traversal | 960 independent query controls match in 3,840 contexts; transition guards, full callable validation and resource publication remain open |
| Ecore inheritance, types, multiplicities, containment, opposites, ordering, uniqueness, defaults and flags | Preserved definitions and contracts | Shared consumers remain partial | Required full-context and semantic-dependency evidence remains open in the matrix |
| Ecore operations, annotations and delegate bindings | Preserved and partially resolved executable definitions | Bounded selected programs plus named Rust dependencies | Recording a signature/delegate is not support; remaining algorithms and validators remain open |
| Xtext grammars, inheritance, terminals, alternatives, groups, cardinality, assignments, actions and predicates | Pinned structures and resolved rules | Shared native execution/construction remains partial | Complete recursive grammar decisions, token arbitration, contexts and global contracts remain open |
| Xtext cross-references and external scoping/linking | Imported reference/type contracts | Native canonical services remain partial | Complete inherited/global scope, deferred producer lifecycle and publication remain open |
| Xtext/Ecore enumeration families | Imported literals, rules, metamodel packages/enums | Bounded controls and candidate consumers | Required whole resources, secondary fields, constraints, persistence and global contracts prevent closure |
| Samples, semantic comparison and timing | Required manifests and historical observations | Opt-in native candidate pipeline | Final all-sample qualification and comparable Pilot timing remain open |

Use cached independent observations with pinned input/toolchain/schema provenance. Run focused tests while editing and broader checks at coherent batch boundaries. Evaluate Xtend translation only for bounded high-value semantic rules; choose generation, interpretation or explicit handwritten Rust from evidence. Report each run's new qualified obligations/families and the precise remaining dependency. Do not claim full support before all required behavior and samples are verified.

