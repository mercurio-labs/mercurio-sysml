//! Shared handwritten FeatureValue dependency assessment over resolved Ecore
//! ownership, value delegates, Boolean defaults and specialization ancestry.
//! The normative valuation-specialization predicate reads effective direction
//! and explicit owned specializations. These selectors do not complete values,
//! construct bindings, validate initial values or qualify receiver lifecycles.
use super::*;

pub(super) struct ValueInputs<'g> {
    pub valuation: &'g KirElement,
    pub expression: &'g KirElement,
    pub default: bool,
    pub initial: bool,
}

/// Preserve every valuation. Pilot's first-valuation selection and the written
/// single-valuation validation are separate algorithms; neither is silently
/// substituted for this dependency enumeration.
pub(super) fn inputs<'g>(query: &DefinitionNameQuery<'g, '_>, owner: &'g KirElement)
    -> Result<Vec<ValueInputs<'g>>, QueryFailure> {
    if !metaclass_conforms(&owner.kind, "Feature") {
        return Err(query_failure("valuation inputs require an imported Feature metaclass"));
    }
    let mut values = Vec::new();
    for value in stored_children_projection_query(query, owner, "owned_relationship")?.into_iter()
        .filter(|relation| metaclass_conforms(&relation.kind, "FeatureValue")) {
        let feature = definition_reference_targets_typed_query(query, value, "feature_with_value")?;
        if feature.len() != 1 || feature[0].id != owner.id
            || scope_container_query(query, value)?.is_none_or(|parent| parent.id != owner.id) {
            return Err(query_failure("valuation requires reciprocal canonical Feature ownership"));
        }
        let expression = definition_reference_targets_typed_query(query, value, "value")?;
        if expression.len() != 1
            || !metaclass_conforms(&expression[0].kind, "Expression")
            || scope_container_query(query, expression[0])?.is_none_or(|parent| parent.id != value.id) {
            return Err(query_failure("valuation requires one canonically owned Expression value"));
        }
        values.push(ValueInputs { valuation: value, expression: expression[0],
            default: scope_boolean(value, "is_default")?, initial: scope_boolean(value, "is_initial")? });
    }
    Ok(values)
}

/// Assess only absence of valuation-derived specialization contributions. The
/// written predicate excludes directed features and explicit specializations,
/// including FeatureTyping. Default/initial flags and expression kinds never
/// establish this absence; unknown bound-value computation stays unsupported.
pub(super) fn assess_non_typing(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement,
    relation: &KirElement) -> Result<bool, QueryFailure> {
    if !metaclass_conforms(&relation.kind, "FeatureValue") { return Ok(false); }
    let values = inputs(query, owner)?;
    if !values.iter().any(|value| value.valuation.id == relation.id) {
        return Err(query_failure("valuation contribution is absent from its owner"));
    }
    if definition_has_direction(owner)? { return Ok(true); }
    for specialization in stored_children_projection_query(query, owner, "owned_relationship")? {
        if !metaclass_conforms(&specialization.kind, "Specialization")
            || scope_boolean(specialization, "is_implied")? { continue; }
        let specific = definition_reference_targets_typed_query(query, specialization, "specific")?;
        if specific.len() != 1 || specific[0].id != owner.id {
            return Err(query_failure("explicit valuation specialization has an invalid specific endpoint"));
        }
        return Ok(true);
    }
    Ok(false)
}

