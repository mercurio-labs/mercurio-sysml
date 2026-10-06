//! Handwritten independent featuring stage of the resolved Transition/Succession
//! program. Pure parent featuring proofs precede member construction. This does
//! not complete the parent, its generalizations, or the Connector end lifecycle.
use super::*;
#[path="transition_context_stage_generated.rs"] mod policy;

pub(super) fn parent<'g>(graph:&'g [KirElement],child:&KirElement)->Result<Option<&'g KirElement>,QueryFailure> {
    let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
    parent_in_view(&DefinitionNameQuery::new(graph,&index),child)
}

pub(super) fn parent_in_view<'g>(query: &DefinitionNameQuery<'g, '_>, child: &KirElement) -> Result<Option<&'g KirElement>, QueryFailure> {
    let _program=policy::PROGRAM_SHA;
    if child.kind.rsplit("::").next()!=Some("SuccessionAsUsage") {return Ok(None);}
    let Some(parent)=generated_parent_namespace_query(query,&child.id)? else {return Ok(None);};
    if parent.kind.rsplit("::").next()!=Some("TransitionUsage") {return Ok(None);}
    let succession=definition_reference_targets_typed_query(query,parent,"succession")?;
    if succession.first().is_some_and(|e|e.id==child.id) {Ok(Some(parent))} else {Ok(None)}

}

// Evaluate only the earlier featuring stage through the shared imported delegate.
// Typed dependency failure cannot turn into an empty context or a completion flag.
pub(super) fn append(graph:&mut Vec<KirElement>,child_id:&str,prefix:&str)->Result<usize,QueryFailure> {
    if prefix.is_empty(){return Err(query_failure("Transition context stage requires nonempty identity prefix"));}
    let child=graph.iter().find(|e|e.id==child_id).ok_or_else(||query_failure("context-stage child is absent"))?;
    let Some(parent)=parent(graph,child)? else {return Ok(0);};
    let targets=definition_featuring_types_typed_query(graph,parent)?;
    let existing=definition_owned_type_featuring_typed_query(graph,child)?.iter().map(|r|query_reference(graph,r,"featuring_type").map(|e|e.id.clone())).collect::<Result<BTreeSet<_>,QueryFailure>>()?;
    let targets=targets.into_iter().filter(|e|!existing.contains(&e.id)).map(|e|e.id.clone()).collect::<Vec<_>>();
    let mut staged=graph.clone();
    for (i,target_id) in targets.iter().enumerate() {
        let id=format!("{prefix}.{i}");
        if staged.iter().any(|e|e.id==id){return Err(query_failure("context-stage identity collision"));}
        let target_index=staged.iter().position(|e|e.id==*target_id).ok_or_else(||query_failure("context-stage target disappeared"))?;
        let mut relation=fresh_definition_element(id,"TypeFeaturing")?;
        ecore_model::validate_reference_endpoint(&relation.kind,"featuring_type",&staged[target_index].kind).map_err(query_failure)?;
        relation.properties.insert("feature_of_type".into(),json!(child_id));relation.properties.insert("featuring_type".into(),json!(target_id));relation.properties.insert("is_implied".into(),json!(true));
        if !staged[target_index].properties.get("owning_relationship").is_some_and(|v|!v.is_null()) {
            let mut target=staged[target_index].clone();
            ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut relation,&mut target)?;
            staged[target_index]=target;
        }
        let child=staged.iter_mut().find(|e|e.id==child_id).unwrap();
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,child,&mut relation)?;
        staged.push(relation);
    }
    if let Some(issue)=ecore_model::validate_publication(&staged,ecore_model::ReferenceCompleteness::Partial).first(){return Err(query_failure(format!("invalid private context-stage output: {}",issue.message)));}
    *graph=staged;Ok(targets.len())
}
