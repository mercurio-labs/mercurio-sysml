//! Cross-authoring proof for the KIR equivalence oracle (DA-11 / C1.3).
//!
//! The same model authored as (1) one file with reordered declarations,
//! (2) a reformatted file under a different name, and (3) a project split
//! across two files must compile to oracle-equivalent KIR, while genuine
//! semantic changes (renamed part, dropped satisfy) must not. The reference
//! model deliberately includes position-id-bearing constructs (an anonymous
//! connection with reference ends, a succession inside an action definition)
//! because those are exactly the cases that fail without id canonicalization.

use mercurio_foundation::kir_canonical::{
    kir_documents_equivalent, kir_equivalence_diff, kir_equivalence_report, semantic_diff_is_empty,
};
use mercurio_sysml::{
    KirDocument, compile_sysml_text, compile_sysml_text_with_context, load_sysml_baseline,
    parse_sysml, sysml_field_specs,
};

const REFERENCE: &str = r#"
package Demo {
    part def Vehicle {
        attribute mass : ScalarValues::Real = 100.0;
    }
    part def Engine;
    part vehicle : Vehicle {
        part engine : Engine;
    }
    connection : Connections::Connection connect vehicle.engine to vehicle;
    requirement def MassLimit;
    requirement massReq : MassLimit;
    satisfy massReq by vehicle;
    action def Drive {
        action first_step;
        action second_step;
        first first_step then second_step;
    }
}
"#;

/// Same declarations, different top-level order — every position-derived id
/// (connection, reference ends, succession) lands on different lines and the
/// namespace member lists come out in a different order.
const REORDERED: &str = r#"
package Demo {
    action def Drive {
        action first_step;
        action second_step;
        first first_step then second_step;
    }
    requirement def MassLimit;
    part def Engine;
    part def Vehicle {
        attribute mass : ScalarValues::Real = 100.0;
    }
    part vehicle : Vehicle {
        part engine : Engine;
    }
    requirement massReq : MassLimit;
    connection : Connections::Connection connect vehicle.engine to vehicle;
    satisfy massReq by vehicle;
}
"#;

/// Same declaration order, but reformatted (indentation, blank lines, and
/// line breaks shifted) so every source span differs; compiled under a
/// different file name as well.
const REFORMATTED: &str = r#"

package Demo {

        part def Vehicle {
                attribute mass : ScalarValues::Real = 100.0; }

        part def Engine;

        part vehicle : Vehicle { part engine : Engine; }


        connection : Connections::Connection connect vehicle.engine to vehicle;

        requirement def MassLimit;
        requirement massReq : MassLimit;
        satisfy massReq by vehicle;

        action def Drive {
                action first_step;

                action second_step;
                first first_step then second_step; }
}
"#;

/// Two-package model authored in a single file — the reference side for the
/// split-across-files case (KIR merge requires distinct packages per file).
const TWO_PACKAGE_REFERENCE: &str = r#"
package Vocabulary {
    part def Vehicle {
        attribute mass : ScalarValues::Real = 100.0;
    }
    part def Engine;
    requirement def MassLimit;
    action def Drive {
        action first_step;
        action second_step;
        first first_step then second_step;
    }
}
package Demo {
    import Vocabulary::*;
    part vehicle : Vehicle {
        part engine : Engine;
    }
    connection : Connections::Connection connect vehicle.engine to vehicle;
    requirement massReq : MassLimit;
    satisfy massReq by vehicle;
}
"#;

/// The definitions half of the split-project variant.
const SPLIT_DEFINITIONS: &str = r#"
package Vocabulary {
    part def Vehicle {
        attribute mass : ScalarValues::Real = 100.0;
    }
    part def Engine;
    requirement def MassLimit;
    action def Drive {
        action first_step;
        action second_step;
        first first_step then second_step;
    }
}
"#;

/// The usages half of the split-project variant.
const SPLIT_USAGES: &str = r#"
package Demo {
    import Vocabulary::*;
    part vehicle : Vehicle {
        part engine : Engine;
    }
    connection : Connections::Connection connect vehicle.engine to vehicle;
    requirement massReq : MassLimit;
    satisfy massReq by vehicle;
}
"#;

/// Reference model with `part def Vehicle` renamed to `Car` — a genuine
/// semantic change the oracle must report.
const RENAMED_PART: &str = r#"
package Demo {
    part def Car {
        attribute mass : ScalarValues::Real = 100.0;
    }
    part def Engine;
    part vehicle : Car {
        part engine : Engine;
    }
    connection : Connections::Connection connect vehicle.engine to vehicle;
    requirement def MassLimit;
    requirement massReq : MassLimit;
    satisfy massReq by vehicle;
    action def Drive {
        action first_step;
        action second_step;
        first first_step then second_step;
    }
}
"#;

/// Reference model with the satisfy relationship dropped — also a genuine
/// semantic change.
const DROPPED_SATISFY: &str = r#"
package Demo {
    part def Vehicle {
        attribute mass : ScalarValues::Real = 100.0;
    }
    part def Engine;
    part vehicle : Vehicle {
        part engine : Engine;
    }
    connection : Connections::Connection connect vehicle.engine to vehicle;
    requirement def MassLimit;
    requirement massReq : MassLimit;
    action def Drive {
        action first_step;
        action second_step;
        first first_step then second_step;
    }
}
"#;

