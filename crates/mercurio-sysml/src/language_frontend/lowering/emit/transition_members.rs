//! Native execution of the guarded resolved Transition additional-member program.
//! Source/link structure and the reviewed fresh binding transaction are separate from
//! full Usage lifecycle, endpoint transformation and semantic validation.
use super::*;
#[path="transition_members_generated.rs"] mod policy;
pub(crate) fn definition_materialize_transition_members(graph:&mut Vec<KirElement>,owner_id:&str,prefix:&str)
    -> Result<usize,Diagnostic> {
    materialize(graph,owner_id,prefix,false)
}
pub(super) fn materialize_with_assessed_owner(graph:&mut Vec<KirElement>,owner_id:&str,prefix:&str)->Result<usize,Diagnostic> {
    materialize(graph,owner_id,prefix,true)
}
fn materialize(graph:&mut Vec<KirElement>,owner_id:&str,prefix:&str,assessed:bool)->Result<usize,Diagnostic> {
    if prefix.is_empty() {return Err(error("Transition members require a nonempty identity prefix"));}
    let owner=graph.iter().find(|e|e.id==owner_id).ok_or_else(||error("Transition member owner is absent"))?;
    if owner.kind.rsplit("::").next()!=Some(policy::OWNER) {return Err(error("unassessed Transition member dispatch"));}
    if !definition_chain_reference_targets(graph,owner,"chaining_feature")?.is_empty() {
        return Err(error("Transition member parameters require chained basic-feature lifecycle assessment"));
    }
    if !scope_boolean(owner,"is_implied_included")? && !(assessed && transition_owner::stable_inputs(graph,owner)?) {return Err(error("Transition member producer requires explicitly complete owner inputs; native Transition lifecycle remains required"));}
    let mut staged=graph.clone();let mut count=usize::from(transition_source::definition_materialize_transition_source(&mut staged,owner_id,&format!("{prefix}.source"))?);
    let owner=staged.iter().find(|e|e.id==owner_id).unwrap();
    if definition_transition_link_feature(&staged,owner)?.is_none() {
        let succession=definition_reference_targets(&staged,owner,"succession")?.first().map(|e|e.id.clone());
        let parameters=definition_owned_features(&staged,owner,false)?.iter().filter_map(|f|match definition_has_direction(f) {Ok(true)=>Some(Ok(f.id.clone())),Ok(false)=>None,Err(e)=>Some(Err(e))}).collect::<Result<Vec<_>,Diagnostic>>()?;
        let source=transition_source_query::source_feature_after_source_stage(&staged,owner).map_err(QueryFailure::into_diagnostic)?
            .map(|f|definition_chain_reference_targets(&staged,f,"feature_target").map(|v|v.first().map(|f|f.id.clone())))
            .transpose()?.flatten();
        if let Some(succession)=succession {
            let mut link=fresh_definition_element(format!("{prefix}.link"),policy::LINK)?;
            let mut membership=fresh_definition_element(format!("{prefix}.link.membership"),policy::MEMBERSHIP)?;
            ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut membership,&mut link)?;
            ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,staged.iter_mut().find(|e|e.id==owner_id).unwrap(),&mut membership)?;
            let link_id=link.id.clone();staged.extend([link,membership]);
            definition_finish_fresh_binding(&mut staged,owner_id,&format!("{prefix}.succession-binding"),&succession,&link_id)?;
            count+=2;
        }
        if let (Some(source),Some(parameter))=(source,parameters.first()) {
            definition_finish_fresh_binding(&mut staged,owner_id,&format!("{prefix}.source-binding"),&source,parameter)?;
            count+=1;
        }
    }
    if let Some(issue)=ecore_model::validate_publication(&staged,ecore_model::ReferenceCompleteness::Closed).first() {
        return Err(error(format!("invalid Transition additional-member output: {}",issue.message)));
    }
    *graph=staged;Ok(count)
}

