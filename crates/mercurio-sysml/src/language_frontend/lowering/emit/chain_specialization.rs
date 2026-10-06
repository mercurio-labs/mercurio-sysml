//! Explicit native chain-specialization transaction over imported Ecore contracts.
//! Written source-target redefinitions and result chained subsetting are separate
//! from cold getters, full expression transformation and lifecycle completion.
//! Scoping and one-level chain expansion are named handwritten dependencies.
use super::*;

struct Inputs {
    source: String,
    parameter: String,
    result: String,
    redefinitions: Vec<String>,
    chain: Vec<String>,
}

fn first_input<'g>(query: &DefinitionNameQuery<'g, '_>, owner: &KirElement)
    -> Result<Option<&'g KirElement>, QueryFailure> {
    feature_chain_targets::first_input(query, owner)
}

fn expanded_chain(query: &DefinitionNameQuery<'_, '_>, inputs: &[&str])
    -> Result<Vec<String>, QueryFailure> {
    let mut result = Vec::new();
    for id in inputs {
        let feature = query.index.get(*id).copied()
            .ok_or_else(|| query_failure("chain input is absent"))?;
        let chain = definition_chain_reference_targets_typed_query(query, feature, "chaining_feature")?;
        if chain.is_empty() { result.push(feature.id.clone()); }
        else { result.extend(chain.iter().map(|node| node.id.clone())); }
    }
    Ok(result)
}

fn inputs(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement)
    -> Result<TransformationPlan<Option<Inputs>>, QueryFailure> {
    if !feature_chain_members::supports(&owner.kind) || scope_boolean(owner, "is_implied_included")? {
        return Err(query_failure("chain specialization requires an incomplete registered chain expression"));
    }
    if !definition_additional_members_ready_in_view(query, owner)? {
        return Ok(TransformationPlan::Required(vec![Prerequisite::AdditionalMembers {owner_id:owner.id.clone()}]));
    }
    let Some(source) = feature_chain_targets::source_target(query, owner)? else {
        return Ok(TransformationPlan::Ready(None));
    };
    let parameter = first_input(query, owner)?
        .ok_or_else(|| query_failure("source target has no first IN parameter"))?;
    let mut reads = TransformationReads::default();
    let redefinitions = reads.capture(feature_chain_targets::implicit_redefinitions(query, source));
    let result = reads.capture(definition_result_parameter_typed_query(query, owner));
    let chain = reads.capture(expanded_chain(query, &[&parameter.id, &source.id]));
    reads.finish(|| {
        let redefinitions = redefinitions.flatten()
            .ok_or_else(|| query_failure("source target does not occupy the canonical chain role"))?;
        let result = result.flatten().ok_or_else(|| query_failure("chain result is absent after construction"))?;
        Ok(Some(Inputs {
            source:source.id.clone(), parameter:parameter.id.clone(), result:result.id.clone(),
            redefinitions:redefinitions.iter().map(|node| node.id.clone()).collect(),
            chain:chain.ok_or_else(|| query_failure("missing resolved chain plan"))?,
        }))
    })
}

fn has_redefinition(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement, target: &str)
    -> Result<bool, QueryFailure> {
    let mut found = false;
    for relation in stored_children_projection_query(query, owner, "owned_relationship")? {
        if metaclass_conforms(&relation.kind, "Redefinition") {
            if query_reference_in_view(query, relation, "specific")?.id != owner.id {
                return Err(query_failure("source-target redefinition disagrees with canonical owner"));
            }
            found |= query_reference_in_view(query, relation, "general")?.id == target;
        }
    }
    Ok(found)
}

fn has_result_chain(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement, chain: &[String])
    -> Result<bool, QueryFailure> {
    let mut found = false;
    for relation in stored_children_projection_query(query, owner, "owned_relationship")? {
        if metaclass_conforms(&relation.kind, "Subsetting") {
            if query_reference_in_view(query, relation, "specific")?.id != owner.id {
                return Err(query_failure("chain result subsetting disagrees with canonical owner"));
            }
            let general = query_reference_in_view(query, relation, "general")?;
            let signature = definition_chain_reference_targets_typed_query(query, general, "chaining_feature")?
                .iter().map(|node| node.id.clone()).collect::<Vec<_>>();
            found |= signature == chain;
        }
    }
    Ok(found)
}

