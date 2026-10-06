//! Resume explicit native dependency waves from retained native construction.
//! Handwritten registry validation over imported Ecore contracts. This is neither
//! model interchange nor publication. Provenance/invalidation belongs to tooling.
use super::*;

pub(crate) fn inspect_constructed_resources_with_plan_traced(
    resources: &[(&str, bool)], graph: Vec<KirElement>,
    descriptors: &[DefinitionPendingReference], requirements: &[Prerequisite],
    observer: &impl Fn(DefinitionConstructionEvent<'_>),
) -> Result<(Vec<KirElement>, Vec<DefinitionPendingReference>), Diagnostic> {
    let index = graph.iter().map(|e| (e.id.as_str(), e)).collect::<BTreeMap<_, _>>();
    if index.len() != graph.len() || index.contains_key("") {
        return Err(error("retained construction requires unique nonempty identities"));
    }
    let roots = resources.iter().map(|(id, _)| *id).collect::<BTreeSet<_>>();
    if roots.is_empty() || roots.len() != resources.len()
        || roots.iter().any(|id| index.get(*id).is_none_or(|e| e.kind.rsplit("::").next() != Some("Namespace"))) {
        return Err(error("retained construction requires distinct canonical Namespace resource roots"));
    }
    let belongs = |id: &str| roots.iter().filter(|root|
        id == **root || id.strip_prefix(**root).is_some_and(|tail| tail.starts_with('.'))).count() == 1;
    if graph.iter().any(|e| !belongs(&e.id)
        || !crate::language_frontend::lowering::relationship_declarations::metaclass_names().contains(&e.kind.rsplit("::").next().unwrap_or(&e.kind))) {
        return Err(error("retained construction contains a foreign resource or unknown Ecore class"));
    }
    if let Some(issue) = ecore_model::validate_publication(&graph, ecore_model::ReferenceCompleteness::Closed).first() {
        return Err(error(format!("invalid retained construction: {}", issue.message)));
    }

    // Preserve first occurrence and per-field syntax order. A descriptor cannot
    // replace an already committed field or add an arbitrary reference writer.
    struct Port { owner_id: String, owner_kind: String, feature: &'static ecore_model::FeatureContract,
        references: Vec<crate::xtext_fragment::PendingReference> }
    let mut ports: Vec<Port> = Vec::new();
    let mut slots = BTreeMap::new();
    for descriptor in descriptors {
        let owner = index.get(descriptor.owner_id.as_str()).ok_or_else(|| error("retained pending owner is absent"))?;
        let feature = ecore_model::feature(&owner.kind, &descriptor.field)
            .ok_or_else(|| error("retained pending feature is absent from Ecore"))?;
        if owner.kind != descriptor.owner_kind || feature.id != descriptor.feature_id || feature.target != descriptor.ecore_target
            || feature.kind != ecore_model::FeatureKind::Reference || feature.containment || feature.container
            || feature.derived || feature.opposite.is_some() || owner.properties.contains_key(&descriptor.field) {
            return Err(error("retained pending contract differs, is unsupported, or is already committed"));
        }
        let prefix = feature.id.split_once("#//").map(|(uri, _)| format!("{uri}#//"))
            .ok_or_else(|| error("retained pending feature has no resolved URI"))?;
        let target = descriptor.grammar_target.strip_prefix(&prefix)
            .ok_or_else(|| error("retained grammar target has a foreign Ecore URI"))?;
        ecore_model::validate_reference_endpoint(&owner.kind, &descriptor.field, target).map_err(error)?;
        if descriptor.spelling.trim().is_empty() || descriptor.spelling.contains('\0') {
            return Err(error("retained pending spelling must be nonempty"));
        }
        let key = (descriptor.owner_id.as_str(), descriptor.field.as_str());
        let slot = *slots.entry(key).or_insert_with(|| {
            let slot = ports.len();
            ports.push(Port { owner_id: descriptor.owner_id.clone(), owner_kind: descriptor.owner_kind.rsplit("::").next().unwrap().to_owned(),
                feature, references: Vec::new() });
            slot
        });
        let port = &mut ports[slot];
        if descriptor.reference_ordinal != port.references.len()
            || feature.upper >= 0 && port.references.len() >= feature.upper as usize {
            return Err(error("retained pending ordinals or multiplicity differ"));
        }
        port.references.push(crate::xtext_fragment::PendingReference {
            target_type: descriptor.grammar_target.clone(), spelling: descriptor.spelling.clone(), span: descriptor.span.clone() });
    }
    let pending = ports.iter().map(|port| PendingModelLink { owner_id: port.owner_id.clone(), owner_kind: port.owner_kind.clone(),
        feature: port.feature, references: &port.references, ancestors: Vec::new() }).collect();
    let (graph, remaining) = execute_definition_dependency_plan(graph, pending, resources, requirements, observer)?;
    Ok((graph, describe_pending_references(&remaining)))
}
