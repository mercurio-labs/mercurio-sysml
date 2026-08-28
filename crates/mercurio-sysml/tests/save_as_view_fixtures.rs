//! Save-as-View — the mapping contract (SV-0) and the view constructs (SV-1).
//!
//! Plan: `docs/save-as-view-plan.md` (visualization-plan V-6, gate opened
//! 2026-08-27).
//!
//! SV-0 froze the `mercurio.view.v1` <-> SysML v2 view map as a fixture corpus
//! and characterised what the compiler did with every construct that map
//! depends on. Those characterisation tests recorded four defects; SV-1 fixed
//! all four, so each one is now an assertion of the corrected behaviour. The
//! discipline holds going forward: when an increment changes a construct,
//! update its test and un-ignore its `sv*_` counterpart in the same commit.
//!
//! What SV-1 fixed, and why each mattered:
//!
//! - `expose X::**` did not merely lack support — it **silently mis-parsed**
//!   into a usage named after the first path segment, dropping the wildcard
//!   with no diagnostic. Because localized writeback falls back to
//!   `canonical_rewrite`, which re-renders declarations from the parsed AST,
//!   that made writing `expose` before compiling it a file-corruption hazard.
//!   It now parses as a namespace query, sharing the `import` path: the two are
//!   parallel metamodel branches (`MembershipExpose`/`NamespaceExpose` beside
//!   `MembershipImport`/`NamespaceImport`).
//! - The `[@Meta]` predicate was dropped on **both** paths, mis-read as a
//!   multiplicity range and discarded. On the import side that was a standing
//!   correctness bug beyond this plan: the pilot's own
//!   `40. Filtering/Filtering Example-2.sysml:28` writes
//!   `public import vehicle::**[@Safety];`, which Mercurio resolved as
//!   `vehicle::**` — every part, not just the safety-annotated ones.
//! - A `view def`'s `filter` member was **dropped entirely**, leaving an empty
//!   body: `block_starts_with_declaration` did not recognise `filter @X;` as a
//!   declaration, so it was consumed as an opaque statement.
//! - Consequently the pilot's safety / non-safety view pair was
//!   indistinguishable. They now diverge at parse; SV-2 owns the remaining half
//!   — that they resolve to disjoint *element sets*.

use std::path::{Path, PathBuf};

use mercurio_foundation::language_contracts::ast::Declaration;
use mercurio_foundation::model::{Graph, NodeId};
use mercurio_foundation::views::{
    ModelViewKindDto, ModelViewSpecDto, ViewDocumentDto, ViewUsageDraft, exposed_elements,
    resolve_exposed_elements, usage_from_view_spec, view_spec_from_usage,
};
use mercurio_sysml::{
    KirDocument, SysmlModule, compile_sysml_text, load_sysml_baseline, parse_sysml,
};

const FIXTURE_DIR: &str = "tests/fixtures/save-as-view";

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_DIR)
}

fn read_fixture(relative: &str) -> String {
    let path = fixture_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read fixture {}: {err}", path.display()))
}

fn parse_fixture(relative: &str) -> SysmlModule {
    let text = read_fixture(relative);
    parse_sysml(&text)
        .unwrap_or_else(|err| panic!("fixture {relative} failed to parse: {}", err.message))
}

/// Flatten a parsed module into one line per declaration, mirroring the shape
/// `inspect_sysml_parse` prints. Keeping the projection textual makes the
/// characterisation assertions readable in a diff.
fn outline(module: &SysmlModule) -> Vec<String> {
    let mut lines = Vec::new();
    for member in &module.members {
        walk(member, 0, &mut lines);
    }
    lines
}

fn walk(declaration: &Declaration, depth: usize, lines: &mut Vec<String>) {
    let pad = "  ".repeat(depth);
    match declaration {
        Declaration::Package(package) => {
            lines.push(format!("{pad}package {}", package.name.as_dot_string()));
        }
        Declaration::Import(import) => {
            let keyword = if import.is_expose { "expose" } else { "import" };
            let filter = import
                .filter
                .as_ref()
                .map(|condition| format!("[{condition}]"))
                .unwrap_or_default();
            lines.push(format!(
                "{pad}{keyword} {}{filter}",
                import.path.as_colon_string()
            ));
        }
        Declaration::Alias(alias) => {
            lines.push(format!("{pad}alias {}", alias.name));
        }
        _ => {
            if let Some(definition) = declaration.as_definition_like() {
                lines.push(format!(
                    "{pad}{} def {}",
                    definition.keyword, definition.name
                ));
            } else if let Some(usage) = declaration.as_usage_like() {
                // An element filter carries a predicate rather than a name.
                match usage.metadata_properties.get("condition") {
                    Some(condition) => lines.push(format!("{pad}{} {condition}", usage.keyword)),
                    None => lines.push(format!("{pad}{} {}", usage.keyword, usage.name)),
                }
            }
        }
    }
    for child in declaration.child_declarations() {
        walk(child, depth + 1, lines);
    }
}

