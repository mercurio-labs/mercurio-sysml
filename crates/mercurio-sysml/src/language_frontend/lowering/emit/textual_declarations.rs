//! Textual representations and invariant ownership, grounded in the pinned overlay.
use super::*;

#[derive(Deserialize)]
struct Rules {
    invariant_type: String,
    invariant_result: String,
    positive_subset: String,
    negative_subset: String,
}

pub(super) fn emit(
    usage: &ResolvedUsage,
    owner_id: &str,
    element: &mut KirElement,
    elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    if usage.construct == "CommentUsage" {
        emit_comment(usage, owner_id, element, elements)?;
    } else if usage.construct == "Documentation" {
        for key in ["body", "locale"] {
            if let Some(value) = usage.metadata_properties.get(key) {
                let value = if key == "body" {
                    crate::parser::textual_representation::semantic_body(value)
                } else { crate::parser::textual_representation::language_value(value)? };
                element.properties.insert(key.into(), json!(value));
            }
        }
        if let Some(short) = modifier_value(&usage.modifiers, "short_name") {
            element.properties.insert("declared_short_name".into(), json!(short));
        }
        element.properties.insert("documented_element".into(), json!(owner_id));
        element.properties.insert("annotated_element".into(), json!([owner_id]));
        add_annotating_ownership(Some(usage), owner_id, element, elements)?;
        if let Some(owner) = elements.iter_mut().find(|e| e.id == owner_id) {
            append_unique_property_ref_list(&mut owner.properties, "documentation", &element.id);
            for key in ["features", "owned_feature"] {
                if let Some(Value::Array(refs)) = owner.properties.get_mut(key) {
                    refs.retain(|value| value.as_str() != Some(&element.id));
                }
            }
        }
    } else if usage.construct == "TextualRepresentation" {
        for key in ["language", "body"] {
            if let Some(value) = usage.metadata_properties.get(key) {
                let value = if key == "body" {
                    crate::parser::textual_representation::semantic_body(value)
                } else {
                    crate::parser::textual_representation::language_value(value)?
                };
                element.properties.insert(key.into(), json!(value));
            }
        }
        element
            .properties
            .insert("represented_element".into(), json!(owner_id));
        element
            .properties
            .insert("annotated_element".into(), json!([owner_id]));
        add_annotating_ownership(Some(usage), owner_id, element, elements)?;
        if let Some(owner) = elements.iter_mut().find(|e| e.id == owner_id) {
            append_unique_property_ref_list(
                &mut owner.properties,
                "textual_representation",
                &element.id,
            );
            // AnnotatingElement is not a Feature of its owning Type.
            for key in ["features", "owned_feature"] {
                if let Some(Value::Array(refs)) = owner.properties.get_mut(key) {
                    refs.retain(|value| value.as_str() != Some(&element.id));
                }
            }
        }
    } else if usage.construct == "Invariant" {
        let rules = rules()?;
        let negated = usage.modifiers.iter().any(|m| m == "is_negated");
        element
            .properties
            .insert("is_negated".into(), json!(negated));
        if usage.type_ref.is_none() {
            element
                .properties
                .insert("type".into(), json!([rules.invariant_type]));
        }
        element
            .properties
            .insert("function".into(), json!(rules.invariant_type));
        element
            .properties
            .insert("result".into(), json!(rules.invariant_result));
        element
            .properties
            .insert("parameter".into(), json!([rules.invariant_result]));
        let subset = if negated {
            &rules.negative_subset
        } else {
            &rules.positive_subset
        };
        append_unique_property_ref_list(&mut element.properties, "specializes", subset);
        append_unique_property_ref_list(&mut element.properties, "subsetted_features", subset);
        add_membership(Some(usage), owner_id, element, elements, "FeatureMembership")?;
        if usage.modifiers.iter().any(|m| m == "expression_is_result")
            && let Some(expression) = &usage.expression
        {
            let id = format!("{}.result-expression", element.id);
            let membership = format!("{id}.membership");
            let mut properties = BTreeMap::from([
                ("owner".into(), json!(element.id)),
                ("owning_namespace".into(), json!(element.id)),
                ("owning_membership".into(), json!(membership)),
                ("owning_feature_membership".into(), json!(membership)),
                ("expression_ir".into(), render_expression_ir(expression)?),
            ]);
            let kind = match expression {
                ResolvedExpr::Literal(value) => {
                    properties.insert("value".into(), value.clone());
                    if value.is_boolean() {
                        "LiteralBoolean"
                    } else if value.is_i64() || value.is_u64() {
                        "LiteralInteger"
                    } else if value.is_number() {
                        "LiteralRational"
                    } else if value.is_string() {
                        "LiteralString"
                    } else {
                        "NullExpression"
                    }
                }
                ResolvedExpr::Binary { .. }
                | ResolvedExpr::Unary { .. }
                | ResolvedExpr::Operation { .. } => "OperatorExpression",
                ResolvedExpr::Call { .. } => "InvocationExpression",
                ResolvedExpr::FeaturePath { .. } => "FeatureReferenceExpression",
                _ => "Expression",
            };
            copy_metadata(element, &mut properties, kind);
            let mut result = KirElement {
                id: id.clone(),
                kind: format!("SysML::{kind}"),
                layer: 2,
                properties,
            };
            operand_construction::emit_eager_operands(expression, &mut result, elements)?;
            let mut member = membership_properties(&element.id, &id);
            member.insert("owning_type".into(), json!(element.id));
            member.insert("owned_result_expression".into(), json!(id));
            copy_metadata(element, &mut member, "ResultExpressionMembership");
            let mut relationship = KirElement {
                id: membership.clone(), kind: "SysML::ResultExpressionMembership".into(),
                layer: 2, properties: member,
            };
            ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut relationship, &mut result)?;
            ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, element, &mut relationship)?;
            elements.push(relationship);
            elements.push(result);
            append_unique_property_ref_list(
                &mut element.properties,
                "owned_membership",
                &membership,
            );
            append_unique_property_ref_list(&mut element.properties, "members", &id);
            element
                .properties
                .insert("result_expression".into(), json!(id));
            element
                .properties
                .insert("expression_is_result".into(), json!(true));
        }
    }
    Ok(())
}

