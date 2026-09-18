//! Executable scalar contracts. Unspecified multiplicity is not assumed scalar.
use mercurio_foundation::kir::{ExpressionContract, ExpressionMultiplicity, ExpressionValueType};
use mercurio_foundation::language_contracts::diagnostics::Diagnostic;
use super::ir::ResolvedUsage;

pub(super) fn usage_expression_contract(usage: &ResolvedUsage) -> Result<Option<ExpressionContract>, Diagnostic> {
    if matches!(usage.construct.as_str(), "ConstraintUsage" | "TransitionUsage")
        || (usage.construct == "RequireUsage" && usage.modifiers.iter().any(|m| m == "constraint")) {
        return Ok(Some(ExpressionContract {
            value_type: ExpressionValueType::Boolean,
            multiplicity: Some(ExpressionMultiplicity { lower: 1, upper: Some(1) }),
        }));
    }
    let value_type = match usage.type_ref.as_deref() {
        Some("ScalarValues::Boolean") => ExpressionValueType::Boolean,
        Some("ScalarValues::String") => ExpressionValueType::String,
        Some("ScalarValues::Integer") => ExpressionValueType::Integer,
        Some("ScalarValues::Natural") => ExpressionValueType::Natural,
        Some("ScalarValues::Positive") => ExpressionValueType::Positive,
        Some("ScalarValues::Real") => ExpressionValueType::Real,
        _ => ExpressionValueType::Any,
    };
    let multiplicity = usage.multiplicity.as_ref().map(|bounds| {
        ExpressionMultiplicity::from_bounds(&bounds.lower, &bounds.upper)
            .map_err(|error| Diagnostic::new(format!("{}: {error}", usage.qualified_name), Some(usage.span.clone())))
    }).transpose()?;
    if value_type == ExpressionValueType::Any && multiplicity.is_none() {
        return Ok(None);
    }
    Ok(Some(ExpressionContract { value_type, multiplicity }))
}
