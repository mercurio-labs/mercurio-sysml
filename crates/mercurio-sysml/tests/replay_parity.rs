//! DA-11 Tier-1 replay-harness integration tests (Track C, C2.1/C2.1b).
//!
//! The corpus-wide runs live in `mercurio-examples/rust/authoring-parity`;
//! these tests pin the harness behavior itself on an inline reference model:
//! derived gesture scripts rebuild the model through the production
//! check → apply pipeline, the result is oracle-equivalent, and constructs
//! outside the action space surface as Blocked steps (never silent drops).

use std::collections::BTreeMap;

use mercurio_sysml::load_authoring_project_from_sysml;
use mercurio_sysml::replay::{
    GESTURE_SCRIPT_SCHEMA_VERSION, GestureExpectation, derive_gestures, replay_gesture_script,
    run_authoring_parity,
};

/// Inline reference exercising the expressible action space: package with
/// imports, definitions with stdlib specialization, attributes with values,
/// nested part tree, state usages, and top-level usages.
const EXPRESSIBLE_REFERENCE: &str = r#"package Demo {
    import ScalarValues::*;

    part def Chassis {
        attribute mass_kg : Real = 400.0;
    }

    part def Vehicle {
        attribute max_mass_kg : Real = 1500.0;
        part chassis : Chassis;

        state lifecycle {
            state Idle;
            state Driving;
        }
    }

    analysis def MassCheck :> AnalysisCase {
        subject vehicle : Vehicle;

        objective massBudget;
    }

    part baselineVehicle : Vehicle;
}
"#;

/// Reference with constructs the action space cannot express (alias,
/// multi-line doc, state transition) — they must land as Blocked steps and
/// the rest of the model must still replay equivalent.
const PARTIALLY_BLOCKED_REFERENCE: &str = r#"package Demo {
    part def Vehicle {
        doc /*
         * A vehicle.
         * With a multi-line doc block.
         */
        state lifecycle {
            state Idle;
            state Driving;
            transition start first Idle accept start then Driving;
        }
    }
    alias Car for Vehicle;
    part vehicle : Vehicle;
}
"#;

fn files(source: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("model.sysml".to_string(), source.to_string())])
}

#[test]
fn derived_script_replays_expressible_reference_equivalently() {
    let outcome =
        run_authoring_parity("expressible", &files(EXPRESSIBLE_REFERENCE)).expect("parity runs");
    assert!(
        outcome.replay.blocked.is_empty(),
        "nothing should be blocked: {:?}",
        outcome.replay.blocked
    );
    assert!(
        !outcome.compared_against_pruned,
        "fully expressible models compare against the original text compile"
    );
    assert!(
        outcome.equivalence.equivalent,
        "replayed model must be oracle-equivalent, diff: {}",
        serde_json::to_string_pretty(&outcome.equivalence.diff).unwrap_or_default()
    );
    assert!(!outcome.replay.applied.is_empty());
}

#[test]
fn derived_script_is_schema_tagged_and_serializable() {
    let project =
        load_authoring_project_from_sysml(files(EXPRESSIBLE_REFERENCE)).expect("project loads");
    let derived = derive_gestures(&project, "expressible");
    assert_eq!(derived.script.schema_version, GESTURE_SCRIPT_SCHEMA_VERSION);

    let serialized = serde_json::to_string_pretty(&derived.script).expect("script serializes");
    let deserialized: mercurio_sysml::replay::GestureScript =
        serde_json::from_str(&serialized).expect("script deserializes");
    assert_eq!(deserialized, derived.script);

    // A deserialized script replays identically (the JSON form is the
    // interchange format for the replay bin and future Tier-2 fixtures).
    let report = replay_gesture_script(&deserialized).expect("replays");
    assert!(report.blocked.is_empty());
}

#[test]
fn blocked_constructs_surface_in_ledger_and_rest_of_model_replays() {
    let outcome = run_authoring_parity("partially-blocked", &files(PARTIALLY_BLOCKED_REFERENCE))
        .expect("parity runs");
    let constructs = outcome
        .replay
        .blocked
        .iter()
        .map(|blocked| blocked.construct.as_str())
        .collect::<Vec<_>>();
    assert!(
        constructs.contains(&"alias"),
        "alias must be ledgered: {constructs:?}"
    );
    assert!(
        constructs.contains(&"multi-line doc block"),
        "multi-line doc must be ledgered: {constructs:?}"
    );
    assert!(
        constructs.contains(&"state transition"),
        "transition must be ledgered: {constructs:?}"
    );
    assert!(outcome.compared_against_pruned);
    assert!(
        outcome.equivalence.equivalent,
        "the expressible remainder must replay equivalent, diff: {}",
        serde_json::to_string_pretty(&outcome.equivalence.diff).unwrap_or_default()
    );

    // The blocked steps are visible in the derived script too, as op-less
    // steps with the construct name.
    let project = load_authoring_project_from_sysml(files(PARTIALLY_BLOCKED_REFERENCE))
        .expect("project loads");
    let derived = derive_gestures(&project, "partially-blocked");
    let script_blocked = derived
        .script
        .steps
        .iter()
        .filter(|step| matches!(step.expect, GestureExpectation::Blocked { .. }))
        .count();
    assert_eq!(script_blocked, outcome.replay.blocked.len());
}
