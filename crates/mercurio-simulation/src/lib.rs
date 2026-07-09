use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde_json::Value;

mod adapter;

use mercurio_core::graph::{Element, Graph};
use mercurio_core::runtime::{Runtime, RuntimeError};
use mercurio_core::{
    AnalysisScope, CapabilityError, CapabilityRunReport, CapabilityRunRequest, CapabilityRunStatus,
    CapabilityTarget, DiagnosticKind, EvidenceEdge, EvidenceGraph, EvidenceNode, EvidenceNodeKind,
    EvidenceRelation, SemanticArtifact, SemanticCapability, SemanticDiagnostic,
    SemanticDiagnosticSeverity, SemanticElementRef, SemanticWorkspaceSnapshot, stable_digest,
};
pub use mercurio_simulation_core::{
    AnalysisCaseInfo, ConcurrentSimulationScenario, ConcurrentSubjectScenario, SimTraceChannel,
    SimTraceChannelSource, SimTraceEntry, SimTraceEvent, SimulationClockConfig, SimulationEvent,
    SimulationModel, SimulationStatus, SimulationTrace, SimulationTriggerKind, TraceChannel,
    TraceChannelSource, TraceEntry, TraceEvent, run_concurrent_simulation_model,
};
pub use mercurio_sysml::{
    AnalysisClockConfig, AnalysisDynamicBehaviorBinding, AnalysisDynamicBehaviorKind,
    AnalysisExecutionContext, AnalysisExecutionPlan, AnalysisExecutionStep,
    AnalysisExecutionStepKind, AnalysisExpectedArtifact, AnalysisReadinessDiagnostic,
    AnalysisReadinessSeverity, AnalysisReadinessStatus, AnalysisSpec, AnalysisSpecError,
    AnalysisTechnique, list_analysis_specs, project_analysis_spec,
};
use mercurio_views::{
    ViewEdgeMarkDto, ViewNodeMarkDto, ViewNodeValueDto, ViewOverlayDto, ViewOverlayFrameDto,
};

const CHANGE_LOOP_LIMIT: usize = 20;
pub const SYSML_DYNAMIC_BEHAVIOR_CAPABILITY_ID: &str = "sysml.behavior.dynamic";
pub const SYSML_ANALYSIS_CASE_CAPABILITY_ID: &str = "sysml.analysis.case";
pub const SYSML_CONSTRAINT_ANALYSIS_CAPABILITY_ID: &str = "sysml.constraint.analysis";
pub const SYSML_ACTIVITY_EXECUTION_CAPABILITY_ID: &str = "sysml.activity.execution";
pub const SYSML_SIMULATION_CAPABILITY_VERSION: &str = "0.1.0";
pub const SIMULATION_TRACE_ARTIFACT_KIND: &str = "simulation_trace";
pub const SIMULATION_TRACE_SCHEMA: &str = "mercurio.simulation.trace.v1";
pub const ACTIVITY_EXECUTION_SUMMARY_ARTIFACT_KIND: &str = "activity_execution_summary";
pub const ACTIVITY_EXECUTION_SUMMARY_SCHEMA: &str =
    "mercurio.analysis.activity_execution_summary.v1";

pub type StateMachineScenarioEvent = SimulationEvent;

#[derive(Debug)]
pub enum SimulationError {
    MissingAnalysisCase(String),
    MissingSubject(String),
    MissingStateMachine(String),
    MissingInitialState(String),
    InvalidProfile(String),
    Runtime(RuntimeError),
    Serialization(serde_json::Error),
    Capability(CapabilityError),
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAnalysisCase(id) => write!(f, "missing analysis case: {id}"),
            Self::MissingSubject(id) => write!(f, "missing simulation subject: {id}"),
            Self::MissingStateMachine(id) => write!(f, "missing state machine: {id}"),
            Self::MissingInitialState(id) => write!(f, "missing initial state: {id}"),
            Self::InvalidProfile(message) => write!(f, "invalid simulation profile: {message}"),
            Self::Runtime(err) => write!(f, "{err}"),
            Self::Serialization(err) => write!(f, "failed to serialize simulation trace: {err}"),
            Self::Capability(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SimulationError {}

impl From<RuntimeError> for SimulationError {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

impl From<serde_json::Error> for SimulationError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value)
    }
}

impl From<CapabilityError> for SimulationError {
    fn from(value: CapabilityError) -> Self {
        Self::Capability(value)
    }
}

pub fn canonical_simulation_model(runtime: &Runtime) -> Result<SimulationModel, SimulationError> {
    adapter::simulation_model_from_runtime(runtime).map_err(map_adapter_error)
}

pub fn trace_to_view_overlay(trace: &SimulationTrace) -> ViewOverlayDto {
    ViewOverlayDto {
        version: 1,
        frames: trace
            .timeline
            .iter()
            .enumerate()
            .map(|(index, entry)| trace_entry_to_overlay_frame(index, entry))
            .collect(),
    }
}

fn trace_entry_to_overlay_frame(index: usize, entry: &SimTraceEntry) -> ViewOverlayFrameDto {
    let mut node_marks = Vec::new();
    for (subject_id, states) in &entry.states {
        for state_id in states {
            let mut properties = serde_json::Map::new();
            properties.insert("subject".to_string(), Value::String(subject_id.clone()));
            node_marks.push(ViewNodeMarkDto {
                element: state_id.clone(),
                kind: "active_state".to_string(),
                label: Some("active".to_string()),
                properties,
            });
        }
    }

    let mut node_values = Vec::new();
    for ((subject_id, feature_id), value) in &entry.values {
        node_values.push(ViewNodeValueDto {
            element: subject_id.clone(),
            key: feature_id.clone(),
            value: value.clone(),
            label: Some(label_for_simulation_feature(feature_id)),
            unit: None,
        });
    }

    let mut edge_marks = Vec::new();
    for event in &entry.events {
        let Some(transition_id) = event.transition_id.as_ref() else {
            continue;
        };
        let mut properties = serde_json::Map::new();
        if let Some(subject_id) = event.subject_id.as_ref() {
            properties.insert("subject".to_string(), Value::String(subject_id.clone()));
        }
        if let Some(trigger) = event.trigger.as_ref() {
            properties.insert("trigger".to_string(), Value::String(trigger.clone()));
        }
        if let Some(reason) = event.reason.as_ref() {
            properties.insert("reason".to_string(), Value::String(reason.clone()));
        }
        edge_marks.push(ViewEdgeMarkDto {
            element: transition_id.clone(),
            kind: "visited_transition".to_string(),
            label: Some("visited".to_string()),
            properties,
        });
    }

    ViewOverlayFrameDto {
        index,
        time_s: Some(entry.t),
        node_marks,
        edge_marks,
        node_values,
        warnings: Vec::new(),
    }
}

