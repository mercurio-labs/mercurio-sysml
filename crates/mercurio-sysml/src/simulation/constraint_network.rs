//! Source projection for the bounded scalar constraint-network capability.
//! Semantic identity is resolved here; numerical solving belongs to foundation.
use super::*;
use mercurio_foundation::simulation_core::constraint_network::{
    ConstraintEquation, ConstraintNetworkRequest, ConstraintNetworkResult, solve_constraint_network,
};
use mercurio_foundation::{BinaryExpressionOp, ExpressionIr, ExpressionPathSegment};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceNetworkVariable {
    pub id: String,
    pub label: String,
    pub default_value: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceConstraintNetwork {
    pub scope_id: String,
    pub label: String,
    pub variables: Vec<SourceNetworkVariable>,
    pub equations: Vec<ConstraintEquation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceNetworkSolveRequest {
    pub scope_id: String,
    /// None chooses attributes lacking both a default and an explicit given; an empty list checks all givens.
    #[serde(default)]
    pub unknowns: Option<Vec<String>>,
    #[serde(default)]
    pub given_values: BTreeMap<String, f64>,
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
}
fn default_tolerance() -> f64 {
    1.0e-9
}
fn invalid(message: impl Into<String>) -> SimulationError {
    SimulationError::InvalidProfile(format!("constraint.network: {}", message.into()))
}
fn owner(element: &Element) -> Option<String> {
    string_property_any(element, &["owner", "owning_type"])
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
fn constraints<'a>(runtime: &'a Runtime, scope: &str) -> Vec<&'a Element> {
    runtime
        .graph()
        .elements()
        .iter()
        .filter(|e| {
            owner(e).as_deref() == Some(scope)
                && matches!(construct(e), "ConstraintUsage" | "AssertConstraintUsage")
        })
        .collect()
}

pub fn list_constraint_networks(runtime: &Runtime) -> Vec<(String, String)> {
    let owners = runtime
        .graph()
        .elements()
        .iter()
        .filter(|e| matches!(construct(e), "ConstraintUsage" | "AssertConstraintUsage"))
        .filter_map(owner)
        .collect::<BTreeSet<_>>();
    runtime
        .graph()
        .elements()
        .iter()
        .filter(|e| {
            owners.contains(&e.element_id) && matches!(construct(e), "Package" | "PartDefinition")
        })
        .map(|e| (e.element_id.clone(), element_label(e)))
        .collect()
}

pub fn project_constraint_network(
    runtime: &Runtime,
    scope_id: &str,
) -> Result<SourceConstraintNetwork, SimulationError> {
    let scope = runtime
        .graph()
        .element_by_element_id(scope_id)
        .ok_or_else(|| invalid("unknown scope"))?;
    if !matches!(construct(scope), "Package" | "PartDefinition") {
        return Err(invalid(
            "select a package or part definition containing direct scalar attributes and equalities",
        ));
    }
    let attributes = runtime
        .graph()
        .elements()
        .iter()
        .filter(|e| owner(e).as_deref() == Some(scope_id) && construct(e) == "AttributeUsage")
        .collect::<Vec<_>>();
    let mut variables = Vec::new();
    for attribute in &attributes {
        let ty = attribute.properties.get("type");
        let real = ty.and_then(Value::as_str) == Some("ScalarValues::Real")
            || ty
                .and_then(Value::as_array)
                .is_some_and(|types| types.len() == 1 && types[0] == "ScalarValues::Real");
        if !real || attribute.properties.contains_key("multiplicity") {
            return Err(invalid(format!(
                "{} must be a scalar Real without explicit multiplicity; quantities and integer domains are not supported",
                attribute.element_id
            )));
        }
        let value = adapter::attribute_default_value(attribute).map_err(map_adapter_error)?;
        let default_value = match value {
            Some(value) => Some(
                value
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| invalid("a given must be a finite scalar number"))?,
            ),
            None if attribute.properties.contains_key("expression_ir") => {
                return Err(invalid(format!(
                    "{} has a dependent default; this profile requires a closed default or an attribute without an initializer",
                    attribute.element_id
                )));
            }
            None => None,
        };
        variables.push(SourceNetworkVariable {
            id: attribute.element_id.clone(),
            label: element_label(attribute),
            default_value,
        });
    }
    let identities = attributes
        .iter()
        .map(|e| e.element_id.clone())
        .collect::<BTreeSet<_>>();
    let mut equations = Vec::new();
    for constraint in constraints(runtime, scope_id) {
        let raw = constraint.properties.get("expression_ir").ok_or_else(|| {
            invalid(format!(
                "{} has no executable expression",
                constraint.element_id
            ))
        })?;
        let mut expression = ExpressionIr::from_value(raw).map_err(|e| invalid(e.to_string()))?;
        while let ExpressionIr::Checked {
            expression: inner, ..
        } = expression
        {
            expression = *inner;
        }
        let ExpressionIr::Binary {
            left,
            op: BinaryExpressionOp::Equal,
            right,
        } = expression
        else {
            return Err(invalid(format!(
                "{} must be an equality",
                constraint.element_id
            )));
        };
        let mut left = *left;
        let mut right = *right;
        bind_paths(&mut left, &identities)?;
        bind_paths(&mut right, &identities)?;
        equations.push(ConstraintEquation {
            id: constraint.element_id.clone(),
            left,
            right,
        });
    }
    if equations.is_empty() {
        return Err(invalid(
            "the selected scope has no direct equality constraints",
        ));
    }
    Ok(SourceConstraintNetwork {
        scope_id: scope_id.into(),
        label: element_label(scope),
        variables,
        equations,
    })
}

