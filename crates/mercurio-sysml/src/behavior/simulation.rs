use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::behavior::{
    StateMachineModel, StateMachineScenarioEvent, TransitionNode, project_state_machines,
};
use mercurio_core::ir::{KirDocument, KirElement};
use mercurio_core::runtime::{ExecutionContext, Runtime, RuntimeError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationSubject {
    pub id: String,
    pub type_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HybridSimulationScenario {
    pub id: String,
    pub subject: SimulationSubject,
    pub machine_id: String,
    pub initial_state_id: Option<String>,
    pub events: Vec<StateMachineScenarioEvent>,
    pub max_steps: usize,
    pub values: BTreeMap<(String, String), Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HybridSimulationReport {
    pub scenario_id: String,
    pub subject_id: String,
    pub machine_id: String,
    pub status: HybridSimulationStatus,
    pub active_configuration: Vec<String>,
    pub values: BTreeMap<(String, String), Value>,
    pub critical_events: Vec<CriticalSimulationEvent>,
    pub trace: Vec<HybridSimulationTraceEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HybridSimulationStatus {
    Completed,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticalSimulationEvent {
    pub step: usize,
    pub kind: String,
    pub subject_id: String,
    pub detail: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HybridSimulationTraceEntry {
    pub step: usize,
    pub event_id: Option<String>,
    pub trigger: Option<String>,
    pub transition_id: Option<String>,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub critical_events: Vec<CriticalSimulationEvent>,
    pub explanation: String,
}

#[derive(Debug)]
pub enum SimulationError {
    MissingSubject(String),
    MissingStateMachine(String),
    MissingInitialState(String),
    MissingOverlayScenario,
    MissingOverlayTarget(String),
    InvalidOverlay(String),
    Runtime(RuntimeError),
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSubject(id) => write!(f, "missing simulation subject: {id}"),
            Self::MissingStateMachine(id) => write!(f, "missing state machine: {id}"),
            Self::MissingInitialState(id) => write!(f, "missing initial state: {id}"),
            Self::MissingOverlayScenario => write!(f, "simulation overlay is missing a scenario"),
            Self::MissingOverlayTarget(id) => {
                write!(f, "simulation overlay target not found: {id}")
            }
            Self::InvalidOverlay(message) => write!(f, "invalid simulation overlay: {message}"),
            Self::Runtime(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SimulationError {}

impl From<RuntimeError> for SimulationError {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

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
                StateMachineScenarioEvent {
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

pub fn run_hybrid_simulation(
    runtime: &Runtime,
    scenario: HybridSimulationScenario,
) -> Result<HybridSimulationReport, SimulationError> {
    if runtime
        .graph()
        .element_by_element_id(&scenario.subject.id)
        .is_none()
    {
        return Err(SimulationError::MissingSubject(scenario.subject.id));
    }

    let machines = project_state_machines(runtime);
    let machine = machines
        .iter()
        .find(|machine| machine.id == scenario.machine_id || machine.label == scenario.machine_id)
        .ok_or_else(|| SimulationError::MissingStateMachine(scenario.machine_id.clone()))?;

    let mut context = ExecutionContext {
        values: scenario
            .values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        version: 1,
    };
    let mut values = scenario.values.clone();
    let mut critical_events = Vec::new();
    let mut trace = Vec::new();

    critical_events.push(critical_event(
        0,
        "simulation.started",
        &scenario.subject.id,
        [("machine", Value::String(machine.id.clone()))],
    ));

    let mut active = initial_configuration(machine, scenario.initial_state_id.as_deref())
        .ok_or_else(|| SimulationError::MissingInitialState(machine.id.clone()))?;

    for (index, event) in scenario
        .events
        .iter()
        .take(scenario.max_steps.max(1))
        .enumerate()
    {
        let step = index + 1;
        let before = active.clone();
        let mut step_critical = vec![critical_event(
            step,
            "event.consumed",
            &scenario.subject.id,
            [
                ("event_id", Value::String(event.id.clone())),
                ("trigger", Value::String(event.trigger.clone())),
            ],
        )];

        let Some(transition) = select_transition(
            runtime,
            machine,
            &scenario.subject.id,
            &active,
            &event.trigger,
            &context,
        )?
        else {
            let entry = HybridSimulationTraceEntry {
                step,
                event_id: Some(event.id.clone()),
                trigger: Some(event.trigger.clone()),
                transition_id: None,
                before: before.clone(),
                after: before,
                critical_events: step_critical.clone(),
                explanation: format!("No enabled transition matched trigger `{}`.", event.trigger),
            };
            critical_events.append(&mut step_critical);
            trace.push(entry);
            return Ok(report(
                scenario,
                machine,
                HybridSimulationStatus::Blocked,
                active,
                values,
                critical_events,
                trace,
            ));
        };

        if let Some(guard_feature) = guard_feature_id(runtime, transition) {
            let result = runtime.evaluate(&guard_feature, &scenario.subject.id, &context)?;
            step_critical.push(critical_event(
                step,
                "guard.evaluated",
                &scenario.subject.id,
                [
                    ("feature", Value::String(guard_feature)),
                    ("result", result.value),
                ],
            ));
        }

        for effect in transition_effects(runtime, transition) {
            match effect {
                TransitionEffect::Assign { feature, value } => {
                    values.insert(
                        (scenario.subject.id.clone(), feature.clone()),
                        value.clone(),
                    );
                    context.values.insert(
                        (scenario.subject.id.clone(), feature.clone()),
                        value.clone(),
                    );
                    context.version += 1;
                    step_critical.push(critical_event(
                        step,
                        "effect.assigned",
                        &scenario.subject.id,
                        [("feature", Value::String(feature)), ("value", value)],
                    ));
                }
                TransitionEffect::Log { kind, detail } => {
                    step_critical.push(critical_event(step, &kind, &scenario.subject.id, detail));
                }
            }
        }

        active = initial_configuration(machine, Some(&transition.target))
            .ok_or_else(|| SimulationError::MissingInitialState(transition.target.clone()))?;
        step_critical.push(critical_event(
            step,
            "state.entered",
            &scenario.subject.id,
            [("state", Value::String(transition.target.clone()))],
        ));

        let entry = HybridSimulationTraceEntry {
            step,
            event_id: Some(event.id.clone()),
            trigger: Some(event.trigger.clone()),
            transition_id: Some(transition.id.clone()),
            before,
            after: active.clone(),
            critical_events: step_critical.clone(),
            explanation: format!(
                "Transition `{}` fired for subject `{}`.",
                transition.id, scenario.subject.id
            ),
        };
        critical_events.append(&mut step_critical);
        trace.push(entry);
    }

    Ok(report(
        scenario,
        machine,
        HybridSimulationStatus::Completed,
        active,
        values,
        critical_events,
        trace,
    ))
}

fn report(
    scenario: HybridSimulationScenario,
    machine: &StateMachineModel,
    status: HybridSimulationStatus,
    active_configuration: Vec<String>,
    values: BTreeMap<(String, String), Value>,
    critical_events: Vec<CriticalSimulationEvent>,
    trace: Vec<HybridSimulationTraceEntry>,
) -> HybridSimulationReport {
    HybridSimulationReport {
        scenario_id: scenario.id,
        subject_id: scenario.subject.id,
        machine_id: machine.id.clone(),
        status,
        active_configuration,
        values,
        critical_events,
        trace,
    }
}

fn initial_configuration(
    machine: &StateMachineModel,
    initial_state_id: Option<&str>,
) -> Option<Vec<String>> {
    let state_id = initial_state_id.map(ToOwned::to_owned).or_else(|| {
        machine
            .states
            .iter()
            .find(|state| state.parent_state_id.is_none() && state.is_initial)
            .map(|state| state.id.clone())
    })?;

    Some(vec![state_id])
}

fn select_transition<'a>(
    runtime: &Runtime,
    machine: &'a StateMachineModel,
    subject_id: &str,
    active_configuration: &[String],
    trigger: &str,
    context: &ExecutionContext,
) -> Result<Option<&'a TransitionNode>, SimulationError> {
    for state_id in active_configuration.iter().rev() {
        for transition in machine.transitions.iter().filter(|transition| {
            transition.source == *state_id && transition.trigger.as_deref() == Some(trigger)
        }) {
            let Some(guard_feature) = guard_feature_id(runtime, transition) else {
                return Ok(Some(transition));
            };
            let result = runtime.evaluate(&guard_feature, subject_id, context)?;
            if result.value.as_bool().unwrap_or(false) {
                return Ok(Some(transition));
            }
        }
    }
    Ok(None)
}

fn guard_feature_id(runtime: &Runtime, transition: &TransitionNode) -> Option<String> {
    runtime
        .graph()
        .element_by_element_id(&transition.id)
        .and_then(|element| element.properties.get("guard_feature"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn transition_effects(runtime: &Runtime, transition: &TransitionNode) -> Vec<TransitionEffect> {
    runtime
        .graph()
        .element_by_element_id(&transition.id)
        .and_then(|element| element.properties.get("effects"))
        .and_then(Value::as_array)
        .map(|effects| {
            effects
                .iter()
                .filter_map(TransitionEffect::from_value)
                .collect()
        })
        .unwrap_or_default()
}

fn critical_event<const N: usize>(
    step: usize,
    kind: &str,
    subject_id: &str,
    detail: [(&str, Value); N],
) -> CriticalSimulationEvent {
    CriticalSimulationEvent {
        step,
        kind: kind.to_string(),
        subject_id: subject_id.to_string(),
        detail: detail
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TransitionEffect {
    Assign {
        feature: String,
        value: Value,
    },
    Log {
        kind: String,
        detail: [(&'static str, Value); 1],
    },
}

impl TransitionEffect {
    fn from_value(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        match object.get("kind").and_then(Value::as_str)? {
            "assign" => Some(Self::Assign {
                feature: object.get("feature")?.as_str()?.to_string(),
                value: object.get("value")?.clone(),
            }),
            "log" => Some(Self::Log {
                kind: object.get("event")?.as_str()?.to_string(),
                detail: [(
                    "source",
                    object
                        .get("source")
                        .cloned()
                        .unwrap_or_else(|| Value::String("transition_effect".to_string())),
                )],
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    use mercurio_core::runtime::Runtime;
    use mercurio_core::{KirDocument, KirElement};

    use super::*;

    #[derive(Debug)]
    struct ModelSimulationHarness {
        text: String,
        output: Value,
    }

    #[test]
    fn runs_individual_hybrid_state_machine_with_parametric_guard_and_critical_events() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Vehicle", "Model::Systems::PartDefinition", []),
                element(
                    "individual.vehicle1",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("vehicle1")),
                        ("type", json!("type.Vehicle")),
                    ],
                ),
                element(
                    "feature.Vehicle.canStart",
                    "Model::CalculationUsage",
                    [
                        ("declared_name", json!("canStart")),
                        ("owner", json!("type.Vehicle")),
                        ("expression_ir", greater_equal_path("batteryVoltage", 12.0)),
                    ],
                ),
                element(
                    "state.Vehicle.Off",
                    "StateUsage",
                    [
                        ("declared_name", json!("Off")),
                        ("owning_type", json!("VehicleLifecycle")),
                        ("is_initial", json!(true)),
                    ],
                ),
                element(
                    "state.Vehicle.Starting",
                    "StateUsage",
                    [
                        ("declared_name", json!("Starting")),
                        ("owning_type", json!("VehicleLifecycle")),
                    ],
                ),
                element(
                    "state.Vehicle.Running",
                    "StateUsage",
                    [
                        ("declared_name", json!("Running")),
                        ("owning_type", json!("VehicleLifecycle")),
                    ],
                ),
                element(
                    "transition.Vehicle.start",
                    "TransitionUsage",
                    [
                        ("owning_type", json!("VehicleLifecycle")),
                        ("source", json!("state.Vehicle.Off")),
                        ("target", json!("state.Vehicle.Starting")),
                        ("trigger", json!("Start")),
                        ("trigger_kind", json!("event")),
                        ("guard_feature", json!("feature.Vehicle.canStart")),
                    ],
                ),
                element(
                    "transition.Vehicle.ready",
                    "TransitionUsage",
                    [
                        ("owning_type", json!("VehicleLifecycle")),
                        ("source", json!("state.Vehicle.Starting")),
                        ("target", json!("state.Vehicle.Running")),
                        ("trigger", json!("Ready")),
                        ("trigger_kind", json!("event")),
                        (
                            "effects",
                            json!([
                                {
                                    "kind": "assign",
                                    "feature": "motorReady",
                                    "value": true
                                },
                                {
                                    "kind": "assign",
                                    "feature": "driveEnabled",
                                    "value": true
                                },
                                {
                                    "kind": "log",
                                    "event": "hybrid.running.enabled",
                                    "source": "ready_transition"
                                }
                            ]),
                        ),
                    ],
                ),
            ],
        })
        .unwrap();

        let report = run_hybrid_simulation(
            &runtime,
            HybridSimulationScenario {
                id: "scenario.vehicle1.startup".to_string(),
                subject: SimulationSubject {
                    id: "individual.vehicle1".to_string(),
                    type_id: Some("type.Vehicle".to_string()),
                },
                machine_id: "VehicleLifecycle".to_string(),
                initial_state_id: None,
                events: vec![
                    StateMachineScenarioEvent {
                        id: "event.start".to_string(),
                        trigger: "Start".to_string(),
                    },
                    StateMachineScenarioEvent {
                        id: "event.ready".to_string(),
                        trigger: "Ready".to_string(),
                    },
                ],
                max_steps: 8,
                values: BTreeMap::from([
                    (
                        (
                            "individual.vehicle1".to_string(),
                            "batteryVoltage".to_string(),
                        ),
                        json!(12.4),
                    ),
                    (
                        ("individual.vehicle1".to_string(), "motorReady".to_string()),
                        json!(false),
                    ),
                    (
                        (
                            "individual.vehicle1".to_string(),
                            "driveEnabled".to_string(),
                        ),
                        json!(false),
                    ),
                ]),
            },
        )
        .unwrap();

        assert_eq!(report.status, HybridSimulationStatus::Completed);
        assert_eq!(report.subject_id, "individual.vehicle1");
        assert_eq!(report.active_configuration, vec!["state.Vehicle.Running"]);
        assert_eq!(
            report
                .values
                .get(&("individual.vehicle1".to_string(), "motorReady".to_string())),
            Some(&json!(true))
        );
        assert_eq!(
            report.values.get(&(
                "individual.vehicle1".to_string(),
                "driveEnabled".to_string()
            )),
            Some(&json!(true))
        );

        let critical_kinds = report
            .critical_events
            .iter()
            .map(|event| event.kind.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            critical_kinds,
            vec![
                "simulation.started",
                "event.consumed",
                "guard.evaluated",
                "state.entered",
                "event.consumed",
                "effect.assigned",
                "effect.assigned",
                "hybrid.running.enabled",
                "state.entered",
            ]
        );
        assert_eq!(
            report.trace[0].transition_id.as_deref(),
            Some("transition.Vehicle.start")
        );
        assert_eq!(
            report.trace[1].transition_id.as_deref(),
            Some("transition.Vehicle.ready")
        );
    }

    #[test]
    fn compiles_model_text_and_runs_individual_hybrid_simulation_harness() {
        let harness = run_model_simulation_harness();

        assert!(harness.text.contains("individual vehicle1"));
        assert_eq!(harness.output["subject_id"], json!("feature.Demo.vehicle1"));
        assert_eq!(harness.output["status"], json!("completed"));
        assert_eq!(
            harness.output["active_configuration"],
            json!(["state.Demo.Vehicle.VehicleLifecycle.Running"])
        );
        assert_eq!(
            harness.output["values"]["feature.Demo.vehicle1|driveEnabled"],
            json!(true)
        );
        assert_eq!(
            harness.output["critical_events"]
                .as_array()
                .unwrap()
                .iter()
                .map(|event| event["kind"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec![
                "simulation.started",
                "event.consumed",
                "guard.evaluated",
                "state.entered",
                "event.consumed",
                "effect.assigned",
                "effect.assigned",
                "hybrid.running.enabled",
                "state.entered",
            ]
        );
    }

    fn run_model_simulation_harness() -> ModelSimulationHarness {
        let text = r#"
package Demo {
    item def Start;
    item def Ready;

    part def Vehicle {
        attribute batteryVoltage;
        attribute motorReady;
        attribute driveEnabled;

        state VehicleLifecycle {
            entry; then Off;
            state Off;
            accept Start then Starting;
            state Starting;
            accept Ready then Running;
            state Running;
        }
    }

    individual vehicle1 : Vehicle;
}
"#
        .trim()
        .to_string();
        let subject_id = "feature.Demo.vehicle1".to_string();
        let start_transition = "transition.Demo.Vehicle.VehicleLifecycle.start".to_string();
        let ready_transition = "transition.Demo.Vehicle.VehicleLifecycle.ready".to_string();
        let document = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Demo.Vehicle", "Model::Systems::PartDefinition", []),
                element(
                    &subject_id,
                    "Model::Systems::PartUsage",
                    [
                        ("declared_name", json!("vehicle1")),
                        ("type", json!("type.Demo.Vehicle")),
                    ],
                ),
                element(
                    "state.Demo.Vehicle.VehicleLifecycle.Off",
                    "Model::Behavior::StateUsage",
                    [("owning_type", json!("VehicleLifecycle"))],
                ),
                element(
                    "state.Demo.Vehicle.VehicleLifecycle.Starting",
                    "Model::Behavior::StateUsage",
                    [("owning_type", json!("VehicleLifecycle"))],
                ),
                element(
                    "state.Demo.Vehicle.VehicleLifecycle.Running",
                    "Model::Behavior::StateUsage",
                    [("owning_type", json!("VehicleLifecycle"))],
                ),
                element(
                    &start_transition,
                    "Model::Behavior::TransitionUsage",
                    [
                        ("owning_type", json!("VehicleLifecycle")),
                        ("source", json!("state.Demo.Vehicle.VehicleLifecycle.Off")),
                        (
                            "target",
                            json!("state.Demo.Vehicle.VehicleLifecycle.Starting"),
                        ),
                        ("trigger", json!("Start")),
                    ],
                ),
                element(
                    &ready_transition,
                    "Model::Behavior::TransitionUsage",
                    [
                        ("owning_type", json!("VehicleLifecycle")),
                        (
                            "source",
                            json!("state.Demo.Vehicle.VehicleLifecycle.Starting"),
                        ),
                        (
                            "target",
                            json!("state.Demo.Vehicle.VehicleLifecycle.Running"),
                        ),
                        ("trigger", json!("Ready")),
                    ],
                ),
            ],
        };

        let overlay = KirDocument {
            metadata: BTreeMap::from([(
                "overlay_kind".to_string(),
                json!("Mercurio::Simulation::Overlay"),
            )]),
            elements: vec![
                element(
                    "simulation.scenario.vehicle1_model_startup",
                    "Mercurio::Simulation::Scenario",
                    [
                        ("subject", json!(subject_id)),
                        ("subject_type", json!("type.Demo.Vehicle")),
                        ("machine", json!("VehicleLifecycle")),
                        (
                            "initial_state",
                            json!("state.Demo.Vehicle.VehicleLifecycle.Off"),
                        ),
                        ("max_steps", json!(8)),
                    ],
                ),
                element(
                    "simulation.value.vehicle1.batteryVoltage",
                    "Mercurio::Simulation::InitialValue",
                    [
                        (
                            "scenario",
                            json!("simulation.scenario.vehicle1_model_startup"),
                        ),
                        ("owner", json!(subject_id)),
                        ("feature", json!("batteryVoltage")),
                        ("value", json!(12.4)),
                    ],
                ),
                element(
                    "simulation.value.vehicle1.motorReady",
                    "Mercurio::Simulation::InitialValue",
                    [
                        (
                            "scenario",
                            json!("simulation.scenario.vehicle1_model_startup"),
                        ),
                        ("owner", json!(subject_id)),
                        ("feature", json!("motorReady")),
                        ("value", json!(false)),
                    ],
                ),
                element(
                    "simulation.value.vehicle1.driveEnabled",
                    "Mercurio::Simulation::InitialValue",
                    [
                        (
                            "scenario",
                            json!("simulation.scenario.vehicle1_model_startup"),
                        ),
                        ("owner", json!(subject_id)),
                        ("feature", json!("driveEnabled")),
                        ("value", json!(false)),
                    ],
                ),
                element(
                    "simulation.event.vehicle1.start",
                    "Mercurio::Simulation::InputEvent",
                    [
                        (
                            "scenario",
                            json!("simulation.scenario.vehicle1_model_startup"),
                        ),
                        ("trigger", json!("Start")),
                        ("order", json!(1)),
                    ],
                ),
                element(
                    "simulation.event.vehicle1.ready",
                    "Mercurio::Simulation::InputEvent",
                    [
                        (
                            "scenario",
                            json!("simulation.scenario.vehicle1_model_startup"),
                        ),
                        ("trigger", json!("Ready")),
                        ("order", json!(2)),
                    ],
                ),
                element(
                    "feature.Demo.Vehicle.canStart",
                    "Model::CalculationUsage",
                    [
                        ("declared_name", json!("canStart")),
                        ("owner", json!("type.Demo.Vehicle")),
                        ("expression_ir", greater_equal_path("batteryVoltage", 12.0)),
                    ],
                ),
                element(
                    "simulation.patch.vehicle.start_guard",
                    "Mercurio::Simulation::TemporarySemanticPatch",
                    [
                        ("target", json!(start_transition)),
                        ("guard_feature", json!("feature.Demo.Vehicle.canStart")),
                    ],
                ),
                element(
                    "simulation.effect.vehicle.ready.motorReady",
                    "Mercurio::Simulation::TransitionEffect",
                    [
                        ("transition", json!(ready_transition)),
                        ("effect_kind", json!("assign")),
                        ("feature", json!("motorReady")),
                        ("value", json!(true)),
                    ],
                ),
                element(
                    "simulation.effect.vehicle.ready.driveEnabled",
                    "Mercurio::Simulation::TransitionEffect",
                    [
                        ("transition", json!(ready_transition)),
                        ("effect_kind", json!("assign")),
                        ("feature", json!("driveEnabled")),
                        ("value", json!(true)),
                    ],
                ),
                element(
                    "simulation.effect.vehicle.ready.log",
                    "Mercurio::Simulation::TransitionEffect",
                    [
                        ("transition", json!(ready_transition)),
                        ("effect_kind", json!("log")),
                        ("event", json!("hybrid.running.enabled")),
                    ],
                ),
            ],
        };

        let report = run_hybrid_simulation_with_overlay(document, overlay).unwrap();

        ModelSimulationHarness {
            text,
            output: harness_output(&report),
        }
    }

    fn element<const N: usize>(id: &str, kind: &str, properties: [(&str, Value); N]) -> KirElement {
        KirElement {
            id: id.to_string(),
            kind: kind.to_string(),
            layer: 0,
            properties: properties
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        }
    }

    fn harness_output(report: &HybridSimulationReport) -> Value {
        let values = report
            .values
            .iter()
            .map(|((owner, feature), value)| (format!("{owner}|{feature}"), value.clone()))
            .collect::<serde_json::Map<_, _>>();
        json!({
            "scenario_id": report.scenario_id,
            "subject_id": report.subject_id,
            "machine_id": report.machine_id,
            "status": report.status,
            "active_configuration": report.active_configuration,
            "values": values,
            "critical_events": report.critical_events,
            "trace": report.trace,
        })
    }

    fn greater_equal_path(feature: &str, threshold: f64) -> Value {
        json!({
            "kind": "binary",
            "op": "greater_equal",
            "left": {
                "kind": "path",
                "root": "self",
                "segments": [feature]
            },
            "right": {
                "kind": "literal",
                "value": threshold
            }
        })
    }
}
