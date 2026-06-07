use std::collections::{BTreeMap, BTreeSet};
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
