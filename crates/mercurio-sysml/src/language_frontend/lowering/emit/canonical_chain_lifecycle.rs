//! Shared handwritten lifecycle for canonical producer-owned, fixed chains.
//! Imported Ecore contracts govern endpoints/containment and the resolved Feature
//! selector supplies defaults. Ordered chain identity and assessed contributions
//! are separate. This does not complete a source owner or qualify validation.
use super::*;

/// Assess stored contributions without reading pending chaining endpoints. Source
/// CrossSubsettings can contain one segment and participate in reciprocal scope
/// linking. Their endpoint prerequisites belong to the reference scheduler.
/// Producer lifecycle admission additionally requires resolved ordered parts.
pub(super) fn assessed_wrapper(query: &DefinitionNameQuery<'_, '_>, chain: &KirElement)
    -> Result<bool, QueryFailure> {
    if chain.kind.rsplit("::").next()!=Some("Feature") {return Ok(false);}
    for field in ["is_end","is_variable","is_composite","is_portion"] {
        if scope_boolean(chain,field)? {return Ok(false);}
    }
    if chain.properties.get("direction").is_some_and(|value|!value.is_null()) {return Ok(false);}
    let relations=stored_children_projection_query(query,chain,"owned_relationship")?;
    let mut default_seen=false;
    let mut chaining_count=0;
    for relation in relations {
        if relation.kind.rsplit("::").next()==Some("FeatureChaining") {
            if !stored_children_projection_query(query,relation,"owned_related_element")?.is_empty() {return Ok(false);}
            chaining_count+=1;
            continue;
        }
        if relation.kind.rsplit("::").next()!=Some("Subsetting")
            || !scope_boolean(relation,"is_implied")? || default_seen {return Ok(false);}
        let name=feature_defaults_generated::feature_default_name(false,false,false,false,false,false);
        let id=query_local_binding(standard_default_binding_query(query,name)?)?;
        if !definition_assessed_type_default_relation_typed_in_view(query,chain,relation,"Subsetting",&[id])? {return Ok(false);}
        default_seen=true;
    }
    Ok(chaining_count>0)
}

/// Select fully resolved producer-owned chains. Stored contribution assessment
/// alone cannot authorize transformation, nor can completion flags or identity.
pub(super) fn parts(query: &DefinitionNameQuery<'_, '_>, chain: &KirElement)
    -> Result<Option<Vec<String>>, QueryFailure> {
    if !assessed_wrapper(query,chain)? {return Ok(None);}
    let targets=definition_chain_reference_targets_typed_query(query,chain,"chaining_feature")?
        .iter().map(|node|node.id.clone()).collect::<Vec<_>>();
    if targets.len()<2 || targets.iter().any(|id|id==&chain.id) {return Ok(None);}
    Ok(Some(targets))
}

pub(super) fn exact(query: &DefinitionNameQuery<'_, '_>, chain: &KirElement, expected: &[String])
    -> Result<bool, QueryFailure> {
    Ok(parts(query,chain)?.is_some_and(|actual|actual==expected))
}

pub(super) fn adopted(query: &DefinitionNameQuery<'_, '_>, chain: &KirElement)->Result<bool,QueryFailure> {
    Ok(general_value_binding::adopted_chain(query,chain)?
        || literal_value_binding::adopted_chain_in_view(query,chain)?
        || chain_specialization::adopted_result_chain(query,chain)?
        || definition_is_assessed_cross_chain_in_view(query,chain)?)
}

