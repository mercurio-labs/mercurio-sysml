use serde_json::Value;

use mercurio_core::runtime::Runtime;
use mercurio_simulation_core::{
    AssignEffect, LogEffect, SignalEffect, SimulationActionSequence, SimulationEffect,
    SimulationGuard, SimulationModel, SimulationRate, SimulationRateSource, SimulationState,
    SimulationStateMachine, SimulationTransition, SimulationTrigger, SimulationTriggerKind,
    StateDoBehavior, validate_simulation_model,
};
use mercurio_sysml::{
    StateMachineModel, StateTransitionTriggerKind, TransitionNode, project_state_machines,
};

#[derive(Debug)]
pub enum SysmlSimulationAdapterError {
    InvalidProfile(mercurio_simulation_core::SimulationProfileError),
}

impl From<mercurio_simulation_core::SimulationProfileError> for SysmlSimulationAdapterError {
    fn from(error: mercurio_simulation_core::SimulationProfileError) -> Self {
        Self::InvalidProfile(error)
    }
}

pub fn simulation_model_from_runtime(
    runtime: &Runtime,
) -> Result<SimulationModel, SysmlSimulationAdapterError> {
    let model = normalize_state_machines(project_state_machines(runtime));
    validate_simulation_model(&model)?;
    Ok(model)
}

pub fn normalize_state_machines(machines: Vec<StateMachineModel>) -> SimulationModel {
    SimulationModel {
        id: "sysml.projected".to_string(),
        machines: machines
            .iter()
            .map(normalize_state_machine)
            .collect::<Vec<_>>(),
    }
}

fn normalize_state_machine(machine: &StateMachineModel) -> SimulationStateMachine {
    SimulationStateMachine {
        id: machine.id.clone(),
        label: machine.label.clone(),
        states: machine.states.iter().map(normalize_state).collect(),
        transitions: machine
            .transitions
            .iter()
            .map(normalize_transition)
            .collect(),
    }
}

fn normalize_state(state: &mercurio_sysml::StateNode) -> SimulationState {
    SimulationState {
        id: state.id.clone(),
        label: state.label.clone(),
        parent_state_id: state.parent_state_id.clone(),
        is_initial: state.is_initial,
        is_final: state.is_final,
        entry_behavior: state
            .entry_behavior
            .as_ref()
            .and_then(normalize_action_sequence),
        exit_behavior: state
            .exit_behavior
            .as_ref()
            .and_then(normalize_action_sequence),
        do_behavior: state.do_behavior.as_ref().and_then(normalize_do_behavior),
    }
}

fn normalize_transition(transition: &TransitionNode) -> SimulationTransition {
    SimulationTransition {
        id: transition.id.clone(),
        source: transition.source.clone(),
        target: transition.target.clone(),
        trigger: SimulationTrigger {
            kind: normalize_trigger_kind(&transition.trigger_kind),
            value: transition.trigger.clone(),
        },
        guard: transition.guard.clone().map(SimulationGuard::ExpressionIr),
        effects: transition
            .effect
            .as_ref()
            .map(|effect| {
                vec![SimulationEffect::Log(LogEffect {
                    kind: "transition.effect".to_string(),
                    source: Some(effect.clone()),
                })]
            })
            .unwrap_or_default(),
    }
}

fn normalize_trigger_kind(kind: &StateTransitionTriggerKind) -> SimulationTriggerKind {
    match kind {
        StateTransitionTriggerKind::Event | StateTransitionTriggerKind::Unknown => {
            SimulationTriggerKind::Event
        }
        StateTransitionTriggerKind::Signal => SimulationTriggerKind::Signal,
        StateTransitionTriggerKind::Time => SimulationTriggerKind::Time,
        StateTransitionTriggerKind::After => SimulationTriggerKind::After,
        StateTransitionTriggerKind::Change => SimulationTriggerKind::Change,
        StateTransitionTriggerKind::Completion => SimulationTriggerKind::Completion,
    }
}

