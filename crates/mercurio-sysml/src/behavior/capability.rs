use std::collections::BTreeMap;

use crate::behavior::{
    StateMachineExecutionStatus, StateMachineScenario, StateMachineScenarioEvent,
    StateMachineValidationSeverity, project_state_machines_from_graph,
};
use mercurio_core::{
    CapabilityCostClass, CapabilityDescriptor, CapabilityError, CapabilityKind, CapabilityMaturity,
    CapabilityReadinessReport, CapabilityReadinessStatus, CapabilityRegistry, CapabilityRunReport,
    CapabilityRunRequest, CapabilityRunStatus, CapabilityTarget, EvidenceGraph, EvidenceNode,
    EvidenceNodeKind, InsightConfidence, InsightKind, InsightPolarity, InsightScope,
    InsightSeverity, SemanticArtifact, SemanticCapability, SemanticDiagnostic,
    SemanticDiagnosticSeverity, SemanticWorkspaceSnapshot, stable_digest,
};
use serde_json::{Value, json};

#[derive(Debug, Clone, Default)]
pub struct SysmlDynamicBehaviorCapability;

pub fn register_sysml_behavior_capability(
    registry: &mut CapabilityRegistry,
) -> Result<(), CapabilityError> {
    registry.register(SysmlDynamicBehaviorCapability)
}

