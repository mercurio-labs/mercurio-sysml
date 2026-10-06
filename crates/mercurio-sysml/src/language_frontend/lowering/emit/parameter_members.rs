//! Handwritten TypeUtil/FeatureUtil query dependencies, selected by imported
//! Ecore contracts and resolved parameter predicates. No lifecycle flags are
//! prepared here; missing existing constructors become concrete scheduler jobs.
use super::*;

pub(super) fn construction_supported(owner: &KirElement) -> bool {
    let kind = owner.kind.rsplit("::").next().unwrap_or(&owner.kind);
    result_construction_generated::supported(kind)
        || feature_chain_members::supports(&owner.kind)
        || multiplicity_construction::supports(&owner.kind)
        || kind == "TransitionUsage"
}

fn basic_feature<'g>(query: &DefinitionNameQuery<'g, '_>, owner: &'g KirElement)
    -> Result<&'g KirElement, QueryFailure>
{
    if !metaclass_conforms(&owner.kind, "Feature") { return Ok(owner); }
    let contract = ecore_model::feature(&owner.kind, "owned_feature_chaining")
        .ok_or_else(|| query_failure("missing owned chain contract"))?;
    let binding = contract.setting_delegate.as_ref()
        .ok_or_else(|| query_failure("missing owned chain delegate"))?;
    if binding.uri != "http://www.omg.org/spec/SysML"
        || binding.status != "default_setting_delegate_fallback_source"
        || !binding.candidates.is_empty() {
        return Err(query_failure("unsupported basic-feature chain delegate"));
    }
    // Exact upstream dependency: last direct owned chaining once. Neither an
    // earlier unresolved target nor a target's own chain is evaluated here.
    let relationships = stored_children_projection_query(query, owner, "owned_relationship")?;
    match relationships.iter().rev().find(|r| metaclass_conforms(&r.kind, contract.target)) {
        Some(chain) => query_reference_in_view(query, chain, "chaining_feature"),
        None => Ok(owner),
    }
}

pub(super) fn owned_parameters<'g>(query: &DefinitionNameQuery<'g, '_>, owner: &'g KirElement)
    -> Result<Vec<&'g KirElement>, QueryFailure>
{
    if !metaclass_conforms(&owner.kind, "Type") {
        return Err(query_failure("parameter owner is not a Type"));
    }
    let owner = basic_feature(query, owner)?;
    let mut parameters = Vec::new();
    for feature in definition_owned_features_query(query, owner, false)? {
        let Some((_, owning_type)) =
            definition_feature_owner_indexed(query.graph.len(), query.index, feature)?
        else { continue; };
        if parameter_collections_generated::is_parameter(
            metaclass_conforms(&owning_type.kind, parameter_collections_generated::OWNER_0),
            metaclass_conforms(&owning_type.kind, parameter_collections_generated::OWNER_1),
            definition_has_direction(feature)?) {
            parameters.push(feature);
        }
    }
    Ok(parameters)
}

pub(super) fn owned_result<'g>(query: &DefinitionNameQuery<'g, '_>, identity: &str)
    -> Result<Option<&'g KirElement>, QueryFailure>
{
    let owner = *query.index.get(identity)
        .ok_or_else(|| query_failure("missing result owner"))?;
    for feature in owned_parameters(query, owner)? {
        if definition_is_result_parameter_query(query, feature)? { return Ok(Some(feature)); }
    }
    Ok(None)
}
