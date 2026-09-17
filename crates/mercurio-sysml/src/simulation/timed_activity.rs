//! Bounded, opt-in sequential activities lowered to the shared simulation scheduler.
use super::adapter::mission_metadata::{number, properties};
use super::*;
use mercurio_foundation::simulation_core::{
    AssignEffect, SimulationEffect, SimulationState, SimulationStateMachine, SimulationTransition,
    SimulationTrigger,
};

fn invalid(message: impl Into<String>) -> SimulationError {
    SimulationError::InvalidProfile(format!("activity.timed.unsupported: {}", message.into()))
}
fn construct(element: &Element) -> &str {
    element
        .properties
        .get("metadata")
        .and_then(|m| m.get("lowering"))
        .and_then(|m| m.get("construct"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| element.kind.rsplit("::").next().unwrap_or(&element.kind))
}
fn annotated(element: &Element, name: &str) -> bool {
    element
        .properties
        .get("metadata")
        .and_then(|m| m.get(format!("Mercurio::Missions::{name}")))
        .is_some()
}
pub(super) fn is_requested(runtime: &Runtime, spec: &AnalysisSpec) -> bool {
    spec.dynamic_behavior_bindings.iter().any(|binding| {
        runtime
            .graph()
            .element_by_element_id(&binding.behavior.element_id)
            .is_some_and(|e| {
                annotated(e, "TimedActivity")
                    || runtime.graph().elements().iter().any(|child| {
                        activity_owner_id(child).as_deref() == Some(e.element_id.as_str())
                            && annotated(child, "Duration")
                    })
            })
    })
}
fn references(value: Option<&Value>) -> Vec<&str> {
    match value {
        Some(Value::String(s)) => vec![s],
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
        _ => vec![],
    }
}
fn endpoint(
    graph: &Graph,
    flow: &Element,
    name: &str,
    actions: &[&Element],
) -> Result<String, SimulationError> {
    let refs = graph
        .elements()
        .iter()
        .filter(|e| {
            activity_owner_id(e).as_deref() == Some(flow.element_id.as_str())
                && element_label(e) == name
        })
        .collect::<Vec<_>>();
    if refs.len() != 1 {
        return Err(invalid(format!("{} requires one {name}", flow.element_id)));
    }
    let targets = references(refs[0].properties.get("specialized_features"));
    let matches = actions
        .iter()
        .filter(|a| {
            targets.contains(&a.element_id.as_str())
                || targets.contains(&a.element_id.replacen("action.", "feature.", 1).as_str())
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(invalid(format!(
            "{} has an unresolved or ambiguous {name}",
            flow.element_id
        )));
    }
    Ok(matches[0].element_id.clone())
}
fn state(id: String, label: String, initial: bool) -> SimulationState {
    SimulationState {
        id,
        label,
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
fn action_event(kind: &str, subject: &str, action: &Element) -> SimTraceEvent {
    SimTraceEvent {
        kind: kind.into(),
        subject_id: Some(subject.into()),
        transition_id: None,
        trigger: Some(element_label(action)),
        reason: Some(action.element_id.clone()),
    }
}

pub(super) fn run(
    runtime: &Runtime,
    spec: &AnalysisSpec,
    run_id: &str,
) -> Result<CapabilityRunReport, SimulationError> {
    if spec.subjects.len() != 1
        || spec.dynamic_behavior_bindings.len() != 1
        || spec.dynamic_behavior_bindings[0].kind != AnalysisDynamicBehaviorKind::Activity
    {
        return Err(invalid(
            "exactly one activity binding is required; mixed or concurrent behavior is unsupported",
        ));
    }
    if !spec.objectives.is_empty()
        || !spec.assumptions.is_empty()
        || !spec.calculations.is_empty()
        || !spec.constraints.is_empty()
    {
        return Err(invalid(
            "objectives, assumptions, calculations and additional constraint techniques are unsupported in timed sequences",
        ));
    }
    let graph = runtime.graph();
    let binding = &spec.dynamic_behavior_bindings[0];
    let behavior = graph
        .element_by_element_id(&binding.behavior.element_id)
        .ok_or_else(|| invalid("missing activity"))?;
    if construct(behavior) != "ActionUsage"
        || references(behavior.properties.get("type"))
            .iter()
            .any(|t| *t != "Actions::Action")
    {
        return Err(invalid(
            "activity must be an inline action usage without inherited behavior",
        ));
    }
    let annotation = properties(runtime, behavior, "TimedActivity", &["completionFeature"])
        .map_err(map_adapter_error)?
        .ok_or_else(|| {
            invalid("duration annotations require TimedActivity on the containing activity")
        })?;
    let completion = annotation
        .get("completionFeature")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("completionFeature must name a Boolean attribute"))?;
    let owner = activity_owner_id(behavior)
        .ok_or_else(|| invalid("activity must belong to a subject definition"))?;
    let attributes = graph
        .elements()
        .iter()
        .filter(|e| {
            activity_owner_id(e).as_deref() == Some(owner.as_str())
                && construct(e) == "AttributeUsage"
                && element_label(e) == completion
        })
        .collect::<Vec<_>>();
    if attributes.len() != 1
        || !references(attributes[0].properties.get("type")).contains(&"ScalarValues::Boolean")
        || attributes[0]
            .properties
            .get("expression_ir")
            .and_then(|e| e.get("value"))
            != Some(&Value::Bool(false))
    {
        return Err(invalid(
            "completionFeature must resolve uniquely to an authored Boolean attribute initialized to false",
        ));
    }
    for element in [
        behavior,
        graph
            .element_by_element_id(&binding.subject.element_id)
            .ok_or_else(|| invalid("missing subject"))?,
    ] {
        if element.properties.contains_key("multiplicity") {
            return Err(invalid(
                "explicit behavior/subject multiplicity is unsupported",
            ));
        }
    }
    let children = graph
        .elements()
        .iter()
        .filter(|e| activity_owner_id(e).as_deref() == Some(behavior.element_id.as_str()))
        .collect::<Vec<_>>();
    let actions = children
        .iter()
        .copied()
        .filter(|e| construct(e) == "ActionUsage")
        .collect::<Vec<_>>();
    if actions.is_empty() || actions.len() > 1000 {
        return Err(invalid("a sequence needs 1 to 1000 leaf actions"));
    }
    for child in &children {
        if !matches!(
            construct(child),
            "ActionUsage" | "SuccessionAsUsage" | "MetadataUsage"
        ) {
            return Err(invalid(format!(
                "{} ({}) is not a sequential action or succession",
                child.element_id,
                construct(child)
            )));
        }
    }
    let mut durations = BTreeMap::new();
    for action in &actions {
        if action.properties.contains_key("multiplicity")
            || action.properties.get("is_abstract") == Some(&Value::Bool(true))
        {
            return Err(invalid(
                "explicit action multiplicity and abstract actions are unsupported",
            ));
        }
        if graph.elements().iter().any(|e| {
            activity_owner_id(e).as_deref() == Some(action.element_id.as_str())
                && construct(e) != "MetadataUsage"
        }) {
            return Err(invalid(format!(
                "{} must be a leaf action; action bodies are unsupported",
                action.element_id
            )));
        }
        if references(action.properties.get("type"))
            .iter()
            .any(|t| *t != "Actions::Action")
        {
            return Err(invalid("typed/inherited action behavior is unsupported"));
        }
        let p = properties(runtime, action, "Duration", &["seconds"])
            .map_err(map_adapter_error)?
            .ok_or_else(|| invalid(format!("{} requires Duration", action.element_id)))?;
        let seconds = number(p, "seconds").map_err(map_adapter_error)?;
        if seconds <= 0.0 {
            return Err(invalid("durations must be positive finite literal seconds"));
        }
        durations.insert(action.element_id.clone(), seconds);
    }
    let mut next = BTreeMap::<String, (String, String)>::new();
    let mut incoming = BTreeSet::new();
    for flow in children
        .iter()
        .filter(|e| construct(e) == "SuccessionAsUsage")
    {
        if ["guard", "guard_expression", "expression_ir"]
            .iter()
            .any(|key| flow.properties.contains_key(*key))
        {
            return Err(invalid("guarded successions are unsupported"));
        }
        if graph.elements().iter().any(|e| {
            activity_owner_id(e).as_deref() == Some(flow.element_id.as_str())
                && construct(e) != "ReferenceUsage"
        }) {
            return Err(invalid("guarded or decorated successions are unsupported"));
        }
        let source = endpoint(graph, flow, "earlierOccurrence", &actions)?;
        let target = endpoint(graph, flow, "laterOccurrence", &actions)?;
        if next
            .insert(source, (target.clone(), flow.element_id.clone()))
            .is_some()
            || !incoming.insert(target)
        {
            return Err(invalid("forks and joins are unsupported"));
        }
    }
    let roots = actions
        .iter()
        .filter(|a| !incoming.contains(&a.element_id))
        .collect::<Vec<_>>();
    if roots.len() != 1 {
        return Err(invalid(
            "sequence must have one entry; cycles and disconnected actions are unsupported",
        ));
    }
    let mut ordered = Vec::new();
    let mut cursor = roots[0].element_id.clone();
    loop {
        if ordered.contains(&cursor) {
            return Err(invalid("cycles are unsupported"));
        }
        ordered.push(cursor.clone());
        match next.get(&cursor) {
            Some((target, _)) => cursor = target.clone(),
            None => break,
        }
    }
    if ordered.len() != actions.len() {
        return Err(invalid("all actions must belong to one connected sequence"));
    }
    let case = graph
        .element_by_element_id(&spec.case_ref.element_id)
        .ok_or_else(|| invalid("missing analysis case"))?;
    if annotated(case, "InitialStimulus") {
        return Err(invalid(
            "timed activity starts at zero; InitialStimulus is unsupported",
        ));
    }
    let subject = &binding.subject.element_id;
    let mut scenario = ConcurrentSimulationScenario {
        id: case.element_id.clone(),
        subjects: vec![ConcurrentSubjectScenario {
            subject_id: subject.clone(),
            machine_id: behavior.element_id.clone(),
            initial_state_id: Some(ordered[0].clone()),
            events: vec![],
        }],
        max_steps: 300,
        step_duration_s: 1.0,
        clock_config: Some(SimulationClockConfig::default()),
        termination_policy: Default::default(),
        initial_values: BTreeMap::from([(
            (subject.clone(), completion.to_owned()),
            Value::Bool(false),
        )]),
        requirements: adapter::native_analysis_requirements(runtime, case),
        objectives: vec![],
    };
    adapter::mission_metadata::apply(runtime, case, &mut scenario).map_err(map_adapter_error)?;
    let clock = scenario.clock_config.clone().unwrap();
    let completed_id = format!("{}.completed", behavior.element_id);
    let mut states = actions
        .iter()
        .map(|a| {
            state(
                a.element_id.clone(),
                element_label(a),
                a.element_id == ordered[0],
            )
        })
        .collect::<Vec<_>>();
    // Keep observing the completed result until the clock/policy ends, allowing
    // the same sampled deadline evaluator to see an exact deadline frame.
    states.push(state(completed_id.clone(), "Completed".into(), false));
    let mut transitions = Vec::new();
    for id in &ordered {
        let last = !next.contains_key(id);
        let (target, transition_id) = next
            .get(id)
            .cloned()
            .unwrap_or_else(|| (completed_id.clone(), format!("{id}.complete")));
        transitions.push(SimulationTransition {
            id: transition_id,
            source: id.clone(),
            target,
            trigger: SimulationTrigger {
                kind: SimulationTriggerKind::After,
                value: Some(durations[id].to_string()),
            },
            guard: None,
            effects: if last {
                vec![SimulationEffect::Assign(AssignEffect {
                    feature: completion.into(),
                    value: Value::Bool(true),
                })]
            } else {
                vec![]
            },
        });
    }
    let machine = SimulationStateMachine {
        id: behavior.element_id.clone(),
        label: element_label(behavior),
        states,
        transitions,
    };
    let model = SimulationModel {
        id: case.element_id.clone(),
        machines: vec![machine.clone()],
        derived_rules: vec![],
        binding_rules: vec![],
    };
    let mut trace = run_concurrent_simulation_model(&model, scenario, clock)
        .map_err(|e| invalid(e.to_string()))?;
    if let Some(first) = trace.timeline.first_mut() {
        let action = actions.iter().find(|a| a.element_id == ordered[0]).unwrap();
        first
            .events
            .push(action_event("action_start", subject, action));
    }
    for frame in &mut trace.timeline {
        let events = frame
            .events
            .iter()
            .filter(|e| e.kind == "transition")
            .cloned()
            .collect::<Vec<_>>();
        for event in events {
            if let Some(transition) = machine
                .transitions
                .iter()
                .find(|t| Some(&t.id) == event.transition_id.as_ref())
            {
                let action = actions
                    .iter()
                    .find(|a| a.element_id == transition.source)
                    .unwrap();
                frame
                    .events
                    .push(action_event("action_end", subject, action));
                if let Some(action) = actions.iter().find(|a| a.element_id == transition.target) {
                    frame
                        .events
                        .push(action_event("action_start", subject, action));
                }
            }
        }
        // The terminal transition is an internal completion step, not authored flow.
        frame.events.retain(|event| {
            !(event.kind == "transition"
                && event.transition_id.as_deref()
                    == Some(format!("{}.complete", ordered.last().unwrap()).as_str()))
        });
        // Internal observation state has no authored diagram identity.
        if let Some(states) = frame.states.get_mut(subject) {
            states.retain(|s| s != &completed_id);
        }
    }
    let mut report = simulation_trace_report_with_overlay(run_id, &case.element_id, trace, None)?;
    report.capability_id = SYSML_ACTIVITY_EXECUTION_CAPABILITY_ID.into();
    report.limitations.push("Bounded timed sequence: positive finite leaf durations, one subject/activity, Boolean completion output. No token branching, loops, nested actions, item flow, external solvers or FMI.".into());
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile_sysml_text, load_sysml_baseline};
    const SOURCE: &str = include_str!("../../tests/fixtures/timed-activity.sysml");
    fn execute(source: &str) -> Result<Value, SimulationError> {
        let doc = compile_sysml_text(
            source,
            "timed-activity.sysml",
            &load_sysml_baseline().unwrap(),
        )
        .unwrap();
        let runtime = Runtime::from_document(doc).unwrap();
        let spec = project_analysis_spec(&runtime, "ActivityProfile").unwrap();
        assert!(
            spec.expected_artifacts
                .iter()
                .any(|a| a.kind == "simulation_trace")
        );
        assert!(
            !spec
                .readiness_diagnostics
                .iter()
                .any(|d| d.code == "analysis.dynamic.activity_execution.pending")
        );
        Ok(
            run_analysis_case(&runtime, "ActivityProfile", "timed.test")?
                .artifacts
                .remove(0)
                .payload,
        )
    }
    #[test]
    fn timed_sequence_records_boundaries_and_deadline_evidence() {
        let trace = execute(SOURCE).unwrap();
        assert_eq!(trace["requirement_outcomes"][0]["status"], "satisfied");
        assert_eq!(trace["requirement_outcomes"][0]["witness_time_s"], 8.0);
        let events = trace["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|frame| {
                frame["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|e| e["kind"].as_str().unwrap().starts_with("action_"))
                    .map(|e| {
                        (
                            frame["t"].as_f64().unwrap(),
                            e["kind"].as_str().unwrap().to_owned(),
                            e["trigger"].as_str().unwrap().to_owned(),
                        )
                    })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            events,
            vec![
                (0., "action_start".into(), "warmup".into()),
                (2., "action_end".into(), "warmup".into()),
                (2., "action_start".into(), "hold".into()),
                (7., "action_end".into(), "hold".into()),
                (7., "action_start".into(), "measure".into()),
                (8., "action_end".into(), "measure".into())
            ]
        );
        assert_eq!(trace, execute(SOURCE).unwrap());
        let fractional = execute(
            &SOURCE
                .replace("seconds = 2.0", "seconds = 2.5")
                .replace("deadline : Real = 8.0", "deadline : Real = 8.5"),
        )
        .unwrap();
        assert_eq!(fractional["requirement_outcomes"][0]["status"], "satisfied");
        assert_eq!(fractional["requirement_outcomes"][0]["witness_time_s"], 8.5);
        let late = execute(&SOURCE.replace("seconds = 2.0", "seconds = 3.0")).unwrap();
        assert_eq!(late["requirement_outcomes"][0]["status"], "violated");
        let short = execute(&SOURCE.replace("maxTime = 10.0", "maxTime = 6.0")).unwrap();
        assert_eq!(short["requirement_outcomes"][0]["status"], "unevaluated");
        assert!(short["timeline"].as_array().unwrap().iter().all(|f| {
            f["events"]
                .as_array()
                .unwrap()
                .iter()
                .all(|e| e["trigger"] != "measure")
        }));
    }
    #[test]
    fn timed_sequence_matches_registered_workspace_normalization() {
        let doc = compile_sysml_text(
            SOURCE,
            "timed-activity.sysml",
            &load_sysml_baseline().unwrap(),
        )
        .unwrap();
        let merged = mercurio_foundation::KirDocument::merge_with_registered_fields(
            vec![doc],
            crate::sysml_field_specs().iter().copied(),
        )
        .unwrap();
        let runtime = Runtime::from_document(merged).unwrap();
        let payload = run_analysis_case(&runtime, "ActivityProfile", "timed.test")
            .unwrap()
            .artifacts
            .remove(0)
            .payload;
        assert_eq!(payload, execute(SOURCE).unwrap());
    }
    #[test]
    fn timed_sequence_rejects_unsupported_graphs_and_durations() {
        for source in [
            SOURCE.replace("seconds = 2.0", "seconds = -2.0"),
            SOURCE.replace("seconds = 2.0", "seconds = 0.0"),
            SOURCE.replace(
                "subject device : Instrument;",
                "subject device : Instrument; subject other : Instrument;",
            ),
            SOURCE.replace("action warmup {", "action warmup[2] {"),
            SOURCE.replace("seconds = 2.0", "seconds = 1.0 + 1.0"),
            SOURCE.replace("@Mercurio::Missions::Duration { seconds = 2.0; }", ""),
            SOURCE.replace(
                "first warmup then hold;",
                "first warmup then hold; first warmup then measure;",
            ),
            SOURCE.replace("first hold then measure;", "first hold then warmup;"),
            SOURCE.replace("first hold then measure;", ""),
            SOURCE.replace(
                "completionFeature = \"completed\"",
                "completionFeature = \"missing\"",
            ),
        ] {
            assert!(
                execute(&source).is_err(),
                "unsupported activity executed: {source}"
            );
        }
    }
}
