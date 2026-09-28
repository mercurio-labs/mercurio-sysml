//! Owned analysis execution and bounded, reproducible experiments for all hosts.
use super::*;
use mercurio_foundation::simulation_core::{
    CoreSimulationError, SimulationSession, SimulationSessionLifecycle,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationInputOverride {
    pub subject_id: String,
    pub feature: String,
    pub value: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSimulationSnapshot {
    pub lifecycle: SimulationSessionLifecycle,
    pub logical_time_s: f64,
    pub execution_steps: usize,
    pub advance_cycles: usize,
    pub trace: Value,
    pub inputs: Vec<SimulationInputOverride>,
    pub error: Option<String>,
}

pub struct AnalysisSimulationSession {
    core: SimulationSession,
    activity: Option<timed_activity::PreparedTimedActivity>,
    inputs: Vec<SimulationInputOverride>,
    analysis_case_id: String,
}
fn invalid(message: impl Into<String>) -> SimulationError {
    SimulationError::InvalidProfile(message.into())
}

// Source Real defaults may serialize as integral JSON numbers, so use the
// authored scalar domain rather than guessing Integer from JSON representation.
fn parameter_is_integer(runtime: &Runtime, subject_id: &str, feature: &str) -> bool {
    fn reference<'a>(element: &'a Element, keys: &[&str]) -> Option<&'a str> {
        keys.iter()
            .find_map(|key| match element.properties.get(*key) {
                Some(Value::String(value)) => Some(value.as_str()),
                Some(Value::Array(values)) if values.len() == 1 => values[0].as_str(),
                _ => None,
            })
    }
    let Some(subject) = runtime.graph().element_by_element_id(subject_id) else {
        return false;
    };
    let Some(definition) = reference(subject, &["type", "definition"]) else {
        return false;
    };
    runtime.graph().elements().iter().any(|attribute| {
        attribute.kind.contains("AttributeUsage")
            && reference(attribute, &["owner", "owning_type"]) == Some(definition)
            && reference(attribute, &["declared_name", "name"]) == Some(feature)
            && reference(attribute, &["type"]) == Some("ScalarValues::Integer")
    })
}

