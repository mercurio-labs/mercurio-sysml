//! Bounded handwritten ownership, import and multiplicity projections. Ecore types,
//! opposite containment and delegate bindings select and check native inputs.
//! These readers do not execute expression evaluation or full language invariants.
use super::*;

fn binding(contract: &ecore_model::FeatureContract) -> Result<(), Diagnostic> {
    let delegate = contract.setting_delegate.as_ref().ok_or_else(|| error("missing projection delegate"))?;
    let expected = format!("org.omg.sysml.delegate.setting.{}_{}_SettingDelegate", contract.owner, contract.name);
    if delegate.uri != "http://www.omg.org/spec/SysML" || delegate.status != "custom_setting_delegate_source"
        || delegate.candidates != [expected.as_str()] {
        return Err(error("unsupported definition projection binding"));
    }
    Ok(())
}

/// Read only requested canonical container steps, with imported reciprocal
/// containment validation at each edge. No ancestor skipping or derived cache.
fn project_query<'g>(query: &DefinitionNameQuery<'g, '_>, owner: &'g KirElement, path: &[&str])
    -> Result<Option<&'g KirElement>, Diagnostic>
{
    let graph = query.graph;
    let index = query.index;
    if index.len() != graph.len() || query.has_empty_identity() {
        return Err(error("ownership projection requires unique nonempty identities"));
    }
    let mut current = owner;
    let mut seen = BTreeSet::new();
    for field in path {
        if !seen.insert(current.id.as_str()) { return Err(error("cyclic ownership projection")); }
        let contract = ecore_model::feature(&current.kind, field).ok_or_else(|| error("missing imported projection container"))?;
        if !contract.container || contract.derived || contract.volatile || contract.kind != ecore_model::FeatureKind::Reference {
            return Err(error("unsupported projection container contract"));
        }
        let Some(value) = current.properties.get(*field).filter(|v| !v.is_null()) else { return Ok(None); };
        ecore_model::validate_value(&current.kind, field, value).map_err(error)?;
        let parent = scope_container_query(query, current)?.ok_or_else(|| error("unresolved projection container"))?;
        if value.as_str() != Some(parent.id.as_str()) { return Err(error("projection container disagrees with canonical ownership")); }
        current = parent;
    }
    if seen.contains(current.id.as_str()) { return Err(error("cyclic ownership projection")); }
    Ok(Some(current))
}

