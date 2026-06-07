use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use mercurio_core::ir::{KirDocument, KirElement};
use mercurio_core::runtime::{ExecutionContext, Runtime, RuntimeError};
use mercurio_language_contracts::expression::{
    ExpressionEvaluationContext, ExpressionEvaluationError, ExpressionIr, ExpressionPathSegment,
};
pub use mercurio_simulation_core::{
    AnalysisCaseInfo, ConcurrentSimulationScenario, ConcurrentSubjectScenario,
    CriticalSimulationEvent, HybridSimulationReport, HybridSimulationScenario,
    HybridSimulationStatus, HybridSimulationTraceEntry, SimulationClockConfig, SimulationEvent,
    SimulationModel, SimulationSubject, SimulationTrace, SimulationTriggerKind, TraceChannel,
    TraceChannelSource, TraceEntry, TraceEvent, run_concurrent_simulation_model,
};
use mercurio_sysml::{
    StateMachineModel, StateTransitionTriggerKind, TransitionNode, project_state_machines,
};

const RATE_SAMPLE_INTERVAL_S: f64 = 1.0;
const CHANGE_LOOP_LIMIT: usize = 20;
const CROSSING_TOLERANCE_S: f64 = 0.01;

pub type StateMachineScenarioEvent = SimulationEvent;

fn default_step_duration() -> f64 {
    1.0
}

