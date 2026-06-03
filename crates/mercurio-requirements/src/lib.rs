//! Requirement-domain APIs.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub use mercurio_core::{
    CapabilityCostClass, CapabilityDescriptor, CapabilityError, CapabilityKind, CapabilityMaturity,
    CapabilityReadinessReport, CapabilityReadinessStatus, CapabilityRegistry, CapabilityRunReport,
    CapabilityRunRequest, CapabilityRunStatus, CapabilityTarget, EvidenceGraph, EvidenceNode,
    EvidenceNodeKind, GoalCheckEvaluation, GoalEvaluation, GoalPolicy, Graph, InsightConfidence,
    InsightKind, InsightPolarity, InsightScope, InsightSeverity, KirDocument, SemanticArtifact,
    SemanticCapability, SemanticDiagnostic, SemanticElementRef, SemanticGoalCheck,
    SemanticGoalExplanation, SemanticGoalProfile, SemanticGoalProfileKind, SemanticGoalSpec,
    SemanticInsight, SemanticReasoningContext, SemanticWorkspaceSnapshot, SourceSpanRef,
};

#[derive(Debug, Clone, Default)]
pub struct RequirementAnalysisCapability;

pub fn register_requirement_analysis_capability(
    registry: &mut CapabilityRegistry,
) -> Result<(), CapabilityError> {
    registry.register(RequirementAnalysisCapability)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementTrace {
    pub relationship: String,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequirementTableViewDto {
    pub title: String,
    pub columns: Vec<RequirementTableColumnDto>,
    pub rows: Vec<RequirementTableRowDto>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequirementTableColumnDto {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequirementTableRowDto {
    pub id: String,
    pub name: Option<String>,
    pub text: Option<String>,
    pub owner: Option<String>,
    pub satisfied_by: Vec<String>,
    pub verified_by: Vec<String>,
    pub source: Option<RequirementSourceDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequirementSourceDto {
    pub file: Option<String>,
    pub start_line: Option<u64>,
    pub end_line: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementsError {
    message: String,
}

impl RequirementsError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RequirementsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RequirementsError {}

pub fn requirement_traces(
    document: &KirDocument,
    requirement_id: &str,
) -> Result<Vec<RequirementTrace>, RequirementsError> {
    let graph = Graph::from_document(document.clone())
        .map_err(|err| RequirementsError::new(format!("failed to build graph: {err}")))?;
    let requirement_node = graph
        .node_id(requirement_id)
        .ok_or_else(|| RequirementsError::new(format!("unknown requirement `{requirement_id}`")))?;
    let mut traces = graph
        .incoming_edges(requirement_node)
        .filter(|edge| is_requirement_trace_relation(&edge.relation))
        .filter_map(|edge| {
            Some(RequirementTrace {
                relationship: edge.relation.to_string(),
                source: graph.element_id(edge.source)?.to_string(),
                target: graph.element_id(edge.target)?.to_string(),
            })
        })
        .collect::<Vec<_>>();
    for element in &document.elements {
        for (property, value) in &element.properties {
            if is_requirement_trace_relation(property) && value_references(value, requirement_id) {
                traces.push(RequirementTrace {
                    relationship: property.clone(),
                    source: element.id.clone(),
                    target: requirement_id.to_string(),
                });
            }
        }
    }
    traces.sort_by(|left, right| {
        (&left.relationship, &left.source, &left.target).cmp(&(
            &right.relationship,
            &right.source,
            &right.target,
        ))
    });
    Ok(traces)
}

impl SemanticCapability for RequirementAnalysisCapability {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            id: "sysml.requirement.analysis".to_string(),
            name: "SysML Requirement Analysis".to_string(),
            kind: CapabilityKind::RequirementAnalysis,
            profile_id: Some("sysml".to_string()),
            target_kinds: vec![
                "SysML::Requirements::RequirementUsage".to_string(),
                "SysML::Requirements::RequirementDefinition".to_string(),
                "SysML::Verification::VerificationCaseUsage".to_string(),
                "SysML::Requirements::SatisfyRequirementUsage".to_string(),
                "SysML::Requirements::VerifyRequirementUsage".to_string(),
            ],
            relationship_kinds: vec![
                "satisfy".to_string(),
                "verify".to_string(),
                "refine".to_string(),
                "derive".to_string(),
                "trace".to_string(),
            ],
            input_artifact_kinds: Vec::new(),
            produced_insight_kinds: vec![
                InsightKind::CoverageGap,
                InsightKind::VerificationGap,
                InsightKind::SatisfactionEvidence,
                InsightKind::RequirementRisk,
                InsightKind::TraceCompleteness,
            ],
            produced_artifact_kinds: vec!["requirement_analysis_summary".to_string()],
            deterministic: true,
            cost_class: CapabilityCostClass::Cheap,
            maturity: CapabilityMaturity::Prototype,
        }
    }

    fn readiness(
        &self,
        workspace: &SemanticWorkspaceSnapshot,
        target: &CapabilityTarget,
    ) -> CapabilityReadinessReport {
        let table = requirements_table_view(&workspace.graph);
        let rows = rows_for_target(&table.rows, target);
        if table.rows.is_empty() {
            return requirement_readiness(
                target.clone(),
                CapabilityReadinessStatus::NotApplicable,
                "workspace has no SysML requirement elements",
            );
        }
        if let CapabilityTarget::Element { element_id } = target
            && workspace.graph.element_by_element_id(element_id).is_none()
        {
            return requirement_readiness(
                target.clone(),
                CapabilityReadinessStatus::Blocked,
                format!("target element `{element_id}` does not exist"),
            );
        }
        if rows.is_empty() {
            return requirement_readiness(
                target.clone(),
                CapabilityReadinessStatus::NotApplicable,
                "selected target has no requirement analysis scope",
            );
        }
        if table.warnings.is_empty() {
            requirement_readiness(
                target.clone(),
                CapabilityReadinessStatus::Ready,
                "requirement analysis can run over the selected scope",
            )
        } else {
            let mut report = requirement_readiness(
                target.clone(),
                CapabilityReadinessStatus::Partial,
                "requirement analysis can run with warnings",
            );
            report.limitations = table.warnings;
            report
        }
    }

    fn run(
        &self,
        workspace: &SemanticWorkspaceSnapshot,
        request: CapabilityRunRequest,
    ) -> Result<CapabilityRunReport, CapabilityError> {
        let readiness = self.readiness(workspace, &request.target);
        if matches!(
            readiness.status,
            CapabilityReadinessStatus::Blocked | CapabilityReadinessStatus::NotApplicable
        ) {
            return Ok(CapabilityRunReport {
                run_id: request.run_id,
                capability_id: request.capability_id,
                status: match readiness.status {
                    CapabilityReadinessStatus::NotApplicable => CapabilityRunStatus::NotApplicable,
                    _ => CapabilityRunStatus::Error,
                },
                target: request.target,
                insights: Vec::new(),
                artifacts: Vec::new(),
                evidence: EvidenceGraph::default(),
                diagnostics: Vec::<SemanticDiagnostic>::new(),
                limitations: vec![readiness.message],
            });
        }

        let table = requirements_table_view(&workspace.graph);
        let rows = rows_for_target(&table.rows, &request.target);
        let mut insights = Vec::new();
        let mut evidence = EvidenceGraph::default();

        for row in &rows {
            let subject = requirement_element_ref(row);
            if row.satisfied_by.is_empty() {
                let evidence_id = format!("evidence.requirement.coverage_gap.{}", row.id);
                evidence.nodes.push(requirement_evidence_node(
                    &evidence_id,
                    "Requirement has no satisfy trace",
                    row,
                ));
                insights.push(SemanticInsight {
                    id: format!("insight.requirement.coverage_gap.{}", row.id),
                    kind: InsightKind::CoverageGap,
                    subject: subject.clone(),
                    claim: format!("Requirement `{}` has no satisfy trace.", row.id),
                    polarity: InsightPolarity::Weakens,
                    severity: InsightSeverity::Warning,
                    confidence: InsightConfidence::High,
                    scope: InsightScope::Element {
                        element_id: row.id.clone(),
                    },
                    evidence_ids: vec![evidence_id],
                    source_spans: source_spans(row),
                    metrics: BTreeMap::new(),
                    assumptions: Vec::new(),
                    limitations: Vec::new(),
                });
            } else {
                let evidence_id = format!("evidence.requirement.satisfied.{}", row.id);
                evidence.nodes.push(requirement_evidence_node(
                    &evidence_id,
                    "Requirement has satisfy evidence",
                    row,
                ));
                insights.push(SemanticInsight {
                    id: format!("insight.requirement.satisfied.{}", row.id),
                    kind: InsightKind::SatisfactionEvidence,
                    subject: subject.clone(),
                    claim: format!(
                        "Requirement `{}` is satisfied by {} element(s).",
                        row.id,
                        row.satisfied_by.len()
                    ),
                    polarity: InsightPolarity::Supports,
                    severity: InsightSeverity::Info,
                    confidence: InsightConfidence::High,
                    scope: InsightScope::Element {
                        element_id: row.id.clone(),
                    },
                    evidence_ids: vec![evidence_id],
                    source_spans: source_spans(row),
                    metrics: BTreeMap::from([(
                        "satisfied_by_count".to_string(),
                        Value::from(row.satisfied_by.len()),
                    )]),
                    assumptions: Vec::new(),
                    limitations: Vec::new(),
                });
            }

            if row.verified_by.is_empty() {
                let evidence_id = format!("evidence.requirement.verification_gap.{}", row.id);
                evidence.nodes.push(requirement_evidence_node(
                    &evidence_id,
                    "Requirement has no verify trace",
                    row,
                ));
                insights.push(SemanticInsight {
                    id: format!("insight.requirement.verification_gap.{}", row.id),
                    kind: InsightKind::VerificationGap,
                    subject,
                    claim: format!("Requirement `{}` has no verify trace.", row.id),
                    polarity: InsightPolarity::Weakens,
                    severity: InsightSeverity::Warning,
                    confidence: InsightConfidence::High,
                    scope: InsightScope::Element {
                        element_id: row.id.clone(),
                    },
                    evidence_ids: vec![evidence_id],
                    source_spans: source_spans(row),
                    metrics: BTreeMap::new(),
                    assumptions: Vec::new(),
                    limitations: Vec::new(),
                });
            }
        }

        let total = rows.len();
        let satisfied = rows
            .iter()
            .filter(|row| !row.satisfied_by.is_empty())
            .count();
        let verified = rows
            .iter()
            .filter(|row| !row.verified_by.is_empty())
            .count();
        let satisfy_percent = percentage(satisfied, total);
        let verify_percent = percentage(verified, total);
        insights.push(SemanticInsight {
            id: format!("insight.requirement.trace_completeness.{}", request.run_id),
            kind: InsightKind::TraceCompleteness,
            subject: SemanticElementRef {
                element_id: "workspace".to_string(),
                qualified_name: None,
                label: Some("Requirement analysis scope".to_string()),
            },
            claim: format!(
                "Requirement scope has {satisfy_percent:.0}% satisfy coverage and {verify_percent:.0}% verify coverage."
            ),
            polarity: if satisfied == total && verified == total {
                InsightPolarity::Supports
            } else {
                InsightPolarity::Weakens
            },
            severity: if satisfied == total && verified == total {
                InsightSeverity::Info
            } else {
                InsightSeverity::Warning
            },
            confidence: InsightConfidence::High,
            scope: match &request.target {
                CapabilityTarget::Element { element_id } => InsightScope::Element {
                    element_id: element_id.clone(),
                },
                _ => InsightScope::Workspace,
            },
            evidence_ids: Vec::new(),
            source_spans: Vec::new(),
            metrics: BTreeMap::from([
                ("requirement_count".to_string(), Value::from(total)),
                ("satisfied_count".to_string(), Value::from(satisfied)),
                ("verified_count".to_string(), Value::from(verified)),
                ("satisfy_coverage_percent".to_string(), Value::from(satisfy_percent)),
                ("verify_coverage_percent".to_string(), Value::from(verify_percent)),
            ]),
            assumptions: vec![
                "requirement analysis uses SysML satisfy/verify trace interpretation".to_string(),
            ],
            limitations: Vec::new(),
        });

        let payload = json!({
            "schema": "mercurio.capability.sysml_requirement_analysis.v1",
            "target": request.target.clone(),
            "requirementCount": total,
            "satisfiedCount": satisfied,
            "verifiedCount": verified,
            "satisfyCoveragePercent": satisfy_percent,
            "verifyCoveragePercent": verify_percent,
            "rows": rows,
        });
        let artifact = SemanticArtifact {
            id: format!("artifact.{}.requirements", request.run_id),
            kind: "requirement_analysis_summary".to_string(),
            schema: "mercurio.capability.sysml_requirement_analysis.v1".to_string(),
            digest: artifact_digest(&payload),
            element_refs: rows.iter().map(requirement_element_ref).collect(),
            payload,
        };
        let has_gaps = insights.iter().any(|insight| {
            matches!(
                insight.kind,
                InsightKind::CoverageGap | InsightKind::VerificationGap
            )
        });

        Ok(CapabilityRunReport {
            run_id: request.run_id,
            capability_id: request.capability_id,
            status: if has_gaps {
                CapabilityRunStatus::Failed
            } else {
                CapabilityRunStatus::Passed
            },
            target: request.target,
            insights,
            artifacts: vec![artifact],
            evidence,
            diagnostics: Vec::new(),
            limitations: readiness.limitations,
        })
    }
}

pub fn requirements_table_view(graph: &Graph) -> RequirementTableViewDto {
    let derived = mercurio_core::materialize_core_indexes(graph, &[]).ok();
    let mut rows = graph
        .elements()
        .iter()
        .filter(|element| !is_library_requirement(element))
        .filter(|element| {
            derived
                .as_ref()
                .is_some_and(|derived| derived.requirements.contains(&element.element_id))
                || is_requirement(element)
        })
        .map(|requirement| RequirementTableRowDto {
            id: requirement.element_id.clone(),
            name: string_property(requirement, "declared_name")
                .or_else(|| string_property(requirement, "name")),
            text: string_property(requirement, "text")
                .or_else(|| string_property(requirement, "documentation")),
            owner: string_property(requirement, "owner"),
            satisfied_by: derived_sources(&derived, &requirement.element_id, "satisfies")
                .unwrap_or_else(|| related_sources(graph, requirement, &["satisfy", "satisfies"])),
            verified_by: derived_sources(&derived, &requirement.element_id, "verifies")
                .unwrap_or_else(|| related_sources(graph, requirement, &["verify", "verifies"])),
            source: source_for(requirement),
        })
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| left.id.cmp(&right.id));

    let mut warnings = Vec::new();
    if rows.is_empty() {
        warnings.push("No requirement elements were found in the semantic graph.".to_string());
    }

    RequirementTableViewDto {
        title: "Requirements".to_string(),
        columns: vec![
            column("id", "ID"),
            column("name", "Name"),
            column("text", "Text"),
            column("owner", "Owner"),
            column("satisfied_by", "Satisfied By"),
            column("verified_by", "Verified By"),
            column("source", "Source"),
        ],
        rows,
        warnings,
    }
}

fn requirement_readiness(
    target: CapabilityTarget,
    status: CapabilityReadinessStatus,
    message: impl Into<String>,
) -> CapabilityReadinessReport {
    CapabilityReadinessReport {
        capability_id: "sysml.requirement.analysis".to_string(),
        target,
        status,
        message: message.into(),
        required_inputs: Vec::new(),
        limitations: Vec::new(),
    }
}

fn rows_for_target(
    rows: &[RequirementTableRowDto],
    target: &CapabilityTarget,
) -> Vec<RequirementTableRowDto> {
    match target {
        CapabilityTarget::Workspace => rows.to_vec(),
        CapabilityTarget::Element { element_id } => rows
            .iter()
            .filter(|row| {
                row.id == *element_id
                    || row.satisfied_by.iter().any(|source| source == element_id)
                    || row.verified_by.iter().any(|source| source == element_id)
            })
            .cloned()
            .collect(),
        CapabilityTarget::Scope { scope_id } => rows
            .iter()
            .filter(|row| {
                row.owner.as_deref() == Some(scope_id.as_str()) || row.id.starts_with(scope_id)
            })
            .cloned()
            .collect(),
    }
}

fn requirement_element_ref(row: &RequirementTableRowDto) -> SemanticElementRef {
    SemanticElementRef {
        element_id: row.id.clone(),
        qualified_name: None,
        label: row.name.clone(),
    }
}

fn requirement_evidence_node(
    id: &str,
    label: impl Into<String>,
    row: &RequirementTableRowDto,
) -> EvidenceNode {
    EvidenceNode {
        id: id.to_string(),
        kind: EvidenceNodeKind::Fact,
        label: label.into(),
        element_refs: vec![requirement_element_ref(row)],
        source_spans: source_spans(row),
        properties: BTreeMap::from([
            (
                "satisfied_by_count".to_string(),
                Value::from(row.satisfied_by.len()),
            ),
            (
                "verified_by_count".to_string(),
                Value::from(row.verified_by.len()),
            ),
        ]),
    }
}

fn source_spans(row: &RequirementTableRowDto) -> Vec<SourceSpanRef> {
    let Some(source) = row.source.as_ref() else {
        return Vec::new();
    };
    let Some(file) = source.file.as_ref() else {
        return Vec::new();
    };
    vec![SourceSpanRef {
        file: file.clone(),
        start_line: source.start_line.unwrap_or(0) as u32,
        start_col: 0,
        end_line: source.end_line.unwrap_or(source.start_line.unwrap_or(0)) as u32,
        end_col: 0,
    }]
}

fn percentage(value: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (value as f64 / total as f64) * 100.0
    }
}

fn artifact_digest(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    mercurio_core::stable_digest([("requirement-analysis".as_bytes(), bytes.as_slice())])
}

pub fn explain_semantic_goal(goal: &SemanticGoalSpec) -> SemanticGoalExplanation {
    mercurio_core::explain_semantic_goal(goal)
}

pub fn evaluate_semantic_goal(
    context: &SemanticReasoningContext,
    goal: &SemanticGoalSpec,
) -> GoalEvaluation {
    mercurio_core::evaluate_semantic_goal(context, goal)
}

pub fn default_model_quality_profile() -> SemanticGoalProfile {
    mercurio_core::default_model_quality_profile()
}

fn value_references(value: &Value, target: &str) -> bool {
    match value {
        Value::String(value) => value == target,
        Value::Array(items) => items.iter().any(|item| value_references(item, target)),
        Value::Object(items) => items.values().any(|item| value_references(item, target)),
        _ => false,
    }
}

fn is_requirement_trace_relation(relation: &str) -> bool {
    let relation = relation.to_ascii_lowercase();
    relation.contains("satisfy")
        || relation.contains("satisfied")
        || relation.contains("verify")
        || relation.contains("verified")
        || relation.contains("refine")
        || relation.contains("refined")
}

fn derived_sources(
    derived: &Option<mercurio_core::DerivedIndexes>,
    requirement_id: &str,
    relation: &str,
) -> Option<Vec<String>> {
    let derived = derived.as_ref()?;
    let sources = match relation {
        "satisfies" => derived.satisfied_by.get(requirement_id),
        "verifies" => derived.verified_by.get(requirement_id),
        _ => None,
    }?;
    Some(sources.iter().cloned().collect())
}

fn column(key: &str, label: &str) -> RequirementTableColumnDto {
    RequirementTableColumnDto {
        key: key.to_string(),
        label: label.to_string(),
    }
}

fn is_requirement(element: &mercurio_core::graph::Element) -> bool {
    if is_requirement_relationship(element) {
        return false;
    }

    element.layer == 2
        && (element.kind.contains("Requirement")
            || element
                .properties
                .get("specializes")
                .and_then(Value::as_array)
                .is_some_and(|specializations| {
                    specializations
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|target| target.contains("Requirement"))
                }))
}

fn is_library_requirement(element: &mercurio_core::graph::Element) -> bool {
    element.element_id.contains("::")
}

fn is_requirement_relationship(element: &mercurio_core::graph::Element) -> bool {
    let kind = element.kind.to_ascii_lowercase();
    ["satisfy", "verify", "derive", "refine"]
        .iter()
        .any(|relationship| kind.contains(relationship))
}

fn related_sources(
    graph: &Graph,
    requirement: &mercurio_core::graph::Element,
    relations: &[&str],
) -> Vec<String> {
    let mut sources = Vec::new();

    for relation in relations {
        for edge in graph.incoming(requirement.id, relation) {
            if let Some(source) = graph.element_id(edge.source) {
                push_unique(&mut sources, source.to_string());
            }
        }
    }

    for element in graph.elements() {
        if !is_relationship_element(element, relations) {
            continue;
        }
        let Some(target) = string_property(element, "target") else {
            continue;
        };
        if target != requirement.element_id {
            continue;
        }
        if let Some(source) = string_property(element, "source") {
            push_unique(&mut sources, source);
        }
    }

    sources.sort();
    sources
}

fn is_relationship_element(element: &mercurio_core::graph::Element, relations: &[&str]) -> bool {
    let kind = element.kind.to_ascii_lowercase();
    relations
        .iter()
        .any(|relation| kind.contains(&relation.to_ascii_lowercase()))
}

fn source_for(element: &mercurio_core::graph::Element) -> Option<RequirementSourceDto> {
    let metadata = element.properties.get("metadata")?;
    let file = metadata
        .get("source_file")
        .and_then(Value::as_str)
        .map(str::to_string);
    let span = metadata.get("source_span");
    let start_line = span
        .and_then(|span| span.get("start_line"))
        .and_then(Value::as_u64);
    let end_line = span
        .and_then(|span| span.get("end_line"))
        .and_then(Value::as_u64);

    Some(RequirementSourceDto {
        file,
        start_line,
        end_line,
    })
}

fn string_property(element: &mercurio_core::graph::Element, key: &str) -> Option<String> {
    element
        .properties
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mercurio_core::KirElement;

    #[test]
    fn requirement_analysis_capability_reports_gaps_and_summary() {
        let document = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![KirElement {
                id: "req.safeStart".to_string(),
                kind: "SysML::Requirements::RequirementUsage".to_string(),
                layer: 2,
                properties: BTreeMap::from([
                    (
                        "declared_name".to_string(),
                        Value::String("Safe Start".to_string()),
                    ),
                    (
                        "text".to_string(),
                        Value::String("Vehicle shall start safely.".to_string()),
                    ),
                ]),
            }],
        };
        let workspace = SemanticWorkspaceSnapshot::from_document_with_profile(
            document,
            Some("sysml".to_string()),
        )
        .unwrap();
        let capability = RequirementAnalysisCapability;

        let report = capability
            .run(
                &workspace,
                CapabilityRunRequest {
                    run_id: "run.requirements".to_string(),
                    capability_id: "sysml.requirement.analysis".to_string(),
                    target: CapabilityTarget::Workspace,
                    parameters: BTreeMap::new(),
                    input_artifacts: Vec::new(),
                },
            )
            .unwrap();

        assert_eq!(report.status, CapabilityRunStatus::Failed);
        assert!(
            report
                .insights
                .iter()
                .any(|insight| insight.kind == InsightKind::CoverageGap)
        );
        assert!(
            report
                .insights
                .iter()
                .any(|insight| insight.kind == InsightKind::TraceCompleteness)
        );
    }

    #[test]
    fn requirement_analysis_capability_reports_satisfaction_evidence() {
        let document = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                KirElement {
                    id: "req.safeStart".to_string(),
                    kind: "SysML::Requirements::RequirementUsage".to_string(),
                    layer: 2,
                    properties: BTreeMap::new(),
                },
                KirElement {
                    id: "part.controller".to_string(),
                    kind: "SysML::Systems::PartUsage".to_string(),
                    layer: 2,
                    properties: BTreeMap::from([(
                        "satisfy".to_string(),
                        Value::Array(vec![Value::String("req.safeStart".to_string())]),
                    )]),
                },
            ],
        };
        let workspace = SemanticWorkspaceSnapshot::from_document_with_profile(
            document,
            Some("sysml".to_string()),
        )
        .unwrap();
        let capability = RequirementAnalysisCapability;

        let report = capability
            .run(
                &workspace,
                CapabilityRunRequest {
                    run_id: "run.requirements".to_string(),
                    capability_id: "sysml.requirement.analysis".to_string(),
                    target: CapabilityTarget::Workspace,
                    parameters: BTreeMap::new(),
                    input_artifacts: Vec::new(),
                },
            )
            .unwrap();

        assert!(
            report
                .insights
                .iter()
                .any(|insight| insight.kind == InsightKind::SatisfactionEvidence)
        );
    }
}
