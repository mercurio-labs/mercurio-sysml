//! Native construction of bounded KerML Flow declaration graphs.
use super::*;
use crate::language_frontend::lowering::relationship_declarations::{self, Operand};

pub(super) fn emit(
    usage: &ResolvedUsage,
    flow: &mut KirElement,
    elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    if !matches!(usage.construct.as_str(), "Flow" | "SuccessionFlow") { return Ok(()); }
    flow.properties.insert("is_sufficient".into(), json!(
        usage.modifiers.iter().any(|modifier| modifier == "is_sufficient")
    ));
    emit_literal_value_part(usage, flow, elements)?;
    emit_typed_payload(usage, flow, elements)?;
    let Some(ends) = relationship_declarations::load::<Operand<String>>(
        &usage.metadata_properties, &usage.span,
    )? else { return Ok(()); };
    if ends.sources.len() != 1 || ends.targets.len() != 1 {
        return Err(Diagnostic::new("Flow requires one source and one target end", Some(usage.span.clone())));
    }
    for (index, operand) in [&ends.sources[0], &ends.targets[0]].into_iter().enumerate() {
        if operand.steps.is_empty() {
            return Err(Diagnostic::new("FlowEnd requires a feature reference", Some(usage.span.clone())));
        }
        emit_end(flow, elements, index, operand)?;
    }
    let derived = crate::kerml::derived_flow_end_ids(flow, elements);
    if derived.len() != 2 {
        return Err(Diagnostic::new("cannot derive two Flow ends from owned memberships", Some(usage.span.clone())));
    }
    Ok(())
}

fn emit_literal_value_part(
    usage: &ResolvedUsage,
    flow: &mut KirElement,
    elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    let Some(expression) = &usage.expression else { return Ok(()) };
    let ResolvedExpr::Literal(value) = expression else {
        return Err(Diagnostic::new(
            "nonliteral Flow ValuePart expression construction is not implemented",
            Some(usage.span.clone()),
        ));
    };
    let kind = if value.is_boolean() { "LiteralBoolean" }
        else if value.is_i64() || value.is_u64() { "LiteralInteger" }
        else if value.is_number() { "LiteralRational" }
        else if value.is_string() { "LiteralString" }
        else { "NullExpression" };
    let relation_id = format!("{}.feature_value", flow.id);
    let expression_id = format!("{relation_id}.expression");
    let mut relation = membership(flow, &relation_id, "FeatureValue", &expression_id);
    let mut owned = object(&relation, &expression_id, kind);
    owned.properties.insert("value".into(), value.clone());
    owned.properties.insert("expression_ir".into(), render_expression_ir(expression)?);
    owned.properties.insert("owner".into(), json!(flow.id));
    owned.properties.insert("owning_namespace".into(), json!(flow.id));
    owned.properties.insert("owning_membership".into(), json!(relation_id));
    relation.properties.extend(BTreeMap::from([
        ("owned_member_element".into(), json!(expression_id)),
        ("feature_with_value".into(), json!(flow.id)),
        ("is_initial".into(), json!(usage.modifiers.iter().any(|modifier| modifier == "feature_value_is_initial"))),
        ("is_default".into(), json!(usage.modifiers.iter().any(|modifier| modifier == "feature_value_is_default"))),
        ("source".into(), json!([flow.id])),
        ("target".into(), json!([expression_id])),
        ("related_element".into(), json!([flow.id, expression_id])),
        ("is_implied".into(), json!(false)),
    ]));
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut relation, &mut owned)?;
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, flow, &mut relation)?;
    append_unique_property_ref_list(&mut flow.properties, "owned_membership", &relation_id);
    append_unique_property_ref_list(&mut flow.properties, "membership", &relation_id);
    append_unique_property_ref_list(&mut flow.properties, "members", &expression_id);
    elements.extend([relation, owned]);
    Ok(())
}

