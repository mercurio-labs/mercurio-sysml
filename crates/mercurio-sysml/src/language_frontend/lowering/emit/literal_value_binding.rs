//! Handwritten FeatureValue value-binding composition over imported defaults,
//! chain/delegate and Ecore ownership contracts. Structural admissions below
//! discharge only non-typing containment for selector queries. They never prove
//! lifecycle completion, validity or complete resource qualification.
use super::*;
use super::binding_dependencies::{BindingDependencies,binding_dependencies_plan};

struct ValueInputs { expression: String, default: bool }


fn value_inputs_query_in_view(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement) -> Result<Option<ValueInputs>, QueryFailure> {
    value_inputs_with_receiver_state(query,owner,true)
}

fn value_inputs_with_receiver_state(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement,
    completed: bool) -> Result<Option<ValueInputs>, QueryFailure> {
    if owner.kind.rsplit("::").next() != Some("Feature") { return Ok(None); }
    for field in ["is_end", "is_variable", "is_composite", "is_portion"] {
        if scope_boolean(owner, field)? { return Ok(None); }
    }
    definition_projection::require_empty_metadata_bases_query(query, owner)?;
    let relations = stored_children_projection_query(query, owner, "owned_relationship")?;
    let values = relations.iter().filter(|r| r.kind.rsplit("::").next() == Some("FeatureValue")).collect::<Vec<_>>();
    if values.len() != 1 { return Ok(None); }
    let value = values[0];
    if scope_boolean(value, "is_implied")? || scope_boolean(value, "is_initial")? { return Ok(None); }
    let children = stored_children_projection_query(query, value, "owned_related_element")?;
    let selected = definition_reference_targets_typed_query(query, value, "value")?;
    if children.len() != 1 || selected.len() != 1 || children[0].id != selected[0].id { return Ok(None); }
    let expression = selected[0];
    if !cold_literal_values::supports(expression.kind.rsplit("::").next().unwrap_or(&expression.kind))
        || scope_boolean(expression, "is_implied_included")? != completed { return Ok(None); }
    // Explicit typing prevents the separate disputed bound-value Subsetting
    // branch. An absent/pending typing must not be an empty successful input.
    let typings = relations.iter().filter(|r| metaclass_conforms(&r.kind, "FeatureTyping")).collect::<Vec<_>>();
    if typings.is_empty() { return Ok(None); }
    for typing in typings {
        if scope_boolean(typing, "is_implied")? { return Ok(None); }
        query_reference_in_view(query, typing, "type")?;
    }
    if definition_value_expression_owner_in_view(query, expression)?.id != owner.id { return Ok(None); }
    Ok(Some(ValueInputs { expression: expression.id.clone(), default: scope_boolean(value, "is_default")? }))
}

/// Canonical structural proof for this one generated contribution. Completion
/// flags are deliberately not used to admit arbitrary supplied bindings.
pub(super) fn binding_contribution(graph: &[KirElement], owner: &KirElement, member: &KirElement)
    -> Result<bool, Diagnostic> {
    binding_contribution_query(graph,owner,member)
        .map_err(|failure|failure.into_diagnostic_at("literal binding_contribution external boundary"))
}

pub(super) fn binding_contribution_query(graph: &[KirElement], owner: &KirElement, member: &KirElement)
    -> Result<bool, QueryFailure> {
    let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
    binding_contribution_query_in_view(&DefinitionNameQuery::new(graph,&index),owner,member)
}

