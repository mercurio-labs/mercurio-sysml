//! SV-0 — the Save-as-View mapping contract, as fixtures.
//!
//! Plan: `docs/save-as-view-plan.md` (visualization-plan V-6, gate opened
//! 2026-08-27). This increment adds **no behaviour**. It freezes the
//! `mercurio.view.v1` <-> SysML v2 view map as a fixture corpus, and — more
//! immediately useful — it *characterises what the compiler does today* with
//! every construct that map depends on.
//!
//! The characterisation tests below PASS on today's compiler. They are not
//! aspirational: each one pins a defect so that SV-1/SV-2 flipping it is a
//! deliberate, reviewed change rather than an invisible one. When an increment
//! fixes a construct, the corresponding `characterisation_*` test is expected
//! to fail — update it, and un-ignore its `sv*_` counterpart, in the same
//! commit.
//!
//! Findings recorded here (all verified 2026-08-27, see the plan's evidence
//! table):
//!
//! - `expose X::**` is not merely unsupported; it **silently mis-parses** into
//!   a usage declaration named after the first path segment. The recursive
//!   wildcard vanishes with no diagnostic.
//! - `import X::**` keeps the wildcard but **silently drops the `[@Meta]`
//!   predicate**. That is a standing correctness bug independent of this plan:
//!   the pilot's own `Filtering Example-2.sysml` resolves `vehicle::**[@Safety]`
//!   as `vehicle::**`, i.e. every part rather than the safety-annotated ones.
//! - A `view def`'s `filter` member is **dropped entirely**, leaving an empty
//!   body.
//! - Consequently the pilot's safety / non-safety view pair is **currently
//!   indistinguishable**. Making those two diverge is SV-2's exit criterion.

use std::path::{Path, PathBuf};

use mercurio_foundation::language_contracts::ast::Declaration;
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
            lines.push(format!("{pad}import {}", import.path.as_colon_string()));
        }
        Declaration::Alias(alias) => {
            lines.push(format!("{pad}alias {}", alias.name));
        }
        _ => {
            if let Some(definition) = declaration.as_definition_like() {
                lines.push(format!("{pad}{} def {}", definition.keyword, definition.name));
            } else if let Some(usage) = declaration.as_usage_like() {
                lines.push(format!("{pad}{} {}", usage.keyword, usage.name));
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

// ------------------------------------------------- characterisation (today)

/// `expose <path>::**` mis-parses into a usage declaration named after the
/// first path segment. The recursive wildcard is dropped with no diagnostic.
///
/// This is the fact that makes SV-5 (writing a view out) hard-blocked on SV-1
/// (compiling it): localized writeback falls back to `canonical_rewrite`, which
/// re-renders declarations from the parsed AST — so writing `expose` before it
/// parses would corrupt the file on the next unrelated mutation to it.
#[test]
fn characterisation_expose_drops_the_recursive_wildcard() {
    let module = parse_fixture("tier1/tree-diagram-subtree.sysml");
    let lines = flat(&module);

    assert!(
        lines.iter().any(|line| line == "expose vehicle"),
        "expected today's mis-parse `expose vehicle`; got {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("::**")),
        "the recursive wildcard is expected to be dropped today; got {lines:?}"
    );
}

/// A `view def`'s `filter` member is dropped entirely, leaving an empty body.
#[test]
fn characterisation_view_def_filter_member_is_dropped() {
    let module = parse_fixture("tier1/view-def-metaclass-filter.sysml");

    let view_def = find_definition(&module, "view", "Part Structure View")
        .expect("the view def itself parses today");
    assert!(
        view_def.is_empty(),
        "`filter @SysML::PartUsage;` is expected to be dropped today; got {view_def:?}"
    );
    assert!(
        !flat(&module).iter().any(|line| line.starts_with("filter")),
        "no filter declaration is expected to survive today"
    );
}

/// `import` and `expose` fail *differently*, and that shapes SV-1: `expose`
/// should route through the existing import path, which already carries the
/// wildcard. The dropped predicate is a shared gap — and on the import side it
/// is a standing correctness bug in shipped behaviour.
#[test]
fn characterisation_import_keeps_wildcard_but_drops_predicate() {
    let module = parse_fixture("tier1/import-expose-parity.sysml");
    let lines = flat(&module);

    assert!(
        lines.iter().any(|line| line == "import vehicle::**"),
        "import is expected to preserve the recursive wildcard; got {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("@Safety")),
        "the [@Safety] predicate is expected to be dropped on BOTH paths today; got {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "expose vehicle"),
        "expose is expected to drop the wildcard entirely; got {lines:?}"
    );
}

/// The discriminating case, and SV-2's exit criterion in negative form. The
/// pilot's safety / non-safety view pair currently parses to the identical
/// member list, so nothing downstream can tell the two views apart.
#[test]
fn characterisation_safety_and_non_safety_views_are_indistinguishable() {
    let module = parse_fixture("tier1/metadata-filtered-expose.sysml");

    let safety = find_usage_members(&module, "view", "safety features view")
        .expect("the safety view parses");
    let non_safety = find_usage_members(&module, "view", "non-safety features view")
        .expect("the non-safety view parses");

    assert_eq!(
        safety, non_safety,
        "today these two views are indistinguishable after parsing; SV-2 is not \
         done until this assertion fails"
    );
    assert_eq!(safety, vec!["expose vehicle", "render asTreeDiagram"]);
}

// ------------------------------------------------------- targets (ignored)

/// SV-2 exit criterion. The inverse of the characterisation above.
#[test]
#[ignore = "SV-2: expose resolution — un-ignore when `expose X::**[@Meta]` resolves"]
fn sv2_safety_and_non_safety_views_must_diverge() {
    let module = parse_fixture("tier1/metadata-filtered-expose.sysml");

    let safety = find_usage_members(&module, "view", "safety features view")
        .expect("the safety view parses");
    let non_safety = find_usage_members(&module, "view", "non-safety features view")
        .expect("the non-safety view parses");

    assert_ne!(
        safety, non_safety,
        "a view exposing [@Safety] and one exposing [not (@Safety)] must not \
         resolve to the same exposed content"
    );
}

/// SV-1 exit criterion for the wildcard half.
#[test]
#[ignore = "SV-1: compile the view metamodel — un-ignore when `expose` parses as a namespace query"]
fn sv1_expose_preserves_the_recursive_wildcard() {
    let module = parse_fixture("tier1/tree-diagram-subtree.sysml");
    let lines = flat(&module);
    assert!(
        lines.iter().any(|line| line.contains("::**")),
        "expose must carry the recursive wildcard; got {lines:?}"
    );
}

/// SV-1 exit criterion for `filter`.
#[test]
#[ignore = "SV-1: compile the view metamodel — un-ignore when `filter` is a body member"]
fn sv1_view_def_retains_its_filter_member() {
    let module = parse_fixture("tier1/view-def-metaclass-filter.sysml");
    let view_def = find_definition(&module, "view", "Part Structure View")
        .expect("the view def parses");
    assert!(
        !view_def.is_empty(),
        "`filter @SysML::PartUsage;` must survive into the view def's body"
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
