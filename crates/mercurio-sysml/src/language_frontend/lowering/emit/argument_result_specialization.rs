//! Handwritten execution of reviewed Ecore Index/Select result-specialization
//! rules. Imported delegates choose arguments and results; library resolution and
//! ancestry remain explicit native dependencies. This stage adds no lifecycle
//! flags and does not implement operator execution or validation.
use super::*;

#[derive(Clone)]
struct Inputs { result: String, argument_result: String }

pub(super) fn supports(kind: &str) -> bool {
    argument_result_specialization_generated::rule(kind.rsplit("::").next().unwrap_or(kind)).is_some()
}

fn plan(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement)
    -> Result<TransformationPlan<Option<Inputs>>, QueryFailure> {
    let key=(owner.id.clone(),"native:argument-result-specialization".to_owned());
    if !query.active_reference_projections.borrow_mut().insert(key.clone()) {
        return Err(query_failure("cyclic argument-result specialization dependency"));
    }
    let answer=inputs(query,owner);
    query.active_reference_projections.borrow_mut().remove(&key);
    answer
}

fn inputs(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement)
    -> Result<TransformationPlan<Option<Inputs>>, QueryFailure> {
    let _bindings=argument_result_specialization_generated::BINDINGS_SHA256;
    let rule=argument_result_specialization_generated::rule(owner.kind.rsplit("::").next().unwrap_or(&owner.kind))
        .ok_or_else(||query_failure("unbound argument-result specialization class"))?;
    let _normative_rule=rule.normative_rule;
    if query.index.len()!=query.graph.len()
        || !query.index.get(owner.id.as_str()).is_some_and(|node|std::ptr::eq(*node,owner))
        || scope_boolean(owner,"is_implied_included")? {
        return Err(query_failure("argument-result specialization requires an incomplete canonical expression"));
    }
    if !definition_additional_members_ready_in_view(query,owner)? {
        return Ok(TransformationPlan::Required(vec![Prerequisite::AdditionalMembers{owner_id:owner.id.clone()}]));
    }
    let arguments=relative_namespace::arguments(query,owner)?;
    let Some(argument)=arguments.first().copied() else {return Ok(TransformationPlan::Ready(None));};
    if argument.id==owner.id {return Err(query_failure("self-dependent argument-result specialization"));}
    if !definition_additional_members_ready_in_view(query,argument)? {
        return Ok(TransformationPlan::Required(vec![Prerequisite::AdditionalMembers{owner_id:argument.id.clone()}]));
    }
    let mut reads=TransformationReads::default();
    let result=reads.capture(definition_result_parameter_typed_query(query,owner));
    let argument_result=reads.capture(definition_result_parameter_typed_query(query,argument));
    reads.finish(|| {
        let result=result.flatten().ok_or_else(||query_failure("argument specialization has no constructed result"))?;
        let argument_result=argument_result.flatten().ok_or_else(||query_failure("first argument has no result"))?;
        let Some((membership,parent))=definition_feature_owner_indexed(query.graph.len(),query.index,result)? else {
            return Err(query_failure("argument specialization result lacks canonical ownership"));
        };
        if parent.id!=owner.id || !metaclass_conforms(&membership.kind,"ReturnParameterMembership")
            || result.kind.rsplit("::").next()!=Some("Feature") || result.properties.get("direction")!=Some(&json!("out"))
            || ["is_end","is_variable","is_composite","is_portion"].iter()
                .map(|field|scope_boolean(result,field)).collect::<Result<Vec<_>,_>>()?.into_iter().any(|flag|flag) {
            return Err(query_failure("argument specialization requires the actual fixed owned out result"));
        }
        if result.id==argument_result.id {return Err(query_failure("argument specialization would create a self relation"));}
        if let Some(name)=rule.unless_library {
            let collection=query_local_binding(standard_default_binding_query(query,name)?)?;
            let collection=query.index.get(collection.as_str()).copied().ok_or_else(||query_failure("collection guard is absent"))?;
            if !metaclass_conforms(&collection.kind,"DataType") {return Err(query_failure("collection guard requires the actual library DataType"));}
            if definition_specializes_depth_in_view(query,argument_result,collection,0)? {return Ok(None);}
        }
        Ok(Some(Inputs{result:result.id.clone(),argument_result:argument_result.id.clone()}))
    })
}

