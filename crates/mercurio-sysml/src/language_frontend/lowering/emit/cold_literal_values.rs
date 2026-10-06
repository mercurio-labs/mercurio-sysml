//! Handwritten lifecycle composition for cold literal/null FeatureValue leaves.
//! Imported adapter dispatch selects defaults, no-member and redefinition policies;
//! Ecore supplies shape, ownership and endpoint contracts. Supplied owners and
//! prototypes are queried, never marked complete. This is receiver completion,
//! not value binding, expression evaluation or complete-document qualification.
use super::*;

pub(super) fn supports(kind: &str) -> bool {
    matches!(
        kind,
        "LiteralBoolean"
            | "LiteralInteger"
            | "LiteralRational"
            | "LiteralString"
            | "LiteralInfinity"
            | "NullExpression"
    )
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Inputs {
    pub(super) result: String,
    generals: Vec<String>,
    types: Vec<String>,
    featuring: Vec<String>,
}

fn input_plan(query: &DefinitionNameQuery<'_, '_>, id: &str) -> Result<TransformationPlan<Inputs>, QueryFailure> {
    let graph = query.graph;
    let expression = graph
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| query_failure("cold literal receiver is absent"))?;
    let kind = expression
        .kind
        .rsplit("::")
        .next()
        .unwrap_or(&expression.kind);
    if !supports(kind) || scope_boolean(expression, "is_implied_included")? {
        return Err(query_failure(
            "cold literal lifecycle requires an incomplete admitted literal/null receiver",
        ));
    }
    for field in ["is_end", "is_variable", "is_composite", "is_portion"] {
        if scope_boolean(expression, field)? {
            return Err(query_failure(
                "cold literal lifecycle requires fixed non-end noncomposite leaf inputs",
            ));
        }
    }
    if expression
        .properties
        .get("direction")
        .is_some_and(|v| !v.is_null())
    {
        return Err(query_failure(
            "cold literal lifecycle requires a nondirected receiver",
        ));
    }
    let owner = definition_value_expression_owner(graph, expression)?;
    let index = query.index;
    let value = scope_container(index, expression)?
        .ok_or_else(|| query_failure("cold literal has no valuation"))?;
    if !canonical_single_value(graph, owner, value, expression)? {
        return Err(query_failure(
            "cold literal requires one canonical explicit FeatureValue",
        ));
    }
    // FeatureValue is an OwningMembership, not FeatureMembership: no owningType,
    // positional/end/assignment redefinition or owning-Type featuring producer.
    if definition_feature_owner(graph, expression)?.is_some()
        || !definition_no_additional_members(expression)?
    {
        return Err(query_failure(
            "cold literal lifecycle requires no owning Type or additional-member producer",
        ));
    }
    definition_projection::require_empty_metadata_bases(graph, expression)?;
    for relation in stored_children(graph, expression, "owned_relationship")? {
        if !stored_children(graph, relation, "owned_related_element")?.is_empty() {
            return Err(query_failure(
                "cold literal leaf requires child-free supplied contributions",
            ));
        }
        if !definition_non_typing_featuring(graph, expression, relation)
            ?
            && !(relation.kind.rsplit("::").next() == Some("Subsetting")
                && scope_boolean(relation, "is_implied")?)
        {
            return Err(query_failure("cold literal has an unassessed child, result binding, valuation or generalization producer"));
        }
    }
    // Validate partial defaults against the actual resolved producer, rather
    // than letting a raw completeness flag hide unrelated implied contributions.

    let mut reads = TransformationReads::default();
    let owner_featuring = reads.capture(query.trace(
        DefinitionConstructionPhase::LiteralStageStarted, DefinitionConstructionPhase::LiteralStageFinished,
        id, Some(&expression.kind), Some("owner_featuring"),
        || definition_featuring_types_in_view(query, owner)));
    let defaults = reads.capture(definition_expression_default_contributions_query(query, expression));
    let redefinitions = reads.capture(definition_redefined_features_typed_query(query, expression, false));
    let result = reads.capture(query.trace(
        DefinitionConstructionPhase::LiteralStageStarted, DefinitionConstructionPhase::LiteralStageFinished,
        id, Some(&expression.kind), Some("result_parameter"),
        || definition_result_parameter_typed_query(query, expression)));
    let types = reads.capture(query.trace(
        DefinitionConstructionPhase::LiteralStageStarted, DefinitionConstructionPhase::LiteralStageFinished,
        id, Some(&expression.kind), Some("receiver_types"),
        || definition_feature_types_in_view(query, expression)));
    let generals = reads.capture(definition_general_type_inputs_with_view(query, expression, ""));
    let featuring = reads.capture(definition_featuring_types_in_view(query, expression));
    // Every independent port above reads the same immutable graph. Dependent
    // ancestry and structural checks run only when those inputs are available.
    // No pending result is replaced by an empty or invented semantic answer.
    if let Some(owner_featuring) = &owner_featuring {
        let ids = owner_featuring.iter().map(|e| e.id.as_str()).collect::<BTreeSet<_>>();
        for relation in definition_owned_type_featuring_typed_query(graph, expression)? {
            if scope_boolean(relation, "is_implied")?
                && !ids.contains(query_reference(graph, relation, "featuring_type")?.id.as_str()) {
                return Err(query_failure("cold literal has an unrelated implied featuring contribution"));
            }
        }
    }
    if redefinitions.as_ref().is_some_and(|values| !values.is_empty()) {
        return Err(query_failure("cold literal lifecycle requires empty computed redefinitions"));
    }
    let mut function = None;
    if let Some(result) = &result {
        let result = result.ok_or_else(|| query_failure("cold literal requires a resolved inherited result"))?;
        let Some((membership, owner)) = definition_feature_owner(graph, result)? else {
            return Err(query_failure("cold literal result requires canonical Function ownership"));
        };
        if !metaclass_conforms(&membership.kind, "ReturnParameterMembership")
            || !metaclass_conforms(&owner.kind, "Function")
            || result.properties.get("direction") != Some(&json!("out")) {
            return Err(query_failure("cold literal result must be an inherited Function return parameter"));
        }
        function = Some(owner);
    }
    if let Some(types) = &types {
        if types.is_empty() || types.iter().any(|t| !metaclass_conforms(&t.kind, "Function")) {
            return Err(query_failure("cold literal requires resolved Function prototypes, not untyped role placeholders"));
        }
        if let Some(function) = function {
            let mut inherited = false;
            let mut complete = true;
            for prototype in types {
                if let Some(value) = reads.capture(definition_specializes_depth_in_view(query, prototype, function, 0)) {
                    inherited |= value;
                } else { complete = false; }
            }
            if complete && !inherited {
                return Err(query_failure("cold literal result is outside its resolved prototype ancestry"));
            }
        }
    }
    reads.finish(|| {
        // Values are present iff all typed ports succeeded. This guard protects
        // the plan invariant without unwraps or semantic placeholders.
        if defaults.is_none() || redefinitions.is_none() || owner_featuring.is_none() {
            return Err(query_failure("cold literal plan has incomplete verified inputs"));
        }
        Ok(Inputs {
            result: result.flatten().ok_or_else(|| query_failure("cold literal plan result absent"))?.id.clone(),
            generals: generals.ok_or_else(|| query_failure("cold literal plan generals absent"))?,
            types: types.ok_or_else(|| query_failure("cold literal plan types absent"))?
                .iter().map(|e| e.id.clone()).collect(),
            featuring: featuring.ok_or_else(|| query_failure("cold literal plan featuring absent"))?
                .iter().map(|e| e.id.clone()).collect(),
        })
    })
}