fn emit_comment(usage: &ResolvedUsage, owner: &str, element: &mut KirElement, elements: &mut Vec<KirElement>) -> Result<(), Diagnostic> {
    for key in ["body", "locale"] {
        if let Some(value) = usage.metadata_properties.get(key) {
            let value = if key == "body" {
                crate::parser::textual_representation::semantic_body(value)
            } else { crate::parser::textual_representation::language_value(value)? };
            element.properties.insert(key.into(), json!(value));
        }
    }
    let mut targets = usage.annotation_targets.iter().map(|value| (value.target.as_str(), &value.span)).collect::<Vec<_>>();
    if targets.is_empty() && let Some(target) = &usage.reference_target { targets.push((target, &usage.span)); }
    let owning_annotation = add_annotating_ownership(Some(usage), owner, element, elements)?;
    let mut annotated = if targets.is_empty() { vec![owner] } else { targets.iter().map(|(target, _)| *target).collect() };
    if owning_annotation.is_some() && !targets.is_empty() { annotated.insert(0, owner); }
    element.properties.insert("annotated_element".into(), json!(dedupe_refs(annotated.into_iter().map(str::to_string).collect())));
    // Retain the existing single-target compatibility field.
    if let Some(target) = targets.first() { element.properties.insert("annotatedElement".into(), json!(target.0)); }
    let mut annotations = owning_annotation.into_iter().collect::<Vec<_>>();
    let mut owned_annotations = Vec::new();
    for (index, (target, span)) in targets.into_iter().enumerate() {
        let id = format!("{}.annotation.{}", element.id, index + 1);
        let mut properties = BTreeMap::from([
            ("owning_related_element".into(), json!(element.id)),
            ("owning_annotating_element".into(), json!(element.id)),
            ("annotating_element".into(), json!(element.id)),
            ("annotated_element".into(), json!(target)),
            ("source".into(), json!([element.id])),
            ("target".into(), json!([target])),
            ("related_element".into(), json!(dedupe_refs(vec![element.id.clone(), target.to_string()]))),
            ("owned_related_element".into(), json!([])),
            ("is_implied".into(), json!(false)),
        ]);
        if target == element.id {
            // checkAnnotation rejects an annotation owned by its target unless
            // it also owns the annotating element. Derive this before validation.
            properties.insert("owning_annotated_element".into(), json!(target));
        }
        copy_metadata(element, &mut properties, "Annotation");
        properties.get_mut("metadata").unwrap()["source_span"] = json!(span);
        elements.push(KirElement { id: id.clone(), kind: "SysML::Annotation".into(), layer: 2, properties });
        append_unique_property_ref_list(&mut element.properties, "owned_relationship", &id);
        owned_annotations.push(id.clone());
        annotations.push(id);
    }
    element.properties.insert("owned_annotating_relationship".into(), json!(owned_annotations));
    element.properties.insert("annotation".into(), json!(annotations));
    if let Some(owner) = elements.iter_mut().find(|e| e.id == owner) {
        for key in ["features", "owned_feature"] {
            if let Some(Value::Array(values)) = owner.properties.get_mut(key) {
                values.retain(|value| value.as_str() != Some(&element.id));
            }
        }
    }
    Ok(())
}