/// Every declaration line anywhere in the module, trimmed of indentation.
fn flat(module: &SysmlModule) -> Vec<String> {
    outline(module)
        .into_iter()
        .map(|line| line.trim().to_string())
        .collect()
}

// ---------------------------------------------------------------- corpus

/// Every `.sysml` fixture in the corpus parses. This is the floor: the map
/// cannot be frozen against source the compiler rejects outright.
#[test]
fn every_fixture_parses() {
    for relative in [
        "tier1/tree-diagram-subtree.sysml",
        "tier1/interconnection-diagram.sysml",
        "tier1/metadata-filtered-expose.sysml",
        "tier1/import-expose-parity.sysml",
        "tier1/explicit-exposes.sysml",
        "tier1/element-table-columns.sysml",
        "tier1/view-def-metaclass-filter.sysml",
        "tier2/mercurio-rendering-params.sysml",
        "tier2/relationship-matrix-subviews.sysml",
        "tier3/free-text-search.sysml",
    ] {
        let _ = parse_fixture(relative);
    }
}

/// The manifest is the contract. Every fixture it lists must exist, and every
/// declared spec file must exist too — so a fixture can never silently lose its
/// expected mapping.
#[test]
fn manifest_matches_the_corpus_on_disk() {
    let manifest: serde_json::Value =
        serde_json::from_str(&read_fixture("manifest.json")).expect("manifest.json is valid JSON");

    let fixtures = manifest["fixtures"]
        .as_array()
        .expect("manifest has a fixtures array");
    assert!(!fixtures.is_empty(), "the corpus must not be empty");

    for fixture in fixtures {
        let id = fixture["id"].as_str().expect("fixture has an id");
        let sysml = fixture["sysml"].as_str().expect("fixture names a .sysml");
        assert!(
            fixture_root().join(sysml).is_file(),
            "fixture `{id}` names a missing source file: {sysml}"
        );

        let tier = fixture["tier"].as_u64().expect("fixture declares a tier");
        assert!(
            (1..=3).contains(&tier),
            "fixture `{id}` has an unknown tier"
        );

        match fixture["spec"].as_str() {
            Some(spec) => assert!(
                fixture_root().join(spec).is_file(),
                "fixture `{id}` names a missing spec file: {spec}"
            ),
            None => assert!(
                tier == 3 || fixture["note"].is_string(),
                "fixture `{id}` has no spec and no note explaining why"
            ),
        }
    }
}

// ------------------------------------------------------- SV-1(b): rule-1 debt

/// SV-1(b). CLAUDE.md rule 1: "KIR `kind` values map 1:1 to SysML v2/KerML
/// metaclass names — no proprietary kinds."
///
/// The three view *definitions* already satisfied it. The three view *usages*
/// were collapsed into `KerML::Core::Feature` — 3 of the 58 (of 101) metaclasses
/// collapsed that way — which mattered here because a saved view IS a usage.
/// This test pins the promotion.
///
/// Only `kir_kind` moved; `id_template` is deliberately unchanged, because
/// element ids are semantic identity anchors and renaming them is a separate
/// and much larger change than repaying the kind debt.
#[test]
fn sv1b_view_usages_emit_one_to_one_kir_kinds() {
    const SOURCE: &str = r#"
package ViewKinds {
    view v;
    viewpoint vp;
    rendering r;
}
"#;

    let document = compile(SOURCE, "view-kinds.sysml");

    for (declared_name, expected_kind) in [
        ("v", "SysML::ViewUsage"),
        ("vp", "SysML::ViewpointUsage"),
        ("r", "SysML::RenderingUsage"),
    ] {
        let kind = kind_of(&document, declared_name).unwrap_or_else(|| {
            panic!("no element named `{declared_name}` in the compiled document")
        });
        assert_eq!(
            kind, expected_kind,
            "`{declared_name}` must carry its own metaclass name, not the \
             collapsed KerML::Core::Feature"
        );
    }
}

/// The two names this increment deletes do not exist in the SysML metamodel.
/// `SysML.ecore` has zero occurrences of `RenderUsage` and `FilterUsage`; the
/// real names for what `render` and `filter` produce are
/// `ViewRenderingMembership` and `ElementFilterMembership`. Nothing may
/// reintroduce them.
#[test]
fn sv1b_no_fabricated_view_metaclass_is_emitted() {
    const SOURCE: &str = r#"
package Fabricated {
    view v {
        render asTreeDiagram;
    }
}
"#;

    let document = compile(SOURCE, "fabricated.sysml");
    for element in &document.elements {
        assert!(
            !element.kind.contains("RenderUsage") && !element.kind.contains("FilterUsage"),
            "`{}` is not a SysML metaclass — see SysML.ecore",
            element.kind
        );
    }
}

// -------------------------------------------------- SV-1(a): the constructs