pub(super) fn reference_targets_query<'g>(query: &DefinitionNameQuery<'g, '_>, owner: &'g KirElement,
    contract: &ecore_model::FeatureContract) -> Result<Option<Vec<&'g KirElement>>, Diagnostic>
{
    let field = contract.field;
    let owned_origin = metaclass_conforms(&owner.kind, "Documentation") && field == "documented_element";
    let chained_origin = metaclass_conforms(&owner.kind, "FeatureChaining") && field == "feature_chained";
    let annotating = metaclass_conforms(&owner.kind, "AnnotatingElement") && matches!(field,
        "annotation" | "owned_annotating_relationship" | "owning_annotating_relationship" | "annotated_element");
    let imported = metaclass_conforms(&owner.kind, "Import") && field == "imported_element";
    let multiplicity = metaclass_conforms(&owner.kind, "MultiplicityRange")
        && matches!(field, "bound" | "lower_bound" | "upper_bound");
    let owned_annotation = field == "owned_annotation";
    let annotation_end = metaclass_conforms(&owner.kind, "Annotation") && matches!(field,
        "annotating_element" | "owned_annotating_element" | "owning_annotating_element");
    if !owned_origin && !chained_origin && !annotating && !imported && !multiplicity
        && !owned_annotation && !annotation_end { return Ok(None); }
    binding(contract)?;
    let targets = if owned_annotation {
        if !owner.properties.contains_key("owned_relationship") {
            return Err(error("owned annotation selection requires canonical ownership"));
        }
        let mut selected = Vec::new();
        for relation in stored_children_projection_query(query, owner, "owned_relationship")? {
            if metaclass_conforms(&relation.kind, contract.target) {
                let targets = definition_reference_targets_query(query, relation, "annotated_element")?;
                if targets.len() != 1 { return Err(error("owned annotation requires one resolved target")); }
                if targets[0].id == owner.id { selected.push(relation); }
            }
        }
        selected
    } else if annotation_end {
        match field {
            "owned_annotating_element" => {
                if !owner.properties.contains_key("owned_related_element") {
                    return Err(error("annotating endpoint requires canonical owned elements"));
                }
                stored_children_projection_query(query, owner, "owned_related_element")?.into_iter()
                    .find(|e| metaclass_conforms(&e.kind, contract.target)).into_iter().collect()
            },
            "owning_annotating_element" => project_query(query, owner, &["owning_related_element"])?
                .filter(|e| metaclass_conforms(&e.kind, contract.target)).into_iter().collect(),
            _ => {
                let owned = definition_reference_targets_query(query, owner, "owned_annotating_element")?;
                if owned.is_empty() { definition_reference_targets_query(query, owner, "owning_annotating_element")? }
                else { owned }
            }
        }
    } else if multiplicity {
        // Written ownedMember selection excludes aliases and imported members.
        // Expression evaluation, numeric legality and bound result typing are
        // separate dependencies; this computes the expression identities only.
        if stored_namespace_membership_inputs_query(query, owner)?.is_none() {
            return Err(error("multiplicity projection requires canonical owned memberships"));
        }
        let mut expressions = Vec::new();
        for membership in stored_children_projection_query(query, owner, "owned_relationship")? {
            if metaclass_conforms(&membership.kind, "OwningMembership") {
                expressions.extend(definition_reference_targets_query(query, membership, "member_element")?
                    .into_iter().filter(|e| metaclass_conforms(&e.kind, "Expression")));
            }
        }
        match field {
            "lower_bound" => if expressions.len() < 2 { Vec::new() } else { vec![expressions[0]] },
            "upper_bound" => expressions.get(if expressions.len() == 1 { 0 } else { 1 }).copied().into_iter().collect(),
            _ => expressions.into_iter().take(2).collect(),
        }
    } else if imported {
        // KerML 8.3.2.4.2: compose canonical stored target readers. Ecore
        // ancestry dispatches the two concrete import kinds. Neither a cached
        // importedElement nor a qualified-name guess supplies a missing target.
        if metaclass_conforms(&owner.kind, "NamespaceImport") {
            definition_reference_targets_query(query, owner, "imported_namespace")?
        } else if metaclass_conforms(&owner.kind, "MembershipImport") {
            let memberships = definition_reference_targets_query(query, owner, "imported_membership")?;
            let mut targets = Vec::new();
            for membership in memberships {
                targets.extend(definition_reference_targets_query(query, membership, "member_element")?);
            }
            targets
        } else { return Err(error("imported element requires a concrete import kind")); }
    } else if owned_origin {
        project_query(query, owner, &["owning_relationship", "owning_related_element"])?.into_iter().collect()
    } else if chained_origin {
        project_query(query, owner, &["owning_related_element"])?.filter(|parent|
            metaclass_conforms(&parent.kind, contract.target)).into_iter().collect()
    } else if field == "owning_annotating_relationship" {
        project_query(query, owner, &["owning_relationship"])?.filter(|parent|
            metaclass_conforms(&parent.kind, contract.target)).into_iter().collect()
    } else if field == "owned_annotating_relationship" {
        if !owner.properties.contains_key("owned_relationship") {
            return Err(error("annotation projection requires complete owned relationships"));
        }
        let mut annotations = Vec::new();
        for relation in stored_children_projection_query(query, owner, "owned_relationship")? {
            if metaclass_conforms(&relation.kind, contract.target) {
                let targets = definition_reference_targets_query(query, relation, "annotated_element")?;
                if targets.len() != 1 {
                    return Err(error("owned annotation requires exactly one resolved target"));
                }
                if targets[0].id != owner.id { annotations.push(relation); }
            }
        }
        annotations
    } else if field == "annotation" {
        let mut annotations = definition_reference_targets_query(query, owner, "owning_annotating_relationship")?;
        annotations.extend(definition_reference_targets_query(query, owner, "owned_annotating_relationship")?);
        annotations
    } else {
        let annotations = definition_reference_targets_query(query, owner, "annotation")?;
        if annotations.is_empty() {
            generated_non_expression_namespace_query(query, &owner.id)?.into_iter().collect()
        } else {
            let mut targets = Vec::new();
            for annotation in annotations { targets.extend(definition_reference_targets_query(query, annotation, "annotated_element")?); }
            targets
        }
    };
    let mut targets: Vec<_> = targets;
    if contract.unique {
        let mut seen = BTreeSet::new();
        targets.retain(|target| seen.insert(target.id.as_str()));
    }
    Ok(Some(targets))
}

