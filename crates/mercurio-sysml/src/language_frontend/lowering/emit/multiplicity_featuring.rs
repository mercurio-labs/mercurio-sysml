//! Explicit handwritten MultiplicityAdapter featuring dependency. Pinned Xtext
//! places ranges in ordinary OwningMemberships; Ecore faithfully preserves that
//! absence of owningType. The upstream transformation separately propagates a
//! Feature namespace's featuring types. Cold getters remain cold: this producer
//! is an explicit scheduler job and never completes the range or its owner.
use super::*;

fn plan(query:&DefinitionNameQuery<'_, '_>,owner:&KirElement)->Result<expression_featuring::FeaturingPlan,QueryFailure> {
    if !metaclass_conforms(&owner.kind,"Multiplicity") || scope_boolean(owner,"is_implied_included")? {
        return Err(query_failure("multiplicity featuring requires an incomplete Multiplicity"));
    }
    let member=scope_container_query(query,owner)?.ok_or_else(||query_failure("multiplicity featuring requires canonical ownership"))?;
    if member.kind.rsplit("::").next()!=Some("OwningMembership") {
        return Err(query_failure("multiplicity featuring requires the pinned ordinary membership; owning-Type and crossing strategies remain separate"));
    }
    let namespace=scope_container_query(query,member)?.ok_or_else(||query_failure("multiplicity membership has no namespace"))?;
    if metaclass_conforms(&namespace.kind,"Feature") {
        if let Some(end)=generated_parent_namespace_query(query,&namespace.id)? {
            if definition_owned_cross_feature_in_view(query,end)?.is_some_and(|cross|cross.id==namespace.id) {
                return Err(query_failure("cross-feature multiplicity bounds require the owning end featuring strategy"));
            }
        }
    }
    let existing=definition_owned_type_featuring_in_view(query,owner)?.iter()
        .map(|relation|query_reference_in_view(query,relation,"featuring_type").map(|target|target.id.clone()))
        .collect::<Result<BTreeSet<_>,_>>()?;
    let mut expected=definition_featuring_types_in_view(query,owner)?.iter().map(|target|target.id.clone()).collect::<Vec<_>>();
    if metaclass_conforms(&namespace.kind,"Feature") {
        expected.extend(definition_featuring_types_in_view(query,namespace)?.iter().map(|target|target.id.clone()));
    }
    let contract=ecore_model::feature(&owner.kind,"featuring_type").ok_or_else(||query_failure("missing multiplicity featuring contract"))?;
    if contract.unique {let mut seen=BTreeSet::new();expected.retain(|id|seen.insert(id.clone()));}
    let missing=expected.iter().filter(|id|!existing.contains(*id)).cloned().collect();
    Ok(expression_featuring::FeaturingPlan{owner:owner.id.clone(),expected,missing})
}

pub(super) fn ready(query:&DefinitionNameQuery<'_, '_>,owner:&KirElement)->Result<bool,QueryFailure> {
    Ok(plan(query,owner)?.missing.is_empty())
}

pub(super) fn materialize(graph:&mut Vec<KirElement>,id:&str)->Result<usize,QueryFailure> {
    let plan={
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
        if index.len()!=graph.len(){return Err(query_failure("duplicate multiplicity featuring identity"));}
        let query=DefinitionNameQuery::new(graph,&index);
        let owner=index.get(id).copied().ok_or_else(||query_failure("multiplicity featuring owner is absent"))?;
        plan(&query,owner)?
    };
    expression_featuring::materialize_plans(graph,&[plan])
}