/// Explicit parent-stage readiness; never used to activate a cold getter.
pub(super) fn ready(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement)
    -> Result<bool, QueryFailure> {
    let Some(plan)=inputs(query,owner)?.into_query_result()? else {return Ok(true);};
    for target in &plan.redefinitions {
        if !has_redefinition(query,query.index[plan.source.as_str()],target)? {return Ok(false);}
    }
    has_result_chain(query,query.index[plan.result.as_str()],&plan.chain)
}

/// Complete all required reads before cloning/writing; the caller discards the
/// entire transaction on failure. Repeated explicit jobs are idempotent.
pub(super) fn materialize(graph: &mut Vec<KirElement>, owner_id: &str, prefix: &str)
    -> Result<bool, QueryFailure> {
    if prefix.is_empty() { return Err(query_failure("chain specialization requires an identity prefix")); }
    if let Some(issue) = ecore_model::validate_publication(graph, ecore_model::ReferenceCompleteness::Closed).first() {
        return Err(query_failure(format!("invalid chain-specialization input: {}", issue.message)));
    }
    let (plan, missing, result_missing) = {
        let index = graph.iter().map(|node| (node.id.as_str(), node)).collect::<BTreeMap<_, _>>();
        let query = DefinitionNameQuery::new(graph, &index);
        let owner = index.get(owner_id).copied().ok_or_else(|| query_failure("chain owner is absent"))?;
        let Some(plan) = inputs(&query, owner)?.into_query_result()? else { return Ok(false); };
        let mut missing = Vec::new();
        for (ordinal, target) in plan.redefinitions.iter().enumerate() {
            if !has_redefinition(&query, index[plan.source.as_str()], target)? {
                missing.push((ordinal, target.clone()));
            }
        }
        let result_missing = !has_result_chain(&query, index[plan.result.as_str()], &plan.chain)?;
        (plan, missing, result_missing)
    };
    if missing.is_empty() && !result_missing { return Ok(false); }
    let mut staged = graph.clone();
    for (ordinal, target) in missing {
        definition_append_implied_specialization(&mut staged, &format!("{prefix}.target.{ordinal}"),
            "Redefinition", &plan.source, &target)?;
    }
    if result_missing {
        let chain_id = format!("{prefix}.result-chain");
        definition_append_feature_chain_query_in_stage(&mut staged, &chain_id, &[&plan.parameter, &plan.source],
            ecore_model::ReferenceCompleteness::Closed)?;
        definition_append_implied_specialization(&mut staged, &format!("{prefix}.result-subsetting"),
            "Subsetting", &plan.result, &chain_id)?;
    }
    let index = staged.iter().map(|node| (node.id.as_str(), node)).collect::<BTreeMap<_, _>>();
    let query = DefinitionNameQuery::new(&staged, &index);
    for target in &plan.redefinitions {
        if !has_redefinition(&query, index[plan.source.as_str()], target)? {
            return Err(query_failure("chain producer omitted a required redefinition"));
        }
    }
    if !has_result_chain(&query, index[plan.result.as_str()], &plan.chain)? {
        return Err(query_failure("chain producer omitted the result chain"));
    }
    *graph = staged;
    Ok(true)
}

/// Admission recomputes canonical context and required endpoints. Identity
/// spelling and producer-origin claims cannot admit an unrelated relationship.
pub(super) fn assessed_relation(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement, relation: &KirElement)
    -> Result<Option<bool>, QueryFailure> {
    if metaclass_conforms(&relation.kind, "Redefinition") {
        if let Some(targets) = feature_chain_targets::implicit_redefinitions(query, owner)? {
            let specific = query_reference_in_view(query, relation, "specific")?;
            let general = query_reference_in_view(query, relation, "general")?;
            return Ok(Some(specific.id == owner.id && targets.iter().any(|target| target.id == general.id)));
        }
    }
    if !metaclass_conforms(&relation.kind, "Subsetting") { return Ok(None); }
    let Some((membership, chain)) = definition_feature_owner_indexed(query.graph.len(), query.index, owner)? else {
        return Ok(None);
    };
    if !metaclass_conforms(&membership.kind, "ReturnParameterMembership")
        || !feature_chain_members::supports(&chain.kind) { return Ok(None); }
    if definition_result_parameter_typed_query(query, chain)?.is_none_or(|result| result.id != owner.id) {
        return Ok(Some(false));
    }
    let Some(source) = feature_chain_targets::source_target(query, chain)? else { return Ok(Some(false)); };
    let Some(parameter) = first_input(query, chain)? else { return Ok(Some(false)); };
    let expected = expanded_chain(query, &[&parameter.id, &source.id])?;
    let general = query_reference_in_view(query, relation, "general")?;
    let actual = definition_chain_reference_targets_typed_query(query, general, "chaining_feature")?
        .iter().map(|node| node.id.clone()).collect::<Vec<_>>();
    Ok(Some(query_reference_in_view(query, relation, "specific")?.id == owner.id && actual == expected))
}