fn canonical_single_value(
    graph: &[KirElement],
    owner: &KirElement,
    value: &KirElement,
    expression: &KirElement,
) -> Result<bool, Diagnostic> {
    if !metaclass_conforms(&value.kind, "FeatureValue") || scope_boolean(value, "is_implied")? {
        return Ok(false);
    }
    let values = stored_children(graph, owner, "owned_relationship")?
        .into_iter()
        .filter(|r| metaclass_conforms(&r.kind, "FeatureValue"))
        .collect::<Vec<_>>();
    if values.len() != 1 || values[0].id != value.id {
        return Ok(false);
    }
    scope_boolean(value, "is_default")?;
    scope_boolean(value, "is_initial")?;
    let children = stored_children(graph, value, "owned_related_element")?;
    let selected = definition_reference_targets(graph, value, "value")?;
    Ok(children.len() == 1 && selected.len() == 1 && selected[0].id == expression.id)
}

/// Execute the whole admitted receiver category as one transaction. Only selected
/// receiver Types are completed after physical contributions reproduce every
/// pre-transform query. No supplied prototype, result, Feature or resource is
/// completed. A failure, including a late identity collision, rolls back all.
pub(crate) fn definition_complete_literal_value_expressions(graph: &mut Vec<KirElement>, owners: &[&str], prefix: &str)
    -> Result<usize, Diagnostic> {
    complete_query(graph,owners,prefix,ecore_model::ReferenceCompleteness::Closed)
        .map_err(|failure|failure.into_diagnostic_at("literal receiver completion external boundary"))
}

