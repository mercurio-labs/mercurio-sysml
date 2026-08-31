//! Manual perf probe for interactive semantic-service latency (single
//! legality checks, reasoning-context construction, scoped next-actions).
//! Not run in CI; run with:
//! `cargo test --test perf_probe -- --nocapture --ignored --test-threads 1`
//!
//! Reference debug-build numbers (2026-08-26, after the capability-profile
//! cache + legality batch work): single check ~6 ms, context ~0.6 ms,
//! next_actions over the whole fixture ~140 ms. Before that work the same
//! calls measured ~1.4 s, ~60 s, and minutes respectively.

use std::collections::BTreeMap;
use std::time::Instant;

use mercurio_foundation::{ElementRef, SemanticNextActionsRequest, WorkspaceRevision};
use mercurio_sysml::{
    load_authoring_project_from_sysml, sysml_semantic_legality_service_for_release,
    sysml_semantic_next_actions_service_for_release,
    sysml_semantic_reasoning_context_from_authoring_project,
};

/// Mirror of mercurio-examples/desktop/vehicle-mass-compliance (inlined so
/// this test compiles in isolated checkouts of this repo).
const VEHICLE_MASS: &str = r#"
package VehicleMassCompliance {
    import ScalarValues::*;

    part def Chassis {
        attribute mass_kg : Real = 400.0;
    }

    part def Battery {
        attribute mass_kg : Real = 550.0;
        attribute capacity_kwh : Real = 80.0;
    }

    part def Motor {
        attribute mass_kg : Real = 120.0;
        attribute power_kw : Real = 220.0;
    }

    part def Vehicle {
        attribute max_mass_kg : Real = 1500.0;

        part chassis : Chassis;
        part battery : Battery;
        part motor : Motor;
    }

    requirement def VehicleMassRequirement;

    analysis def VehicleMassComplianceAnalysis {
        subject vehicle : Vehicle;

        objective massRequirement : VehicleMassRequirement {
            subject = vehicle;
        }
    }
}
"#;

#[test]
#[ignore = "manual perf probe"]
fn probe_context_construction() {
    let project = load_authoring_project_from_sysml(BTreeMap::from([(
        "vehicle-mass-compliance.sysml".to_string(),
        VEHICLE_MASS.to_string(),
    )]))
    .expect("project parses");
    println!("project files: {}", project.files().count());

    let t = Instant::now();
    let context = sysml_semantic_reasoning_context_from_authoring_project(
        &project,
        WorkspaceRevision::unchecked(),
        Vec::new(),
        512,
    );
    println!(
        "context (no focus): {:?} ({} elements, {} rels, {} facts)",
        t.elapsed(),
        context.elements.len(),
        context.relationships.len(),
        context.facts.len()
    );

    let t = Instant::now();
    let context = sysml_semantic_reasoning_context_from_authoring_project(
        &project,
        WorkspaceRevision::unchecked(),
        vec![ElementRef::new("VehicleMassCompliance.Vehicle")],
        512,
    );
    println!(
        "context (focus): {:?} ({} elements, {} rels, {} facts)",
        t.elapsed(),
        context.elements.len(),
        context.relationships.len(),
        context.facts.len()
    );
}

#[test]
#[ignore = "manual perf probe"]
fn probe_next_actions_latency() {
    let t = Instant::now();
    let service = sysml_semantic_next_actions_service_for_release("latest").expect("service");
    println!("service construction: {:?}", t.elapsed());

    let t = Instant::now();
    let legality = sysml_semantic_legality_service_for_release("latest").expect("legality");
    println!("legality service construction: {:?}", t.elapsed());

    for round in 0..3 {
        let t = Instant::now();
        let report = legality.check(mercurio_foundation::SemanticLegalityRequest::relationship(
            "satisfy", "part", "part",
        ));
        println!(
            "single legality check #{round}: {:?} (status {:?})",
            t.elapsed(),
            report.status
        );
    }

    let t = Instant::now();
    let project = load_authoring_project_from_sysml(BTreeMap::from([(
        "vehicle-mass-compliance.sysml".to_string(),
        VEHICLE_MASS.to_string(),
    )]))
    .expect("project parses");
    println!("project parse: {:?}", t.elapsed());

    let t = Instant::now();
    let context = sysml_semantic_reasoning_context_from_authoring_project(
        &project,
        WorkspaceRevision::unchecked(),
        vec![ElementRef::new("VehicleMassCompliance.Vehicle")],
        512,
    );
    println!(
        "reasoning context: {:?} ({} elements, {} facts)",
        t.elapsed(),
        context.elements.len(),
        context.facts.len()
    );

    // Mimic scoped_next_actions_request_from_workspace: fill candidates from
    // the workspace context.
    let candidate_targets = context
        .elements
        .iter()
        .filter(|element| element.element.qualified_name != "VehicleMassCompliance.Vehicle")
        .map(|element| mercurio_foundation::SemanticNextActionTarget {
            element: ElementRef::new(element.element.qualified_name.clone()),
            kind: element
                .attributes
                .get("keyword")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(&element.kind)
                .to_string(),
            label: None,
        })
        .collect::<Vec<_>>();
    let candidate_target_kinds = candidate_targets
        .iter()
        .map(|target| target.kind.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let facts = context
        .facts
        .iter()
        .map(|fact| mercurio_foundation::Fact {
            predicate: fact.predicate.clone(),
            terms: fact.terms.clone(),
        })
        .collect::<Vec<_>>();
    println!(
        "candidates: {} targets, {} kinds, {} facts",
        candidate_targets.len(),
        candidate_target_kinds.len(),
        facts.len()
    );

    let request = SemanticNextActionsRequest {
        element: Some(ElementRef::new("VehicleMassCompliance.Vehicle")),
        element_kind: "part def".to_string(),
        candidate_target_kinds,
        candidate_targets,
        candidate_attributes: vec!["text".to_string()],
        facts,
        max_actions: Some(64),
    };

    for round in 0..3 {
        let t = Instant::now();
        let report = service.next_actions(request.clone());
        println!(
            "next_actions #{round}: {:?} ({} actions, truncated={})",
            t.elapsed(),
            report.actions.len(),
            report.truncated
        );
    }
}
