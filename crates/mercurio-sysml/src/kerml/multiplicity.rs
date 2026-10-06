//! Native derivation of the Ecore MultiplicityRange bound references.

use mercurio_foundation::kir::KirElement;

/// Ordered owned bound expressions. The historical name also covers the
/// pinned feature-reference arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralBounds {
    pub bound: Vec<String>,
    pub lower_bound: Option<String>,
    pub upper_bound: String,
}

/// Reproduce the pinned lower/upper/bound setting delegates over owned
/// Expression members. Evaluation of referenced values is separate.
pub fn derived_literal_bounds(range: &KirElement, elements: &[KirElement]) -> Option<LiteralBounds> {
    if range.kind.rsplit("::").next()? != "MultiplicityRange" { return None; }
    let members = range.properties.get("members")?.as_array()?;
    let owned_relationships = range.properties.get("owned_relationship")?.as_array()?;
    let bound = members.iter().filter_map(|member| {
        let id = member.as_str()?;
        let element = elements.iter().find(|element| element.id == id)?;
        if !crate::language_frontend::lowering::relationship_declarations::metaclass_is(&element.kind, "Expression").ok()? {
            return None;
        }
        let membership_id = element.properties.get("owning_membership")?.as_str()?;
        if !owned_relationships.iter().any(|relationship| relationship.as_str() == Some(membership_id)) {
            return None;
        }
        let membership = elements.iter().find(|element| element.id == membership_id)?;
        (membership.kind == "SysML::OwningMembership"
            && membership.properties.get("member_element")?.as_str()? == id
            && membership.properties.get("owning_related_element")?.as_str()? == range.id)
            .then(|| id.to_owned())
    }).take(2).collect::<Vec<_>>();
    if bound.is_empty() { return None; }
    Some(LiteralBounds {
        lower_bound: (bound.len() == 2).then(|| bound[0].clone()),
        upper_bound: bound.last()?.clone(),
        bound,
    })
}

/// Native bounded implementation of MultiplicityRange::valueOf for direct
/// integer/infinity literals. Feature-reference evaluation requires the
/// Expression::evaluate dependency and deliberately returns its null sentinel.
pub fn derived_bound_value(range: &KirElement, bound_id: Option<&str>, elements: &[KirElement]) -> i32 {
    let Some(bound_id) = bound_id else { return -2 };
    let Some(bounds) = derived_literal_bounds(range, elements) else { return -2 };
    if !bounds.bound.iter().any(|id| id == bound_id) { return -2; }
    let Some(bound) = elements.iter().find(|element| element.id == bound_id) else { return -2 };
    match bound.kind.as_str() {
        "SysML::LiteralInfinity" => -1,
        "SysML::LiteralInteger" => bound.properties.get("value")
            .and_then(|value| value.as_i64())
            .and_then(|value| i32::try_from(value).ok())
            .filter(|value| *value >= 0)
            .unwrap_or(-2),
        _ => -2,
    }
}

/// Native bounded implementation of MultiplicityRange::hasBounds. The
/// valueOf sentinel for unsupported reference evaluation preserves the pinned
/// delegate's false result unless a caller asks for that sentinel explicitly.
pub fn derived_has_bounds(range: &KirElement, lower: i32, upper: i32, elements: &[KirElement]) -> bool {
    let Some(bounds) = derived_literal_bounds(range, elements) else { return false };
    if derived_bound_value(range, Some(&bounds.upper_bound), elements) != upper { return false; }
    let lower_value = derived_bound_value(range, bounds.lower_bound.as_deref(), elements);
    lower_value == lower || (lower_value < -1 && (lower == upper || lower == 0 && upper == -1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn derived_bounds_follow_first_two_owned_expression_members() {
        let range = KirElement { id: "range".into(), kind: "SysML::MultiplicityRange".into(), layer: 2,
            properties: BTreeMap::from([
                ("members".into(), json!(["first", "second", "third"])),
                ("owned_relationship".into(), json!(["first.member", "second.member", "third.member"])),
            ]) };
        let mut elements = Vec::new();
        for (id, kind) in [("first", "LiteralInteger"), ("second", "Expression"), ("third", "LiteralInfinity")] {
            elements.push(KirElement { id: id.into(), kind: format!("SysML::{kind}"), layer: 2,
                properties: BTreeMap::from([("owning_membership".into(), json!(format!("{id}.member")))]) });
            elements.push(KirElement { id: format!("{id}.member"), kind: "SysML::OwningMembership".into(), layer: 2,
                properties: BTreeMap::from([
                    ("member_element".into(), json!(id)),
                    ("owning_related_element".into(), json!(range.id)),
                ]) });
        }
        let bounds = derived_literal_bounds(&range, &elements).unwrap();
        assert_eq!(bounds.bound, ["first", "second"]);
        assert_eq!(bounds.lower_bound.as_deref(), Some("first"));
        assert_eq!(bounds.upper_bound, "second");
    }
}
