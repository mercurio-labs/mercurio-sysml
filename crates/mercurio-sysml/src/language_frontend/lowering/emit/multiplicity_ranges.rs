//! Native bound arm of pinned KerML MultiplicityBounds.
//! Pilot accepts all five LiteralExpression syntax alternatives, but its
//! transform requires a Natural value before it constructs non-Natural bounds.
use super::*;

pub(super) fn emit(
    usage: &ResolvedUsage,
    range: &mut KirElement,
    elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    if usage.construct != "MultiplicityRange" { return Ok(()); }
    let raw = usage.metadata_properties.get("__multiplicity_range_bounds")
        .ok_or_else(|| Diagnostic::new("missing MultiplicityRange bounds", Some(usage.span.clone())))?;
    let references: Vec<Option<String>> = serde_json::from_str(usage.metadata_properties.get("__multiplicity_range_reference_ids")
        .ok_or_else(|| Diagnostic::new("missing resolved MultiplicityRange bound references", Some(usage.span.clone())))?)
        .map_err(|error| Diagnostic::new(format!("invalid MultiplicityRange bound references: {error}"), Some(usage.span.clone())))?;
    emit_bounds(raw, &references, &usage.span, range, elements)
}

pub(super) fn emit_bounds(
    raw: &str,
    references: &[Option<String>],
    span: &SourceSpan,
    range: &mut KirElement,
    elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    let values = crate::parser::split_multiplicity_bound_text(raw);
    if values.is_empty() || values.len() > 2 {
        return Err(Diagnostic::new("invalid MultiplicityRange bound count", Some(span.clone())));
    }
    if references.len() != values.len() {
        return Err(Diagnostic::new("MultiplicityRange bound/reference count mismatch", Some(span.clone())));
    }
    // The grammar owns membership/expression construction and attribute values.
    // Scope/link resolution and derived membership enrichment below remain
    // explicit handwritten services, using already-resolved feature identities.
    let tree = crate::xtext_fragment::multiplicity_tree(raw)
        .map_err(|message| Diagnostic::new(message, Some(span.clone())))?;
    let bounds = tree.children.get("owned_relationship")
        .filter(|items| items.len() == values.len())
        .ok_or_else(|| Diagnostic::new("MultiplicityBounds construction/count mismatch", Some(span.clone())))?;
    let mut expressions = Vec::new();
    for (index, value) in values.iter().enumerate() {
        let bound = &bounds[index];
        let membership_kind = bound.object_kind.as_deref().ok_or_else(|| Diagnostic::new("Missing bound membership class", Some(span.clone())))?;
        let constructed = bound.children.get("owned_related_element")
            .filter(|children| children.len() == 1).and_then(|children| children.first())
            .ok_or_else(|| Diagnostic::new("Missing constructed bound expression", Some(span.clone())))?;
        let kind = constructed.object_kind.as_deref().ok_or_else(|| Diagnostic::new("Missing bound expression class", Some(span.clone())))?;
        match kind {
            "LiteralBoolean" | "LiteralString" | "LiteralRational" => {
                return Err(Diagnostic::new("Must have a Natural value", Some(span.clone())));
            }
            "LiteralInteger" | "LiteralInfinity" if references[index].is_none() => {},
            "FeatureReferenceExpression" if references[index].is_some() => {
                let pending = constructed.children.get("owned_relationship")
                    .filter(|children| children.len() == 1).and_then(|children| children.first())
                    .and_then(|member| member.links.get("member_element"))
                    .filter(|links| links.len() == 1).and_then(|links| links.first())
                    .ok_or_else(|| Diagnostic::new("Missing typed bound reference", Some(span.clone())))?;
                if pending.spelling != *value || !pending.target_type.ends_with("#//Feature") {
                    return Err(Diagnostic::new("Bound syntax/linker contract mismatch", Some(span.clone())));
                }
            }
            _ => return Err(Diagnostic::new("Unsupported bound construction/link state", Some(span.clone()))),
        }
        let expression_id = format!("{}.bound.{index}", range.id);
        let membership_id = format!("{expression_id}.membership");
        let mut expression_properties = BTreeMap::from([
            ("owner".into(), json!(range.id)),
            ("owning_namespace".into(), json!(range.id)),
            ("owning_membership".into(), json!(membership_id)),
        ]);
        expression_properties.extend(constructed.fields.clone());
        copy_metadata(range, &mut expression_properties, kind);
        let mut expression = KirElement {
            id: expression_id.clone(), kind: format!("SysML::{kind}"), layer: 2,
            properties: expression_properties,
        };
        let mut reference_member = if let Some(target) = &references[index] {
            let id = format!("{expression_id}.reference");
            let mut properties = BTreeMap::from([
                ("membership_owning_namespace".into(), json!(expression_id)),
                ("member_element".into(), json!(target)),
                ("source".into(), json!([expression_id])),
                ("target".into(), json!([target])),
                ("related_element".into(), json!([expression_id, target])),
                ("is_implied".into(), json!(false)),
            ]);
            copy_metadata(&expression, &mut properties, "Membership");
            let mut member = KirElement { id: id.clone(), kind: "SysML::Membership".into(), layer: 2, properties };
            ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, &mut expression, &mut member)?;
            append_unique_property_ref_list(&mut expression.properties, "membership", &id);
            Some(member)
        } else { None };
        let mut membership_properties = BTreeMap::from([
            ("membership_owning_namespace".into(), json!(range.id)),
            ("member_element".into(), json!(expression_id)),
            ("owned_member_element".into(), json!(expression_id)),
            ("source".into(), json!([range.id])),
            ("target".into(), json!([expression_id])),
            ("related_element".into(), json!([range.id, expression_id])),
            ("visibility".into(), json!("public")),
            ("is_implied".into(), Value::Bool(false)),
        ]);
        copy_metadata(range, &mut membership_properties, membership_kind);
        let mut membership = KirElement {
            id: membership_id.clone(), kind: format!("SysML::{membership_kind}"), layer: 2,
            properties: membership_properties,
        };
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut membership, &mut expression)?;
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, range, &mut membership)?;
        append_unique_property_ref_list(&mut range.properties, "owned_membership", &membership_id);
        append_unique_property_ref_list(&mut range.properties, "membership", &membership_id);
        expressions.push(expression_id);
        elements.push(membership);
        elements.push(expression);
        if let Some(member) = reference_member.take() { elements.push(member); }
    }
    // MultiplicityBounds precedes TypeBody in KerML.xtext. Preserve that order
    // even when the generic lowering has already attached body members.
    let mut members = expressions.iter().map(|id| json!(id)).collect::<Vec<_>>();
    if let Some(body_members) = range.properties.get("members").and_then(Value::as_array) {
        for id in body_members {
            if !members.contains(id) { members.push(id.clone()); }
        }
    }
    range.properties.insert("members".into(), json!(members));
    let derived = crate::kerml::derived_literal_bounds(range, elements)
        .ok_or_else(|| Diagnostic::new("cannot derive MultiplicityRange bounds", Some(span.clone())))?;
    debug_assert_eq!(derived.bound, expressions);
    let lower = if values.len() == 2 { values[0] } else if values[0] == "*" { "0" } else { values[0] };
    range.properties.insert("multiplicity_lower".into(), json!(lower));
    range.properties.insert("multiplicity_upper".into(), json!(values[values.len() - 1]));
    Ok(())
}

fn copy_metadata(owner: &KirElement, properties: &mut BTreeMap<String, Value>, kind: &str) {
    let mut metadata = owner.properties.get("metadata").cloned().unwrap_or_else(|| json!({}));
    metadata["lowering"] = json!({"construct":kind,"metaclass":format!("SysML::{kind}")});
    properties.insert("metadata".into(), metadata);
}