impl SemanticCapability for SysmlDynamicBehaviorCapability {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            id: "sysml.behavior.dynamic".to_string(),
            name: "SysML Dynamic Behavior".to_string(),
            kind: CapabilityKind::DynamicBehavior,
            profile_id: Some("sysml".to_string()),
            target_kinds: vec![
                "SysML::Behavior::StateUsage".to_string(),
                "SysML::Behavior::StateActionUsage".to_string(),
                "SysML::Behavior::TransitionUsage".to_string(),
                "SysML::Behavior::AcceptActionUsage".to_string(),
            ],
            relationship_kinds: vec![
                "source".to_string(),
                "target".to_string(),
                "trigger".to_string(),
            ],
            input_artifact_kinds: Vec::new(),
            produced_insight_kinds: vec![
                InsightKind::BehaviorObserved,
                InsightKind::ScenarioFailure,
                InsightKind::ReachabilityFinding,
                InsightKind::RuntimeMetric,
            ],
            produced_artifact_kinds: vec!["state_machine_execution_report".to_string()],
            deterministic: true,
            cost_class: CapabilityCostClass::Moderate,
            maturity: CapabilityMaturity::Prototype,
        }
    }

    fn readiness(
        &self,
        workspace: &SemanticWorkspaceSnapshot,
        target: &CapabilityTarget,
    ) -> CapabilityReadinessReport {
        let machines = project_state_machines_from_graph(&workspace.graph);
        if machines.is_empty() {
            return behavior_readiness(
                target.clone(),
                CapabilityReadinessStatus::NotApplicable,
                "workspace has no SysML state machine candidates",
            );
        }
        if let CapabilityTarget::Element { element_id } = target
            && !machines
                .iter()
                .any(|machine| machine.id == *element_id || machine.label == *element_id)
        {
            return behavior_readiness(
                target.clone(),
                CapabilityReadinessStatus::Blocked,
                format!("target `{element_id}` is not a projected SysML state machine"),
            );
        }

        let has_errors = machines.iter().any(|machine| {
            machine
                .validate_structure()
                .iter()
                .any(|finding| finding.severity == StateMachineValidationSeverity::Error)
        });
        if has_errors {
            let mut report = behavior_readiness(
                target.clone(),
                CapabilityReadinessStatus::Partial,
                "SysML state machine candidates exist but some have structural errors",
            );
            report
                .limitations
                .push("capability can still report readiness and diagnostics".to_string());
            report
        } else {
            behavior_readiness(
                target.clone(),
                CapabilityReadinessStatus::Ready,
                "SysML state machine candidates are available for dynamic behavior analysis",
            )
        }
    }

    fn run(
        &self,
        workspace: &SemanticWorkspaceSnapshot,
        request: CapabilityRunRequest,
    ) -> Result<CapabilityRunReport, CapabilityError> {
        let mut machines = project_state_machines_from_graph(&workspace.graph);
        if machines.is_empty() {
            return Ok(CapabilityRunReport {
                run_id: request.run_id,
                capability_id: request.capability_id,
                status: CapabilityRunStatus::NotApplicable,
                target: request.target,
                insights: Vec::new(),
                artifacts: Vec::new(),
                evidence: EvidenceGraph::default(),
                diagnostics: Vec::new(),
                limitations: vec!["workspace has no SysML state machine candidates".to_string()],
            });
        }
        machines.sort_by(|left, right| left.id.cmp(&right.id));

        let machine_id = parameter_string(&request, "machineId")
            .or_else(|| parameter_string(&request, "machine_id"))
            .or_else(|| match &request.target {
                CapabilityTarget::Element { element_id } => Some(element_id.clone()),
                _ => None,
            });
        let machine = machine_id
            .as_deref()
            .and_then(|id| {
                machines
                    .iter()
                    .find(|machine| machine.id == id || machine.label == id)
            })
            .or_else(|| machines.first())
            .ok_or_else(|| CapabilityError::Execution("no state machine candidates".to_string()))?;
        let scenario = StateMachineScenario {
            id: parameter_string(&request, "scenarioId")
                .or_else(|| parameter_string(&request, "scenario_id"))
                .unwrap_or_else(|| "default".to_string()),
            initial_state_id: parameter_string(&request, "initialStateId")
                .or_else(|| parameter_string(&request, "initial_state_id")),
            events: scenario_events(&request)?,
            max_steps: parameter_usize(&request, "maxSteps", 64).max(1),
        };
        let execution = machine.execute_scenario(&scenario);
        let diagnostics = execution
            .diagnostics
            .iter()
            .map(|finding| SemanticDiagnostic {
                code: format!("state_machine.{}", finding.code),
                severity: match finding.severity {
                    StateMachineValidationSeverity::Warning => SemanticDiagnosticSeverity::Warning,
                    StateMachineValidationSeverity::Error => SemanticDiagnosticSeverity::Error,
                },
                message: finding.message.clone(),
                element: finding
                    .state_id
                    .as_ref()
                    .or(finding.transition_id.as_ref())
                    .map(|element_id| workspace.element_ref(element_id)),
                source_spans: finding
                    .state_id
                    .as_ref()
                    .or(finding.transition_id.as_ref())
                    .map(|element_id| workspace.source_spans(element_id))
                    .unwrap_or_default(),
            })
            .collect::<Vec<_>>();
        let subject = workspace.element_ref(&machine.id);
        let evidence_id = format!("evidence.{}.behavior.{}", request.run_id, machine.id);
        let status = match execution.status {
            StateMachineExecutionStatus::Completed => CapabilityRunStatus::Passed,
            StateMachineExecutionStatus::Blocked => CapabilityRunStatus::Partial,
            StateMachineExecutionStatus::Failed => CapabilityRunStatus::Failed,
        };
        let insight_kind = match execution.status {
            StateMachineExecutionStatus::Completed => InsightKind::BehaviorObserved,
            StateMachineExecutionStatus::Blocked | StateMachineExecutionStatus::Failed => {
                InsightKind::ScenarioFailure
            }
        };
        let claim = match execution.status {
            StateMachineExecutionStatus::Completed => format!(
                "SysML state machine `{}` executed scenario `{}` with {} trace steps.",
                machine.id,
                scenario.id,
                execution.steps.len()
            ),
            StateMachineExecutionStatus::Blocked => format!(
                "SysML state machine `{}` blocked during scenario `{}`.",
                machine.id, scenario.id
            ),
            StateMachineExecutionStatus::Failed => {
                format!(
                    "SysML state machine `{}` failed structural simulation readiness.",
                    machine.id
                )
            }
        };
        let scenario_id = scenario.id.clone();
        let insight = mercurio_core::SemanticInsight {
            id: format!("insight.{}.behavior.{}", request.run_id, machine.id),
            kind: insight_kind,
            subject: subject.clone(),
            claim,
            polarity: match execution.status {
                StateMachineExecutionStatus::Completed => InsightPolarity::Supports,
                StateMachineExecutionStatus::Blocked | StateMachineExecutionStatus::Failed => {
                    InsightPolarity::Weakens
                }
            },
            severity: match execution.status {
                StateMachineExecutionStatus::Completed => InsightSeverity::Info,
                StateMachineExecutionStatus::Blocked => InsightSeverity::Warning,
                StateMachineExecutionStatus::Failed => InsightSeverity::Error,
            },
            confidence: InsightConfidence::High,
            scope: InsightScope::Scenario {
                scenario_id: scenario_id.clone(),
            },
            evidence_ids: vec![evidence_id.clone()],
            source_spans: workspace.source_spans(&machine.id),
            metrics: BTreeMap::from([
                ("state_count".to_string(), Value::from(machine.states.len())),
                (
                    "transition_count".to_string(),
                    Value::from(machine.transitions.len()),
                ),
                (
                    "trace_steps".to_string(),
                    Value::from(execution.steps.len()),
                ),
            ]),
            assumptions: vec![
                "dynamic behavior uses SysML state-machine projection rules".to_string(),
            ],
            limitations: Vec::new(),
        };
        let payload = json!({
            "machine": machine,
            "scenario": scenario,
            "execution": execution,
        });
        let artifact = SemanticArtifact {
            id: format!("artifact.{}.behavior.{}", request.run_id, machine.id),
            kind: "state_machine_execution_report".to_string(),
            schema: "mercurio.capability.sysml_state_machine_execution.v1".to_string(),
            digest: value_digest(&payload),
            element_refs: vec![subject.clone()],
            payload,
        };

        Ok(CapabilityRunReport {
            run_id: request.run_id.clone(),
            capability_id: request.capability_id,
            status,
            target: request.target,
            insights: vec![insight],
            artifacts: vec![artifact],
            evidence: EvidenceGraph {
                nodes: vec![EvidenceNode {
                    id: evidence_id,
                    kind: EvidenceNodeKind::AnalysisRun,
                    label: format!("SysML dynamic behavior run for {}", machine.id),
                    element_refs: vec![subject],
                    source_spans: workspace.source_spans(&machine.id),
                    properties: BTreeMap::from([(
                        "scenario_id".to_string(),
                        Value::String(scenario_id),
                    )]),
                }],
                edges: Vec::new(),
            },
            diagnostics,
            limitations: Vec::new(),
        })
    }
}

