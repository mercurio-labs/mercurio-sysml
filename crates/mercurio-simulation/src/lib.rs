use std::fmt;

use serde_json::Value;

use mercurio_core::runtime::{Runtime, RuntimeError};
pub use mercurio_simulation_core::{
    AnalysisCaseInfo, ConcurrentSimulationScenario, ConcurrentSubjectScenario,
    CriticalSimulationEvent, HybridSimulationReport, HybridSimulationScenario,
    HybridSimulationStatus, HybridSimulationTraceEntry, SimulationClockConfig, SimulationEvent,
    SimulationModel, SimulationSubject, SimulationTrace, SimulationTriggerKind, TraceChannel,
    TraceChannelSource, TraceEntry, TraceEvent, run_concurrent_simulation_model,
};
use mercurio_sysml::project_state_machines;

mod legacy_overlay;

pub use legacy_overlay::{
    SimulationOverlayComposition, compose_simulation_overlay, run_hybrid_simulation_with_overlay,
};

const CHANGE_LOOP_LIMIT: usize = 20;
#[allow(dead_code)]
const CROSSING_TOLERANCE_S: f64 = 0.01;

pub type StateMachineScenarioEvent = SimulationEvent;

#[cfg(test)]
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

    let machine_id = project_state_machines(runtime)
        .iter()
        .find(|machine| machine.id == scenario.machine_id || machine.label == scenario.machine_id)
        .map(|machine| machine.id.clone())
        .ok_or_else(|| SimulationError::MissingStateMachine(scenario.machine_id.clone()))?;

    let concurrent = ConcurrentSimulationScenario {
        id: scenario.id.clone(),
        subjects: vec![ConcurrentSubjectScenario {
            subject_id: scenario.subject.id.clone(),
            machine_id: scenario.machine_id.clone(),
            initial_state_id: scenario.initial_state_id.clone(),
            events: scenario.events.clone(),
        }],
        max_steps: scenario.max_steps,
        step_duration_s: scenario.step_duration_s,
        initial_values: scenario.values.clone(),
        requirements: Vec::new(),
        objectives: Vec::new(),
    };
    let trace = run_concurrent_simulation(runtime, concurrent)?;
    Ok(hybrid_report_from_trace(scenario, machine_id, trace))
}

fn hybrid_report_from_trace(
    scenario: HybridSimulationScenario,
    machine_id: String,
    simulation_trace: SimulationTrace,
) -> HybridSimulationReport {
    let subject_id = scenario.subject.id.clone();
    let mut critical_events = vec![CriticalSimulationEvent {
        step: 0,
        kind: "simulation.started".to_string(),
        subject_id: subject_id.clone(),
        detail: [("machine".to_string(), Value::String(machine_id.clone()))]
            .into_iter()
            .collect(),
    }];
    let mut trace = Vec::new();
    for (index, window) in simulation_trace.timeline.windows(2).enumerate() {
        let before_entry = &window[0];
        let after_entry = &window[1];
        let transition = after_entry
            .events
            .iter()
            .find(|event| event.kind == "transition");
        let before = before_entry
            .states
            .get(&subject_id)
            .cloned()
            .unwrap_or_default();
        let after = after_entry
            .states
            .get(&subject_id)
            .cloned()
            .unwrap_or_default();
        let step = index + 1;
        let step_critical = transition
            .map(|event| {
                vec![CriticalSimulationEvent {
                    step,
                    kind: "transition.fired".to_string(),
                    subject_id: subject_id.clone(),
                    detail: [
                        (
                            "transition".to_string(),
                            event
                                .transition_id
                                .clone()
                                .map(Value::String)
                                .unwrap_or(Value::Null),
                        ),
                        (
                            "trigger".to_string(),
                            event
                                .trigger
                                .clone()
                                .map(Value::String)
                                .unwrap_or(Value::Null),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                }]
            })
            .unwrap_or_default();
        critical_events.extend(step_critical.clone());
        trace.push(HybridSimulationTraceEntry {
            step,
            t: after_entry.t,
            event_id: None,
            trigger: transition.and_then(|event| event.trigger.clone()),
            transition_id: transition.and_then(|event| event.transition_id.clone()),
            before,
            after,
            values: after_entry.values.clone(),
            critical_events: step_critical,
            explanation: transition
                .and_then(|event| event.transition_id.as_deref())
                .map(|transition_id| {
                    format!("Transition `{transition_id}` fired for subject `{subject_id}`.")
                })
                .unwrap_or_else(|| "Simulation state advanced.".to_string()),
        });
    }
    let active_configuration = simulation_trace
        .timeline
        .last()
        .and_then(|entry| entry.states.get(&subject_id))
        .cloned()
        .unwrap_or_default();
    let values = simulation_trace
        .timeline
        .last()
        .map(|entry| entry.values.clone())
        .unwrap_or_else(|| scenario.values.clone());
    let rate_channels = simulation_trace
        .channels
        .iter()
        .filter(|channel| channel.source == TraceChannelSource::RateEffect)
        .filter_map(|channel| {
            let (subject, feature) = channel.id.split_once('.')?;
            Some((subject.to_string(), feature.to_string()))
        })
        .collect();
    HybridSimulationReport {
        scenario_id: scenario.id,
        subject_id,
        machine_id,
        status: simulation_trace.status,
        active_configuration,
        values,
        critical_events,
        trace,
        rate_channels,
    }
}

pub fn run_concurrent_simulation(
    runtime: &Runtime,
    scenario: ConcurrentSimulationScenario,
) -> Result<SimulationTrace, SimulationError> {
    run_canonical_core(runtime, &scenario)
}
fn run_canonical_core(
    runtime: &Runtime,
    scenario: &ConcurrentSimulationScenario,
) -> Result<SimulationTrace, SimulationError> {
    let model = canonical_simulation_model(runtime)?;
    if runtime_has_legacy_rate_transition_effects(runtime, &model, scenario) {
        return Err(SimulationError::InvalidProfile(
            "legacy transition `rate` effects are no longer supported by concurrent simulation; move rates to state `do_behavior`".to_string(),
        ));
    }
    if !core_runner_can_handle(&model, scenario) {
        return Err(SimulationError::InvalidProfile(
            "scenario contains simulation profile features unsupported by the canonical core runner"
                .to_string(),
        ));
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
    .map_err(|error| SimulationError::InvalidProfile(error.to_string()))
}

fn runtime_has_legacy_rate_transition_effects(
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
                        .and_then(Value::as_array)
                        .is_some_and(|effects| {
                            effects.iter().any(|effect| {
                                effect
                                    .get("kind")
                                    .and_then(Value::as_str)
                                    .is_some_and(|kind| kind == "rate")
                            })
                        })
                })
            })
    })
}

