use std::collections::BTreeMap;

use mercurio_core::graph::Element;
use serde_json::Value;

use mercurio_core::runtime::Runtime;
use mercurio_simulation_core::{
    AnalysisCaseInfo, AssignEffect, ConcurrentSimulationScenario, ConcurrentSubjectScenario,
    LogEffect, SignalEffect, SimulationActionSequence, SimulationEffect, SimulationEvent,
    SimulationGuard, SimulationModel, SimulationObjective, SimulationRate, SimulationRateSource,
    SimulationRequirement, SimulationState, SimulationStateMachine, SimulationTransition,
    SimulationTrigger, SimulationTriggerKind, StateDoBehavior, validate_simulation_model,
};
use mercurio_sysml::{
    StateMachineModel, StateTransitionTriggerKind, TransitionNode, project_state_machines,
};

#[derive(Debug)]
pub enum SysmlSimulationAdapterError {
    InvalidProfile(mercurio_simulation_core::SimulationProfileError),
    MissingAnalysisCase(String),
    MissingStateMachine(String),
    InvalidAnalysisCase(String),
}

impl From<mercurio_simulation_core::SimulationProfileError> for SysmlSimulationAdapterError {
    fn from(error: mercurio_simulation_core::SimulationProfileError) -> Self {
        Self::InvalidProfile(error)
    }
}

pub fn simulation_model_from_runtime(
    runtime: &Runtime,
) -> Result<SimulationModel, SysmlSimulationAdapterError> {
    let model = normalize_state_machines_from_runtime(runtime, project_state_machines(runtime));
    validate_simulation_model(&model)?;
    Ok(model)
}

pub fn list_analysis_cases(runtime: &Runtime) -> Vec<AnalysisCaseInfo> {
    runtime
        .graph()
        .elements()
        .iter()
        .filter(|element| is_project_analysis_case(element))
        .map(|element| {
            let subject_count = element
                .properties
                .get("subjects")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or_else(|| native_analysis_subject_elements(runtime, element).len());
            AnalysisCaseInfo {
                id: element.element_id.clone(),
                label: element_label_element(element),
                subject_count,
            }
        })
        .collect()
}