fn compile(source: &str, source_name: &str) -> KirDocument {
    let stdlib = load_sysml_baseline().expect("stdlib baseline loads");
    compile_sysml_text(source, source_name, &stdlib).expect("reference model compiles")
}

fn compile_project(files: &[(&str, &str)]) -> KirDocument {
    let stdlib = load_sysml_baseline().expect("stdlib baseline loads");
    let modules = files
        .iter()
        .map(|(_, source)| parse_sysml(source).expect("project file parses"))
        .collect::<Vec<_>>();
    let documents = files
        .iter()
        .map(|(name, source)| {
            compile_sysml_text_with_context(source, name, &modules, &stdlib)
                .expect("project file compiles")
        })
        .collect::<Vec<_>>();
    KirDocument::merge_with_registered_fields(documents, sysml_field_specs().iter().copied())
        .expect("project documents merge")
}

fn assert_position_ids_present(document: &KirDocument) {
    let has_positional = document.elements.iter().any(|element| {
        element
            .id
            .rsplit('.')
            .next()
            .map(|segment| {
                segment
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'_')
                    && segment.bytes().any(|byte| byte.is_ascii_digit())
            })
            .unwrap_or(false)
    });
    assert!(
        has_positional,
        "fixture must exercise position-derived ids; got {:?}",
        document
            .elements
            .iter()
            .map(|element| element.id.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn reordered_declarations_are_equivalent() {
    let reference = compile(REFERENCE, "demo.sysml");
    let reordered = compile(REORDERED, "demo.sysml");
    assert_position_ids_present(&reference);

    let diff = kir_equivalence_diff(&reference, &reordered);
    assert!(
        semantic_diff_is_empty(&diff),
        "reordered declarations must be oracle-equivalent, diff: {}",
        serde_json::to_string_pretty(&diff).unwrap_or_default()
    );
}

#[test]
fn reformatted_and_renamed_file_is_equivalent() {
    let reference = compile(REFERENCE, "demo.sysml");
    let reformatted = compile(REFORMATTED, "renamed.sysml");

    let diff = kir_equivalence_diff(&reference, &reformatted);
    assert!(
        semantic_diff_is_empty(&diff),
        "reformatted/renamed-file variant must be oracle-equivalent, diff: {}",
        serde_json::to_string_pretty(&diff).unwrap_or_default()
    );
}

#[test]
fn split_across_two_files_is_equivalent() {
    let reference = compile_project(&[("demo.sysml", TWO_PACKAGE_REFERENCE)]);
    let split = compile_project(&[
        ("definitions.sysml", SPLIT_DEFINITIONS),
        ("usages.sysml", SPLIT_USAGES),
    ]);
    assert_position_ids_present(&split);

    let diff = kir_equivalence_diff(&reference, &split);
    assert!(
        semantic_diff_is_empty(&diff),
        "split-across-files variant must be oracle-equivalent, diff: {}",
        serde_json::to_string_pretty(&diff).unwrap_or_default()
    );
}

#[test]
fn raw_diff_is_position_sensitive_but_oracle_is_not() {
    // The measuring stick: without canonicalization the reordered pair
    // raw-diffs as different (position-suffixed ids and member order), so an
    // empty oracle diff is meaningful.
    let reference = compile(REFERENCE, "demo.sysml");
    let reordered = compile(REORDERED, "demo.sysml");

    let raw = mercurio_foundation::diff_kir_documents(&reference, &reordered);
    assert!(
        !semantic_diff_is_empty(&raw),
        "expected the raw diff to be position-sensitive for this fixture"
    );
    assert!(kir_documents_equivalent(&reference, &reordered));
}

#[test]
fn renamed_part_is_reported() {
    let reference = compile(REFERENCE, "demo.sysml");
    let renamed = compile(RENAMED_PART, "demo.sysml");

    let report = kir_equivalence_report(&reference, &renamed);
    assert!(
        !report.equivalent,
        "a renamed part definition is a genuine semantic change"
    );
    let mentions_car = report
        .diff
        .added_elements
        .iter()
        .chain(report.diff.removed_elements.iter())
        .any(|element| {
            element.element_id.contains("Car") || element.element_id.contains("Vehicle")
        })
        || !report.diff.renamed_elements.is_empty();
    assert!(mentions_car, "diff should surface the rename: {report:?}");
}

#[test]
fn dropped_satisfy_is_reported() {
    let reference = compile(REFERENCE, "demo.sysml");
    let dropped = compile(DROPPED_SATISFY, "demo.sysml");

    let diff = kir_equivalence_diff(&reference, &dropped);
    assert!(
        !semantic_diff_is_empty(&diff),
        "a dropped satisfy relationship is a genuine semantic change"
    );
    let satisfy_removed = diff
        .removed_elements
        .iter()
        .any(|element| element.element_id.starts_with("satisfy."))
        || !diff.removed_relationships.is_empty();
    assert!(
        satisfy_removed,
        "diff should surface the dropped satisfy: {diff:?}"
    );
}
