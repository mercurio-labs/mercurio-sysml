//! Shared explicit expression contribution stage, selected by imported default
//! programs and Ecore classes. Cold getters do not execute it. Default/result
//! structure is distinct from connectors, validation and lifecycle completion.
use super::*;

pub(super) fn supports(kind: &str) -> bool {
    let kind=kind.rsplit("::").next().unwrap_or(kind);
    compatibility_generated::expression_default_name(kind,false).is_some()
        || instantiation_strategy_generated::base(kind).is_some()
}

fn plan(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement)
    -> Result<TransformationPlan<Vec<(&'static str,String)>>, QueryFailure> {
    if !supports(&owner.kind) || scope_boolean(owner,"is_implied_included")? {
        return Err(query_failure("expression contribution stage requires an incomplete imported dispatch"));
    }
    let mut reads=TransformationReads::default();
    let effects=reads.capture(definition_expression_default_contributions_query(query,owner));
    if parameter_members::construction_supported(owner) {
        if let Some(false)=reads.capture(definition_additional_members_ready_in_view(query,owner)) {
            reads.capture::<()>(Err(QueryFailure::Required(Prerequisite::AdditionalMembers{owner_id:owner.id.clone()})));
        }
    }
    if feature_chain_members::supports(&owner.kind) {
        if let Some(false)=reads.capture(chain_specialization::ready(query,owner)) {
            reads.capture::<()>(Err(QueryFailure::Required(Prerequisite::ChainSpecialization{owner_id:owner.id.clone()})));
        }
    } else if owner.kind.rsplit("::").next()==Some(specialization_construction_generated::RESULT_PROVIDER) {
        if let Some(false)=reads.capture(reference_result_scope::ready(query,owner)) {
            reads.capture::<()>(Err(QueryFailure::Required(Prerequisite::ReferenceResultSubsetting{owner_id:owner.id.clone()})));
        }
    }
    if argument_result_specialization::supports(&owner.kind) {
        if let Some(false)=reads.capture(argument_result_specialization::ready(query,owner)) {
            reads.capture::<()>(Err(QueryFailure::Required(Prerequisite::ArgumentResultSpecializationBatch{owner_ids:vec![owner.id.clone()]})));
        }
    }
    reads.finish(|| effects.ok_or_else(||query_failure("expression contribution plan has no resolved defaults")))
}

/// A ready plan authorizes the existing atomic generalization/reduction worker.
/// Nested member/reference/chain jobs are explicit typed dependencies. This does
/// not mark any receiver complete or assert that connectors/checks ran.
pub(super) fn materialize(graph: &mut Vec<KirElement>, id: &str, prefix: &str)
    -> Result<usize, QueryFailure> {
    materialize_many(graph,&[(id.to_owned(),prefix.to_owned())])
}

pub(super) fn materialize_batch(graph: &mut Vec<KirElement>, ids: &[String])
    -> Result<usize, QueryFailure> {
    let owners=ids.iter().map(|id|(id.clone(),format!("{id}.implicit.expression-contributions"))).collect::<Vec<_>>();
    materialize_many(graph,&owners)
}

fn materialize_many(graph: &mut Vec<KirElement>, owners: &[(String,String)])
    -> Result<usize, QueryFailure> {
    if owners.is_empty() || owners.iter().any(|(_,prefix)|prefix.is_empty()) {
        return Err(query_failure("expression contributions require nonempty owners and prefixes"));
    }
    if owners.iter().map(|(id,_)|id).collect::<BTreeSet<_>>().len()!=owners.len() {
        return Err(query_failure("duplicate expression contribution owner"));
    }
    let plans={
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
        if index.len()!=graph.len() {return Err(query_failure("duplicate expression contribution identity"));}
        let query=DefinitionNameQuery::new(graph,&index);
        let mut reads=TransformationReads::default();let mut plans=Vec::new();
        for (id,prefix) in owners {
            let owner=index.get(id.as_str()).copied().ok_or_else(||query_failure("expression contribution owner is absent"))?;
            if let Some(effects)=reads.capture_plan(plan(&query,owner)) {
                plans.push((id.clone(),prefix.clone(),effects));
            }
        }
        reads.finish(||Ok(plans))?.into_query_result()?
    };
    let selections=plans.iter().map(|(_,_,effects)|effects.iter()
        .map(|(kind,target)|(*kind,target.as_str())).collect::<Vec<_>>()).collect::<Vec<_>>();
    let batches=plans.iter().zip(&selections).map(|((id,prefix,_),effects)|
        (id.as_str(),prefix.as_str(),effects.as_slice())).collect::<Vec<_>>();
    definition_materialize_general_batch_query(graph,&batches,ecore_model::ReferenceCompleteness::Closed)
}