pub(super) fn binding_contribution_query_in_view(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement, member: &KirElement) -> Result<bool, QueryFailure> {
    if member.kind.rsplit("::").next() != Some("OwningMembership") { return Ok(false); }
    let children = stored_children_projection_query(query, member, "owned_related_element")?;
    if children.len() != 1 { return Ok(false); }
    let binding = children[0];
    if binding.kind.rsplit("::").next() != Some("BindingConnector") || !scope_boolean(binding, "is_implied")? { return Ok(false); }
    let Some(value) = value_inputs_query_in_view(query, owner)? else { return Ok(false); };
    if value.default { return Ok(false); }
    for field in ["is_end", "is_variable", "is_composite", "is_portion"] {
        if scope_boolean(binding, field)? { return Ok(false); }
    }
    if binding.properties.get("direction").is_some_and(|v| !v.is_null()) { return Ok(false); }
    let ends = definition_owned_features_query(query, binding, true)?;
    if ends.len() != 2 { return Ok(false); }
    let mut targets = Vec::new();
    for end in &ends {
        let references = stored_children_projection_query(query, end, "owned_relationship")?.into_iter()
            .filter(|r| r.kind.rsplit("::").next() == Some("ReferenceSubsetting")).collect::<Vec<_>>();
        if references.len() != 1 { return Ok(false); }
        targets.push(query_reference_in_view(query, references[0], "referenced_feature")?);
    }
    if targets[1].id != owner.id || targets[0].kind.rsplit("::").next() != Some("Feature") { return Ok(false); }
    let index = query.index;
    let Some(reference) = scope_container_query(query, targets[0])? else { return Ok(false); };
    if reference.kind.rsplit("::").next() != Some("ReferenceSubsetting")
        || scope_container_query(query, reference)?.is_none_or(|e| e.id != ends[0].id) { return Ok(false); }
    let expression = index.get(value.expression.as_str()).copied().ok_or_else(|| query_failure("value expression absent"))?;
    let Some(result) = definition_result_parameter_typed_query(query, expression)? else { return Ok(false); };
    canonical_chain_lifecycle::exact(query,targets[0],&[expression.id.clone(),result.id.clone()])
}

pub(super) fn adopted_chain_in_view(query: &DefinitionNameQuery<'_, '_>, feature: &KirElement) -> Result<bool, Diagnostic> {
    if feature.kind.rsplit("::").next() != Some("Feature") { return Ok(false); }
    // Avoid traversing unrelated ownership; the caller already has a checked index.
    // The full reciprocal containment proof below still governs admission.
    if !feature.properties.get("owning_relationship").and_then(Value::as_str)
        .and_then(|id| query.index.get(id).copied())
        .is_some_and(|r| r.kind.rsplit("::").next() == Some("ReferenceSubsetting")) { return Ok(false); }
    let Some(reference) = scope_container_query(query, feature)? else { return Ok(false); };
    if reference.kind.rsplit("::").next() != Some("ReferenceSubsetting") { return Ok(false); }
    let Some(end) = scope_container_query(query, reference)? else { return Ok(false); };
    let Some((_, binding)) = definition_feature_owner_indexed(query.graph.len(),query.index,end)? else { return Ok(false); };
    let Some(member) = scope_container_query(query, binding)? else { return Ok(false); };
    let Some(owner) = scope_container_query(query, member)? else { return Ok(false); };
    binding_contribution_query_in_view(query,owner,member).map_err(|failure|failure.into_diagnostic_at("literal binding_contribution external boundary"))

}

/// Prove that value/binding containment cannot supply extra typing or owner
/// predicate inputs. This permits an incomplete owner, never completes it.
pub(super) fn stable_inputs_query_in_view(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement) -> Result<bool, QueryFailure> {
    stable_inputs_with_receiver_state(query,owner,true)
}

fn stable_inputs_with_receiver_state(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement,
    completed: bool) -> Result<bool, QueryFailure> {
    let graph=query.graph;
    if value_inputs_with_receiver_state(query, owner, completed)?.is_none() { return Ok(false); }
    for relation in stored_children_projection_query(query, owner, "owned_relationship")? {
        if metaclass_conforms(&relation.kind, "FeatureTyping")
            || relation.kind.rsplit("::").next() == Some("FeatureValue")
            || metaclass_conforms(&relation.kind, "Subsetting") && !metaclass_conforms(&relation.kind, "CrossSubsetting")
            || relation.kind.rsplit("::").next() == Some("TypeFeaturing")
            || result_multiplicity::is_owned_context(graph, owner, relation)?
            || binding_contribution_query_in_view(query, owner, relation)? { continue; }
        return Ok(false);
    }
    Ok(true)
}

