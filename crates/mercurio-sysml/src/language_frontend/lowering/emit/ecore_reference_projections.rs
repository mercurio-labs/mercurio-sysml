//! Shared handwritten execution for imported Ecore subset-list fallback bindings.
//! Annotation source order and fallback class come from generated contracts.
//! Source getters retain their exact semantic dependencies; no missing source is
//! replaced with an empty list. These projections never store derived snapshots.
use super::*;

pub(super) fn project<'g>(query: &DefinitionNameQuery<'g, '_>, owner: &'g KirElement,
    contract: &ecore_model::FeatureContract) -> Result<Option<Vec<&'g KirElement>>, QueryFailure> {
    if let Some(source) = ecore_model::default_projection_source(contract) {
        let key = (owner.id.clone(), contract.id.to_owned());
        if !query.active_reference_projections.borrow_mut().insert(key.clone()) {
            return Err(query_failure("recursive default derived-property source"));
        }
        let result = (|| {
            // The pinned default delegates read the first direct subset source.
            // Ecore redefinitions and the source's algorithm remain authoritative.
            let values = definition_reference_targets_typed_query(query, owner, source.field)?;
            let mut seen = BTreeSet::new();
            Ok(values.into_iter().filter(|target|
                metaclass_conforms(&target.kind, contract.target) && seen.insert(target.id.as_str()))
                .collect::<Vec<_>>())
        })();
        query.active_reference_projections.borrow_mut().remove(&key);
        return result.map(Some);
    }
    if contract.owner == "Namespace" && contract.field == "membership" {
        let sources = ecore_model::membership_union_sources(&owner.kind).map_err(error)?;
        let mut members = Vec::new();
        let mut seen = BTreeSet::new();
        for (source_owner, source_field) in sources {
            if !metaclass_conforms(&owner.kind, source_owner) { continue; }
            for member in definition_reference_targets_typed_query(query,owner,source_field)? {
                if seen.insert(member.id.as_str()) { members.push(member); }
            }
        }
        return Ok(Some(members));
    }
    if contract.owner == "Namespace" && contract.field == "imported_membership" {
        custom_binding(contract)?;
        return match scope_memberships_query(query.graph,owner,true,false,"",&BTreeSet::new(),
            &mut BTreeSet::new(),MembershipScopeMode::CanonicalImported,query,&|_| {})? {
            Members::Ready(members) => Ok(Some(members)),
            Members::Deferred(prerequisite) => Err(query_requirement(prerequisite)),
        };
    }
    if contract.owner == "Type" && matches!(contract.field,"input"|"output") {
        custom_binding(contract)?;
        let mut parameters = Vec::new();
        for feature in definition_effective_features_typed_query(query,owner)? {
            let direction = type_direction::of(query,owner,feature)?;
            if direction == Some("inout") || direction == Some(if contract.field == "input" {"in"} else {"out"}) {
                parameters.push(feature);
            }
        }
        return Ok(Some(parameters));
    }
    // Shared structural delegates for the complete lifecycle read bundle.
    // Imported contracts select the exact binding; reciprocal containment and
    // selection algorithms are explicit handwritten semantic dependencies.
    if matches!((contract.owner, contract.field),
        ("Element", "owning_namespace") | ("Relationship", "related_element")
        | ("Namespace", "owned_member") | ("Type", "multiplicity")
        | ("Feature", "owned_cross_subsetting") | ("FeatureValue", "feature_with_value")) {
        custom_binding(contract)?;
        let targets = match (contract.owner, contract.field) {
            ("Element", "owning_namespace") => {
                let member = scope_container_query(query, owner)?
                    .filter(|parent| metaclass_conforms(&parent.kind, "OwningMembership"));
                match member {
                    None => Vec::new(),
                    Some(member) => {
                        let endpoint = definition_reference_targets_typed_query(query, member, "member_element")?;
                        if endpoint.len() != 1 || endpoint[0].id != owner.id {
                            return Err(query_failure("owning namespace disagrees with canonical member endpoint"));
                        }
                        definition_reference_targets_typed_query(query, member, "membership_owning_namespace")?
                    },
                }
            },
            ("Relationship", "related_element") => {
                // This is a Sequence union, not an identity set. Source/target
                // redefinitions retain their native algorithms and typed reads.
                let mut related = definition_reference_targets_typed_query(query, owner, "source")?;
                related.extend(definition_reference_targets_typed_query(query, owner, "target")?);
                related
            },
            ("Namespace", "owned_member") => {
                let mut members = Vec::new();
                for membership in definition_reference_targets_typed_query(query, owner, "owned_membership")? {
                    if metaclass_conforms(&membership.kind, "OwningMembership") {
                        members.extend(definition_reference_targets_typed_query(query, membership, "member_element")?);
                    }
                }
                members
            },
            ("Type", "multiplicity") =>
                // KerML formal deriveTypeMultiplicity uses ownedMember, not
                // the separate inherited TypeUtil.getMultiplicityOf algorithm.
                definition_reference_targets_typed_query(query, owner, "owned_member")?
                    .into_iter().find(|member| metaclass_conforms(&member.kind, contract.target))
                    .into_iter().collect(),
            ("Feature", "owned_cross_subsetting") =>
                definition_reference_targets_typed_query(query, owner, "owned_subsetting")?
                    .into_iter().find(|relationship| metaclass_conforms(&relationship.kind, contract.target))
                    .into_iter().collect(),
            ("FeatureValue", "feature_with_value") =>
                definition_reference_targets_typed_query(query, owner, "membership_owning_namespace")?
                    .into_iter().filter(|namespace| metaclass_conforms(&namespace.kind, contract.target)).collect(),
            _ => unreachable!("reviewed structural delegate dispatch"),
        };
        let mut targets: Vec<_> = targets;
        if contract.unique {
            let mut seen = BTreeSet::new();
            targets.retain(|target| seen.insert(target.id.as_str()));
        }
        return Ok(Some(targets));
    }
    // Reviewed named delegates supplying all six terminal sources of the
    // pinned default-list bundle. Target types/order/uniqueness are imported;
    // canonical reads and endpoint predicates remain handwritten dependencies.
    if matches!((contract.owner, contract.field),
        ("Namespace", "owned_membership") | ("Type", "owned_specialization")
        | ("Element", "owned_element") | ("Type", "feature_membership")
        | ("Type", "inherited_membership") | ("Type", "directed_feature")) {
        custom_binding(contract)?;
        let targets = match (contract.owner, contract.field) {
            ("Namespace", "owned_membership") =>
                stored_children_projection_query(query, owner, "owned_relationship")?.into_iter()
                    .filter(|child| metaclass_conforms(&child.kind, contract.target)).collect(),
            ("Element", "owned_element") => {
                let mut children = Vec::new();
                for relationship in stored_children_projection_query(query, owner, "owned_relationship")? {
                    children.extend(stored_children_projection_query(query, relationship, "owned_related_element")?);
                }
                children
            },
            ("Type", "owned_specialization") => {
                let mut relationships = Vec::new();
                for relationship in stored_children_projection_query(query, owner, "owned_relationship")? {
                    if metaclass_conforms(&relationship.kind, contract.target) {
                        let specific = definition_reference_targets_typed_query(query, relationship, "specific")?;
                        if specific.len() > 1 { return Err(query_failure("specialization specific is not scalar")); }
                        if specific.first().is_some_and(|specific| specific.id == owner.id) {
                            relationships.push(relationship);
                        }
                    }
                }
                relationships
            },
            ("Type", "feature_membership") =>
                definition_effective_feature_memberships_typed_query(query, owner)?,
            ("Type", "inherited_membership") =>
                match definition_inherited_memberships(query, owner, "", &BTreeSet::new())? {
                    Members::Ready(members) => members,
                    Members::Deferred(prerequisite) => return Err(query_requirement(prerequisite)),
                },
            ("Type", "directed_feature") =>
                definition_parameter_collection_typed_query(query, owner, false)?,
            _ => unreachable!("reviewed source dispatch"),
        };
        let mut targets: Vec<_> = targets;
        if contract.unique {
            let mut seen = BTreeSet::new();
            targets.retain(|target| seen.insert(target.id.as_str()));
        }
        return Ok(Some(targets));
    }
    if !metaclass_conforms(&owner.kind, "Feature")
        || !matches!(contract.field, "owning_feature_membership" | "owning_type") {
        return Ok(None);
    }
    let expected = format!("org.omg.sysml.delegate.setting.Feature_{}_SettingDelegate", contract.name);
    if !contract.setting_delegate.as_ref().is_some_and(|binding|
        binding.uri == "http://www.omg.org/spec/SysML"
        && binding.status == "custom_setting_delegate_source"
        && binding.candidates == [expected.as_str()]) {
        return Err(query_failure("unreviewed canonical Feature ownership delegate"));
    }
    let membership = scope_container_query(query, owner)?
        .filter(|parent| metaclass_conforms(&parent.kind, "FeatureMembership"));
    let Some(membership) = membership else { return Ok(Some(Vec::new())); };
    let StoredMembershipEndpoint::Resolved(target) =
        stored_membership_endpoint_indexed(query.graph.len(), query.index, membership, "member_element")?
    else { return Err(query_failure("owning Feature membership endpoint is unavailable")); };
    if target.id != owner.id {
        return Err(query_failure("owning Feature membership disagrees with canonical endpoint"));
    }
    let targets = if contract.field == "owning_feature_membership" {
        vec![membership]
    } else {
        scope_container_query(query, membership)?
            .filter(|parent| metaclass_conforms(&parent.kind, contract.target)).into_iter().collect()
    };
    Ok(Some(targets))
}


fn custom_binding(contract: &ecore_model::FeatureContract) -> Result<(), QueryFailure> {
    let expected = format!("org.omg.sysml.delegate.setting.{}_{}_SettingDelegate", contract.owner, contract.name);
    if !contract.setting_delegate.as_ref().is_some_and(|binding|
        binding.uri == "http://www.omg.org/spec/SysML"
        && binding.status == "custom_setting_delegate_source"
        && binding.candidates == [expected.as_str()]) {
        return Err(query_failure("unreviewed shared source getter binding"));
    }
    Ok(())
}