/// Compose construction with the admitted fresh fixed link lifecycle. Any
/// failure discards source/link, binding and completion changes together.
pub(crate) fn definition_materialize_transition_members_with_fixed_links(graph:&mut Vec<KirElement>,owner:&str,prefix:&str)
    -> Result<usize,Diagnostic> {
    let ids=graph.iter().map(|e|e.id.clone()).collect::<BTreeSet<_>>();
    let mut staged=graph.clone();let count=definition_materialize_transition_members(&mut staged,owner,prefix)?;
    let new_links=staged.iter().filter(|e|!ids.contains(&e.id) && e.kind.rsplit("::").next()==Some("ReferenceUsage")).map(|e|e.id.clone()).collect::<Vec<_>>();
    for id in new_links {definition_complete_fresh_transition_link(&mut staged,&id,&format!("{id}.lifecycle"))?;}
    *graph=staged;Ok(count)
}

// Physical additional-member production within the linker's private graph.
// Source construction and chained/basic-feature strategies remain explicit
// dependencies. No receiver or fresh member is marked transformation-complete.
pub(super) fn materialize_for_linker_query(graph:&mut Vec<KirElement>,owner_id:&str,prefix:&str)
    -> Result<usize,QueryFailure> {
    if prefix.is_empty(){return Err(query_failure("Transition members require a nonempty identity prefix"));}
    let owner=graph.iter().find(|e|e.id==owner_id).ok_or_else(||query_failure("Transition member owner is absent"))?;
    if owner.kind.rsplit("::").next()!=Some(policy::OWNER){return Err(query_failure("unassessed Transition member dispatch"));}
    if !transition_source::membership_stable_query(graph,owner)? {
        return Err(query_failure("Transition linker requires source-member construction before connector production"));
    }
    if !definition_chain_reference_targets(graph,owner,"chaining_feature")?.is_empty(){return Err(query_failure("Transition member parameters require chained basic-feature lifecycle assessment"));}
    if definition_transition_link_feature(graph,owner)?.is_some(){return Ok(0);}
    let succession=definition_reference_targets(graph,owner,"succession")?.first().map(|e|e.id.clone());
    let parameters=definition_owned_features(graph,owner,false)?.iter().filter_map(|f|match definition_has_direction(f){Ok(true)=>Some(Ok(f.id.clone())),Ok(false)=>None,Err(e)=>Some(Err(e))}).collect::<Result<Vec<_>,Diagnostic>>()?;
    let source=transition_source_query::source_feature_after_source_stage(graph,owner)?
        .map(|f|definition_chain_reference_targets(graph,f,"feature_target").map(|v|v.first().map(|f|f.id.clone()))).transpose()?.flatten();
    let mut staged=graph.clone();let mut count=0;
    if let Some(ref succession)=succession {transition_context_stage::append(&mut staged,succession,&format!("{prefix}.succession-context"))?;}
    if let Some(succession)=succession {
        let mut link=fresh_definition_element(format!("{prefix}.link"),policy::LINK)?;
        let mut membership=fresh_definition_element(format!("{prefix}.link.membership"),policy::MEMBERSHIP)?;
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut membership,&mut link)?;
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,staged.iter_mut().find(|e|e.id==owner_id).unwrap(),&mut membership)?;
        let link_id=link.id.clone();staged.extend([link,membership]);
        definition_materialize_binding_structure_with_boundary(&mut staged,owner_id,&format!("{prefix}.succession-binding"),&succession,&link_id,ecore_model::ReferenceCompleteness::Partial)?;
        count+=2;
    }
    if let (Some(source),Some(parameter))=(source,parameters.first()) {
        definition_materialize_binding_structure_with_boundary(&mut staged,owner_id,&format!("{prefix}.source-binding"),&source,parameter,ecore_model::ReferenceCompleteness::Partial)?;
        count+=1;
    }
    if let Some(issue)=ecore_model::validate_publication(&staged,ecore_model::ReferenceCompleteness::Partial).first(){return Err(query_failure(format!("invalid private Transition member output: {}",issue.message)));}
    *graph=staged;Ok(count)
}