pub fn scenario_from_analysis_case(
    runtime: &Runtime,
    analysis_case_id: &str,
) -> Result<ConcurrentSimulationScenario, SysmlSimulationAdapterError> {
    let analysis_case = runtime
        .graph()
        .elements()
        .iter()
        .find(|element| {
            is_project_analysis_case(element)
                && (element.element_id == analysis_case_id
                    || element_label_element(element) == analysis_case_id)
        })
        .ok_or_else(|| {
            SysmlSimulationAdapterError::MissingAnalysisCase(analysis_case_id.to_string())
        })?;

    let mut subjects = analysis_case
        .properties
        .get("subjects")
        .and_then(Value::as_array)
        .map(|subjects| {
            subjects
                .iter()
                .enumerate()
                .map(|(index, value)| subject_scenario_from_analysis_value(value, index))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .map(Ok)
        .unwrap_or_else(|| native_analysis_subjects(runtime, analysis_case))?;

    if subjects.is_empty() {
        return Err(SysmlSimulationAdapterError::InvalidAnalysisCase(format!(
            "{} must define at least one analysis subject",
            analysis_case.element_id
        )));
    }

    let mut initial_values = native_analysis_attribute_defaults(runtime, &subjects);
    initial_values.extend(
        analysis_case
            .properties
            .get("initial_values")
            .or_else(|| analysis_case.properties.get("initialValues"))
            .and_then(Value::as_object)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|(key, value)| {
                        let (subject, feature) = key.split_once('|')?;
                        Some(((subject.to_string(), feature.to_string()), value.clone()))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default(),
    );
    initial_values.extend(native_analysis_initial_values(
        runtime,
        analysis_case,
        &subjects,
    )?);
    apply_analysis_script_events(runtime, analysis_case, &mut subjects)?;
    let requirements = native_analysis_requirements(runtime, analysis_case);
    let objectives = native_analysis_objectives(runtime, analysis_case, &subjects);

    Ok(ConcurrentSimulationScenario {
        id: analysis_case.element_id.clone(),
        subjects,
        max_steps: analysis_case
            .properties
            .get("max_steps")
            .or_else(|| analysis_case.properties.get("maxSteps"))
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(300),
        step_duration_s: analysis_case
            .properties
            .get("step_duration_s")
            .or_else(|| analysis_case.properties.get("stepDurationS"))
            .and_then(Value::as_f64)
            .unwrap_or(1.0),
        initial_values,
        requirements,
        objectives,
    })
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

pub fn normalize_state_machines_from_runtime(
    runtime: &Runtime,
    machines: Vec<StateMachineModel>,
) -> SimulationModel {
    SimulationModel {
        id: "sysml.projected".to_string(),
        machines: machines
            .iter()
            .map(|machine| normalize_state_machine_from_runtime(runtime, machine))
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

fn normalize_state_machine_from_runtime(
    runtime: &Runtime,
    machine: &StateMachineModel,
) -> SimulationStateMachine {
    SimulationStateMachine {
        id: machine.id.clone(),
        label: machine.label.clone(),
        states: machine.states.iter().map(normalize_state).collect(),
        transitions: machine
            .transitions
            .iter()
            .map(|transition| normalize_transition_from_runtime(runtime, transition))
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
        is_orthogonal: state.is_orthogonal,
        is_history: state.is_history,
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
    normalize_transition_with_effects(transition, Vec::new())
}

fn normalize_transition_from_runtime(
    runtime: &Runtime,
    transition: &TransitionNode,
) -> SimulationTransition {
    let effects = runtime
        .graph()
        .element_by_element_id(&transition.id)
        .and_then(|element| element.properties.get("effects"))
        .and_then(Value::as_array)
        .map(|effects| {
            effects
                .iter()
                .filter_map(normalize_effect)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            transition
                .effect
                .as_ref()
                .map(|effect| {
                    vec![SimulationEffect::Log(LogEffect {
                        kind: "transition.effect".to_string(),
                        source: Some(effect.clone()),
                    })]
                })
                .unwrap_or_default()
        });
    normalize_transition_with_effects(transition, effects)
}

fn normalize_transition_with_effects(
    transition: &TransitionNode,
    effects: Vec<SimulationEffect>,
) -> SimulationTransition {
    SimulationTransition {
        id: transition.id.clone(),
        source: transition.source.clone(),
        target: transition.target.clone(),
        trigger: SimulationTrigger {
            kind: normalize_trigger_kind(&transition.trigger_kind),
            value: transition.trigger.clone(),
        },
        guard: transition.guard.clone().map(SimulationGuard::ExpressionIr),
        effects,
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

fn subject_scenario_from_analysis_value(
    value: &Value,
    index: usize,
) -> Result<ConcurrentSubjectScenario, SysmlSimulationAdapterError> {
    let object = value.as_object().ok_or_else(|| {
        SysmlSimulationAdapterError::InvalidAnalysisCase(format!(
            "analysis case subject at index {index} must be an object"
        ))
    })?;
    let subject_id = object
        .get("subject_id")
        .or_else(|| object.get("subjectId"))
        .or_else(|| object.get("subject"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            SysmlSimulationAdapterError::InvalidAnalysisCase(format!(
                "analysis case subject at index {index} must define `subject`"
            ))
        })?
        .to_string();
    let machine_id = object
        .get("machine_id")
        .or_else(|| object.get("machineId"))
        .or_else(|| object.get("machine"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            SysmlSimulationAdapterError::InvalidAnalysisCase(format!(
                "analysis case subject `{subject_id}` must define `machine`"
            ))
        })?
        .to_string();
    let initial_state_id = object
        .get("initial_state_id")
        .or_else(|| object.get("initialStateId"))
        .or_else(|| object.get("initial_state"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let events = object
        .get("events")
        .and_then(Value::as_array)
        .map(|events| {
            events
                .iter()
                .enumerate()
                .map(|(event_index, event)| {
                    let event_object = event.as_object().ok_or_else(|| {
                        SysmlSimulationAdapterError::InvalidAnalysisCase(format!(
                            "analysis case event {event_index} for `{subject_id}` must be an object"
                        ))
                    })?;
                    let trigger = event_object
                        .get("trigger")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            SysmlSimulationAdapterError::InvalidAnalysisCase(format!(
                                "analysis case event {event_index} for `{subject_id}` must define `trigger`"
                            ))
                        })?
                        .to_string();
                    let id = event_object
                        .get("id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| format!("{subject_id}.event.{event_index}"));
                    Ok(SimulationEvent { id, trigger })
                })
                .collect::<Result<Vec<_>, SysmlSimulationAdapterError>>()
        })
        .transpose()?
        .unwrap_or_default();

    Ok(ConcurrentSubjectScenario {
        subject_id,
        machine_id,
        initial_state_id,
        events,
    })
}

fn native_analysis_subjects(
    runtime: &Runtime,
    analysis_case: &Element,
) -> Result<Vec<ConcurrentSubjectScenario>, SysmlSimulationAdapterError> {
    let machines = project_state_machines(runtime);
    native_analysis_subject_elements(runtime, analysis_case)
        .into_iter()
        .map(|subject| {
            let subject_type = string_property_any_element(subject, &["type", "definition"])
                .ok_or_else(|| {
                    SysmlSimulationAdapterError::InvalidAnalysisCase(format!(
                        "{} must define `type` to infer a state machine",
                        subject.element_id
                    ))
                })?;
            let machine = machines
                .iter()
                .find(|machine| {
                    machine.states.iter().any(|state| {
                        state.parent_state_id.is_none() && state.owner_id == subject_type
                    })
                })
                .ok_or_else(|| {
                    SysmlSimulationAdapterError::MissingStateMachine(subject_type.clone())
                })?;
            Ok(ConcurrentSubjectScenario {
                subject_id: subject.element_id.clone(),
                machine_id: machine.id.clone(),
                initial_state_id: None,
                events: default_native_subject_events(machine),
            })
        })
        .collect()
}

fn default_native_subject_events(machine: &StateMachineModel) -> Vec<SimulationEvent> {
    if machine.transitions.iter().any(|transition| {
        transition.trigger_kind == StateTransitionTriggerKind::Event
            && transition.trigger.as_deref() == Some("start")
    }) {
        vec![SimulationEvent {
            id: format!("{}.event.start", machine.id),
            trigger: "start".to_string(),
        }]
    } else {
        Vec::new()
    }
}

fn native_analysis_subject_elements<'a>(
    runtime: &'a Runtime,
    analysis_case: &Element,
) -> Vec<&'a Element> {
    runtime
        .graph()
        .elements()
        .iter()
        .filter(|candidate| {
            candidate.element_id.starts_with("subject.")
                && string_property_any_element(candidate, &["owner", "owning_type"]).as_deref()
                    == Some(analysis_case.element_id.as_str())
        })
        .collect()
}

fn native_analysis_initial_values(
    runtime: &Runtime,
    analysis_case: &Element,
    subjects: &[ConcurrentSubjectScenario],
) -> Result<BTreeMap<(String, String), Value>, SysmlSimulationAdapterError> {
    let subject_aliases = native_analysis_subject_elements(runtime, analysis_case)
        .into_iter()
        .map(|subject| (element_label_element(subject), subject.element_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let default_subject = (subjects.len() == 1).then(|| subjects[0].subject_id.clone());
    let mut values = BTreeMap::new();

    for assume in runtime.graph().elements().iter().filter(|candidate| {
        candidate.element_id.starts_with("assume.")
            && string_property_any_element(candidate, &["owner", "owning_type"]).as_deref()
                == Some(analysis_case.element_id.as_str())
    }) {
        let Some(expression) = assume.properties.get("expression_ir") else {
            continue;
        };
        if let Some(((subject, feature), value)) = initial_value_from_assume_expression(
            expression,
            &subject_aliases,
            default_subject.as_deref(),
        ) {
            values.insert((subject, feature), value);
        }
    }

    Ok(values)
}

fn native_analysis_attribute_defaults(
    runtime: &Runtime,
    subjects: &[ConcurrentSubjectScenario],
) -> BTreeMap<(String, String), Value> {
    let mut values = BTreeMap::new();
    for subject in subjects {
        let Some(subject_element) = runtime.graph().element_by_element_id(&subject.subject_id)
        else {
            continue;
        };
        let Some(subject_type) =
            string_property_any_element(subject_element, &["type", "definition"])
        else {
            continue;
        };

        for attribute in runtime.graph().elements().iter().filter(|candidate| {
            candidate.kind.contains("AttributeUsage")
                && string_property_any_element(candidate, &["owner", "owning_type"]).as_deref()
                    == Some(subject_type.as_str())
        }) {
            let Some(feature) = string_property_any_element(attribute, &["declared_name", "name"])
            else {
                continue;
            };
            let Some(value) = attribute_default_value(attribute) else {
                continue;
            };
            values.insert((subject.subject_id.clone(), feature), value);
        }
    }
    values
}

fn apply_analysis_script_events(
    runtime: &Runtime,
    analysis_case: &Element,
    subjects: &mut [ConcurrentSubjectScenario],
) -> Result<(), SysmlSimulationAdapterError> {
    let Some(script_events) = analysis_script_events(analysis_case) else {
        return Ok(());
    };
    let mut aliases = subjects
        .iter()
        .map(|subject| (subject.subject_id.clone(), subject.subject_id.clone()))
        .collect::<BTreeMap<_, _>>();
    for subject in native_analysis_subject_elements(runtime, analysis_case) {
        aliases.insert(element_label_element(subject), subject.element_id.clone());
        aliases.insert(subject.element_id.clone(), subject.element_id.clone());
    }
    for subject in subjects.iter_mut() {
        subject.events.clear();
    }
    for (index, event) in script_events.iter().enumerate() {
        let object = event.as_object().ok_or_else(|| {
            SysmlSimulationAdapterError::InvalidAnalysisCase(format!(
                "SimulationScript event {index} must be an object"
            ))
        })?;
        let trigger =
            string_property_any(object, &["trigger", "event", "signal"]).ok_or_else(|| {
                SysmlSimulationAdapterError::InvalidAnalysisCase(format!(
                    "SimulationScript event {index} must define `trigger`"
                ))
            })?;
        let subject_ref = string_property_any(object, &["subject", "target", "subject_id"]);
        let subject_id = subject_ref
            .as_ref()
            .and_then(|subject| aliases.get(subject))
            .cloned()
            .or_else(|| (subjects.len() == 1).then(|| subjects[0].subject_id.clone()))
            .ok_or_else(|| {
                SysmlSimulationAdapterError::InvalidAnalysisCase(format!(
                    "SimulationScript event {index} must resolve a subject"
                ))
            })?;
        let subject = subjects
            .iter_mut()
            .find(|subject| subject.subject_id == subject_id)
            .ok_or_else(|| SysmlSimulationAdapterError::InvalidAnalysisCase(subject_id.clone()))?;
        let id = string_property_any(object, &["id", "name"])
            .unwrap_or_else(|| format!("{}.script.{index}", subject.subject_id));
        subject.events.push(SimulationEvent { id, trigger });
    }
    Ok(())
}

fn analysis_script_events(analysis_case: &Element) -> Option<&Vec<Value>> {
    analysis_case
        .properties
        .get("simulation_script")
        .or_else(|| analysis_case.properties.get("simulationScript"))
        .or_else(|| analysis_case.properties.get("events"))
        .and_then(Value::as_array)
        .or_else(|| {
            analysis_case
                .properties
                .get("metadata")?
                .get("SimulationScript")?
                .get("properties")?
                .get("events")?
                .as_array()
        })
}

fn native_analysis_requirements(
    runtime: &Runtime,
    analysis_case: &Element,
) -> Vec<SimulationRequirement> {
    runtime
        .graph()
        .elements()
        .iter()
        .filter(|candidate| {
            is_analysis_requirement(candidate)
                && string_property_any_element(candidate, &["owner", "owning_type"]).as_deref()
                    == Some(analysis_case.element_id.as_str())
        })
        .map(|requirement| SimulationRequirement {
            id: requirement.element_id.clone(),
            label: element_label_element(requirement),
            expression: requirement.properties.get("expression_ir").cloned(),
        })
        .collect()
}

fn native_analysis_objectives(
    runtime: &Runtime,
    analysis_case: &Element,
    subjects: &[ConcurrentSubjectScenario],
) -> Vec<SimulationObjective> {
    let subject_aliases = native_analysis_subject_elements(runtime, analysis_case)
        .into_iter()
        .flat_map(|subject| {
            [
                (element_label_element(subject), subject.element_id.clone()),
                (subject.element_id.clone(), subject.element_id.clone()),
            ]
        })
        .collect::<BTreeMap<_, _>>();
    let default_subject = (subjects.len() == 1).then(|| subjects[0].subject_id.clone());

    runtime
        .graph()
        .elements()
        .iter()
        .filter(|candidate| {
            is_analysis_objective(candidate)
                && string_property_any_element(candidate, &["owner", "owning_type"]).as_deref()
                    == Some(analysis_case.element_id.as_str())
        })
        .map(|objective| {
            let expression = objective
                .properties
                .get("expression_ir")
                .or_else(|| objective.properties.get("subject"))
                .cloned();
            let (subject, feature) = expression
                .as_ref()
                .and_then(|expression| {
                    objective_subject_feature(
                        expression,
                        &subject_aliases,
                        default_subject.as_deref(),
                    )
                })
                .unwrap_or((None, None));
            SimulationObjective {
                id: objective.element_id.clone(),
                label: element_label_element(objective),
                subject,
                feature,
                expression,
            }
        })
        .collect()
}

fn objective_subject_feature(
    expression: &Value,
    subject_aliases: &BTreeMap<String, String>,
    default_subject: Option<&str>,
) -> Option<(Option<String>, Option<String>)> {
    let path = if expression.get("kind").and_then(Value::as_str) == Some("path") {
        expression
    } else if expression
        .get("kind")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind == "literal")
    {
        return None;
    } else {
        expression.get("subject").unwrap_or(expression)
    };
    let names = path
        .get("segments")?
        .as_array()?
        .iter()
        .filter_map(expression_path_segment_name)
        .collect::<Vec<_>>();
    match names.as_slice() {
        [subject_name, feature @ ..] if !feature.is_empty() => Some((
            subject_aliases.get(subject_name).cloned(),
            Some(feature.join(".")),
        )),
        [feature] => Some((
            default_subject.map(ToOwned::to_owned),
            Some(feature.clone()),
        )),
        _ => None,
    }
}

fn is_analysis_requirement(element: &Element) -> bool {
    element.kind.contains("RequireUsage")
        || element.kind.contains("RequirementUsage")
        || element.element_id.starts_with("require.")
}

fn is_analysis_objective(element: &Element) -> bool {
    element.kind.contains("ObjectiveUsage") || element.element_id.starts_with("objective.")
}

fn attribute_default_value(attribute: &Element) -> Option<Value> {
    let expression = attribute.properties.get("expression_ir")?;
    let object = expression.as_object()?;
    (object.get("kind")?.as_str()? == "literal").then(|| object.get("value").cloned())?
}

fn initial_value_from_assume_expression(
    expression: &Value,
    subject_aliases: &BTreeMap<String, String>,
    default_subject: Option<&str>,
) -> Option<((String, String), Value)> {
    let object = expression.as_object()?;
    if object.get("kind")?.as_str()? != "binary" || object.get("op")?.as_str()? != "equal" {
        return None;
    }
    let left = object.get("left")?;
    let right = object.get("right")?;
    path_literal_initial_value(left, right, subject_aliases, default_subject)
        .or_else(|| path_literal_initial_value(right, left, subject_aliases, default_subject))
}

fn path_literal_initial_value(
    path: &Value,
    literal: &Value,
    subject_aliases: &BTreeMap<String, String>,
    default_subject: Option<&str>,
) -> Option<((String, String), Value)> {
    if path.get("kind")?.as_str()? != "path" || literal.get("kind")?.as_str()? != "literal" {
        return None;
    }
    let names = path
        .get("segments")?
        .as_array()?
        .iter()
        .filter_map(expression_path_segment_name)
        .collect::<Vec<_>>();
    let value = literal.get("value")?.clone();
    match names.as_slice() {
        [subject_name, feature @ ..] if !feature.is_empty() => {
            let subject = subject_aliases.get(subject_name)?.clone();
            Some(((subject, feature.join(".")), value))
        }
        [feature] => {
            let subject = default_subject?.to_string();
            Some(((subject, feature.clone()), value))
        }
        _ => None,
    }
}

fn expression_path_segment_name(segment: &Value) -> Option<String> {
    segment.as_str().map(ToOwned::to_owned).or_else(|| {
        segment
            .get("name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

fn is_project_analysis_case(element: &Element) -> bool {
    element.kind.contains("AnalysisCaseDefinition")
        && element.element_id != "AnalysisCases::AnalysisCase"
        && !element.element_id.starts_with("SysML::")
        && !string_property_any_element(element, &["source_file", "sourceFile"])
            .is_some_and(|source| source.starts_with("Systems Library/"))
}

fn element_label_element(element: &Element) -> String {
    element
        .properties
        .get("declared_name")
        .or_else(|| element.properties.get("name"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| element.element_id.clone())
}

fn string_property_any_element(element: &Element, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        element
            .properties
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
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

    #[test]
    fn extracts_native_analysis_case_scenario() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element(
                    "analysis.PrintSequence",
                    "AnalysisCaseDefinition",
                    [
                        ("declared_name", json!("PrintSequence")),
                        (
                            "simulation_script",
                            json!([
                                {
                                    "id": "script.start",
                                    "subject": "printer",
                                    "trigger": "start"
                                }
                            ]),
                        ),
                    ],
                ),
                element(
                    "subject.PrintSequence.printer",
                    "SubjectUsage",
                    [
                        ("owner", json!("analysis.PrintSequence")),
                        ("declared_name", json!("printer")),
                        ("type", json!("VoronPrinter")),
                    ],
                ),
                element(
                    "attribute.VoronPrinter.bed_temperature",
                    "AttributeUsage",
                    [
                        ("owner", json!("VoronPrinter")),
                        ("declared_name", json!("bed_temperature")),
                        (
                            "expression_ir",
                            json!({
                                "kind": "literal",
                                "value": 22
                            }),
                        ),
                    ],
                ),
                element(
                    "assume.PrintSequence.bed_temperature",
                    "AssumeUsage",
                    [
                        ("owner", json!("analysis.PrintSequence")),
                        (
                            "expression_ir",
                            json!({
                                "kind": "binary",
                                "op": "equal",
                                "left": {
                                    "kind": "path",
                                    "segments": ["printer", "bed_temperature"]
                                },
                                "right": {
                                    "kind": "literal",
                                    "value": 30
                                }
                            }),
                        ),
                    ],
                ),
                element(
                    "require.PrintSequence.printing",
                    "RequireUsage",
                    [
                        ("owner", json!("analysis.PrintSequence")),
                        ("declared_name", json!("PrinterEventuallyPrinting")),
                        (
                            "expression_ir",
                            json!({
                                "kind": "binary",
                                "op": "equal",
                                "left": {
                                    "kind": "path",
                                    "segments": ["printer", "state"]
                                },
                                "right": {
                                    "kind": "literal",
                                    "value": "Printing"
                                }
                            }),
                        ),
                    ],
                ),
                element(
                    "objective.PrintSequence.thermalProfile",
                    "ObjectiveUsage",
                    [
                        ("owner", json!("analysis.PrintSequence")),
                        ("declared_name", json!("thermalProfile")),
                        (
                            "expression_ir",
                            json!({
                                "kind": "path",
                                "segments": ["printer", "bed_temperature"]
                            }),
                        ),
                    ],
                ),
                element(
                    "state.VoronPrinter.idle",
                    "StateUsage",
                    [
                        ("owning_type", json!("VoronPrinter")),
                        ("is_initial", json!(true)),
                    ],
                ),
                element(
                    "state.VoronPrinter.homing",
                    "StateUsage",
                    [("owning_type", json!("VoronPrinter"))],
                ),
                element(
                    "transition.VoronPrinter.start",
                    "TransitionUsage",
                    [
                        ("owning_type", json!("VoronPrinter")),
                        ("source", json!("state.VoronPrinter.idle")),
                        ("target", json!("state.VoronPrinter.homing")),
                        ("trigger", json!("start")),
                        ("trigger_kind", json!("event")),
                    ],
                ),
            ],
        })
        .unwrap();

        let cases = list_analysis_cases(&runtime);
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].label, "PrintSequence");
        assert_eq!(cases[0].subject_count, 1);

        let scenario = scenario_from_analysis_case(&runtime, "PrintSequence").unwrap();
        assert_eq!(scenario.id, "analysis.PrintSequence");
        assert_eq!(scenario.subjects.len(), 1);
        assert_eq!(
            scenario.subjects[0].subject_id,
            "subject.PrintSequence.printer"
        );
        assert_eq!(scenario.subjects[0].machine_id, "VoronPrinter");
        assert_eq!(scenario.subjects[0].events[0].id, "script.start");
        assert_eq!(scenario.subjects[0].events[0].trigger, "start");
        assert_eq!(
            scenario.initial_values.get(&(
                "subject.PrintSequence.printer".to_string(),
                "bed_temperature".to_string()
            )),
            Some(&json!(30))
        );
        assert_eq!(scenario.requirements.len(), 1);
        assert_eq!(scenario.requirements[0].label, "PrinterEventuallyPrinting");
        assert!(scenario.requirements[0].expression.is_some());
        assert_eq!(scenario.objectives.len(), 1);
        assert_eq!(scenario.objectives[0].label, "thermalProfile");
        assert_eq!(
            scenario.objectives[0].subject.as_deref(),
            Some("subject.PrintSequence.printer")
        );
        assert_eq!(
            scenario.objectives[0].feature.as_deref(),
            Some("bed_temperature")
        );
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
