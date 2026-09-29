//! Bounded, opt-in acyclic timed activities lowered to the shared simulation scheduler.
use super::adapter::mission_metadata::{number, properties};
use super::*;
use mercurio_foundation::simulation_core::{
    AssignEffect, SimulationEffect, SimulationGuard, SimulationState, SimulationStateMachine,
    SimulationTransition, SimulationTrigger,
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
#[derive(Debug, Clone)]
struct ActionIdentity {
    id: String,
    label: String,
}

/// Owned lowering shared by batch execution and resumable sessions.
#[derive(Debug, Clone)]
pub(super) struct PreparedTimedActivity {
    pub(super) model: SimulationModel,
    pub(super) scenario: ConcurrentSimulationScenario,
    pub(super) clock: SimulationClockConfig,
    subject: String,
    actions: Vec<ActionIdentity>,
    initial_actions: Vec<String>,
    internal_states: BTreeSet<String>,
    internal_transitions: BTreeSet<String>,
    internal_features: BTreeSet<String>,
}

impl PreparedTimedActivity {
    pub(super) fn is_internal_feature(&self, subject: &str, feature: &str) -> bool {
        subject == self.subject && self.internal_features.contains(feature)
    }

    pub(super) fn decorate_trace(&self, trace: &mut SimulationTrace) {
        let action_event = |kind: &str, action: &ActionIdentity| SimTraceEvent {
            kind: kind.into(),
            subject_id: Some(self.subject.clone()),
            transition_id: None,
            trigger: Some(action.label.clone()),
            reason: Some(action.id.clone()),
        };
        if let Some(first) = trace.timeline.first_mut() {
            for id in &self.initial_actions {
                if let Some(action) = self.actions.iter().find(|action| &action.id == id) {
                    let event = action_event("action_start", action);
                    if !first.events.contains(&event) {
                        first.events.push(event);
                    }
                }
            }
        }
        for frame in &mut trace.timeline {
            let events = frame
                .events
                .iter()
                .filter(|event| event.kind == "transition")
                .cloned()
                .collect::<Vec<_>>();
            for event in events {
                if let Some(transition) = self
                    .model
                    .machines
                    .iter()
                    .flat_map(|machine| &machine.transitions)
                    .find(|transition| Some(&transition.id) == event.transition_id.as_ref())
                {
                    for (kind, id) in [
                        ("action_end", &transition.source),
                        ("action_start", &transition.target),
                    ] {
                        if let Some(action) = self.actions.iter().find(|action| &action.id == id) {
                            let event = action_event(kind, action);
                            if !frame.events.contains(&event) {
                                frame.events.push(event);
                            }
                        }
                    }
                }
            }
            frame.events.retain(|event| {
                !(event.kind == "transition"
                    && event
                        .transition_id
                        .as_ref()
                        .is_some_and(|id| self.internal_transitions.contains(id)))
            });
            if let Some(states) = frame.states.get_mut(&self.subject) {
                states.retain(|id| !self.internal_states.contains(id));
            }
            frame.values.retain(|(subject, feature), _| {
                subject != &self.subject || !self.internal_features.contains(feature)
            });
        }
        trace.channels.retain(|channel| {
            !self
                .internal_features
                .iter()
                .any(|feature| channel.id == format!("{}.{}", self.subject, feature))
        });
    }
}

pub(super) fn prepare(
    runtime: &Runtime,
    spec: &AnalysisSpec,
) -> Result<PreparedTimedActivity, SimulationError> {
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
        || behavior.properties.get("is_abstract") == Some(&Value::Bool(true))
        || references(behavior.properties.get("type"))
            .iter()
            .any(|t| *t != "Actions::Action")
    {
        return Err(invalid(
            "activity must be a concrete inline action usage without inherited behavior",
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
        || attributes[0].properties.contains_key("multiplicity")
        || !references(attributes[0].properties.get("type")).contains(&"ScalarValues::Boolean")
        || adapter::attribute_default_value(attributes[0]).map_err(map_adapter_error)?
            != Some(Value::Bool(false))
    {
        return Err(invalid(
            "completionFeature must resolve uniquely to an authored scalar Boolean attribute initialized to false; explicit multiplicity is unsupported",
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
        return Err(invalid("a timed activity needs 1 to 1000 leaf actions"));
    }
    for child in &children {
        if !matches!(
            construct(child),
            "ActionUsage" | "SuccessionAsUsage" | "MetadataUsage"
        ) {
            return Err(invalid(format!(
                "{} ({}) is not a leaf action or unguarded succession; item flow and control nodes are unsupported",
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
    let mut outgoing = BTreeMap::<String, Vec<(String, String)>>::new();
    let mut incoming = BTreeMap::<String, Vec<String>>::new();
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
        let successors = outgoing.entry(source.clone()).or_default();
        if successors.iter().any(|(existing, _)| existing == &target) {
            return Err(invalid("duplicate succession dependencies are unsupported"));
        }
        successors.push((target.clone(), flow.element_id.clone()));
        incoming.entry(target).or_default().push(source);
    }
    let roots = actions
        .iter()
        .filter(|action| !incoming.contains_key(&action.element_id))
        .map(|action| action.element_id.clone())
        .collect::<Vec<_>>();
    if roots.len() != 1 {
        return Err(invalid(
            "activity must have one entry; cycles and disconnected actions are unsupported",
        ));
    }
    // Kahn ordering proves the bounded dependency graph is acyclic. A single
    // entry and complete traversal also exclude disconnected components.
    let mut remaining = incoming
        .iter()
        .map(|(id, predecessors)| (id.clone(), predecessors.len()))
        .collect::<BTreeMap<_, _>>();
    let mut available = BTreeSet::from([roots[0].clone()]);
    let mut ordered = Vec::new();
    while let Some(id) = available.pop_first() {
        ordered.push(id.clone());
        for (target, _) in outgoing.get(&id).into_iter().flatten() {
            let count = remaining
                .get_mut(target)
                .ok_or_else(|| invalid("missing dependency target"))?;
            *count -= 1;
            if *count == 0 {
                available.insert(target.clone());
            }
        }
    }
    if ordered.len() != actions.len() {
        return Err(invalid("cycles and disconnected actions are unsupported"));
    }
    let is_sequence = outgoing.values().all(|edges| edges.len() <= 1)
        && incoming.values().all(|edges| edges.len() <= 1);
    let next = outgoing
        .iter()
        .filter_map(|(id, edges)| edges.first().map(|edge| (id.clone(), edge.clone())))
        .collect::<BTreeMap<_, _>>();
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
        requirements: adapter::native_analysis_requirements(runtime, case)
            .map_err(map_adapter_error)?,
        objectives: vec![],
    };
    adapter::mission_metadata::apply(runtime, case, &mut scenario).map_err(map_adapter_error)?;
    let clock = scenario
        .clock_config
        .clone()
        .ok_or_else(|| invalid("missing simulation clock"))?;
    if !is_sequence {
        return prepare_dependency_activity(
            behavior, subject, completion, &actions, &durations, &incoming, scenario, clock,
        );
    }
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
            constraint_networks: Vec::new(),
        id: case.element_id.clone(),
        machines: vec![machine.clone()],
        derived_rules: vec![],
        binding_rules: vec![],
    };
    let last_action = ordered
        .last()
        .ok_or_else(|| invalid("missing terminal action"))?;
    Ok(PreparedTimedActivity {
        model,
        scenario,
        clock,
        subject: subject.clone(),
        actions: actions
            .iter()
            .map(|action| ActionIdentity {
                id: action.element_id.clone(),
                label: element_label(action),
            })
            .collect(),
        initial_actions: vec![ordered[0].clone()],
        internal_states: BTreeSet::from([completed_id]),
        internal_transitions: BTreeSet::from([format!("{last_action}.complete")]),
        internal_features: BTreeSet::new(),
    })
}

/// Each action owns an orthogonal region. Its running state keeps the authored
/// action id, while only scheduler-private states/flags represent dependencies.
fn prepare_dependency_activity(
    behavior: &Element,
    subject: &str,
    completion: &str,
    actions: &[&Element],
    durations: &BTreeMap<String, f64>,
    incoming: &BTreeMap<String, Vec<String>>,
    mut scenario: ConcurrentSimulationScenario,
    clock: SimulationClockConfig,
) -> Result<PreparedTimedActivity, SimulationError> {
    let root_id = format!("{}.parallel", behavior.element_id);
    let completed_id = format!("{}.completed", behavior.element_id);
    let mut root = state(root_id.clone(), "Activity branches".into(), true);
    root.is_orthogonal = true;
    let mut states = vec![root, state(completed_id.clone(), "Completed".into(), false)];
    let mut transitions = Vec::new();
    let mut internal_states = BTreeSet::from([root_id.clone(), completed_id.clone()]);
    let mut initial_actions = Vec::new();
    let flags = actions
        .iter()
        .enumerate()
        .map(|(index, action)| {
            let mut name = format!("__timed_activity_done_{index}");
            while name == completion {
                name.push('_');
            }
            (action.element_id.clone(), name)
        })
        .collect::<BTreeMap<_, _>>();
    // Shared expression IR evaluates every prerequisite; the adapter contains
    // no separate guard evaluator or host-side schedule.
    let all_done = |ids: &[String]| -> Result<SimulationGuard, SimulationError> {
        let mut terms = ids
            .iter()
            .map(|id| {
                let feature = flags
                    .get(id)
                    .ok_or_else(|| invalid("missing dependency completion flag"))?;
                Ok(serde_json::json!({"kind":"path", "segments":[feature]}))
            })
            .collect::<Result<Vec<_>, SimulationError>>()?;
        // Keep large valid joins shallow for the shared expression validator.
        while terms.len() > 1 {
            terms = terms
                .chunks(2)
                .map(|pair| match pair {
                    [left, right] => {
                        serde_json::json!({"kind":"binary", "op":"and", "left":left, "right":right})
                    }
                    [single] => single.clone(),
                    _ => serde_json::json!({"kind":"literal", "value":true}),
                })
                .collect();
        }
        Ok(SimulationGuard::ExpressionIr(terms.pop().unwrap_or_else(
            || serde_json::json!({"kind":"literal", "value":true}),
        )))
    };
    for action in actions {
        let id = &action.element_id;
        let region_id = format!("{id}.region");
        let waiting_id = format!("{id}.waiting");
        let done_id = format!("{id}.done");
        let predecessors = incoming.get(id).map(Vec::as_slice).unwrap_or(&[]);
        let is_entry = predecessors.is_empty();
        let mut region = state(
            region_id.clone(),
            format!("{} region", element_label(action)),
            true,
        );
        region.parent_state_id = Some(root_id.clone());
        states.push(region);
        for (state_id, label, initial) in [
            (waiting_id.clone(), "Waiting".into(), !is_entry),
            (id.clone(), element_label(action), is_entry),
            (done_id.clone(), "Done".into(), false),
        ] {
            let mut child = state(state_id, label, initial);
            child.parent_state_id = Some(region_id.clone());
            states.push(child);
        }
        internal_states.extend([region_id, waiting_id.clone(), done_id.clone()]);
        if is_entry {
            initial_actions.push(id.clone());
        } else {
            transitions.push(SimulationTransition {
                id: format!("{id}.start"),
                source: waiting_id,
                target: id.clone(),
                trigger: SimulationTrigger {
                    kind: SimulationTriggerKind::Completion,
                    value: None,
                },
                guard: Some(all_done(predecessors)?),
                effects: vec![],
            });
        }
        let feature = flags
            .get(id)
            .ok_or_else(|| invalid("missing action completion flag"))?;
        scenario
            .initial_values
            .insert((subject.into(), feature.clone()), Value::Bool(false));
        transitions.push(SimulationTransition {
            id: format!("{id}.finish"),
            source: id.clone(),
            target: done_id,
            trigger: SimulationTrigger {
                kind: SimulationTriggerKind::After,
                value: Some(durations[id].to_string()),
            },
            guard: None,
            effects: vec![SimulationEffect::Assign(AssignEffect {
                feature: feature.clone(),
                value: Value::Bool(true),
            })],
        });
    }
    transitions.push(SimulationTransition {
        id: format!("{}.complete", behavior.element_id),
        source: root_id.clone(),
        target: completed_id,
        trigger: SimulationTrigger {
            kind: SimulationTriggerKind::Completion,
            value: None,
        },
        guard: Some(all_done(
            &actions
                .iter()
                .map(|action| action.element_id.clone())
                .collect::<Vec<_>>(),
        )?),
        effects: vec![SimulationEffect::Assign(AssignEffect {
            feature: completion.into(),
            value: Value::Bool(true),
        })],
    });
    let internal_transitions = transitions
        .iter()
        .map(|transition| transition.id.clone())
        .collect();
    if let Some(subject_scenario) = scenario.subjects.first_mut() {
        subject_scenario.initial_state_id = Some(root_id);
    }
    let model = SimulationModel {
            constraint_networks: Vec::new(),
        id: scenario.id.clone(),
        machines: vec![SimulationStateMachine {
            id: behavior.element_id.clone(),
            label: element_label(behavior),
            states,
            transitions,
        }],
        derived_rules: vec![],
        binding_rules: vec![],
    };
    Ok(PreparedTimedActivity {
        model,
        scenario,
        clock,
        subject: subject.into(),
        actions: actions
            .iter()
            .map(|action| ActionIdentity {
                id: action.element_id.clone(),
                label: element_label(action),
            })
            .collect(),
        initial_actions,
        internal_states,
        internal_transitions,
        internal_features: flags.into_values().collect(),
    })
}

pub(super) fn run(
    runtime: &Runtime,
    spec: &AnalysisSpec,
    run_id: &str,
) -> Result<CapabilityRunReport, SimulationError> {
    let prepared = prepare(runtime, spec)?;
    let mut trace = run_concurrent_simulation_model(
        &prepared.model,
        prepared.scenario.clone(),
        prepared.clock.clone(),
    )
    .map_err(|error| invalid(error.to_string()))?;
    prepared.decorate_trace(&mut trace);
    let mut report =
        simulation_trace_report_with_overlay(run_id, &spec.case_ref.element_id, trace, None)?;
    report.capability_id = SYSML_ACTIVITY_EXECUTION_CAPABILITY_ID.into();
    report.limitations.push("Bounded timed activity: positive finite leaf durations, one subject/activity, Boolean completion output. Acyclic succession dependencies support fork/join; no loops, guarded flow, nested actions, item flow, external solvers or FMI.".into());
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
    fn timed_sequence_rejects_abstract_activity_and_collection_output() {
        for source in [
            SOURCE.replace("action procedure {", "abstract action procedure {"),
            SOURCE.replace("action warmup {", "abstract action warmup {"),
            SOURCE.replace(
                "attribute completed : Boolean = false;",
                "attribute completed : Boolean[2] = false;",
            ),
        ] {
            let error = execute(&source)
                .err()
                .unwrap_or_else(|| panic!("unsupported declaration executed: {source}"));
            assert!(format!("{error:?}").contains("activity.timed.unsupported"));
        }
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
                "first warmup then hold; first warmup then hold;",
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

    const FORK_JOIN: &str = include_str!("../../tests/fixtures/timed-activity-fork-join.sysml");

    #[test]
    fn timed_fork_join_waits_for_every_predecessor_and_retains_authored_ids() {
        let trace = execute(FORK_JOIN).unwrap();
        assert_eq!(trace["requirement_outcomes"][0]["status"], "satisfied");
        assert_eq!(trace["requirement_outcomes"][0]["witness_time_s"], 8.0);
        let mut boundaries = BTreeMap::new();
        let mut authored_ids = BTreeSet::new();
        for frame in trace["timeline"].as_array().unwrap() {
            for event in frame["events"].as_array().unwrap() {
                if event["kind"].as_str().unwrap().starts_with("action_") {
                    let key = (
                        event["trigger"].as_str().unwrap().to_owned(),
                        event["kind"].as_str().unwrap().to_owned(),
                    );
                    assert!(
                        boundaries
                            .insert(key, frame["t"].as_f64().unwrap())
                            .is_none(),
                        "action executes only once"
                    );
                    authored_ids.insert(event["reason"].as_str().unwrap().to_owned());
                }
            }
        }
        for (name, start, end) in [("a", 0., 2.), ("b", 2., 5.), ("c", 2., 7.), ("d", 7., 8.)] {
            assert_eq!(
                boundaries.get(&(name.into(), "action_start".into())),
                Some(&start)
            );
            assert_eq!(
                boundaries.get(&(name.into(), "action_end".into())),
                Some(&end)
            );
        }
        let parallel_frame = trace["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .find(|frame| frame["t"] == 3.0)
            .unwrap();
        assert_eq!(
            parallel_frame["states"]
                .as_object()
                .unwrap()
                .values()
                .next()
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            2
        );
        for frame in trace["timeline"].as_array().unwrap() {
            for states in frame["states"].as_object().unwrap().values() {
                assert!(
                    states
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|id| authored_ids.contains(id.as_str().unwrap()))
                );
            }
        }
        assert!(!trace.to_string().contains("__timed_activity_done_"));
        assert_eq!(trace, execute(FORK_JOIN).unwrap());
        let late = execute(&FORK_JOIN.replace("seconds = 5.0", "seconds = 6.0")).unwrap();
        assert_eq!(late["requirement_outcomes"][0]["status"], "violated");
        let incomplete = execute(&FORK_JOIN.replace("maxTime = 10.0", "maxTime = 6.0")).unwrap();
        assert_eq!(
            incomplete["requirement_outcomes"][0]["status"],
            "unevaluated"
        );
        assert!(
            incomplete["timeline"]
                .as_array()
                .unwrap()
                .iter()
                .all(|frame| frame["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|event| event["trigger"] != "d"))
        );
    }

    #[test]
    fn prepared_activity_decoration_is_idempotent_for_sequences_and_dependencies() {
        for source in [SOURCE, FORK_JOIN] {
            let document =
                compile_sysml_text(source, "activity.sysml", &load_sysml_baseline().unwrap())
                    .unwrap();
            let runtime = Runtime::from_document(document).unwrap();
            let spec = project_analysis_spec(&runtime, "ActivityProfile").unwrap();
            let prepared = prepare(&runtime, &spec).unwrap();
            let mut trace = run_concurrent_simulation_model(
                &prepared.model,
                prepared.scenario.clone(),
                prepared.clock.clone(),
            )
            .unwrap();
            prepared.decorate_trace(&mut trace);
            let decorated = trace.clone();
            prepared.decorate_trace(&mut trace);
            assert_eq!(trace, decorated);
        }
    }

    #[test]
    fn timed_dependencies_reject_cycles_duplicate_edges_and_nested_actions() {
        for source in [
            FORK_JOIN.replace("first c then d;", "first c then d; first d then b;"),
            FORK_JOIN.replace("first a then b;", "first a then b; first a then b;"),
            FORK_JOIN.replace("action b {", "action b { action nested;"),
        ] {
            let error = execute(&source).expect_err("unsupported timed dependency graph executed");
            assert!(format!("{error:?}").contains("activity.timed.unsupported"));
        }
    }
}
