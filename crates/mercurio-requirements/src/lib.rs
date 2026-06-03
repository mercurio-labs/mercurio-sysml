//! Requirement-domain APIs.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use mercurio_core::{
    GoalCheckEvaluation, GoalEvaluation, GoalPolicy, Graph, KirDocument, SemanticGoalCheck,
    SemanticGoalExplanation, SemanticGoalProfile, SemanticGoalProfileKind, SemanticGoalSpec,
    SemanticReasoningContext,
};

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