fn emit_typed_payload(
    usage: &ResolvedUsage,
    flow: &mut KirElement,
    elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    let Some(target) = usage.metadata_properties.get("__flow_payload_type_ref") else {
        return Ok(());
    };
    let feature_id = format!("{}.payload", flow.id);
    let membership_id = format!("{feature_id}.membership");
    let typing_id = format!("{feature_id}.typing");
    let mut payload = object(flow, &feature_id, "PayloadFeature");
    payload.properties.insert("owner".into(), json!(flow.id));
    payload.properties.insert("owning_namespace".into(), json!(flow.id));
    payload.properties.insert("owning_membership".into(), json!(membership_id));
    payload.properties.insert("featuring_type".into(), json!([flow.id]));
    if let Some(name) = usage.metadata_properties.get("__flow_payload_name") {
        payload.properties.insert("declared_name".into(), json!(name));
        payload.properties.insert("name".into(), json!(name));
    }
    payload.properties.insert("type".into(), json!(target));
    let mut membership = membership(flow, &membership_id, "FeatureMembership", &feature_id);
    membership.properties.insert("owned_member_element".into(), json!(feature_id));
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut membership, &mut payload)?;
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, flow, &mut membership)?;
    append_unique_property_ref_list(&mut flow.properties, "owned_membership", &membership_id);
    append_unique_property_ref_list(&mut flow.properties, "membership", &membership_id);
    append_unique_property_ref_list(&mut flow.properties, "members", &feature_id);

    let mut typing = object(&payload, &typing_id, "FeatureTyping");
    typing.properties.extend(BTreeMap::from([
        ("typed_feature".into(), json!(feature_id)),
        ("type".into(), json!(target)),
        ("specific".into(), json!(feature_id)),
        ("general".into(), json!(target)),
        ("source".into(), json!([feature_id])),
        ("target".into(), json!([target])),
        ("related_element".into(), json!([feature_id, target])),
        ("is_implied".into(), json!(false)),
    ]));
    let mut range_graph = if let Some(raw) = usage.metadata_properties.get("__flow_payload_bounds") {
        let references: Vec<Option<String>> = serde_json::from_str(usage.metadata_properties.get("__flow_payload_reference_ids")
            .ok_or_else(|| Diagnostic::new("missing resolved Flow payload bounds", Some(usage.span.clone())))?)
            .map_err(|error| Diagnostic::new(format!("invalid Flow payload bounds: {error}"), Some(usage.span.clone())))?;
        let range_id = format!("{feature_id}.multiplicity");
        let member_id = format!("{range_id}.membership");
        let mut range = object(&payload, &range_id, "MultiplicityRange");
        range.properties.insert("owner".into(), json!(feature_id));
        range.properties.insert("owning_namespace".into(), json!(feature_id));
        range.properties.insert("owning_membership".into(), json!(member_id));
        range.properties.insert("featuring_type".into(), json!([flow.id]));
        let mut member = self::membership(&payload, &member_id, "OwningMembership", &range_id);
        member.properties.insert("owned_member_element".into(), json!(range_id));
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut member, &mut range)?;
        crate::language_frontend::lowering::emit::multiplicity_ranges::emit_bounds(
            raw, &references, &usage.span, &mut range, elements,
        )?;
        Some((member, range))
    } else { None };
    let bounds_first = usage.metadata_properties.get("__flow_payload_bounds_first").is_some_and(|value| value == "true");
    if bounds_first {
        if let Some((member, _)) = range_graph.as_mut() {
            ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, &mut payload, member)?;
        }
    }
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, &mut payload, &mut typing)?;
    if !bounds_first {
        if let Some((member, _)) = range_graph.as_mut() {
            ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, &mut payload, member)?;
        }
    }
    if let Some((member, range)) = &range_graph {
        append_unique_property_ref_list(&mut payload.properties, "owned_membership", &member.id);
        append_unique_property_ref_list(&mut payload.properties, "membership", &member.id);
        append_unique_property_ref_list(&mut payload.properties, "members", &range.id);
    }
    elements.extend([membership, typing, payload]);
    if let Some((member, range)) = range_graph { elements.extend([member, range]); }
    Ok(())
}