/// `expose <path>::**` now parses as a namespace query and keeps its wildcard.
///
/// Before SV-1 it mis-parsed into a usage declaration named after the first
/// path segment, dropping the wildcard with no diagnostic. That silent
/// mis-parse is what made writing `expose` before compiling it a
/// file-corruption hazard, since localized writeback falls back to
/// `canonical_rewrite`, which re-renders declarations from the parsed AST.
#[test]
fn sv1_expose_parses_as_a_namespace_query() {
    let module = parse_fixture("tier1/tree-diagram-subtree.sysml");
    let lines = flat(&module);

    assert!(
        lines.iter().any(|line| line == "expose vehicle::**"),
        "expose must parse as a namespace query carrying its wildcard; got {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line == "expose vehicle"),
        "the old wildcard-dropping mis-parse must not reappear; got {lines:?}"
    );
}

/// The filter predicate survives on BOTH namespace-query paths.
///
/// This half is a fix to shipped behaviour beyond the save-as-view plan: the
/// pilot's own `40. Filtering/Filtering Example-2.sysml:28` writes
/// `public import vehicle::**[@Safety];`, and before SV-1 Mercurio resolved
/// that as `vehicle::**` — every part, not just the safety-annotated ones.
#[test]
fn sv1_import_and_expose_both_retain_the_filter_predicate() {
    let module = parse_fixture("tier1/import-expose-parity.sysml");
    let lines = flat(&module);

    assert!(
        lines
            .iter()
            .any(|line| line == "import vehicle::**[@Safety]"),
        "import must retain its filter predicate; got {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "expose vehicle::**[@Safety]"),
        "expose must retain its filter predicate; got {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "expose vehicle::**"),
        "an unfiltered expose must stay unfiltered; got {lines:?}"
    );
}

/// A `view def`'s `filter` member survives, and the interoperable
/// `filter @<Metaclass>` form is captured structurally rather than as text —
/// that is the shape `TableRowTypeDto` maps onto.
#[test]
fn sv1_view_def_retains_its_filter_member() {
    let module = parse_fixture("tier1/view-def-metaclass-filter.sysml");

    let view_def =
        find_definition(&module, "view", "Part Structure View").expect("the view def parses");
    assert_eq!(
        view_def,
        vec!["filter @SysML::PartUsage"],
        "`filter @SysML::PartUsage;` must survive into the view def's body"
    );

    let filter = find_usage(&module, "filter").expect("the filter declaration is a usage");
    assert_eq!(
        filter
            .reference_target
            .as_ref()
            .map(|name| name.as_colon_string()),
        Some("SysML::PartUsage".to_string()),
        "the metaclass predicate must be captured structurally, not only as text"
    );
}

/// The discriminating case, now positive. The pilot's safety / non-safety view
/// pair must be distinguishable — this is the parse-level half of it.
#[test]
fn sv1_safety_and_non_safety_views_diverge_at_parse() {
    let module = parse_fixture("tier1/metadata-filtered-expose.sysml");

    let safety = find_usage_members(&module, "view", "safety features view")
        .expect("the safety view parses");
    let non_safety = find_usage_members(&module, "view", "non-safety features view")
        .expect("the non-safety view parses");

    assert_ne!(
        safety, non_safety,
        "a view exposing [@Safety] and one exposing [not (@Safety)] must not \
         parse identically"
    );
    assert_eq!(
        safety,
        vec!["expose vehicle::**[@Safety]", "render asTreeDiagram"]
    );
    assert_eq!(
        non_safety,
        vec!["expose vehicle::**[not (@Safety)]", "render asTreeDiagram"]
    );
}

/// Complex conditions survive verbatim, including the metadata-cast form the
/// pilot uses. The parser's job is to stop losing the condition; evaluating it
/// is SV-2's.
#[test]
fn sv1_complex_filter_conditions_survive_verbatim() {
    let module = parse_sysml(
        r#"
package Complex {
    metadata def Safety { attribute isMandatory : ScalarValues::Boolean; }
    part vehicle { part brake { @Safety { isMandatory = true; } } }
    package Mandatory {
        public import vehicle::**[@Safety and (as Safety).isMandatory];
    }
}
"#,
    )
    .expect("complex filter conditions parse");

    assert!(
        flat(&module)
            .iter()
            .any(|line| line == "import vehicle::**[@Safety and (as Safety).isMandatory]"),
        "got {:?}",
        flat(&module)
    );
}

/// `expose` lowers to its own KIR kind, carrying the filter condition.
///
/// It must not share `SysML::Import`: a saved view's scope is an Expose, and
/// the whole point of reifying views is that the distinction survives into the
/// model. `MembershipExpose` and `NamespaceExpose` collapse onto the abstract
/// `SysML::Expose` exactly as their Import twins collapse onto `SysML::Import`.
#[test]
fn sv1_expose_lowers_to_its_own_kir_kind_with_the_filter() {
    const SOURCE: &str = r#"
package Scoped {
    metadata def Safety;
    part vehicle { part brake { @Safety; } }
    view v {
        expose vehicle::**[@Safety];
    }
    public import vehicle::**;
}
"#;

    let document = compile(SOURCE, "scoped.sysml");

    let exposes: Vec<_> = document
        .elements
        .iter()
        .filter(|element| element.kind == "SysML::Expose")
        .collect();
    assert_eq!(
        exposes.len(),
        1,
        "expected exactly one Expose element; kinds were {:?}",
        document
            .elements
            .iter()
            .map(|element| element.kind.as_str())
            .collect::<Vec<_>>()
    );

    assert_eq!(
        exposes[0].properties.get("filter").and_then(|v| v.as_str()),
        Some("@Safety"),
        "the expose must carry its filter condition into KIR"
    );

    let imports = document
        .elements
        .iter()
        .filter(|element| element.kind == "SysML::Import")
        .count();
    assert_eq!(imports, 1, "the plain import must still be an Import");
}