fn bind_paths(
    expression: &mut ExpressionIr,
    allowed: &BTreeSet<String>,
) -> Result<(), SimulationError> {
    match expression {
        ExpressionIr::Path { segments, .. } => {
            let [
                ExpressionPathSegment::Resolved {
                    feature: Some(feature),
                    ..
                },
            ] = segments.as_slice()
            else {
                return Err(invalid(
                    "only direct, resolved scalar attribute references are supported",
                ));
            };
            if !allowed.contains(feature) {
                return Err(invalid(format!(
                    "reference outside selected network: {feature}"
                )));
            }
            *segments = vec![ExpressionPathSegment::Name(feature.clone())];
        }
        ExpressionIr::Unary { expr, .. }
        | ExpressionIr::Checked {
            expression: expr, ..
        } => bind_paths(expr, allowed)?,
        ExpressionIr::Binary { left, right, .. } => {
            bind_paths(left, allowed)?;
            bind_paths(right, allowed)?;
        }
        ExpressionIr::Call { args, .. } | ExpressionIr::Tuple { items: args } => {
            for expr in args {
                bind_paths(expr, allowed)?;
            }
        }
        ExpressionIr::Literal { .. } => {}
        _ => {
            return Err(invalid(
                "invocation, self and non-scalar binding constructs are not supported in constraint networks",
            ));
        }
    }
    Ok(())
}

