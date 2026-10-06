//! Explicit handwritten execution of the resolved Type.directionOf operation.
//! Ecore supplies the signature/enum/dispatch. Canonical ownership, ordered
//! general-type inputs and conjugation endpoints supply semantic dependencies.
use super::*;

fn contract() -> Result<(), QueryFailure> {
    let operation = ecore_model::direction_of_contract();
    let [feature] = operation.parameters else { return Err(query_failure("unreviewed directionOf parameters")); };
    if operation.name != "directionOf" || operation.owner != "Type" || operation.target != "FeatureDirectionKind"
        || operation.lower != 0 || operation.upper != 1 || operation.ordered || !operation.unique
        || feature.name != "feature" || feature.target != "Feature" || feature.lower != 1 || feature.upper != 1
        || feature.ordered || !feature.unique || operation.delegate_uri != "http://www.omg.org/spec/SysML"
        || operation.binding_status != "dynamic_invocation_candidates_not_selected"
        || operation.branches != [("Type","org.omg.sysml.delegate.invocation.Type_directionOf_InvocationDelegate")] {
        return Err(query_failure("unreviewed directionOf invocation contract"));
    }
    let direction = ecore_model::feature("Feature","direction").ok_or_else(|| query_failure("missing direction enum"))?;
    if direction.lower != 0 || direction.upper != 1 || direction.default_literal.is_some()
        || direction.emf_default_json != r#""in""# {
        return Err(query_failure("unreviewed direction default/nullable contract"));
    }
    let literals = direction.enum_literals.ok_or_else(|| query_failure("direction is not an imported enum"))?;
    if literals.len() != 3 || !["in","out","inout"].iter().all(|literal| literals.contains(literal)) {
        return Err(query_failure("unreviewed direction enum literals"));
    }
    Ok(())
}

pub(super) fn of(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement, feature: &KirElement)
    -> Result<Option<&'static str>, QueryFailure> {
    contract()?;
    if !metaclass_conforms(&owner.kind,"Type") || !metaclass_conforms(&feature.kind,"Feature") {
        return Err(query_failure("directionOf requires Type and Feature arguments"));
    }
    let owner = *query.index.get(owner.id.as_str()).ok_or_else(|| query_failure("direction owner is absent"))?;
    let feature = *query.index.get(feature.id.as_str()).ok_or_else(|| query_failure("direction feature is absent"))?;
    // Depth-first work preserves the delegate's ordered first successful path.
    // One visited set is shared by sibling paths. No recursion-depth guess,
    // computed snapshot or graph mutation is used.
    let mut visited = BTreeSet::new();
    let mut pending = vec![(owner,false)];
    while let Some((current,inverted)) = pending.pop() {
        if !visited.insert(current.id.as_str()) { continue; }
        let conjugator = definition_reference_targets_typed_query(query,current,"owned_conjugator")?;
        if conjugator.len() > 1 { return Err(query_failure("owned conjugator is not scalar")); }
        let owning = definition_reference_targets_typed_query(query,feature,"owning_type")?;
        if owning.first().is_some_and(|owning| owning.id == current.id) {
            // The imported EMF default is IN; generated FeatureImpl initializes
            // direction to null. The existing named stored-direction consumer
            // follows this explicit constructor dependency, independently
            // observed for fresh undirected Features. Never adopt EMF IN here.
            if !definition_has_direction(feature)? { continue; }
            let value = &feature.properties["direction"];
            match value.as_str() {
                Some("in") => return Ok(Some(if inverted {"out"} else {"in"})),
                Some("out") => return Ok(Some(if inverted {"in"} else {"out"})),
                Some("inout") => return Ok(Some("inout")),
                None if value.is_null() => continue,
                _ => return Err(query_failure("invalid stored direction")),
            }
        }
        if let Some(conjugator) = conjugator.first() {
            let original = definition_reference_targets_typed_query(query,conjugator,"original_type")?;
            if original.len() > 1 { return Err(query_failure("conjugated original is not scalar")); }
            if let Some(original) = original.first().copied() {
                if !visited.contains(original.id.as_str()) { pending.push((original,!inverted)); }
            }
        } else {
            let generals = definition_general_type_inputs_with_view(query,current,"")?;
            for identity in generals.into_iter().rev() {
                let general = query.index.get(identity.as_str()).copied().ok_or_else(|| query_failure("direction general is absent"))?;
                if !metaclass_conforms(&general.kind,"Type") { return Err(query_failure("direction general is not a Type")); }
                if !visited.contains(general.id.as_str()) { pending.push((general,inverted)); }
            }
        }
    }
    Ok(None)
}
