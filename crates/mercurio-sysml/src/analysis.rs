use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use mercurio_foundation::analysis::{AnalysisCaseModel, AnalysisElementRef, AnalysisInventory};
use mercurio_foundation::graph::{Element, Graph};
use mercurio_foundation::runtime::Runtime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const SIMULATION_TRACE_SCHEMA: &str = "mercurio.simulation.trace.v1";
const CONSTRAINT_SUMMARY_SCHEMA: &str = "mercurio.capability.sysml_constraint_analysis.v1";
const ACTIVITY_EXECUTION_SUMMARY_SCHEMA: &str = "mercurio.analysis.activity_execution_summary.v1";
const VERDICT_SUMMARY_SCHEMA: &str = "mercurio.analysis.verdicts.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisSpec {
    pub case_ref: AnalysisElementRef,
    pub model_revision: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subjects: Vec<AnalysisElementRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<AnalysisElementRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<AnalysisElementRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objectives: Vec<AnalysisElementRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calculations: Vec<AnalysisElementRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<AnalysisElementRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<AnalysisElementRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verification_cases: Vec<AnalysisElementRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<AnalysisElementRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub concerns: Vec<AnalysisElementRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub techniques: Vec<AnalysisTechnique>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dynamic_behavior_bindings: Vec<AnalysisDynamicBehaviorBinding>,
    #[serde(default)]
    pub execution_context: AnalysisExecutionContext,
    pub execution_plan: AnalysisExecutionPlan,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_artifacts: Vec<AnalysisExpectedArtifact>,
    pub readiness: AnalysisReadinessStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readiness_diagnostics: Vec<AnalysisReadinessDiagnostic>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnalysisExecutionContext {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub initial_values: BTreeMap<String, BTreeMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock: Option<AnalysisClockConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub provider_bindings: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisDynamicBehaviorBinding {
    pub subject: AnalysisElementRef,
    pub behavior: AnalysisElementRef,
    pub kind: AnalysisDynamicBehaviorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisDynamicBehaviorKind {
    StateMachine,
    Activity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisClockConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_duration_s: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_time_s: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_step_s: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_interval_s: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_loop_limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisExecutionPlan {
    pub steps: Vec<AnalysisExecutionStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisExecutionStep {
    pub kind: AnalysisExecutionStepKind,
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub techniques: Vec<AnalysisTechnique>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<AnalysisElementRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisExecutionStepKind {
    ScopeSubjects,
    BindAssumptions,
    ResolveInputs,
    PreRunCalculation,
    PreRunConstraintEvaluation,
    DynamicBehavior,
    PostRunRequirementEvaluation,
    VerificationEvidence,
    ProduceViews,
    RecordResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisTechnique {
    Query,
    Calculation,
    ConstraintEvaluation,
    DynamicBehavior,
    Verification,
    TradeStudy,
    ExternalProvider,
    ViewGeneration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisExpectedArtifact {
    pub kind: String,
    pub schema: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisReadinessStatus {
    Ready,
    Partial,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisReadinessDiagnostic {
    pub severity: AnalysisReadinessSeverity,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisReadinessSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisSpecError {
    MissingAnalysisCase(String),
}

impl fmt::Display for AnalysisSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAnalysisCase(id) => write!(f, "missing analysis case: {id}"),
        }
    }
}

impl std::error::Error for AnalysisSpecError {}

pub fn list_analysis_specs(runtime: &Runtime) -> Vec<AnalysisSpec> {
    let graph = runtime.graph();
    let inventory = AnalysisInventory::from_graph(graph);
    graph
        .elements()
        .iter()
        .filter(|element| is_project_analysis_case(element))
        .map(|element| {
            let model = inventory
                .analysis_cases
                .iter()
                .find(|case| case.element.element_id == element.element_id);
            analysis_spec_from_case(runtime, element, model)
        })
        .collect()
}

pub fn project_analysis_spec(
    runtime: &Runtime,
    analysis_case_id: &str,
) -> Result<AnalysisSpec, AnalysisSpecError> {
    let graph = runtime.graph();
    let inventory = AnalysisInventory::from_graph(graph);
    let element = graph
        .elements()
        .iter()
        .find(|element| {
            is_project_analysis_case(element)
                && (element.element_id == analysis_case_id
                    || element_label(element).as_deref() == Some(analysis_case_id))
        })
        .ok_or_else(|| AnalysisSpecError::MissingAnalysisCase(analysis_case_id.to_string()))?;
    let model = inventory
        .analysis_cases
        .iter()
        .find(|case| case.element.element_id == element.element_id);

    Ok(analysis_spec_from_case(runtime, element, model))
}

fn analysis_spec_from_case(
    runtime: &Runtime,
    analysis_case: &Element,
    model: Option<&AnalysisCaseModel>,
) -> AnalysisSpec {
    let graph = runtime.graph();
    let subjects = collect_subjects(graph, analysis_case, model);
    let inputs = collect_model_refs(model, |case| case.inputs.clone());
    let assumptions = collect_assumptions(graph, analysis_case, model);
    let objectives = collect_objectives(graph, analysis_case);
    let calculations = collect_calculations(graph, analysis_case, model);
    let constraints = collect_constraints(graph, analysis_case, model);
    let requirements = collect_requirements(graph, analysis_case, model);
    let verification_cases = collect_verification_cases(graph, analysis_case, model);
    let views = collect_model_refs(model, |case| case.views.clone())
        .into_iter()
        .chain(collect_refs_from_properties(
            graph,
            analysis_case,
            &["view", "views"],
        ))
        .collect::<Vec<_>>();
    let concerns = collect_model_refs(model, |case| case.concerns.clone())
        .into_iter()
        .chain(collect_refs_from_properties(
            graph,
            analysis_case,
            &["concern", "concerns", "viewpoint", "viewpoints"],
        ))
        .collect::<Vec<_>>();
    let execution_context = execution_context_from_case(graph, analysis_case, &subjects);
    let dynamic_behavior_bindings =
        collect_dynamic_behavior_bindings(runtime, analysis_case, &subjects);
    let requires_dynamic_behavior = dynamic_behavior_required(
        runtime,
        analysis_case,
        &subjects,
        &execution_context,
        &dynamic_behavior_bindings,
    );
    let mut diagnostics = readiness_diagnostics(
        graph,
        analysis_case,
        &subjects,
        &requirements,
        &verification_cases,
        &dynamic_behavior_bindings,
        requires_dynamic_behavior,
    );
    let techniques = infer_techniques(
        runtime,
        analysis_case,
        &subjects,
        &calculations,
        &constraints,
        &requirements,
        &verification_cases,
        &views,
        &concerns,
        &execution_context,
        &dynamic_behavior_bindings,
    );
    if techniques.is_empty() {
        diagnostics.push(readiness_warning(
            "analysis.techniques.empty",
            "analysis case does not declare calculations, constraints, dynamic behavior, verification, views, or provider bindings",
            Some(analysis_case.element_id.clone()),
        ));
    }

    let readiness = readiness_status(&diagnostics);
    let expected_artifacts = expected_artifacts(&techniques, &dynamic_behavior_bindings);
    let mut spec = AnalysisSpec {
        case_ref: element_ref(analysis_case),
        model_revision: runtime.derived_feature_revision().to_string(),
        subjects: dedup_refs(subjects),
        inputs: dedup_refs(inputs),
        assumptions: dedup_refs(assumptions),
        objectives: dedup_refs(objectives),
        calculations: dedup_refs(calculations),
        constraints: dedup_refs(constraints),
        requirements: dedup_refs(requirements),
        verification_cases: dedup_refs(verification_cases),
        views: dedup_refs(views),
        concerns: dedup_refs(concerns),
        techniques,
        dynamic_behavior_bindings,
        execution_context,
        execution_plan: AnalysisExecutionPlan { steps: Vec::new() },
        expected_artifacts,
        readiness,
        readiness_diagnostics: diagnostics,
    };
    spec.execution_plan = execution_plan(&spec);
    spec
}

fn collect_subjects(
    graph: &Graph,
    analysis_case: &Element,
    model: Option<&AnalysisCaseModel>,
) -> Vec<AnalysisElementRef> {
    collect_model_refs(model, |case| case.subjects.clone())
        .into_iter()
        .chain(collect_refs_from_properties(
            graph,
            analysis_case,
            &["subject", "subjects"],
        ))
        .chain(native_owned_elements(graph, analysis_case, |element| {
            element.element_id.starts_with("subject.")
        }))
        .collect()
}

fn collect_assumptions(
    graph: &Graph,
    analysis_case: &Element,
    model: Option<&AnalysisCaseModel>,
) -> Vec<AnalysisElementRef> {
    collect_model_refs(model, |case| case.assumptions.clone())
        .into_iter()
        .chain(collect_refs_from_properties(
            graph,
            analysis_case,
            &["assumption", "assumptions"],
        ))
        .chain(native_owned_elements(graph, analysis_case, |element| {
            element.element_id.starts_with("assume.") || kind_contains(&element.kind, "assumption")
        }))
        .collect()
}

fn collect_objectives(graph: &Graph, analysis_case: &Element) -> Vec<AnalysisElementRef> {
    collect_refs_from_properties(graph, analysis_case, &["objective", "objectives"])
        .into_iter()
        .chain(native_owned_elements(graph, analysis_case, |element| {
            element.element_id.starts_with("objective.")
                || kind_contains(&element.kind, "objective")
        }))
        .collect()
}

fn collect_calculations(
    graph: &Graph,
    analysis_case: &Element,
    model: Option<&AnalysisCaseModel>,
) -> Vec<AnalysisElementRef> {
    collect_model_refs(model, |case| case.calculations.clone())
        .into_iter()
        .chain(collect_refs_from_properties(
            graph,
            analysis_case,
            &["calculation", "calculations", "calc", "calcs"],
        ))
        .chain(native_owned_elements(graph, analysis_case, |element| {
            kind_contains(&element.kind, "calculationusage")
        }))
        .collect()
}

fn collect_constraints(
    graph: &Graph,
    analysis_case: &Element,
    model: Option<&AnalysisCaseModel>,
) -> Vec<AnalysisElementRef> {
    collect_model_refs(model, |case| case.constraints.clone())
        .into_iter()
        .chain(collect_refs_from_properties(
            graph,
            analysis_case,
            &["constraint", "constraints"],
        ))
        .chain(native_owned_elements(graph, analysis_case, |element| {
            is_constraint_usage_kind(element)
                && !element.element_id.starts_with("assume.")
                && !element.element_id.starts_with("require.")
        }))
        .collect()
}

fn collect_requirements(
    graph: &Graph,
    analysis_case: &Element,
    model: Option<&AnalysisCaseModel>,
) -> Vec<AnalysisElementRef> {
    collect_model_refs(model, |case| case.requirements.clone())
        .into_iter()
        .chain(collect_refs_from_properties(
            graph,
            analysis_case,
            &["requirement", "requirements", "require", "requires"],
        ))
        .chain(native_owned_elements(
            graph,
            analysis_case,
            is_analysis_requirement,
        ))
        .collect()
}

fn collect_verification_cases(
    graph: &Graph,
    analysis_case: &Element,
    model: Option<&AnalysisCaseModel>,
) -> Vec<AnalysisElementRef> {
    collect_model_refs(model, |case| case.verification_cases.clone())
        .into_iter()
        .chain(collect_refs_from_properties(
            graph,
            analysis_case,
            &[
                "verification_case",
                "verification_cases",
                "verification",
                "verify",
            ],
        ))
        .collect()
}

fn collect_model_refs<F>(model: Option<&AnalysisCaseModel>, selector: F) -> Vec<AnalysisElementRef>
where
    F: FnOnce(&AnalysisCaseModel) -> Vec<AnalysisElementRef>,
{
    model.map(selector).unwrap_or_default()
}

fn collect_dynamic_behavior_bindings(
    runtime: &Runtime,
    analysis_case: &Element,
    subjects: &[AnalysisElementRef],
) -> Vec<AnalysisDynamicBehaviorBinding> {
    let graph = runtime.graph();
    let machines = crate::behavior::project_state_machines(runtime);
    let mut bindings = Vec::new();

    for subject in subjects {
        if let Some(machine_id) = explicit_subject_machine_id(analysis_case, subject) {
            let behavior = machines
                .iter()
                .find(|machine| machine.id == machine_id || machine.label == machine_id)
                .map(|machine| machine_behavior_ref(graph, machine))
                .or_else(|| graph.element_by_element_id(&machine_id).map(element_ref))
                .unwrap_or_else(|| {
                    AnalysisElementRef::new(machine_id.clone())
                        .with_kind("StateUsage")
                        .with_label(machine_id)
                });
            bindings.push(AnalysisDynamicBehaviorBinding {
                subject: subject.clone(),
                behavior,
                kind: AnalysisDynamicBehaviorKind::StateMachine,
            });
        }

        let Some(subject_element) = graph.element_by_element_id(&subject.element_id) else {
            continue;
        };
        let owner_candidates = subject_behavior_owner_candidates(subject, subject_element);

        if !bindings.iter().any(|binding| {
            binding.subject.element_id == subject.element_id
                && binding.kind == AnalysisDynamicBehaviorKind::StateMachine
        }) && let Some(machine) = machines.iter().find(|machine| {
            machine.states.iter().any(|state| {
                state.parent_state_id.is_none() && owner_candidates.contains(&state.owner_id)
            })
        }) {
            bindings.push(AnalysisDynamicBehaviorBinding {
                subject: subject.clone(),
                behavior: machine_behavior_ref(graph, machine),
                kind: AnalysisDynamicBehaviorKind::StateMachine,
            });
        }

        for action in graph
            .elements()
            .iter()
            .filter(|candidate| is_activity_behavior_source(candidate))
            .filter(|candidate| {
                behavior_owner_id(candidate)
                    .as_ref()
                    .is_some_and(|owner| owner_candidates.contains(owner))
            })
        {
            bindings.push(AnalysisDynamicBehaviorBinding {
                subject: subject.clone(),
                behavior: element_ref(action),
                kind: AnalysisDynamicBehaviorKind::Activity,
            });
        }
    }

    dedup_dynamic_behavior_bindings(bindings)
}

fn subject_behavior_owner_candidates(
    subject: &AnalysisElementRef,
    subject_element: &Element,
) -> Vec<String> {
    let mut owner_candidates = Vec::new();
    if let Some(subject_type) =
        string_property_any(subject_element, &["type", "definition", "owning_type"])
    {
        owner_candidates.push(subject_type);
    }
    owner_candidates.push(subject.element_id.clone());
    owner_candidates
}

fn explicit_subject_machine_id(
    analysis_case: &Element,
    subject: &AnalysisElementRef,
) -> Option<String> {
    analysis_case
        .properties
        .get("subjects")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(Value::as_object)
        .find_map(|object| {
            let subject_id = object
                .get("subject")
                .or_else(|| object.get("subject_id"))
                .or_else(|| object.get("subjectId"))
                .and_then(Value::as_str)?;
            let matches_subject = subject.element_id == subject_id
                || subject.label.as_deref() == Some(subject_id)
                || subject.element_id.ends_with(&format!(".{subject_id}"));
            matches_subject
                .then(|| {
                    object
                        .get("machine")
                        .or_else(|| object.get("machine_id"))
                        .or_else(|| object.get("machineId"))
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .flatten()
        })
}

fn machine_behavior_ref(
    graph: &Graph,
    machine: &crate::behavior::StateMachineModel,
) -> AnalysisElementRef {
    if let Some(element) = graph.element_by_element_id(&machine.id) {
        return element_ref(element);
    }
    AnalysisElementRef::new(machine.id.clone())
        .with_kind("StateUsage")
        .with_label(machine.label.clone())
}

fn behavior_owner_id(element: &Element) -> Option<String> {
    string_property_any(
        element,
        &[
            "owner",
            "owning_type",
            "owning_definition",
            "owning_namespace",
        ],
    )
}

fn is_activity_behavior_source(element: &Element) -> bool {
    let kind = canonical_kind(&element.kind);
    !kind.contains("state")
        && (kind.contains("actionusage")
            || kind.contains("actiondefinition")
            || kind.contains("performactionusage")
            || element.element_id.starts_with("action.")
            || element.element_id.starts_with("perform."))
}

fn dedup_dynamic_behavior_bindings(
    bindings: Vec<AnalysisDynamicBehaviorBinding>,
) -> Vec<AnalysisDynamicBehaviorBinding> {
    let mut seen = BTreeSet::new();
    bindings
        .into_iter()
        .filter(|binding| {
            seen.insert((
                binding.subject.element_id.clone(),
                binding.behavior.element_id.clone(),
                binding.kind,
            ))
        })
        .collect()
}

fn execution_context_from_case(
    graph: &Graph,
    analysis_case: &Element,
    subjects: &[AnalysisElementRef],
) -> AnalysisExecutionContext {
    let mut initial_values = direct_initial_values(analysis_case);
    merge_initial_values(
        &mut initial_values,
        initial_values_from_assumptions(graph, analysis_case, subjects),
    );
    AnalysisExecutionContext {
        initial_values,
        clock: clock_config(analysis_case),
        provider_bindings: provider_bindings(analysis_case),
    }
}

fn direct_initial_values(analysis_case: &Element) -> BTreeMap<String, BTreeMap<String, Value>> {
    let mut values = BTreeMap::new();
    let Some(object) = analysis_case
        .properties
        .get("initial_values")
        .or_else(|| analysis_case.properties.get("initialValues"))
        .and_then(Value::as_object)
    else {
        return values;
    };

    for (key, value) in object {
        if let Some((subject, feature)) = key.split_once('|') {
            values
                .entry(subject.to_string())
                .or_insert_with(BTreeMap::new)
                .insert(feature.to_string(), value.clone());
        }
    }
    values
}

fn initial_values_from_assumptions(
    graph: &Graph,
    analysis_case: &Element,
    subjects: &[AnalysisElementRef],
) -> BTreeMap<String, BTreeMap<String, Value>> {
    let mut values = BTreeMap::new();
    let subject_aliases = subjects
        .iter()
        .flat_map(|subject| {
            [
                (subject.element_id.clone(), subject.element_id.clone()),
                (
                    subject
                        .label
                        .clone()
                        .unwrap_or_else(|| subject.element_id.clone()),
                    subject.element_id.clone(),
                ),
            ]
        })
        .collect::<BTreeMap<_, _>>();
    let default_subject = (subjects.len() == 1).then(|| subjects[0].element_id.as_str());

    for assumption in native_owned_elements_raw(graph, analysis_case, |element| {
        element.element_id.starts_with("assume.")
    }) {
        let Some(expression) = assumption.properties.get("expression_ir") else {
            continue;
        };
        if let Some(((subject, feature), value)) =
            initial_value_from_assume_expression(expression, &subject_aliases, default_subject)
        {
            values
                .entry(subject)
                .or_insert_with(BTreeMap::new)
                .insert(feature, value);
        }
    }
    values
}

fn merge_initial_values(
    target: &mut BTreeMap<String, BTreeMap<String, Value>>,
    source: BTreeMap<String, BTreeMap<String, Value>>,
) {
    for (subject, features) in source {
        target.entry(subject).or_default().extend(features);
    }
}

fn initial_value_from_assume_expression(
    expression: &Value,
    subject_aliases: &BTreeMap<String, String>,
    default_subject: Option<&str>,
) -> Option<((String, String), Value)> {
    let object = expression.as_object()?;
    if object.get("kind")?.as_str()? != "binary" || object.get("op")?.as_str()? != "equal" {
        return None;
    }
    let left = object.get("left")?;
    let right = object.get("right")?;
    path_literal_initial_value(left, right, subject_aliases, default_subject)
        .or_else(|| path_literal_initial_value(right, left, subject_aliases, default_subject))
}

fn path_literal_initial_value(
    path: &Value,
    literal: &Value,
    subject_aliases: &BTreeMap<String, String>,
    default_subject: Option<&str>,
) -> Option<((String, String), Value)> {
    if path.get("kind")?.as_str()? != "path" || literal.get("kind")?.as_str()? != "literal" {
        return None;
    }
    let segments = path
        .get("segments")?
        .as_array()?
        .iter()
        .filter_map(expression_path_segment_name)
        .collect::<Vec<_>>();
    let value = literal.get("value")?.clone();
    match segments.as_slice() {
        [subject_name, feature @ ..] if !feature.is_empty() => {
            let subject = subject_aliases.get(subject_name)?.clone();
            Some(((subject, feature.join(".")), value))
        }
        [feature] => {
            let subject = default_subject?.to_string();
            Some(((subject, feature.clone()), value))
        }
        _ => None,
    }
}

fn expression_path_segment_name(segment: &Value) -> Option<String> {
    segment.as_str().map(ToOwned::to_owned).or_else(|| {
        segment
            .get("name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

fn clock_config(analysis_case: &Element) -> Option<AnalysisClockConfig> {
    let config = AnalysisClockConfig {
        max_steps: usize_property_any(analysis_case, &["max_steps", "maxSteps"]),
        step_duration_s: f64_property_any(analysis_case, &["step_duration_s", "stepDurationS"]),
        max_time_s: f64_property_any(analysis_case, &["max_time_s", "maxTimeS"]),
        fixed_step_s: f64_property_any(analysis_case, &["fixed_step_s", "fixedStepS"]),
        sample_interval_s: f64_property_any(
            analysis_case,
            &["sample_interval_s", "sampleIntervalS"],
        ),
        change_loop_limit: usize_property_any(
            analysis_case,
            &["change_loop_limit", "changeLoopLimit"],
        ),
    };
    (config.max_steps.is_some()
        || config.step_duration_s.is_some()
        || config.max_time_s.is_some()
        || config.fixed_step_s.is_some()
        || config.sample_interval_s.is_some()
        || config.change_loop_limit.is_some())
    .then_some(config)
}

fn provider_bindings(analysis_case: &Element) -> BTreeMap<String, Value> {
    [
        "provider",
        "providers",
        "tool",
        "tool_execution",
        "toolExecution",
    ]
    .into_iter()
    .filter_map(|key| {
        analysis_case
            .properties
            .get(key)
            .map(|value| (key.to_string(), value.clone()))
    })
    .collect()
}

fn infer_techniques(
    runtime: &Runtime,
    analysis_case: &Element,
    subjects: &[AnalysisElementRef],
    calculations: &[AnalysisElementRef],
    constraints: &[AnalysisElementRef],
    requirements: &[AnalysisElementRef],
    verification_cases: &[AnalysisElementRef],
    views: &[AnalysisElementRef],
    concerns: &[AnalysisElementRef],
    context: &AnalysisExecutionContext,
    dynamic_behavior_bindings: &[AnalysisDynamicBehaviorBinding],
) -> Vec<AnalysisTechnique> {
    let mut techniques = BTreeSet::new();
    if has_text_property(analysis_case, &["query", "dsl", "script"]) {
        techniques.insert(AnalysisTechnique::Query);
    }
    if !calculations.is_empty() {
        techniques.insert(AnalysisTechnique::Calculation);
    }
    if !constraints.is_empty() {
        techniques.insert(AnalysisTechnique::ConstraintEvaluation);
    }
    if dynamic_behavior_required(
        runtime,
        analysis_case,
        subjects,
        context,
        dynamic_behavior_bindings,
    ) {
        techniques.insert(AnalysisTechnique::DynamicBehavior);
    }
    if !requirements.is_empty() || !verification_cases.is_empty() {
        techniques.insert(AnalysisTechnique::Verification);
    }
    if is_trade_study(analysis_case) {
        techniques.insert(AnalysisTechnique::TradeStudy);
    }
    if !context.provider_bindings.is_empty() {
        techniques.insert(AnalysisTechnique::ExternalProvider);
    }
    if !views.is_empty() || !concerns.is_empty() {
        techniques.insert(AnalysisTechnique::ViewGeneration);
    }
    techniques.into_iter().collect()
}

fn dynamic_behavior_required(
    runtime: &Runtime,
    analysis_case: &Element,
    subjects: &[AnalysisElementRef],
    context: &AnalysisExecutionContext,
    dynamic_behavior_bindings: &[AnalysisDynamicBehaviorBinding],
) -> bool {
    let graph = runtime.graph();
    !dynamic_behavior_bindings.is_empty()
        || context.clock.is_some()
        || analysis_case
            .properties
            .get("events")
            .or_else(|| analysis_case.properties.get("simulation_script"))
            .or_else(|| analysis_case.properties.get("simulationScript"))
            .is_some()
        || collect_refs_from_properties(graph, analysis_case, &["simulation", "simulations"])
            .iter()
            .any(|reference| !reference.element_id.is_empty())
        || explicit_subjects_have_machines(analysis_case)
        || explicit_subjects_have_events(analysis_case)
        || subjects_have_state_machines(runtime, subjects)
}

fn explicit_subjects_have_machines(analysis_case: &Element) -> bool {
    analysis_case
        .properties
        .get("subjects")
        .and_then(Value::as_array)
        .is_some_and(|subjects| {
            subjects.iter().any(|subject| {
                subject.as_object().is_some_and(|object| {
                    object
                        .get("machine")
                        .or_else(|| object.get("machine_id"))
                        .or_else(|| object.get("machineId"))
                        .and_then(Value::as_str)
                        .is_some()
                })
            })
        })
}

fn explicit_subjects_have_events(analysis_case: &Element) -> bool {
    analysis_case
        .properties
        .get("subjects")
        .and_then(Value::as_array)
        .is_some_and(|subjects| {
            subjects.iter().any(|subject| {
                subject.as_object().is_some_and(|object| {
                    object
                        .get("events")
                        .or_else(|| object.get("simulation_script"))
                        .or_else(|| object.get("simulationScript"))
                        .is_some()
                })
            })
        })
}

fn subjects_have_state_machines(runtime: &Runtime, subjects: &[AnalysisElementRef]) -> bool {
    let graph = runtime.graph();
    let machines = crate::behavior::project_state_machines(runtime);
    subjects.iter().any(|subject| {
        graph
            .element_by_element_id(&subject.element_id)
            .and_then(|element| string_property_any(element, &["type", "definition"]))
            .is_some_and(|subject_type| {
                machines.iter().any(|machine| {
                    machine.states.iter().any(|state| {
                        state.parent_state_id.is_none() && state.owner_id == subject_type
                    })
                })
            })
    })
}

fn is_trade_study(analysis_case: &Element) -> bool {
    kind_contains(&analysis_case.kind, "tradestudy")
        || analysis_case
            .properties
            .get("alternatives")
            .or_else(|| analysis_case.properties.get("alternative"))
            .is_some()
}

fn expected_artifacts(
    techniques: &[AnalysisTechnique],
    dynamic_behavior_bindings: &[AnalysisDynamicBehaviorBinding],
) -> Vec<AnalysisExpectedArtifact> {
    let mut artifacts = Vec::new();
    if techniques.contains(&AnalysisTechnique::DynamicBehavior) {
        if dynamic_behavior_bindings
            .iter()
            .any(|binding| binding.kind == AnalysisDynamicBehaviorKind::StateMachine)
            || dynamic_behavior_bindings.is_empty()
        {
            artifacts.push(AnalysisExpectedArtifact {
                kind: "simulation_trace".to_string(),
                schema: SIMULATION_TRACE_SCHEMA.to_string(),
            });
        }
        if dynamic_behavior_bindings
            .iter()
            .any(|binding| binding.kind == AnalysisDynamicBehaviorKind::Activity)
        {
            artifacts.push(AnalysisExpectedArtifact {
                kind: "activity_execution_summary".to_string(),
                schema: ACTIVITY_EXECUTION_SUMMARY_SCHEMA.to_string(),
            });
        }
    }
    if techniques.contains(&AnalysisTechnique::ConstraintEvaluation) {
        artifacts.push(AnalysisExpectedArtifact {
            kind: "constraint_analysis_summary".to_string(),
            schema: CONSTRAINT_SUMMARY_SCHEMA.to_string(),
        });
    }
    if techniques.contains(&AnalysisTechnique::Verification) {
        artifacts.push(AnalysisExpectedArtifact {
            kind: "requirement_verdicts".to_string(),
            schema: VERDICT_SUMMARY_SCHEMA.to_string(),
        });
    }
    if techniques.contains(&AnalysisTechnique::ViewGeneration) {
        artifacts.push(AnalysisExpectedArtifact {
            kind: "analysis_view".to_string(),
            schema: "mercurio.analysis.view.v1".to_string(),
        });
    }
    artifacts
}

fn execution_plan(spec: &AnalysisSpec) -> AnalysisExecutionPlan {
    let mut steps = vec![AnalysisExecutionStep {
        kind: AnalysisExecutionStepKind::ScopeSubjects,
        label: "Scope analysis subjects".to_string(),
        techniques: Vec::new(),
        elements: spec.subjects.clone(),
    }];

    if !(spec.assumptions.is_empty() && spec.execution_context.initial_values.is_empty()) {
        steps.push(AnalysisExecutionStep {
            kind: AnalysisExecutionStepKind::BindAssumptions,
            label: "Bind assumptions and initial values".to_string(),
            techniques: Vec::new(),
            elements: spec.assumptions.clone(),
        });
    }
    if !spec.inputs.is_empty() {
        steps.push(AnalysisExecutionStep {
            kind: AnalysisExecutionStepKind::ResolveInputs,
            label: "Resolve analysis inputs".to_string(),
            techniques: Vec::new(),
            elements: spec.inputs.clone(),
        });
    }
    if !spec.calculations.is_empty() {
        steps.push(AnalysisExecutionStep {
            kind: AnalysisExecutionStepKind::PreRunCalculation,
            label: "Project pre-run calculations".to_string(),
            techniques: vec![AnalysisTechnique::Calculation],
            elements: spec.calculations.clone(),
        });
    }
    if !spec.constraints.is_empty() {
        steps.push(AnalysisExecutionStep {
            kind: AnalysisExecutionStepKind::PreRunConstraintEvaluation,
            label: "Project pre-run constraints".to_string(),
            techniques: vec![AnalysisTechnique::ConstraintEvaluation],
            elements: spec.constraints.clone(),
        });
    }
    if spec
        .techniques
        .contains(&AnalysisTechnique::DynamicBehavior)
    {
        let mut elements = spec.subjects.clone();
        elements.extend(
            spec.dynamic_behavior_bindings
                .iter()
                .map(|binding| binding.behavior.clone()),
        );
        steps.push(AnalysisExecutionStep {
            kind: AnalysisExecutionStepKind::DynamicBehavior,
            label: "Project dynamic behavior execution".to_string(),
            techniques: vec![AnalysisTechnique::DynamicBehavior],
            elements: dedup_refs(elements),
        });
    }
    if !spec.requirements.is_empty() {
        steps.push(AnalysisExecutionStep {
            kind: AnalysisExecutionStepKind::PostRunRequirementEvaluation,
            label: "Project requirement checks".to_string(),
            techniques: vec![AnalysisTechnique::Verification],
            elements: spec.requirements.clone(),
        });
    }
    if !spec.verification_cases.is_empty() {
        steps.push(AnalysisExecutionStep {
            kind: AnalysisExecutionStepKind::VerificationEvidence,
            label: "Project verification evidence capture".to_string(),
            techniques: vec![AnalysisTechnique::Verification],
            elements: spec.verification_cases.clone(),
        });
    }
    if !(spec.views.is_empty() && spec.concerns.is_empty()) {
        let mut elements = spec.views.clone();
        elements.extend(spec.concerns.clone());
        steps.push(AnalysisExecutionStep {
            kind: AnalysisExecutionStepKind::ProduceViews,
            label: "Project stakeholder views".to_string(),
            techniques: vec![AnalysisTechnique::ViewGeneration],
            elements,
        });
    }
    steps.push(AnalysisExecutionStep {
        kind: AnalysisExecutionStepKind::RecordResult,
        label: "Record analysis result, artifacts, diagnostics, and evidence".to_string(),
        techniques: spec.techniques.clone(),
        elements: vec![spec.case_ref.clone()],
    });

    AnalysisExecutionPlan { steps }
}

fn readiness_diagnostics(
    graph: &Graph,
    analysis_case: &Element,
    subjects: &[AnalysisElementRef],
    requirements: &[AnalysisElementRef],
    verification_cases: &[AnalysisElementRef],
    dynamic_behavior_bindings: &[AnalysisDynamicBehaviorBinding],
    requires_dynamic_behavior: bool,
) -> Vec<AnalysisReadinessDiagnostic> {
    let mut diagnostics = Vec::new();
    if subjects.is_empty() {
        diagnostics.push(readiness_error(
            "analysis.subjects.missing",
            "analysis case does not bind any subjects",
            Some(analysis_case.element_id.clone()),
        ));
    }
    diagnostics.extend(explicit_subject_diagnostics(
        graph,
        analysis_case,
        dynamic_behavior_bindings,
        requires_dynamic_behavior,
    ));
    if !requirements.is_empty() && verification_cases.is_empty() {
        diagnostics.push(readiness_warning(
            "analysis.verification.missing",
            "analysis case references requirements but no verification case",
            Some(analysis_case.element_id.clone()),
        ));
    }
    diagnostics.extend(activity_execution_diagnostics(dynamic_behavior_bindings));
    diagnostics
}

fn activity_execution_diagnostics(
    dynamic_behavior_bindings: &[AnalysisDynamicBehaviorBinding],
) -> Vec<AnalysisReadinessDiagnostic> {
    dynamic_behavior_bindings
        .iter()
        .filter(|binding| binding.kind == AnalysisDynamicBehaviorKind::Activity)
        .map(|binding| {
            readiness_warning(
                "analysis.dynamic.activity_execution.pending",
                format!(
                    "activity `{}` is projected as dynamic behavior, but activity execution is not implemented yet",
                    binding
                        .behavior
                        .label
                        .as_deref()
                        .unwrap_or(binding.behavior.element_id.as_str())
                ),
                Some(binding.behavior.element_id.clone()),
            )
        })
        .collect()
}

fn explicit_subject_diagnostics(
    graph: &Graph,
    analysis_case: &Element,
    dynamic_behavior_bindings: &[AnalysisDynamicBehaviorBinding],
    requires_dynamic_behavior: bool,
) -> Vec<AnalysisReadinessDiagnostic> {
    let Some(subjects) = analysis_case
        .properties
        .get("subjects")
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    let mut diagnostics = Vec::new();
    for (index, subject) in subjects.iter().enumerate() {
        let Some(object) = subject.as_object() else {
            diagnostics.push(readiness_error(
                "analysis.subject.shape",
                format!("analysis case subject at index {index} must be an object"),
                Some(analysis_case.element_id.clone()),
            ));
            continue;
        };
        let subject_id = object
            .get("subject")
            .or_else(|| object.get("subject_id"))
            .or_else(|| object.get("subjectId"))
            .and_then(Value::as_str);
        match subject_id {
            Some(id) if graph.element_by_element_id(id).is_none() => {
                diagnostics.push(readiness_error(
                    "analysis.subject.unknown",
                    format!("analysis case subject `{id}` does not resolve to a model element"),
                    Some(analysis_case.element_id.clone()),
                ));
            }
            Some(_) => {}
            None => diagnostics.push(readiness_error(
                "analysis.subject.missing_ref",
                format!("analysis case subject at index {index} must define `subject`"),
                Some(analysis_case.element_id.clone()),
            )),
        }
        let has_explicit_machine = object
            .get("machine")
            .or_else(|| object.get("machine_id"))
            .or_else(|| object.get("machineId"))
            .and_then(Value::as_str)
            .is_some();
        if requires_dynamic_behavior
            && !has_explicit_machine
            && !subject_has_dynamic_behavior_binding(subject_id, dynamic_behavior_bindings)
        {
            diagnostics.push(readiness_error(
                "analysis.subject.machine_missing",
                format!("analysis case subject at index {index} must define `machine` or resolve owned dynamic behavior"),
                Some(analysis_case.element_id.clone()),
            ));
        }
    }
    diagnostics
}

fn subject_has_dynamic_behavior_binding(
    subject_id: Option<&str>,
    dynamic_behavior_bindings: &[AnalysisDynamicBehaviorBinding],
) -> bool {
    let Some(subject_id) = subject_id else {
        return false;
    };
    dynamic_behavior_bindings
        .iter()
        .any(|binding| binding.subject.element_id == subject_id)
}

fn readiness_status(diagnostics: &[AnalysisReadinessDiagnostic]) -> AnalysisReadinessStatus {
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == AnalysisReadinessSeverity::Error)
    {
        AnalysisReadinessStatus::Blocked
    } else if diagnostics.is_empty() {
        AnalysisReadinessStatus::Ready
    } else {
        AnalysisReadinessStatus::Partial
    }
}

fn readiness_error(
    code: &str,
    message: impl Into<String>,
    element_id: Option<String>,
) -> AnalysisReadinessDiagnostic {
    AnalysisReadinessDiagnostic {
        severity: AnalysisReadinessSeverity::Error,
        code: code.to_string(),
        message: message.into(),
        element_id,
    }
}

fn readiness_warning(
    code: &str,
    message: impl Into<String>,
    element_id: Option<String>,
) -> AnalysisReadinessDiagnostic {
    AnalysisReadinessDiagnostic {
        severity: AnalysisReadinessSeverity::Warning,
        code: code.to_string(),
        message: message.into(),
        element_id,
    }
}

fn collect_refs_from_properties(
    graph: &Graph,
    element: &Element,
    properties: &[&str],
) -> Vec<AnalysisElementRef> {
    let mut refs = Vec::new();
    for property in properties {
        if let Some(value) = element.properties.get(*property) {
            collect_refs_from_value(graph, value, &mut refs);
        }
    }
    dedup_refs(refs)
}

fn collect_refs_from_value(graph: &Graph, value: &Value, refs: &mut Vec<AnalysisElementRef>) {
    match value {
        Value::String(element_id) => {
            if let Some(element) = graph.element_by_element_id(element_id) {
                refs.push(element_ref(element));
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_refs_from_value(graph, value, refs);
            }
        }
        Value::Object(object) => {
            for value in object.values() {
                collect_refs_from_value(graph, value, refs);
            }
        }
        _ => {}
    }
}

fn native_owned_elements<F>(graph: &Graph, owner: &Element, predicate: F) -> Vec<AnalysisElementRef>
where
    F: Fn(&Element) -> bool,
{
    native_owned_elements_raw(graph, owner, predicate)
        .into_iter()
        .map(element_ref)
        .collect()
}

fn native_owned_elements_raw<'a, F>(
    graph: &'a Graph,
    owner: &Element,
    predicate: F,
) -> Vec<&'a Element>
where
    F: Fn(&Element) -> bool,
{
    graph
        .elements()
        .iter()
        .filter(|candidate| {
            string_property_any(candidate, &["owner", "owning_type"]).as_deref()
                == Some(owner.element_id.as_str())
                && predicate(candidate)
        })
        .collect()
}

fn element_ref(element: &Element) -> AnalysisElementRef {
    AnalysisElementRef::from_graph_element(element)
}

fn element_label(element: &Element) -> Option<String> {
    string_property_any(element, &["declared_name", "name"]).or_else(|| {
        element
            .element_id
            .rsplit([':', '.'])
            .find(|part| !part.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn string_property_any(element: &Element, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| element.properties.get(*key).and_then(trimmed_string_value))
}

fn trimmed_string_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => non_empty_trimmed_string(text),
        Value::Array(values) => values.iter().find_map(trimmed_string_value),
        _ => None,
    }
}

fn non_empty_trimmed_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn has_text_property(element: &Element, keys: &[&str]) -> bool {
    keys.iter()
        .any(|key| string_property_any(element, &[*key]).is_some())
}

fn usize_property_any(element: &Element, keys: &[&str]) -> Option<usize> {
    keys.iter()
        .find_map(|key| element.properties.get(*key).and_then(Value::as_u64))
        .map(|value| value as usize)
}

fn f64_property_any(element: &Element, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| element.properties.get(*key).and_then(Value::as_f64))
}

fn dedup_refs(refs: Vec<AnalysisElementRef>) -> Vec<AnalysisElementRef> {
    let mut seen = BTreeSet::new();
    refs.into_iter()
        .filter(|reference| seen.insert(reference.element_id.clone()))
        .collect()
}

fn is_project_analysis_case(element: &Element) -> bool {
    element.layer >= 2
        && kind_contains(&element.kind, "analysiscase")
        && element.element_id != "AnalysisCases::AnalysisCase"
        && !element.element_id.starts_with("SysML::")
        && !string_property_any(element, &["source_file", "sourceFile"])
            .is_some_and(|source| source.starts_with("Systems Library/"))
}

fn is_constraint_usage_kind(element: &Element) -> bool {
    kind_contains(&element.kind, "constraintusage")
        || canonical_kind(&element.kind) == "constraint"
        || kind_contains(&element.kind, "assertconstraintusage")
}

fn is_analysis_requirement(element: &Element) -> bool {
    kind_contains(&element.kind, "requireusage")
        || kind_contains(&element.kind, "requirementusage")
        || element.element_id.starts_with("require.")
}

fn kind_contains(kind: &str, needle: &str) -> bool {
    canonical_kind(kind).contains(needle)
}

fn canonical_kind(kind: &str) -> String {
    kind.replace([':', '.', ' ', '_'], "").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile_sysml_text, load_sysml_baseline};
    use mercurio_foundation::{KirDocument, KirElement};
    use serde_json::json;

    #[test]
    fn projects_property_backed_analysis_spec_without_running() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element(
                    "part.Vehicle",
                    "PartUsage",
                    [("declared_name", json!("Vehicle"))],
                ),
                element(
                    "calc.TotalMass",
                    "CalculationUsage",
                    [("declared_name", json!("TotalMass"))],
                ),
                element(
                    "constraint.MaxMass",
                    "ConstraintUsage",
                    [("declared_name", json!("MaxMass"))],
                ),
                element(
                    "req.MassLimit",
                    "RequirementUsage",
                    [("declared_name", json!("MassLimit"))],
                ),
                element(
                    "verify.MassLimit",
                    "VerificationCaseUsage",
                    [("declared_name", json!("VerifyMassLimit"))],
                ),
                element(
                    "state.Vehicle.idle",
                    "StateUsage",
                    [
                        ("owning_type", json!("VehicleMachine")),
                        ("is_initial", json!(true)),
                    ],
                ),
                element(
                    "analysis.MassCompliance",
                    "SysML::Systems::AnalysisCaseDefinition",
                    [
                        ("declared_name", json!("MassCompliance")),
                        (
                            "subjects",
                            json!([
                                {
                                    "subject": "part.Vehicle",
                                    "machine": "VehicleMachine",
                                    "events": [
                                        { "id": "event.start", "trigger": "start" }
                                    ]
                                }
                            ]),
                        ),
                        ("calculations", json!(["calc.TotalMass"])),
                        ("constraints", json!(["constraint.MaxMass"])),
                        ("requirements", json!(["req.MassLimit"])),
                        ("verification_cases", json!(["verify.MassLimit"])),
                        ("max_steps", json!(12)),
                        ("step_duration_s", json!(0.5)),
                        ("initial_values", json!({ "part.Vehicle|mass": 950.0 })),
                    ],
                ),
            ],
        })
        .unwrap();

        let spec = project_analysis_spec(&runtime, "MassCompliance").unwrap();

        assert_eq!(spec.case_ref.element_id, "analysis.MassCompliance");
        assert_eq!(spec.subjects[0].element_id, "part.Vehicle");
        assert_eq!(
            spec.execution_context.initial_values["part.Vehicle"]["mass"],
            json!(950.0)
        );
        assert_eq!(spec.execution_context.clock.unwrap().max_steps, Some(12));
        assert!(spec.techniques.contains(&AnalysisTechnique::Calculation));
        assert!(
            spec.techniques
                .contains(&AnalysisTechnique::ConstraintEvaluation)
        );
        assert!(
            spec.techniques
                .contains(&AnalysisTechnique::DynamicBehavior)
        );
        assert_eq!(spec.dynamic_behavior_bindings.len(), 1);
        assert_eq!(
            spec.dynamic_behavior_bindings[0].subject.element_id,
            "part.Vehicle"
        );
        assert_eq!(
            spec.dynamic_behavior_bindings[0].behavior.element_id,
            "VehicleMachine"
        );
        assert_eq!(
            spec.dynamic_behavior_bindings[0].kind,
            AnalysisDynamicBehaviorKind::StateMachine
        );
        assert!(spec.techniques.contains(&AnalysisTechnique::Verification));
        assert_eq!(spec.readiness, AnalysisReadinessStatus::Ready);
        assert!(spec.expected_artifacts.iter().any(|artifact| {
            artifact.kind == "simulation_trace" && artifact.schema == SIMULATION_TRACE_SCHEMA
        }));
        assert!(
            spec.execution_plan
                .steps
                .iter()
                .any(|step| { step.kind == AnalysisExecutionStepKind::DynamicBehavior })
        );
    }

    #[test]
    fn projects_native_subject_assumption_and_objective() {
        let stdlib = load_sysml_baseline().unwrap();
        let document = compile_sysml_text(
            r#"
            package Demo {
                import ScalarValues::*;

                part def Printer {
                    attribute bed_temperature : Real = 22.0;
                    attribute target_temperature : Real = 80.0;
                    attribute heat_rate : Real = 10.0;

                    state lifecycle {
                        state Idle;
                        state Printing {
                            do action integrate {
                                assert constraint {
                                    bed_temperature == bed_temperature + heat_rate * duration;
                                }
                            }
                        }
                        state Complete;
                        transition start first Idle accept start then Printing;
                        transition complete first Printing accept when bed_temperature >= target_temperature then Complete;
                    }
                }

                analysis def PrintSequence :> AnalysisCase {
                    subject printer : Printer;
                    assume constraint = printer.bed_temperature == 22.0;
                    objective thermalProfile { subject = printer.bed_temperature; }
                }
            }
            "#,
            "native-analysis-spec.sysml",
            &stdlib,
        )
        .unwrap();
        let runtime = Runtime::from_document(document).unwrap();

        let specs = list_analysis_specs(&runtime);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].case_ref.label.as_deref(), Some("PrintSequence"));

        let spec = project_analysis_spec(&runtime, "PrintSequence").unwrap();

        assert_eq!(spec.subjects.len(), 1);
        assert!(spec.subjects[0].element_id.starts_with("subject."));
        assert_eq!(spec.assumptions.len(), 1);
        assert_eq!(spec.objectives.len(), 1);
        assert!(
            spec.techniques
                .contains(&AnalysisTechnique::DynamicBehavior)
        );
        assert_eq!(spec.dynamic_behavior_bindings.len(), 1);
        assert_eq!(
            spec.dynamic_behavior_bindings[0].subject.element_id,
            spec.subjects[0].element_id
        );
        assert_eq!(
            spec.dynamic_behavior_bindings[0].kind,
            AnalysisDynamicBehaviorKind::StateMachine
        );
        assert_eq!(
            spec.dynamic_behavior_bindings[0].behavior.label.as_deref(),
            Some("lifecycle")
        );
        assert_eq!(
            spec.execution_context.initial_values[&spec.subjects[0].element_id]["bed_temperature"],
            json!(22.0)
        );
    }

    #[test]
    fn projects_2026_04_do_behavior_subject_binding() {
        let locator = crate::StdlibLocator::for_release("2026-04").unwrap();
        let stdlib = crate::load_sysml_baseline_from_locator(&locator).unwrap();
        let document = compile_sysml_text(
            r#"
            package Demo {
                import ScalarValues::*;

                part def ThermalChamber {
                    attribute temperature : Real = 20.0;
                    attribute targetTemperature : Real = 80.0;
                    attribute heatRate : Real = 10.0;

                    state lifecycle {
                        state Cold;
                        state Heating {
                            do action integrate {
                                assert constraint {
                                    temperature == temperature + heatRate * duration;
                                }
                            }
                        }
                        state Ready;

                        transition cold_heating first Cold accept start then Heating;
                        transition heating_ready first Heating accept when temperature >= targetTemperature then Ready;
                    }
                }

                analysis def HeatProfile :> AnalysisCase {
                    subject chamber : ThermalChamber;
                    assume constraint = chamber.temperature == 20.0;

                    objective chamberTemperature {
                        subject = chamber.temperature;
                    }
                }
            }
            "#,
            "do-behavior-analysis-2026-04.sysml",
            &stdlib,
        )
        .unwrap();
        let runtime = Runtime::from_document(document).unwrap();

        let spec = project_analysis_spec(&runtime, "HeatProfile").unwrap();

        assert_eq!(spec.readiness, AnalysisReadinessStatus::Ready);
        assert_eq!(spec.dynamic_behavior_bindings.len(), 1);
        assert!(
            spec.techniques
                .contains(&AnalysisTechnique::DynamicBehavior)
        );
        assert!(spec.expected_artifacts.iter().any(|artifact| {
            artifact.kind == "simulation_trace" && artifact.schema == SIMULATION_TRACE_SCHEMA
        }));

        let graph_runtime = Runtime::from_graph(runtime.graph().clone()).unwrap();
        let graph_spec = project_analysis_spec(&graph_runtime, "HeatProfile").unwrap();
        assert_eq!(graph_spec.readiness, AnalysisReadinessStatus::Ready);
        assert_eq!(graph_spec.dynamic_behavior_bindings.len(), 1);
    }

    #[test]
    fn projects_activity_binding_as_future_dynamic_behavior() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element(
                    "type.Printer",
                    "PartDefinition",
                    [("declared_name", json!("Printer"))],
                ),
                element(
                    "action.Printer.heatBed",
                    "ActionUsage",
                    [
                        ("declared_name", json!("heatBed")),
                        ("owner", json!("type.Printer")),
                    ],
                ),
                element(
                    "analysis.Warmup",
                    "AnalysisCaseUsage",
                    [("declared_name", json!("Warmup"))],
                ),
                element(
                    "subject.Warmup.printer",
                    "PartUsage",
                    [
                        ("declared_name", json!("printer")),
                        ("owner", json!("analysis.Warmup")),
                        ("type", json!("type.Printer")),
                    ],
                ),
            ],
        })
        .unwrap();

        let spec = project_analysis_spec(&runtime, "Warmup").unwrap();

        assert!(
            spec.techniques
                .contains(&AnalysisTechnique::DynamicBehavior)
        );
        assert_eq!(spec.dynamic_behavior_bindings.len(), 1);
        assert_eq!(
            spec.dynamic_behavior_bindings[0].kind,
            AnalysisDynamicBehaviorKind::Activity
        );
        assert_eq!(
            spec.dynamic_behavior_bindings[0].behavior.element_id,
            "action.Printer.heatBed"
        );
        assert!(spec.expected_artifacts.iter().any(|artifact| {
            artifact.kind == "activity_execution_summary"
                && artifact.schema == ACTIVITY_EXECUTION_SUMMARY_SCHEMA
        }));
        assert_eq!(spec.readiness, AnalysisReadinessStatus::Partial);
        assert!(spec.readiness_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "analysis.dynamic.activity_execution.pending"
        }));
    }

    #[test]
    fn accepts_explicit_subject_with_inferred_activity_binding() {
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
                    "action.Printer.heatBed",
                    "ActionUsage",
                    [
                        ("declared_name", json!("heatBed")),
                        ("owner", json!("type.Printer")),
                    ],
                ),
                element(
                    "analysis.Warmup",
                    "AnalysisCaseUsage",
                    [
                        ("declared_name", json!("Warmup")),
                        ("subjects", json!([{ "subject": "part.Printer" }])),
                    ],
                ),
            ],
        })
        .unwrap();

        let spec = project_analysis_spec(&runtime, "Warmup").unwrap();

        assert_eq!(spec.readiness, AnalysisReadinessStatus::Partial);
        assert_eq!(spec.subjects[0].element_id, "part.Printer");
        assert_eq!(spec.dynamic_behavior_bindings.len(), 1);
        assert_eq!(
            spec.dynamic_behavior_bindings[0].kind,
            AnalysisDynamicBehaviorKind::Activity
        );
        assert!(
            spec.readiness_diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "analysis.subject.machine_missing")
        );
        assert!(spec.readiness_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "analysis.dynamic.activity_execution.pending"
        }));
    }

    #[test]
    fn reports_readiness_diagnostics_without_running() {
        let runtime = Runtime::from_document(KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![element(
                "analysis.Bad",
                "AnalysisCaseUsage",
                [
                    ("declared_name", json!("Bad")),
                    (
                        "subjects",
                        json!([{ "subject": "part.Missing", "events": [{ "trigger": "start" }] }]),
                    ),
                ],
            )],
        })
        .unwrap();

        let spec = project_analysis_spec(&runtime, "Bad").unwrap();

        assert_eq!(spec.readiness, AnalysisReadinessStatus::Blocked);
        assert!(
            spec.readiness_diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == "analysis.subject.unknown" })
        );
        assert!(
            spec.readiness_diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == "analysis.subject.machine_missing" })
        );
    }

    fn element<const N: usize>(id: &str, kind: &str, properties: [(&str, Value); N]) -> KirElement {
        KirElement {
            id: id.to_string(),
            kind: kind.to_string(),
            layer: 2,
            properties: properties
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        }
    }
}
