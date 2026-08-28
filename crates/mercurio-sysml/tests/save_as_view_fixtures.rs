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
use mercurio_foundation::views::{exposed_elements, resolve_exposed_elements};
use mercurio_sysml::{KirDocument, SysmlModule, compile_sysml_text, load_sysml_baseline, parse_sysml};

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
                lines.push(format!("{pad}{} def {}", definition.keyword, definition.name));
            } else if let Some(usage) = declaration.as_usage_like() {
                // An element filter carries a predicate rather than a name.
                match usage.metadata_properties.get("condition") {
                    Some(condition) => {
                        lines.push(format!("{pad}{} {condition}", usage.keyword))
                    }
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
        assert!((1..=3).contains(&tier), "fixture `{id}` has an unknown tier");

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

    let view_def = find_definition(&module, "view", "Part Structure View")
        .expect("the view def parses");
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
        flat(&module).iter().any(|line| line
            == "import vehicle::**[@Safety and (as Safety).isMandatory]"),
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
    assert_eq!(safety, vec!["feature.SafetyViews.vehicle.brake".to_string()]);
    assert_eq!(non_safety, vec!["feature.SafetyViews.vehicle.radio".to_string()]);
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
/// Two deviations from the file, both about **import visibility rather than
/// views**, and neither touching what this test measures:
///
/// - The pilot splits the model across `AnnotationDefinitions`, `PartsTree`,
///   `ViewDefinitions`, and `Views` packages joined by `public import`. A
///   metadata definition is not visible across those package boundaries today,
///   so everything is declared in one package here.
/// - The pilot writes `private import PartsTree::vehicle;`. A qualified path
///   to a *usage* does not resolve -- see
///   `sv2_a_qualified_path_to_a_usage_does_not_resolve`.
///
/// Both are pre-existing name-resolution gaps, unrelated to `expose`.
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

/// SV-3 exit criterion. Every tier-1 and tier-2 fixture must round-trip
/// through `view_spec_from_usage` / `usage_from_view_spec` under the rule its
/// manifest entry declares; every tier-3 fixture must return `NotReifiable`.
/// The mapper lives in `mercurio-views` (foundation), so this test grows a
/// foundation dependency when SV-3 lands.
#[test]
#[ignore = "SV-3: the bidirectional map does not exist yet"]
fn sv3_every_fixture_round_trips_under_its_declared_rule() {
    unimplemented!("SV-3: view_spec_from_usage / usage_from_view_spec");
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

fn find(
    module: &SysmlModule,
    keyword: &str,
    name: &str,
    definition: bool,
) -> Option<Vec<String>> {
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
            graph
                .element_by_element_id(id)
                .and_then(|element| element.properties.to_btree_map().get("declared_name").cloned())
        })
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect();
    names.sort();
    names
}

/// A standing gap this increment did **not** close, recorded so it is not
/// rediscovered: a qualified path naming a *usage* does not resolve in an
/// `import`. Only definitions are bound at that point, so the pilot's own
/// `private import PartsTree::vehicle;` fails to compile.
///
/// `expose` no longer depends on this -- SV-2 made a non-wildcard expose scope
/// fall through as verbatim text, exactly as a wildcard scope always has, and
/// `exposed_elements` binds it against the graph. So this blocks `import`, not
/// views. Closing it means teaching import resolution to render usage ids,
/// which lives in the emission phase; that is its own increment.
#[test]
#[ignore = "known gap: import cannot resolve a qualified path to a usage"]
fn sv2_a_qualified_path_to_a_usage_does_not_resolve() {
    let stdlib = load_sysml_baseline().expect("stdlib baseline loads");
    let result = compile_sysml_text(
        r#"
package P {
    package Tree { part vehicle { part brake; } }
    package Uses { private import Tree::vehicle; }
}
"#,
        "usage-import.sysml",
        &stdlib,
    );

    assert!(
        result.is_ok(),
        "un-ignore this when import resolves usage paths; got {:?}",
        result.err().map(|error| error.message)
    );
}