#[derive(Debug)]
pub enum SimulationError {
    MissingAnalysisCase(String),
    MissingSubject(String),
    MissingStateMachine(String),
    MissingInitialState(String),
    MissingOverlayScenario,
    MissingOverlayTarget(String),
    InvalidOverlay(String),
    InvalidProfile(String),
    Runtime(RuntimeError),
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAnalysisCase(id) => write!(f, "missing analysis case: {id}"),
            Self::MissingSubject(id) => write!(f, "missing simulation subject: {id}"),
            Self::MissingStateMachine(id) => write!(f, "missing state machine: {id}"),
            Self::MissingInitialState(id) => write!(f, "missing initial state: {id}"),
            Self::MissingOverlayScenario => write!(f, "simulation overlay is missing a scenario"),
            Self::MissingOverlayTarget(id) => {
                write!(f, "simulation overlay target not found: {id}")
            }
            Self::InvalidOverlay(message) => write!(f, "invalid simulation overlay: {message}"),
            Self::InvalidProfile(message) => write!(f, "invalid simulation profile: {message}"),
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

pub fn canonical_simulation_model(runtime: &Runtime) -> Result<SimulationModel, SimulationError> {
    mercurio_sysml_simulation::simulation_model_from_runtime(runtime).map_err(map_adapter_error)
}

fn map_adapter_error(
    error: mercurio_sysml_simulation::SysmlSimulationAdapterError,
) -> SimulationError {
    match error {
        mercurio_sysml_simulation::SysmlSimulationAdapterError::InvalidProfile(error) => {
            SimulationError::InvalidProfile(
                error
                    .findings
                    .into_iter()
                    .map(|finding| format!("{}: {}", finding.code, finding.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        }
        mercurio_sysml_simulation::SysmlSimulationAdapterError::MissingAnalysisCase(id) => {
            SimulationError::MissingAnalysisCase(id)
        }
        mercurio_sysml_simulation::SysmlSimulationAdapterError::MissingStateMachine(id) => {
            SimulationError::MissingStateMachine(id)
        }
        mercurio_sysml_simulation::SysmlSimulationAdapterError::InvalidAnalysisCase(message) => {
            SimulationError::InvalidOverlay(message)
        }
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

pub fn list_analysis_cases(runtime: &Runtime) -> Vec<AnalysisCaseInfo> {
    mercurio_sysml_simulation::list_analysis_cases(runtime)
}

pub fn scenario_from_analysis_case(
    runtime: &Runtime,
    analysis_case_id: &str,
) -> Result<ConcurrentSimulationScenario, SimulationError> {
    mercurio_sysml_simulation::scenario_from_analysis_case(runtime, analysis_case_id)
        .map_err(map_adapter_error)
}

pub fn run_analysis_case(
    runtime: &Runtime,
    analysis_case_id: &str,
) -> Result<SimulationTrace, SimulationError> {
    let scenario = scenario_from_analysis_case(runtime, analysis_case_id)?;
    run_concurrent_simulation(runtime, scenario)
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
        step_duration_s: default_step_duration(),
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
        "rate" => Ok(serde_json::json!({
            "kind": "rate",
            "feature": required_string_property(effect, "feature")?,
            "rate_per_second": effect.properties.get("rate_per_second").and_then(Value::as_f64).ok_or_else(|| {
                SimulationError::InvalidOverlay(format!("{} must define numeric `rate_per_second`", effect.id))
            })?,
            "unit": effect.properties.get("unit").and_then(Value::as_str),
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
    let mut history = BTreeMap::<(String, String), String>::new();
    let mut t = 0.0;
    let mut step = 0usize;
    let mut rate_channels: BTreeSet<(String, String)> = BTreeSet::new();

    critical_events.push(critical_event(
        0,
        "simulation.started",
        &scenario.subject.id,
        [("machine", Value::String(machine.id.clone()))],
    ));

    let mut active = initial_configuration(machine, scenario.initial_state_id.as_deref())
        .ok_or_else(|| SimulationError::MissingInitialState(machine.id.clone()))?;
    for state_id in &active {
        apply_state_behavior(
            machine,
            state_id,
            StateBehaviorKind::Entry,
            &scenario.subject.id,
            0,
            &mut values,
            &mut context,
            &mut critical_events,
            None,
        );
    }

    let max_steps = scenario.max_steps.max(1);
    let mut event_index = 0usize;
    while step < max_steps {
        if event_index >= scenario.events.len() {
            if !fire_after_transitions(
                runtime,
                machine,
                &scenario.subject.id,
                &mut active,
                &mut values,
                &mut context,
                &mut critical_events,
                &mut trace,
                &mut t,
                &mut step,
                scenario.step_duration_s,
                max_steps,
                &mut rate_channels,
                &mut history,
            )? {
                break;
            }
            continue;
        }

        let event = &scenario.events[event_index];
        event_index += 1;
        step += 1;
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
                t,
                values: values.clone(),
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
                rate_channels,
            ));
        };

        t += scenario.step_duration_s;
        append_guard_evaluation(
            runtime,
            transition,
            &scenario.subject.id,
            &context,
            step,
            &mut step_critical,
        )?;
        apply_transition_effects(
            runtime,
            transition,
            &scenario.subject.id,
            step,
            &mut values,
            &mut context,
            &mut step_critical,
        );

        active = apply_transition_state_change(
            machine,
            &scenario.subject.id,
            &before,
            &transition.source,
            &transition.target,
            step,
            &mut values,
            &mut context,
            &mut step_critical,
            None,
            Some(&mut history),
        )?;

        let entry = HybridSimulationTraceEntry {
            step,
            event_id: Some(event.id.clone()),
            trigger: Some(event.trigger.clone()),
            transition_id: Some(transition.id.clone()),
            before,
            after: active.clone(),
            t,
            values: values.clone(),
            critical_events: step_critical.clone(),
            explanation: format!(
                "Transition `{}` fired for subject `{}`.",
                transition.id, scenario.subject.id
            ),
        };
        critical_events.append(&mut step_critical);
        trace.push(entry);

        fire_change_transitions(
            runtime,
            machine,
            &scenario.subject.id,
            &mut active,
            &mut values,
            &mut context,
            &mut critical_events,
            &mut trace,
            t,
            step,
            &mut history,
        )?;
        // After transitions only fire once all queued events are consumed;
        // they are driven in the events-exhausted branch at the top of the loop.
    }

    Ok(report(
        scenario,
        machine,
        HybridSimulationStatus::Completed,
        active,
        values,
        critical_events,
        trace,
        rate_channels,
    ))
}

struct SubjectRunState<'m> {
    subject_id: String,
    machine: &'m StateMachineModel,
    active: Vec<String>,
    event_index: usize,
    events: Vec<StateMachineScenarioEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingSignal {
    source_subject_id: String,
    signal_type: String,
    target: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct ActiveStateRate {
    subject_id: String,
    feature: String,
    source: ActiveStateRateSource,
}

#[derive(Debug, Clone, PartialEq)]
enum ActiveStateRateSource {
    Constant(f64),
    Feature(String),
    Expression(Value),
}

#[derive(Debug, Clone, PartialEq)]
struct ChangeCrossing {
    subject_index: usize,
    offset: f64,
    transition: TransitionNode,
}

fn make_concurrent_entry(
    t: f64,
    subjects: &[SubjectRunState<'_>],
    values: &BTreeMap<(String, String), Value>,
    fired_events: Vec<TraceEvent>,
) -> TraceEntry {
    let states = subjects
        .iter()
        .map(|subject| (subject.subject_id.clone(), subject.active.clone()))
        .collect();
    TraceEntry {
        t,
        states,
        values: values.clone(),
        events: fired_events,
    }
}

fn apply_concurrent_rate_samples(
    runtime: &Runtime,
    transition: &TransitionNode,
    subject_id: &str,
    subjects: &[SubjectRunState<'_>],
    values: &mut BTreeMap<(String, String), Value>,
    context: &mut ExecutionContext,
    timeline: &mut Vec<TraceEntry>,
    t_enter: f64,
    duration: f64,
    rate_channels: &mut BTreeSet<(String, String)>,
) {
    let rates = transition_effects(runtime, transition)
        .into_iter()
        .filter_map(|effect| match effect {
            TransitionEffect::Rate {
                feature,
                rate_per_second,
                unit: _,
            } => Some((feature, rate_per_second)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if rates.is_empty() {
        return;
    }

    for (feature, _) in &rates {
        rate_channels.insert((subject_id.to_string(), feature.clone()));
    }

    let initial_values = rates
        .iter()
        .map(|(feature, _)| {
            let key = (subject_id.to_string(), feature.clone());
            let initial = values.get(&key).and_then(Value::as_f64).unwrap_or(0.0);
            (feature.clone(), initial)
        })
        .collect::<BTreeMap<_, _>>();

    let mut offset = RATE_SAMPLE_INTERVAL_S;
    while offset < duration {
        for (feature, rate) in &rates {
            let initial = initial_values.get(feature).copied().unwrap_or(0.0);
            let value = initial + rate * offset;
            let key = (subject_id.to_string(), feature.clone());
            values.insert(key.clone(), Value::from(value));
            context.values.insert(key, Value::from(value));
        }
        context.version += 1;
        timeline.push(make_concurrent_entry(
            t_enter + offset,
            subjects,
            values,
            vec![TraceEvent {
                kind: "rate_sample".to_string(),
                transition_id: None,
                trigger: Some("rate_sample".to_string()),
            }],
        ));
        offset += RATE_SAMPLE_INTERVAL_S;
    }

    for (feature, rate) in &rates {
        let initial = initial_values.get(feature).copied().unwrap_or(0.0);
        let value = initial + rate * duration;
        let key = (subject_id.to_string(), feature.clone());
        values.insert(key.clone(), Value::from(value));
        context.values.insert(key, Value::from(value));
    }
    context.version += 1;
}

fn active_state_rates(
    subjects: &[SubjectRunState<'_>],
    _values: &BTreeMap<(String, String), Value>,
) -> Vec<ActiveStateRate> {
    let mut rates = Vec::new();
    for subject in subjects {
        for state_id in &subject.active {
            let Some(state) = subject
                .machine
                .states
                .iter()
                .find(|state| state.id == *state_id)
            else {
                continue;
            };
            rates.extend(state_do_rates(state, &subject.subject_id));
        }
    }
    rates
}

fn state_do_rates(state: &mercurio_sysml::StateNode, subject_id: &str) -> Vec<ActiveStateRate> {
    let Some(object) = state.do_behavior.as_ref().and_then(Value::as_object) else {
        return Vec::new();
    };
    if object.get("kind").and_then(Value::as_str) != Some("rate_integration") {
        return Vec::new();
    }

    object
        .get("rates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|rate| {
            let rate = rate.as_object()?;
            let feature = rate.get("feature")?.as_str()?.to_string();
            let source = if let Some(rate_per_second) =
                rate.get("rate_per_second").and_then(Value::as_f64)
            {
                ActiveStateRateSource::Constant(rate_per_second)
            } else if let Some(rate_feature) = rate.get("rate_feature").and_then(Value::as_str) {
                ActiveStateRateSource::Feature(rate_feature.to_string())
            } else if let Some(rate_expr) = rate.get("rate_expr") {
                ActiveStateRateSource::Expression(rate_expr.clone())
            } else {
                return None;
            };
            Some(ActiveStateRate {
                subject_id: subject_id.to_string(),
                feature,
                source,
            })
        })
        .collect()
}

fn integrate_active_state_behaviors(
    rates: &[ActiveStateRate],
    values: &mut BTreeMap<(String, String), Value>,
    context: &mut ExecutionContext,
    duration: f64,
    rate_channels: &mut BTreeSet<(String, String)>,
) {
    for rate in rates {
        rate_channels.insert((rate.subject_id.clone(), rate.feature.clone()));
    }
    let base_values = values.clone();
    apply_rates_at_offset(rates, &base_values, values, context, duration);
}

fn context_with_rates_at_offset(
    context: &ExecutionContext,
    values: &BTreeMap<(String, String), Value>,
    rates: &[ActiveStateRate],
    offset: f64,
) -> ExecutionContext {
    let mut scratch = context.clone();
    for (key, value) in integrated_rate_values_at_offset(rates, values, offset) {
        scratch.values.insert(key, Value::from(value));
    }
    scratch.version += 1;
    scratch
}

fn apply_rates_at_offset(
    rates: &[ActiveStateRate],
    base_values: &BTreeMap<(String, String), Value>,
    values: &mut BTreeMap<(String, String), Value>,
    context: &mut ExecutionContext,
    offset: f64,
) {
    for (key, integrated) in integrated_rate_values_at_offset(rates, base_values, offset) {
        let value = Value::from(integrated);
        values.insert(key.clone(), value.clone());
        context.values.insert(key, value);
    }
    context.version += 1;
}

fn integrated_rate_values_at_offset(
    rates: &[ActiveStateRate],
    base_values: &BTreeMap<(String, String), Value>,
    offset: f64,
) -> BTreeMap<(String, String), f64> {
    let mut result = BTreeMap::new();
    if offset <= 0.0 {
        for rate in rates {
            let key = (rate.subject_id.clone(), rate.feature.clone());
            result.insert(
                key.clone(),
                base_values.get(&key).and_then(Value::as_f64).unwrap_or(0.0),
            );
        }
        return result;
    }

    let base_numbers = rates
        .iter()
        .map(|rate| {
            let key = (rate.subject_id.clone(), rate.feature.clone());
            base_values.get(&key).and_then(Value::as_f64).unwrap_or(0.0)
        })
        .collect::<Vec<_>>();
    let k1 = rate_derivatives(rates, base_values);
    let k2_values = stage_values(base_values, rates, &base_numbers, &k1, offset * 0.5);
    let k2 = rate_derivatives(rates, &k2_values);
    let k3_values = stage_values(base_values, rates, &base_numbers, &k2, offset * 0.5);
    let k3 = rate_derivatives(rates, &k3_values);
    let k4_values = stage_values(base_values, rates, &base_numbers, &k3, offset);
    let k4 = rate_derivatives(rates, &k4_values);

    for (index, rate) in rates.iter().enumerate() {
        let integrated = base_numbers[index]
            + offset * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]) / 6.0;
        result.insert((rate.subject_id.clone(), rate.feature.clone()), integrated);
    }
    result
}

fn stage_values(
    base_values: &BTreeMap<(String, String), Value>,
    rates: &[ActiveStateRate],
    base_numbers: &[f64],
    derivatives: &[f64],
    offset: f64,
) -> BTreeMap<(String, String), Value> {
    let mut values = base_values.clone();
    for (index, rate) in rates.iter().enumerate() {
        values.insert(
            (rate.subject_id.clone(), rate.feature.clone()),
            Value::from(base_numbers[index] + derivatives[index] * offset),
        );
    }
    values
}

fn rate_derivatives(
    rates: &[ActiveStateRate],
    values: &BTreeMap<(String, String), Value>,
) -> Vec<f64> {
    rates
        .iter()
        .map(|rate| rate_value(rate, values).unwrap_or(0.0))
        .collect()
}

fn rate_value(rate: &ActiveStateRate, values: &BTreeMap<(String, String), Value>) -> Option<f64> {
    match &rate.source {
        ActiveStateRateSource::Constant(value) => Some(*value),
        ActiveStateRateSource::Feature(feature) => values
            .get(&(rate.subject_id.clone(), feature.clone()))
            .and_then(Value::as_f64),
        ActiveStateRateSource::Expression(expression) => {
            evaluate_rate_expression(expression, &rate.subject_id, values)
        }
    }
}

fn evaluate_rate_expression(
    expression: &Value,
    subject_id: &str,
    values: &BTreeMap<(String, String), Value>,
) -> Option<f64> {
    let expression = ExpressionIr::from_value(expression).ok()?;
    let mut context = RateExpressionContext { subject_id, values };
    expression.evaluate(&mut context).ok()?.as_f64()
}

struct RateExpressionContext<'a> {
    subject_id: &'a str,
    values: &'a BTreeMap<(String, String), Value>,
}

impl ExpressionEvaluationContext for RateExpressionContext<'_> {
    fn owner_id(&self) -> &str {
        self.subject_id
    }

    fn resolve_path(
        &mut self,
        segments: &[ExpressionPathSegment],
    ) -> Result<Vec<Value>, ExpressionEvaluationError> {
        let feature = segments
            .iter()
            .map(ExpressionPathSegment::name)
            .collect::<Vec<_>>()
            .join(".");
        self.values
            .get(&(self.subject_id.to_string(), feature.clone()))
            .cloned()
            .map(|value| vec![value])
            .ok_or(ExpressionEvaluationError::NonNumericValue {
                owner: self.subject_id.to_string(),
                feature,
            })
    }
}

fn earliest_change_crossing(
    runtime: &Runtime,
    subjects: &[SubjectRunState<'_>],
    subject_ids: &[String],
    context: &ExecutionContext,
    values: &BTreeMap<(String, String), Value>,
    rates: &[ActiveStateRate],
    duration: f64,
) -> Result<Option<ChangeCrossing>, SimulationError> {
    let mut earliest: Option<ChangeCrossing> = None;
    for (idx, subject) in subjects.iter().enumerate() {
        let end_context = context_with_rates_at_offset(context, values, rates, duration);
        let Some(end_transition) = select_concurrent_change_transition(
            runtime,
            subject.machine,
            &subject.subject_id,
            &subject.active,
            subject_ids,
            &end_context,
        )?
        .cloned() else {
            continue;
        };

        let mut low = 0.0;
        let mut high = duration;
        let mut transition = end_transition;
        while high - low > CROSSING_TOLERANCE_S {
            let mid = (low + high) / 2.0;
            let mid_context = context_with_rates_at_offset(context, values, rates, mid);
            if let Some(mid_transition) = select_concurrent_change_transition(
                runtime,
                subject.machine,
                &subject.subject_id,
                &subject.active,
                subject_ids,
                &mid_context,
            )?
            .cloned()
            {
                high = mid;
                transition = mid_transition;
            } else {
                low = mid;
            }
        }

        let crossing = ChangeCrossing {
            subject_index: idx,
            offset: high,
            transition,
        };
        if earliest
            .as_ref()
            .is_none_or(|current| crossing.offset < current.offset)
        {
            earliest = Some(crossing);
        }
    }
    Ok(earliest)
}

pub fn run_concurrent_simulation(
    runtime: &Runtime,
    scenario: ConcurrentSimulationScenario,
) -> Result<SimulationTrace, SimulationError> {
    if let Some(trace) = try_run_canonical_core(runtime, &scenario)? {
        return Ok(trace);
    }

    let all_machines = project_state_machines(runtime);
    let mut subjects = Vec::<SubjectRunState<'_>>::new();
    for subject in &scenario.subjects {
        if runtime
            .graph()
            .element_by_element_id(&subject.subject_id)
            .is_none()
        {
            return Err(SimulationError::MissingSubject(subject.subject_id.clone()));
        }
        let machine = all_machines
            .iter()
            .find(|machine| machine.id == subject.machine_id || machine.label == subject.machine_id)
            .ok_or_else(|| SimulationError::MissingStateMachine(subject.machine_id.clone()))?;
        let active = initial_configuration(machine, subject.initial_state_id.as_deref())
            .ok_or_else(|| SimulationError::MissingInitialState(subject.machine_id.clone()))?;
        subjects.push(SubjectRunState {
            subject_id: subject.subject_id.clone(),
            machine,
            active,
            event_index: 0,
            events: subject.events.clone(),
        });
    }

    let mut values = scenario.initial_values.clone();
    let mut context = ExecutionContext {
        values: values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        version: 1,
    };
    let mut t = 0.0f64;
    let mut step = 0usize;
    let mut rate_channels = BTreeSet::<(String, String)>::new();
    let mut pending_signals = VecDeque::<PendingSignal>::new();
    let mut history = BTreeMap::<(String, String), String>::new();
    let mut initial_critical = Vec::new();
    for subject in &subjects {
        for state_id in &subject.active {
            apply_state_behavior(
                subject.machine,
                state_id,
                StateBehaviorKind::Entry,
                &subject.subject_id,
                0,
                &mut values,
                &mut context,
                &mut initial_critical,
                Some(&mut pending_signals),
            );
        }
    }
    let mut timeline = vec![make_concurrent_entry(t, &subjects, &values, Vec::new())];
    let max_steps = scenario.max_steps.max(1);

    while step < max_steps {
        let mut fired_this_round = false;
        let mut round_events = Vec::<TraceEvent>::new();

        if deliver_pending_signals(
            runtime,
            &mut subjects,
            &mut values,
            &mut context,
            &mut pending_signals,
            &mut history,
            &mut step,
            max_steps,
            &mut round_events,
        )? {
            fired_this_round = true;
        }

        for subject in subjects.iter_mut() {
            if step >= max_steps || subject.event_index >= subject.events.len() {
                continue;
            }

            let event = subject.events[subject.event_index].clone();
            let transition = select_transition(
                runtime,
                subject.machine,
                &subject.subject_id,
                &subject.active,
                &event.trigger,
                &context,
            )?
            .cloned();

            subject.event_index += 1;
            let Some(transition) = transition else {
                continue;
            };

            t += scenario.step_duration_s;
            step += 1;
            let mut step_critical = Vec::new();
            append_guard_evaluation(
                runtime,
                &transition,
                &subject.subject_id,
                &context,
                step,
                &mut step_critical,
            )?;
            apply_transition_effects(
                runtime,
                &transition,
                &subject.subject_id,
                step,
                &mut values,
                &mut context,
                &mut step_critical,
            );
            enqueue_transition_signals(
                runtime,
                &transition,
                &subject.subject_id,
                &mut pending_signals,
            );
            let before = subject.active.clone();
            subject.active = apply_transition_state_change(
                subject.machine,
                &subject.subject_id,
                &before,
                &transition.source,
                &transition.target,
                step,
                &mut values,
                &mut context,
                &mut step_critical,
                Some(&mut pending_signals),
                Some(&mut history),
            )?;
            round_events.push(TraceEvent {
                kind: "transition".to_string(),
                transition_id: Some(transition.id),
                trigger: Some(event.trigger),
            });
            fired_this_round = true;
        }

        for idx in 0..subjects.len() {
            if step >= max_steps {
                break;
            }

            let transition = {
                let subject = &subjects[idx];
                subject
                    .active
                    .iter()
                    .rev()
                    .flat_map(|state_id| {
                        subject
                            .machine
                            .transitions
                            .iter()
                            .filter(move |transition| {
                                transition.source == *state_id
                                    && transition.trigger_kind == StateTransitionTriggerKind::After
                            })
                    })
                    .next()
                    .cloned()
            };
            let Some(transition) = transition else {
                continue;
            };
            let Some(duration) = transition
                .trigger
                .as_deref()
                .and_then(|trigger| trigger.parse::<f64>().ok())
                .filter(|duration| duration.is_finite() && *duration >= 0.0)
            else {
                continue;
            };
            let remaining = max_steps.saturating_sub(step);
            if duration > scenario.step_duration_s * remaining as f64 {
                continue;
            }

            step += 1;
            let t_enter = t;
            let subject_id = subjects[idx].subject_id.clone();
            apply_concurrent_rate_samples(
                runtime,
                &transition,
                &subject_id,
                &subjects,
                &mut values,
                &mut context,
                &mut timeline,
                t_enter,
                duration,
                &mut rate_channels,
            );
            t = t_enter + duration;

            let mut step_critical = Vec::new();
            apply_transition_effects(
                runtime,
                &transition,
                &subject_id,
                step,
                &mut values,
                &mut context,
                &mut step_critical,
            );
            enqueue_transition_signals(runtime, &transition, &subject_id, &mut pending_signals);
            let before = subjects[idx].active.clone();
            subjects[idx].active = apply_transition_state_change(
                subjects[idx].machine,
                &subject_id,
                &before,
                &transition.source,
                &transition.target,
                step,
                &mut values,
                &mut context,
                &mut step_critical,
                Some(&mut pending_signals),
                Some(&mut history),
            )?;
            round_events.push(TraceEvent {
                kind: "transition".to_string(),
                transition_id: Some(transition.id.clone()),
                trigger: Some(format!("after:{duration}")),
            });
            fired_this_round = true;

            for _ in 0..CHANGE_LOOP_LIMIT {
                let mut change_fired = false;
                let subject_ids = subjects
                    .iter()
                    .map(|subject| subject.subject_id.clone())
                    .collect::<Vec<_>>();
                for subject in subjects.iter_mut() {
                    let transition = select_concurrent_change_transition(
                        runtime,
                        subject.machine,
                        &subject.subject_id,
                        &subject.active,
                        &subject_ids,
                        &context,
                    )?
                    .cloned();
                    let Some(transition) = transition else {
                        continue;
                    };
                    let mut change_critical = Vec::new();
                    apply_transition_effects(
                        runtime,
                        &transition,
                        &subject.subject_id,
                        step,
                        &mut values,
                        &mut context,
                        &mut change_critical,
                    );
                    enqueue_transition_signals(
                        runtime,
                        &transition,
                        &subject.subject_id,
                        &mut pending_signals,
                    );
                    let before = subject.active.clone();
                    subject.active = apply_transition_state_change(
                        subject.machine,
                        &subject.subject_id,
                        &before,
                        &transition.source,
                        &transition.target,
                        step,
                        &mut values,
                        &mut context,
                        &mut change_critical,
                        Some(&mut pending_signals),
                        Some(&mut history),
                    )?;
                    round_events.push(TraceEvent {
                        kind: "transition".to_string(),
                        transition_id: Some(transition.id.clone()),
                        trigger: Some(format!(
                            "change:{}",
                            transition.trigger.as_deref().unwrap_or("")
                        )),
                    });
                    change_fired = true;
                }
                if !change_fired {
                    break;
                }
            }
        }

        for _ in 0..CHANGE_LOOP_LIMIT {
            let mut change_fired = false;
            let subject_ids = subjects
                .iter()
                .map(|subject| subject.subject_id.clone())
                .collect::<Vec<_>>();
            for subject in subjects.iter_mut() {
                let transition = select_concurrent_change_transition(
                    runtime,
                    subject.machine,
                    &subject.subject_id,
                    &subject.active,
                    &subject_ids,
                    &context,
                )?
                .cloned();
                let Some(transition) = transition else {
                    continue;
                };
                let mut step_critical = Vec::new();
                apply_transition_effects(
                    runtime,
                    &transition,
                    &subject.subject_id,
                    step,
                    &mut values,
                    &mut context,
                    &mut step_critical,
                );
                enqueue_transition_signals(
                    runtime,
                    &transition,
                    &subject.subject_id,
                    &mut pending_signals,
                );
                let before = subject.active.clone();
                subject.active = apply_transition_state_change(
                    subject.machine,
                    &subject.subject_id,
                    &before,
                    &transition.source,
                    &transition.target,
                    step,
                    &mut values,
                    &mut context,
                    &mut step_critical,
                    Some(&mut pending_signals),
                    Some(&mut history),
                )?;
                round_events.push(TraceEvent {
                    kind: "transition".to_string(),
                    transition_id: Some(transition.id.clone()),
                    trigger: Some(format!(
                        "change:{}",
                        transition.trigger.as_deref().unwrap_or("")
                    )),
                });
                change_fired = true;
                fired_this_round = true;
            }
            if !change_fired {
                break;
            }
        }

        if !fired_this_round && step < max_steps {
            let rates = active_state_rates(&subjects, &values);
            if !rates.is_empty() {
                let subject_ids = subjects
                    .iter()
                    .map(|subject| subject.subject_id.clone())
                    .collect::<Vec<_>>();
                let crossing = earliest_change_crossing(
                    runtime,
                    &subjects,
                    &subject_ids,
                    &context,
                    &values,
                    &rates,
                    scenario.step_duration_s,
                )?;
                let duration = crossing
                    .as_ref()
                    .map(|crossing| crossing.offset)
                    .unwrap_or(scenario.step_duration_s);

                integrate_active_state_behaviors(
                    &rates,
                    &mut values,
                    &mut context,
                    duration,
                    &mut rate_channels,
                );
                t += duration;
                step += 1;
                fired_this_round = true;

                if let Some(crossing) = crossing {
                    let subject_id = subjects[crossing.subject_index].subject_id.clone();
                    let transition = crossing.transition;
                    let mut step_critical = Vec::new();
                    apply_transition_effects(
                        runtime,
                        &transition,
                        &subject_id,
                        step,
                        &mut values,
                        &mut context,
                        &mut step_critical,
                    );
                    enqueue_transition_signals(
                        runtime,
                        &transition,
                        &subject_id,
                        &mut pending_signals,
                    );
                    let before = subjects[crossing.subject_index].active.clone();
                    subjects[crossing.subject_index].active = apply_transition_state_change(
                        subjects[crossing.subject_index].machine,
                        &subject_id,
                        &before,
                        &transition.source,
                        &transition.target,
                        step,
                        &mut values,
                        &mut context,
                        &mut step_critical,
                        Some(&mut pending_signals),
                        Some(&mut history),
                    )?;
                    round_events.push(TraceEvent {
                        kind: "transition".to_string(),
                        transition_id: Some(transition.id.clone()),
                        trigger: Some(format!(
                            "change:{}",
                            transition.trigger.as_deref().unwrap_or("")
                        )),
                    });
                }
            }
        }

        if fired_this_round {
            timeline.push(make_concurrent_entry(t, &subjects, &values, round_events));
        } else {
            break;
        }
    }

    let primary_subject_id = scenario
        .subjects
        .first()
        .map(|subject| subject.subject_id.clone())
        .unwrap_or_default();
    let mut channel_ids = BTreeSet::<(String, String)>::new();
    for entry in &timeline {
        channel_ids.extend(entry.values.keys().cloned());
    }
    let channels = channel_ids
        .into_iter()
        .map(|(subject, feature)| {
            let source = if rate_channels.contains(&(subject.clone(), feature.clone())) {
                TraceChannelSource::RateEffect
            } else {
                TraceChannelSource::AssignEffect
            };
            TraceChannel {
                id: format!("{subject}.{feature}"),
                unit: None,
                source,
            }
        })
        .collect();

    Ok(SimulationTrace {
        scenario_id: scenario.id,
        subject_id: primary_subject_id,
        channels,
        timeline,
        status: HybridSimulationStatus::Completed,
    })
}

fn try_run_canonical_core(
    runtime: &Runtime,
    scenario: &ConcurrentSimulationScenario,
) -> Result<Option<SimulationTrace>, SimulationError> {
    let Ok(model) = canonical_simulation_model(runtime) else {
        return Ok(None);
    };
    if runtime_has_legacy_transition_effects(runtime, &model, scenario)
        || !core_runner_can_handle(&model, scenario)
    {
        return Ok(None);
    }
    run_concurrent_simulation_model(
        &model,
        scenario.clone(),
        SimulationClockConfig {
            max_time_s: scenario.max_steps.max(1) as f64 * scenario.step_duration_s.max(0.0),
            fixed_step_s: scenario.step_duration_s,
            sample_interval_s: scenario.step_duration_s,
            change_loop_limit: CHANGE_LOOP_LIMIT,
        },
    )
    .map(Some)
    .map_err(|error| SimulationError::InvalidProfile(error.to_string()))
}

fn runtime_has_legacy_transition_effects(
    runtime: &Runtime,
    model: &SimulationModel,
    scenario: &ConcurrentSimulationScenario,
) -> bool {
    scenario.subjects.iter().any(|subject| {
        model
            .machines
            .iter()
            .find(|machine| machine.id == subject.machine_id || machine.label == subject.machine_id)
            .is_some_and(|machine| {
                machine.transitions.iter().any(|transition| {
                    runtime
                        .graph()
                        .element_by_element_id(&transition.id)
                        .and_then(|element| element.properties.get("effects"))
                        .is_some()
                })
            })
    })
}

fn core_runner_can_handle(
    model: &SimulationModel,
    scenario: &ConcurrentSimulationScenario,
) -> bool {
    if scenario_uses_legacy_transition_effects(model, scenario) {
        return false;
    }
    scenario.subjects.iter().all(|subject| {
        model
            .machines
            .iter()
            .find(|machine| machine.id == subject.machine_id || machine.label == subject.machine_id)
            .is_some_and(|machine| {
                machine
                    .states
                    .iter()
                    .all(|state| state.do_behavior.is_none())
                    && machine.transitions.iter().all(|transition| {
                        transition.guard.is_none()
                            && matches!(
                                transition.trigger.kind,
                                SimulationTriggerKind::Event | SimulationTriggerKind::Signal
                            )
                    })
            })
    })
}

fn scenario_uses_legacy_transition_effects(
    model: &SimulationModel,
    scenario: &ConcurrentSimulationScenario,
) -> bool {
    scenario.subjects.iter().any(|subject| {
        model
            .machines
            .iter()
            .find(|machine| machine.id == subject.machine_id || machine.label == subject.machine_id)
            .is_some_and(|machine| {
                machine.transitions.iter().any(|transition| {
                    !transition.effects.is_empty()
                        || transition.effects.iter().any(|effect| {
                            matches!(effect, mercurio_simulation_core::SimulationEffect::Log(_))
                        })
                })
            })
    })
}

fn fire_after_transitions(
    runtime: &Runtime,
    machine: &StateMachineModel,
    subject_id: &str,
    active: &mut Vec<String>,
    values: &mut BTreeMap<(String, String), Value>,
    context: &mut ExecutionContext,
    critical_events: &mut Vec<CriticalSimulationEvent>,
    trace: &mut Vec<HybridSimulationTraceEntry>,
    t: &mut f64,
    step: &mut usize,
    step_duration_s: f64,
    max_steps: usize,
    rate_channels: &mut BTreeSet<(String, String)>,
    history: &mut BTreeMap<(String, String), String>,
) -> Result<bool, SimulationError> {
    let mut fired_any = false;
    while *step < max_steps {
        let after_transitions = active
            .iter()
            .rev()
            .flat_map(|state_id| {
                machine.transitions.iter().filter(move |transition| {
                    transition.source == *state_id
                        && transition.trigger_kind == StateTransitionTriggerKind::After
                })
            })
            .collect::<Vec<_>>();
        let [transition] = after_transitions.as_slice() else {
            return Ok(fired_any);
        };
        let Some(duration) = transition
            .trigger
            .as_deref()
            .and_then(|trigger| trigger.parse::<f64>().ok())
            .filter(|duration| duration.is_finite() && *duration >= 0.0)
        else {
            return Ok(fired_any);
        };
        let remaining_steps = max_steps.saturating_sub(*step);
        if duration > step_duration_s * remaining_steps as f64 {
            return Ok(fired_any);
        }

        *step += 1;
        let before = active.clone();
        let t_enter = *t;
        apply_rate_samples(
            runtime,
            transition,
            subject_id,
            &before,
            values,
            context,
            trace,
            t_enter,
            duration,
            *step,
            rate_channels,
        );
        *t = t_enter + duration;

        let mut step_critical = Vec::new();
        append_guard_evaluation(
            runtime,
            transition,
            subject_id,
            context,
            *step,
            &mut step_critical,
        )?;
        apply_transition_effects(
            runtime,
            transition,
            subject_id,
            *step,
            values,
            context,
            &mut step_critical,
        );
        *active = apply_transition_state_change(
            machine,
            subject_id,
            &before,
            &transition.source,
            &transition.target,
            *step,
            values,
            context,
            &mut step_critical,
            None,
            Some(history),
        )?;

        let trigger = format!("after:{duration}");
        trace.push(HybridSimulationTraceEntry {
            step: *step,
            t: *t,
            event_id: None,
            trigger: Some(trigger),
            transition_id: Some(transition.id.clone()),
            before,
            after: active.clone(),
            values: values.clone(),
            critical_events: step_critical.clone(),
            explanation: format!(
                "After transition `{}` fired after {duration}s.",
                transition.id
            ),
        });
        critical_events.append(&mut step_critical);
        fired_any = true;

        fire_change_transitions(
            runtime,
            machine,
            subject_id,
            active,
            values,
            context,
            critical_events,
            trace,
            *t,
            *step,
            history,
        )?;
    }
    Ok(fired_any)
}

fn apply_rate_samples(
    runtime: &Runtime,
    transition: &TransitionNode,
    subject_id: &str,
    active: &[String],
    values: &mut BTreeMap<(String, String), Value>,
    context: &mut ExecutionContext,
    trace: &mut Vec<HybridSimulationTraceEntry>,
    t_enter: f64,
    duration: f64,
    step: usize,
    rate_channels: &mut BTreeSet<(String, String)>,
) {
    let rates = transition_effects(runtime, transition)
        .into_iter()
        .filter_map(|effect| match effect {
            TransitionEffect::Rate {
                feature,
                rate_per_second,
                unit: _,
            } => Some((feature, rate_per_second)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if rates.is_empty() {
        return;
    }

    // Record every rate-driven (subject, feature) pair for channel source tracking.
    for (feature, _) in &rates {
        rate_channels.insert((subject_id.to_string(), feature.clone()));
    }

    let initial_values = rates
        .iter()
        .map(|(feature, _)| {
            let key = (subject_id.to_string(), feature.clone());
            let initial = values.get(&key).and_then(Value::as_f64).unwrap_or(0.0);
            (feature.clone(), initial)
        })
        .collect::<BTreeMap<_, _>>();

    let mut sample_offset = RATE_SAMPLE_INTERVAL_S;
    while sample_offset < duration {
        let mut explanations = Vec::new();
        for (feature, rate_per_second) in &rates {
            let initial = initial_values.get(feature).copied().unwrap_or(0.0);
            let value = initial + rate_per_second * sample_offset;
            let key = (subject_id.to_string(), feature.clone());
            values.insert(key.clone(), Value::from(value));
            context.values.insert(key, Value::from(value));
            explanations.push(format!("{feature} = {value}"));
        }
        context.version += 1;
        trace.push(HybridSimulationTraceEntry {
            step,
            t: t_enter + sample_offset,
            event_id: None,
            trigger: Some("rate_sample".to_string()),
            transition_id: None,
            before: active.to_vec(),
            after: active.to_vec(),
            values: values.clone(),
            critical_events: Vec::new(),
            explanation: format!("rate sample: {}", explanations.join(", ")),
        });
        sample_offset += RATE_SAMPLE_INTERVAL_S;
    }

    for (feature, rate_per_second) in &rates {
        let initial = initial_values.get(feature).copied().unwrap_or(0.0);
        let value = initial + rate_per_second * duration;
        let key = (subject_id.to_string(), feature.clone());
        values.insert(key.clone(), Value::from(value));
        context.values.insert(key, Value::from(value));
    }
    context.version += 1;
}

fn fire_change_transitions(
    runtime: &Runtime,
    machine: &StateMachineModel,
    subject_id: &str,
    active: &mut Vec<String>,
    values: &mut BTreeMap<(String, String), Value>,
    context: &mut ExecutionContext,
    critical_events: &mut Vec<CriticalSimulationEvent>,
    trace: &mut Vec<HybridSimulationTraceEntry>,
    t: f64,
    step: usize,
    history: &mut BTreeMap<(String, String), String>,
) -> Result<(), SimulationError> {
    for _ in 0..CHANGE_LOOP_LIMIT {
        let Some(transition) =
            select_change_transition(runtime, machine, subject_id, active, context)?
        else {
            return Ok(());
        };
        let before = active.clone();
        let mut step_critical = Vec::new();
        append_guard_evaluation(
            runtime,
            transition,
            subject_id,
            context,
            step,
            &mut step_critical,
        )?;
        apply_transition_effects(
            runtime,
            transition,
            subject_id,
            step,
            values,
            context,
            &mut step_critical,
        );
        *active = apply_transition_state_change(
            machine,
            subject_id,
            &before,
            &transition.source,
            &transition.target,
            step,
            values,
            context,
            &mut step_critical,
            None,
            Some(history),
        )?;
        trace.push(HybridSimulationTraceEntry {
            step,
            t,
            event_id: None,
            trigger: transition
                .trigger
                .as_ref()
                .map(|trigger| format!("change:{trigger}")),
            transition_id: Some(transition.id.clone()),
            before,
            after: active.clone(),
            values: values.clone(),
            critical_events: step_critical.clone(),
            explanation: format!("Change transition `{}` fired.", transition.id),
        });
        critical_events.append(&mut step_critical);
    }

    let event = critical_event(
        step,
        "change.loop.detected",
        subject_id,
        [("limit", Value::from(CHANGE_LOOP_LIMIT))],
    );
    trace.push(HybridSimulationTraceEntry {
        step,
        t,
        event_id: None,
        trigger: Some("change.loop.detected".to_string()),
        transition_id: None,
        before: active.clone(),
        after: active.clone(),
        values: values.clone(),
        critical_events: vec![event.clone()],
        explanation: "Change transition recursion limit exceeded.".to_string(),
    });
    critical_events.push(event);
    Ok(())
}

fn select_change_transition<'a>(
    runtime: &Runtime,
    machine: &'a StateMachineModel,
    subject_id: &str,
    active_configuration: &[String],
    context: &ExecutionContext,
) -> Result<Option<&'a TransitionNode>, SimulationError> {
    let mut candidates = active_configuration
        .iter()
        .rev()
        .flat_map(|state_id| {
            machine.transitions.iter().filter(move |transition| {
                transition.source == *state_id
                    && transition.trigger_kind == StateTransitionTriggerKind::Change
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.id.cmp(&right.id));

    for transition in candidates {
        if transition_guard_satisfied(runtime, transition, subject_id, context)? {
            return Ok(Some(transition));
        }
    }
    Ok(None)
}

fn select_concurrent_change_transition<'a>(
    runtime: &Runtime,
    machine: &'a StateMachineModel,
    subject_id: &str,
    active_configuration: &[String],
    subject_ids: &[String],
    context: &ExecutionContext,
) -> Result<Option<&'a TransitionNode>, SimulationError> {
    let mut candidates = active_configuration
        .iter()
        .rev()
        .flat_map(|state_id| {
            machine.transitions.iter().filter(move |transition| {
                transition.source == *state_id
                    && transition.trigger_kind == StateTransitionTriggerKind::Change
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.id.cmp(&right.id));

    for transition in candidates {
        let mut evaluation_subjects = vec![subject_id.to_string()];
        for candidate_subject_id in subject_ids {
            if candidate_subject_id != subject_id {
                evaluation_subjects.push(candidate_subject_id.clone());
            }
        }

        let mut last_error = None;
        let mut evaluated = false;
        for evaluation_subject_id in evaluation_subjects {
            match transition_guard_satisfied(runtime, transition, &evaluation_subject_id, context) {
                Ok(satisfied) => {
                    evaluated = true;
                    if satisfied {
                        return Ok(Some(transition));
                    }
                }
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }
        if !evaluated && let Some(err) = last_error {
            return Err(err);
        }
    }
    Ok(None)
}

fn transition_guard_satisfied(
    runtime: &Runtime,
    transition: &TransitionNode,
    subject_id: &str,
    context: &ExecutionContext,
) -> Result<bool, SimulationError> {
    if let Some(guard_feature) = guard_feature_id(runtime, transition) {
        let result = runtime.evaluate(&guard_feature, subject_id, context)?;
        return Ok(result.value.as_bool().unwrap_or(false));
    }

    if transition.trigger_kind == StateTransitionTriggerKind::Change
        && let Some(trigger) = transition.trigger.as_deref()
    {
        return Ok(evaluate_textual_change_guard(trigger, subject_id, context).unwrap_or(false));
    }

    Ok(true)
}

fn evaluate_textual_change_guard(
    trigger: &str,
    subject_id: &str,
    context: &ExecutionContext,
) -> Option<bool> {
    let parts = trigger.split_whitespace().collect::<Vec<_>>();
    let [left, op, right] = parts.as_slice() else {
        return None;
    };
    let left = textual_guard_operand_value(left, subject_id, context)?;
    let right = textual_guard_operand_value(right, subject_id, context)?;
    Some(match *op {
        "==" | "=" => left == right,
        "!=" => left != right,
        "<" => left < right,
        "<=" => left <= right,
        ">" => left > right,
        ">=" => left >= right,
        _ => return None,
    })
}

fn textual_guard_operand_value(
    operand: &str,
    subject_id: &str,
    context: &ExecutionContext,
) -> Option<f64> {
    operand.parse::<f64>().ok().or_else(|| {
        context
            .values
            .get(&(subject_id.to_string(), operand.to_string()))
            .and_then(Value::as_f64)
    })
}

fn append_guard_evaluation(
    runtime: &Runtime,
    transition: &TransitionNode,
    subject_id: &str,
    context: &ExecutionContext,
    step: usize,
    step_critical: &mut Vec<CriticalSimulationEvent>,
) -> Result<(), SimulationError> {
    if let Some(guard_feature) = guard_feature_id(runtime, transition) {
        let result = runtime.evaluate(&guard_feature, subject_id, context)?;
        step_critical.push(critical_event(
            step,
            "guard.evaluated",
            subject_id,
            [
                ("feature", Value::String(guard_feature)),
                ("result", result.value),
            ],
        ));
    }
    Ok(())
}

fn apply_transition_effects(
    runtime: &Runtime,
    transition: &TransitionNode,
    subject_id: &str,
    step: usize,
    values: &mut BTreeMap<(String, String), Value>,
    context: &mut ExecutionContext,
    step_critical: &mut Vec<CriticalSimulationEvent>,
) {
    for effect in transition_effects(runtime, transition) {
        match effect {
            TransitionEffect::Assign { feature, value } => {
                values.insert((subject_id.to_string(), feature.clone()), value.clone());
                context
                    .values
                    .insert((subject_id.to_string(), feature.clone()), value.clone());
                context.version += 1;
                step_critical.push(critical_event(
                    step,
                    "effect.assigned",
                    subject_id,
                    [("feature", Value::String(feature)), ("value", value)],
                ));
            }
            TransitionEffect::Rate { .. } => {}
            TransitionEffect::SendSignal { .. } => {}
            TransitionEffect::Log { kind, detail } => {
                step_critical.push(critical_event(step, &kind, subject_id, detail));
            }
        }
    }
}

fn apply_transition_state_change(
    machine: &StateMachineModel,
    subject_id: &str,
    before: &[String],
    source_state_id: &str,
    target_state_id: &str,
    step: usize,
    values: &mut BTreeMap<(String, String), Value>,
    context: &mut ExecutionContext,
    step_critical: &mut Vec<CriticalSimulationEvent>,
    pending_signals: Option<&mut VecDeque<PendingSignal>>,
    history: Option<&mut BTreeMap<(String, String), String>>,
) -> Result<Vec<String>, SimulationError> {
    let mut history = history;
    let resolved_target_state_id =
        resolve_history_target(machine, subject_id, target_state_id, history.as_deref())
            .unwrap_or_else(|| target_state_id.to_string());
    let target_configuration = initial_configuration(machine, Some(&resolved_target_state_id))
        .ok_or_else(|| SimulationError::MissingInitialState(resolved_target_state_id.clone()))?;
    let source_path = ancestor_path(machine, source_state_id)
        .ok_or_else(|| SimulationError::MissingInitialState(source_state_id.to_string()))?;
    let target_path = ancestor_path(machine, &resolved_target_state_id)
        .ok_or_else(|| SimulationError::MissingInitialState(resolved_target_state_id.clone()))?;
    let common_prefix_len = source_path
        .iter()
        .zip(target_path.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let common_ancestor = common_prefix_len
        .checked_sub(1)
        .and_then(|index| source_path.get(index));
    let exit_states = before
        .iter()
        .filter(|state_id| {
            state_id.as_str() == source_state_id
                || is_descendant_of(machine, state_id, source_state_id)
                || (common_ancestor.is_none_or(|ancestor| state_id.as_str() != ancestor)
                    && source_path.contains(state_id))
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut after = before
        .iter()
        .filter(|state_id| !exit_states.contains(state_id))
        .cloned()
        .collect::<Vec<_>>();
    let entry_states = target_configuration
        .iter()
        .filter(|state_id| !after.contains(state_id))
        .cloned()
        .collect::<Vec<_>>();
    after.extend(entry_states.clone());
    let mut pending_signals = pending_signals;

    for state_id in exit_states.iter().rev() {
        record_shallow_history(machine, subject_id, state_id, history.as_deref_mut());
        apply_state_behavior(
            machine,
            state_id,
            StateBehaviorKind::Exit,
            subject_id,
            step,
            values,
            context,
            step_critical,
            pending_signals.as_deref_mut(),
        );
    }
    for state_id in &entry_states {
        step_critical.push(critical_event(
            step,
            "state.entered",
            subject_id,
            [("state", Value::String(state_id.clone()))],
        ));
        apply_state_behavior(
            machine,
            state_id,
            StateBehaviorKind::Entry,
            subject_id,
            step,
            values,
            context,
            step_critical,
            pending_signals.as_deref_mut(),
        );
    }

    Ok(after)
}

fn resolve_history_target(
    machine: &StateMachineModel,
    subject_id: &str,
    target_state_id: &str,
    history: Option<&BTreeMap<(String, String), String>>,
) -> Option<String> {
    let target = machine
        .states
        .iter()
        .find(|state| state.id == target_state_id)?;
    if !target.is_history {
        return Some(target_state_id.to_string());
    }
    let parent_id = target.parent_state_id.as_ref()?;
    history
        .and_then(|history| {
            history
                .get(&(subject_id.to_string(), parent_id.clone()))
                .cloned()
        })
        .or_else(|| default_child_state(machine, parent_id).map(|state| state.id.clone()))
}

fn record_shallow_history(
    machine: &StateMachineModel,
    subject_id: &str,
    state_id: &str,
    history: Option<&mut BTreeMap<(String, String), String>>,
) {
    let Some(history) = history else {
        return;
    };
    let Some(state) = machine.states.iter().find(|state| state.id == state_id) else {
        return;
    };
    let Some(parent_id) = &state.parent_state_id else {
        return;
    };
    history.insert(
        (subject_id.to_string(), parent_id.clone()),
        state_id.to_string(),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StateBehaviorKind {
    Entry,
    Exit,
}

fn apply_state_behavior(
    machine: &StateMachineModel,
    state_id: &str,
    kind: StateBehaviorKind,
    subject_id: &str,
    step: usize,
    values: &mut BTreeMap<(String, String), Value>,
    context: &mut ExecutionContext,
    step_critical: &mut Vec<CriticalSimulationEvent>,
    pending_signals: Option<&mut VecDeque<PendingSignal>>,
) {
    let Some(state) = machine.states.iter().find(|state| state.id == state_id) else {
        return;
    };
    let behavior = match kind {
        StateBehaviorKind::Entry => state.entry_behavior.as_ref(),
        StateBehaviorKind::Exit => state.exit_behavior.as_ref(),
    };
    let Some(behavior) = behavior else {
        return;
    };

    let mut pending_signals = pending_signals;
    for effect in action_sequence_effects(behavior) {
        match effect {
            TransitionEffect::Assign { feature, value } => {
                values.insert((subject_id.to_string(), feature.clone()), value.clone());
                context
                    .values
                    .insert((subject_id.to_string(), feature.clone()), value.clone());
                context.version += 1;
                step_critical.push(critical_event(
                    step,
                    "state.behavior.assigned",
                    subject_id,
                    [
                        ("state", Value::String(state_id.to_string())),
                        ("feature", Value::String(feature)),
                    ],
                ));
            }
            TransitionEffect::SendSignal {
                signal_type,
                target,
            } => {
                if let Some(queue) = pending_signals.as_deref_mut() {
                    queue.push_back(PendingSignal {
                        source_subject_id: subject_id.to_string(),
                        signal_type: signal_type.clone(),
                        target,
                    });
                }
                step_critical.push(critical_event(
                    step,
                    "state.behavior.signal",
                    subject_id,
                    [
                        ("state", Value::String(state_id.to_string())),
                        ("signal", Value::String(signal_type)),
                    ],
                ));
            }
            TransitionEffect::Log { kind, detail } => {
                step_critical.push(critical_event(step, &kind, subject_id, detail));
            }
            TransitionEffect::Rate { .. } => {}
        }
    }
}

fn action_sequence_effects(behavior: &Value) -> Vec<TransitionEffect> {
    behavior
        .as_object()
        .and_then(|object| object.get("actions"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(TransitionEffect::from_value)
        .collect()
}

fn report(
    scenario: HybridSimulationScenario,
    machine: &StateMachineModel,
    status: HybridSimulationStatus,
    active_configuration: Vec<String>,
    values: BTreeMap<(String, String), Value>,
    critical_events: Vec<CriticalSimulationEvent>,
    trace: Vec<HybridSimulationTraceEntry>,
    rate_channels: BTreeSet<(String, String)>,
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
        rate_channels,
    }
}

fn initial_configuration(
    machine: &StateMachineModel,
    initial_state_id: Option<&str>,
) -> Option<Vec<String>> {
    if let Some(state_id) = initial_state_id {
        return enter_state_configuration(machine, state_id);
    }

    if let Some(top_level_initial) = machine
        .states
        .iter()
        .find(|state| state.parent_state_id.is_none() && state.is_initial)
    {
        return enter_state_configuration(machine, &top_level_initial.id);
    }

    let root_id = machine
        .states
        .iter()
        .find(|state| state.parent_state_id.is_none())?
        .id
        .clone();
    enter_state_configuration(machine, &root_id)
}

fn enter_state_configuration(machine: &StateMachineModel, state_id: &str) -> Option<Vec<String>> {
    let mut configuration = ancestor_path(machine, state_id)?;
    append_default_descendants(machine, state_id, &mut configuration);
    Some(configuration)
}

fn append_default_descendants(
    machine: &StateMachineModel,
    state_id: &str,
    configuration: &mut Vec<String>,
) {
    let Some(state) = machine.states.iter().find(|state| state.id == state_id) else {
        return;
    };
    if state.is_orthogonal {
        let children = default_orthogonal_children(machine, state_id);
        for child in children {
            if !configuration.contains(&child.id) {
                configuration.push(child.id.clone());
            }
            append_default_descendants(machine, &child.id, configuration);
        }
        return;
    }

    if let Some(child) = default_child_state(machine, state_id) {
        if !configuration.contains(&child.id) {
            configuration.push(child.id.clone());
        }
        append_default_descendants(machine, &child.id, configuration);
    }
}

fn ancestor_path(machine: &StateMachineModel, state_id: &str) -> Option<Vec<String>> {
    let mut path = Vec::new();
    let mut cursor = machine.states.iter().find(|state| state.id == state_id)?;
    loop {
        path.push(cursor.id.clone());
        let Some(parent_id) = &cursor.parent_state_id else {
            path.reverse();
            return Some(path);
        };
        cursor = machine.states.iter().find(|state| state.id == *parent_id)?;
    }
}

fn default_child_state<'a>(
    machine: &'a StateMachineModel,
    parent_id: &str,
) -> Option<&'a mercurio_sysml::StateNode> {
    machine
        .states
        .iter()
        .filter(|state| state.parent_state_id.as_deref() == Some(parent_id))
        .find(|state| state.is_initial)
        .or_else(|| {
            machine
                .states
                .iter()
                .filter(|state| state.parent_state_id.as_deref() == Some(parent_id))
                .min_by_key(|state| state.source_order.unwrap_or((u64::MAX, u64::MAX)))
        })
}

fn default_orthogonal_children<'a>(
    machine: &'a StateMachineModel,
    parent_id: &str,
) -> Vec<&'a mercurio_sysml::StateNode> {
    let initial_children = machine
        .states
        .iter()
        .filter(|state| state.parent_state_id.as_deref() == Some(parent_id) && state.is_initial)
        .collect::<Vec<_>>();
    if !initial_children.is_empty() {
        return initial_children;
    }
    machine
        .states
        .iter()
        .filter(|state| state.parent_state_id.as_deref() == Some(parent_id))
        .collect()
}

fn is_descendant_of(machine: &StateMachineModel, state_id: &str, ancestor_id: &str) -> bool {
    let mut cursor = machine.states.iter().find(|state| state.id == state_id);
    while let Some(state) = cursor {
        let Some(parent_id) = &state.parent_state_id else {
            return false;
        };
        if parent_id == ancestor_id {
            return true;
        }
        cursor = machine
            .states
            .iter()
            .find(|candidate| candidate.id == *parent_id);
    }
    false
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
            transition.source == *state_id
                && matches!(
                    transition.trigger_kind,
                    StateTransitionTriggerKind::Event | StateTransitionTriggerKind::Unknown
                )
                && transition.trigger.as_deref() == Some(trigger)
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

fn select_signal_transition<'a>(
    runtime: &Runtime,
    machine: &'a StateMachineModel,
    subject_id: &str,
    active_configuration: &[String],
    signal_type: &str,
    context: &ExecutionContext,
) -> Result<Option<&'a TransitionNode>, SimulationError> {
    for state_id in active_configuration.iter().rev() {
        for transition in machine.transitions.iter().filter(|transition| {
            transition.source == *state_id
                && transition.trigger_kind == StateTransitionTriggerKind::Signal
                && transition.trigger.as_deref() == Some(signal_type)
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

fn enqueue_transition_signals(
    runtime: &Runtime,
    transition: &TransitionNode,
    source_subject_id: &str,
    pending_signals: &mut VecDeque<PendingSignal>,
) {
    for effect in transition_effects(runtime, transition) {
        if let TransitionEffect::SendSignal {
            signal_type,
            target,
        } = effect
        {
            pending_signals.push_back(PendingSignal {
                source_subject_id: source_subject_id.to_string(),
                signal_type,
                target,
            });
        }
    }
}

fn signal_targets_subject(signal: &PendingSignal, subject_id: &str) -> bool {
    match signal.target.as_deref() {
        Some(target) => target == subject_id,
        None => true,
    }
}

fn deliver_pending_signals(
    runtime: &Runtime,
    subjects: &mut [SubjectRunState<'_>],
    values: &mut BTreeMap<(String, String), Value>,
    context: &mut ExecutionContext,
    pending_signals: &mut VecDeque<PendingSignal>,
    history: &mut BTreeMap<(String, String), String>,
    step: &mut usize,
    max_steps: usize,
    round_events: &mut Vec<TraceEvent>,
) -> Result<bool, SimulationError> {
    let mut fired_any = false;
    let signal_count = pending_signals.len();
    for _ in 0..signal_count {
        if *step >= max_steps {
            break;
        }
        let Some(signal) = pending_signals.pop_front() else {
            break;
        };

        let mut consumed = false;
        for subject in subjects.iter_mut() {
            if *step >= max_steps || !signal_targets_subject(&signal, &subject.subject_id) {
                continue;
            }

            let transition = select_signal_transition(
                runtime,
                subject.machine,
                &subject.subject_id,
                &subject.active,
                &signal.signal_type,
                context,
            )?
            .cloned();
            let Some(transition) = transition else {
                continue;
            };

            *step += 1;
            let mut step_critical = Vec::new();
            append_guard_evaluation(
                runtime,
                &transition,
                &subject.subject_id,
                context,
                *step,
                &mut step_critical,
            )?;
            apply_transition_effects(
                runtime,
                &transition,
                &subject.subject_id,
                *step,
                values,
                context,
                &mut step_critical,
            );
            enqueue_transition_signals(runtime, &transition, &subject.subject_id, pending_signals);
            let before = subject.active.clone();
            subject.active = apply_transition_state_change(
                subject.machine,
                &subject.subject_id,
                &before,
                &transition.source,
                &transition.target,
                *step,
                values,
                context,
                &mut step_critical,
                Some(pending_signals),
                Some(history),
            )?;
            round_events.push(TraceEvent {
                kind: "transition".to_string(),
                transition_id: Some(transition.id.clone()),
                trigger: Some(format!(
                    "signal:{}:{}",
                    signal.source_subject_id, signal.signal_type
                )),
            });
            fired_any = true;
            consumed = true;
        }
        if !consumed {
            pending_signals.push_back(signal);
        }
    }
    Ok(fired_any)
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

#[derive(Debug, Clone, PartialEq)]
enum TransitionEffect {
    Assign {
        feature: String,
        value: Value,
    },
    Rate {
        feature: String,
        rate_per_second: f64,
        unit: Option<String>,
    },
    SendSignal {
        signal_type: String,
        target: Option<String>,
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
            "rate" => Some(Self::Rate {
                feature: object.get("feature")?.as_str()?.to_string(),
                rate_per_second: object.get("rate_per_second")?.as_f64()?,
                unit: object
                    .get("unit")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            }),
            "send_signal" => Some(Self::SendSignal {
                signal_type: string_property_any_value(object, &["signal_type", "signal", "type"])?,
                target: string_property_any_value(object, &["target", "target_subject"])
                    .filter(|target| target != "*"),
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

fn string_property_any_value(
    object: &serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    use mercurio_core::runtime::Runtime;
    use mercurio_core::{KirDocument, KirElement};
    use mercurio_sysml::{compile_sysml_text, load_sysml_baseline};

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
                step_duration_s: default_step_duration(),
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

    #[test]
    fn trace_entries_carry_timestamps() {
        let runtime = Runtime::from_document(simple_machine([
            transition_element(
                "transition.test.go",
                "TestMachine",
                "state.test.A",
                "state.test.B",
                "go",
                "event",
                [],
            ),
            transition_element(
                "transition.test.next",
                "TestMachine",
                "state.test.B",
                "state.test.C",
                "next",
                "event",
                [],
            ),
        ]))
        .unwrap();

        let report = run_hybrid_simulation(
            &runtime,
            scenario(
                vec![
                    StateMachineScenarioEvent {
                        id: "event.go".to_string(),
                        trigger: "go".to_string(),
                    },
                    StateMachineScenarioEvent {
                        id: "event.next".to_string(),
                        trigger: "next".to_string(),
                    },
                ],
                BTreeMap::new(),
                2.0,
                8,
            ),
        )
        .unwrap();

        assert_eq!(report.trace[0].t, 2.0);
        assert_eq!(report.trace[1].t, 4.0);
    }

    #[test]
    fn trace_entries_carry_value_snapshots() {
        let runtime = Runtime::from_document(simple_machine([
            transition_element(
                "transition.test.observe",
                "TestMachine",
                "state.test.A",
                "state.test.B",
                "observe",
                "event",
                [],
            ),
            transition_element(
                "transition.test.heat",
                "TestMachine",
                "state.test.B",
                "state.test.C",
                "heat",
                "event",
                [(
                    "effects",
                    json!([{ "kind": "assign", "feature": "temperature", "value": 50.0 }]),
                )],
            ),
        ]))
        .unwrap();

        let key = ("individual.test".to_string(), "temperature".to_string());
        let report = run_hybrid_simulation(
            &runtime,
            scenario(
                vec![
                    StateMachineScenarioEvent {
                        id: "event.observe".to_string(),
                        trigger: "observe".to_string(),
                    },
                    StateMachineScenarioEvent {
                        id: "event.heat".to_string(),
                        trigger: "heat".to_string(),
                    },
                ],
                BTreeMap::from([(key.clone(), json!(22.0))]),
                1.0,
                8,
            ),
        )
        .unwrap();

        assert_eq!(report.trace[0].values.get(&key), Some(&json!(22.0)));
        assert_eq!(report.trace[1].values.get(&key), Some(&json!(50.0)));
    }

    #[test]
    fn after_trigger_fires_without_event() {
        let runtime = Runtime::from_document(simple_machine([transition_element(
            "transition.test.after",
            "TestMachine",
            "state.test.A",
            "state.test.B",
            "3.0",
            "after",
            [],
        )]))
        .unwrap();

        let report =
            run_hybrid_simulation(&runtime, scenario(Vec::new(), BTreeMap::new(), 1.0, 10))
                .unwrap();

        let entry = report
            .trace
            .iter()
            .find(|entry| entry.after == vec!["state.test.B"])
            .unwrap();
        assert!(entry.t >= 3.0);
        assert_eq!(entry.event_id, None);
    }

    #[test]
    fn rate_effect_generates_intermediate_samples() {
        let runtime = Runtime::from_document(simple_machine([
            transition_element(
                "transition.test.after",
                "TestMachine",
                "state.test.A",
                "state.test.B",
                "5.0",
                "after",
                [(
                    "effects",
                    json!([{ "kind": "rate", "feature": "temperature", "rate_per_second": 2.0, "unit": "C" }]),
                )],
            ),
        ]))
        .unwrap();

        let key = ("individual.test".to_string(), "temperature".to_string());
        let report = run_hybrid_simulation(
            &runtime,
            scenario(
                Vec::new(),
                BTreeMap::from([(key.clone(), json!(20.0))]),
                1.0,
                10,
            ),
        )
        .unwrap();

        for expected_t in [1.0, 2.0, 3.0, 4.0] {
            assert!(report.trace.iter().any(|entry| entry.t == expected_t));
        }
        let sample = report.trace.iter().find(|entry| entry.t == 3.0).unwrap();
        assert_eq!(sample.values.get(&key), Some(&json!(26.0)));
        let final_entry = report
            .trace
            .iter()
            .find(|entry| entry.transition_id.as_deref() == Some("transition.test.after"))
            .unwrap();
        assert_eq!(final_entry.t, 5.0);
        assert_eq!(final_entry.values.get(&key), Some(&json!(30.0)));
    }

    #[test]
    fn change_trigger_fires_when_guard_is_true() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Test", "Model::Systems::PartDefinition", []),
                element(
                    "individual.test",
                    "Model::IndividualUsage",
                    [("declared_name", json!("test")), ("type", json!("type.Test"))],
                ),
                element(
                    "feature.Test.hotEnough",
                    "Model::CalculationUsage",
                    [
                        ("declared_name", json!("hotEnough")),
                        ("owner", json!("type.Test")),
                        ("expression_ir", greater_equal_path("temperature", 28.0)),
                    ],
                ),
                state_element("state.test.A", "TestMachine", true),
                state_element("state.test.B", "TestMachine", false),
                state_element("state.test.C", "TestMachine", false),
                transition_element(
                    "transition.test.after",
                    "TestMachine",
                    "state.test.A",
                    "state.test.B",
                    "5.0",
                    "after",
                    [(
                        "effects",
                        json!([{ "kind": "rate", "feature": "temperature", "rate_per_second": 2.0 }]),
                    )],
                ),
                transition_element(
                    "transition.test.change",
                    "TestMachine",
                    "state.test.B",
                    "state.test.C",
                    "temperature >= 28.0",
                    "change",
                    [("guard_feature", json!("feature.Test.hotEnough"))],
                ),
            ],
        })
        .unwrap();

        let report = run_hybrid_simulation(
            &runtime,
            scenario(
                Vec::new(),
                BTreeMap::from([(
                    ("individual.test".to_string(), "temperature".to_string()),
                    json!(20.0),
                )]),
                1.0,
                10,
            ),
        )
        .unwrap();

        assert!(
            report
                .trace
                .iter()
                .any(|entry| entry.after == vec!["state.test.C"])
        );
    }

    #[test]
    fn to_trace_produces_unified_trace() {
        let runtime = Runtime::from_document(simple_machine([transition_element(
            "transition.test.assign",
            "TestMachine",
            "state.test.A",
            "state.test.B",
            "assign",
            "event",
            [(
                "effects",
                json!([{ "kind": "assign", "feature": "temperature", "value": 25.0 }]),
            )],
        )]))
        .unwrap();

        let report = run_hybrid_simulation(
            &runtime,
            scenario(
                vec![StateMachineScenarioEvent {
                    id: "event.assign".to_string(),
                    trigger: "assign".to_string(),
                }],
                BTreeMap::from([(
                    ("individual.test".to_string(), "temperature".to_string()),
                    json!(20.0),
                )]),
                1.0,
                8,
            ),
        )
        .unwrap();
        let trace = report.to_trace();

        assert_eq!(trace.timeline.len(), report.trace.len());
        assert_eq!(trace.timeline[0].t, report.trace[0].t);
        assert!(!trace.channels.is_empty());
    }

    #[test]
    fn concurrent_simulation_fires_transitions_on_multiple_subjects() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.A", "Model::Systems::PartDefinition", []),
                element("type.B", "Model::Systems::PartDefinition", []),
                element(
                    "individual.a",
                    "Model::IndividualUsage",
                    [("declared_name", json!("a")), ("type", json!("type.A"))],
                ),
                element(
                    "individual.b",
                    "Model::IndividualUsage",
                    [("declared_name", json!("b")), ("type", json!("type.B"))],
                ),
                state_element("state.A.one", "MachineA", true),
                state_element("state.A.two", "MachineA", false),
                state_element("state.B.one", "MachineB", true),
                state_element("state.B.two", "MachineB", false),
                transition_element(
                    "transition.A.go",
                    "MachineA",
                    "state.A.one",
                    "state.A.two",
                    "go",
                    "event",
                    [],
                ),
                transition_element(
                    "transition.B.run",
                    "MachineB",
                    "state.B.one",
                    "state.B.two",
                    "run",
                    "event",
                    [],
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.concurrent".to_string(),
                subjects: vec![
                    ConcurrentSubjectScenario {
                        subject_id: "individual.a".to_string(),
                        machine_id: "MachineA".to_string(),
                        initial_state_id: None,
                        events: vec![StateMachineScenarioEvent {
                            id: "event.go".to_string(),
                            trigger: "go".to_string(),
                        }],
                    },
                    ConcurrentSubjectScenario {
                        subject_id: "individual.b".to_string(),
                        machine_id: "MachineB".to_string(),
                        initial_state_id: None,
                        events: vec![StateMachineScenarioEvent {
                            id: "event.run".to_string(),
                            trigger: "run".to_string(),
                        }],
                    },
                ],
                max_steps: 8,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
        )
        .unwrap();

        assert!(trace.timeline.iter().any(|entry| {
            entry
                .states
                .get("individual.a")
                .is_some_and(|states| states == &vec!["state.A.two".to_string()])
        }));
        assert!(trace.timeline.iter().any(|entry| {
            entry
                .states
                .get("individual.b")
                .is_some_and(|states| states == &vec!["state.B.two".to_string()])
        }));
    }

    #[test]
    fn analysis_case_extracts_and_runs_concurrent_scenario() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Printer", "Model::Systems::PartDefinition", []),
                element(
                    "individual.printer",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("printer")),
                        ("type", json!("type.Printer")),
                    ],
                ),
                state_element("state.Printer.idle", "PrinterLifecycle", true),
                state_element("state.Printer.printing", "PrinterLifecycle", false),
                transition_element(
                    "transition.Printer.start",
                    "PrinterLifecycle",
                    "state.Printer.idle",
                    "state.Printer.printing",
                    "start",
                    "event",
                    [],
                ),
                element(
                    "analysis.PrintSequence",
                    "SysML::Systems::AnalysisCaseDefinition",
                    [
                        ("declared_name", json!("PrintSequence")),
                        ("max_steps", json!(8)),
                        ("step_duration_s", json!(1.0)),
                        (
                            "subjects",
                            json!([
                                {
                                    "subject": "individual.printer",
                                    "machine": "PrinterLifecycle",
                                    "events": [
                                        { "id": "event.start", "trigger": "start" }
                                    ]
                                }
                            ]),
                        ),
                        (
                            "initial_values",
                            json!({ "individual.printer|bed_temperature": 22.0 }),
                        ),
                    ],
                ),
            ],
        })
        .unwrap();

        let cases = list_analysis_cases(&runtime);
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].label, "PrintSequence");
        assert_eq!(cases[0].subject_count, 1);

        let scenario = scenario_from_analysis_case(&runtime, "analysis.PrintSequence").unwrap();
        assert_eq!(scenario.id, "analysis.PrintSequence");
        assert_eq!(scenario.subjects[0].subject_id, "individual.printer");
        assert_eq!(scenario.subjects[0].events[0].trigger, "start");
        assert_eq!(
            scenario.initial_values.get(&(
                "individual.printer".to_string(),
                "bed_temperature".to_string()
            )),
            Some(&json!(22.0))
        );

        let trace = run_analysis_case(&runtime, "analysis.PrintSequence").unwrap();
        assert!(trace.timeline.iter().any(|entry| {
            entry
                .states
                .get("individual.printer")
                .is_some_and(|states| states == &vec!["state.Printer.printing".to_string()])
        }));
    }

    #[test]
    fn analysis_case_extracts_native_subjects_assumes_and_initial_state() {
        let stdlib = load_sysml_baseline().unwrap();
        let document = compile_sysml_text(
            r#"
            package Demo {
                import ScalarValues::*;

                part def Printer {
                    attribute bed_temperature : Real = 22.0;
                    attribute targetTemp : Real = 110.0;
                    attribute heatRate : Real = 2.3;

                    state lifecycle {
                        state Idle;
                        state Printing;

                        transition start first Idle accept start then Printing;
                    }
                }

                analysis def PrintSequence :> AnalysisCase {
                    subject printer : Printer;
                    assume constraint = printer.bed_temperature == 22.0;
                }
            }
            "#,
            "native-analysis.sysml",
            &stdlib,
        )
        .unwrap();
        let runtime = Runtime::from_document(document).unwrap();

        let cases = list_analysis_cases(&runtime);
        let print_sequence = cases
            .iter()
            .find(|case| case.label == "PrintSequence")
            .unwrap();
        assert_eq!(print_sequence.subject_count, 1);

        let scenario = scenario_from_analysis_case(&runtime, &print_sequence.id).unwrap();
        assert_eq!(scenario.subjects.len(), 1);
        assert!(scenario.subjects[0].subject_id.starts_with("subject."));
        assert!(
            scenario.subjects[0]
                .machine_id
                .ends_with(".Printer.lifecycle")
        );
        assert_eq!(scenario.subjects[0].events.len(), 1);
        assert_eq!(scenario.subjects[0].events[0].trigger, "start");
        assert_eq!(
            scenario.initial_values.get(&(
                scenario.subjects[0].subject_id.clone(),
                "bed_temperature".to_string()
            )),
            Some(&json!(22.0))
        );
        assert_eq!(
            scenario.initial_values.get(&(
                scenario.subjects[0].subject_id.clone(),
                "targetTemp".to_string()
            )),
            Some(&json!(110.0))
        );
        assert_eq!(
            scenario.initial_values.get(&(
                scenario.subjects[0].subject_id.clone(),
                "heatRate".to_string()
            )),
            Some(&json!(2.3))
        );

        let trace = run_concurrent_simulation(&runtime, scenario).unwrap();
        assert!(trace.timeline.iter().any(|entry| {
            entry
                .states
                .values()
                .any(|states| states.iter().any(|state| state.ends_with(".Printing")))
        }));
        assert!(
            trace
                .timeline
                .first()
                .unwrap()
                .states
                .values()
                .any(|states| states.iter().any(|state| state.ends_with(".Idle")))
        );
    }

    #[test]
    fn concurrent_simulation_cross_part_change_guard_fires() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Printer", "Model::Systems::PartDefinition", []),
                element("type.Bed", "Model::Systems::PartDefinition", []),
                element(
                    "individual.printer",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("printer")),
                        ("type", json!("type.Printer")),
                        ("bed", json!("individual.bed")),
                    ],
                ),
                element(
                    "individual.bed",
                    "Model::IndividualUsage",
                    [("declared_name", json!("bed")), ("type", json!("type.Bed"))],
                ),
                element(
                    "feature.Bed.bedReady",
                    "Model::CalculationUsage",
                    [
                        ("declared_name", json!("bedReady")),
                        ("owner", json!("type.Bed")),
                        ("expression_ir", greater_equal_path("bed_ready", 1.0)),
                    ],
                ),
                state_element("state.Printer.Waiting", "PrinterMachine", true),
                state_element("state.Printer.Printing", "PrinterMachine", false),
                state_element("state.Bed.Cold", "BedMachine", true),
                state_element("state.Bed.Hot", "BedMachine", false),
                transition_element(
                    "transition.Printer.print",
                    "PrinterMachine",
                    "state.Printer.Waiting",
                    "state.Printer.Printing",
                    "bed_ready >= 1.0",
                    "change",
                    [("guard_feature", json!("feature.Bed.bedReady"))],
                ),
                transition_element(
                    "transition.Bed.after",
                    "BedMachine",
                    "state.Bed.Cold",
                    "state.Bed.Hot",
                    "3.0",
                    "after",
                    [(
                        "effects",
                        json!([{ "kind": "rate", "feature": "bed_ready", "rate_per_second": 0.5 }]),
                    )],
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.cross_part".to_string(),
                subjects: vec![
                    ConcurrentSubjectScenario {
                        subject_id: "individual.printer".to_string(),
                        machine_id: "PrinterMachine".to_string(),
                        initial_state_id: None,
                        events: Vec::new(),
                    },
                    ConcurrentSubjectScenario {
                        subject_id: "individual.bed".to_string(),
                        machine_id: "BedMachine".to_string(),
                        initial_state_id: None,
                        events: Vec::new(),
                    },
                ],
                max_steps: 20,
                step_duration_s: 1.0,
                initial_values: BTreeMap::from([(
                    ("individual.bed".to_string(), "bed_ready".to_string()),
                    json!(0.0),
                )]),
            },
        )
        .unwrap();

        assert!(trace.timeline.iter().any(|entry| {
            entry
                .states
                .get("individual.printer")
                .is_some_and(|states| states == &vec!["state.Printer.Printing".to_string()])
        }));
    }

    #[test]
    fn state_do_behavior_drives_rate_integration_to_guard_crossing() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Bed", "Model::Systems::PartDefinition", []),
                element(
                    "individual.bed",
                    "Model::IndividualUsage",
                    [("declared_name", json!("bed")), ("type", json!("type.Bed"))],
                ),
                element(
                    "state.Bed.Heating",
                    "StateUsage",
                    [
                        ("declared_name", json!("Heating")),
                        ("owning_type", json!("BedMachine")),
                        ("is_initial", json!(true)),
                        (
                            "do_behavior",
                            json!({
                                "kind": "rate_integration",
                                "rates": [
                                    {
                                        "feature": "temperature",
                                        "rate_feature": "heatRate"
                                    }
                                ]
                            }),
                        ),
                    ],
                ),
                state_element("state.Bed.Ready", "BedMachine", false),
                transition_element(
                    "transition.Bed.ready",
                    "BedMachine",
                    "state.Bed.Heating",
                    "state.Bed.Ready",
                    "temperature >= targetTemp",
                    "change",
                    [],
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.state_do_rate".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "individual.bed".to_string(),
                    machine_id: "BedMachine".to_string(),
                    initial_state_id: None,
                    events: Vec::new(),
                }],
                max_steps: 100,
                step_duration_s: 1.0,
                initial_values: BTreeMap::from([
                    (
                        ("individual.bed".to_string(), "temperature".to_string()),
                        json!(22.0),
                    ),
                    (
                        ("individual.bed".to_string(), "heatRate".to_string()),
                        json!(2.3),
                    ),
                    (
                        ("individual.bed".to_string(), "targetTemp".to_string()),
                        json!(110.0),
                    ),
                ]),
            },
        )
        .unwrap();

        let ready_entry = trace
            .timeline
            .iter()
            .find(|entry| {
                entry
                    .states
                    .get("individual.bed")
                    .is_some_and(|states| states == &vec!["state.Bed.Ready".to_string()])
            })
            .unwrap();
        let expected = (110.0 - 22.0) / 2.3;
        assert!((ready_entry.t - expected).abs() <= 0.1);
        assert!(trace.timeline.len() > 30);
        assert!(
            ready_entry
                .values
                .get(&("individual.bed".to_string(), "temperature".to_string()))
                .and_then(Value::as_f64)
                .is_some_and(|temperature| temperature >= 110.0)
        );
    }

    #[test]
    fn state_do_rate_expression_integrates_newton_cooling_with_rk4() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Bed", "Model::Systems::PartDefinition", []),
                element(
                    "individual.bed",
                    "Model::IndividualUsage",
                    [("declared_name", json!("bed")), ("type", json!("type.Bed"))],
                ),
                element(
                    "state.Bed.Cooling",
                    "StateUsage",
                    [
                        ("declared_name", json!("Cooling")),
                        ("owning_type", json!("BedMachine")),
                        ("is_initial", json!(true)),
                        (
                            "do_behavior",
                            json!({
                                "kind": "rate_integration",
                                "rates": [
                                    {
                                        "feature": "temperature",
                                        "rate_expr": {
                                            "kind": "binary",
                                            "op": "multiply",
                                            "left": { "kind": "literal", "value": -0.05 },
                                            "right": {
                                                "kind": "binary",
                                                "op": "subtract",
                                                "left": {
                                                    "kind": "path",
                                                    "segments": ["temperature"]
                                                },
                                                "right": {
                                                    "kind": "path",
                                                    "segments": ["ambient"]
                                                }
                                            }
                                        }
                                    }
                                ]
                            }),
                        ),
                    ],
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.newton_cooling".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "individual.bed".to_string(),
                    machine_id: "BedMachine".to_string(),
                    initial_state_id: None,
                    events: Vec::new(),
                }],
                max_steps: 60,
                step_duration_s: 1.0,
                initial_values: BTreeMap::from([
                    (
                        ("individual.bed".to_string(), "temperature".to_string()),
                        json!(110.0),
                    ),
                    (
                        ("individual.bed".to_string(), "ambient".to_string()),
                        json!(22.0),
                    ),
                ]),
            },
        )
        .unwrap();

        let final_temperature = trace
            .timeline
            .last()
            .unwrap()
            .values
            .get(&("individual.bed".to_string(), "temperature".to_string()))
            .and_then(Value::as_f64)
            .unwrap();
        let expected = 22.0 + (110.0 - 22.0) * f64::exp(-0.05 * 60.0);
        assert!(
            (final_temperature - expected).abs() < 1.0,
            "final_temperature={final_temperature}, expected={expected}"
        );
    }

    #[test]
    fn concurrent_signal_effect_routes_to_accepting_subject() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Bed", "Model::Systems::PartDefinition", []),
                element("type.Printer", "Model::Systems::PartDefinition", []),
                element(
                    "individual.bed",
                    "Model::IndividualUsage",
                    [("declared_name", json!("bed")), ("type", json!("type.Bed"))],
                ),
                element(
                    "individual.printer",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("printer")),
                        ("type", json!("type.Printer")),
                    ],
                ),
                state_element("state.Bed.Heating", "BedMachine", true),
                state_element("state.Bed.Ready", "BedMachine", false),
                state_element("state.Printer.Heating", "PrinterMachine", true),
                state_element("state.Printer.Printing", "PrinterMachine", false),
                transition_element(
                    "transition.Bed.ready",
                    "BedMachine",
                    "state.Bed.Heating",
                    "state.Bed.Ready",
                    "finish",
                    "event",
                    [(
                        "effects",
                        json!([
                            {
                                "kind": "send_signal",
                                "signal_type": "BedReady",
                                "target": "individual.printer"
                            }
                        ]),
                    )],
                ),
                transition_element(
                    "transition.Printer.print",
                    "PrinterMachine",
                    "state.Printer.Heating",
                    "state.Printer.Printing",
                    "BedReady",
                    "signal",
                    [],
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.signal".to_string(),
                subjects: vec![
                    ConcurrentSubjectScenario {
                        subject_id: "individual.bed".to_string(),
                        machine_id: "BedMachine".to_string(),
                        initial_state_id: None,
                        events: vec![StateMachineScenarioEvent {
                            id: "event.finish".to_string(),
                            trigger: "finish".to_string(),
                        }],
                    },
                    ConcurrentSubjectScenario {
                        subject_id: "individual.printer".to_string(),
                        machine_id: "PrinterMachine".to_string(),
                        initial_state_id: None,
                        events: Vec::new(),
                    },
                ],
                max_steps: 8,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
        )
        .unwrap();

        assert!(trace.timeline.iter().any(|entry| {
            entry
                .states
                .get("individual.printer")
                .is_some_and(|states| states == &vec!["state.Printer.Printing".to_string()])
        }));
        assert!(trace.timeline.iter().any(|entry| {
            entry.events.iter().any(|event| {
                event.transition_id.as_deref() == Some("transition.Printer.print")
                    && event.trigger.as_deref() == Some("signal:individual.bed:BedReady")
            })
        }));
    }

    #[test]
    fn concurrent_signals_can_join_regardless_of_arrival_order() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Bed", "Model::Systems::PartDefinition", []),
                element("type.Hotend", "Model::Systems::PartDefinition", []),
                element("type.Printer", "Model::Systems::PartDefinition", []),
                element(
                    "individual.bed",
                    "Model::IndividualUsage",
                    [("declared_name", json!("bed")), ("type", json!("type.Bed"))],
                ),
                element(
                    "individual.hotend",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("hotend")),
                        ("type", json!("type.Hotend")),
                    ],
                ),
                element(
                    "individual.printer",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("printer")),
                        ("type", json!("type.Printer")),
                    ],
                ),
                state_element("state.Bed.Heating", "BedMachine", true),
                state_element("state.Bed.Ready", "BedMachine", false),
                state_element("state.Hotend.Heating", "HotendMachine", true),
                state_element("state.Hotend.Ready", "HotendMachine", false),
                state_element("state.Printer.Heating", "PrinterMachine", true),
                state_element("state.Printer.BedOnly", "PrinterMachine", false),
                state_element("state.Printer.HotendOnly", "PrinterMachine", false),
                state_element("state.Printer.Printing", "PrinterMachine", false),
                transition_element(
                    "transition.Bed.ready",
                    "BedMachine",
                    "state.Bed.Heating",
                    "state.Bed.Ready",
                    "finish_bed",
                    "event",
                    [(
                        "effects",
                        json!([{ "kind": "send_signal", "signal_type": "BedReady" }]),
                    )],
                ),
                transition_element(
                    "transition.Hotend.ready",
                    "HotendMachine",
                    "state.Hotend.Heating",
                    "state.Hotend.Ready",
                    "finish_hotend",
                    "event",
                    [(
                        "effects",
                        json!([{ "kind": "send_signal", "signal_type": "HotendReady" }]),
                    )],
                ),
                transition_element(
                    "transition.Printer.bed_first",
                    "PrinterMachine",
                    "state.Printer.Heating",
                    "state.Printer.BedOnly",
                    "BedReady",
                    "signal",
                    [],
                ),
                transition_element(
                    "transition.Printer.hotend_first",
                    "PrinterMachine",
                    "state.Printer.Heating",
                    "state.Printer.HotendOnly",
                    "HotendReady",
                    "signal",
                    [],
                ),
                transition_element(
                    "transition.Printer.bed_then_hotend",
                    "PrinterMachine",
                    "state.Printer.BedOnly",
                    "state.Printer.Printing",
                    "HotendReady",
                    "signal",
                    [],
                ),
                transition_element(
                    "transition.Printer.hotend_then_bed",
                    "PrinterMachine",
                    "state.Printer.HotendOnly",
                    "state.Printer.Printing",
                    "BedReady",
                    "signal",
                    [],
                ),
            ],
        })
        .unwrap();

        for (id, subjects) in [
            (
                "bed_first",
                vec![
                    ConcurrentSubjectScenario {
                        subject_id: "individual.bed".to_string(),
                        machine_id: "BedMachine".to_string(),
                        initial_state_id: None,
                        events: vec![StateMachineScenarioEvent {
                            id: "event.finish_bed".to_string(),
                            trigger: "finish_bed".to_string(),
                        }],
                    },
                    ConcurrentSubjectScenario {
                        subject_id: "individual.hotend".to_string(),
                        machine_id: "HotendMachine".to_string(),
                        initial_state_id: None,
                        events: vec![StateMachineScenarioEvent {
                            id: "event.finish_hotend".to_string(),
                            trigger: "finish_hotend".to_string(),
                        }],
                    },
                    ConcurrentSubjectScenario {
                        subject_id: "individual.printer".to_string(),
                        machine_id: "PrinterMachine".to_string(),
                        initial_state_id: None,
                        events: Vec::new(),
                    },
                ],
            ),
            (
                "hotend_first",
                vec![
                    ConcurrentSubjectScenario {
                        subject_id: "individual.hotend".to_string(),
                        machine_id: "HotendMachine".to_string(),
                        initial_state_id: None,
                        events: vec![StateMachineScenarioEvent {
                            id: "event.finish_hotend".to_string(),
                            trigger: "finish_hotend".to_string(),
                        }],
                    },
                    ConcurrentSubjectScenario {
                        subject_id: "individual.bed".to_string(),
                        machine_id: "BedMachine".to_string(),
                        initial_state_id: None,
                        events: vec![StateMachineScenarioEvent {
                            id: "event.finish_bed".to_string(),
                            trigger: "finish_bed".to_string(),
                        }],
                    },
                    ConcurrentSubjectScenario {
                        subject_id: "individual.printer".to_string(),
                        machine_id: "PrinterMachine".to_string(),
                        initial_state_id: None,
                        events: Vec::new(),
                    },
                ],
            ),
        ] {
            let trace = run_concurrent_simulation(
                &runtime,
                ConcurrentSimulationScenario {
                    id: format!("scenario.signal_join.{id}"),
                    subjects,
                    max_steps: 12,
                    step_duration_s: 1.0,
                    initial_values: BTreeMap::new(),
                },
            )
            .unwrap();

            assert!(
                trace.timeline.iter().any(|entry| {
                    entry
                        .states
                        .get("individual.printer")
                        .is_some_and(|states| states == &vec!["state.Printer.Printing".to_string()])
                }),
                "{id} did not reach Printing"
            );
        }
    }

    #[test]
    fn initial_configuration_enters_deep_initial_nested_state() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Controller", "Model::Systems::PartDefinition", []),
                element(
                    "individual.controller",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("controller")),
                        ("type", json!("type.Controller")),
                    ],
                ),
                state_element("state.Controller.Active", "ControllerMachine", true),
                nested_state_element(
                    "state.Controller.Active.Starting",
                    "ControllerMachine",
                    "state.Controller.Active",
                    true,
                ),
                nested_state_element(
                    "state.Controller.Active.Starting.Homing",
                    "ControllerMachine",
                    "state.Controller.Active.Starting",
                    true,
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.hsm.initial".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "individual.controller".to_string(),
                    machine_id: "ControllerMachine".to_string(),
                    initial_state_id: None,
                    events: Vec::new(),
                }],
                max_steps: 4,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
        )
        .unwrap();

        assert_eq!(
            trace
                .timeline
                .first()
                .unwrap()
                .states
                .get("individual.controller")
                .unwrap(),
            &vec![
                "state.Controller.Active".to_string(),
                "state.Controller.Active.Starting".to_string(),
                "state.Controller.Active.Starting.Homing".to_string(),
            ]
        );
    }

    #[test]
    fn transition_targeting_composite_state_enters_default_descendant() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Controller", "Model::Systems::PartDefinition", []),
                element(
                    "individual.controller",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("controller")),
                        ("type", json!("type.Controller")),
                    ],
                ),
                state_element("state.Controller.Off", "ControllerMachine", true),
                state_element("state.Controller.Active", "ControllerMachine", false),
                nested_state_element(
                    "state.Controller.Active.Starting",
                    "ControllerMachine",
                    "state.Controller.Active",
                    true,
                ),
                nested_state_element(
                    "state.Controller.Active.Running",
                    "ControllerMachine",
                    "state.Controller.Active",
                    false,
                ),
                transition_element(
                    "transition.Controller.start",
                    "ControllerMachine",
                    "state.Controller.Off",
                    "state.Controller.Active",
                    "start",
                    "event",
                    [],
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.hsm.composite_target".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "individual.controller".to_string(),
                    machine_id: "ControllerMachine".to_string(),
                    initial_state_id: None,
                    events: vec![StateMachineScenarioEvent {
                        id: "event.start".to_string(),
                        trigger: "start".to_string(),
                    }],
                }],
                max_steps: 4,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
        )
        .unwrap();

        assert!(trace.timeline.iter().any(|entry| {
            entry
                .states
                .get("individual.controller")
                .is_some_and(|states| {
                    states
                        == &vec![
                            "state.Controller.Active".to_string(),
                            "state.Controller.Active.Starting".to_string(),
                        ]
                })
        }));
    }

    #[test]
    fn hsm_sibling_transition_runs_leaf_exit_and_entry_without_parent_exit() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Controller", "Model::Systems::PartDefinition", []),
                element(
                    "individual.controller",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("controller")),
                        ("type", json!("type.Controller")),
                    ],
                ),
                element(
                    "state.Controller.Active",
                    "StateUsage",
                    [
                        ("declared_name", json!("Active")),
                        ("owning_type", json!("ControllerMachine")),
                        ("is_initial", json!(true)),
                        (
                            "exit_behavior",
                            json!({
                                "kind": "action_sequence",
                                "actions": [
                                    { "kind": "assign", "feature": "parentExited", "value": true }
                                ]
                            }),
                        ),
                    ],
                ),
                element(
                    "state.Controller.Active.Starting",
                    "StateUsage",
                    [
                        ("declared_name", json!("Starting")),
                        ("owning_type", json!("ControllerMachine")),
                        ("parent_state", json!("state.Controller.Active")),
                        ("is_initial", json!(true)),
                        (
                            "exit_behavior",
                            json!({
                                "kind": "action_sequence",
                                "actions": [
                                    { "kind": "assign", "feature": "startingExited", "value": true }
                                ]
                            }),
                        ),
                    ],
                ),
                element(
                    "state.Controller.Active.Running",
                    "StateUsage",
                    [
                        ("declared_name", json!("Running")),
                        ("owning_type", json!("ControllerMachine")),
                        ("parent_state", json!("state.Controller.Active")),
                        (
                            "entry_behavior",
                            json!({
                                "kind": "action_sequence",
                                "actions": [
                                    { "kind": "assign", "feature": "runningEntered", "value": true }
                                ]
                            }),
                        ),
                    ],
                ),
                transition_element(
                    "transition.Controller.ready",
                    "ControllerMachine",
                    "state.Controller.Active.Starting",
                    "state.Controller.Active.Running",
                    "ready",
                    "event",
                    [],
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.hsm.sibling".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "individual.controller".to_string(),
                    machine_id: "ControllerMachine".to_string(),
                    initial_state_id: None,
                    events: vec![StateMachineScenarioEvent {
                        id: "event.ready".to_string(),
                        trigger: "ready".to_string(),
                    }],
                }],
                max_steps: 4,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
        )
        .unwrap();

        let final_values = &trace.timeline.last().unwrap().values;
        assert_eq!(
            final_values.get(&(
                "individual.controller".to_string(),
                "startingExited".to_string()
            )),
            Some(&json!(true))
        );
        assert_eq!(
            final_values.get(&(
                "individual.controller".to_string(),
                "runningEntered".to_string()
            )),
            Some(&json!(true))
        );
        assert_eq!(
            final_values.get(&(
                "individual.controller".to_string(),
                "parentExited".to_string()
            )),
            None
        );
    }

    #[test]
    fn concurrent_entry_behavior_can_emit_signal() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Bed", "Model::Systems::PartDefinition", []),
                element("type.Printer", "Model::Systems::PartDefinition", []),
                element(
                    "individual.bed",
                    "Model::IndividualUsage",
                    [("declared_name", json!("bed")), ("type", json!("type.Bed"))],
                ),
                element(
                    "individual.printer",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("printer")),
                        ("type", json!("type.Printer")),
                    ],
                ),
                state_element("state.Bed.Heating", "BedMachine", true),
                element(
                    "state.Bed.Ready",
                    "StateUsage",
                    [
                        ("declared_name", json!("Ready")),
                        ("owning_type", json!("BedMachine")),
                        (
                            "entry_behavior",
                            json!({
                                "kind": "action_sequence",
                                "actions": [
                                    {
                                        "kind": "send_signal",
                                        "signal_type": "BedReady",
                                        "target": "individual.printer"
                                    }
                                ]
                            }),
                        ),
                    ],
                ),
                state_element("state.Printer.Heating", "PrinterMachine", true),
                state_element("state.Printer.Printing", "PrinterMachine", false),
                transition_element(
                    "transition.Bed.ready",
                    "BedMachine",
                    "state.Bed.Heating",
                    "state.Bed.Ready",
                    "finish",
                    "event",
                    [],
                ),
                transition_element(
                    "transition.Printer.print",
                    "PrinterMachine",
                    "state.Printer.Heating",
                    "state.Printer.Printing",
                    "BedReady",
                    "signal",
                    [],
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.hsm.entry_signal".to_string(),
                subjects: vec![
                    ConcurrentSubjectScenario {
                        subject_id: "individual.bed".to_string(),
                        machine_id: "BedMachine".to_string(),
                        initial_state_id: None,
                        events: vec![StateMachineScenarioEvent {
                            id: "event.finish".to_string(),
                            trigger: "finish".to_string(),
                        }],
                    },
                    ConcurrentSubjectScenario {
                        subject_id: "individual.printer".to_string(),
                        machine_id: "PrinterMachine".to_string(),
                        initial_state_id: None,
                        events: Vec::new(),
                    },
                ],
                max_steps: 8,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
        )
        .unwrap();

        assert!(trace.timeline.iter().any(|entry| {
            entry
                .states
                .get("individual.printer")
                .is_some_and(|states| states == &vec!["state.Printer.Printing".to_string()])
        }));
    }

    #[test]
    fn orthogonal_state_enters_all_initial_children() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Controller", "Model::Systems::PartDefinition", []),
                element(
                    "individual.controller",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("controller")),
                        ("type", json!("type.Controller")),
                    ],
                ),
                element(
                    "state.Controller.Active",
                    "StateUsage",
                    [
                        ("declared_name", json!("Active")),
                        ("owning_type", json!("ControllerMachine")),
                        ("is_initial", json!(true)),
                        ("is_orthogonal", json!(true)),
                    ],
                ),
                nested_state_element(
                    "state.Controller.Active.RegionAIdle",
                    "ControllerMachine",
                    "state.Controller.Active",
                    true,
                ),
                nested_state_element(
                    "state.Controller.Active.RegionBIdle",
                    "ControllerMachine",
                    "state.Controller.Active",
                    true,
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.hsm.orthogonal_initial".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "individual.controller".to_string(),
                    machine_id: "ControllerMachine".to_string(),
                    initial_state_id: None,
                    events: Vec::new(),
                }],
                max_steps: 4,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
        )
        .unwrap();

        let states = trace
            .timeline
            .first()
            .unwrap()
            .states
            .get("individual.controller")
            .unwrap();
        assert!(states.contains(&"state.Controller.Active".to_string()));
        assert!(states.contains(&"state.Controller.Active.RegionAIdle".to_string()));
        assert!(states.contains(&"state.Controller.Active.RegionBIdle".to_string()));
    }

    #[test]
    fn orthogonal_branch_transition_preserves_other_branch() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Controller", "Model::Systems::PartDefinition", []),
                element(
                    "individual.controller",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("controller")),
                        ("type", json!("type.Controller")),
                    ],
                ),
                element(
                    "state.Controller.Active",
                    "StateUsage",
                    [
                        ("declared_name", json!("Active")),
                        ("owning_type", json!("ControllerMachine")),
                        ("is_initial", json!(true)),
                        ("is_orthogonal", json!(true)),
                    ],
                ),
                nested_state_element(
                    "state.Controller.Active.RegionAIdle",
                    "ControllerMachine",
                    "state.Controller.Active",
                    true,
                ),
                nested_state_element(
                    "state.Controller.Active.RegionARunning",
                    "ControllerMachine",
                    "state.Controller.Active",
                    false,
                ),
                nested_state_element(
                    "state.Controller.Active.RegionBIdle",
                    "ControllerMachine",
                    "state.Controller.Active",
                    true,
                ),
                transition_element(
                    "transition.Controller.start_a",
                    "ControllerMachine",
                    "state.Controller.Active.RegionAIdle",
                    "state.Controller.Active.RegionARunning",
                    "start_a",
                    "event",
                    [],
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.hsm.orthogonal_branch".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "individual.controller".to_string(),
                    machine_id: "ControllerMachine".to_string(),
                    initial_state_id: None,
                    events: vec![StateMachineScenarioEvent {
                        id: "event.start_a".to_string(),
                        trigger: "start_a".to_string(),
                    }],
                }],
                max_steps: 4,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
        )
        .unwrap();

        let states = trace
            .timeline
            .last()
            .unwrap()
            .states
            .get("individual.controller")
            .unwrap();
        assert!(states.contains(&"state.Controller.Active.RegionARunning".to_string()));
        assert!(states.contains(&"state.Controller.Active.RegionBIdle".to_string()));
        assert!(!states.contains(&"state.Controller.Active.RegionAIdle".to_string()));
    }

    #[test]
    fn shallow_history_target_restores_last_active_child() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.Controller", "Model::Systems::PartDefinition", []),
                element(
                    "individual.controller",
                    "Model::IndividualUsage",
                    [
                        ("declared_name", json!("controller")),
                        ("type", json!("type.Controller")),
                    ],
                ),
                state_element("state.Controller.Off", "ControllerMachine", true),
                state_element("state.Controller.Active", "ControllerMachine", false),
                nested_state_element(
                    "state.Controller.Active.A",
                    "ControllerMachine",
                    "state.Controller.Active",
                    true,
                ),
                nested_state_element(
                    "state.Controller.Active.B",
                    "ControllerMachine",
                    "state.Controller.Active",
                    false,
                ),
                element(
                    "state.Controller.Active.History",
                    "StateUsage",
                    [
                        ("declared_name", json!("History")),
                        ("owning_type", json!("ControllerMachine")),
                        ("parent_state", json!("state.Controller.Active")),
                        ("is_history", json!(true)),
                    ],
                ),
                transition_element(
                    "transition.Controller.start",
                    "ControllerMachine",
                    "state.Controller.Off",
                    "state.Controller.Active",
                    "start",
                    "event",
                    [],
                ),
                transition_element(
                    "transition.Controller.to_b",
                    "ControllerMachine",
                    "state.Controller.Active.A",
                    "state.Controller.Active.B",
                    "to_b",
                    "event",
                    [],
                ),
                transition_element(
                    "transition.Controller.stop",
                    "ControllerMachine",
                    "state.Controller.Active",
                    "state.Controller.Off",
                    "stop",
                    "event",
                    [],
                ),
                transition_element(
                    "transition.Controller.resume",
                    "ControllerMachine",
                    "state.Controller.Off",
                    "state.Controller.Active.History",
                    "resume",
                    "event",
                    [],
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.hsm.history".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "individual.controller".to_string(),
                    machine_id: "ControllerMachine".to_string(),
                    initial_state_id: None,
                    events: vec![
                        StateMachineScenarioEvent {
                            id: "event.start".to_string(),
                            trigger: "start".to_string(),
                        },
                        StateMachineScenarioEvent {
                            id: "event.to_b".to_string(),
                            trigger: "to_b".to_string(),
                        },
                        StateMachineScenarioEvent {
                            id: "event.stop".to_string(),
                            trigger: "stop".to_string(),
                        },
                        StateMachineScenarioEvent {
                            id: "event.resume".to_string(),
                            trigger: "resume".to_string(),
                        },
                    ],
                }],
                max_steps: 8,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
        )
        .unwrap();

        let states = trace
            .timeline
            .last()
            .unwrap()
            .states
            .get("individual.controller")
            .unwrap();
        assert!(states.contains(&"state.Controller.Active".to_string()));
        assert!(states.contains(&"state.Controller.Active.B".to_string()));
        assert!(!states.contains(&"state.Controller.Active.A".to_string()));
    }

    #[test]
    fn textual_state_do_action_lowers_to_rate_integration_behavior() {
        let stdlib = load_sysml_baseline().unwrap();
        let document = compile_sysml_text(
            r#"
            package Demo {
                import ScalarValues::*;

                part def Bed {
                    attribute temperature : Real;
                    attribute heatRate : Real;

                    state lifecycle {
                        state Heating {
                            do action integrate {
                                assert constraint {
                                    temperature == temperature + heatRate * duration;
                                }
                            }
                        }
                    }
                }
            }
            "#,
            "state-do-rate.sysml",
            &stdlib,
        )
        .unwrap();

        let heating = document
            .elements
            .iter()
            .find(|element| element.id.ends_with(".Bed.lifecycle.Heating"))
            .expect("Heating state");
        assert_eq!(
            heating.properties.get("do_behavior"),
            Some(&json!({
                "kind": "rate_integration",
                "rates": [
                    { "feature": "temperature", "rate_feature": "heatRate" }
                ]
            }))
        );
    }

    #[test]
    fn concurrent_trace_states_map_contains_all_subjects() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("type.A", "Model::Systems::PartDefinition", []),
                element("type.B", "Model::Systems::PartDefinition", []),
                element(
                    "individual.a",
                    "Model::IndividualUsage",
                    [("declared_name", json!("a")), ("type", json!("type.A"))],
                ),
                element(
                    "individual.b",
                    "Model::IndividualUsage",
                    [("declared_name", json!("b")), ("type", json!("type.B"))],
                ),
                state_element("state.A.one", "MachineA", true),
                state_element("state.A.two", "MachineA", false),
                state_element("state.B.one", "MachineB", true),
                state_element("state.B.two", "MachineB", false),
                transition_element(
                    "transition.A.go",
                    "MachineA",
                    "state.A.one",
                    "state.A.two",
                    "go",
                    "event",
                    [],
                ),
                transition_element(
                    "transition.B.run",
                    "MachineB",
                    "state.B.one",
                    "state.B.two",
                    "run",
                    "event",
                    [],
                ),
            ],
        })
        .unwrap();

        let trace = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.concurrent.states".to_string(),
                subjects: vec![
                    ConcurrentSubjectScenario {
                        subject_id: "individual.a".to_string(),
                        machine_id: "MachineA".to_string(),
                        initial_state_id: None,
                        events: vec![StateMachineScenarioEvent {
                            id: "event.go".to_string(),
                            trigger: "go".to_string(),
                        }],
                    },
                    ConcurrentSubjectScenario {
                        subject_id: "individual.b".to_string(),
                        machine_id: "MachineB".to_string(),
                        initial_state_id: None,
                        events: vec![StateMachineScenarioEvent {
                            id: "event.run".to_string(),
                            trigger: "run".to_string(),
                        }],
                    },
                ],
                max_steps: 8,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
        )
        .unwrap();

        assert!(trace.timeline.iter().all(|entry| {
            entry.states.contains_key("individual.a") && entry.states.contains_key("individual.b")
        }));
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

    fn scenario(
        events: Vec<StateMachineScenarioEvent>,
        values: BTreeMap<(String, String), Value>,
        step_duration_s: f64,
        max_steps: usize,
    ) -> HybridSimulationScenario {
        HybridSimulationScenario {
            id: "scenario.test".to_string(),
            subject: SimulationSubject {
                id: "individual.test".to_string(),
                type_id: Some("type.Test".to_string()),
            },
            machine_id: "TestMachine".to_string(),
            initial_state_id: None,
            events,
            max_steps,
            values,
            step_duration_s,
        }
    }

    fn simple_machine<const N: usize>(transitions: [KirElement; N]) -> KirDocument {
        let mut elements = vec![
            element("type.Test", "Model::Systems::PartDefinition", []),
            element(
                "individual.test",
                "Model::IndividualUsage",
                [
                    ("declared_name", json!("test")),
                    ("type", json!("type.Test")),
                ],
            ),
            state_element("state.test.A", "TestMachine", true),
            state_element("state.test.B", "TestMachine", false),
            state_element("state.test.C", "TestMachine", false),
        ];
        elements.extend(transitions);
        KirDocument {
            metadata: BTreeMap::new(),
            elements,
        }
    }

    fn state_element(id: &str, owner: &str, initial: bool) -> KirElement {
        element(
            id,
            "StateUsage",
            [
                ("declared_name", json!(id)),
                ("owning_type", json!(owner)),
                ("is_initial", json!(initial)),
            ],
        )
    }

    fn nested_state_element(id: &str, owner: &str, parent: &str, initial: bool) -> KirElement {
        element(
            id,
            "StateUsage",
            [
                ("declared_name", json!(id)),
                ("owning_type", json!(owner)),
                ("parent_state", json!(parent)),
                ("is_initial", json!(initial)),
            ],
        )
    }

    fn transition_element<const N: usize>(
        id: &str,
        owner: &str,
        source: &str,
        target: &str,
        trigger: &str,
        trigger_kind: &str,
        extra_properties: [(&str, Value); N],
    ) -> KirElement {
        let mut properties = BTreeMap::from([
            ("owning_type".to_string(), json!(owner)),
            ("source".to_string(), json!(source)),
            ("target".to_string(), json!(target)),
            ("trigger".to_string(), json!(trigger)),
            ("trigger_kind".to_string(), json!(trigger_kind)),
        ]);
        properties.extend(
            extra_properties
                .into_iter()
                .map(|(key, value)| (key.to_string(), value)),
        );
        KirElement {
            id: id.to_string(),
            kind: "TransitionUsage".to_string(),
            layer: 0,
            properties,
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
        greater_equal_path_segments([feature], threshold)
    }

    fn greater_equal_path_segments<const N: usize>(segments: [&str; N], threshold: f64) -> Value {
        let segments = segments.to_vec();
        json!({
            "kind": "binary",
            "op": "greater_equal",
            "left": {
                "kind": "path",
                "root": "self",
                "segments": segments
            },
            "right": {
                "kind": "literal",
                "value": threshold
            }
        })
    }
}
