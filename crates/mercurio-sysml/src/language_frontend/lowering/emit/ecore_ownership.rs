//! Handwritten construction algorithm parameterized by checked Ecore contracts.
//! Only initial attachment is supported; reparenting requires an explicit removal.
use super::*;
use super::super::relationship_declarations::metaclass_is;
use super::super::ecore_model::{self, FeatureKind};

pub(super) struct OwnershipContract {
    pub owner_kind: &'static str,
    pub child_kind: &'static str,
    pub field: &'static str,
    pub inverse: &'static str,
}

/// Validate before modifying either endpoint. Preserve insertion order, suppress
/// duplicates, and reject conflicting ownership rather than orphaning old links.
pub(super) fn attach(
    contract: &OwnershipContract, owner: &mut KirElement, child: &mut KirElement,
) -> Result<(), Diagnostic> {
    let invalid = |message: &str| Diagnostic::new(format!("Ecore ownership {}: {message}", contract.field), None);
    if owner.id == child.id || owner.id.is_empty() || child.id.is_empty() {
        return Err(invalid("invalid endpoint identity"));
    }
    // Existing KIR uses this legacy kind for VerifyUsage. The pinned Xtext
    // RequirementVerificationUsage rule returns SysML::RequirementUsage.
    // Keep this exact compatibility binding local; unknown kinds still fail.
    fn canonical(kind: &str) -> &str { match kind {
        "SysML::Requirements::VerifyRequirementUsage" => super::ecore_ownership_generated::LEGACY_VERIFY_USAGE_METACLASS,
        other => other,
    }}
    let owner_kind = canonical(&owner.kind);
    let child_kind = canonical(&child.kind);
    if !metaclass_is(owner_kind, contract.owner_kind)? || !metaclass_is(child_kind, contract.child_kind)? {
        return Err(invalid("endpoint metaclass does not conform"));
    }
    let owner_feature = ecore_model::feature(owner_kind, contract.field)
        .ok_or_else(|| invalid("missing pinned containment feature"))?;
    let inverse_feature = ecore_model::feature(child_kind, contract.inverse)
        .ok_or_else(|| invalid("missing pinned inverse feature"))?;
    if owner_feature.kind != FeatureKind::Reference || !owner_feature.containment ||
        inverse_feature.kind != FeatureKind::Reference || !inverse_feature.container {
        return Err(invalid("features are not reciprocal Ecore containment"));
    }
    ecore_model::validate_reference_endpoint(owner_kind, contract.field, child_kind)
        .map_err(|reason| invalid(&reason))?;
    ecore_model::validate_reference_endpoint(child_kind, contract.inverse, owner_kind)
        .map_err(|reason| invalid(&reason))?;
    let mut values = match owner.properties.get(contract.field) {
        None => Vec::new(),
        Some(Value::Array(values)) if values.iter().all(|v| v.as_str().is_some_and(|s| !s.is_empty())) => values.clone(),
        _ => return Err(invalid("containment must be a reference list")),
    };
    for field in ["owning_relationship", "owning_related_element"] {
        match child.properties.get(field) {
            None | Some(Value::Null) => {},
            Some(Value::String(id)) if field == contract.inverse && id == &owner.id => {},
            _ => return Err(invalid("child already has conflicting containment")),
        }
    }
    if !values.iter().any(|v| v.as_str() == Some(&child.id)) { values.push(json!(child.id)); }
    ecore_model::validate_value(owner_kind, contract.field, &Value::Array(values.clone()))
        .map_err(|reason| invalid(&reason))?;
    ecore_model::validate_value(child_kind, contract.inverse, &json!(owner.id))
        .map_err(|reason| invalid(&reason))?;
    owner.properties.insert(contract.field.into(), Value::Array(values));
    child.properties.insert(contract.inverse.into(), json!(owner.id));
    Ok(())
}