struct BindingPlan {
    owner: String,
    result: String,
    dependencies: Option<BindingDependencies>,
}


fn binding_dependencies(query: &DefinitionNameQuery<'_, '_>, result: &KirElement)
    -> Result<BindingDependencies, QueryFailure> {
    binding_dependencies_plan(query, result)?.into_query_result()
}


fn binding_plan(query: &DefinitionNameQuery<'_, '_>, receiver: &str,
    result_id: &str) -> Result<TransformationPlan<BindingPlan>, QueryFailure> {
    let expression=query.index.get(receiver).copied().ok_or_else(||query_failure("binding receiver is absent"))?;
    let owner=definition_value_expression_owner_in_view(query,expression)?;
    // Only callers with a successful cold plan use this pre-completion branch.
    // It assesses containment without supplying a lifecycle completion flag.
    if !stable_inputs_with_receiver_state(query,owner,false)? {
        return Err(query_failure("literal value binding requires stable fixed explicitly typed owner inputs"));
    }
    let value=value_inputs_with_receiver_state(query,owner,false)?.ok_or_else(||query_failure("literal value binding inputs absent"))?;

    if value.default {
        return Ok(TransformationPlan::Ready(BindingPlan {owner: owner.id.clone(), result: result_id.to_owned(), dependencies: None}));
    }
    let result = query.index.get(result_id).copied().ok_or_else(|| query_failure("binding result is absent"))?;
    match binding_dependencies_plan(query, result)? {
        TransformationPlan::Required(requirements) => Ok(TransformationPlan::Required(requirements)),
        TransformationPlan::Ready(dependencies) => Ok(TransformationPlan::Ready(
            BindingPlan {owner: owner.id.clone(), result: result_id.to_owned(), dependencies: Some(dependencies)})),
    }
}

/// Complete selected cold receivers and their non-default value bindings in one
/// transaction. Supplied receiving Features, Functions, results and resources
/// retain their flags; the document scheduler and complete validation are separate.
pub(crate) fn definition_complete_literal_value_bindings(graph: &mut Vec<KirElement>, receivers: &[&str], prefix: &str)
    -> Result<usize, Diagnostic> {
    complete_query(graph,receivers,prefix,ecore_model::ReferenceCompleteness::Closed)
        .map_err(|failure|failure.into_diagnostic_at("literal value binding external boundary"))
}

// Shared atomic receiver/binding composition. Partial is private staging only;
// caller-owned inputs and unrelated lifecycle flags are never completed here.
pub(super) fn complete_query(graph: &mut Vec<KirElement>,
    receivers: &[&str], prefix: &str, boundary: ecore_model::ReferenceCompleteness) -> Result<usize, QueryFailure> {
    complete_query_observed(graph,receivers,prefix,boundary,&|_|{})
}

pub(super) fn complete_query_observed(graph: &mut Vec<KirElement>,
    receivers: &[&str], prefix: &str, boundary: ecore_model::ReferenceCompleteness,
    observer: &dyn for<'e> Fn(DefinitionConstructionEvent<'e>)) -> Result<usize, QueryFailure> {
    complete_query_planned_observed(graph, receivers, prefix, boundary, observer)?.into_query_result()
}