/// Bounded handwritten operation bodies selected by imported dynamic dispatch.
/// Constant evaluability is independent of numeric evaluation/result construction.
pub(super) fn model_level_evaluable(owner: &KirElement) -> Result<bool, Diagnostic> {
    let contract = ecore_model::model_level_evaluable_contract();
    let [visited] = contract.parameters else { return Err(error("unsupported evaluability parameters")); };
    if contract.name != "modelLevelEvaluable" || contract.owner != "Expression"
        || contract.target != "Ecore::EBoolean" || contract.lower != 1 || contract.upper != 1
        || contract.ordered || !contract.unique || visited.name != "visited" || visited.target != "Feature"
        || visited.lower != 0 || visited.upper != -1 || visited.ordered || !visited.unique
        || contract.delegate_uri != "http://www.omg.org/spec/SysML"
        || contract.binding_status != "dynamic_invocation_candidates_not_selected" {
        return Err(error("unsupported model-level evaluability invocation contract"));
    }
    let applicable: Vec<_> = contract.branches.iter().filter(|(kind, _)| metaclass_conforms(&owner.kind, kind)).collect();
    let specific: Vec<_> = applicable.iter().filter(|(kind, _)| !applicable.iter().any(|(other, _)|
        other != kind && metaclass_conforms(other, kind))).collect();
    let [(kind, delegate)] = specific.as_slice() else { return Err(error("ambiguous or absent evaluability invocation branch")); };
    let expected = format!("org.omg.sysml.delegate.invocation.{kind}_modelLevelEvaluable_InvocationDelegate");
    if **delegate != expected { return Err(error("unsupported evaluability invocation binding")); }
    match *kind {
        "LiteralExpression" | "NullExpression" | "MetadataAccessExpression" => Ok(true),
        _ => Err(error(format!("model-level evaluability requires an unimplemented {kind} algorithm"))),
    }
}

/// Handwritten ElementUtil.getAllMetadataFeaturesOf selection. Imported Ecore
/// ancestry, delegate contracts and reciprocal ownership drive both branches.
/// Nonempty selection is not baseType expression evaluation or semantic support.
pub(super) fn metadata_features<'g>(graph: &'g [KirElement], owner: &'g KirElement)
    -> Result<Vec<&'g KirElement>, Diagnostic> {
    let index = graph.iter().map(|e| (e.id.as_str(), e)).collect::<BTreeMap<_, _>>();
    metadata_features_query(&DefinitionNameQuery::new(graph, &index), owner)
}

pub(super) fn metadata_features_query<'g>(query: &DefinitionNameQuery<'g, '_>, owner: &'g KirElement)
    -> Result<Vec<&'g KirElement>, Diagnostic> {
    query.trace(DefinitionConstructionPhase::MetadataSelectionStarted, DefinitionConstructionPhase::MetadataSelectionFinished,
        &owner.id, Some(&owner.kind), None, || {
    let mut metadata = Vec::new();
    for annotation in definition_reference_targets_query(query, owner, "owned_annotation")? {
        metadata.extend(definition_reference_targets_query(query, annotation, "annotating_element")?
            .into_iter().filter(|e| metaclass_conforms(&e.kind, "MetadataFeature")));
    }
    if metaclass_conforms(&owner.kind, "Namespace") {
        if stored_namespace_membership_inputs_query(query, owner)?.is_none() {
            return Err(error("metadata selection requires complete owned memberships"));
        }
        for membership in stored_children_projection_query(query, owner, "owned_relationship")? {
            if metaclass_conforms(&membership.kind, "OwningMembership") {
                for member in definition_reference_targets_query(query, membership, "member_element")? {
                    if metaclass_conforms(&member.kind, "MetadataFeature")
                        && definition_reference_targets_query(query, member, "annotated_element")?
                            .iter().any(|target| target.id == owner.id) {
                        metadata.push(member);
                    }
                }
            }
        }
    }
    Ok(metadata)
    })
}

pub(super) fn require_empty_metadata_bases(graph: &[KirElement], owner: &KirElement)
    -> Result<(), Diagnostic> {
    let index = graph.iter().map(|e| (e.id.as_str(), e)).collect::<BTreeMap<_, _>>();
    require_empty_metadata_bases_query(&DefinitionNameQuery::new(graph, &index), owner)
}

pub(super) fn require_empty_metadata_bases_query(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement)
    -> Result<(), Diagnostic> {
    let metadata = metadata_features_query(query, owner)?;
    if !metadata.is_empty() {
        return Err(error(format!("semantic metadata base-type evaluation remains unsupported for {}: {:?}",
            owner.id, metadata.iter().map(|e| e.id.as_str()).collect::<Vec<_>>())));
    }
    Ok(())
}