#[derive(Clone,PartialEq,Eq)]
struct Plan {owner:String, targets:Vec<String>, types:Vec<String>, featuring:Vec<String>, general:String, complete:bool}
fn inputs(query:&DefinitionNameQuery<'_, '_>, chain:&KirElement)->Result<Plan,QueryFailure> {
    if query.index.len()!=query.graph.len() || !query.index.get(chain.id.as_str()).is_some_and(|node|std::ptr::eq(*node,chain)) {
        return Err(query_failure("chain lifecycle requires canonical unique inputs"));
    }
    if definition_attribute_value(query.graph,chain,"is_conjugated")?!=json!(false) {
        return Err(query_failure("chain lifecycle requires a nonconjugated wrapper"));
    }
    let targets=parts(query,chain)?.ok_or_else(||query_failure("chain lifecycle has unassessed wrapper contributions"))?;
    if !adopted(query,chain)? {return Err(query_failure("chain lifecycle lacks a canonical producer context"));}
    // The complete stored wrapper above excludes valuations, metadata,
    // multiplicity children, nested/cross Features, conjugation and write
    // memberships. Imported dispatch and canonical owner queries discharge the
    // remaining additional-member, owning-Type and parameter branches.
    definition_projection::require_empty_metadata_bases_query(query,chain)?;
    if !definition_no_additional_members(chain)? || definition_feature_owner_indexed(query.graph.len(),query.index,chain)?.is_some() {
        return Err(query_failure("chain lifecycle requires no additional members or owning Type"));
    }
    if !definition_redefined_features_typed_query(query,chain,false)?.is_empty() {
        return Err(query_failure("chain lifecycle has unassessed computed redefinitions"));
    }
    let name=definition_feature_default_name_in_view(query,chain)?;
    let resolved_name=feature_defaults_generated::feature_default_name(false,false,false,false,false,false);
    if name!=resolved_name {return Err(query_failure("chain lifecycle selector disagrees with fixed untyped wrapper"));}
    let general=query_local_binding(standard_default_binding_query(query,name)?)?;
    if definition_implicit_feature_generals_in_view(query,chain,&general)?!=vec![("Subsetting",general.clone())] {
        return Err(query_failure("chain lifecycle has additional semantic producers"));
    }
    let complete=scope_boolean(chain,"is_implied_included")?;
    if complete && !stored_children_projection_query(query,chain,"owned_relationship")?.iter().any(|relation|
        relation.kind.rsplit("::").next()==Some("Subsetting")) {
        return Err(query_failure("completed chain omits its required normative Feature specialization"));
    }
    Ok(Plan {owner:chain.id.clone(),targets,general,complete,
        types:definition_feature_types_in_view(query,chain)?.iter().map(|node|node.id.clone()).collect(),
        featuring:definition_featuring_types_in_view(query,chain)?.iter().map(|node|node.id.clone()).collect()})
}

/// All receivers and dependencies preflight before one private transaction.
/// Completion means this exact wrapper's transformation stages were discharged;
/// validation, enclosing lifecycle and terminal qualification stay separate.
pub(super) fn materialize_batch(graph:&mut Vec<KirElement>, owners:&[String])->Result<usize,QueryFailure> {
    if owners.is_empty() || owners.windows(2).any(|pair|pair[0]>=pair[1]) {
        return Err(query_failure("chain lifecycle requires sorted distinct nonempty owners"));
    }
    let boundary=ecore_model::ReferenceCompleteness::Partial;
    if let Some(issue)=ecore_model::validate_publication(graph,boundary).first() {
        return Err(query_failure(format!("invalid chain lifecycle input: {}",issue.message)));
    }
    definition_check_specialization_storage(graph)?;
    let plans={
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
        let query=DefinitionNameQuery::new(graph,&index);let mut reads=TransformationReads::default();let mut plans=Vec::new();
        for id in owners {
            let chain=index.get(id.as_str()).copied().ok_or_else(||query_failure("chain lifecycle receiver is absent"))?;
            if let Some(plan)=reads.capture(inputs(&query,chain)) {plans.push(plan);}
        }
        reads.finish(||Ok(plans))?.into_query_result()?
    };
    let pending=plans.iter().filter(|plan|!plan.complete).collect::<Vec<_>>();
    if pending.is_empty() {return Ok(0);}
    let mut staged=graph.clone();
    let prefixes=pending.iter().map(|plan|format!("{}.implicit.chain-default",plan.owner)).collect::<Vec<_>>();
    let contributions=pending.iter().map(|plan|vec![("Subsetting",plan.general.as_str())]).collect::<Vec<_>>();
    let batches=pending.iter().zip(&prefixes).zip(&contributions)
        .map(|((plan,prefix),items)|(plan.owner.as_str(),prefix.as_str(),items.as_slice())).collect::<Vec<_>>();
    definition_materialize_general_batch_query(&mut staged,&batches,boundary)?;
    for plan in &pending {
        staged.iter_mut().find(|node|node.id==plan.owner).unwrap().properties.insert("is_implied_included".into(),json!(true));
    }
    if let Some(issue)=ecore_model::validate_publication(&staged,boundary).first() {
        return Err(query_failure(format!("invalid chain lifecycle output: {}",issue.message)));
    }
    let index=staged.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
    let query=DefinitionNameQuery::new(&staged,&index);
    for plan in &plans {
        let after=inputs(&query,index[plan.owner.as_str()])?;
        if !after.complete || after.targets!=plan.targets || after.general!=plan.general
            || after.types!=plan.types || after.featuring!=plan.featuring {
            return Err(query_failure("chain lifecycle changes a canonical endpoint/type/featuring projection"));
        }
    }
    *graph=staged;Ok(pending.len())
}
