//! Ordered connector relations through owned ends and locally bound values.
use super::*;

pub(super) fn derive_relations(
    module: &mut ResolvedModule,
    mappings: &MappingBundle,
) -> Result<(), Diagnostic> {
    let mut pending = module.usages.iter().collect::<Vec<_>>();
    pending.extend(module.definitions.iter().flat_map(|d| &d.members));
    let mut usages = BTreeMap::new();
    while let Some(usage) = pending.pop() {
        pending.extend(&usage.members);
        usages.insert(feature_id_from_qualified_name(&usage.qualified_name), usage);
    }
    fn related(
        usage: &ResolvedUsage,
        usages: &BTreeMap<String, &ResolvedUsage>,
        seen: &mut BTreeSet<String>,
    ) -> Vec<String> {
        if !seen.insert(usage.qualified_name.clone()) {
            return Vec::new();
        }
        let ends = usage
            .members
            .iter()
            .filter(|u| {
                u.modifiers
                    .iter()
                    .any(|m| m == "end" || m.starts_with("end-"))
            })
            .collect::<Vec<_>>();
        if !ends.is_empty() {
            return ends
                .iter()
                .map(|end| end.reference_target.clone())
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default();
        }
        if !usage.has_explicit_specialization
            && !usage.modifiers.iter().any(|m| {
                matches!(
                    m.as_str(),
                    "feature_value_is_default" | "in" | "out" | "inout" | "return"
                )
            })
            && let Some(ResolvedExpr::FeaturePath { segments }) = &usage.expression
            && let Some(target) = segments.last().and_then(|s| usages.get(&s.feature_id))
        {
            return related(target, usages, seen);
        }
        Vec::new()
    }
    let mut relations = BTreeMap::new();
    for usage in usages.values() {
        if !usage.members.iter().any(|m| {
            m.modifiers
                .iter()
                .any(|v| v == "end" || v.starts_with("end-"))
        }) && !matches!(usage.expression, Some(ResolvedExpr::FeaturePath { .. }))
        {
            continue;
        }
        if typing::kind_conforms(mappings.metaclass_for(&usage.construct)?, "Connector")? {
            relations.insert(
                usage.qualified_name.clone(),
                related(usage, &usages, &mut BTreeSet::new()),
            );
        }
    }
    fn assign(usage: &mut ResolvedUsage, values: &mut BTreeMap<String, Vec<String>>) {
        if let Some(related) = values.remove(&usage.qualified_name) {
            usage.related_features = related;
        }
        for member in &mut usage.members {
            assign(member, values);
        }
    }
    for usage in &mut module.usages {
        assign(usage, &mut relations);
    }
    for definition in &mut module.definitions {
        for usage in &mut definition.members {
            assign(usage, &mut relations);
        }
    }
    Ok(())
}
