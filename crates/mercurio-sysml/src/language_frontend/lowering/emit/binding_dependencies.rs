//! Read-before-write dependencies shared by fresh binary binding producers.
//! Imported connector defaults select the library Feature; typed semantic reads
//! establish its effective ends and the target result's typing. Recording the
//! dependencies never substitutes for constructing or validating a binding.
use super::*;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct BindingDependencies {
    pub(super) default_feature: String,
    pub(super) default_types: Vec<String>,
    pub(super) effective_ends: Vec<String>,
    pub(super) result_types: Vec<String>,
}

pub(super) fn binding_dependencies_plan(query: &DefinitionNameQuery<'_, '_>, result: &KirElement)
    -> Result<TransformationPlan<BindingDependencies>, QueryFailure> {
    let name = connector_defaults_generated::default_for("BindingConnector", 2, false)
        .ok_or_else(|| query_failure("fresh binding has no imported binary default"))?;
    if connector_defaults_generated::default_for("BindingConnector", 2, true) != Some(name) {
        return Err(query_failure("binary default prerequisites require a resolved structure classification"));
    }
    let target_id = query_local_binding(standard_default_binding_query(query, name)?)?;
    let target = query.index.get(target_id.as_str()).copied().ok_or_else(|| query_failure("binding default is absent"))?;
    ecore_model::validate_reference_endpoint("Subsetting", "general", &target.kind).map_err(query_failure)?;
    let ids = |nodes: Vec<&KirElement>| nodes.into_iter().map(|node| node.id.clone()).collect::<Vec<_>>();
    let mut reads = TransformationReads::default();
    let default_types = reads.capture(definition_feature_types_in_view(query, target));
    let effective_ends = reads.capture(definition_end_features_typed_query(query, target, false));
    let result_types = reads.capture(definition_feature_types_in_view(query, result));
    reads.finish(|| Ok(BindingDependencies {
        default_feature: target_id,
        default_types: ids(default_types.ok_or_else(|| query_failure("binding plan default types absent"))?),
        effective_ends: ids(effective_ends.ok_or_else(|| query_failure("binding plan effective ends absent"))?),
        result_types: ids(result_types.ok_or_else(|| query_failure("binding plan result types absent"))?),
    }))
}

// A deterministic name alone is never a replay proof. Prove canonical placement,
// ordered endpoints, fresh fixed roles, and successful public connector queries.
pub(super) fn verify_completed_binding(query:&DefinitionNameQuery<'_, '_>, owner:&KirElement, binding:&KirElement,
    source:&KirElement, target:&KirElement)->Result<(),QueryFailure> {
    if binding.kind.rsplit("::").next()!=Some("BindingConnector")
        || !scope_boolean(binding,"is_implied")? || !scope_boolean(binding,"is_implied_included")? {
        return Err(query_failure("reference binding replay requires a canonical completed fresh connector"));
    }
    for field in ["is_end","is_variable","is_composite","is_portion"] {
        if scope_boolean(binding,field)? {return Err(query_failure("reference binding replay has incompatible fixed-role flags"));}
    }
    let member=scope_container_query(query,binding)?.ok_or_else(||query_failure("reference binding has no placement membership"))?;
    let namespace=scope_container_query(query,member)?.ok_or_else(||query_failure("reference binding membership has no owner"))?;
    if namespace.id!=owner.id {return Err(query_failure("reference binding belongs to another owner"));}
    let context=definition_related_feature_context_in_view(query,&[source,target],true)?;
    let kind=if context.is_some_and(|node|node.id==owner.id) {"FeatureMembership"}else{"OwningMembership"};
    if member.kind.rsplit("::").next()!=Some(kind) {return Err(query_failure("reference binding has incorrect placement kind"));}
    let ends=definition_owned_features_query(query,binding,true)?;
    if ends.len()!=2 {return Err(query_failure("reference binding must have exactly two ordered owned ends"));}
    for (end,endpoint) in ends.iter().zip([source,target]) {
        if !scope_boolean(end,"is_end")? || !scope_boolean(end,"is_implied_included")? {
            return Err(query_failure("reference binding replay has incomplete end roles"));
        }
        let references=stored_children_projection_query(query,end,"owned_relationship")?.into_iter()
            .filter(|relation|relation.kind.rsplit("::").next()==Some("ReferenceSubsetting")).collect::<Vec<_>>();
        if references.len()!=1 || query_reference_in_view(query,references[0],"referenced_feature")?.id!=endpoint.id
            || query_reference_in_view(query,references[0],"referencing_feature")?.id!=end.id {
            return Err(query_failure("reference binding replay has incompatible ordered endpoints"));
        }
    }
    // Completion flags alone cannot discharge missing default contributions.
    // Re-run the imported selector and shared reduction over all three roles.
    for node in std::iter::once(binding).chain(ends.iter().copied()) {
        let name=if node.id==binding.id {definition_connector_default_name_in_view(query,node)?}
            else {definition_feature_default_name_typed_query(query.graph,node)?};
        let general=query_local_binding(standard_default_binding_query(query,name)?)?;
        let effects=definition_implicit_feature_generals_in_view(query,node,&general)?;
        let mut seen=BTreeSet::new();
        let effects=effects.into_iter().filter(|effect|seen.insert(effect.clone())).collect::<Vec<_>>();
        let pending=effects.iter().map(|(kind,id)|(*kind,id.as_str())).collect::<Vec<_>>();
        if !definition_reduce_implicit_generals_query(query.graph,node,&pending)?.is_empty() {
            return Err(query_failure("reference binding replay is missing an imported semantic contribution"));
        }
    }
    for (field,expected) in [("source",vec![source.id.as_str()]),("target",vec![target.id.as_str()]),
        ("related_feature",vec![source.id.as_str(),target.id.as_str()])] {
        let actual=definition_reference_targets_typed_query(query,binding,field)?;
        if actual.iter().map(|node|node.id.as_str()).collect::<Vec<_>>()!=expected {
            return Err(query_failure("reference binding replay has incompatible public endpoint projections"));
        }
    }
    Ok(())
}

