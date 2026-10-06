//! Handwritten producer-disjointness admission for guarded Transition lifecycle.
//! Assessed Class/Action/State ownership and fixed portion flags rule out variation, owned
//! crossing, valuation and variable featuring. This proves stable general inputs,
//! not added-member completion; the transaction must still execute that stage.
use super::*;
pub(super) fn stable_inputs(graph:&[KirElement],owner:&KirElement)->Result<bool,Diagnostic> {
    if owner.kind.rsplit("::").next()!=Some("TransitionUsage") || !scope_boolean(owner,"is_portion")?
        || scope_boolean(owner,"is_end")? || scope_boolean(owner,"is_composite")?
        || owner.properties.get("direction").is_some_and(|v|!v.is_null())
        || !ecore_defaults::read_attribute(owner,"portion_kind").map_err(error)?.is_null() {return Ok(false);}
    definition_projection::require_empty_metadata_bases(graph,owner)?;
    let Some((membership,parent))=definition_feature_owner(graph,owner)? else {return Ok(false);};
    if membership.kind.rsplit("::").next()!=Some("FeatureMembership") || !matches!(parent.kind.rsplit("::").next(),Some("Class"|"ActionDefinition"|"StateDefinition"|"ActionUsage"|"StateUsage"))
        || !scope_boolean(parent,"is_implied_included")? {return Ok(false);}
    for rel in stored_children(graph,owner,"owned_relationship")? {
        if !metaclass_conforms(&rel.kind,"Membership") && rel.kind.rsplit("::").next()!=Some("TypeFeaturing")
            && !(rel.kind.rsplit("::").next()==Some("Subsetting") && scope_boolean(rel,"is_implied")?) {return Ok(false);}
    }
    // Consume the guarded resolved selector, preserving its normal library and
    // partial-contribution checks. Extra owner generalizations remain excluded.
    // Imported predicates select all inherited contributions for this parent;
    // admission proves their physical inputs rather than discarding extra roles.
    let names=occurrence_strategy_names(graph,owner)?;
    if names.is_empty(){return Ok(false);}
    for name in names {
        let GeneratedLinkTarget::Local(role)=standard_default_binding(graph,name)? else {return Ok(false);};
        let role=graph.iter().find(|e|e.id==role).ok_or_else(||error("Transition role is absent"))?;
        if !scope_boolean(role,"is_implied_included")? {return Ok(false);}
    }
    Ok(true)
}
/// Complete the admitted receiver through explicit producers, atomically. This
/// neither completes supplied children/resources nor integrates the scheduler.
pub(crate) fn definition_complete_fixed_transition_owner(graph:&mut Vec<KirElement>,id:&str,prefix:&str)
    -> Result<(),Diagnostic> {
    if prefix.is_empty(){return Err(error("Transition owner lifecycle requires a nonempty prefix"));}
    if let Some(issue)=ecore_model::validate_publication(graph,ecore_model::ReferenceCompleteness::Closed).first(){return Err(error(format!("invalid Transition lifecycle input: {}",issue.message)));}
    reject_result_snapshots(graph)?;
    definition_require_ordinary_lifecycle_dependencies(graph)?;
    let owner=graph.iter().find(|e|e.id==id).ok_or_else(||error("Transition lifecycle owner is absent"))?;
    if scope_boolean(owner,"is_implied_included")? || !stable_inputs(graph,owner)? {return Err(error("Transition lifecycle requires incomplete fixed portion receiver under an assessed Class/Action/State owner"));}
    for rel in stored_children(graph,owner,"owned_relationship")? {
        if metaclass_conforms(&rel.kind,"Membership") {
            if let Some(child)=transition_source::member_query(graph,rel).map_err(QueryFailure::into_diagnostic)? {
                if !metaclass_conforms(&child.kind,"Feature") || !scope_boolean(child,"is_implied_included")? {return Err(error("Transition lifecycle requires supplied complete member/source inputs"));}
            } else {return Err(error("Transition lifecycle requires resolved supplied source/member inputs"));}
        }
    }
    let mut staged=graph.clone();
    // Feature featuring is computed before Type's additional-member producers.
    definition_materialize_owning_type_featuring(&mut staged,id,&format!("{prefix}.featuring"))?;
    transition_members::materialize_with_assessed_owner(&mut staged,id,&format!("{prefix}.members"))?;
    definition_materialize_feature_defaults(&mut staged,&[(id,&format!("{prefix}.generals"))])?;
    let links=staged.iter().filter(|e|!graph.iter().any(|old|old.id==e.id) && e.kind.rsplit("::").next()==Some("ReferenceUsage")).map(|e|e.id.clone()).collect::<Vec<_>>();
    for link in links {definition_complete_fresh_transition_link(&mut staged,&link,&format!("{link}.lifecycle"))?;}
    // Specializations precede featuring in Pilot's physical pending flush.
    let owner=staged.iter().find(|e|e.id==id).unwrap();
    let rels=stored_children(&staged,owner,"owned_relationship")?;
    let ordered=rels.iter().filter(|e|!metaclass_conforms(&e.kind,"TypeFeaturing"))
        .chain(rels.iter().filter(|e|metaclass_conforms(&e.kind,"TypeFeaturing"))).map(|e|e.id.clone()).collect::<Vec<_>>();
    let owner=staged.iter_mut().find(|e|e.id==id).unwrap();owner.properties.insert("owned_relationship".into(),json!(ordered));
    owner.properties.insert("is_implied_included".into(),json!(true));
    let owner=staged.iter().find(|e|e.id==id).unwrap();
    definition_general_type_inputs_query(&staged,owner).map_err(QueryFailure::into_diagnostic)?;
    definition_featuring_types(&staged,owner)?;generated_effective_names(&staged,owner)?;
    if let Some(issue)=ecore_model::validate_publication(&staged,ecore_model::ReferenceCompleteness::Closed).first(){return Err(error(format!("invalid completed Transition: {}",issue.message)));}
    *graph=staged;Ok(())
}
