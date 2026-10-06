//! Native chain-target semantics over imported Ecore contracts.
//!
//! Selection and the target-role rule follow the written 2026-08 specification.
//! These are explicit handwritten algorithms, not translations of Ecore
//! signatures. The Pilot delegate differs for interleaved return memberships
//! and a first IN parameter without an owned Feature; those differences must
//! remain visible. Physical specialization construction, result chains and
//! validation are separate obligations.
use super::*;

// Named semantic dependency: the written target-redefinition constraint.
// This role name is not inferred from samples or executable observations.
const TARGET_ROLE: &str = "ControlFunctions::'.'::source::target";

fn invocation_contract(chain: &KirElement) -> Result<(), QueryFailure> {
    let operation = ecore_model::source_target_feature_contract();
    if !metaclass_conforms(&chain.kind, operation.owner)
        || operation.name != "sourceTargetFeature" || operation.owner != "FeatureChainExpression"
        || operation.target != "Feature" || operation.lower != 0 || operation.upper != 1
        || operation.ordered || !operation.unique || !operation.parameters.is_empty()
        || operation.delegate_uri != "http://www.omg.org/spec/SysML"
        || operation.binding_status != "dynamic_invocation_candidates_not_selected"
        || operation.branches != [("FeatureChainExpression",
            "org.omg.sysml.delegate.invocation.FeatureChainExpression_sourceTargetFeature_InvocationDelegate")] {
        return Err(query_failure("unreviewed sourceTargetFeature invocation contract"));
    }
    Ok(())
}

pub(super) fn first_input<'g>(query: &DefinitionNameQuery<'g, '_>, chain: &KirElement)
    -> Result<Option<&'g KirElement>, QueryFailure> {
    invocation_contract(chain)?;
    for parameter in definition_owned_features_query(query, chain, false)? {
        let direction = parameter.properties.get("direction").unwrap_or(&Value::Null);
        ecore_model::validate_value(&parameter.kind, "direction", direction).map_err(query_failure)?;
        if direction.as_str() == Some("in") { return Ok(Some(parameter)); }
    }
    Ok(None)
}

pub(super) fn source_target<'g>(query: &DefinitionNameQuery<'g, '_>, chain: &KirElement)
    -> Result<Option<&'g KirElement>, QueryFailure> {
    // Stop at the FIRST IN parameter, including a genuinely empty first input.
    match first_input(query, chain)? {
        Some(parameter) => Ok(definition_owned_features_query(query, parameter, false)?.first().copied()),
        None => Ok(None),
    }
}

pub(super) fn target_feature<'g>(query: &DefinitionNameQuery<'g, '_>, chain: &KirElement)
    -> Result<Option<&'g KirElement>, QueryFailure> {
    invocation_contract(chain)?;
    let contract = ecore_model::feature(&chain.kind, "target_feature")
        .ok_or_else(|| query_failure("missing chain target Feature contract"))?;
    let binding = contract.setting_delegate.as_ref()
        .ok_or_else(|| query_failure("missing chain target Feature delegate"))?;
    if binding.uri != "http://www.omg.org/spec/SysML" || binding.status != "custom_setting_delegate_source"
        || binding.candidates != ["org.omg.sysml.delegate.setting.FeatureChainExpression_targetFeature_SettingDelegate"] {
        return Err(query_failure("unreviewed chain target Feature delegate"));
    }
    for membership in stored_children_projection_query(query, chain, "owned_relationship")? {
        if !metaclass_conforms(&membership.kind, "Membership")
            || metaclass_conforms(&membership.kind, "ParameterMembership") { continue; }
        let target = match membership_element_query(query, membership)? {
            GeneratedLinkTarget::Local(id) => query.index.get(id.as_str()).copied()
                .ok_or_else(|| query_failure("chain target member is absent"))?,
            pending => return Err(query_requirement(pending)),
        };
        // Do not skip a non-Feature first member to select a later one.
        return Ok(metaclass_conforms(&target.kind, contract.target).then_some(target));
    }
    Ok(None)
}