fn label_for_simulation_feature(feature_id: &str) -> String {
    feature_id
        .rsplit(['.', ':'])
        .next()
        .filter(|label| !label.is_empty())
        .unwrap_or(feature_id)
        .to_string()
}

fn map_adapter_error(error: adapter::SysmlSimulationAdapterError) -> SimulationError {
    match error {
        adapter::SysmlSimulationAdapterError::InvalidProfile(error) => {
            SimulationError::InvalidProfile(
                error
                    .findings
                    .into_iter()
                    .map(|finding| format!("{}: {}", finding.code, finding.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        }
        adapter::SysmlSimulationAdapterError::MissingAnalysisCase(id) => {
            SimulationError::MissingAnalysisCase(id)
        }
        adapter::SysmlSimulationAdapterError::MissingStateMachine(id) => {
            SimulationError::MissingStateMachine(id)
        }
        adapter::SysmlSimulationAdapterError::InvalidAnalysisCase(message) => {
            SimulationError::InvalidProfile(message)
        }
    }
}

pub fn list_analysis_cases(runtime: &Runtime) -> Vec<AnalysisCaseInfo> {
    adapter::list_analysis_cases(runtime)
}

pub fn scenario_from_analysis_case(
    runtime: &Runtime,
    analysis_case_id: &str,
) -> Result<ConcurrentSimulationScenario, SimulationError> {
    adapter::scenario_from_analysis_case(runtime, analysis_case_id).map_err(map_adapter_error)
}

pub fn run_analysis_case(
    runtime: &Runtime,
    analysis_case_id: &str,
    run_id: &str,
) -> Result<CapabilityRunReport, SimulationError> {
    let spec = project_analysis_spec(runtime, analysis_case_id).map_err(map_analysis_spec_error)?;
    let mut reports = Vec::new();

    if has_executable_state_machine_binding(&spec) {
        let scenario = scenario_from_analysis_case(runtime, analysis_case_id)?;
        let trace = run_concurrent_simulation(runtime, scenario)?;
        reports.push(simulation_trace_report(run_id, analysis_case_id, trace)?);
    }

    if requires_constraint_analysis(&spec) {
        reports.push(constraint_analysis_report(runtime, &spec, run_id)?);
    }

    if has_activity_binding(&spec) {
        reports.push(activity_execution_report(runtime, &spec, run_id)?);
    }

    match reports.len() {
        0 => Err(SimulationError::InvalidProfile(format!(
            "analysis case `{analysis_case_id}` does not have an executable or reportable Phase 4 technique"
        ))),
        1 => Ok(reports.remove(0)),
        _ => Ok(composite_analysis_report(run_id, &spec, reports)),
    }
}

fn map_analysis_spec_error(error: AnalysisSpecError) -> SimulationError {
    match error {
        AnalysisSpecError::MissingAnalysisCase(id) => SimulationError::MissingAnalysisCase(id),
    }
}

fn has_executable_state_machine_binding(spec: &AnalysisSpec) -> bool {
    spec.dynamic_behavior_bindings
        .iter()
        .any(|binding| binding.kind == AnalysisDynamicBehaviorKind::StateMachine)
}

fn has_activity_binding(spec: &AnalysisSpec) -> bool {
    spec.dynamic_behavior_bindings
        .iter()
        .any(|binding| binding.kind == AnalysisDynamicBehaviorKind::Activity)
}

fn requires_constraint_analysis(spec: &AnalysisSpec) -> bool {
    spec.techniques.iter().any(|technique| {
        matches!(
            technique,
            AnalysisTechnique::Calculation
                | AnalysisTechnique::ConstraintEvaluation
                | AnalysisTechnique::Verification
        )
    })
}

fn constraint_analysis_report(
    runtime: &Runtime,
    spec: &AnalysisSpec,
    run_id: &str,
) -> Result<CapabilityRunReport, SimulationError> {
    let workspace = SemanticWorkspaceSnapshot::from_graph_with_profile(
        runtime.graph().clone(),
        Some("sysml".to_string()),
    )?;
    let context_values = serde_json::to_value(&spec.execution_context.initial_values)?;
    mercurio_sysml::SysmlConstraintAnalysisCapability
        .run(
            &workspace,
            CapabilityRunRequest {
                run_id: run_id.to_string(),
                capability_id: SYSML_CONSTRAINT_ANALYSIS_CAPABILITY_ID.to_string(),
                target: CapabilityTarget::Scope {
                    scope_id: spec.case_ref.element_id.clone(),
                },
                parameters: BTreeMap::from([
                    (
                        "analysis_scope".to_string(),
                        Value::String(AnalysisScope::AuthoredModel.as_str().to_string()),
                    ),
                    (
                        "analysis_case_id".to_string(),
                        Value::String(spec.case_ref.element_id.clone()),
                    ),
                    ("context_values".to_string(), context_values),
                ]),
                input_artifacts: Vec::new(),
            },
        )
        .map_err(Into::into)
}

fn activity_execution_report(
    runtime: &Runtime,
    spec: &AnalysisSpec,
    run_id: &str,
) -> Result<CapabilityRunReport, SimulationError> {
    let activity_bindings = spec
        .dynamic_behavior_bindings
        .iter()
        .filter(|binding| binding.kind == AnalysisDynamicBehaviorKind::Activity)
        .collect::<Vec<_>>();
    let execution_results = activity_bindings
        .iter()
        .map(|binding| execute_activity_binding(runtime.graph(), binding))
        .collect::<Vec<_>>();
    let status = aggregate_capability_status(execution_results.iter().map(|result| result.status));
    let execution_state = activity_execution_state(status);
    let payload_bindings = activity_bindings
        .iter()
        .zip(execution_results.iter())
        .map(|(binding, result)| {
            let mut payload = result.payload.clone();
            if let Some(object) = payload.as_object_mut() {
                object.insert(
                    "subject".to_string(),
                    analysis_ref_payload(&binding.subject),
                );
                object.insert(
                    "behavior".to_string(),
                    analysis_ref_payload(&binding.behavior),
                );
                object.insert("kind".to_string(), Value::String("activity".to_string()));
            }
            payload
        })
        .collect::<Vec<_>>();
    let payload = serde_json::json!({
        "schema": ACTIVITY_EXECUTION_SUMMARY_SCHEMA,
        "analysisCase": analysis_ref_payload(&spec.case_ref),
        "bindingCount": activity_bindings.len(),
        "status": capability_run_status_label(status),
        "executionState": execution_state,
        "bindings": payload_bindings,
    });
    let payload_bytes = serde_json::to_vec(&payload)?;
    let digest = stable_digest([(
        "activity-execution-summary".as_bytes(),
        payload_bytes.as_slice(),
    )]);
    let artifact_id = format!(
        "artifact.{}.activity_execution.{}",
        sanitize_identifier(run_id),
        sanitize_identifier(&spec.case_ref.element_id)
    );
    let run_evidence_id = format!(
        "evidence.{}.activity_execution.{}",
        sanitize_identifier(run_id),
        sanitize_identifier(&spec.case_ref.element_id)
    );
    let artifact = SemanticArtifact {
        id: artifact_id.clone(),
        kind: ACTIVITY_EXECUTION_SUMMARY_ARTIFACT_KIND.to_string(),
        schema: ACTIVITY_EXECUTION_SUMMARY_SCHEMA.to_string(),
        digest,
        element_refs: activity_bindings
            .iter()
            .flat_map(|binding| {
                [
                    semantic_ref_from_analysis_ref(&binding.subject),
                    semantic_ref_from_analysis_ref(&binding.behavior),
                ]
            })
            .collect(),
        payload,
    };
    let mut evidence_nodes = vec![
        EvidenceNode {
            id: run_evidence_id.clone(),
            kind: EvidenceNodeKind::AnalysisRun,
            label: format!(
                "Activity execution analysis case {}",
                spec.case_ref.element_id
            ),
            element_refs: vec![semantic_ref_from_analysis_ref(&spec.case_ref)],
            source_spans: Vec::new(),
            properties: BTreeMap::from([(
                "analysis_case_id".to_string(),
                Value::String(spec.case_ref.element_id.clone()),
            )]),
        },
        EvidenceNode {
            id: artifact_id.clone(),
            kind: EvidenceNodeKind::Artifact,
            label: "Activity execution summary".to_string(),
            element_refs: Vec::new(),
            source_spans: Vec::new(),
            properties: BTreeMap::new(),
        },
    ];
    evidence_nodes.extend(
        activity_bindings
            .iter()
            .enumerate()
            .map(|(index, binding)| EvidenceNode {
                id: format!(
                    "evidence.{}.activity_binding.{}",
                    sanitize_identifier(run_id),
                    index
                ),
                kind: EvidenceNodeKind::KirElement,
                label: format!(
                    "Activity binding {} -> {}",
                    binding.subject.element_id, binding.behavior.element_id
                ),
                element_refs: vec![
                    semantic_ref_from_analysis_ref(&binding.subject),
                    semantic_ref_from_analysis_ref(&binding.behavior),
                ],
                source_spans: Vec::new(),
                properties: BTreeMap::from([
                    (
                        "subject_id".to_string(),
                        Value::String(binding.subject.element_id.clone()),
                    ),
                    (
                        "behavior_id".to_string(),
                        Value::String(binding.behavior.element_id.clone()),
                    ),
                ]),
            }),
    );
    let diagnostics = execution_results
        .iter()
        .flat_map(|result| result.diagnostics.clone())
        .collect::<Vec<_>>();
    let limitations = dedup_strings(
        execution_results
            .iter()
            .flat_map(|result| result.limitations.clone())
            .chain([
                "Phase 5 activity execution supports deterministic action/control-flow DAGs; object flow, decisions, guards, durations, and streaming tokens are not implemented yet"
                    .to_string(),
            ])
            .collect(),
    );

    Ok(CapabilityRunReport {
        run_id: run_id.to_string(),
        capability_id: SYSML_ACTIVITY_EXECUTION_CAPABILITY_ID.to_string(),
        capability_version: Some(SYSML_SIMULATION_CAPABILITY_VERSION.to_string()),
        status,
        target: CapabilityTarget::Element {
            element_id: spec.case_ref.element_id.clone(),
        },
        insights: Vec::new(),
        artifacts: vec![artifact],
        evidence: EvidenceGraph {
            nodes: evidence_nodes,
            edges: vec![EvidenceEdge {
                source_id: artifact_id,
                target_id: run_evidence_id,
                relation: EvidenceRelation::ProducedBy,
            }],
        },
        diagnostics,
        limitations,
    })
}

#[derive(Debug, Clone)]
struct ActivityBindingExecution {
    status: CapabilityRunStatus,
    payload: Value,
    diagnostics: Vec<SemanticDiagnostic>,
    limitations: Vec<String>,
}

#[derive(Debug, Clone)]
struct ActivityFlow {
    id: String,
    source: String,
    target: String,
}

fn execute_activity_binding(
    graph: &Graph,
    binding: &AnalysisDynamicBehaviorBinding,
) -> ActivityBindingExecution {
    let behavior_id = binding.behavior.element_id.as_str();
    let activity_nodes = activity_execution_nodes(graph, behavior_id);
    if activity_nodes.is_empty() {
        return ActivityBindingExecution {
            status: CapabilityRunStatus::Partial,
            payload: serde_json::json!({
                "status": "partial",
                "executionState": "no_executable_nodes",
                "nodeCount": 0,
                "edgeCount": 0,
                "steps": [],
                "blockedNodes": [],
            }),
            diagnostics: vec![activity_diagnostic(
                "analysis.dynamic.activity_execution.no_nodes",
                format!("activity `{behavior_id}` has no executable action or control nodes"),
                binding,
            )],
            limitations: Vec::new(),
        };
    }

    let node_ids = activity_nodes
        .iter()
        .map(|node| node.element_id.clone())
        .collect::<BTreeSet<_>>();
    let mut diagnostics = Vec::new();
    let flows = activity_flows(graph, behavior_id, &node_ids, binding, &mut diagnostics);
    let mut incoming = node_ids
        .iter()
        .map(|id| (id.clone(), BTreeSet::<String>::new()))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = node_ids
        .iter()
        .map(|id| (id.clone(), BTreeSet::<String>::new()))
        .collect::<BTreeMap<_, _>>();
    for flow in &flows {
        if let Some(targets) = outgoing.get_mut(&flow.source) {
            targets.insert(flow.target.clone());
        }
        if let Some(sources) = incoming.get_mut(&flow.target) {
            sources.insert(flow.source.clone());
        }
    }

    let node_by_id = activity_nodes
        .iter()
        .map(|node| (node.element_id.as_str(), *node))
        .collect::<BTreeMap<_, _>>();
    let mut completed = BTreeSet::<String>::new();
    let mut steps = Vec::<Value>::new();
    let mut step_index = 0usize;
    while completed.len() < node_ids.len() {
        let enabled = node_ids
            .iter()
            .filter(|node_id| !completed.contains(*node_id))
            .filter(|node_id| {
                incoming
                    .get(*node_id)
                    .is_none_or(|predecessors| predecessors.iter().all(|id| completed.contains(id)))
            })
            .cloned()
            .collect::<Vec<_>>();

        if enabled.is_empty() {
            break;
        }

        let executed_nodes = enabled
            .iter()
            .filter_map(|node_id| node_by_id.get(node_id.as_str()).copied())
            .map(activity_node_payload)
            .collect::<Vec<_>>();
        steps.push(serde_json::json!({
            "index": step_index,
            "nodes": executed_nodes,
        }));
        completed.extend(enabled);
        step_index += 1;
    }

    let blocked_nodes = node_ids
        .iter()
        .filter(|node_id| !completed.contains(*node_id))
        .filter_map(|node_id| node_by_id.get(node_id.as_str()).copied())
        .map(activity_node_payload)
        .collect::<Vec<_>>();
    let status = if blocked_nodes.is_empty() && diagnostics.is_empty() {
        CapabilityRunStatus::Passed
    } else {
        if !blocked_nodes.is_empty() {
            diagnostics.push(activity_diagnostic(
                "analysis.dynamic.activity_execution.blocked_graph",
                format!(
                    "activity `{behavior_id}` could not complete; remaining nodes may be cyclic or waiting on unsupported control semantics"
                ),
                binding,
            ));
        }
        CapabilityRunStatus::Partial
    };
    let flow_payload = flows
        .iter()
        .map(|flow| {
            serde_json::json!({
                "id": flow.id.clone(),
                "source": flow.source.clone(),
                "target": flow.target.clone(),
            })
        })
        .collect::<Vec<_>>();

    ActivityBindingExecution {
        status,
        payload: serde_json::json!({
            "status": capability_run_status_label(status),
            "executionState": activity_execution_state(status),
            "nodeCount": node_ids.len(),
            "edgeCount": flows.len(),
            "steps": steps,
            "flows": flow_payload,
            "blockedNodes": blocked_nodes,
        }),
        diagnostics,
        limitations: Vec::new(),
    }
}

fn activity_execution_nodes<'a>(graph: &'a Graph, behavior_id: &str) -> Vec<&'a Element> {
    let mut nodes = graph
        .elements()
        .iter()
        .filter(|element| {
            is_activity_execution_node(element)
                && activity_owner_id(element).as_deref() == Some(behavior_id)
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.element_id.cmp(&right.element_id));
    if nodes.is_empty()
        && let Some(element) = graph.element_by_element_id(behavior_id)
        && is_activity_execution_node(element)
    {
        nodes.push(element);
    }
    nodes
}

fn activity_flows(
    graph: &Graph,
    behavior_id: &str,
    node_ids: &BTreeSet<String>,
    binding: &AnalysisDynamicBehaviorBinding,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) -> Vec<ActivityFlow> {
    let mut flows = Vec::new();
    for element in graph.elements().iter().filter(|element| {
        is_activity_flow_element(element)
            && (activity_owner_id(element).as_deref() == Some(behavior_id)
                || flow_touches_nodes(element, node_ids))
    }) {
        let Some(source) = string_property_any(element, &["source", "source_id", "sourceId"])
        else {
            continue;
        };
        let Some(target) = string_property_any(element, &["target", "target_id", "targetId"])
        else {
            continue;
        };
        let Some(source_id) = resolve_activity_reference(&source, node_ids) else {
            diagnostics.push(activity_diagnostic(
                "analysis.dynamic.activity_execution.unresolved_flow_source",
                format!(
                    "activity flow `{}` references unknown source `{source}`",
                    element.element_id
                ),
                binding,
            ));
            continue;
        };
        let Some(target_id) = resolve_activity_reference(&target, node_ids) else {
            diagnostics.push(activity_diagnostic(
                "analysis.dynamic.activity_execution.unresolved_flow_target",
                format!(
                    "activity flow `{}` references unknown target `{target}`",
                    element.element_id
                ),
                binding,
            ));
            continue;
        };
        flows.push(ActivityFlow {
            id: element.element_id.clone(),
            source: source_id,
            target: target_id,
        });
    }
    flows.sort_by(|left, right| left.id.cmp(&right.id));
    flows
}

fn flow_touches_nodes(element: &Element, node_ids: &BTreeSet<String>) -> bool {
    let Some(source) = string_property_any(element, &["source", "source_id", "sourceId"]) else {
        return false;
    };
    let Some(target) = string_property_any(element, &["target", "target_id", "targetId"]) else {
        return false;
    };
    resolve_activity_reference(&source, node_ids).is_some()
        || resolve_activity_reference(&target, node_ids).is_some()
}

fn resolve_activity_reference(reference: &str, node_ids: &BTreeSet<String>) -> Option<String> {
    if node_ids.contains(reference) {
        return Some(reference.to_string());
    }
    let normalized = reference.replace("::", ".");
    if node_ids.contains(&normalized) {
        return Some(normalized);
    }
    let first_segment = normalized.split('.').next().unwrap_or(reference);
    let last_segment = normalized.rsplit('.').next().unwrap_or(reference);
    node_ids
        .iter()
        .find(|node_id| {
            node_id.ends_with(&format!(".{reference}"))
                || node_id.ends_with(&format!(".{normalized}"))
                || node_id.ends_with(&format!(".{first_segment}"))
                || node_id.ends_with(&format!(".{last_segment}"))
        })
        .cloned()
}

fn activity_node_payload(element: &Element) -> Value {
    serde_json::json!({
        "elementId": element.element_id.clone(),
        "kind": element.kind.clone(),
        "label": element_label(element),
    })
}

fn is_activity_execution_node(element: &Element) -> bool {
    let kind = canonical_kind(&element.kind);
    (kind.contains("actionusage")
        || kind.contains("actiondefinition")
        || kind.contains("performactionusage")
        || kind.contains("forknode")
        || kind.contains("joinnode")
        || kind.contains("decisionnode")
        || kind.contains("mergenode"))
        && !is_activity_flow_element(element)
}

fn is_activity_flow_element(element: &Element) -> bool {
    let kind = canonical_kind(&element.kind);
    kind.contains("succession") || (kind.contains("flow") && has_source_and_target(element))
}

fn has_source_and_target(element: &Element) -> bool {
    string_property_any(element, &["source", "source_id", "sourceId"]).is_some()
        && string_property_any(element, &["target", "target_id", "targetId"]).is_some()
}

fn activity_owner_id(element: &Element) -> Option<String> {
    string_property_any(
        element,
        &[
            "owner",
            "owning_type",
            "owning_definition",
            "owning_namespace",
            "owningNamespace",
        ],
    )
}

fn activity_diagnostic(
    code: impl Into<String>,
    message: impl Into<String>,
    binding: &AnalysisDynamicBehaviorBinding,
) -> SemanticDiagnostic {
    SemanticDiagnostic::new(
        DiagnosticKind::Execution,
        SemanticDiagnosticSeverity::Warning,
        code,
        message,
    )
    .with_subject(semantic_ref_from_analysis_ref(&binding.behavior).element_id)
}

fn capability_run_status_label(status: CapabilityRunStatus) -> &'static str {
    match status {
        CapabilityRunStatus::Passed => "passed",
        CapabilityRunStatus::Failed => "failed",
        CapabilityRunStatus::Inconclusive => "inconclusive",
        CapabilityRunStatus::Partial => "partial",
        CapabilityRunStatus::NotApplicable => "not_applicable",
        CapabilityRunStatus::Error => "error",
    }
}

fn activity_execution_state(status: CapabilityRunStatus) -> &'static str {
    match status {
        CapabilityRunStatus::Passed => "completed",
        CapabilityRunStatus::Partial => "partial",
        CapabilityRunStatus::Failed | CapabilityRunStatus::Error => "failed",
        CapabilityRunStatus::Inconclusive => "inconclusive",
        CapabilityRunStatus::NotApplicable => "not_applicable",
    }
}

