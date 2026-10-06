// Generated bounded dispatch/default recipes; algorithms and prerequisites are handwritten Rust.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(super) enum Selector { Membership, Operator, Trigger }
pub(super) const DEFINITION_SHA: &str = "79b4b89853a9274a3b37de3676134010b4294d82335738b667e9447075b22bbc";
pub(super) fn selector(kind: &str)->Option<Selector> { match kind {
"CollectExpression" => Some(Selector::Operator),
"ConstructorExpression" => Some(Selector::Membership),
"FeatureChainExpression" => Some(Selector::Operator),
"IndexExpression" => Some(Selector::Operator),
"InvocationExpression" => Some(Selector::Membership),
"OperatorExpression" => Some(Selector::Operator),
"SelectExpression" => Some(Selector::Operator),
"TriggerInvocationExpression" => Some(Selector::Trigger),
_ => None, }}
pub(super) const TYPE_BEFORE_SUBSETTING: bool = true;
pub(super) fn base(kind: &str)->Option<&'static str> { match kind {
"CollectExpression" => Some("Performances::evaluations"),
"ConstructorExpression" => Some("Performances::constructorEvaluations"),
"FeatureChainExpression" => Some("Performances::evaluations"),
"IndexExpression" => Some("Performances::evaluations"),
"InvocationExpression" => Some("Performances::evaluations"),
"OperatorExpression" => Some("Performances::evaluations"),
"SelectExpression" => Some("Performances::evaluations"),
"TriggerInvocationExpression" => Some("Performances::evaluations"),
_ => None, }}
pub(super) fn adds_instantiated_general(kind: &str)->bool { matches!(kind, "CollectExpression" | "FeatureChainExpression" | "IndexExpression" | "InvocationExpression" | "OperatorExpression" | "SelectExpression" | "TriggerInvocationExpression") }
pub(super) fn names(kind: &str, composite: bool, structure_owned: bool, behavior_owned: bool)->Option<Vec<&'static str>> {
let mut result=vec![base(kind)?];
match kind {
"CollectExpression" => {
if composite && structure_owned { result.push("Objects::Object::ownedPerformances"); }
if composite && behavior_owned { result.push("Performances::Performance::subperformances"); }
if behavior_owned { result.push("Performances::Performance::enclosedPerformances"); }
},
"ConstructorExpression" => {
if composite && structure_owned { result.push("Objects::Object::ownedPerformances"); }
if composite && behavior_owned { result.push("Performances::Performance::subperformances"); }
if behavior_owned { result.push("Performances::Performance::enclosedPerformances"); }
},
"FeatureChainExpression" => {
if composite && structure_owned { result.push("Objects::Object::ownedPerformances"); }
if composite && behavior_owned { result.push("Performances::Performance::subperformances"); }
if behavior_owned { result.push("Performances::Performance::enclosedPerformances"); }
},
"IndexExpression" => {
if composite && structure_owned { result.push("Objects::Object::ownedPerformances"); }
if composite && behavior_owned { result.push("Performances::Performance::subperformances"); }
if behavior_owned { result.push("Performances::Performance::enclosedPerformances"); }
},
"InvocationExpression" => {
if composite && structure_owned { result.push("Objects::Object::ownedPerformances"); }
if composite && behavior_owned { result.push("Performances::Performance::subperformances"); }
if behavior_owned { result.push("Performances::Performance::enclosedPerformances"); }
},
"OperatorExpression" => {
if composite && structure_owned { result.push("Objects::Object::ownedPerformances"); }
if composite && behavior_owned { result.push("Performances::Performance::subperformances"); }
if behavior_owned { result.push("Performances::Performance::enclosedPerformances"); }
},
"SelectExpression" => {
if composite && structure_owned { result.push("Objects::Object::ownedPerformances"); }
if composite && behavior_owned { result.push("Performances::Performance::subperformances"); }
if behavior_owned { result.push("Performances::Performance::enclosedPerformances"); }
},
"TriggerInvocationExpression" => {
if composite && structure_owned { result.push("Objects::Object::ownedPerformances"); }
if composite && behavior_owned { result.push("Performances::Performance::subperformances"); }
if behavior_owned { result.push("Performances::Performance::enclosedPerformances"); }
},
_ => return None, }
Some(result)
}