fn normalize_do_behavior(value: &Value) -> Option<StateDoBehavior> {
    let object = value.as_object()?;
    if object.get("kind").and_then(Value::as_str) != Some("rate_integration") {
        return None;
    }
    let rates = object
        .get("rates")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(normalize_rate)
        .collect::<Vec<_>>();
    Some(StateDoBehavior::RateIntegration { rates })
}

fn normalize_rate(value: &Value) -> Option<SimulationRate> {
    let object = value.as_object()?;
    let feature = object.get("feature")?.as_str()?.to_string();
    let source =
        if let Some(rate_per_second) = object.get("rate_per_second").and_then(Value::as_f64) {
            SimulationRateSource::Constant(rate_per_second)
        } else if let Some(rate_feature) = object.get("rate_feature").and_then(Value::as_str) {
            SimulationRateSource::Feature(rate_feature.to_string())
        } else {
            SimulationRateSource::ExpressionIr(object.get("rate_expr")?.clone())
        };
    Some(SimulationRate { feature, source })
}

fn normalize_action_sequence(value: &Value) -> Option<SimulationActionSequence> {
    let object = value.as_object()?;
    let actions = object
        .get("actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(normalize_effect)
        .collect::<Vec<_>>();
    Some(SimulationActionSequence { actions })
}

fn normalize_effect(value: &Value) -> Option<SimulationEffect> {
    let object = value.as_object()?;
    match object.get("kind").and_then(Value::as_str)? {
        "assign" => Some(SimulationEffect::Assign(AssignEffect {
            feature: object.get("feature")?.as_str()?.to_string(),
            value: object.get("value")?.clone(),
        })),
        "send_signal" => Some(SimulationEffect::EmitSignal(SignalEffect {
            signal_type: string_property_any(object, &["signal_type", "signal", "type"])?,
            target: string_property_any(object, &["target", "target_subject"])
                .filter(|target| target != "*"),
        })),
        "log" => Some(SimulationEffect::Log(LogEffect {
            kind: object.get("event")?.as_str()?.to_string(),
            source: object
                .get("source")
                .and_then(Value::as_str)
                .map(str::to_string),
        })),
        _ => None,
    }
}

fn string_property_any(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use mercurio_core::runtime::Runtime;
    use mercurio_core::{KirDocument, KirElement};
    use mercurio_simulation_core::{SimulationRateSource, StateDoBehavior};

    use super::*;

    #[test]
    fn normalizes_projected_state_machine_to_simulation_model() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element(
                    "state.Demo.Off",
                    "StateUsage",
                    [
                        ("owning_type", json!("DemoMachine")),
                        ("is_initial", json!(true)),
                    ],
                ),
                element(
                    "state.Demo.On",
                    "StateUsage",
                    [
                        ("owning_type", json!("DemoMachine")),
                        (
                            "do_behavior",
                            json!({
                                "kind": "rate_integration",
                                "rates": [{ "feature": "temperature", "rate_feature": "heatRate" }]
                            }),
                        ),
                    ],
                ),
                element(
                    "transition.Demo.start",
                    "TransitionUsage",
                    [
                        ("owning_type", json!("DemoMachine")),
                        ("source", json!("state.Demo.Off")),
                        ("target", json!("state.Demo.On")),
                        ("trigger", json!("start")),
                        ("trigger_kind", json!("event")),
                    ],
                ),
            ],
        })
        .unwrap();

        let model = simulation_model_from_runtime(&runtime).unwrap();
        let machine = model.machines.first().unwrap();
        assert_eq!(machine.id, "DemoMachine");
        assert_eq!(
            machine.transitions[0].trigger.kind,
            SimulationTriggerKind::Event
        );
        let on = machine
            .states
            .iter()
            .find(|state| state.id == "state.Demo.On")
            .unwrap();
        match on.do_behavior.as_ref().unwrap() {
            StateDoBehavior::RateIntegration { rates } => {
                assert_eq!(rates[0].feature, "temperature");
                assert_eq!(
                    rates[0].source,
                    SimulationRateSource::Feature("heatRate".to_string())
                );
            }
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
}