fn string_property_any(element: &Element, keys: &[&str]) -> Option<String> {
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

fn element_label(element: &Element) -> String {
    string_property_any(element, &["declared_name", "name"])
        .unwrap_or_else(|| element.element_id.clone())
}

fn canonical_kind(kind: &str) -> String {
    kind.replace([':', '.', ' ', '_'], "").to_ascii_lowercase()
}

fn dedup_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn semantic_ref_from_analysis_ref(
    reference: &mercurio_core::analysis::AnalysisElementRef,
) -> SemanticElementRef {
    reference.clone()
}

fn analysis_ref_payload(reference: &mercurio_core::analysis::AnalysisElementRef) -> Value {
    serde_json::json!({
        "elementId": reference.element_id.clone(),
        "kind": reference.kind.clone().unwrap_or_else(|| "Element".to_string()),
        "label": reference.label.clone(),
    })
}

fn composite_analysis_report(
    run_id: &str,
    spec: &AnalysisSpec,
    reports: Vec<CapabilityRunReport>,
) -> CapabilityRunReport {
    let status = aggregate_capability_status(reports.iter().map(|report| report.status));
    let mut insights = Vec::new();
    let mut artifacts = Vec::new();
    let mut evidence_nodes = Vec::new();
    let mut evidence_edges = Vec::new();
    let mut diagnostics = Vec::new();
    let mut limitations = Vec::new();

    for report in reports {
        insights.extend(report.insights);
        artifacts.extend(report.artifacts);
        evidence_nodes.extend(report.evidence.nodes);
        evidence_edges.extend(report.evidence.edges);
        diagnostics.extend(report.diagnostics);
        limitations.extend(report.limitations);
    }

    CapabilityRunReport {
        run_id: run_id.to_string(),
        capability_id: SYSML_ANALYSIS_CASE_CAPABILITY_ID.to_string(),
        capability_version: Some(SYSML_SIMULATION_CAPABILITY_VERSION.to_string()),
        status,
        target: CapabilityTarget::Element {
            element_id: spec.case_ref.element_id.clone(),
        },
        insights,
        artifacts,
        evidence: EvidenceGraph {
            nodes: evidence_nodes,
            edges: evidence_edges,
        },
        diagnostics,
        limitations,
    }
}

fn aggregate_capability_status(
    statuses: impl IntoIterator<Item = CapabilityRunStatus>,
) -> CapabilityRunStatus {
    let statuses = statuses.into_iter().collect::<Vec<_>>();
    if statuses
        .iter()
        .any(|status| *status == CapabilityRunStatus::Error)
    {
        CapabilityRunStatus::Error
    } else if statuses
        .iter()
        .any(|status| *status == CapabilityRunStatus::Failed)
    {
        CapabilityRunStatus::Failed
    } else if statuses
        .iter()
        .any(|status| *status == CapabilityRunStatus::Partial)
    {
        CapabilityRunStatus::Partial
    } else if statuses
        .iter()
        .any(|status| *status == CapabilityRunStatus::Inconclusive)
    {
        CapabilityRunStatus::Inconclusive
    } else if statuses
        .iter()
        .all(|status| *status == CapabilityRunStatus::Passed)
    {
        CapabilityRunStatus::Passed
    } else if statuses
        .iter()
        .any(|status| *status == CapabilityRunStatus::Passed)
    {
        CapabilityRunStatus::Partial
    } else {
        CapabilityRunStatus::NotApplicable
    }
}

pub fn simulation_trace_report(
    run_id: &str,
    analysis_case_id: &str,
    trace: SimulationTrace,
) -> Result<CapabilityRunReport, SimulationError> {
    let reported_analysis_case_id = if trace.scenario_id.is_empty() {
        analysis_case_id
    } else {
        trace.scenario_id.as_str()
    };
    let payload = serde_json::to_value(&trace)?;
    let payload_bytes = serde_json::to_vec(&payload)?;
    let digest = stable_digest([("simulation-trace".as_bytes(), payload_bytes.as_slice())]);
    let analysis_case_ref = SemanticElementRef::new(reported_analysis_case_id);
    let subject_ref = SemanticElementRef::new(trace.subject_id.clone());
    let mut element_refs = vec![analysis_case_ref.clone()];
    if !subject_ref.element_id.is_empty() && subject_ref.element_id != analysis_case_ref.element_id
    {
        element_refs.push(subject_ref);
    }

    let artifact_id = format!(
        "artifact.{}.simulation_trace.{}",
        sanitize_identifier(run_id),
        sanitize_identifier(reported_analysis_case_id)
    );
    let evidence_id = format!(
        "evidence.{}.simulation_analysis.{}",
        sanitize_identifier(run_id),
        sanitize_identifier(reported_analysis_case_id)
    );
    let artifact = SemanticArtifact {
        id: artifact_id.clone(),
        kind: SIMULATION_TRACE_ARTIFACT_KIND.to_string(),
        schema: SIMULATION_TRACE_SCHEMA.to_string(),
        digest,
        element_refs: element_refs.clone(),
        payload,
    };

    Ok(CapabilityRunReport {
        run_id: run_id.to_string(),
        capability_id: SYSML_DYNAMIC_BEHAVIOR_CAPABILITY_ID.to_string(),
        capability_version: Some(SYSML_SIMULATION_CAPABILITY_VERSION.to_string()),
        status: capability_status_from_simulation_status(trace.status),
        target: CapabilityTarget::Element {
            element_id: reported_analysis_case_id.to_string(),
        },
        insights: Vec::new(),
        artifacts: vec![artifact],
        evidence: EvidenceGraph {
            nodes: vec![
                EvidenceNode {
                    id: evidence_id.clone(),
                    kind: EvidenceNodeKind::AnalysisRun,
                    label: format!("Simulation analysis case {reported_analysis_case_id}"),
                    element_refs,
                    source_spans: Vec::new(),
                    properties: BTreeMap::from([
                        (
                            "analysis_case_id".to_string(),
                            Value::String(reported_analysis_case_id.to_string()),
                        ),
                        (
                            "scenario_id".to_string(),
                            Value::String(trace.scenario_id.clone()),
                        ),
                    ]),
                },
                EvidenceNode {
                    id: artifact_id.clone(),
                    kind: EvidenceNodeKind::Artifact,
                    label: "Simulation trace".to_string(),
                    element_refs: Vec::new(),
                    source_spans: Vec::new(),
                    properties: BTreeMap::new(),
                },
            ],
            edges: vec![EvidenceEdge {
                source_id: artifact_id,
                target_id: evidence_id,
                relation: EvidenceRelation::ProducedBy,
            }],
        },
        diagnostics: Vec::new(),
        limitations: Vec::new(),
    })
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
    let clock_config = scenario
        .clock_config
        .clone()
        .unwrap_or_else(|| SimulationClockConfig {
            max_time_s: scenario.max_steps.max(1) as f64 * scenario.step_duration_s.max(0.0),
            fixed_step_s: scenario.step_duration_s,
            sample_interval_s: scenario.step_duration_s,
            change_loop_limit: CHANGE_LOOP_LIMIT,
        });
    run_concurrent_simulation_model(&model, scenario.clone(), clock_config)
        .map_err(|error| SimulationError::InvalidProfile(error.to_string()))
}

fn capability_status_from_simulation_status(status: SimulationStatus) -> CapabilityRunStatus {
    match status {
        SimulationStatus::Completed => CapabilityRunStatus::Passed,
        SimulationStatus::Blocked => CapabilityRunStatus::Partial,
        SimulationStatus::Failed => CapabilityRunStatus::Failed,
    }
}

fn sanitize_identifier(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        sanitized.push_str("unnamed");
    }
    sanitized
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

    #[test]
    fn trace_to_view_overlay_projects_states_events_and_values() {
        let trace = SimulationTrace {
            scenario_id: "scenario.demo".to_string(),
            subject_id: "part.controller".to_string(),
            channels: Vec::new(),
            timeline: vec![SimTraceEntry {
                t: 2.0,
                states: BTreeMap::from([(
                    "part.controller".to_string(),
                    vec!["state.Controller.Running".to_string()],
                )]),
                values: BTreeMap::from([(
                    (
                        "part.controller".to_string(),
                        "feature.Controller.temperature".to_string(),
                    ),
                    json!(42),
                )]),
                events: vec![SimTraceEvent {
                    kind: "transition".to_string(),
                    subject_id: Some("part.controller".to_string()),
                    transition_id: Some("transition.Controller.ready".to_string()),
                    trigger: Some("ready".to_string()),
                    reason: None,
                }],
            }],
            status: SimulationStatus::Completed,
            requirements: Vec::new(),
            objectives: Vec::new(),
        };

        let overlay = trace_to_view_overlay(&trace);

        assert_eq!(overlay.frames.len(), 1);
        let frame = &overlay.frames[0];
        assert_eq!(frame.index, 0);
        assert_eq!(frame.time_s, Some(2.0));
        assert!(frame.node_marks.iter().any(|mark| {
            mark.element == "state.Controller.Running"
                && mark.kind == "active_state"
                && mark.label.as_deref() == Some("active")
        }));
        assert!(frame.edge_marks.iter().any(|mark| {
            mark.element == "transition.Controller.ready"
                && mark.kind == "visited_transition"
                && mark.properties["trigger"] == json!("ready")
        }));
        assert!(frame.node_values.iter().any(|value| {
            value.element == "part.controller"
                && value.key == "feature.Controller.temperature"
                && value.label.as_deref() == Some("temperature")
                && value.value == json!(42)
        }));
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
                clock_config: None,
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

        let trace = run_concurrent_simulation(&runtime, scenario).unwrap();
        assert!(trace.timeline.iter().any(|entry| {
            entry
                .states
                .get("individual.printer")
                .is_some_and(|states| states == &vec!["state.Printer.printing".to_string()])
        }));

        let report = run_analysis_case(&runtime, "analysis.PrintSequence", "test.run").unwrap();
        assert_eq!(report.capability_id, SYSML_DYNAMIC_BEHAVIOR_CAPABILITY_ID);
        assert_eq!(report.status, CapabilityRunStatus::Passed);
        assert_eq!(report.artifacts.len(), 1);
        assert_eq!(report.artifacts[0].kind, SIMULATION_TRACE_ARTIFACT_KIND);
        assert_eq!(report.artifacts[0].schema, SIMULATION_TRACE_SCHEMA);
        assert_eq!(
            report.artifacts[0].payload["scenario_id"],
            json!("analysis.PrintSequence")
        );
    }

    #[test]
    fn analysis_case_runs_constraint_summary_when_no_state_machine_is_bound() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element(
                    "part.Vehicle",
                    "PartUsage",
                    [("declared_name", json!("vehicle"))],
                ),
                element(
                    "constraint.totalMass",
                    "ConstraintUsage",
                    [("expression", json!("totalMass == dryMass + fuelMass"))],
                ),
                element(
                    "req.maxMass",
                    "RequirementUsage",
                    [("expression", json!("totalMass <= maxMass"))],
                ),
                element(
                    "analysis.MassCompliance",
                    "SysML::Systems::AnalysisCaseDefinition",
                    [
                        ("declared_name", json!("MassCompliance")),
                        ("subjects", json!([{ "subject": "part.Vehicle" }])),
                        ("constraints", json!(["constraint.totalMass"])),
                        ("requirements", json!(["req.maxMass"])),
                        (
                            "initial_values",
                            json!({
                                "scenario|dryMass": 90.0,
                                "scenario|fuelMass": 30.0,
                                "scenario|maxMass": 125.0
                            }),
                        ),
                    ],
                ),
            ],
        })
        .unwrap();

        let report = run_analysis_case(&runtime, "analysis.MassCompliance", "test.mass").unwrap();

        assert_eq!(
            report.capability_id,
            SYSML_CONSTRAINT_ANALYSIS_CAPABILITY_ID
        );
        assert_eq!(report.status, CapabilityRunStatus::Passed);
        assert_eq!(report.artifacts.len(), 1);
        assert_eq!(report.artifacts[0].kind, "constraint_analysis_summary");
        assert_eq!(
            report.artifacts[0].payload["result"]["requirements"][0]["status"],
            "satisfied"
        );
        assert_eq!(
            report.artifacts[0].payload["result"]["variables"]
                .as_array()
                .unwrap()
                .iter()
                .find(|variable| variable["id"] == "totalMass")
                .and_then(|variable| variable["value"].as_f64()),
            Some(120.0)
        );
    }

    #[test]
    fn analysis_case_executes_sequential_activity_summary() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element(
                    "type.Printer",
                    "PartDefinition",
                    [("declared_name", json!("Printer"))],
                ),
                element(
                    "part.Printer",
                    "PartUsage",
                    [
                        ("declared_name", json!("printer")),
                        ("type", json!("type.Printer")),
                    ],
                ),
                element(
                    "action.Printer.warmup",
                    "ActionUsage",
                    [
                        ("declared_name", json!("warmup")),
                        ("owner", json!("type.Printer")),
                    ],
                ),
                element(
                    "action.Printer.warmup.home",
                    "ActionUsage",
                    [
                        ("declared_name", json!("home")),
                        ("owner", json!("action.Printer.warmup")),
                    ],
                ),
                element(
                    "action.Printer.warmup.heat",
                    "ActionUsage",
                    [
                        ("declared_name", json!("heatBed")),
                        ("owner", json!("action.Printer.warmup")),
                    ],
                ),
                element(
                    "flow.Printer.warmup.home_to_heat",
                    "SuccessionUsage",
                    [
                        ("owner", json!("action.Printer.warmup")),
                        ("source", json!("action.Printer.warmup.home")),
                        ("target", json!("action.Printer.warmup.heat")),
                    ],
                ),
                element(
                    "analysis.Warmup",
                    "SysML::Systems::AnalysisCaseDefinition",
                    [
                        ("declared_name", json!("Warmup")),
                        ("subjects", json!([{ "subject": "part.Printer" }])),
                    ],
                ),
            ],
        })
        .unwrap();

        let report = run_analysis_case(&runtime, "Warmup", "test.activity").unwrap();

        assert_eq!(report.capability_id, SYSML_ACTIVITY_EXECUTION_CAPABILITY_ID);
        assert_eq!(report.status, CapabilityRunStatus::Passed);
        assert_eq!(report.artifacts.len(), 1);
        assert_eq!(
            report.artifacts[0].kind,
            ACTIVITY_EXECUTION_SUMMARY_ARTIFACT_KIND
        );
        assert_eq!(
            report.artifacts[0].payload["bindings"][0]["behavior"]["elementId"],
            "action.Printer.warmup"
        );
        assert_eq!(
            report.artifacts[0].payload["bindings"][0]["executionState"],
            "completed"
        );
        assert_eq!(
            report.artifacts[0].payload["bindings"][0]["steps"][0]["nodes"][0]["elementId"],
            "action.Printer.warmup.home"
        );
        assert_eq!(
            report.artifacts[0].payload["bindings"][0]["steps"][1]["nodes"][0]["elementId"],
            "action.Printer.warmup.heat"
        );
        assert!(report.diagnostics.is_empty());
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
                clock_config: None,
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
                clock_config: None,
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
                clock_config: None,
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
    fn constraint_rule_derives_rate_used_by_state_do_behavior() {
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
                element(
                    "constraint.Bed.thermalLoad",
                    "ConstraintUsage",
                    [
                        ("declared_name", json!("ThermalLoad")),
                        (
                            "expression_ir",
                            json!({
                                "kind": "binary",
                                "op": "equal",
                                "left": { "kind": "path", "segments": ["heatRate"] },
                                "right": {
                                    "kind": "binary",
                                    "op": "divide",
                                    "left": { "kind": "path", "segments": ["power"] },
                                    "right": {
                                        "kind": "binary",
                                        "op": "multiply",
                                        "left": { "kind": "path", "segments": ["mass"] },
                                        "right": { "kind": "path", "segments": ["heatCap"] }
                                    }
                                }
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
                id: "scenario.thermal_constraint".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "individual.bed".to_string(),
                    machine_id: "BedMachine".to_string(),
                    initial_state_id: None,
                    events: Vec::new(),
                }],
                max_steps: 2,
                step_duration_s: 1.0,
                clock_config: Some(SimulationClockConfig {
                    max_time_s: 2.0,
                    fixed_step_s: 1.0,
                    sample_interval_s: 1.0,
                    change_loop_limit: CHANGE_LOOP_LIMIT,
                }),
                initial_values: BTreeMap::from([
                    (
                        ("individual.bed".to_string(), "power".to_string()),
                        json!(1000.0),
                    ),
                    (
                        ("individual.bed".to_string(), "mass".to_string()),
                        json!(5.0),
                    ),
                    (
                        ("individual.bed".to_string(), "heatCap".to_string()),
                        json!(500.0),
                    ),
                    (
                        ("individual.bed".to_string(), "temperature".to_string()),
                        json!(20.0),
                    ),
                ]),
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
                .values
                .get(&("individual.bed".to_string(), "heatRate".to_string())),
            Some(&json!(0.4))
        );
        let final_temperature = trace
            .timeline
            .last()
            .unwrap()
            .values
            .get(&("individual.bed".to_string(), "temperature".to_string()))
            .and_then(Value::as_f64)
            .unwrap();
        assert!((final_temperature - 20.8).abs() <= 1e-9);
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
                clock_config: None,
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
    fn state_do_lookup_table_interpolates_continuous_value() {
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
                                "kind": "lookup_table",
                                "tables": [
                                    {
                                        "feature": "temperature",
                                        "samples": [
                                            { "time": 0.0, "value": 20.0 },
                                            { "time": 5.0, "value": 60.0 },
                                            { "time": 10.0, "value": 100.0 }
                                        ]
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
                id: "scenario.lookup_curve".to_string(),
                subjects: vec![ConcurrentSubjectScenario {
                    subject_id: "individual.bed".to_string(),
                    machine_id: "BedMachine".to_string(),
                    initial_state_id: None,
                    events: Vec::new(),
                }],
                max_steps: 2,
                step_duration_s: 2.5,
                clock_config: Some(SimulationClockConfig {
                    max_time_s: 5.0,
                    fixed_step_s: 2.5,
                    sample_interval_s: 2.5,
                    change_loop_limit: CHANGE_LOOP_LIMIT,
                }),
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
                .values
                .get(&("individual.bed".to_string(), "temperature".to_string())),
            Some(&json!(20.0))
        );
        let mid_temperature = trace
            .timeline
            .iter()
            .find(|entry| (entry.t - 2.5).abs() <= f64::EPSILON)
            .unwrap()
            .values
            .get(&("individual.bed".to_string(), "temperature".to_string()))
            .and_then(Value::as_f64)
            .unwrap();
        assert!((mid_temperature - 40.0).abs() <= f64::EPSILON);
        let final_temperature = trace
            .timeline
            .last()
            .unwrap()
            .values
            .get(&("individual.bed".to_string(), "temperature".to_string()))
            .and_then(Value::as_f64)
            .unwrap();
        assert!((final_temperature - 60.0).abs() <= f64::EPSILON);
        assert!(trace.channels.iter().any(|channel| {
            channel.id == "individual.bed.temperature"
                && channel.source == SimTraceChannelSource::LookupTable
        }));
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
                clock_config: None,
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
                    clock_config: None,
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
                clock_config: None,
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
                clock_config: None,
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
                clock_config: None,
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
                clock_config: None,
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
                clock_config: None,
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
                clock_config: None,
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
                clock_config: None,
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
                clock_config: None,
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
            layer: if kind.contains("AnalysisCaseDefinition") {
                2
            } else {
                0
            },
            properties: properties
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        }
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
