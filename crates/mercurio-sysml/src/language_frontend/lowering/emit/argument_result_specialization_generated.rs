// Generated reviewed bindings from pinned Ecore declarations; execution is handwritten.
pub(super) struct Rule { pub(super) unless_library: Option<&'static str>, pub(super) normative_rule: &'static str }
pub(super) fn rule(kind: &str) -> Option<Rule> { match kind {
    "IndexExpression" => Some(Rule { unless_library: Some("Collections::Collection"), normative_rule: "checkIndexExpressionResultSpecialization" }),
    "SelectExpression" => Some(Rule { unless_library: None, normative_rule: "checkSelectExpressionResultSpecialization" }),
    _ => None,
} }
pub(super) const BINDINGS_SHA256: &str = "260fda2456f04c9a2347c315f7dd87029b2a9b73f8950f329f0f769c3b9541e9";