fn has_relation(query: &DefinitionNameQuery<'_, '_>, input: &Inputs) -> Result<bool,QueryFailure> {
    let result=query.index[input.result.as_str()];
    for relation in stored_children_projection_query(query,result,"owned_relationship")? {
        if relation.kind.rsplit("::").next()!=Some("Subsetting") {continue;}
        if query_reference_in_view(query,relation,"specific")?.id!=result.id {
            return Err(query_failure("argument-result relation disagrees with canonical ownership"));
        }
        if query_reference_in_view(query,relation,"general")?.id==input.argument_result {return Ok(true);}
    }
    Ok(false)
}

pub(super) fn ready(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement) -> Result<bool,QueryFailure> {
    match plan(query,owner)?.into_query_result()? {Some(input)=>has_relation(query,&input),None=>Ok(true)}
}

/// Preflight the complete group before one private transaction. The registered
/// scheduler owns any missing reads/constructions; unavailable inputs never
/// become empty answers or invented argument positions.
pub(super) fn materialize_batch(graph: &mut Vec<KirElement>, owners: &[String]) -> Result<usize,QueryFailure> {
    if owners.is_empty() || owners.windows(2).any(|pair|pair[0]>=pair[1]) {
        return Err(query_failure("argument-result batch requires sorted distinct owners"));
    }
    let boundary=ecore_model::ReferenceCompleteness::Partial;
    if let Some(issue)=ecore_model::validate_publication(graph,boundary).first() {
        return Err(query_failure(format!("invalid argument-result input: {}",issue.message)));
    }
    definition_check_specialization_storage(graph)?;
    let missing={
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
        if index.len()!=graph.len() {return Err(query_failure("duplicate argument-result identity"));}
        let query=DefinitionNameQuery::new(graph,&index);
        let mut reads=TransformationReads::default();let mut missing=Vec::new();
        for id in owners {
            let owner=index.get(id.as_str()).copied().ok_or_else(||query_failure("argument-result owner is absent"))?;
            if let Some(Some(input))=reads.capture_plan(plan(&query,owner)) {
                // Separate preflight checks all existing result contributions;
                // the admission callback below does not read result.type itself.
                reads.capture(definition_validate_result_feature_inputs_query(&query,index[input.result.as_str()]));
                if let Some(false)=reads.capture(has_relation(&query,&input)) {missing.push((id.clone(),input));}
            }
        }
        reads.finish(||Ok(missing))?.into_query_result()?
    };
    if missing.is_empty() {return Ok(0);}
    let mut staged=graph.clone();
    for (owner,input) in &missing {
        definition_append_implied_specialization_in_stage(&mut staged,
            &format!("{owner}.implicit.argument-result-specialization"),"Subsetting",&input.result,&input.argument_result)?;
    }
    if let Some(issue)=ecore_model::validate_publication(&staged,boundary).first() {
        return Err(query_failure(format!("invalid argument-result output: {}",issue.message)));
    }
    let index=staged.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
    let query=DefinitionNameQuery::new(&staged,&index);
    for id in owners {if !ready(&query,index[id.as_str()])? {return Err(query_failure("argument-result producer omitted its postcondition"));}}
    *graph=staged;Ok(missing.len())
}

/// Recompute the actual return owner and argument selection. Neither identity
/// spelling nor an implied/completed flag authorizes an endpoint. Other result
/// contributions retain their own admission algorithms.
pub(super) fn assessed_relation(query: &DefinitionNameQuery<'_, '_>, result: &KirElement, relation: &KirElement)
    -> Result<Option<bool>,QueryFailure> {
    if relation.kind.rsplit("::").next()!=Some("Subsetting") {return Ok(None);}
    let Some((membership,owner))=definition_feature_owner_indexed(query.graph.len(),query.index,result)? else {return Ok(None);};
    if !metaclass_conforms(&membership.kind,"ReturnParameterMembership") || !supports(&owner.kind) {return Ok(None);}
    let Some(input)=plan(query,owner)?.into_query_result()? else {return Ok(None);};
    if query_reference_in_view(query,relation,"general")?.id!=input.argument_result {return Ok(None);}
    Ok(Some(input.result==result.id && query_reference_in_view(query,relation,"specific")?.id==result.id))
}