/// Canonical containment identifies the source-target role, regardless of
/// source spelling, generated identities or declared names. No state changes
/// or transformation-completion flags are supplied. This helper provides
/// explicit transformation inputs consumed by chain_specialization; cold
/// getters must not apply constraints eagerly.
pub(super) fn implicit_redefinitions<'g>(query: &DefinitionNameQuery<'g, '_>, feature: &KirElement)
    -> Result<Option<Vec<&'g KirElement>>, QueryFailure> {
    if !metaclass_conforms(&feature.kind, "Feature") { return Ok(None); }
    let Some((membership, parameter)) = definition_feature_owner_indexed(query.graph.len(), query.index, feature)? else {
        return Ok(None);
    };
    if !metaclass_conforms(&membership.kind, "FeatureMembership") { return Ok(None); }
    let Some((parameter_membership, chain)) = definition_feature_owner_indexed(query.graph.len(), query.index, parameter)? else {
        return Ok(None);
    };
    if !metaclass_conforms(&parameter_membership.kind, "ParameterMembership")
        || !metaclass_conforms(&chain.kind, "FeatureChainExpression") {
        return Ok(None);
    }
    if source_target(query, chain)?.is_none_or(|source| source.id != feature.id) {
        return Ok(None);
    }
    let default = match standard_default_binding_query(query, TARGET_ROLE)? {
        GeneratedLinkTarget::Local(id) => query.index.get(id.as_str()).copied()
            .ok_or_else(|| query_failure("chain target-role endpoint is absent"))?,
        pending => return Err(query_requirement(pending)),
    };
    ecore_model::validate_reference_endpoint("Redefinition", "redefined_feature", &default.kind)
        .map_err(query_failure)?;
    let mut targets = vec![default];
    if let Some(target) = target_feature(query, chain)? {
        if !targets.iter().any(|endpoint| endpoint.id == target.id) { targets.push(target); }
    }
    Ok(Some(targets))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn append(graph: &mut Vec<KirElement>, owner: &str, id: &str, kind: &str, relation: bool) {
        let mut child = fresh_definition_element(id.to_owned(), kind).unwrap();
        let parent = graph.iter_mut().find(|node| node.id == owner).unwrap();
        ecore_ownership::attach(if relation { &ecore_ownership_generated::OWNED_RELATIONSHIPS }
            else { &ecore_ownership_generated::OWNED_ELEMENTS }, parent, &mut child).unwrap();
        graph.push(child);
    }
    fn graph(first_direction: &str, first_child: bool, second_child: bool, return_before_target: bool)
        -> Vec<KirElement> {
        let mut graph = vec![fresh_definition_element("chain".into(), "FeatureChainExpression").unwrap()];
        for (id, direction, child) in [("first", first_direction, first_child), ("second", "in", second_child)] {
            let membership = format!("{id}.membership");
            append(&mut graph, "chain", &membership, "ParameterMembership", true);
            append(&mut graph, &membership, id, "Feature", false);
            graph.iter_mut().find(|node| node.id == id).unwrap().properties.insert("direction".into(),json!(direction));
            if child {
                let member = format!("{id}.target.membership");
                append(&mut graph, id, &member, "FeatureMembership", true);
                append(&mut graph, &member, &format!("{id}.target"), "Feature", false);
            }
        }
        let relationships = if return_before_target { [("return.membership","ReturnParameterMembership","result"),("target.membership","OwningMembership","target")] }
            else { [("target.membership","OwningMembership","target"),("return.membership","ReturnParameterMembership","result")] };
        for (membership, kind, child) in relationships {
            append(&mut graph, "chain", membership, kind, true);
            append(&mut graph, membership, child, "Feature", false);
        }
        graph
    }
    #[test]
    fn definition_chain_targets_follow_normative_first_input_and_membership_selection() {
        for (direction, first, second, expected) in [
            ("in",true,true,Some("first.target")),("in",false,true,None),
            ("out",true,true,Some("second.target")),("inout",true,true,Some("second.target")),
            ("in",false,false,None),
        ] {
            let graph=graph(direction,first,second,false);
            let before=serde_json::to_value(&graph).unwrap();
            let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();
            let query=DefinitionNameQuery::new(&graph,&index);
            assert_eq!(source_target(&query,index["chain"]).unwrap().map(|node|node.id.as_str()),expected);
            assert_eq!(serde_json::to_value(&graph).unwrap(),before);
        }
        for return_first in [false,true] {
            let graph=graph("in",true,true,return_first);
            let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();
            let query=DefinitionNameQuery::new(&graph,&index);
            assert_eq!(target_feature(&query,index["chain"]).unwrap().unwrap().id,"target");
            let mut persisted: Vec<KirElement>=serde_json::from_value(serde_json::to_value(&graph).unwrap()).unwrap();
            persisted.reverse();
            let index=persisted.iter().map(|node|(node.id.as_str(),node)).collect();
            assert_eq!(target_feature(&DefinitionNameQuery::new(&persisted,&index),index["chain"]).unwrap().unwrap().id,"target");
        }
    }

    #[test]
    fn definition_chain_targets_preserve_typed_pending_and_non_feature_boundaries() {
        let mut graph=graph("in",true,false,false);
        let old_target=graph.iter().find(|node|node.id=="target").unwrap().clone();
        graph.iter_mut().find(|node|node.id=="target").unwrap().kind="SysML::DataType".into();
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();
        assert!(target_feature(&DefinitionNameQuery::new(&graph,&index),index["chain"]).unwrap().is_none());
        *graph.iter_mut().find(|node|node.id=="target").unwrap()=old_target;
        graph.iter_mut().find(|node|node.id=="target.membership").unwrap().kind="SysML::Membership".into();
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();
        assert!(matches!(target_feature(&DefinitionNameQuery::new(&graph,&index),index["chain"]),
            Err(QueryFailure::Required(Prerequisite::ReadField {owner_id,field}))
                if owner_id=="target.membership" && field=="member_element"));
    }

    #[test]
    fn definition_chain_targets_match_all_independent_controls_with_explicit_normative_dispositions() {
        let observations: Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-result-chain-selection-registered-reference-controls.json"))).unwrap();
        let disposition: Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-result-chain-selection-registered-reference-run.json"))).unwrap();
        let controls = observations["controls"].as_array().unwrap();
        assert_eq!(controls.len(), 9);
        assert_eq!(disposition["controls_observed"], json!(9));
        for control in controls {
            assert_eq!(control["completion_flag"], json!(false));
            let source = control["operation"] == json!("sourceTargetFeature");
            let mut graph = if source {
                graph(control["first_direction"].as_str().unwrap(),
                    control["first_owned_feature"].as_bool().unwrap(),
                    control["second_owned_feature"].as_bool().unwrap(), false)
            } else {
                let mut graph = graph("in", false, false, control["return_before_target"].as_bool().unwrap());
                graph.retain(|node| node.id != "second" && node.id != "second.membership");
                graph.iter_mut().find(|node| node.id == "chain").unwrap().properties
                    .get_mut("owned_relationship").unwrap().as_array_mut().unwrap()
                    .retain(|value| value != &json!("second.membership"));
                if control["target_is_data_type"] == json!(true) {
                    graph.iter_mut().find(|node| node.id == "target").unwrap().kind = "SysML::DataType".into();
                }
                graph
            };
            if source {
                graph.iter_mut().find(|node| node.id == "second").unwrap().properties
                    .insert("direction".into(), control["second_direction"].clone());
            }
            let before = serde_json::to_value(&graph).unwrap();
            let index = graph.iter().map(|node| (node.id.as_str(), node)).collect();
            let query = DefinitionNameQuery::new(&graph, &index);
            let selected = if source { source_target(&query, index["chain"]).unwrap() }
                else { target_feature(&query, index["chain"]).unwrap() };
            let actual = selected.map(|node| json!(node.id)).unwrap_or(Value::Null);
            let reviewed = disposition["normative_comparison"].as_array().unwrap().iter()
                .find(|row| row["id"] == control["id"]).unwrap();
            assert_eq!(actual, reviewed["normative"], "{}", control["id"]);
            if reviewed["disagrees"] == json!(false) { assert_eq!(actual, control["pilot_endpoint"]); }
            if !source {
                let targets = definition_reference_targets_typed_query(&query, index["chain"], "target_feature").unwrap();
                assert_eq!(targets.first().map(|node| json!(node.id)).unwrap_or(Value::Null), actual);
            }
            assert_eq!(serde_json::to_value(&graph).unwrap(), before);
        }
    }

    #[test]
    fn definition_chain_targets_keep_computed_redefinition_reads_typed_through_inheritance() {
        // Materialized prototype fixtures; pinned resources remain unprepared.
        let mut graph=vec![fresh_definition_element("function".into(),"Function").unwrap(),
            fresh_definition_element("prototype".into(),"Function").unwrap()];
        for owner in ["function","prototype"] {
            graph.iter_mut().find(|n|n.id==owner).unwrap().properties.insert("is_implied_included".into(),json!(true));
            let member=format!("{owner}.member");let parameter=format!("{owner}.parameter");
            append(&mut graph,owner,&member,"ParameterMembership",true);
            append(&mut graph,&member,&parameter,"Feature",false);
            graph.iter_mut().find(|n|n.id==parameter).unwrap().properties.insert("direction".into(),json!("in"));
        }
        graph.iter_mut().find(|n|n.id=="prototype.parameter").unwrap().properties.insert("is_implied_included".into(),json!(true));
        append(&mut graph,"function","generalization","Subclassification",true);
        let kind=graph.iter().find(|n|n.id=="generalization").unwrap().kind.clone();
        let specific=ecore_model::redefined_feature(&kind,"specific").unwrap().field;
        let general=ecore_model::redefined_feature(&kind,"general").unwrap().field;
        graph.iter_mut().find(|n|n.id=="generalization").unwrap().properties.insert(specific.into(),json!("function"));
        let before=serde_json::to_value(&graph).unwrap();
        let index=graph.iter().map(|n|(n.id.as_str(),n)).collect();
        let query=DefinitionNameQuery::new(&graph,&index);
        assert!(matches!(materialized_redefinition_closure_typed_query(&query,index["function.parameter"]),
            Err(QueryFailure::Required(Prerequisite::ReadField {owner_id,field})) if owner_id=="generalization" && field==general));
        assert!(!query.redefinition_closures.borrow().contains_key("function.parameter"));
        assert!(matches!(remove_redefined_memberships_evaluate(&query,index["function"],vec![index["prototype.member"]],None,false).unwrap(),
            Members::Deferred(GeneratedLinkTarget::Deferred {owner_id,field}) if owner_id=="generalization" && field==general));
        assert_eq!(serde_json::to_value(&graph).unwrap(),before);
        graph.iter_mut().find(|n|n.id=="generalization").unwrap().properties.insert(general.into(),json!("prototype"));
        let index=graph.iter().map(|n|(n.id.as_str(),n)).collect();
        let query=DefinitionNameQuery::new(&graph,&index);
        assert!(matches!(remove_redefined_memberships_evaluate(&query,index["function"],vec![index["prototype.member"]],None,false).unwrap(),
            Members::Ready(members) if members.is_empty()));
    }
}