/// Attach a newly constructed relationship to its already-emitted owner. Only
/// the anonymous resource namespace may be materialized here; other missing
/// owners indicate an emission-order or identity error.
pub(super) fn attach_to_owner(
    owner_id: &str, relationship: &mut KirElement, elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    if owner_id == "pkg.root" && !elements.iter().any(|e| e.id == owner_id) {
        let mut metadata = relationship.properties.get("metadata").cloned().unwrap_or_else(|| json!({}));
        if !metadata.is_object() {
            return Err(Diagnostic::new("invalid relationship metadata for resource namespace", None));
        }
        metadata["generated"] = json!(true);
        metadata["source_span"] = json!({"start_line":1,"start_col":1,"end_line":1,"end_col":1});
        metadata.as_object_mut().unwrap().remove("lowering");
        let mut root = KirElement { id: owner_id.into(), kind: "SysML::Namespace".into(), layer: 2,
            properties: BTreeMap::from([("metadata".into(), metadata)]) };
        attach(&super::ecore_ownership_generated::OWNED_RELATIONSHIPS, &mut root, relationship)?;
        elements.push(root);
        return Ok(());
    }
    let owner = elements.iter_mut().find(|e| e.id == owner_id)
        .ok_or_else(|| Diagnostic::new(format!("missing Ecore relationship owner {owner_id}"), None))?;
    attach(&super::ecore_ownership_generated::OWNED_RELATIONSHIPS, owner, relationship)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ecore_ownership_generated::{OWNED_ELEMENTS, OWNED_RELATIONSHIPS};
    fn element(id: &str, kind: &str) -> KirElement {
        KirElement { id: id.into(), kind: format!("SysML::{kind}"), layer: 2, properties: BTreeMap::new() }
    }
    #[test]
    fn ecore_ownership_preserves_order_uniqueness_and_both_opposites() {
        let mut package = element("p", "Package");
        let mut membership = element("m", "OwningMembership");
        let mut first = element("a", "Class");
        let mut second = element("b", "Class");
        attach(&OWNED_RELATIONSHIPS, &mut package, &mut membership).unwrap();
        attach(&OWNED_ELEMENTS, &mut membership, &mut first).unwrap();
        attach(&OWNED_ELEMENTS, &mut membership, &mut second).unwrap();
        attach(&OWNED_ELEMENTS, &mut membership, &mut first).unwrap();
        assert_eq!(package.properties["owned_relationship"], json!(["m"]));
        assert_eq!(membership.properties["owning_related_element"], "p");
        assert_eq!(membership.properties["owned_related_element"], json!(["a", "b"]));
        assert_eq!(first.properties["owning_relationship"], "m");
        assert_eq!(second.properties["owning_relationship"], "m");
    }
    #[test]
    fn ecore_ownership_explicit_legacy_verification_binding() {
        let mut membership = element("m", "OwningMembership");
        let mut legacy = element("v", "Requirements::VerifyRequirementUsage");
        attach(&OWNED_ELEMENTS, &mut membership, &mut legacy).unwrap();
        let mut unknown = element("u", "UnknownRequirementUsage");
        assert!(attach(&OWNED_ELEMENTS, &mut membership, &mut unknown).is_err());
        assert_eq!(membership.properties["owned_related_element"], json!(["v"]));
    }
    #[test]
    fn ecore_ownership_rejects_conflicts_without_partial_mutation() {
        for invalid in [json!("other"), json!(["other"]), json!(3)] {
            let mut owner = element("m", "Annotation");
            let mut child = element("c", "Comment");
            child.properties.insert("owning_relationship".into(), invalid);
            let before = child.properties.clone();
            assert!(attach(&OWNED_ELEMENTS, &mut owner, &mut child).is_err());
            assert!(owner.properties.is_empty());
            assert_eq!(child.properties, before);
        }
        let mut owner = element("p", "Package");
        let mut child = element("c", "Class");
        assert!(attach(&OWNED_ELEMENTS, &mut owner, &mut child).is_err());
        assert!(owner.properties.is_empty());
        let mut relation = element("r", "Relationship");
        relation.properties.insert("owned_related_element".into(), json!(["c", "c"]));
        assert!(attach(&OWNED_ELEMENTS, &mut relation, &mut child).is_err());
        assert!(child.properties.is_empty());
    }
}