/// A producer-owned general chain is contained by Subsetting, which gives no
/// owning Type. Prove that exact canonical result role before admitting defaults.
/// This supports reads only; no wrapper/receiver lifecycle flags are inferred.
pub(super) fn adopted_result_chain(query: &DefinitionNameQuery<'_, '_>, feature: &KirElement)
    -> Result<bool, QueryFailure> {
    if feature.kind.rsplit("::").next() != Some("Feature") { return Ok(false); }
    let Some(relation) = scope_container_query(query, feature)? else { return Ok(false); };
    if relation.kind.rsplit("::").next() != Some("Subsetting") || !scope_boolean(relation, "is_implied")? { return Ok(false); }
    for field in ["is_end", "is_variable", "is_composite", "is_portion"] {
        if scope_boolean(feature, field)? { return Ok(false); }
    }
    if feature.properties.get("direction").is_some_and(|value| !value.is_null()) { return Ok(false); }
    if canonical_chain_lifecycle::parts(query,feature)?.is_none() {return Ok(false);}
    if query_reference_in_view(query, relation, "general")?.id != feature.id { return Ok(false); }
    let Some(result) = scope_container_query(query, relation)? else { return Ok(false); };
    Ok(assessed_relation(query, result, relation)? == Some(true))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn own(graph: &mut Vec<KirElement>, owner: &str, member: &str, kind: &str, id: &str, child_kind: &str) {
        let mut relationship = fresh_definition_element(member.into(), kind).unwrap();
        let mut child = fresh_definition_element(id.into(), child_kind).unwrap();
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut relationship, &mut child).unwrap();
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,
            graph.iter_mut().find(|node| node.id == owner).unwrap(), &mut relationship).unwrap();
        graph.extend([relationship, child]);
    }
    fn fixture() -> Vec<KirElement> {
        let mut graph = vec![fresh_definition_element("chain".into(), "FeatureChainExpression").unwrap(),
            fresh_definition_element("library".into(), "LibraryPackage").unwrap()];
        graph[1].properties.insert("declared_name".into(), json!("ControlFunctions"));
        graph[1].properties.insert("is_standard".into(), json!(true));
        own(&mut graph, "library", "operator.member", "OwningMembership", "operator", "Function");
        own(&mut graph, "operator", "source.member", "ParameterMembership", "source", "Feature");
        own(&mut graph, "source", "role.member", "FeatureMembership", "role", "Feature");
        for (id, name) in [("operator","."),("source","source"),("role","target")] {
            graph.iter_mut().find(|node| node.id == id).unwrap().properties.insert("declared_name".into(), json!(name));
        }
        own(&mut graph, "chain", "input.member", "ParameterMembership", "input", "Feature");
        own(&mut graph, "input", "source.target.member", "FeatureMembership", "source.target", "Feature");
        own(&mut graph, "chain", "return.member", "ReturnParameterMembership", "result", "Feature");
        own(&mut graph, "chain", "target.member", "OwningMembership", "target", "Feature");
        graph.iter_mut().find(|node| node.id=="input").unwrap().properties.insert("direction".into(),json!("in"));
        graph.iter_mut().find(|node| node.id=="result").unwrap().properties.insert("direction".into(),json!("out"));
        graph
    }
    #[test]
    fn definition_chain_specialization_materializes_ordered_constraints_and_is_idempotent() {
        let mut graph=fixture();
        let flags=graph.iter().map(|node|(node.id.clone(),node.properties["is_implied_included"].clone())).collect::<BTreeMap<_,_>>();
        assert!(materialize(&mut graph,"chain","batch").unwrap());
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();
        let query=DefinitionNameQuery::new(&graph,&index);
        let redefs=definition_redefined_features_typed_query(&query,index["source.target"],false).unwrap();
        assert!(redefs.iter().any(|node|node.id=="role"));
        assert!(redefs.iter().any(|node|node.id=="target"));
        let chain=definition_chain_reference_targets_typed_query(&query,index["batch.result-chain"],"chaining_feature").unwrap();
        assert_eq!(chain.iter().map(|node|node.id.as_str()).collect::<Vec<_>>(),vec!["input","source.target"]);
        for id in ["batch.target.0","batch.target.1"] {
            assert_eq!(assessed_relation(&query,index["source.target"],index[id]).unwrap(),Some(true));
        }
        assert_eq!(assessed_relation(&query,index["result"],index["batch.result-subsetting"]).unwrap(),Some(true));
        assert!(ecore_model::validate_publication(&graph,ecore_model::ReferenceCompleteness::Closed).is_empty());
        let persisted:Vec<KirElement>=serde_json::from_str(&serde_json::to_string(&graph).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(&persisted).unwrap(),serde_json::to_value(&graph).unwrap());
        for (id, flag) in flags {assert_eq!(graph.iter().find(|node|node.id==id).unwrap().properties["is_implied_included"],flag);}
        let before=serde_json::to_value(&graph).unwrap();
        assert!(!materialize(&mut graph,"chain","another-prefix").unwrap());
        assert_eq!(serde_json::to_value(&graph).unwrap(),before);
    }
    #[test]
    fn definition_result_feature_defaults_reuse_canonical_chain_specializations() {
        let mut graph = fixture();
        let library = crate::definition_document::parse_and_link("standard library package Base { feature things; } standard library package Performances { function Evaluation; feature evaluations; }", crate::SourceLanguage::Kerml).unwrap();
        graph.extend(library.elements);
        materialize(&mut graph, "chain", "batch").unwrap();
        let mut reversed = graph.clone(); reversed.reverse();
        let persisted: Vec<KirElement> = serde_json::from_value(serde_json::to_value(&graph).unwrap()).unwrap();
        for model in [&graph, &reversed, &persisted] {
            let before = serde_json::to_value(model).unwrap();
            let index = model.iter().map(|node| (node.id.as_str(), node)).collect();
            let query = DefinitionNameQuery::new(model, &index);
            assert!(adopted_result_chain(&query, index["batch.result-chain"]).unwrap());
            let effects = definition_result_feature_generals_query(&query, index["result"]).unwrap();
            assert_eq!(effects.len(), 1);
            assert_eq!(snapshot_qualified_name(model, index[effects[0].1.as_str()], &mut BTreeSet::new()).unwrap().as_deref(), Some("Base::things"));
            assert_eq!(serde_json::to_value(model).unwrap(), before);
            assert!(!scope_boolean(index["result"], "is_implied_included").unwrap());
        }
        // A valid Feature endpoint and producer-like identity do not authorize
        // an unrelated, shortened or reordered result chain.
        for mutation in ["endpoint", "shortened", "order", "specific"] {
            let mut broken = graph.clone();
            match mutation {
                "endpoint" => { broken.iter_mut().find(|node| node.id == "batch.result-subsetting").unwrap().properties.insert("subsetted_feature".into(), json!("input")); },
                "shortened" => { broken.iter_mut().find(|node| node.id == "batch.result-chain").unwrap().properties.insert("owned_relationship".into(), json!(["batch.result-chain.chain.0"])); },
                "order" => { broken.iter_mut().find(|node| node.id == "batch.result-chain").unwrap().properties.insert("owned_relationship".into(), json!(["batch.result-chain.chain.1", "batch.result-chain.chain.0"])); },
                _ => { broken.iter_mut().find(|node| node.id == "batch.result-subsetting").unwrap().properties.insert("subsetting_feature".into(), json!("input")); },
            }
            let before = serde_json::to_value(&broken).unwrap();
            let index = broken.iter().map(|node| (node.id.as_str(), node)).collect();
            let query = DefinitionNameQuery::new(&broken, &index);
            assert!(matches!(definition_result_feature_generals_query(&query, index["result"]), Err(QueryFailure::Rejected(_))), "{mutation}");
            assert_eq!(serde_json::to_value(&broken).unwrap(), before);
        }
    }

    #[test]
    fn definition_chain_specialization_keeps_pending_reads_and_late_failures_atomic() {
        for mode in ["pending","collision","unsupported","wrong_endpoint","first_empty"] {
            let mut graph=fixture();
            if mode=="pending" {
                let member=graph.iter_mut().find(|node|node.id=="target.member").unwrap();
                member.kind="SysML::Membership".into();
                member.properties.remove("owned_related_element");
                graph.iter_mut().find(|node|node.id=="target").unwrap().properties.remove("owning_relationship");
            } else if mode=="collision" {
                graph.push(fresh_definition_element("batch.result-chain".into(),"Feature").unwrap());
            } else if mode=="unsupported" {graph[0].kind="SysML::OperatorExpression".into();}
            else if mode=="wrong_endpoint" {graph.iter_mut().find(|node|node.id=="role").unwrap().kind="SysML::Class".into();}
            else {
                graph.retain(|node|node.id!="source.target" && node.id!="source.target.member");
                graph.iter_mut().find(|node|node.id=="input").unwrap().properties.insert("owned_relationship".into(),json!([]));
                // Construction supplies the genuinely absent first source target;
                // this pending constructor is distinct from writing constraints.
            }
            let before=serde_json::to_value(&graph).unwrap();
            let result=materialize(&mut graph,"chain","batch");
            if mode=="pending" {
                assert!(matches!(result,Err(QueryFailure::Required(Prerequisite::ReadField{owner_id,field})) if owner_id=="target.member"&&field=="member_element"));
            } else if mode=="first_empty" {
                assert!(matches!(result,Err(QueryFailure::Required(Prerequisite::AdditionalMembers{owner_id})) if owner_id=="chain"));
            } else {assert!(matches!(result,Err(QueryFailure::Rejected(_))),"{mode}: {result:?}");}
            assert_eq!(serde_json::to_value(&graph).unwrap(),before,"{mode}");
        }
    }
    #[test]
    fn definition_chain_specialization_scheduler_reads_before_constructing_and_reuses_jobs() {
        let mut graph=fixture();
        let member=graph.iter_mut().find(|node|node.id=="target.member").unwrap();
        member.kind="SysML::Membership".into();member.properties.remove("owned_related_element");
        graph.iter_mut().find(|node|node.id=="target").unwrap().properties.remove("owning_relationship");
        let refs=vec![crate::xtext_fragment::PendingReference{spelling:"target".into(),span:None,
            target_type:"https://www.omg.org/spec/SysML/20250201#//Feature".into()}];
        let pending=vec![PendingModelLink{owner_id:"target.member".into(),owner_kind:"Membership".into(),
            feature:ecore_model::feature("Membership","member_element").unwrap(),references:&refs,ancestors:vec![]}];
        let root=Prerequisite::ChainSpecialization{owner_id:"chain".into()};
        let reads=std::cell::Cell::new(0);
        let before=serde_json::to_value(&graph).unwrap();
        let (after,remaining)=link_generated_graph_planned(graph.clone(),pending,&|site| {
            assert_eq!(site.owner_id,"target.member");
            assert!(!site.graph.iter().any(|node|node.id.contains("implicit.chain-specialization")));
            reads.set(reads.get()+1);Ok(GeneratedLinkTarget::Local("target".into()))
        },true,&[],&|_|{},Some(&[root.clone(),root])).unwrap();
        assert_eq!(reads.get(),1);assert!(remaining.is_empty());
        assert!(after.iter().any(|node|node.id=="chain.implicit.chain-specialization.result-subsetting"));
        assert_eq!(serde_json::to_value(&graph).unwrap(),before);
    }
    #[test]
    fn definition_chain_specialization_rejects_unrelated_implied_endpoints() {
        let mut graph=fixture();
        materialize(&mut graph,"chain","batch").unwrap();
        for (relation, owner, field) in [("batch.target.0","source.target","redefined_feature"),
            ("batch.result-subsetting","result","subsetted_feature")] {
            let mut broken=graph.clone();
            broken.iter_mut().find(|node|node.id==relation).unwrap().properties.insert(field.into(),json!("input"));
            let index=broken.iter().map(|node|(node.id.as_str(),node)).collect();
            let query=DefinitionNameQuery::new(&broken,&index);
            assert_eq!(assessed_relation(&query,index[owner],index[relation]).unwrap(),Some(false));
            assert!(definition_unassessed_implied_feature_relation_in_view(&query,index[owner],index[relation]).unwrap());
        }
    }
}