pub fn solve_source_constraint_network(
    runtime: &Runtime,
    request: SourceNetworkSolveRequest,
) -> Result<ConstraintNetworkResult, SimulationError> {
    let network = project_constraint_network(runtime, &request.scope_id)?;
    let variables = network
        .variables
        .iter()
        .map(|v| v.id.clone())
        .collect::<BTreeSet<_>>();
    let unknowns = request.unknowns.unwrap_or_else(|| {
        network
            .variables
            .iter()
            .filter(|v| v.default_value.is_none() && !request.given_values.contains_key(&v.id))
            .map(|v| v.id.clone())
            .collect()
    });
    if unknowns.iter().any(|id| !variables.contains(id))
        || request
            .given_values
            .keys()
            .any(|id| !variables.contains(id))
    {
        return Err(invalid(
            "all given and unknown identities must belong to the selected network",
        ));
    }
    let mut knowns = network
        .variables
        .iter()
        .filter_map(|v| v.default_value.map(|value| (v.id.clone(), value)))
        .filter(|(id, _)| !unknowns.contains(id))
        .collect::<BTreeMap<_, _>>();
    knowns.extend(request.given_values);
    Ok(solve_constraint_network(&ConstraintNetworkRequest {
        knowns,
        unknowns,
        equations: network.equations,
        tolerance: request.tolerance,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile_sysml_text, load_sysml_baseline};
    const SOURCE: &str = "package Budget { import ScalarValues::*; attribute total: Real = 10.0; attribute offset: Real = 2.0; attribute x: Real; attribute y: Real; constraint sum { x + y == total } constraint difference { 2.0*x - y == offset } }";
    fn runtime(source: &str) -> Runtime {
        Runtime::from_document(
            compile_sysml_text(source, "network.sysml", &load_sysml_baseline().unwrap()).unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn solves_coupled_source_equalities_and_reverses_knowns() {
        let runtime = runtime(SOURCE);
        let network = project_constraint_network(&runtime, "pkg.Budget").unwrap();
        assert_eq!(network.equations.len(), 2);
        let request = SourceNetworkSolveRequest {
            scope_id: "pkg.Budget".into(),
            unknowns: None,
            given_values: BTreeMap::new(),
            tolerance: 1e-9,
        };
        let result = solve_source_constraint_network(&runtime, request).unwrap();
        assert_eq!(result.values["feature.Budget.x"], 4.0);
        assert_eq!(result.values["feature.Budget.y"], 6.0);
        let reverse = solve_source_constraint_network(
            &runtime,
            SourceNetworkSolveRequest {
                scope_id: "pkg.Budget".into(),
                unknowns: Some(vec![
                    "feature.Budget.total".into(),
                    "feature.Budget.y".into(),
                ]),
                given_values: BTreeMap::from([("feature.Budget.x".into(), 4.0)]),
                tolerance: 1e-9,
            },
        )
        .unwrap();
        assert_eq!(reverse.values["feature.Budget.total"], 10.0);
        assert_eq!(reverse.values["feature.Budget.y"], 6.0);
    }
    #[test]
    fn rejects_foreign_variables_and_unsupported_domains() {
        let runtime = runtime(SOURCE);
        assert!(
            solve_source_constraint_network(
                &runtime,
                SourceNetworkSolveRequest {
                    scope_id: "pkg.Budget".into(),
                    unknowns: Some(vec!["foreign".into()]),
                    given_values: BTreeMap::new(),
                    tolerance: 1e-9
                }
            )
            .is_err()
        );
        assert!(
            project_constraint_network(
                &self::runtime(&SOURCE.replace("attribute x: Real", "attribute x: Integer")),
                "pkg.Budget"
            )
            .is_err()
        );
    }
    #[test]
    fn automatic_unknowns_exclude_explicit_givens() {
        let runtime = runtime(SOURCE);
        let result = solve_source_constraint_network(
            &runtime,
            SourceNetworkSolveRequest {
                scope_id: "pkg.Budget".into(),
                unknowns: None,
                given_values: BTreeMap::from([("feature.Budget.x".into(), 4.0)]),
                tolerance: 1e-9,
            },
        )
        .unwrap();
        assert_eq!(result.values["feature.Budget.y"], 6.0);
        assert_eq!(result.values["feature.Budget.x"], 4.0);
    }

    #[test]
    fn source_network_reports_contradictions_and_rank_deficiency_without_unknown_values() {
        use mercurio_foundation::simulation_core::constraint_network::ConstraintNetworkStatus;
        let request = || SourceNetworkSolveRequest {
            scope_id: "pkg.Budget".into(),
            unknowns: None,
            given_values: BTreeMap::new(),
            tolerance: 1e-9,
        };
        let inconsistent = runtime(&SOURCE.replace(
            "constraint difference { 2.0*x - y == offset }",
            "constraint difference { x + y == offset }",
        ));
        let result = solve_source_constraint_network(&inconsistent, request()).unwrap();
        assert_eq!(result.status, ConstraintNetworkStatus::Inconsistent);
        assert!(!result.values.contains_key("feature.Budget.x"));
        assert!(!result.values.contains_key("feature.Budget.y"));
        assert_eq!(result.values["feature.Budget.total"], 10.0);
        let underdetermined =
            runtime(&SOURCE.replace("constraint difference { 2.0*x - y == offset }", ""));
        let result = solve_source_constraint_network(&underdetermined, request()).unwrap();
        assert_eq!(result.status, ConstraintNetworkStatus::Underdetermined);
        assert!(!result.values.contains_key("feature.Budget.x"));
        assert!(!result.values.contains_key("feature.Budget.y"));
        assert_eq!(result.values["feature.Budget.total"], 10.0);
        let nonlinear = runtime(&SOURCE.replace("2.0*x - y", "x*x - y"));
        let result = solve_source_constraint_network(&nonlinear, request()).unwrap();
        assert_eq!(result.status, ConstraintNetworkStatus::Unsupported);
        assert!(!result.values.contains_key("feature.Budget.x"));
        assert!(!result.values.contains_key("feature.Budget.y"));
        assert_eq!(result.values["feature.Budget.total"], 10.0);
    }
}

/// Opt-in source projection for networks that execute inside simulation cycles.
/// All direct equalities on the annotated type belong to one simultaneous system.
pub(super) fn simulation_networks(runtime: &Runtime) -> Result<Vec<mercurio_foundation::simulation_core::SimulationConstraintNetwork>, SimulationError> {
    use mercurio_foundation::simulation_core::SimulationConstraintNetwork;
    let mut networks = Vec::new();
    for scope in runtime.graph().elements() {
        let Some(properties) = adapter::mission_metadata::properties(runtime, scope, "ConstraintNetwork", &["unknowns", "tolerance"])
            .map_err(map_adapter_error)? else { continue; };
        if construct(scope) != "PartDefinition" {
            return Err(invalid("dynamic ConstraintNetwork metadata requires a part definition"));
        }
        let projected = project_constraint_network(runtime, &scope.element_id)?;
        let names = properties.get("unknowns").and_then(Value::as_str)
            .ok_or_else(|| invalid("ConstraintNetwork.unknowns must list comma-separated direct attribute names"))?;
        let mut unknowns = Vec::new();
        for name in names.split(',').map(str::trim) {
            let matching = projected.variables.iter().filter(|variable| variable.label == name).collect::<Vec<_>>();
            if matching.len() != 1 || unknowns.contains(&matching[0].id) {
                return Err(invalid(format!("unknown must resolve once to a direct Real attribute: {name}")));
            }
            unknowns.push(matching[0].id.clone());
        }
        let tolerance = if properties.contains_key("tolerance") {
            adapter::mission_metadata::number(properties, "tolerance").map_err(map_adapter_error)?
        } else { default_tolerance() };
        if tolerance <= 0.0 || tolerance >= 1.0 { return Err(invalid("network tolerance must be greater than zero and less than one")); }
        for subject in runtime.graph().elements().iter().filter(|candidate| ["type","definition"].iter().any(|key| {
            candidate.properties.get(*key).is_some_and(|value| value.as_str() == Some(scope.element_id.as_str()) || value.as_array().is_some_and(|items| items.iter().any(|value| value.as_str() == Some(scope.element_id.as_str()))))
        })) {
            networks.push(SimulationConstraintNetwork {
                id: format!("{}@{}", scope.element_id, subject.element_id), subject_id: subject.element_id.clone(),
                variables: projected.variables.iter().map(|variable| (variable.id.clone(), variable.label.clone())).collect(),
                unknowns: unknowns.clone(), equations: projected.equations.clone(), tolerance,
            });
        }
    }
    Ok(networks)
}