/// RelationshipBody owns AnnotatingElement through OwnedAnnotation; namespace
/// bodies instead use AnnotatingMember (OwningMembership) in both grammars.
fn add_annotating_ownership(
    usage: Option<&ResolvedUsage>, owner: &str, element: &mut KirElement,
    elements: &mut Vec<KirElement>,
) -> Result<Option<String>, Diagnostic> {
    let relationship_body = elements.iter().find(|e| e.id == owner)
        .map(|e| super::super::relationship_declarations::uses_owned_annotation(&e.kind))
        .transpose()?.unwrap_or(false);
    if !relationship_body {
        add_membership(usage, owner, element, elements, "OwningMembership")?;
        element.properties.entry("annotation".into()).or_insert(json!([]));
        return Ok(None);
    }
    let id = format!("{}.owning-annotation", element.id);
    let mut properties = BTreeMap::from([
        ("owning_annotated_element".into(), json!(owner)),
        ("owned_annotating_element".into(), json!(element.id)),
        ("annotating_element".into(), json!(element.id)),
        ("annotated_element".into(), json!(owner)),
        ("source".into(), json!([element.id])),
        ("target".into(), json!([owner])),
        ("related_element".into(), json!([element.id, owner])),
        ("is_implied".into(), json!(false)),
    ]);
    copy_metadata(element, &mut properties, "Annotation");
    for key in ["owning_membership", "owning_namespace", "owning_feature_membership"] {
        element.properties.remove(key);
    }
    element.properties.insert("owner".into(), json!(owner));
    let mut relationship = KirElement { id: id.clone(), kind: "SysML::Annotation".into(), layer: 2, properties };
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut relationship, element)?;
    element.properties.insert("owning_annotating_relationship".into(), json!(id));
    element.properties.insert("annotation".into(), json!([id]));
    if let Some(parent) = elements.iter_mut().find(|e| e.id == owner) {
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, parent, &mut relationship)?;
        for key in ["owned_annotation"] {
            append_unique_property_ref_list(&mut parent.properties, key, &id);
        }
    }
    elements.push(relationship);
    Ok(Some(id))
}

pub(super) fn attach_legacy_documentation(
    owner: &str, element: &mut KirElement, elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    element.properties.insert("documented_element".into(), json!(owner));
    element.properties.insert("annotated_element".into(), json!([owner]));
    add_annotating_ownership(None, owner, element, elements)?;
    if let Some(parent) = elements.iter_mut().find(|e| e.id == owner) {
        append_unique_property_ref_list(&mut parent.properties, "documentation", &element.id);
    }
    Ok(())
}

fn membership_properties(owner: &str, member: &str) -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("membership_owning_namespace".into(), json!(owner)),
        ("member_element".into(), json!(member)),
        ("source".into(), json!([owner])),
        ("target".into(), json!([member])),
        ("related_element".into(), json!([owner, member])),
        ("visibility".into(), json!("public")),
        ("is_implied".into(), json!(false)),
    ])
}

fn add_membership(
    usage: Option<&ResolvedUsage>,
    owner: &str,
    element: &mut KirElement,
    elements: &mut Vec<KirElement>,
    kind: &str,
) -> Result<(), Diagnostic> {
    let id = format!("{}.membership", element.id);
    let mut properties = membership_properties(owner, &element.id);
    if let Some(usage) = usage {
    if !usage.is_implicit_name {
        properties.insert("member_name".into(), json!(usage.declared_name));
        properties.insert("owned_member_name".into(), json!(usage.declared_name));
    }
    if let Some(short) = modifier_value(&usage.modifiers, "short_name") {
        properties.insert("member_short_name".into(), json!(short));
        properties.insert("owned_member_short_name".into(), json!(short));
    }
    if let Some(visibility) = usage
        .modifiers
        .iter()
        .find(|m| matches!(m.as_str(), "public" | "private" | "protected"))
    {
        properties.insert("visibility".into(), json!(visibility));
    }
    }
    if kind == "FeatureMembership" {
        properties.insert("owning_type".into(), json!(owner));
        element
            .properties
            .insert("owning_feature_membership".into(), json!(id));
    }
    copy_metadata(element, &mut properties, kind);
    element
        .properties
        .insert("owning_membership".into(), json!(id));
    element
        .properties
        .insert("owning_namespace".into(), json!(owner));
    let mut relationship = KirElement { id: id.clone(), kind: format!("SysML::{kind}"), layer: 2, properties };
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut relationship, element)?;
    ecore_ownership::attach_to_owner(owner, &mut relationship, elements)?;
    if let Some(owner) = elements.iter_mut().find(|e| e.id == owner) {
        append_unique_property_ref_list(&mut owner.properties, "owned_membership", &id);
    }
    elements.push(relationship);
    Ok(())
}

fn copy_metadata(element: &KirElement, properties: &mut BTreeMap<String, Value>, kind: &str) {
    let mut metadata = element
        .properties
        .get("metadata")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if let Some(object) = metadata.as_object_mut() {
        object.insert(
            "lowering".into(),
            json!({"construct":kind,"metaclass":format!("SysML::{kind}")}),
        );
    }
    properties.insert("metadata".into(), metadata);
}

fn rules() -> Result<&'static Rules, Diagnostic> {
    static RULES: OnceLock<Result<Rules, String>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../resources/kernel/textual-declarations.overlay.json"
            ))
            .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| Diagnostic::new(format!("invalid textual declaration overlay: {e}"), None))
}
