use std::collections::BTreeMap;

use mercurio_core::ir::{KirDocument, KirElement};
use mercurio_core::runtime::Runtime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    HybridSimulationReport, HybridSimulationScenario, SimulationError, SimulationEvent,
    SimulationSubject, run_hybrid_simulation,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationOverlayComposition {
    pub document: KirDocument,
    pub scenario: HybridSimulationScenario,
}

pub fn run_hybrid_simulation_with_overlay(
    base: KirDocument,
    overlay: KirDocument,
) -> Result<HybridSimulationReport, SimulationError> {
    let composition = compose_simulation_overlay(base, overlay)?;
    let runtime = Runtime::from_document(composition.document)?;
    run_hybrid_simulation(&runtime, composition.scenario)
}

pub fn compose_simulation_overlay(
    mut base: KirDocument,
    overlay: KirDocument,
) -> Result<SimulationOverlayComposition, SimulationError> {
    let scenario = scenario_from_overlay(&overlay)?;
    apply_overlay_patches(&mut base, &overlay)?;
    base.elements.extend(overlay.elements);
    Ok(SimulationOverlayComposition {
        document: base,
        scenario,
    })
}

fn scenario_from_overlay(
    overlay: &KirDocument,
) -> Result<HybridSimulationScenario, SimulationError> {
    let scenario_element = overlay
        .elements
        .iter()
        .find(|element| element.kind == "Mercurio::Simulation::Scenario")
        .ok_or(SimulationError::MissingOverlayScenario)?;
    let scenario_id = scenario_element.id.clone();
    let subject_id = required_string_property(scenario_element, "subject")?;
    let type_id = string_property(scenario_element, "subject_type");
    let machine_id = string_property(scenario_element, "machine")
        .or_else(|| string_property(scenario_element, "target"))
        .ok_or_else(|| {
            SimulationError::InvalidOverlay(format!(
                "{} must define `machine` or `target`",
                scenario_element.id
            ))
        })?;
    let initial_state_id = string_property(scenario_element, "initial_state");
    let max_steps = scenario_element
        .properties
        .get("max_steps")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(64);

    let mut events = overlay
        .elements
        .iter()
        .filter(|element| element.kind == "Mercurio::Simulation::InputEvent")
        .filter(|element| belongs_to_scenario(element, &scenario_id))
        .map(|element| {
            let order = element
                .properties
                .get("order")
                .and_then(Value::as_u64)
                .unwrap_or(u64::MAX);
            let trigger = required_string_property(element, "trigger")?;
            Ok((
                order,
                SimulationEvent {
                    id: element.id.clone(),
                    trigger,
                },
            ))
        })
        .collect::<Result<Vec<_>, SimulationError>>()?;
    events.sort_by_key(|(order, _)| *order);

    let values = overlay
        .elements
        .iter()
        .filter(|element| element.kind == "Mercurio::Simulation::InitialValue")
        .filter(|element| belongs_to_scenario(element, &scenario_id))
        .map(|element| {
            let owner = string_property(element, "owner").unwrap_or_else(|| subject_id.clone());
            let feature = required_string_property(element, "feature")?;
            let value = element.properties.get("value").cloned().ok_or_else(|| {
                SimulationError::InvalidOverlay(format!("{} must define `value`", element.id))
            })?;
            Ok(((owner, feature), value))
        })
        .collect::<Result<BTreeMap<_, _>, SimulationError>>()?;

    Ok(HybridSimulationScenario {
        id: scenario_id,
        subject: SimulationSubject {
            id: subject_id,
            type_id,
        },
        machine_id,
        initial_state_id,
        events: events.into_iter().map(|(_, event)| event).collect(),
        max_steps,
        values,
        step_duration_s: 1.0,
    })
}

fn apply_overlay_patches(
    base: &mut KirDocument,
    overlay: &KirDocument,
) -> Result<(), SimulationError> {
    for patch in overlay
        .elements
        .iter()
        .filter(|element| element.kind == "Mercurio::Simulation::TemporarySemanticPatch")
    {
        let target = required_string_property(patch, "target")?;
        let element = base
            .elements
            .iter_mut()
            .find(|element| element.id == target)
            .ok_or_else(|| SimulationError::MissingOverlayTarget(target.clone()))?;
        if let Some(guard_feature) = patch.properties.get("guard_feature").cloned() {
            element
                .properties
                .insert("guard_feature".to_string(), guard_feature);
        }
        if let Some(effects) = patch.properties.get("effects").cloned() {
            element.properties.insert("effects".to_string(), effects);
        }
    }

    for effect in overlay
        .elements
        .iter()
        .filter(|element| element.kind == "Mercurio::Simulation::TransitionEffect")
    {
        let transition_id = required_string_property(effect, "transition")?;
        let transition = base
            .elements
            .iter_mut()
            .find(|element| element.id == transition_id)
            .ok_or_else(|| SimulationError::MissingOverlayTarget(transition_id.clone()))?;
        let effect_value = transition_effect_overlay_value(effect)?;
        let effects = transition
            .properties
            .entry("effects".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        let Some(effects) = effects.as_array_mut() else {
            return Err(SimulationError::InvalidOverlay(format!(
                "{} has non-array `effects`",
                transition.id
            )));
        };
        effects.push(effect_value);
    }

    Ok(())
}

fn transition_effect_overlay_value(effect: &KirElement) -> Result<Value, SimulationError> {
    match required_string_property(effect, "effect_kind")?.as_str() {
        "assign" => Ok(serde_json::json!({
            "kind": "assign",
            "feature": required_string_property(effect, "feature")?,
            "value": effect.properties.get("value").cloned().ok_or_else(|| {
                SimulationError::InvalidOverlay(format!("{} must define `value`", effect.id))
            })?,
        })),
        "log" => Ok(serde_json::json!({
            "kind": "log",
            "event": required_string_property(effect, "event")?,
            "source": effect.id,
        })),
        "rate" => Err(SimulationError::InvalidOverlay(format!(
            "{} uses legacy transition rate effects; move rates to state `do_behavior`",
            effect.id
        ))),
        other => Err(SimulationError::InvalidOverlay(format!(
            "{} has unsupported effect_kind `{other}`",
            effect.id
        ))),
    }
}

fn belongs_to_scenario(element: &KirElement, scenario_id: &str) -> bool {
    string_property(element, "scenario").is_none_or(|scenario| scenario == scenario_id)
}

fn required_string_property(
    element: &KirElement,
    property: &str,
) -> Result<String, SimulationError> {
    string_property(element, property).ok_or_else(|| {
        SimulationError::InvalidOverlay(format!("{} must define `{property}`", element.id))
    })
}

fn string_property(element: &KirElement, property: &str) -> Option<String> {
    element
        .properties
        .get(property)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}