/// Admission uses exactly the same imported producer and canonical endpoint
/// contracts. A familiar identity or implied flag cannot admit unrelated edges.
pub(super) fn assessed_relation(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement, relation: &KirElement)
    -> Result<Option<bool>, QueryFailure> {
    let kind=relation.kind.rsplit("::").next().unwrap_or(&relation.kind);
    if !supports(&owner.kind) || !matches!(kind,"Subsetting"|"FeatureTyping") {return Ok(None);}
    let effects=definition_expression_default_contributions_query(query,owner)?;
    let targets=effects.iter().filter(|(candidate,_)|*candidate==kind).map(|(_,id)|id.clone()).collect::<Vec<_>>();
    Ok(Some(definition_assessed_type_default_relation_typed_in_view(query,owner,relation,kind,&targets)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn own(graph: &mut Vec<KirElement>, owner: &str, member: &str, membership: &str, id: &str, kind: &str) {
        let mut relation=fresh_definition_element(member.into(),membership).unwrap();
        let mut node=fresh_definition_element(id.into(),kind).unwrap();
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut relation,&mut node).unwrap();
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,
            graph.iter_mut().find(|node|node.id==owner).unwrap(),&mut relation).unwrap();
        graph.extend([relation,node]);
    }
    fn fixture()->Vec<KirElement> {
        let mut graph=vec![fresh_definition_element("performances".into(),"LibraryPackage").unwrap()];
        graph[0].properties.insert("declared_name".into(),json!("Performances"));
        graph[0].properties.insert("is_standard".into(),json!(true));
        for name in ["literalIntegerEvaluations","literalBooleanEvaluations","nullEvaluations","evaluations"] {
            own(&mut graph,"performances",&format!("{name}.member"),"OwningMembership",name,"Feature");
            let prototype=graph.iter_mut().find(|node|node.id==name).unwrap();
            prototype.properties.insert("declared_name".into(),json!(name));
            // Materialized unit prototypes only; actual pinned libraries remain
            // unprepared in the independent category qualification run.
            prototype.properties.insert("is_implied_included".into(),json!(true));
        }
        graph
    }
    #[test]
    fn definition_expression_contributions_share_imported_dispatch_and_idempotent_native_defaults() {
        for kind in ["LiteralInteger","LiteralBoolean","NullExpression"] {
            let mut graph=fixture();graph.push(fresh_definition_element("expression".into(),kind).unwrap());
            assert_eq!(materialize(&mut graph,"expression","batch").unwrap(),1);
            let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&graph,&index);
            assert_eq!(assessed_relation(&query,index["expression"],index["batch.0"]).unwrap(),Some(true));
            assert!(!definition_unassessed_implied_feature_relation_in_view(&query,index["expression"],index["batch.0"]).unwrap());
            assert!(!scope_boolean(index["expression"],"is_implied_included").unwrap());
            let encoded=serde_json::to_string(&graph).unwrap();let restored:Vec<KirElement>=serde_json::from_str(&encoded).unwrap();
            assert_eq!(serde_json::to_value(&restored).unwrap(),serde_json::to_value(&graph).unwrap());
            let before=serde_json::to_value(&graph).unwrap();assert_eq!(materialize(&mut graph,"expression","again").unwrap(),0);
            assert_eq!(serde_json::to_value(&graph).unwrap(),before);
        }
        let mut graph=fixture();
        for (id,kind) in [("a","LiteralInteger"),("b","LiteralBoolean")] {
            graph.push(fresh_definition_element(id.into(),kind).unwrap());
        }
        let owners=vec!["a".to_owned(),"b".to_owned()];
        assert_eq!(materialize_batch(&mut graph,&owners).unwrap(),2);
        assert_eq!(materialize_batch(&mut graph,&owners).unwrap(),0);
    }
    #[test]
    fn definition_expression_contributions_keep_typed_default_reads_and_reject_unrelated_edges() {
        let mut graph=fixture();graph.push(fresh_definition_element("expression".into(),"LiteralInteger").unwrap());
        let member=graph.iter_mut().find(|node|node.id=="literalIntegerEvaluations.member").unwrap();
        member.kind="SysML::Membership".into();member.properties.remove("owned_related_element");
        member.properties.insert("member_name".into(),json!("literalIntegerEvaluations"));
        graph.iter_mut().find(|node|node.id=="literalIntegerEvaluations").unwrap().properties.remove("owning_relationship");
        let before=serde_json::to_value(&graph).unwrap();
        assert!(matches!(materialize(&mut graph,"expression","batch"),Err(QueryFailure::Required(Prerequisite::ReadField{owner_id,field}))
            if owner_id=="literalIntegerEvaluations.member"&&field=="member_element"));
        assert_eq!(serde_json::to_value(&graph).unwrap(),before);
        graph.iter_mut().find(|node|node.id=="literalIntegerEvaluations.member").unwrap().properties.insert("member_element".into(),json!("literalIntegerEvaluations"));
        materialize(&mut graph,"expression","batch").unwrap();
        graph.iter_mut().find(|node|node.id=="batch.0").unwrap().properties.insert("subsetted_feature".into(),json!("nullEvaluations"));
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&graph,&index);
        assert!(matches!(assessed_relation(&query,index["expression"],index["batch.0"]),Err(QueryFailure::Rejected(_))));
    }
    #[test]
    fn definition_expression_contributions_scheduler_discards_a_late_failure_across_receivers() {
        let mut graph=fixture();
        for id in ["a","b"] {graph.push(fresh_definition_element(id.into(),"LiteralInteger").unwrap());}
        graph.push(fresh_definition_element("b.implicit.expression-contributions.0".into(),"Feature").unwrap());
        let before=serde_json::to_value(&graph).unwrap();
        let roots=[Prerequisite::ExpressionContributionBatch{owner_ids:vec!["a".into(),"b".into()]}];
        let error=link_generated_graph_planned(graph.clone(),Vec::new(),&|_|Err(error("no lexical reads requested")),
            true,&[],&|_|{},Some(&roots)).err().expect("late collision must reject");
        assert!(error.message.contains("fresh")||error.message.contains("collision"),"{}",error.message);
        assert_eq!(serde_json::to_value(&graph).unwrap(),before);
    }
    #[test]
    fn definition_expression_contributions_include_registered_chain_dependencies_without_flags() {
        let mut graph=fixture();
        let mut cf=fresh_definition_element("control".into(),"LibraryPackage").unwrap();
        cf.properties.insert("declared_name".into(),json!("ControlFunctions"));cf.properties.insert("is_standard".into(),json!(true));graph.push(cf);
        own(&mut graph,"control","operator.member","OwningMembership","operator","Function");
        own(&mut graph,"operator","source.member","ParameterMembership","source","Feature");
        own(&mut graph,"source","role.member","FeatureMembership","role","Feature");
        for (id,name) in [("operator","."),("source","source"),("role","target")] {
            let node=graph.iter_mut().find(|node|node.id==id).unwrap();
            node.properties.insert("declared_name".into(),json!(name));node.properties.insert("is_implied_included".into(),json!(true));
        }
        graph.push(fresh_definition_element("chain".into(),"FeatureChainExpression").unwrap());
        own(&mut graph,"chain","input.member","ParameterMembership","input","Feature");
        own(&mut graph,"input","source.target.member","FeatureMembership","source.target","Feature");
        own(&mut graph,"chain","return.member","ReturnParameterMembership","result","Feature");
        own(&mut graph,"chain","target.member","OwningMembership","target","Feature");
        graph.iter_mut().find(|node|node.id=="input").unwrap().properties.insert("direction".into(),json!("in"));
        graph.iter_mut().find(|node|node.id=="result").unwrap().properties.insert("direction".into(),json!("out"));
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&graph,&index);
        assert!(matches!(plan(&query,index["chain"]).unwrap(),TransformationPlan::Required(ref jobs)
            if jobs.contains(&Prerequisite::ChainSpecialization{owner_id:"chain".into()})));
        let roots=[Prerequisite::ExpressionContributions{owner_id:"chain".into()}];
        let (graph,remaining)=link_generated_graph_planned(graph,Vec::new(),&|_|Err(error("no lexical reads expected")),
            true,&[],&|_|{},Some(&roots)).unwrap();
        assert!(remaining.is_empty());
        assert!(graph.iter().any(|node|node.id=="chain.implicit.chain-specialization.result-subsetting"));
        assert!(graph.iter().any(|node|node.id=="chain.implicit.expression-contributions.0"));
        assert!(!scope_boolean(graph.iter().find(|node|node.id=="chain").unwrap(),"is_implied_included").unwrap());
    }
}