fn emit_end(
    flow: &mut KirElement,
    elements: &mut Vec<KirElement>,
    index: usize,
    operand: &Operand<String>,
) -> Result<(), Diagnostic> {
    let end_id = format!("{}.end.{index}", flow.id);
    let end_membership_id = format!("{end_id}.membership");
    let feature_id = format!("{end_id}.feature");
    let feature_membership_id = format!("{feature_id}.membership");
    let redefinition_id = format!("{feature_id}.redefinition");
    let mut end = object(flow, &end_id, "FlowEnd");
    let mut end_membership = membership(flow, &end_membership_id, "EndFeatureMembership", &end_id);
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut end_membership, &mut end)?;
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, flow, &mut end_membership)?;
    append_unique_property_ref_list(&mut flow.properties, "owned_membership", &end_membership_id);
    append_unique_property_ref_list(&mut flow.properties, "members", &end_id);

    if operand.steps.len() >= 2 {
        let reference_id = format!("{end_id}.reference_subsetting");
        let mut reference = object(&end, &reference_id, "ReferenceSubsetting");
        let prefix = if operand.steps.len() == 2 {
            operand.steps[0].clone()
        } else {
            relationship_declarations::emit_owned_feature_chain(
                &mut reference, elements,
                &Operand { steps: operand.steps[..operand.steps.len() - 1].to_vec(), type_ref: None },
                "prefix", 0,
            )?
        };
        reference.properties.extend(BTreeMap::from([
            ("referenced_feature".into(), json!(prefix)),
            ("subsetted_feature".into(), json!(prefix)),
            ("subsetting_feature".into(), json!(end_id)),
            ("specific".into(), json!(end_id)),
            ("general".into(), json!(prefix)),
            ("source".into(), json!([end_id])),
            ("target".into(), json!([prefix])),
            ("related_element".into(), json!([end_id, prefix])),
            ("is_implied".into(), json!(false)),
        ]));
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, &mut end, &mut reference)?;
        end.properties.insert("owned_reference_subsetting".into(), json!(reference_id));
        append_unique_property_ref_list(&mut end.properties, "subsetted_features", &prefix);
        elements.push(reference);
    }

    let mut feature = object(&end, &feature_id, "Feature");
    let mut feature_membership = membership(&end, &feature_membership_id, "FeatureMembership", &feature_id);
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut feature_membership, &mut feature)?;
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, &mut end, &mut feature_membership)?;
    append_unique_property_ref_list(&mut end.properties, "owned_membership", &feature_membership_id);
    append_unique_property_ref_list(&mut end.properties, "members", &feature_id);

    let mut redef = object(&feature, &redefinition_id, "Redefinition");
    let target = operand.steps.last().expect("nonempty FlowEnd steps");
    redef.properties.extend(BTreeMap::from([
        ("redefining_feature".into(), json!(feature_id)),
        ("redefined_feature".into(), json!(target)),
        ("specific".into(), json!(feature_id)),
        ("general".into(), json!(target)),
        ("source".into(), json!([feature_id])),
        ("target".into(), json!([target])),
        ("related_element".into(), json!([feature_id, target])),
        ("is_implied".into(), json!(false)),
    ]));
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, &mut feature, &mut redef)?;
    append_unique_property_ref_list(&mut feature.properties, "redefined_features", target);
    elements.extend([end_membership, feature_membership, redef, feature, end]);
    Ok(())
}

fn object(owner: &KirElement, id: &str, kind: &str) -> KirElement {
    let mut metadata = owner.properties.get("metadata").cloned().unwrap_or_else(|| json!({}));
    metadata["lowering"] = json!({"construct":kind,"metaclass":format!("SysML::{kind}")});
    KirElement {
        id: id.into(), kind: format!("SysML::{kind}"), layer: 2,
        properties: BTreeMap::from([("metadata".into(), metadata)]),
    }
}

fn membership(owner: &KirElement, id: &str, kind: &str, member: &str) -> KirElement {
    let mut element = object(owner, id, kind);
    element.properties.extend(BTreeMap::from([
        ("membership_owning_namespace".into(), json!(owner.id)),
        ("member_element".into(), json!(member)),
        ("source".into(), json!([owner.id])),
        ("target".into(), json!([member])),
        ("related_element".into(), json!([owner.id, member])),
        ("visibility".into(), json!("public")),
        ("is_implied".into(), json!(false)),
    ]));
    element
}