/// Ordinary namespace ownership does not turn a directed Feature into a
/// Behavior/Step parameter. Direction affects the valuation predicate; it does
/// not enter the imported default selector's type/composition predicates.
/// This batch admits canonical directed valuations. Unvalued directed Features,
/// parameter memberships, detached receivers and specialized ownership retain
/// separate native dependencies. This is an implementation boundary, not a
/// language constraint.
pub(super) fn ordinary_directed_context(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement)
    -> Result<bool, QueryFailure> {
    if owner.kind.rsplit("::").next() != Some("Feature") || !definition_has_direction(owner)?
        || scope_boolean(owner, "is_end")? { return Ok(false); }
    let Some(membership) = scope_container_query(query, owner)? else { return Ok(false); };
    let Some(namespace) = scope_container_query(query, membership)? else { return Ok(false); };
    // Element.owningNamespace is wider than Feature.owningType: Package-owned
    // Features use ordinary OwningMembership and have no owning Type.
    let member = definition_reference_targets_typed_query(query, membership, "member_element")?;
    if member.len() != 1 || member[0].id != owner.id {
        return Err(query_failure("directed selector requires a canonical membership endpoint"));
    }
    Ok(metaclass_conforms(&membership.kind, "OwningMembership")
        && !metaclass_conforms(&membership.kind, "ParameterMembership")
        && !metaclass_conforms(&membership.kind, "FeatureValue")
        && (matches!(namespace.kind.rsplit("::").next(), Some("Namespace" | "Package" | "LibraryPackage"))
            || metaclass_conforms(&namespace.kind, "Classifier"))
        && !inputs(query, owner)?.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attach(graph: &mut Vec<KirElement>, owner: &str, id: &str, kind: &str, relationships: bool) {
        let mut child = fresh_definition_element(id.into(), kind).unwrap();
        let rule = if relationships { &ecore_ownership_generated::OWNED_RELATIONSHIPS }
            else { &ecore_ownership_generated::OWNED_ELEMENTS };
        ecore_ownership::attach(rule, graph.iter_mut().find(|node| node.id == owner).unwrap(), &mut child).unwrap();
        graph.push(child);
    }

    fn fixture(kind: &str, typed: bool) -> Vec<KirElement> {
        let mut graph = vec![fresh_definition_element("owner".into(), "Feature").unwrap()];
        if typed {
            attach(&mut graph, "owner", "typing", "FeatureTyping", true);
            graph.iter_mut().find(|node| node.id == "typing").unwrap().properties.insert("typed_feature".into(), json!("owner"));
        }
        attach(&mut graph, "owner", "value", "FeatureValue", true);
        attach(&mut graph, "value", "expression", kind, false);
        graph
    }

    #[test]
    fn definition_general_value_inputs_assess_all_expression_and_flag_categories() {
        for kind in ["LiteralInteger", "LiteralBoolean", "LiteralInfinity", "FeatureReferenceExpression",
            "FeatureChainExpression", "OperatorExpression", "InvocationExpression", "IndexExpression"] {
            for (default, initial) in [(false,false),(true,false),(false,true),(true,true)] {
                let mut graph = fixture(kind, true);
                let value = graph.iter_mut().find(|node| node.id == "value").unwrap();
                value.properties.insert("is_default".into(), json!(default));
                value.properties.insert("is_initial".into(), json!(initial));
                for reverse in [false,true] {
                    if reverse { graph.reverse(); }
                    let snapshot = serde_json::to_value(&graph).unwrap();
                    let index = graph.iter().map(|node| (node.id.as_str(),node)).collect();
                    let query = DefinitionNameQuery::new(&graph,&index);
                    let values = inputs(&query,query.index["owner"]).unwrap();
                    assert_eq!(values.len(),1);
                    assert_eq!(values[0].expression.kind,format!("SysML::{kind}"));
                    assert_eq!((values[0].default,values[0].initial),(default,initial));
                    assert!(assess_non_typing(&query,query.index["owner"],query.index["value"]).unwrap());
                    assert_eq!(snapshot,serde_json::to_value(&graph).unwrap());
                    assert!(graph.iter().all(|node| node.properties.get("is_implied_included") != Some(&json!(true))));
                }
            }
        }
        // Preserve additional valuations for explicit normative validation,
        // rather than copying Pilot's first-value rule into structural support.
        let mut graph=fixture("LiteralInteger",true);
        attach(&mut graph,"owner","value2","FeatureValue",true);
        attach(&mut graph,"value2","expression2","LiteralBoolean",false);
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();
        let query=DefinitionNameQuery::new(&graph,&index);
        assert_eq!(inputs(&query,query.index["owner"]).unwrap().iter().map(|v|v.valuation.id.as_str()).collect::<Vec<_>>(),vec!["value","value2"]);
        assert!(assess_non_typing(&query,query.index["owner"],query.index["value2"]).unwrap());
    }

    #[test]
    fn definition_general_value_inputs_directed_namespace_uses_owner_not_owning_type() {
        for namespace in ["package", "classifier", "class", "struct"] {
            let source=format!("standard library package Base {{ feature things; }} {namespace} P {{ classifier T; out feature v : T = 1; }}");
            let document=crate::definition_document::parse_and_link(&source,crate::SourceLanguage::Kerml).unwrap();
            for graph in [document.elements.clone(),serde_json::from_str::<Vec<KirElement>>(&serde_json::to_string(&document.elements).unwrap()).unwrap()] {
                let owner=graph.iter().find(|node|node.properties.get("declared_name")==Some(&json!("v"))).unwrap();
                let snapshot=serde_json::to_value(&graph).unwrap();
                let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();
                let query=DefinitionNameQuery::new(&graph,&index);
                assert!(ordinary_directed_context(&query,owner).unwrap(),"{namespace}");
                if namespace=="package" {assert!(definition_feature_owner_indexed(graph.len(),&index,owner).unwrap().is_none());}
                assert!(definition_feature_default_name_in_view(&query,owner).is_ok(),"{namespace}");
                assert_eq!(snapshot,serde_json::to_value(&graph).unwrap());
            }
        }
        for membership in ["ParameterMembership","ReturnParameterMembership","FeatureValue"] {
            let mut graph=vec![fresh_definition_element("namespace".into(),"Package").unwrap()];
            attach(&mut graph,"namespace","membership",membership,true);
            attach(&mut graph,"membership","owner","Feature",false);
            graph.iter_mut().find(|node|node.id=="owner").unwrap().properties.insert("direction".into(),json!("out"));
            let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();
            let query=DefinitionNameQuery::new(&graph,&index);
            let outcome=ordinary_directed_context(&query,query.index["owner"]);
            if membership=="FeatureValue" {
                // This supplied Feature child violates the imported Expression
                // target contract; its malformed endpoint must remain an error.
                assert!(outcome.is_err(),"{membership}");
            } else { assert!(!outcome.unwrap(),"{membership}"); }
        }
    }

    #[test]
    fn definition_general_value_inputs_keep_bound_dependencies_and_corruption_explicit() {
        for mode in ["bound","default_bound","initial_bound","implied_typing","directed","invalid_direction",
            "invalid_default","invalid_initial","missing_expression","non_expression","bad_value_owner","bad_expression_owner"] {
            let mut graph=fixture("LiteralInteger",mode=="implied_typing");
            match mode {
                "default_bound" => { graph.iter_mut().find(|n|n.id=="value").unwrap().properties.insert("is_default".into(),json!(true)); },
                "initial_bound" => { graph.iter_mut().find(|n|n.id=="value").unwrap().properties.insert("is_initial".into(),json!(true)); },
                "implied_typing" => { graph.iter_mut().find(|n|n.id=="typing").unwrap().properties.insert("is_implied".into(),json!(true)); },
                "directed" | "invalid_direction" => { graph[0].properties.insert("direction".into(),json!(if mode=="directed" {"out"} else {"invalid"})); },
                "invalid_default" | "invalid_initial" => { graph.iter_mut().find(|n|n.id=="value").unwrap().properties.insert(if mode=="invalid_default" {"is_default"} else {"is_initial"}.into(),json!("true")); },
                "missing_expression" => { graph.iter_mut().find(|n|n.id=="value").unwrap().properties.insert("owned_related_element".into(),json!([])); graph.retain(|n|n.id!="expression"); },
                "non_expression" => { graph.iter_mut().find(|n|n.id=="expression").unwrap().kind="SysML::Feature".into(); },
                "bad_value_owner" => { graph.iter_mut().find(|n|n.id=="value").unwrap().properties.insert("owning_related_element".into(),json!("expression")); },
                "bad_expression_owner" => { graph.iter_mut().find(|n|n.id=="expression").unwrap().properties.insert("owning_relationship".into(),json!("owner")); },
                _ => {},
            }
            let snapshot=serde_json::to_value(&graph).unwrap();
            let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();
            let query=DefinitionNameQuery::new(&graph,&index);
            let outcome=assess_non_typing(&query,query.index["owner"],query.index["value"]);
            match mode {
                "directed" => assert!(outcome.unwrap(),"{mode}"),
                "bound" | "default_bound" | "initial_bound" | "implied_typing" => assert!(!outcome.unwrap(),"{mode}"),
                _ => assert!(outcome.is_err(),"{mode}"),
            }
            assert_eq!(snapshot,serde_json::to_value(&graph).unwrap());
        }
    }
}