// Only the document transaction uses Partial. Selected semantic inputs and each
// contribution are still checked; a missing input remains a typed prerequisite.
pub(super) fn complete_query(
    graph: &mut Vec<KirElement>,
    owners: &[&str],
    prefix: &str,
    boundary: ecore_model::ReferenceCompleteness,
) -> Result<usize, QueryFailure> {
    complete_query_observed(graph,owners,prefix,boundary,&|_|{})
}

pub(super) fn complete_query_observed(
    graph: &mut Vec<KirElement>, owners: &[&str], prefix: &str,
    boundary: ecore_model::ReferenceCompleteness,
    observer: &dyn for<'e> Fn(DefinitionConstructionEvent<'e>),
) -> Result<usize, QueryFailure> {
    let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
    let query=DefinitionNameQuery::observed(graph,&index,observer);
    let plans=plan_query_in_view(&query,owners,prefix,boundary)?;
    complete_planned_query_observed(graph,&plans,prefix,boundary,observer)
}

// A plan records the same guarded semantic inputs used by completion. It is
// scoped to this transaction; writes must reproduce its projections below.
pub(super) fn plan_query_in_view(query: &DefinitionNameQuery<'_, '_>, owners: &[&str],
    prefix: &str, boundary: ecore_model::ReferenceCompleteness) -> Result<Vec<(String, Inputs)>, QueryFailure> {
    plan_batch_query_in_view(query, owners, prefix, boundary)?.into_query_result()
}

pub(super) fn plan_batch_query_in_view(query: &DefinitionNameQuery<'_, '_>, owners: &[&str],
    prefix: &str, boundary: ecore_model::ReferenceCompleteness) -> Result<TransformationPlan<Vec<(String, Inputs)>>, QueryFailure> {
    let graph=query.graph;
    if prefix.is_empty() {
        return Err(query_failure(
            "cold literal lifecycle requires a nonempty identity prefix",
        ));
    }
    if let Some(issue) =
        ecore_model::validate_publication(graph, boundary).first()
    {
        return Err(query_failure(format!(
            "invalid cold literal input: {}",
            issue.message
        )));
    }
    reject_result_snapshots(graph)?;
    let mut selected = owners.to_vec();
    selected.sort();
    if selected.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(query_failure("cold literal batch has duplicate receivers"));
    }

    let mut reads = TransformationReads::default();
    let mut plans = Vec::new();
    for id in selected {
        if let Some(plan) = reads.capture_plan(query.trace_plan(
            DefinitionConstructionPhase::LiteralStageStarted, DefinitionConstructionPhase::LiteralStageFinished,
            id, None, Some("cold_inputs"), || input_plan(query, id))) {
            plans.push((id.to_owned(), plan));
        }
    }
    reads.finish(|| Ok(plans))
}


