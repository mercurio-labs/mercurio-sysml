// Generated from resolved pinned utility/delegate programs and constructor defaults.
pub(super) const DEFINITION_SHA: &str = "fa6ac52ab37c27ce65df5ff1084c92d66b03d57c4fd6df479d9d0d90a5a0be51";
pub(super) fn operator_default(kind: &str)->Option<Option<&'static str>> { match kind {
"CollectExpression" => Some(Some("collect")),
"FeatureChainExpression" => Some(Some(".")),
"IndexExpression" => Some(Some("#")),
"OperatorExpression" => Some(None),
"SelectExpression" => Some(Some("select")),
_ => None, }}
pub(super) fn operator_names(operator: &str)->Vec<String> {
vec![format!("BaseFunctions::'{}'",operator), format!("DataFunctions::'{}'",operator), format!("ControlFunctions::'{}'",operator)]
}
pub(super) fn trigger_name(kind: &str)->Option<&'static str> { match kind {
"after" => Some("Triggers::TriggerAfter"),
"at" => Some("Triggers::TriggerAt"),
"when" => Some("Triggers::TriggerWhen"),
_ => None, }}