impl AnalysisSimulationSession {
    pub fn start(
        runtime: &Runtime,
        analysis_case_id: &str,
        overrides: &[SimulationInputOverride],
    ) -> Result<Self, SimulationError> {
        let spec =
            project_analysis_spec(runtime, analysis_case_id).map_err(map_analysis_spec_error)?;
        let activity = if timed_activity::is_requested(runtime, &spec) {
            Some(timed_activity::prepare(runtime, &spec)?)
        } else {
            None
        };
        let (model, mut scenario, clock) = if let Some(activity) = &activity {
            (
                activity.model.clone(),
                activity.scenario.clone(),
                activity.clock.clone(),
            )
        } else {
            let scenario = scenario_from_analysis_case(runtime, analysis_case_id)?;
            let model = canonical_simulation_model(runtime)?;
            if runtime_has_legacy_rate_transition_effects(runtime, &model, &scenario)
                || !core_runner_can_handle(&model, &scenario)
            {
                return Err(invalid(
                    "analysis.session.unsupported: the scenario cannot execute in the shared core",
                ));
            }
            let clock = scenario
                .clock_config
                .clone()
                .unwrap_or_else(|| SimulationClockConfig {
                    max_time_s: scenario.max_steps.max(1) as f64
                        * scenario.step_duration_s.max(0.0),
                    fixed_step_s: scenario.step_duration_s,
                    sample_interval_s: scenario.step_duration_s,
                    adaptive: None,
                    change_loop_limit: CHANGE_LOOP_LIMIT,
                });
            (model, scenario, clock)
        };
        let solved = model.constraint_networks.iter().flat_map(|network| network.unknowns.iter()
            .filter_map(|id| network.variables.get(id).map(|feature| (network.subject_id.clone(), feature.clone())))).collect::<BTreeSet<_>>();
        let mut seen = BTreeSet::new();
        for input in overrides {
            let key = (input.subject_id.clone(), input.feature.clone());
            if solved.contains(&key) || activity.as_ref().is_some_and(|activity| {
                activity.is_internal_feature(&input.subject_id, &input.feature)
            }) {
                return Err(invalid(
                    "experiment.input.internal: generated execution state is not an input parameter",
                ));
            }
            if !seen.insert(key.clone()) {
                return Err(invalid(
                    "experiment.input.duplicate: a parameter was overridden twice",
                ));
            }
            let original = scenario.initial_values.get(&key).ok_or_else(|| {
                invalid(format!(
                    "experiment.input.unknown: {}.{} is not an initialized parameter",
                    input.subject_id, input.feature
                ))
            })?;
            let valid = (original.is_number() && input.value.as_f64().is_some_and(f64::is_finite))
                || (original.is_boolean() && input.value.is_boolean())
                || (original.is_string() && input.value.is_string());
            if !valid
                || (parameter_is_integer(runtime, &input.subject_id, &input.feature)
                    && !(input.value.is_i64() || input.value.is_u64()))
            {
                return Err(invalid(
                    "experiment.input.type: overrides must preserve the initialized scalar domain",
                ));
            }
            scenario.initial_values.insert(key, input.value.clone());
        }
        let inputs = scenario
            .initial_values
            .iter()
            .filter(|((subject, feature), _)| {
                !solved.contains(&(subject.clone(), feature.clone())) && !activity
                    .as_ref()
                    .is_some_and(|activity| activity.is_internal_feature(subject, feature))
            })
            .map(|((subject_id, feature), value)| SimulationInputOverride {
                subject_id: subject_id.clone(),
                feature: feature.clone(),
                value: value.clone(),
            })
            .collect();
        let core = SimulationSession::initialize(model, scenario, clock)
            .map_err(|e| invalid(e.to_string()))?;
        Ok(Self {
            core,
            activity,
            inputs,
            analysis_case_id: spec.case_ref.element_id,
        })
    }
    pub fn snapshot(&self) -> Result<AnalysisSimulationSnapshot, SimulationError> {
        let mut snapshot = self.core.snapshot();
        if let Some(activity) = &self.activity {
            activity.decorate_trace(&mut snapshot.trace);
        }
        let report =
            simulation_trace_report("analysis.session", &self.analysis_case_id, snapshot.trace)?;
        let mut trace = report
            .artifacts
            .into_iter()
            .find(|artifact| artifact.kind == SIMULATION_TRACE_ARTIFACT_KIND)
            .ok_or_else(|| invalid("analysis.session: trace artifact missing"))?
            .payload;
        // Session lifecycle is explicit independently of the legacy trace health field.
        trace["session_lifecycle"] = serde_json::to_value(snapshot.lifecycle)?;
        Ok(AnalysisSimulationSnapshot {
            lifecycle: snapshot.lifecycle,
            logical_time_s: snapshot.logical_time_s,
            execution_steps: snapshot.execution_steps,
            advance_cycles: snapshot.advance_cycles,
            trace,
            inputs: self.inputs.clone(),
            error: snapshot.error,
        })
    }
    pub fn advance(
        &mut self,
        cycles: usize,
    ) -> Result<AnalysisSimulationSnapshot, SimulationError> {
        if cycles == 0 || cycles > 1000 {
            return Err(invalid(
                "analysis.session: advance requires 1 to 1000 scheduling cycles",
            ));
        }
        // Failed cycles preserve a last committed trace and a terminal error in the snapshot.
        let error = self.core.advance(cycles).err();
        self.snapshot_after_execution(error)
    }
    pub fn inject_event(
        &mut self,
        subject_id: &str,
        event: SimulationEvent,
    ) -> Result<AnalysisSimulationSnapshot, SimulationError> {
        self.core
            .inject_event(subject_id, event)
            .map_err(|e| invalid(e.to_string()))?;
        self.snapshot()
    }
    pub fn cancel(&mut self) -> Result<AnalysisSimulationSnapshot, SimulationError> {
        self.core.cancel();
        self.snapshot()
    }
    pub fn run_to_completion(&mut self) -> Result<AnalysisSimulationSnapshot, SimulationError> {
        let error = self.core.run_to_completion().err();
        self.snapshot_after_execution(error)
    }
    fn snapshot_after_execution(
        &self,
        error: Option<CoreSimulationError>,
    ) -> Result<AnalysisSimulationSnapshot, SimulationError> {
        let snapshot = self.snapshot()?;
        if let Some(error) = error {
            // Execution failures are data when the core retained terminal evidence;
            // never silently swallow an error that failed to produce that evidence.
            if snapshot.lifecycle != SimulationSessionLifecycle::Failed || snapshot.error.is_none()
            {
                return Err(invalid(error.to_string()));
            }
        }
        Ok(snapshot)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationExperimentAxis {
    pub subject_id: String,
    pub feature: String,
    pub values: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisExperimentRequest {
    pub axes: Vec<SimulationExperimentAxis>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisExperimentVariant {
    pub index: usize,
    pub overrides: Vec<SimulationInputOverride>,
    pub snapshot: Option<AnalysisSimulationSnapshot>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisExperimentResult {
    pub schema: String,
    pub analysis_case_id: String,
    pub axes: Vec<SimulationExperimentAxis>,
    pub variants: Vec<AnalysisExperimentVariant>,
}

/// Cartesian scenarios in caller-provided axis/value order; no random inputs or shared mutable run state.
/// Limit 32 variants and 4 axes to bound a synchronous host call. Each result includes its exact inputs/trace.
pub fn run_analysis_experiment(
    runtime: &Runtime,
    analysis_case_id: &str,
    request: AnalysisExperimentRequest,
) -> Result<AnalysisExperimentResult, SimulationError> {
    if request.axes.is_empty() || request.axes.len() > 4 {
        return Err(invalid("experiment.axes: provide 1 to 4 parameter axes"));
    }
    let mut combinations = vec![Vec::<SimulationInputOverride>::new()];
    let mut keys = BTreeSet::new();
    for axis in &request.axes {
        if !keys.insert((axis.subject_id.clone(), axis.feature.clone())) {
            return Err(invalid("experiment.axes: duplicate parameter"));
        }
        if axis.values.is_empty()
            || axis.values.len() > 32
            || combinations.len() * axis.values.len() > 32
        {
            return Err(invalid(
                "experiment.budget: provide nonempty axes totaling at most 32 variants",
            ));
        }
        combinations = combinations
            .into_iter()
            .flat_map(|prefix| {
                axis.values.iter().map(move |value| {
                    let mut next = prefix.clone();
                    next.push(SimulationInputOverride {
                        subject_id: axis.subject_id.clone(),
                        feature: axis.feature.clone(),
                        value: value.clone(),
                    });
                    next
                })
            })
            .collect();
    }
    let mut variants = Vec::new();
    for (index, overrides) in combinations.into_iter().enumerate() {
        let result = AnalysisSimulationSession::start(runtime, analysis_case_id, &overrides)
            .and_then(|mut session| session.run_to_completion());
        let (snapshot, error) = match result {
            Ok(snapshot) => {
                let error = snapshot.error.clone();
                (Some(snapshot), error)
            }
            Err(error) => (None, Some(error.to_string())),
        };
        variants.push(AnalysisExperimentVariant {
            index,
            overrides,
            snapshot,
            error,
        });
    }
    Ok(AnalysisExperimentResult {
        schema: "mercurio.simulation.experiment.v1".into(),
        analysis_case_id: analysis_case_id.into(),
        axes: request.axes,
        variants,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile_sysml_text, load_sysml_baseline};
    fn runtime(source: &str) -> Runtime {
        Runtime::from_document(
            compile_sysml_text(source, "session.sysml", &load_sysml_baseline().unwrap()).unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn source_session_matches_whole_run_and_preserves_sampled_outcomes() {
        let runtime = runtime(include_str!("instrument-cooldown.sysml"));
        let case = list_analysis_cases(&runtime)
            .into_iter()
            .find(|case| case.label == "CooldownProfile")
            .unwrap();
        let expected = run_analysis_case(&runtime, &case.id, "batch")
            .unwrap()
            .artifacts[0]
            .payload
            .clone();
        let mut session = AnalysisSimulationSession::start(&runtime, &case.id, &[]).unwrap();
        let mut current = session.snapshot().unwrap();
        assert_eq!(current.lifecycle, SimulationSessionLifecycle::Paused);
        assert_eq!(current.logical_time_s, 0.0);
        while current.lifecycle == SimulationSessionLifecycle::Paused {
            current = session.advance(1).unwrap();
        }
        for key in [
            "timeline",
            "configuration",
            "status",
            "termination",
            "requirement_outcomes",
        ] {
            assert_eq!(current.trace[key], expected[key], "{key}");
        }
    }
    #[test]
    fn experiment_isolates_inputs_is_repeatable_and_reports_missed_deadlines() {
        let runtime = runtime(include_str!("instrument-cooldown.sysml"));
        let case = list_analysis_cases(&runtime)
            .into_iter()
            .find(|case| case.label == "CooldownProfile")
            .unwrap();
        let request = AnalysisExperimentRequest {
            axes: vec![SimulationExperimentAxis {
                subject_id: "subject.InstrumentCooldown.CooldownProfile.instrument".into(),
                feature: "finalCoolingRate".into(),
                values: vec![serde_json::json!(-1.0), serde_json::json!(-3.0)],
            }],
        };
        let result = run_analysis_experiment(&runtime, &case.id, request.clone()).unwrap();
        assert_eq!(result.variants.len(), 2);
        assert_eq!(
            result.variants[0].snapshot.as_ref().unwrap().trace["requirement_outcomes"][0]["status"],
            "violated"
        );
        assert_eq!(
            result.variants[1].snapshot.as_ref().unwrap().trace["requirement_outcomes"][0]["status"],
            "satisfied"
        );
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::to_value(run_analysis_experiment(&runtime, &case.id, request).unwrap())
                .unwrap()
        );
        let original = AnalysisSimulationSession::start(&runtime, &case.id, &[])
            .unwrap()
            .run_to_completion()
            .unwrap();
        assert_eq!(
            original.trace["requirement_outcomes"][0]["status"],
            "violated"
        );
    }
    #[test]
    fn rejects_unknown_overrides_and_unbounded_experiments() {
        let runtime = runtime(include_str!("instrument-cooldown.sysml"));
        let case = list_analysis_cases(&runtime)
            .into_iter()
            .find(|case| case.label == "CooldownProfile")
            .unwrap();
        assert!(
            AnalysisSimulationSession::start(
                &runtime,
                &case.id,
                &[SimulationInputOverride {
                    subject_id: "invalid".into(),
                    feature: "temperature".into(),
                    value: serde_json::json!(1)
                }]
            )
            .is_err()
        );
        assert!(
            run_analysis_experiment(
                &runtime,
                &case.id,
                AnalysisExperimentRequest { axes: vec![] }
            )
            .is_err()
        );
    }
    #[test]
    fn activity_sessions_hide_and_reject_generated_parameters() {
        let runtime = runtime(include_str!(
            "../../tests/fixtures/timed-activity-fork-join.sysml"
        ));
        let case = list_analysis_cases(&runtime)
            .into_iter()
            .find(|case| case.label == "ActivityProfile")
            .unwrap();
        let mut session = AnalysisSimulationSession::start(&runtime, &case.id, &[]).unwrap();
        let initial = session.snapshot().unwrap();
        assert_eq!(initial.inputs.len(), 1);
        assert_eq!(initial.inputs[0].feature, "completed");
        assert!(
            !serde_json::to_string(&initial)
                .unwrap()
                .contains("__timed_activity_done_")
        );
        let error = AnalysisSimulationSession::start(
            &runtime,
            &case.id,
            &[SimulationInputOverride {
                subject_id: initial.inputs[0].subject_id.clone(),
                feature: "__timed_activity_done_0".into(),
                value: Value::Bool(true),
            }],
        )
        .err()
        .unwrap();
        assert!(error.to_string().contains("experiment.input.internal"));
        let expected = run_analysis_case(&runtime, &case.id, "batch")
            .unwrap()
            .artifacts[0]
            .payload
            .clone();
        let mut current = initial;
        while current.lifecycle == SimulationSessionLifecycle::Paused {
            current = session.advance(1).unwrap();
        }
        for key in [
            "timeline",
            "channels",
            "requirement_outcomes",
            "termination",
        ] {
            assert_eq!(current.trace[key], expected[key], "{key}");
        }
    }

    #[test]
    fn session_wrapper_preserves_failure_evidence_and_cancelled_boundary() {
        use mercurio_foundation::simulation_core::SimulationEffect;
        let runtime = runtime(include_str!("instrument-cooldown.sysml"));
        let case = list_analysis_cases(&runtime)
            .into_iter()
            .find(|case| case.label == "CooldownProfile")
            .unwrap();
        let failing_session = || {
            let mut session = AnalysisSimulationSession::start(&runtime, &case.id, &[]).unwrap();
            let scenario = scenario_from_analysis_case(&runtime, &case.id).unwrap();
            let clock = scenario.clock_config.clone().unwrap();
            let mut model = canonical_simulation_model(&runtime).unwrap();
            let transition = model
                .machines
                .iter_mut()
                .flat_map(|m| &mut m.transitions)
                .find(|t| t.trigger.kind == SimulationTriggerKind::Event)
                .unwrap();
            transition.effects.push(SimulationEffect::AssignExpression { feature: "temperature".into(),
                expression: serde_json::json!({"kind":"binary", "op":"/", "left":{"kind":"literal","value":1}, "right":{"kind":"literal","value":0}}) });
            session.core = SimulationSession::initialize(model, scenario, clock).unwrap();
            session
        };
        for drain in [false, true] {
            let mut session = failing_session();
            let initial = session.snapshot().unwrap();
            let failed = if drain {
                session.run_to_completion()
            } else {
                session.advance(1)
            }
            .unwrap();
            assert_eq!(failed.lifecycle, SimulationSessionLifecycle::Failed);
            assert!(failed.error.is_some());
            assert_eq!(failed.trace["status"], "failed");
            assert_eq!(failed.trace["timeline"], initial.trace["timeline"]);
            assert_eq!(failed.execution_steps, 0);
        }
        let mut session = AnalysisSimulationSession::start(&runtime, &case.id, &[]).unwrap();
        assert!(session.advance(0).is_err());
        assert!(session.advance(1001).is_err());
        let initial = session.snapshot().unwrap();
        let cancelled = session.cancel().unwrap();
        assert_eq!(cancelled.lifecycle, SimulationSessionLifecycle::Cancelled);
        assert_eq!(cancelled.trace["termination"], "cancelled");
        assert_eq!(cancelled.trace["timeline"], initial.trace["timeline"]);
        assert_eq!(
            serde_json::to_value(&cancelled).unwrap(),
            serde_json::to_value(session.advance(3).unwrap()).unwrap()
        );
    }

    #[test]
    fn experiment_orders_cartesian_inputs_and_preserves_valid_variants_after_errors() {
        let runtime = runtime(include_str!("instrument-cooldown.sysml"));
        let case = list_analysis_cases(&runtime)
            .into_iter()
            .find(|case| case.label == "CooldownProfile")
            .unwrap();
        let subject = "subject.InstrumentCooldown.CooldownProfile.instrument";
        let request = AnalysisExperimentRequest {
            axes: vec![
                SimulationExperimentAxis {
                    subject_id: subject.into(),
                    feature: "finalCoolingRate".into(),
                    values: vec![Value::from(-1.0), Value::from("invalid"), Value::from(-3.0)],
                },
                SimulationExperimentAxis {
                    subject_id: subject.into(),
                    feature: "handoffTemperature".into(),
                    values: vec![Value::from(25.0), Value::from(30.0)],
                },
            ],
        };
        let result = run_analysis_experiment(&runtime, &case.id, request).unwrap();
        assert_eq!(result.variants.len(), 6);
        for (index, variant) in result.variants.iter().enumerate() {
            assert_eq!(variant.index, index);
            assert_eq!(
                variant.overrides[1].value,
                Value::from(if index % 2 == 0 { 25.0 } else { 30.0 })
            );
            assert_eq!(variant.error.is_some(), matches!(index, 2 | 3));
            assert_eq!(variant.snapshot.is_some(), !matches!(index, 2 | 3));
        }
        assert_eq!(
            result.variants[4].snapshot.as_ref().unwrap().trace["requirement_outcomes"][0]["status"],
            "satisfied"
        );
    }

    #[test]
    fn input_overrides_preserve_authored_integer_and_scalar_domains() {
        let source = include_str!("instrument-cooldown.sysml").replace(
            "attribute temperature : Real = 80.0;",
            "attribute temperature : Real = 80.0; attribute count : Integer = 2;",
        );
        let runtime = runtime(&source);
        let case = list_analysis_cases(&runtime)
            .into_iter()
            .find(|case| case.label == "CooldownProfile")
            .unwrap();
        let subject = "subject.InstrumentCooldown.CooldownProfile.instrument";
        let input = |value| SimulationInputOverride {
            subject_id: subject.into(),
            feature: "count".into(),
            value,
        };
        for bad in [
            Value::from(2.5),
            Value::Bool(true),
            Value::Null,
            serde_json::json!([1, 2]),
        ] {
            assert!(AnalysisSimulationSession::start(&runtime, &case.id, &[input(bad)]).is_err());
        }
        assert!(
            AnalysisSimulationSession::start(&runtime, &case.id, &[input(Value::from(3))]).is_ok()
        );
        assert!(
            AnalysisSimulationSession::start(
                &runtime,
                &case.id,
                &[input(Value::from(3)), input(Value::from(4))]
            )
            .is_err()
        );
    }
}
