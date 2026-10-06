//! Handwritten recursive stored-stage execution on the assessed Transition slice.
//! Generated resolved dispatch/default maps drive selection. State/Action,
//! endpoint expressions, decision/merge and actual resource lifecycle stay open.
use super::*;
#[path="transition_child_stages_generated.rs"] mod program;
#[path="succession_defaults_generated.rs"] mod defaults;
fn append_context(graph:&mut Vec<KirElement>,id:&str,context:&str,prefix:&str)->Result<(),Diagnostic> {
    let child=graph.iter().find(|e|e.id==id).ok_or_else(||error("child context receiver absent"))?;
    if definition_owned_type_featuring(graph,child)?.iter().any(|r|r.properties.get("featuring_type").and_then(Value::as_str)==Some(context)){return Ok(());}
    let mut relation=fresh_definition_element(prefix.into(),"TypeFeaturing")?;
    relation.properties.insert("is_implied".into(),json!(true));
    for (field,target) in [("feature_of_type",id),("featuring_type",context)] {
        let endpoint=graph.iter().find(|e|e.id==target).ok_or_else(||error("child context endpoint absent"))?;
        ecore_model::validate_reference_endpoint(&relation.kind,field,&endpoint.kind).map_err(error)?;
        relation.properties.insert(field.into(),json!(target));
    }
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id==id).unwrap(),&mut relation)?;
    graph.push(relation);Ok(())
}
fn child(graph:&mut Vec<KirElement>,id:&str,owner_id:&str,prefix:&str)->Result<(),Diagnostic> {
    let _resolved=program::PROGRAM_SHA;
    let feature=graph.iter().find(|e|e.id==id).ok_or_else(||error("recursive child is absent"))?;
    if !stored_children(graph,feature,"owned_relationship")?.is_empty() || scope_boolean(feature,"is_end")? || scope_boolean(feature,"is_composite")? {
        return Err(error("recursive child stage requires a cold non-end noncomposite leaf"));
    }
    definition_projection::require_empty_metadata_bases(graph,feature)?;
    let kind=feature.kind.rsplit("::").next().unwrap_or(&feature.kind);
    let mut pending=Vec::<(String,String)>::new();
    let mut contexts=Vec::<String>::new();
    if kind=="ReferenceUsage" {
        let owner=graph.iter().find(|e|e.id==owner_id).unwrap();
        if !scope_boolean(feature,"is_portion")? || feature.properties.get("direction").is_some_and(|v|!v.is_null())
            || !definition_transition_link_feature(graph,owner)?.is_some_and(|f|f.id==id) {return Err(error("recursive ReferenceUsage requires the selected fixed portion link"));}
        let role=definition_reference_link_redefinitions(graph,feature)?;
        for target in &role {if !definition_feature_types(graph,target)?.is_empty(){return Err(error("typed existing-link role requires inherited default assessment"));}}
        pending.extend(role.iter().map(|target|("Redefinition".into(),target.id.clone())));
        // Execute the inherited default producer too; reduction discharges
        // Base::things through the computed role, rather than omitting it.
        let name=feature_defaults_generated::usage_default_name(kind,false,false,false).ok_or_else(||error("missing imported ReferenceUsage default"))?;
        let GeneratedLinkTarget::Local(default)=standard_default_binding(graph,name)? else{return Err(error("recursive link requires a local default"));};
        pending.push(("Subsetting".into(),default));contexts.push(owner_id.into());
    } else if kind=="Feature" {
        let owner=definition_fixed_parameter_owner(graph,feature)?.ok_or_else(||error("recursive Feature requires an assessed fixed parameter"))?;
        if owner.id!=owner_id{return Err(error("parameter owner mismatch"));}
        pending.extend(definition_positional_parameters(graph,feature,owner)?.iter().map(|target|("Redefinition".into(),target.id.clone())));
        let name=definition_feature_default_name(graph,feature)?;
        let GeneratedLinkTarget::Local(default)=standard_default_binding(graph,name)? else{return Err(error("recursive parameter requires a local default"));};
        pending.push(("Subsetting".into(),default));contexts.push(owner_id.into());
    } else if kind==program::SUCCESSION {
        if !scope_boolean(feature,"is_portion")? || feature.properties.get("direction").is_some_and(|v|!v.is_null()) {return Err(error("recursive succession requires fixed portion flags"));}
        let owner=graph.iter().find(|e|e.id==owner_id).unwrap();
        if owner.kind.rsplit("::").next()!=Some(program::OWNER) || !definition_reference_targets(graph,owner,"succession")?.iter().any(|e|e.id==id){return Err(error("succession context requires the selected Transition member"));}
        if graph.iter().any(|e|metaclass_conforms(&e.kind,"DecisionNode") || metaclass_conforms(&e.kind,"MergeNode")){return Err(error("decision/merge succession specialization remains unsupported"));}
        let name=defaults::default_for(kind,0,false).ok_or_else(||error("unbound succession default selector"))?;
        let GeneratedLinkTarget::Local(default)=standard_default_binding(graph,name)? else{return Err(error("recursive succession requires a local default"));};
        let role=graph.iter().find(|e|e.id==default).ok_or_else(||error("succession role absent"))?;
        if !scope_boolean(role,"is_implied_included")? || !definition_feature_types(graph,role)?.is_empty(){return Err(error("typed/incomplete succession role remains unsupported by the leaf stage"));}
        pending.push(("Subsetting".into(),default));
        contexts.extend(definition_featuring_types(graph,owner)?.iter().map(|e|e.id.clone()));
    } else {return Err(error(format!("unsupported recursive Transition child {kind}")));}
    let pending=pending.iter().map(|(k,id)|(k.as_str(),id.as_str())).collect::<Vec<_>>();
    definition_materialize_selected_generals(graph,id,&format!("{prefix}.generals"),&pending)?;
    for (i,context) in contexts.iter().enumerate(){append_context(graph,id,context,&format!("{prefix}.featuring.{i}"))?;}
    Ok(())
}
/// Extend the receiver transaction with assessed stored stages of its original
/// owned children. This is not complete model validation or actual-library support.
pub(crate) fn definition_complete_fixed_transition_stored_tree(graph:&mut Vec<KirElement>,owner_id:&str,prefix:&str)->Result<(),Diagnostic> {
    let owner=graph.iter().find(|e|e.id==owner_id).ok_or_else(||error("stored tree owner absent"))?;
    let children=stored_children(graph,owner,"owned_relationship")?.iter().filter(|m|metaclass_conforms(&m.kind,"OwningMembership"))
        .map(|m|stored_membership_endpoint(graph,m,"member_element").and_then(|endpoint|match endpoint {StoredMembershipEndpoint::Resolved(f)=>Ok(f.id.clone()),_=>Err(error("recursive child endpoint unavailable"))})).collect::<Result<Vec<_>,Diagnostic>>()?;
    let mut staged=graph.clone();definition_complete_fixed_transition_owner(&mut staged,owner_id,prefix)?;
    // Succession context is an eager connector dependency. Ordinary parameter
    // stored stages follow binding context selection; moving them earlier changes
    // the construction-time context of source bindings.
    let mut deferred=Vec::new();
    for id in children {
        if staged.iter().find(|e|e.id==id).unwrap().kind.rsplit("::").next()==Some(program::SUCCESSION) {
            child(&mut staged,&id,owner_id,&format!("{prefix}.child.{id}"))?;
        } else {deferred.push(id);}
    }
    // Recursive transformation revisits newly constructed connectors after the
    // related child stages. Reuse the independently assessed binary context
    // algorithm; never substitute the Transition owner for endpoint inference.
    let bindings=staged.iter().filter(|e|metaclass_conforms(&e.kind,"BindingConnector") && !graph.iter().any(|old|old.id==e.id)).map(|e|e.id.clone()).collect::<Vec<_>>();
    for binding in bindings {
        let b=staged.iter().find(|e|e.id==binding).unwrap();
        if definition_feature_owner(&staged,b)?.is_some() || !definition_owned_type_featuring(&staged,b)?.is_empty(){continue;}
        let source=definition_reference_targets(&staged,b,"source")?;
        let target=definition_reference_targets(&staged,b,"target")?;
        if source.len()!=1 || target.len()!=1{return Err(error("recursive binding requires binary resolved related features"));}
        if let Some(context)=definition_binding_context(&staged,source[0],target[0])?.map(|e|e.id.clone()) {
            append_context(&mut staged,&binding,&context,&format!("{binding}.recursive-context"))?;
        }
    }
    for id in deferred {child(&mut staged,&id,owner_id,&format!("{prefix}.child.{id}"))?;}
    if let Some(issue)=ecore_model::validate_publication(&staged,ecore_model::ReferenceCompleteness::Closed).first(){return Err(error(format!("invalid recursive stored output: {}",issue.message)));}
    *graph=staged;Ok(())
}
