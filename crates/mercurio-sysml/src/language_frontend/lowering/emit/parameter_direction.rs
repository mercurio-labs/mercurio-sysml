//! Handwritten parser initialization over the resolved Ecore operation.
//! Pilot's ParameterMembershipParserPostProcessor invokes parameterDirection
//! for its first owned Feature. Ecore supplies dispatch and the enum domain;
//! these two constant delegate algorithms are explicit native dependencies.
use super::*;

pub(super) fn of(kind: &str) -> Result<&'static str, Diagnostic> {
    let operation = ecore_model::parameter_direction_contract();
    if operation.name != "parameterDirection" || operation.owner != "ParameterMembership"
        || operation.target != "FeatureDirectionKind" || operation.lower != 1 || operation.upper != 1
        || operation.ordered || !operation.unique || !operation.parameters.is_empty()
        || operation.delegate_uri != "http://www.omg.org/spec/SysML"
        || operation.binding_status != "dynamic_invocation_candidates_not_selected" {
        return Err(error("unreviewed parameterDirection invocation contract"));
    }
    let candidates = operation.branches.iter().filter(|(owner, _)| metaclass_conforms(kind, owner))
        .filter(|(owner, _)| !operation.branches.iter().any(|(other, _)|
            other != owner && metaclass_conforms(kind, other) && metaclass_conforms(other, owner)))
        .collect::<Vec<_>>();
    let [(_, implementation)] = candidates.as_slice() else {
        return Err(error("missing or ambiguous parameterDirection branch"));
    };
    let direction = match *implementation {
        "org.omg.sysml.delegate.invocation.ParameterMembership_parameterDirection_InvocationDelegate" => "in",
        "org.omg.sysml.delegate.invocation.ReturnParameterMembership_parameterDirection_InvocationDelegate" => "out",
        _ => return Err(error("unimplemented parameterDirection delegate")),
    };
    ecore_model::validate_value("Feature", "direction", &json!(direction)).map_err(error)?;
    Ok(direction)
}