fn behavior_readiness(
    target: CapabilityTarget,
    status: CapabilityReadinessStatus,
    message: impl Into<String>,
) -> CapabilityReadinessReport {
    CapabilityReadinessReport {
        capability_id: "sysml.behavior.dynamic".to_string(),
        target,
        status,
        message: message.into(),
        required_inputs: Vec::new(),
        limitations: Vec::new(),
    }
}

fn scenario_events(
    request: &CapabilityRunRequest,
) -> Result<Vec<StateMachineScenarioEvent>, CapabilityError> {
    let Some(events) = request.parameters.get("events") else {
        return Ok(Vec::new());
    };
    let events = events
        .as_array()
        .ok_or_else(|| CapabilityError::InvalidRequest("`events` must be an array".to_string()))?;
    events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let trigger = event
                .get("trigger")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    CapabilityError::InvalidRequest(
                        "each event must define a string `trigger`".to_string(),
                    )
                })?;
            let id = event
                .get("id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("event.{}", index + 1));
            Ok(StateMachineScenarioEvent {
                id,
                trigger: trigger.to_string(),
            })
        })
        .collect()
}

fn parameter_string(request: &CapabilityRunRequest, key: &str) -> Option<String> {
    request
        .parameters
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn parameter_usize(request: &CapabilityRunRequest, key: &str, default: usize) -> usize {
    request
        .parameters
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(default)
}

fn value_digest(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    stable_digest([("semantic-artifact".as_bytes(), bytes.as_slice())])
}

#[cfg(test)]
mod tests {
    use super::*;
    use mercurio_core::{KirDocument, KirElement};

    #[test]
    fn behavior_capability_reports_not_applicable_without_state_machines() {
        let workspace = SemanticWorkspaceSnapshot::from_document_with_profile(
            KirDocument {
                metadata: BTreeMap::new(),
                elements: vec![KirElement {
                    id: "pkg.Demo".to_string(),
                    kind: "Model::Package".to_string(),
                    layer: 2,
                    properties: BTreeMap::new(),
                }],
            },
            Some("sysml".to_string()),
        )
        .unwrap();
        let capability = SysmlDynamicBehaviorCapability;
        let readiness = capability.readiness(&workspace, &CapabilityTarget::Workspace);

        assert_eq!(readiness.status, CapabilityReadinessStatus::NotApplicable);
        assert_eq!(readiness.capability_id, "sysml.behavior.dynamic");
    }

    #[test]
    fn behavior_capability_executes_projected_state_machine() {
        let workspace = SemanticWorkspaceSnapshot::from_document_with_profile(
            KirDocument {
                metadata: BTreeMap::new(),
                elements: vec![
                    KirElement {
                        id: "state.Controller.Off".to_string(),
                        kind: "SysML::Behavior::StateUsage".to_string(),
                        layer: 2,
                        properties: BTreeMap::from([
                            (
                                "owning_type".to_string(),
                                Value::String("Controller".to_string()),
                            ),
                            ("is_initial".to_string(), Value::Bool(true)),
                        ]),
                    },
                    KirElement {
                        id: "state.Controller.On".to_string(),
                        kind: "SysML::Behavior::StateUsage".to_string(),
                        layer: 2,
                        properties: BTreeMap::from([(
                            "owning_type".to_string(),
                            Value::String("Controller".to_string()),
                        )]),
                    },
                    KirElement {
                        id: "transition.Controller.start".to_string(),
                        kind: "SysML::Behavior::TransitionUsage".to_string(),
                        layer: 2,
                        properties: BTreeMap::from([
                            (
                                "owning_type".to_string(),
                                Value::String("Controller".to_string()),
                            ),
                            (
                                "source".to_string(),
                                Value::String("state.Controller.Off".to_string()),
                            ),
                            (
                                "target".to_string(),
                                Value::String("state.Controller.On".to_string()),
                            ),
                            ("trigger".to_string(), Value::String("start".to_string())),
                        ]),
                    },
                ],
            },
            Some("sysml".to_string()),
        )
        .unwrap();
        let capability = SysmlDynamicBehaviorCapability;

        let report = capability
            .run(
                &workspace,
                CapabilityRunRequest {
                    run_id: "run.behavior".to_string(),
                    capability_id: "sysml.behavior.dynamic".to_string(),
                    target: CapabilityTarget::Workspace,
                    parameters: BTreeMap::from([(
                        "events".to_string(),
                        json!([{ "trigger": "start" }]),
                    )]),
                    input_artifacts: Vec::new(),
                },
            )
            .unwrap();

        assert_eq!(report.status, CapabilityRunStatus::Passed);
        assert_eq!(report.capability_id, "sysml.behavior.dynamic");
        assert!(
            report
                .insights
                .iter()
                .any(|insight| insight.kind == InsightKind::BehaviorObserved)
        );
    }
}