pub(super) fn complete_query_planned_observed(graph: &mut Vec<KirElement>,
    receivers: &[&str], prefix: &str, boundary: ecore_model::ReferenceCompleteness,
    observer: &dyn for<'e> Fn(DefinitionConstructionEvent<'e>)) -> Result<TransformationPlan<usize>, QueryFailure> {
    let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
    let query=DefinitionNameQuery::observed(graph,&index,observer);

    let cold_plans = match cold_literal_values::plan_batch_query_in_view(&query, receivers, prefix, boundary)? {
        TransformationPlan::Required(requirements) => return Ok(TransformationPlan::Required(requirements)),
        TransformationPlan::Ready(plans) => plans,
    };
    let mut reads = TransformationReads::default();
    let mut plans = BTreeMap::new();
    for (id, inputs) in &cold_plans {
        if let Some(plan) = reads.capture_plan(query.trace_plan(
            DefinitionConstructionPhase::LiteralStageStarted, DefinitionConstructionPhase::LiteralStageFinished,
            id, None, Some("binding_prerequisites"), || binding_plan(&query, id, &inputs.result))) {
            plans.insert(id.clone(), plan);
        }
    }
    let binding_plans = match reads.finish(|| Ok(plans))? {
        TransformationPlan::Required(requirements) => return Ok(TransformationPlan::Required(requirements)),
        TransformationPlan::Ready(plans) => plans,
    };
    let mut staged = graph.clone();
    cold_literal_values::complete_planned_query_observed_in_stage(&mut staged,&cold_plans,prefix,boundary,observer)?;
    let mut count = 0;
    let mut selected = receivers.to_vec(); selected.sort();
    for id in selected {
        let expression = staged.iter().find(|e| e.id == id).ok_or_else(|| query_failure("value binding receiver absent"))?;
        let owner = definition_value_expression_owner(&staged, expression)?;
        let owner_id = owner.id.clone();
        let index=staged.iter().map(|e|(e.id.as_str(),e)).collect();
        let query=DefinitionNameQuery::observed(&staged,&index,observer);
        if !query.trace(DefinitionConstructionPhase::LiteralStageStarted, DefinitionConstructionPhase::LiteralStageFinished, id, None, Some("binding_inputs"), || stable_inputs_query_in_view(&query, owner))? { return Err(query_failure("literal value binding requires stable fixed explicitly typed owner inputs")); }
        let value = value_inputs_query_in_view(&query, owner)?.ok_or_else(|| query_failure("literal value binding inputs absent"))?;
        let plan=binding_plans.get(id).ok_or_else(||query_failure("binding prerequisite plan is absent"))?;
        let result = definition_result_parameter_typed_query(&query, expression)?.ok_or_else(|| query_failure("literal value binding result absent"))?;
        if owner_id != plan.owner || result.id != plan.result || value.default != plan.dependencies.is_none() {
            return Err(query_failure("literal completion changed binding prerequisite selection"));
        }
        if value.default { continue; }
        if plan.dependencies.as_ref() != Some(&binding_dependencies(&query,result)?) {
            return Err(query_failure("literal completion changed binding dependency projections"));
        }
        let result=result.id.clone();
        let chain_id = format!("{prefix}.{id}.value-chain");
        let binding_id = format!("{prefix}.{id}.value-binding");
        trace_definition_stage(&mut staged, id, "binding_chain", observer, |graph| definition_append_feature_chain_query_in_stage(graph, &chain_id, &[id, &result],boundary))?;
        // Same fresh-binding constructor/default/reduction/featuring services as
        // ordinary bindings; the structural admissions above cover adoption and
        // incomplete value owners without the unrelated graph-wide exclusion.
        trace_definition_stage(&mut staged, id, "binding_lifecycle", observer, |graph| definition_finish_fresh_binding_observed_query_in_stage(graph, &owner_id, &binding_id, &chain_id, &owner_id,boundary,id,observer))?;
        let member = staged.iter().find(|e| e.id == format!("{binding_id}.membership")).ok_or_else(|| query_failure("value binding membership absent"))?;
        let owner = staged.iter().find(|e| e.id == owner_id).ok_or_else(|| query_failure("value binding owner absent"))?;
        let index=staged.iter().map(|e|(e.id.as_str(),e)).collect();
        let query=DefinitionNameQuery::observed(&staged,&index,observer);
        if !binding_contribution_query_in_view(&query, owner, member)? { return Err(query_failure("constructed value binding disagrees with canonical valuation/result chain")); }
        count += 1;
    }
    if let Some(issue) = ecore_model::validate_publication(&staged, boundary).first() {
        return Err(query_failure(format!("invalid literal value-binding output: {}", issue.message)));
    }
    *graph = staged; Ok(TransformationPlan::Ready(count))
}