/// An unfiltered namespace query emits no `filter` property at all, rather than
/// an empty string — `insert_rendered_property` drops nulls.
#[test]
fn sv1_unfiltered_expose_emits_no_filter_property() {
    const SOURCE: &str = r#"
package Plain {
    part vehicle;
    view v {
        expose vehicle::**;
    }
}
"#;

    let document = compile(SOURCE, "plain.sysml");
    let expose = document
        .elements
        .iter()
        .find(|element| element.kind == "SysML::Expose")
        .expect("an Expose element is emitted");
    assert!(
        !expose.properties.contains_key("filter"),
        "an unfiltered expose must not carry a filter property; got {:?}",
        expose.properties
    );
}

/// A `doc` member documents its **owner**, not whichever member happens to
/// follow it. It was being treated as a prefix annotation, so the fixture's
///
/// ```text
/// view 'vehicle structure view' {
///     doc /* ... */
///     expose vehicle::**;
/// ```
///
/// attached the view's own documentation to its `expose` -- leaving V-6.3 with
/// no `description` to map. Bare `/* ... */` comments are unaffected: those are
/// trivia and never become Documentation elements.
#[test]
fn sv3_a_doc_member_documents_its_owner() {
    let document = compile(
        r#"
package P {
    part vehicle { part brake; }
    view v {
        doc /* What this view shows. */
        expose vehicle::**;
    }
    part def A {
        doc /* What A is. */
        attribute q;
    }
}
"#,
        "doc-ownership.sysml",
    );

    let owner_of = |body: &str| -> Option<String> {
        document
            .elements
            .iter()
            .find(|element| {
                element.kind.contains("Documentation")
                    && element.properties.get("body").and_then(|v| v.as_str()) == Some(body)
            })
            .and_then(|element| {
                element
                    .properties
                    .get("owner")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
    };

    assert_eq!(owner_of("What this view shows."), Some("view.P.v".to_string()));
    assert_eq!(owner_of("What A is."), Some("type.P.A".to_string()));
}

/// The counterpart inconsistency this increment did **not** resolve, recorded
/// so it is not mistaken for an oversight: at *package* level a prefix `doc`
/// still documents the declaration that follows it.
///
/// SysML v2 says otherwise -- `doc` is an owned Documentation of its namespace
/// -- but Mercurio has a shipped convention in the other direction here, and
/// `set_documentation` depends on it: it replaces a prefix `doc` in place, and
/// reversing the ownership makes it append a second one instead. That is an
/// authoring-convention decision, not a parser fix.
#[test]
fn sv3_at_package_level_a_prefix_doc_still_annotates_the_next_declaration() {
    let document = compile(
        "package P {
    doc /* About the part, by current convention. */
    part v;
}",
        "package-doc.sysml",
    );

    let owner = document
        .elements
        .iter()
        .find(|element| element.kind.contains("Documentation"))
        .and_then(|element| element.properties.get("owner").and_then(|v| v.as_str()))
        .map(str::to_string);

    assert_eq!(
        owner,
        Some("feature.P.v".to_string()),
        "change this deliberately, together with set_documentation, not by accident"
    );
}

// ------------------------------------------------------- targets (ignored)

/// SV-2 exit criterion. Parse-level divergence is necessary but not
/// sufficient: the two views must resolve to disjoint *element sets*.
#[test]
fn sv2_safety_and_non_safety_views_resolve_to_disjoint_sets() {
    let document = compile(
        &read_fixture("tier1/metadata-filtered-expose.sysml"),
        "metadata-filtered-expose.sysml",
    );
    let graph = Graph::from_document(document).expect("the fixture builds a graph");

    let safety = exposed(&graph, "safety features view");
    let non_safety = exposed(&graph, "non-safety features view");

    // `brake` carries @Safety; `radio` does not. Both views expose the same
    // `vehicle::**` scope, so only the predicate can tell them apart -- which
    // is the whole point of the increment.
    assert_eq!(
        safety,
        vec!["feature.SafetyViews.vehicle.brake".to_string()]
    );
    assert_eq!(
        non_safety,
        vec!["feature.SafetyViews.vehicle.radio".to_string()]
    );
    assert!(
        safety.iter().all(|id| !non_safety.contains(id)),
        "the two views must resolve to disjoint sets; got {safety:?} and {non_safety:?}"
    );
}

/// The same revision must resolve to the same ordered answer every time --
/// callers cache renders by artifact key, so a set that reorders would look
/// like a model change.
#[test]
fn sv2_resolution_is_deterministic() {
    let document = compile(
        &read_fixture("tier1/tree-diagram-subtree.sysml"),
        "tree-diagram-subtree.sysml",
    );
    let graph = Graph::from_document(document).expect("the fixture builds a graph");

    let first = exposed(&graph, "vehicle structure view");
    let second = exposed(&graph, "vehicle structure view");

    assert_eq!(first, second);
    assert_eq!(
        first,
        vec![
            "feature.VehicleViews.vehicle.engine".to_string(),
            "feature.VehicleViews.vehicle.wheel".to_string(),
        ],
        "`expose vehicle::**` is the subtree under vehicle, excluding vehicle itself"
    );
}

/// A scope naming nothing must be reported, not silently dropped. A view with
/// a typo should look broken rather than empty.
#[test]
fn sv2_an_unresolvable_scope_is_reported() {
    let document = compile(
        r#"
package P {
    part vehicle { part brake; }
    view v { expose nosuchthing::**; }
}
"#,
        "unresolvable.sysml",
    );
    let graph = Graph::from_document(document).expect("the source builds a graph");
    let view = view_node(&graph, "v");

    let resolution = resolve_exposed_elements(&graph, view);
    assert!(resolution.elements.is_empty());
    assert_eq!(resolution.unresolved, vec!["nosuchthing::**".to_string()]);
}

/// **The SV-2 exit criterion**, on the pilot's own discriminating case:
/// `11-View and Viewpoint/11b-Safety and Security Feature Views.sysml`.
///
/// The corpus file is vendored under the outer repo's `external/`, which a
/// crate test cannot reach, so its content is reproduced here. Before SV-2 the
/// three views resolved to identical exposed content; they must now diverge,
/// and each must be *correct* -- divergence alone would be satisfied by three
/// different wrong answers.
///
/// One deviation from the file, about **import visibility rather than views**,
/// and not touching what this test measures: the pilot splits the model across
/// `AnnotationDefinitions`, `PartsTree`, `ViewDefinitions`, and `Views`
/// packages joined by `public import`. A metadata definition is not visible
/// across those package boundaries today, so everything is declared in one
/// package here. That is a pre-existing name-resolution gap, unrelated to
/// `expose`.
///
/// The pilot's `private import PartsTree::vehicle;` was a second deviation
/// through SV-2; it now compiles -- see
/// `an_import_resolves_a_qualified_path_to_a_usage`.
#[test]
fn sv2_the_pilot_safety_and_security_views_diverge_correctly() {
    let document = compile(
        r#"
package SafetyAndSecurityFeatureViews {
    metadata def Safety { attribute isMandatory : ScalarValues::Boolean; }
    metadata def Security;

    part vehicle {
        part interior {
            part alarm { @Security; }
            part seatBelt { @Safety { isMandatory = true; } }
            part frontSeat;
            part driverAirBag { @Safety { isMandatory = false; } }
        }
        part bodyAssy {
            part body;
            part bumper { @Safety { isMandatory = true; } }
            part keylessEntry { @Security; }
        }
        part wheelAssy {
            part wheel;
            part antilockBrakes { @Safety { isMandatory = false; } }
        }
    }

    view def SafetyFeatureView { filter @Safety; }
    view def SafetyOrSecurityFeatureView { filter @Safety | @Security; }

    view vehicleSafetyFeatureView : SafetyFeatureView {
        expose vehicle;
    }

    view vehicleSafetyOrSecurityFeatureView : SafetyOrSecurityFeatureView {
        expose vehicle;
    }

    view vehicleMandatorySafetyFeatureViewStandalone {
        expose vehicle::**[@Safety and (as Safety).isMandatory];
    }
}
"#,
        "11b-safety-and-security.sysml",
    );
    let graph = Graph::from_document(document).expect("the pilot source builds a graph");

    let safety = short_names(&graph, "vehicleSafetyFeatureView");
    let safety_or_security = short_names(&graph, "vehicleSafetyOrSecurityFeatureView");
    let mandatory = short_names(&graph, "vehicleMandatorySafetyFeatureViewStandalone");

    assert_eq!(
        safety,
        vec!["antilockBrakes", "bumper", "driverAirBag", "seatBelt"],
        "`filter @Safety` selects exactly the @Safety-annotated parts"
    );
    assert_eq!(
        safety_or_security,
        vec![
            "alarm",
            "antilockBrakes",
            "bumper",
            "driverAirBag",
            "keylessEntry",
            "seatBelt"
        ],
        "`filter @Safety | @Security` adds the two @Security parts"
    );
    assert_eq!(
        mandatory,
        vec!["bumper", "seatBelt"],
        "`@Safety and (as Safety).isMandatory` drops the two isMandatory=false parts"
    );

    // Divergence is the headline: before SV-2 all three were identical.
    assert_ne!(safety, safety_or_security);
    assert_ne!(safety, mandatory);
    assert_ne!(safety_or_security, mandatory);
}

/// A view inherits the filters of the view it is typed by and of every view it
/// specializes. This is what makes the pilot's 11b views diverge.
#[test]
fn sv2_filters_are_inherited_through_typing_and_specialization() {
    let document = compile(
        r#"
package P {
    metadata def Safety;
    part vehicle {
        part brake { @Safety; }
        part radio;
    }
    view def SafetyView { filter @Safety; }
    view scoped : SafetyView { expose vehicle::**; }
    view narrowed :> scoped { expose vehicle::**; filter not (@Safety); }
}
"#,
        "inheritance.sysml",
    );
    let graph = Graph::from_document(document).expect("the source builds a graph");

    assert_eq!(
        exposed(&graph, "scoped"),
        vec!["feature.P.vehicle.brake".to_string()],
        "`scoped` inherits `filter @Safety` from the definition it is typed by"
    );
    assert!(
        exposed(&graph, "narrowed").is_empty(),
        "`narrowed` inherits @Safety and adds not(@Safety), so nothing can satisfy both"
    );
}

/// V-6.3 exit criterion, first half: every fixture with a frozen spec must
/// **read back** as exactly that spec. The `.view.json` files are the map's
/// contract, so this is the assertion that the map is implemented as written
/// rather than as convenient.
#[test]
fn sv3_every_fixture_reads_back_as_its_frozen_spec() {
    for (fixture, view_name) in FIXTURES_WITH_SPECS {
        let document = compile(&read_fixture(&format!("{fixture}.sysml")), fixture);
        let graph = Graph::from_document(document).expect("the fixture builds a graph");
        let actual = view_spec_from_usage(&graph, view_node(&graph, view_name))
            .unwrap_or_else(|error| panic!("{fixture}: {error}"));

        let expected: serde_json::Value =
            serde_json::from_str(&read_fixture(&format!("{fixture}.view.json")))
                .unwrap_or_else(|error| panic!("{fixture}.view.json parses: {error}"));
        let expected = expected
            .get("documents")
            .and_then(|documents| {
                documents
                    .as_array()?
                    .iter()
                    .find(|document| document["diagram"]["title"] == *view_name)
                    .cloned()
            })
            .unwrap_or(expected);

        assert_eq!(
            serde_json::to_value(&actual).expect("the spec serializes"),
            expected,
            "{fixture} -> {view_name} did not read back as its frozen spec"
        );
    }
}

/// V-6.3 exit criterion, second half: `spec -> usage -> spec` is the identity
/// for tier 1. The draft is rendered to SysML, compiled for real, and read
/// back, so this exercises the whole loop rather than a pair of pure functions
/// agreeing with each other.
#[test]
fn sv3_every_tier1_spec_round_trips_through_real_sysml() {
    for (fixture, view_name) in FIXTURES_WITH_SPECS {
        let source = read_fixture(&format!("{fixture}.sysml"));
        let document = compile(&source, fixture);
        let graph = Graph::from_document(document).expect("the fixture builds a graph");
        let original = view_spec_from_usage(&graph, view_node(&graph, view_name))
            .unwrap_or_else(|error| panic!("{fixture}: {error}"));

        let draft = usage_from_view_spec(&original)
            .unwrap_or_else(|error| panic!("{fixture} drafts a usage: {error}"));

        // Rebuild a whole source file around the drafted view: its exposes name
        // qualified paths, so they need the elements they point at to exist.
        let rebuilt = rebuild_source(&source, view_name, &draft);
        let recompiled = compile(&rebuilt, &format!("{fixture}-roundtrip"));
        let graph = Graph::from_document(recompiled).expect("the round-trip builds a graph");
        let returned = view_spec_from_usage(&graph, view_node(&graph, view_name))
            .unwrap_or_else(|error| panic!("{fixture} round-trip: {error}"));

        assert_eq!(
            returned, original,
            "{fixture} -> {view_name} did not round-trip; rebuilt source was:\n{rebuilt}"
        );
    }
}

/// The tier-3 rule: refuse, and name the field. A free-text search has no
/// scope, no notation, and no stable element set.
#[test]
fn sv3_a_tier3_view_is_refused_by_field_name() {
    let spec = ViewDocumentDto::model(ModelViewSpecDto {
        version: 1,
        kind: ModelViewKindDto::Search,
        title: "find brake".to_string(),
        description: None,
        root: None,
        graph_scope: None,
        query: Some("brake".to_string()),
        expanded_parents: Vec::new(),
        expanded_children: Vec::new(),
        include_reference_edges: true,
    });

    let error = usage_from_view_spec(&spec).expect_err("a search is not a view");
    assert_eq!(error.field, "model.query");
}

// ---------------------------------------------------------------- helpers

fn compile(source: &str, source_name: &str) -> KirDocument {
    let stdlib = load_sysml_baseline().expect("stdlib baseline loads");
    compile_sysml_text(source, source_name, &stdlib).expect("source compiles")
}

/// KIR kind of the element with the given declared name.
fn kind_of(document: &KirDocument, declared_name: &str) -> Option<String> {
    document
        .elements
        .iter()
        .find(|element| {
            element
                .properties
                .get("declared_name")
                .and_then(|value| value.as_str())
                == Some(declared_name)
        })
        .map(|element| element.kind.clone())
}

/// Member outline of the first definition matching `keyword` and `name`.
fn find_definition(module: &SysmlModule, keyword: &str, name: &str) -> Option<Vec<String>> {
    find(module, keyword, name, true)
}

/// Member outline of the first usage matching `keyword` and `name`.
fn find_usage_members(module: &SysmlModule, keyword: &str, name: &str) -> Option<Vec<String>> {
    find(module, keyword, name, false)
}

/// The first usage declaration anywhere in the module with the given keyword.
fn find_usage(
    module: &SysmlModule,
    keyword: &str,
) -> Option<mercurio_foundation::language_contracts::ast::GenericUsageDecl> {
    fn visit(
        declaration: &Declaration,
        keyword: &str,
    ) -> Option<mercurio_foundation::language_contracts::ast::GenericUsageDecl> {
        if let Some(usage) = declaration.as_usage_like()
            && usage.keyword == keyword
        {
            return Some(usage);
        }
        declaration
            .child_declarations()
            .iter()
            .find_map(|child| visit(child, keyword))
    }

    module
        .members
        .iter()
        .find_map(|member| visit(member, keyword))
}

fn find(module: &SysmlModule, keyword: &str, name: &str, definition: bool) -> Option<Vec<String>> {
    fn visit(
        declaration: &Declaration,
        keyword: &str,
        name: &str,
        definition: bool,
    ) -> Option<Vec<String>> {
        let matched = if definition {
            declaration
                .as_definition_like()
                .is_some_and(|decl| decl.keyword == keyword && decl.name == name)
        } else {
            declaration
                .as_usage_like()
                .is_some_and(|decl| decl.keyword == keyword && decl.name == name)
        };

        if matched {
            let mut lines = Vec::new();
            for child in declaration.child_declarations() {
                walk(child, 0, &mut lines);
            }
            return Some(lines);
        }

        declaration
            .child_declarations()
            .iter()
            .find_map(|child| visit(child, keyword, name, definition))
    }

    module
        .members
        .iter()
        .find_map(|member| visit(member, keyword, name, definition))
}

/// Node id of the view usage or definition with this declared name.
fn view_node(graph: &Graph, declared_name: &str) -> NodeId {
    graph
        .elements()
        .iter()
        .find(|element| {
            element.kind.contains("View")
                && element.properties.to_btree_map().get("declared_name")
                    == Some(&serde_json::Value::String(declared_name.to_string()))
        })
        .unwrap_or_else(|| panic!("no view named `{declared_name}`"))
        .id
}

fn exposed(graph: &Graph, declared_name: &str) -> Vec<String> {
    exposed_elements(graph, view_node(graph, declared_name))
}

/// Declared names of what a view exposes -- the readable projection for
/// assertions, since element ids carry the whole containment path.
fn short_names(graph: &Graph, view: &str) -> Vec<String> {
    let mut names: Vec<String> = exposed(graph, view)
        .iter()
        .filter_map(|id| {
            graph.element_by_element_id(id).and_then(|element| {
                element
                    .properties
                    .to_btree_map()
                    .get("declared_name")
                    .cloned()
            })
        })
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect();
    names.sort();
    names
}

/// A qualified path may name a **usage**, not just a definition:
/// `import Tree::vehicle;` -- the shape the pilot itself writes as
/// `private import PartsTree::vehicle;`. This failed to compile through SV-2,
/// which recorded it as a standing gap; the import path now consults the
/// module's usages after its definitions, and binds the usage's element id.
///
/// The assertion is on the emitted target rather than on compiling at all,
/// because "it compiles" would also be satisfied by an import that resolved to
/// the wrong element -- or to a plausible id naming nothing.
#[test]
fn an_import_resolves_a_qualified_path_to_a_usage() {
    let document = compile(
        r#"
package P {
    package Tree { part vehicle { part brake; } }
    package Uses { private import Tree::vehicle; }
}
"#,
        "usage-import.sysml",
    );

    let vehicle = document
        .elements
        .iter()
        .find(|element| {
            element
                .properties
                .get("declared_name")
                .and_then(|value| value.as_str())
                == Some("vehicle")
        })
        .expect("the model declares a `vehicle` part");

    let imports = document
        .elements
        .iter()
        .filter(|element| element.kind == "SysML::Import")
        .collect::<Vec<_>>();
    assert_eq!(imports.len(), 1, "the source declares exactly one import");

    let targets = imports[0]
        .properties
        .get("imports")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    assert_eq!(
        targets,
        vec![vehicle.id.clone()],
        "the import must point at the `vehicle` element itself, not at an          unresolved path"
    );
}

/// The same path, one segment shorter than the declaration is deep: a suffix
/// match, the way definitions have always resolved.
#[test]
fn an_import_resolves_a_usage_by_unique_suffix() {
    let document = compile(
        r#"
package P {
    package Tree { part vehicle { part brake; } }
    package Uses { private import vehicle::brake; }
}
"#,
        "usage-import-suffix.sysml",
    );

    let brake = document
        .elements
        .iter()
        .find(|element| {
            element
                .properties
                .get("declared_name")
                .and_then(|value| value.as_str())
                == Some("brake")
        })
        .expect("the model declares a `brake` part");

    let target = document
        .elements
        .iter()
        .find(|element| element.kind == "SysML::Import")
        .and_then(|element| element.properties.get("imports").cloned())
        .and_then(|value| value.as_array().and_then(|values| values.first().cloned()))
        .and_then(|value| value.as_str().map(str::to_string));

    assert_eq!(target, Some(brake.id.clone()));
}

/// A path naming nothing is still a hard error. An import binds a name into a
/// scope, so an unresolvable one is a defect the author must see -- widening
/// resolution to usages must not turn that into a silent verbatim target.
#[test]
fn an_unresolvable_import_is_still_an_error() {
    let stdlib = load_sysml_baseline().expect("stdlib baseline loads");
    let result = compile_sysml_text(
        r#"
package P {
    package Tree { part vehicle; }
    package Uses { private import Tree::nosuchthing; }
}
"#,
        "usage-import-missing.sysml",
        &stdlib,
    );

    let message = result.err().map(|error| error.message);
    assert_eq!(
        message.as_deref(),
        Some("unresolved import `Tree::nosuchthing`"),
        "an import that names nothing must fail the compile"
    );
}

/// Fixtures whose `.view.json` freezes an expected spec, paired with the view
/// each one is about.
const FIXTURES_WITH_SPECS: &[(&str, &str)] = &[
    ("tier1/tree-diagram-subtree", "vehicle structure view"),
    ("tier1/element-table-columns", "component table"),
    ("tier1/interconnection-diagram", "system internals"),
    ("tier1/view-def-metaclass-filter", "vehicle parts"),
    ("tier1/explicit-exposes", "curated pair"),
    ("tier1/metadata-filtered-expose", "safety features view"),
    ("tier1/metadata-filtered-expose", "non-safety features view"),
];

/// Replace one named view declaration with the drafted one, keeping the model
/// around it. A view's exposes are qualified paths into that model, so a draft
/// cannot be compiled on its own.
///
/// Matched by name, not by the `view ` keyword: a `rendering` body also holds
/// `view` members (its columns), and replacing those would rewrite the very
/// structure the round-trip is checking.
fn rebuild_source(original: &str, view_name: &str, draft: &ViewUsageDraft) -> String {
    let bare = format!("view {view_name}");
    let quoted = format!("view '{view_name}'");

    let mut out = String::new();
    let mut depth = 0usize;
    let mut skipping = false;

    for line in original.lines() {
        let trimmed = line.trim_start();
        if skipping {
            depth += line.matches('{').count();
            depth = depth.saturating_sub(line.matches('}').count());
            if depth == 0 {
                skipping = false;
            }
            continue;
        }
        if trimmed.starts_with(&quoted) || trimmed.starts_with(&bare) {
            for drafted in draft.to_sysml().lines() {
                out.push_str("    ");
                out.push_str(drafted);
                out.push('\n');
            }
            depth = line.matches('{').count();
            depth = depth.saturating_sub(line.matches('}').count());
            skipping = depth > 0;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Multiple table columns are **subsets** of `columnView`, not redefinitions of
/// it, and the difference is why this fixture used to fail to compile.
///
/// `asElementTable` declares `view columnView[0..*] ordered`. `:>` adds
/// instances to a many-valued feature; `:>>` replaces it. The pilot only ever
/// writes one column -- `view :>> columnView[1]`, where `[1]` is the redefining
/// feature's *multiplicity*, not an index (Views Example.sysml:16-20). This
/// fixture originally wrote `[1]` and `[2]` as though `[n]` numbered the
/// columns, which is two definitions of one feature; the duplicate-id error was
/// the compiler being right, not an id-template defect.
///
/// Both forms are asserted here so the distinction cannot quietly rot back.
#[test]
fn sv3_columns_are_subsets_of_column_view_not_redefinitions() {
    let stdlib = load_sysml_baseline().expect("stdlib baseline loads");
    let model = "package P {\n    private import Views::*;\n    part def Component;\n    part alpha : Component;\n";

    let subsets = format!(
        "{model}    rendering r :> asElementTable {{\n        view name :> columnView {{ render asTextualNotation; }}\n        view documentation :> columnView {{ render asTextualNotation; }}\n    }}\n    view t {{ expose P::**; render r; }}\n}}"
    );
    assert!(
        compile_sysml_text(&subsets, "subsets.sysml", &stdlib).is_ok(),
        "two columnView subsets must compile"
    );

    let redefinitions = format!(
        "{model}    rendering r :> asElementTable {{\n        view :>> columnView[1] {{ render asTextualNotation; }}\n        view :>> columnView[2] {{ render asTextualNotation; }}\n    }}\n    view t {{ expose P::**; render r; }}\n}}"
    );
    let error = compile_sysml_text(&redefinitions, "redefinitions.sysml", &stdlib)
        .expect_err("two redefinitions of one feature must not compile");
    assert!(
        error.message.contains("duplicate emitted KIR id"),
        "the collision must be reported as what it is; got {}",
        error.message
    );
}