pub(super) fn complete_planned_query_observed(graph: &mut Vec<KirElement>,
    plans: &[(String, Inputs)], prefix: &str, boundary: ecore_model::ReferenceCompleteness,
    observer: &dyn for<'e> Fn(DefinitionConstructionEvent<'e>)) -> Result<usize, QueryFailure> {
    let mut staged=graph.clone();
    let result=complete_planned_query_observed_in_stage(&mut staged,plans,prefix,boundary,observer)?;
    *graph=staged;
    Ok(result)
}

// Private worker: the caller must discard its entire stage on any failure.
pub(super) fn complete_planned_query_observed_in_stage(graph: &mut Vec<KirElement>,
    plans: &[(String, Inputs)], prefix: &str, boundary: ecore_model::ReferenceCompleteness,
    observer: &dyn for<'e> Fn(DefinitionConstructionEvent<'e>)) -> Result<usize, QueryFailure> {

    for (id, expected) in plans {
        trace_definition_stage(graph, id, "receiver_defaults", observer, |graph| definition_materialize_value_expression_defaults_query(
            graph,
            id,
            &format!("{prefix}.{id}.generals"),
            boundary,
        ))?;
        trace_definition_stage(graph, id, "receiver_featuring", observer, |graph| definition_materialize_value_expression_featuring_query(
            graph,
            id,
            &format!("{prefix}.{id}.featuring"),
            boundary,
        ))?;
        // Preserve original explicit storage, then flush every admitted partial
        // implicit featuring contribution after implicit specializations.
        let expression = graph
            .iter()
            .find(|e| e.id == *id)
            .ok_or_else(|| query_failure("graph literal is absent"))?;
        let relationships = stored_children(graph, expression, "owned_relationship")?;
        let mut original = Vec::new();
        let mut featuring = Vec::new();
        for relation in relationships {
            // Shared constructors may adopt a detached endpoint. Such an owned
            // subtree needs its own transformation, so it cannot complete as a
            // leaf merely because the endpoint queries happened to succeed.
            if !stored_children(graph, relation, "owned_related_element")?.is_empty() {
                return Err(query_failure("cold literal contribution adopted a subtree requiring separate lifecycle completion"));
            }
            if metaclass_conforms(&relation.kind, "TypeFeaturing")
                && scope_boolean(relation, "is_implied")?
            {
                featuring.push(relation.id.clone());
            } else {
                original.push(relation.id.clone());
            }
        }
        original.extend(featuring);
        // All omitted producer branches have been discharged by the leaf admission.
        let expression = graph
            .iter_mut()
            .find(|e| e.id == *id)
            .ok_or_else(|| query_failure("graph literal is absent"))?;
        expression
            .properties
            .insert("owned_relationship".into(), json!(original));
        expression
            .properties
            .insert("is_implied_included".into(), json!(true));
        let expression = graph
            .iter()
            .find(|e| e.id == *id)
            .ok_or_else(|| query_failure("completed literal is absent"))?;
        let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
        let query=DefinitionNameQuery::observed(graph,&index,observer);
        let actual = Inputs {
            result: definition_result_parameter_typed_query(&query,expression)?
                .ok_or_else(|| query_failure("completed literal lost its result"))?
                .id
                .clone(),
            generals: definition_general_type_inputs_with_view(&query, expression, "")
                ?,
            types: definition_feature_types_in_view(&query, expression)
                ?
                .iter()
                .map(|e| e.id.clone())
                .collect(),
            featuring: definition_featuring_types_in_view(&query, expression)?
                .iter()
                .map(|e| e.id.clone())
                .collect(),
        };
        if actual != *expected {
            return Err(query_failure(
                "cold literal physical stages changed result, general types, typing or featuring",
            ));
        }
    }
    if let Some(issue) =
        ecore_model::validate_publication(graph, boundary)
            .first()
    {
        return Err(query_failure(format!(
            "invalid cold literal output: {}",
            issue.message
        )));
    }

    Ok(plans.len())
}
