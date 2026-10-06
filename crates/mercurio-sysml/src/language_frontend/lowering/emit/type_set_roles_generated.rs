// Generated from resolved Ecore/Xtext by tools/generate_type_set_roles.py.
// Execution of delegates and normative predicates is explicit handwritten Rust.
pub struct Binding {pub kind: &'static str, pub source: &'static str, pub derived_source: bool, pub target: &'static str, pub owning: &'static str, pub owned: &'static str, pub endpoints: Option<&'static str>, pub count_constraint: Option<&'static str>, pub self_constraint: Option<&'static str>}
pub const BINDINGS: &[Binding] = &[
    Binding {kind: "Differencing", source: "type_differenced", derived_source: true, target: "differencing_type", owning: "type_differenced", owned: "owned_differencing", endpoints: Some("differencing_type"), count_constraint: Some("validateTypeOwnedDifferencingNotOne"), self_constraint: Some("validateTypeDifferencingTypesNotSelf")},
    Binding {kind: "Disjoining", source: "type_disjoined", derived_source: false, target: "disjoining_type", owning: "owning_type", owned: "owned_disjoining", endpoints: None, count_constraint: None, self_constraint: None},
    Binding {kind: "Intersecting", source: "type_intersected", derived_source: true, target: "intersecting_type", owning: "type_intersected", owned: "owned_intersecting", endpoints: Some("intersecting_type"), count_constraint: Some("validateTypeOwnedIntersectingNotOne"), self_constraint: Some("validateTypeIntersectingTypesNotSelf")},
    Binding {kind: "Unioning", source: "type_unioned", derived_source: true, target: "unioning_type", owning: "type_unioned", owned: "owned_unioning", endpoints: Some("unioning_type"), count_constraint: Some("validateTypeOwnedUnioningNotOne"), self_constraint: Some("validateTypeUnioningTypesNotSelf")},
];
pub fn binding(kind:&str)->Option<&'static Binding> {let kind=kind.rsplit("::").next().unwrap_or(kind);BINDINGS.iter().find(|b|b.kind==kind)}