fn core_runner_can_handle(
    model: &SimulationModel,
    scenario: &ConcurrentSimulationScenario,
) -> bool {
    scenario.subjects.iter().all(|subject| {
        model
            .machines
            .iter()
            .find(|machine| machine.id == subject.machine_id || machine.label == subject.machine_id)
            .is_some_and(|machine| {
                machine.transitions.iter().all(|transition| {
                    matches!(
                        transition.trigger.kind,
                        SimulationTriggerKind::Event
                            | SimulationTriggerKind::Signal
                            | SimulationTriggerKind::Time
                            | SimulationTriggerKind::After
                            | SimulationTriggerKind::Change
                            | SimulationTriggerKind::Completion
                    )
                })
            })
    })
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
            vec!["simulation.started", "transition.fired", "transition.fired",]
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
            vec!["simulation.started", "transition.fired", "transition.fired",]
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

        assert_eq!(report.trace[0].t, 0.0);
        assert_eq!(report.trace[1].t, 0.0);
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
    fn hybrid_simulation_rejects_legacy_transition_rate_effects() {
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
        let error = run_hybrid_simulation(
            &runtime,
            scenario(
                Vec::new(),
                BTreeMap::from([(key.clone(), json!(20.0))]),
                1.0,
                10,
            ),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            SimulationError::InvalidProfile(message)
                if message.contains("legacy transition `rate` effects")
        ));
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
                    [
                        ("declared_name", json!("test")),
                        ("type", json!("type.Test")),
                    ],
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
                    [],
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
                    json!(30.0),
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
                element(
                    "state.Bed.Hot",
                    "StateUsage",
                    [
                        ("declared_name", json!("Hot")),
                        ("owning_type", json!("BedMachine")),
                        ("is_initial", json!(false)),
                        (
                            "do_behavior",
                            json!({
                                "kind": "rate_integration",
                                "rates": [
                                    {
                                        "feature": "bed_ready",
                                        "rate_per_second": 0.5
                                    }
                                ]
                            }),
                        ),
                    ],
                ),
                transition_element(
                    "transition.Printer.print",
                    "PrinterMachine",
                    "state.Printer.Waiting",
                    "state.Printer.Printing",
                    "individual.bed.bed_ready >= 1.0",
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
                    [],
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
    fn concurrent_simulation_rejects_legacy_transition_rate_effects() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                state_element("state.Bed.Cold", "BedMachine", true),
                state_element("state.Bed.Hot", "BedMachine", false),
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

        let error = run_concurrent_simulation(
            &runtime,
            ConcurrentSimulationScenario {
                id: "scenario.legacy_rate".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "individual.bed".to_string(),
                    machine_id: "BedMachine".to_string(),
                    initial_state_id: None,
                    events: Vec::new(),
                }],
                max_steps: 4,
                step_duration_s: 1.0,
                initial_values: BTreeMap::new(),
                requirements: Vec::new(),
                objectives: Vec::new(),
            },
        )
        .unwrap_err();

        assert!(matches!(
            error,
            SimulationError::InvalidProfile(message)
                if message.contains("legacy transition `rate` effects")
        ));
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
                    requirements: Vec::new(),
                    objectives: Vec::new(),
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
                requirements: Vec::new(),
                objectives: Vec::new(),
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
