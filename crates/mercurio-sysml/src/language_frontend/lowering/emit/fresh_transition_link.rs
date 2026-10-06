//! Handwritten fresh fixed transition-link execution of the guarded lifecycle.
//! Complete owner/library inputs are supplied; variable, nonfresh and arbitrary
//! ReferenceUsage transformation are not admitted by this producer.
use super::*;
#[path="fresh_transition_link_generated.rs"] mod policy;
pub(crate) fn definition_complete_fresh_transition_link(graph:&mut Vec<KirElement>,id:&str,prefix:&str)
    -> Result<(),Diagnostic> {
    let _resolved_program=policy::PROGRAM_SHA;
    if prefix.is_empty() {return Err(error("fresh transition link requires a nonempty identity prefix"));}
    if let Some(issue)=ecore_model::validate_publication(graph,ecore_model::ReferenceCompleteness::Closed).first() {return Err(error(format!("invalid fresh transition-link input: {}",issue.message)));}
    reject_result_snapshots(graph)?;
    let feature=graph.iter().find(|e|e.id==id).ok_or_else(||error("fresh transition link is absent"))?;
    if feature.kind.rsplit("::").next()!=Some(policy::KIND) || scope_boolean(feature,"is_implied_included")?
        || !definition_reference_transition_default_context(graph,feature)? || !stored_children(graph,feature,"owned_relationship")?.is_empty()
        || feature.properties.get("declared_name").is_some_and(|v|!v.is_null()) || feature.properties.get("declared_short_name").is_some_and(|v|!v.is_null()) {
        return Err(error("transition-link completion requires a fresh unnamed ordinary ReferenceUsage link"));
    }
    definition_projection::require_empty_metadata_bases(graph,feature)?;
    let (_,owner)=definition_feature_owner(graph,feature)?.ok_or_else(||error("fresh link has no canonical owning Type"))?;
    if !scope_boolean(owner,"is_implied_included")? && !transition_owner::stable_inputs(graph,owner)? {return Err(error("fresh transition-link lifecycle requires complete owner inputs"));}
    if !definition_no_additional_members(feature)? {return Err(error("fresh link requires an unsupported added-member producer"));}
    if definition_may_time_vary(graph,feature)? {return Err(error("fresh variable transition-link lifecycle remains unsupported"));}
    let mut staged=graph.clone();
    definition_materialize_feature_defaults(&mut staged,&[(id,&format!("{prefix}.generals"))])?;
    definition_materialize_owning_type_featuring(&mut staged,id,&format!("{prefix}.featuring"))?;
    // Only this fresh receiver's admitted producers have run. No supplied
    // owner, endpoint, library or other Usage is marked complete.
    staged.iter_mut().find(|e|e.id==id).unwrap().properties.insert("is_implied_included".into(),json!(true));
    let target=staged.iter().find(|e|e.id==id).unwrap();
    definition_general_type_inputs_query(&staged,target).map_err(QueryFailure::into_diagnostic)?;
    definition_featuring_types(&staged,target)?;definition_feature_types(&staged,target)?;generated_effective_names(&staged,target)?;
    if let Some(issue)=ecore_model::validate_publication(&staged,ecore_model::ReferenceCompleteness::Closed).first() {return Err(error(format!("invalid completed transition link: {}",issue.message)));}
    *graph=staged;Ok(())
}
