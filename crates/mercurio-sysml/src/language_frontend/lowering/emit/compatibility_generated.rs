// Generated admission guard for pinned resolved compatibility semantics.
pub(super) const DEFINITION_SHA: &str = "9d78704d2f127466077661a35bed9eed242633f489177b74642701cb5ac23113";
pub(super) fn expression_featuring_supported(kind: &str) -> bool { matches!(kind, "AnalysisCaseUsage" | "AssertConstraintUsage" | "BooleanExpression" | "CalculationUsage" | "CaseUsage" | "CollectExpression" | "ConcernUsage" | "ConstraintUsage" | "ConstructorExpression" | "Expression" | "FeatureChainExpression" | "FeatureReferenceExpression" | "IncludeUseCaseUsage" | "IndexExpression" | "Invariant" | "InvocationExpression" | "LiteralBoolean" | "LiteralExpression" | "LiteralInfinity" | "LiteralInteger" | "LiteralRational" | "LiteralString" | "MetadataAccessExpression" | "NullExpression" | "OperatorExpression" | "RequirementUsage" | "SatisfyRequirementUsage" | "SelectExpression" | "TriggerInvocationExpression" | "UseCaseUsage" | "VerificationCaseUsage" | "ViewpointUsage") }
pub(super) fn expression_default_name(kind: &str, negated: bool) -> Option<&'static str> { match kind {
"BooleanExpression" => Some("Performances::booleanEvaluations"),
"ConstructorExpression" => Some("Performances::constructorEvaluations"),
"Expression" => Some("Performances::evaluations"),
"FeatureReferenceExpression" => Some("Performances::evaluations"),
"Invariant" => Some(if negated { "Performances::falseEvaluations" } else { "Performances::trueEvaluations" }),
"LiteralBoolean" => Some("Performances::literalBooleanEvaluations"),
"LiteralExpression" => Some("Performances::literalEvaluations"),
"LiteralInfinity" => Some("Performances::literalIntegerEvaluations"),
"LiteralInteger" => Some("Performances::literalIntegerEvaluations"),
"LiteralRational" => Some("Performances::literalRationalEvaluations"),
"LiteralString" => Some("Performances::literalStringEvaluations"),
"MetadataAccessExpression" => Some("Performances::metadataAccessEvaluations"),
"NullExpression" => Some("Performances::nullEvaluations"),
_ => None, } }
