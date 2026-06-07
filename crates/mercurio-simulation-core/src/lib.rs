use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationClockConfig {
    pub max_time_s: f64,
    pub fixed_step_s: f64,
    pub sample_interval_s: f64,
    pub change_loop_limit: usize,
}

impl Default for SimulationClockConfig {
    fn default() -> Self {
        Self {
            max_time_s: 300.0,
            fixed_step_s: 1.0,
            sample_interval_s: 1.0,
            change_loop_limit: 20,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationSubject {
    pub id: String,
    pub type_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridSimulationScenario {
    pub id: String,
    pub subject: SimulationSubject,
    pub machine_id: String,
    pub initial_state_id: Option<String>,
    pub events: Vec<SimulationEvent>,
    pub max_steps: usize,
    pub values: BTreeMap<(String, String), Value>,
    #[serde(default = "default_step_duration")]
    pub step_duration_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConcurrentSimulationScenario {
    pub id: String,
    pub subjects: Vec<ConcurrentSubjectScenario>,
    pub max_steps: usize,
    #[serde(default = "default_step_duration")]
    pub step_duration_s: f64,
    #[serde(with = "tuple_value_map")]
    pub initial_values: BTreeMap<(String, String), Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConcurrentSubjectScenario {
    pub subject_id: String,
    pub machine_id: String,
    #[serde(default)]
    pub initial_state_id: Option<String>,
    #[serde(default)]
    pub events: Vec<SimulationEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationEvent {
    pub id: String,
    pub trigger: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisCaseInfo {
    pub id: String,
    pub label: String,
    pub subject_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridSimulationReport {
    pub scenario_id: String,
    pub subject_id: String,
    pub machine_id: String,
    pub status: HybridSimulationStatus,
    pub active_configuration: Vec<String>,
    pub values: BTreeMap<(String, String), Value>,
    pub critical_events: Vec<CriticalSimulationEvent>,
    pub trace: Vec<HybridSimulationTraceEntry>,
    #[serde(default)]
    pub rate_channels: BTreeSet<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationTrace {
    pub scenario_id: String,
    pub subject_id: String,
    pub channels: Vec<TraceChannel>,
    pub timeline: Vec<TraceEntry>,
    pub status: HybridSimulationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceChannel {
    pub id: String,
    pub unit: Option<String>,
    pub source: TraceChannelSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceChannelSource {
    StateMachine,
    RateEffect,
    AssignEffect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceEntry {
    pub t: f64,
    pub states: BTreeMap<String, Vec<String>>,
    #[serde(with = "tuple_value_map")]
    pub values: BTreeMap<(String, String), Value>,
    pub events: Vec<TraceEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    pub kind: String,
    pub transition_id: Option<String>,
    pub trigger: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridSimulationTraceEntry {
    pub step: usize,
    #[serde(default)]
    pub t: f64,
    pub event_id: Option<String>,
    pub trigger: Option<String>,
    pub transition_id: Option<String>,
    pub before: Vec<String>,
    pub after: Vec<String>,
    #[serde(default, with = "tuple_value_map")]
    pub values: BTreeMap<(String, String), Value>,
    pub critical_events: Vec<CriticalSimulationEvent>,
    pub explanation: String,
}

impl HybridSimulationReport {
    pub fn to_trace(&self) -> SimulationTrace {
        let mut channel_ids: BTreeSet<(String, String)> = BTreeSet::new();
        for entry in &self.trace {
            for key in entry.values.keys() {
                channel_ids.insert(key.clone());
            }
        }

        let channels = channel_ids
            .into_iter()
            .map(|(subject, feature)| {
                let source = if self
                    .rate_channels
                    .contains(&(subject.clone(), feature.clone()))
                {
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

        let timeline = self
            .trace
            .iter()
            .map(|entry| {
                let mut states = BTreeMap::new();
                states.insert(self.subject_id.clone(), entry.after.clone());

                let events = entry
                    .transition_id
                    .iter()
                    .map(|tid| TraceEvent {
                        kind: "transition".to_string(),
                        transition_id: Some(tid.clone()),
                        trigger: entry.trigger.clone(),
                    })
                    .collect();

                TraceEntry {
                    t: entry.t,
                    states,
                    values: entry.values.clone(),
                    events,
                }
            })
            .collect();

        SimulationTrace {
            scenario_id: self.scenario_id.clone(),
            subject_id: self.subject_id.clone(),
            channels,
            timeline,
            status: self.status,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationModel {
    pub id: String,
    pub machines: Vec<SimulationStateMachine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationStateMachine {
    pub id: String,
    pub label: String,
    pub states: Vec<SimulationState>,
    pub transitions: Vec<SimulationTransition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationState {
    pub id: String,
    pub label: String,
    pub parent_state_id: Option<String>,
    pub is_initial: bool,
    pub is_final: bool,
    pub is_orthogonal: bool,
    pub is_history: bool,
    pub entry_behavior: Option<SimulationActionSequence>,
    pub exit_behavior: Option<SimulationActionSequence>,
    pub do_behavior: Option<StateDoBehavior>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationTransition {
    pub id: String,
    pub source: String,
    pub target: String,
    pub trigger: SimulationTrigger,
    pub guard: Option<SimulationGuard>,
    pub effects: Vec<SimulationEffect>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationTriggerKind {
    Event,
    Signal,
    Time,
    After,
    Change,
    Completion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationTrigger {
    pub kind: SimulationTriggerKind,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimulationGuard {
    ExpressionIr(Value),
    RuntimeFeature(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimulationEffect {
    Assign(AssignEffect),
    EmitSignal(SignalEffect),
    Log(LogEffect),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssignEffect {
    pub feature: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalEffect {
    pub signal_type: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEffect {
    pub kind: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationActionSequence {
    pub actions: Vec<SimulationEffect>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StateDoBehavior {
    RateIntegration { rates: Vec<SimulationRate> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationRate {
    pub feature: String,
    pub source: SimulationRateSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimulationRateSource {
    Constant(f64),
    Feature(String),
    ExpressionIr(Value),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationProfileFinding {
    pub code: String,
    pub message: String,
    pub machine_id: Option<String>,
    pub element_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationProfileError {
    pub findings: Vec<SimulationProfileFinding>,
}

impl fmt::Display for SimulationProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid simulation profile: {} finding(s)",
            self.findings.len()
        )
    }
}

impl std::error::Error for SimulationProfileError {}

#[derive(Debug)]
pub enum CoreSimulationError {
    InvalidProfile(SimulationProfileError),
    MissingStateMachine(String),
    MissingSubject(String),
    MissingInitialState(String),
}

impl fmt::Display for CoreSimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile(error) => write!(f, "{error}"),
            Self::MissingStateMachine(id) => write!(f, "missing state machine: {id}"),
            Self::MissingSubject(id) => write!(f, "missing simulation subject: {id}"),
            Self::MissingInitialState(id) => write!(f, "missing initial state: {id}"),
        }
    }
}

impl std::error::Error for CoreSimulationError {}

impl From<SimulationProfileError> for CoreSimulationError {
    fn from(error: SimulationProfileError) -> Self {
        Self::InvalidProfile(error)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct CoreSubjectRunState<'m> {
    subject_id: String,
    machine: &'m SimulationStateMachine,
    active: Vec<String>,
    event_index: usize,
    events: Vec<SimulationEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CorePendingSignal {
    source_subject_id: String,
    signal_type: String,
    target: Option<String>,
}

pub fn run_concurrent_simulation_model(
    model: &SimulationModel,
    scenario: ConcurrentSimulationScenario,
    clock: SimulationClockConfig,
) -> Result<SimulationTrace, CoreSimulationError> {
    validate_simulation_model(model)?;
    let mut subjects = Vec::<CoreSubjectRunState<'_>>::new();
    for subject in &scenario.subjects {
        if subject.subject_id.is_empty() {
            return Err(CoreSimulationError::MissingSubject(
                subject.subject_id.clone(),
            ));
        }
        let machine = model
            .machines
            .iter()
            .find(|machine| machine.id == subject.machine_id || machine.label == subject.machine_id)
            .ok_or_else(|| CoreSimulationError::MissingStateMachine(subject.machine_id.clone()))?;
        let active = initial_configuration(machine, subject.initial_state_id.as_deref())
            .ok_or_else(|| CoreSimulationError::MissingInitialState(subject.machine_id.clone()))?;
        subjects.push(CoreSubjectRunState {
            subject_id: subject.subject_id.clone(),
            machine,
            active,
            event_index: 0,
            events: subject.events.clone(),
        });
    }

    let mut values = scenario.initial_values.clone();
    let mut pending_signals = VecDeque::<CorePendingSignal>::new();
    let mut history = BTreeMap::<(String, String), String>::new();
    let mut t = 0.0;
    let mut step = 0usize;
    for subject in &subjects {
        for state_id in &subject.active {
            apply_state_behavior(
                subject.machine,
                state_id,
                &subject.subject_id,
                &mut values,
                &mut pending_signals,
            );
        }
    }

    let mut timeline = vec![make_core_entry(t, &subjects, &values, Vec::new())];
    let max_steps = scenario.max_steps.max(1);
    while step < max_steps && t <= clock.max_time_s {
        let mut fired = false;
        let mut events = Vec::<TraceEvent>::new();

        if deliver_pending_signals(
            &mut subjects,
            &mut values,
            &mut pending_signals,
            &mut history,
            &mut step,
            max_steps,
            &mut events,
        )? {
            fired = true;
        }

        for subject in subjects.iter_mut() {
            if step >= max_steps || subject.event_index >= subject.events.len() {
                continue;
            }
            let event = subject.events[subject.event_index].clone();
            subject.event_index += 1;
            let Some(transition) = select_transition(
                subject.machine,
                &subject.active,
                SimulationTriggerKind::Event,
                &event.trigger,
            )
            .cloned() else {
                continue;
            };
            step += 1;
            t += clock.fixed_step_s.max(0.0);
            let before = subject.active.clone();
            apply_effects(
                &transition.effects,
                &subject.subject_id,
                &mut values,
                &mut pending_signals,
            );
            subject.active = apply_state_change(
                subject.machine,
                &subject.subject_id,
                &before,
                &transition.source,
                &transition.target,
                &mut values,
                &mut pending_signals,
                &mut history,
            )?;
            events.push(TraceEvent {
                kind: "transition".to_string(),
                transition_id: Some(transition.id),
                trigger: Some(event.trigger),
            });
            fired = true;
        }

        if fired {
            timeline.push(make_core_entry(t, &subjects, &values, events));
        } else {
            break;
        }
    }

    let primary_subject_id = scenario
        .subjects
        .first()
        .map(|subject| subject.subject_id.clone())
        .unwrap_or_default();
    let channels = values
        .keys()
        .map(|(subject, feature)| TraceChannel {
            id: format!("{subject}.{feature}"),
            unit: None,
            source: TraceChannelSource::AssignEffect,
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

fn make_core_entry(
    t: f64,
    subjects: &[CoreSubjectRunState<'_>],
    values: &BTreeMap<(String, String), Value>,
    events: Vec<TraceEvent>,
) -> TraceEntry {
    TraceEntry {
        t,
        states: subjects
            .iter()
            .map(|subject| (subject.subject_id.clone(), subject.active.clone()))
            .collect(),
        values: values.clone(),
        events,
    }
}

fn deliver_pending_signals(
    subjects: &mut [CoreSubjectRunState<'_>],
    values: &mut BTreeMap<(String, String), Value>,
    pending_signals: &mut VecDeque<CorePendingSignal>,
    history: &mut BTreeMap<(String, String), String>,
    step: &mut usize,
    max_steps: usize,
    events: &mut Vec<TraceEvent>,
) -> Result<bool, CoreSimulationError> {
    let mut fired = false;
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
            let Some(transition) = select_transition(
                subject.machine,
                &subject.active,
                SimulationTriggerKind::Signal,
                &signal.signal_type,
            )
            .cloned() else {
                continue;
            };
            *step += 1;
            let before = subject.active.clone();
            apply_effects(
                &transition.effects,
                &subject.subject_id,
                values,
                pending_signals,
            );
            subject.active = apply_state_change(
                subject.machine,
                &subject.subject_id,
                &before,
                &transition.source,
                &transition.target,
                values,
                pending_signals,
                history,
            )?;
            events.push(TraceEvent {
                kind: "transition".to_string(),
                transition_id: Some(transition.id.clone()),
                trigger: Some(format!(
                    "signal:{}:{}",
                    signal.source_subject_id, signal.signal_type
                )),
            });
            consumed = true;
            fired = true;
        }
        if !consumed {
            pending_signals.push_back(signal);
        }
    }
    Ok(fired)
}

fn signal_targets_subject(signal: &CorePendingSignal, subject_id: &str) -> bool {
    match signal.target.as_deref() {
        Some(target) => target == subject_id,
        None => true,
    }
}

fn select_transition<'a>(
    machine: &'a SimulationStateMachine,
    active: &[String],
    kind: SimulationTriggerKind,
    trigger: &str,
) -> Option<&'a SimulationTransition> {
    active.iter().rev().find_map(|state_id| {
        machine.transitions.iter().find(|transition| {
            transition.source == *state_id
                && transition.trigger.kind == kind
                && transition.trigger.value.as_deref() == Some(trigger)
                && transition.guard.is_none()
        })
    })
}

fn apply_state_change(
    machine: &SimulationStateMachine,
    subject_id: &str,
    before: &[String],
    source_state_id: &str,
    target_state_id: &str,
    values: &mut BTreeMap<(String, String), Value>,
    pending_signals: &mut VecDeque<CorePendingSignal>,
    history: &mut BTreeMap<(String, String), String>,
) -> Result<Vec<String>, CoreSimulationError> {
    let resolved_target = resolve_history_target(machine, subject_id, target_state_id, history)
        .unwrap_or_else(|| target_state_id.to_string());
    let target_configuration = initial_configuration(machine, Some(&resolved_target))
        .ok_or_else(|| CoreSimulationError::MissingInitialState(resolved_target.clone()))?;
    let source_path = ancestor_path(machine, source_state_id)
        .ok_or_else(|| CoreSimulationError::MissingInitialState(source_state_id.to_string()))?;
    let target_path = ancestor_path(machine, &resolved_target)
        .ok_or_else(|| CoreSimulationError::MissingInitialState(resolved_target.clone()))?;
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

    for state_id in exit_states.iter().rev() {
        apply_exit_behavior(machine, state_id, subject_id, values, pending_signals);
        record_shallow_history(machine, subject_id, state_id, history);
    }
    for state_id in &entry_states {
        apply_state_behavior(machine, state_id, subject_id, values, pending_signals);
    }
    Ok(after)
}

fn apply_state_behavior(
    machine: &SimulationStateMachine,
    state_id: &str,
    subject_id: &str,
    values: &mut BTreeMap<(String, String), Value>,
    pending_signals: &mut VecDeque<CorePendingSignal>,
) {
    let Some(state) = machine.states.iter().find(|state| state.id == state_id) else {
        return;
    };
    if let Some(behavior) = &state.entry_behavior {
        apply_effects(&behavior.actions, subject_id, values, pending_signals);
    }
}

fn apply_exit_behavior(
    machine: &SimulationStateMachine,
    state_id: &str,
    subject_id: &str,
    values: &mut BTreeMap<(String, String), Value>,
    pending_signals: &mut VecDeque<CorePendingSignal>,
) {
    let Some(state) = machine.states.iter().find(|state| state.id == state_id) else {
        return;
    };
    if let Some(behavior) = &state.exit_behavior {
        apply_effects(&behavior.actions, subject_id, values, pending_signals);
    }
}

fn apply_effects(
    effects: &[SimulationEffect],
    subject_id: &str,
    values: &mut BTreeMap<(String, String), Value>,
    pending_signals: &mut VecDeque<CorePendingSignal>,
) {
    for effect in effects {
        match effect {
            SimulationEffect::Assign(effect) => {
                values.insert(
                    (subject_id.to_string(), effect.feature.clone()),
                    effect.value.clone(),
                );
            }
            SimulationEffect::EmitSignal(effect) => {
                pending_signals.push_back(CorePendingSignal {
                    source_subject_id: subject_id.to_string(),
                    signal_type: effect.signal_type.clone(),
                    target: effect.target.clone(),
                });
            }
            SimulationEffect::Log(_) => {}
        }
    }
}

fn initial_configuration(
    machine: &SimulationStateMachine,
    initial_state_id: Option<&str>,
) -> Option<Vec<String>> {
    if let Some(state_id) = initial_state_id {
        return enter_state_configuration(machine, state_id);
    }
    let root = machine
        .states
        .iter()
        .find(|state| state.parent_state_id.is_none() && state.is_initial)?;
    enter_state_configuration(machine, &root.id)
}

fn enter_state_configuration(
    machine: &SimulationStateMachine,
    state_id: &str,
) -> Option<Vec<String>> {
    let mut configuration = ancestor_path(machine, state_id)?;
    append_default_descendants(machine, state_id, &mut configuration);
    Some(configuration)
}

fn append_default_descendants(
    machine: &SimulationStateMachine,
    state_id: &str,
    configuration: &mut Vec<String>,
) {
    let Some(state) = machine.states.iter().find(|state| state.id == state_id) else {
        return;
    };
    if state.is_orthogonal {
        for child in default_orthogonal_children(machine, state_id) {
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

fn ancestor_path(machine: &SimulationStateMachine, state_id: &str) -> Option<Vec<String>> {
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
    machine: &'a SimulationStateMachine,
    parent_id: &str,
) -> Option<&'a SimulationState> {
    machine
        .states
        .iter()
        .filter(|state| state.parent_state_id.as_deref() == Some(parent_id) && !state.is_history)
        .find(|state| state.is_initial)
        .or_else(|| {
            machine.states.iter().find(|state| {
                state.parent_state_id.as_deref() == Some(parent_id) && !state.is_history
            })
        })
}

fn default_orthogonal_children<'a>(
    machine: &'a SimulationStateMachine,
    parent_id: &str,
) -> Vec<&'a SimulationState> {
    let initial = machine
        .states
        .iter()
        .filter(|state| {
            state.parent_state_id.as_deref() == Some(parent_id)
                && state.is_initial
                && !state.is_history
        })
        .collect::<Vec<_>>();
    if !initial.is_empty() {
        return initial;
    }
    machine
        .states
        .iter()
        .filter(|state| state.parent_state_id.as_deref() == Some(parent_id) && !state.is_history)
        .collect()
}

fn is_descendant_of(machine: &SimulationStateMachine, state_id: &str, ancestor_id: &str) -> bool {
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

fn resolve_history_target(
    machine: &SimulationStateMachine,
    subject_id: &str,
    target_state_id: &str,
    history: &BTreeMap<(String, String), String>,
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
        .get(&(subject_id.to_string(), parent_id.clone()))
        .cloned()
        .or_else(|| default_child_state(machine, parent_id).map(|state| state.id.clone()))
}

fn record_shallow_history(
    machine: &SimulationStateMachine,
    subject_id: &str,
    state_id: &str,
    history: &mut BTreeMap<(String, String), String>,
) {
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

pub fn validate_simulation_model(model: &SimulationModel) -> Result<(), SimulationProfileError> {
    let mut findings = Vec::new();
    if model.machines.is_empty() {
        findings.push(finding(
            "model.no_machines",
            "Simulation model has no state machines.",
            None,
            None,
        ));
    }

    for machine in &model.machines {
        validate_machine(machine, &mut findings);
    }

    if findings.is_empty() {
        Ok(())
    } else {
        Err(SimulationProfileError { findings })
    }
}

fn validate_machine(
    machine: &SimulationStateMachine,
    findings: &mut Vec<SimulationProfileFinding>,
) {
    let mut state_ids = BTreeSet::new();
    for state in &machine.states {
        if !state_ids.insert(state.id.clone()) {
            findings.push(finding(
                "state.duplicate_id",
                "State IDs must be unique within a simulation machine.",
                Some(&machine.id),
                Some(&state.id),
            ));
        }
    }

    if machine.states.is_empty() {
        findings.push(finding(
            "machine.no_states",
            "State machine has no states.",
            Some(&machine.id),
            None,
        ));
    }

    let top_initial_count = machine
        .states
        .iter()
        .filter(|state| state.parent_state_id.is_none() && state.is_initial)
        .count();
    if top_initial_count == 0 && !machine.states.is_empty() {
        findings.push(finding(
            "machine.no_initial_state",
            "State machine must have a top-level initial state.",
            Some(&machine.id),
            None,
        ));
    }
    if top_initial_count > 1 {
        findings.push(finding(
            "machine.multiple_initial_states",
            "State machine has more than one top-level initial state.",
            Some(&machine.id),
            None,
        ));
    }

    for state in &machine.states {
        if let Some(parent_id) = &state.parent_state_id
            && !state_ids.contains(parent_id)
        {
            findings.push(finding(
                "state.missing_parent",
                "State parent must reference another state in the same machine.",
                Some(&machine.id),
                Some(&state.id),
            ));
        }
    }

    for parent in &machine.states {
        let initial_child_count = machine
            .states
            .iter()
            .filter(|state| {
                state.parent_state_id.as_deref() == Some(parent.id.as_str()) && state.is_initial
            })
            .count();
        if initial_child_count > 1 && !parent.is_orthogonal {
            findings.push(finding(
                "state.multiple_initial_children",
                "Compound state has multiple initial children but is not marked orthogonal.",
                Some(&machine.id),
                Some(&parent.id),
            ));
        }
    }

    let mut transition_keys =
        BTreeMap::<(String, SimulationTriggerKind, Option<String>), usize>::new();
    for transition in &machine.transitions {
        if !state_ids.contains(&transition.source) {
            findings.push(finding(
                "transition.missing_source",
                "Transition source must reference a state in the same machine.",
                Some(&machine.id),
                Some(&transition.id),
            ));
        }
        if !state_ids.contains(&transition.target) {
            findings.push(finding(
                "transition.missing_target",
                "Transition target must reference a state in the same machine.",
                Some(&machine.id),
                Some(&transition.id),
            ));
        }
        if matches!(
            transition.trigger.kind,
            SimulationTriggerKind::Event
                | SimulationTriggerKind::Signal
                | SimulationTriggerKind::After
                | SimulationTriggerKind::Time
                | SimulationTriggerKind::Change
        ) && transition
            .trigger
            .value
            .as_deref()
            .unwrap_or_default()
            .is_empty()
        {
            findings.push(finding(
                "transition.missing_trigger",
                "Triggered transitions must declare a trigger value.",
                Some(&machine.id),
                Some(&transition.id),
            ));
        }
        let key = (
            transition.source.clone(),
            transition.trigger.kind.clone(),
            transition.trigger.value.clone(),
        );
        *transition_keys.entry(key).or_default() += 1;
    }

    for ((source, _, trigger), count) in transition_keys {
        if count > 1 {
            findings.push(finding(
                "transition.ambiguous_trigger",
                &format!(
                    "Source state `{source}` has {count} transitions for trigger `{}`.",
                    trigger.unwrap_or_else(|| "<none>".to_string())
                ),
                Some(&machine.id),
                Some(&source),
            ));
        }
    }
}

fn finding(
    code: &str,
    message: &str,
    machine_id: Option<&str>,
    element_id: Option<&str>,
) -> SimulationProfileFinding {
    SimulationProfileFinding {
        code: code.to_string(),
        message: message.to_string(),
        machine_id: machine_id.map(str::to_string),
        element_id: element_id.map(str::to_string),
    }
}

fn default_step_duration() -> f64 {
    1.0
}

pub mod tuple_value_map {
    use std::collections::BTreeMap;
    use std::fmt;

    use serde::de::{self, Visitor};
    use serde::ser::SerializeMap;
    use serde::{Deserializer, Serializer};
    use serde_json::Value;

    pub fn serialize<S>(
        values: &BTreeMap<(String, String), Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(values.len()))?;
        for ((subject, feature), value) in values {
            map.serialize_entry(&format!("{subject}|{feature}"), value)?;
        }
        map.end()
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<(String, String), Value>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TupleMapVisitor;

        impl<'de> Visitor<'de> for TupleMapVisitor {
            type Value = BTreeMap<(String, String), Value>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a map keyed by `subject|feature`")
            }

            fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut values = BTreeMap::new();
                while let Some((key, value)) = access.next_entry::<String, Value>()? {
                    let Some((subject, feature)) = key.split_once('|') else {
                        return Err(de::Error::custom(format!(
                            "invalid tuple key `{key}`, expected `subject|feature`"
                        )));
                    };
                    values.insert((subject.to_string(), feature.to_string()), value);
                }
                Ok(values)
            }
        }

        deserializer.deserialize_map(TupleMapVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_rejects_ambiguous_transitions() {
        let model = SimulationModel {
            id: "demo".to_string(),
            machines: vec![SimulationStateMachine {
                id: "Machine".to_string(),
                label: "Machine".to_string(),
                states: vec![state("s1", true), state("s2", false), state("s3", false)],
                transitions: vec![
                    transition("t1", "s1", "s2", "go"),
                    transition("t2", "s1", "s3", "go"),
                ],
            }],
        };

        let error = validate_simulation_model(&model).unwrap_err();
        assert!(
            error
                .findings
                .iter()
                .any(|finding| finding.code == "transition.ambiguous_trigger")
        );
    }

    #[test]
    fn validator_accepts_minimal_machine() {
        let model = SimulationModel {
            id: "demo".to_string(),
            machines: vec![SimulationStateMachine {
                id: "Machine".to_string(),
                label: "Machine".to_string(),
                states: vec![state("s1", true), state("s2", false)],
                transitions: vec![transition("t1", "s1", "s2", "go")],
            }],
        };

        validate_simulation_model(&model).unwrap();
    }

    #[test]
    fn core_runner_rejects_invalid_model_before_execution() {
        let model = SimulationModel {
            id: "demo".to_string(),
            machines: vec![SimulationStateMachine {
                id: "Machine".to_string(),
                label: "Machine".to_string(),
                states: vec![state("s1", true), state("s2", false)],
                transitions: vec![
                    transition("t1", "s1", "s2", "go"),
                    transition("t2", "s1", "s2", "go"),
                ],
            }],
        };

        let error = run_concurrent_simulation_model(
            &model,
            ConcurrentSimulationScenario {
                id: "scenario".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "subject".to_string(),
                    machine_id: "Machine".to_string(),
                    initial_state_id: None,
                    events: vec![SimulationEvent {
                        id: "event.go".to_string(),
                        trigger: "go".to_string(),
                    }],
                }],
                max_steps: 4,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
            SimulationClockConfig::default(),
        )
        .unwrap_err();

        assert!(matches!(error, CoreSimulationError::InvalidProfile(_)));
    }

    #[test]
    fn core_runner_executes_event_signal_flow() {
        let model = SimulationModel {
            id: "demo".to_string(),
            machines: vec![
                SimulationStateMachine {
                    id: "BedMachine".to_string(),
                    label: "BedMachine".to_string(),
                    states: vec![state("bed.heating", true), state("bed.ready", false)],
                    transitions: vec![SimulationTransition {
                        id: "bed.ready".to_string(),
                        source: "bed.heating".to_string(),
                        target: "bed.ready".to_string(),
                        trigger: SimulationTrigger {
                            kind: SimulationTriggerKind::Event,
                            value: Some("finish".to_string()),
                        },
                        guard: None,
                        effects: vec![SimulationEffect::EmitSignal(SignalEffect {
                            signal_type: "BedReady".to_string(),
                            target: Some("printer".to_string()),
                        })],
                    }],
                },
                SimulationStateMachine {
                    id: "PrinterMachine".to_string(),
                    label: "PrinterMachine".to_string(),
                    states: vec![
                        state("printer.heating", true),
                        state("printer.printing", false),
                    ],
                    transitions: vec![SimulationTransition {
                        id: "printer.print".to_string(),
                        source: "printer.heating".to_string(),
                        target: "printer.printing".to_string(),
                        trigger: SimulationTrigger {
                            kind: SimulationTriggerKind::Signal,
                            value: Some("BedReady".to_string()),
                        },
                        guard: None,
                        effects: Vec::new(),
                    }],
                },
            ],
        };

        let trace = run_concurrent_simulation_model(
            &model,
            ConcurrentSimulationScenario {
                id: "scenario".to_string(),
                subjects: vec![
                    ConcurrentSubjectScenario {
                        subject_id: "bed".to_string(),
                        machine_id: "BedMachine".to_string(),
                        initial_state_id: None,
                        events: vec![SimulationEvent {
                            id: "event.finish".to_string(),
                            trigger: "finish".to_string(),
                        }],
                    },
                    ConcurrentSubjectScenario {
                        subject_id: "printer".to_string(),
                        machine_id: "PrinterMachine".to_string(),
                        initial_state_id: None,
                        events: Vec::new(),
                    },
                ],
                max_steps: 6,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
            },
            SimulationClockConfig::default(),
        )
        .unwrap();

        assert!(trace.timeline.iter().any(|entry| {
            entry
                .states
                .get("printer")
                .is_some_and(|states| states == &vec!["printer.printing".to_string()])
        }));
    }

    fn state(id: &str, initial: bool) -> SimulationState {
        SimulationState {
            id: id.to_string(),
            label: id.to_string(),
            parent_state_id: None,
            is_initial: initial,
            is_final: false,
            is_orthogonal: false,
            is_history: false,
            entry_behavior: None,
            exit_behavior: None,
            do_behavior: None,
        }
    }

    fn transition(id: &str, source: &str, target: &str, trigger: &str) -> SimulationTransition {
        SimulationTransition {
            id: id.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            trigger: SimulationTrigger {
                kind: SimulationTriggerKind::Event,
                value: Some(trigger.to_string()),
            },
            guard: None,
            effects: Vec::new(),
        }
    }
}
