//! Bounded Pilot implicit feature defaults. Values and selection rules are
//! checked against the pinned adapters by the release extractor.
use super::*;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Defaults {
    usage_defaults: BTreeMap<String, BTreeMap<String, String>>,
}

pub(super) fn apply(
    module: &mut ResolvedModule,
    context: &ResolverContext,
    mappings: &MappingBundle,
    lookup: &FeatureLookup<'_>,
) -> Result<(), Diagnostic> {
    static DEFAULTS: OnceLock<Result<Defaults, String>> = OnceLock::new();
    let defaults = DEFAULTS
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../resources/kernel/conditional-usage-defaults.extract.json"
            ))
            .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| {
            Diagnostic::new(format!("invalid conditional usage defaults: {error}"), None)
        })?;
    let mut usages = BTreeMap::new();
    let mut previous = BTreeMap::new();
    fn index<'a>(
        siblings: &'a [ResolvedUsage],
        usages: &mut BTreeMap<String, &'a ResolvedUsage>,
        previous: &mut BTreeMap<String, &'a ResolvedUsage>,
    ) {
        for (i, usage) in siblings.iter().enumerate() {
            usages.insert(feature_id_from_qualified_name(&usage.qualified_name), usage);
            if i > 0 {
                previous.insert(usage.qualified_name.clone(), &siblings[i - 1]);
            }
            index(&usage.members, usages, previous);
        }
    }
    index(&module.usages, &mut usages, &mut previous);
    for definition in &module.definitions {
        index(&definition.members, &mut usages, &mut previous);
    }
    let mut selected = BTreeMap::new();
    for usage in usages.values() {
        let kind = mappings
            .metaclass_for(&usage.construct)?
            .rsplit("::")
            .next()
            .unwrap_or("");
        let Some(values) = defaults.usage_defaults.get(kind) else {
            continue;
        };
        let selector = if matches!(
            kind,
            "ConnectionUsage"
                | "Connector"
                | "BindingConnector"
                | "Succession"
                | "SuccessionAsUsage"
                | "BindingConnectorAsUsage"
        ) {
            let ends = usage
                .members
                .iter()
                .filter(|member| {
                    member
                        .modifiers
                        .iter()
                        .any(|m| m == "end" || m.starts_with("end-"))
                })
                .count();
            let mut structure_type = false;
            let mut class_type = false;
            let mut data_type = false;
            if kind != "ConnectionUsage" && usage.has_explicit_type {
                for ty in usage.type_ref.iter().chain(&usage.additional_type_refs) {
                    let context_kind = ty
                        .strip_prefix("type.")
                        .and_then(|name| context.definition_index.get(name))
                        .map(|d| mappings.metaclass_for(&d.construct))
                        .transpose()?;
                    if let Some(kind) = context_kind
                        .or_else(|| context.library_indexes.kinds.get(ty).map(String::as_str))
                    {
                        structure_type |= typing::kind_conforms(kind, "Structure")?;
                        if usage.construct == "BindingConnectorAsUsage" {
                            class_type |= typing::kind_conforms(kind, "Class")?;
                            data_type |= typing::kind_conforms(kind, "DataType")?;
                        }
                    }
                }
            }
            if kind == "BindingConnectorAsUsage" {
                if structure_type {
                    "object"
                } else if class_type {
                    if usage
                        .modifiers
                        .iter()
                        .any(|m| matches!(m.as_str(), "portion" | "snapshot" | "timeslice"))
                    {
                        "portion"
                    } else {
                        "occurrence"
                    }
                } else if data_type {
                    "dataValue"
                } else {
                    "base"
                }
            } else {
                match (ends == 2, structure_type) {
                    (true, true) => "binaryObject",
                    (false, true) if kind != "SuccessionAsUsage" => "object",
                    (false, true) => "base",
                    (true, false) => "binary",
                    (false, false) => "base",
                }
            }
        } else {
            let source = usage
                .modifiers
                .iter()
                .find_map(|m| m.strip_prefix("transition_source="));
            let source_id = source.and_then(|source| {
                lookup.resolve(
                    &QualifiedName {
                        segments: source
                            .replace("::", ".")
                            .split('.')
                            .map(str::to_string)
                            .collect(),
                        span: usage.span.clone(),
                    },
                    &usage.owner_qualified_name,
                    "",
                    &mut BTreeSet::new(),
                )
            });
            let source_usage = source_id
                .as_ref()
                .and_then(|id| usages.get(id).copied())
                .or_else(|| {
                    source
                        .is_none()
                        .then(|| previous.get(&usage.qualified_name).copied())
                        .flatten()
                });
            let source_kind = if let Some(source) = source_usage {
                Some(mappings.metaclass_for(&source.construct)?)
            } else if let Some(source) = source_id
                .as_ref()
                .and_then(|id| id.strip_prefix("feature."))
                .and_then(|name| context.local_usage_map.get(name))
            {
                Some(mappings.metaclass_for(&source.construct)?)
            } else {
                source_id
                    .as_ref()
                    .and_then(|id| context.library_indexes.kinds.get(id).map(String::as_str))
            };
            let is_state_source = source_kind
                .map(|kind| typing::kind_conforms(kind, "StateUsage"))
                .transpose()?
                .unwrap_or(false);
            let owner = mappings.metaclass_for(&usage.owner_construct)?;
            let composite = usage.derived_properties.get("is_composite") == Some(&true);
            if composite
                && is_state_source
                && (typing::kind_conforms(owner, "StateDefinition")?
                    || typing::kind_conforms(owner, "StateUsage")?)
            {
                "stateTransition"
            } else if composite
                && !is_state_source
                && (typing::kind_conforms(owner, "ActionDefinition")?
                    || typing::kind_conforms(owner, "ActionUsage")?)
            {
                "actionTransition"
            } else {
                "base"
            }
        };
        if let Some(feature) = values.get(selector) {
            // Partial library fixtures do not establish these implicit types.
            if context.library_indexes.feature_types.contains_key(feature) {
                selected.insert(usage.qualified_name.clone(), feature.clone());
            }
        }
    }
    fn add(usage: &mut ResolvedUsage, selected: &BTreeMap<String, String>) {
        if let Some(feature) = selected.get(&usage.qualified_name) {
            if !usage.subsetted_features.contains(feature) {
                usage.subsetted_features.push(feature.clone());
            }
        }
        for member in &mut usage.members {
            add(member, selected);
        }
    }
    for usage in &mut module.usages {
        add(usage, &selected);
    }
    for definition in &mut module.definitions {
        for usage in &mut definition.members {
            add(usage, &selected);
        }
    }
    // Materialize the same minimal type set used by validation, including every
    // inherited type of binary connections and referenced connector ends, without
    // replacing declared typing. ReferenceSubsetting contributes typing features
    // through the same traversal used by Pilot's FeatureAdapter.getAllTypes.
    let mut usages = BTreeMap::new();
    index(&module.usages, &mut usages, &mut BTreeMap::new());
    for definition in &module.definitions {
        index(&definition.members, &mut usages, &mut BTreeMap::new());
    }
    let types: BTreeMap<_, _> = usages
        .values()
        .filter(|u| {
            selected.contains_key(&u.qualified_name)
                || u.modifiers.iter().any(|m| m == "owned_crossing_feature")
                // RequirementConstraintUsage reference alternatives retain
                // the referenced constraint's minimal typing, not just the base default.
                || (matches!(u.construct.as_str(), "AssumeUsage" | "RequireUsage")
                    && u.reference_target.is_some())
                || (!u.has_explicit_type
                    && u.reference_target.is_some()
                    && u.modifiers
                        .iter()
                        .any(|m| m == "end" || m.starts_with("end-")))
        })
        .map(|u| {
            (
                u.qualified_name.clone(),
                typing::all_types(u, &usages, context, mappings, lookup),
            )
        })
        .collect();
    fn assign(usage: &mut ResolvedUsage, types: &BTreeMap<String, Vec<String>>) {
        if let Some(types) = types.get(&usage.qualified_name) {
            usage.type_ref = types.first().cloned();
            usage.additional_type_refs = types.iter().skip(1).cloned().collect();
        }
        for member in &mut usage.members {
            assign(member, types);
        }
    }
    for usage in &mut module.usages {
        assign(usage, &types);
    }
    for definition in &mut module.definitions {
        for usage in &mut definition.members {
            assign(usage, &types);
        }
    }
    Ok(())
}
