//! Source semantic metadata specialization, following Pilot TypeAdapter,
//! ClassifierAdapter and FeatureAdapter. Only statically resolved metaclass
//! references are evaluated here; arbitrary metadata expression evaluation and
//! imported-library feature values remain separate qualification work.
use super::*;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Policy {
    base_feature: String,
    classifier_base: String,
    feature_base: String,
}

fn policy() -> Result<&'static Policy, Diagnostic> {
    static POLICY: OnceLock<Result<Policy, String>> = OnceLock::new();
    POLICY.get_or_init(|| {
        let policy: Policy = serde_json::from_str(include_str!(
            "../../../../resources/metamodels/sysml-2.0-pilot-2026-08/semantic-metadata.extract.json"
        )).map_err(|error| error.to_string())?;
        if policy.classifier_base != "feature_types_or_classifier" || policy.feature_base != "feature_only" {
            return Err("unsupported semantic metadata policy".into());
        }
        Ok(policy)
    }).as_ref().map_err(|error| Diagnostic::new(format!("invalid semantic metadata extraction: {error}"), None))
}

fn annotations(modifiers: &[String]) -> impl Iterator<Item = &str> {
    modifiers
        .iter()
        .filter_map(|modifier| modifier.strip_prefix("language_extension="))
}

fn qualified(id: &str, span: &SourceSpan) -> QualifiedName {
    QualifiedName {
        segments: id
            .strip_prefix("feature.")
            .or_else(|| id.strip_prefix("type."))
            .unwrap_or(id)
            .replace("::", ".")
            .split('.')
            .map(str::to_string)
            .collect(),
        span: span.clone(),
    }
}

fn feature_specializes(
    id: &str,
    general: &str,
    context: &ResolverContext,
    lookup: &FeatureLookup<'_>,
    seen: &mut BTreeSet<String>,
) -> bool {
    if id == general {
        return true;
    }
    if !seen.insert(id.to_string()) {
        return false;
    }
    if let Some(usage) = id
        .strip_prefix("feature.")
        .and_then(|name| context.local_usage_map.get(name))
    {
        return usage
            .redefines
            .iter()
            .chain(&usage.subsets)
            .chain(&usage.specializes)
            .any(|name| {
                lookup
                    .resolve(name, &usage.owner_qualified_name, id, &mut BTreeSet::new())
                    .is_some_and(|parent| {
                        feature_specializes(&parent, general, context, lookup, seen)
                    })
            });
    }
    context
        .library_indexes
        .specializations
        .get(id)
        .is_some_and(|parents| {
            parents
                .iter()
                .any(|parent| feature_specializes(parent, general, context, lookup, seen))
        })
}

fn bases(
    annotation: &str,
    scope: &str,
    span: &SourceSpan,
    context: &ResolverContext,
    lookup: &FeatureLookup<'_>,
    base_feature: &str,
) -> Vec<String> {
    let Some(metadata) = lookup.resolve(
        &qualified(annotation, span),
        scope,
        "",
        &mut BTreeSet::new(),
    ) else {
        return Vec::new();
    };
    let mut pending = vec![metadata];
    let mut seen = BTreeSet::new();
    let mut candidates = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(definition) = id
            .strip_prefix("type.")
            .and_then(|name| context.definition_index.get(name))
        {
            for member in &definition.members {
                let member_id = feature_id_from_qualified_name(&member.qualified_name);
                if feature_specializes(
                    &member_id,
                    base_feature,
                    context,
                    lookup,
                    &mut BTreeSet::new(),
                ) {
                    candidates.insert(member_id);
                }
            }
            pending.extend(definition.implicit_specializations.iter().cloned());
            pending.extend(definition.specializes.iter().filter_map(|name| {
                lookup.resolve(name, &definition.qualified_name, "", &mut BTreeSet::new())
            }));
        }
    }
    candidates
        .iter()
        .filter(|id| {
            // A redefinition replaces the inherited value, including a default.
            !candidates.iter().any(|other| {
                other != *id
                    && feature_specializes(other, id, context, lookup, &mut BTreeSet::new())
            })
        })
        .filter_map(|id| {
            let value = context.local_usage_map.get(id.strip_prefix("feature.")?)?;
            let Expr::Operation {
                operator, operands, ..
            } = value.expression.as_ref()?
            else {
                return None;
            };
            if operator != "meta" {
                return None;
            }
            // `element meta Metaclass` denotes the element on the left, not the
            // metaclass on the right. General expression type lookup does the latter.
            let name = match operands.first()? {
                Expr::Name(name) | Expr::TypeReference(name) => name,
                _ => return None,
            };
            lookup.resolve(name, &value.owner_qualified_name, "", &mut BTreeSet::new())
        })
        .collect()
}

fn is_feature(id: &str, context: &ResolverContext) -> Result<bool, Diagnostic> {
    if id.starts_with("feature.") {
        return Ok(true);
    }
    context
        .library_indexes
        .kinds
        .get(id)
        .map_or(Ok(false), |kind| typing::kind_conforms(kind, "Feature"))
}

fn sync_usage(usage: &mut CollectedUsage, context: &ResolverContext) {
    if let Some(indexed) = context.local_usage_map.get(&usage.qualified_name) {
        usage.subsets.clone_from(&indexed.subsets);
    }
    for member in &mut usage.members {
        sync_usage(member, context);
    }
}

pub(super) fn augment(
    context: &ResolverContext,
    definitions: &mut [CollectedDefinition],
    usages: &mut [CollectedUsage],
    aliases: &BTreeMap<String, QualifiedName>,
    imports: &ImportAliases,
    mappings: &MappingBundle,
) -> Result<Option<ResolverContext>, Diagnostic> {
    if !context
        .definition_index
        .values()
        .any(|d| annotations(&d.modifiers).next().is_some())
        && !context
            .local_usage_map
            .values()
            .any(|u| annotations(&u.modifiers).next().is_some())
    {
        return Ok(None);
    }
    let policy = policy()?;
    let mut augmented = context.clone();
    // Prefer the actual module over a same-named declaration in a support file.
    for definition in definitions.iter() {
        augmented
            .definition_index
            .insert(definition.qualified_name.clone(), definition.clone());
    }
    augmented
        .local_usage_map
        .extend(build_local_usage_map(definitions, usages));
    loop {
        let lookup = FeatureLookup {
            stdlib_ids: &augmented.library_indexes.ids,
            stdlib_feature_index: &augmented.library_indexes.feature_index,
            stdlib_aliases: &augmented.library_indexes.aliases,
            local_definitions: &augmented.local_definitions,
            local_aliases: aliases,
            import_aliases: imports,
            definition_index: &augmented.definition_index,
            local_feature_index: &augmented.local_feature_index,
            local_usage_map: &augmented.local_usage_map,
        };
        let mut definition_parents = BTreeMap::<String, Vec<String>>::new();
        let mut usage_parents = BTreeMap::<String, Vec<String>>::new();
        for definition in augmented.definition_index.values() {
            // Most support declarations have no annotation. Avoid walking the
            // metaclass lattice for those declarations on every target compile.
            if annotations(&definition.modifiers).next().is_none() {
                continue;
            }
            let kind = mappings.metaclass_for(&definition.construct)?;
            if !typing::kind_conforms(kind, "Classifier")? {
                continue;
            }
            for annotation in annotations(&definition.modifiers) {
                for base in bases(
                    annotation,
                    &definition.qualified_name,
                    &definition.span,
                    &augmented,
                    &lookup,
                    &policy.base_feature,
                ) {
                    let parents = if is_feature(&base, &augmented)? {
                        typing::feature_types(
                            &base,
                            &BTreeMap::new(),
                            &augmented,
                            mappings,
                            &lookup,
                        )
                    } else {
                        vec![base]
                    };
                    for parent in parents {
                        let classifier = if let Some(local) = parent
                            .strip_prefix("type.")
                            .and_then(|name| augmented.definition_index.get(name))
                        {
                            typing::kind_conforms(
                                mappings.metaclass_for(&local.construct)?,
                                "Classifier",
                            )?
                        } else {
                            augmented
                                .library_indexes
                                .kinds
                                .get(&parent)
                                .map_or(Ok(false), |kind| {
                                    typing::kind_conforms(kind, "Classifier")
                                })?
                        };
                        if classifier && !definition.implicit_specializations.contains(&parent) {
                            definition_parents
                                .entry(definition.qualified_name.clone())
                                .or_default()
                                .push(parent);
                        }
                    }
                }
            }
        }
        for usage in augmented.local_usage_map.values() {
            if annotations(&usage.modifiers).next().is_none() {
                continue;
            }
            if typing::kind_conforms(mappings.metaclass_for(&usage.construct)?, "MetadataFeature")?
            {
                continue;
            }
            for annotation in annotations(&usage.modifiers) {
                for base in bases(
                    annotation,
                    &usage.owner_qualified_name,
                    &usage.span,
                    &augmented,
                    &lookup,
                    &policy.base_feature,
                ) {
                    if is_feature(&base, &augmented)?
                        && !usage
                            .subsets
                            .iter()
                            .any(|name| name.segments == qualified(&base, &usage.span).segments)
                    {
                        usage_parents
                            .entry(usage.qualified_name.clone())
                            .or_default()
                            .push(base);
                    }
                }
            }
        }
        if definition_parents.is_empty() && usage_parents.is_empty() {
            break;
        }
        for (name, parents) in definition_parents {
            if let Some(definition) = augmented.definition_index.get_mut(&name) {
                for parent in parents {
                    if !definition.implicit_specializations.contains(&parent) {
                        definition.implicit_specializations.push(parent);
                    }
                }
            }
        }
        for (name, parents) in usage_parents {
            if let Some(usage) = augmented.local_usage_map.get_mut(&name) {
                for parent in parents {
                    let name = qualified(&parent, &usage.span);
                    if !usage
                        .subsets
                        .iter()
                        .any(|other| other.segments == name.segments)
                    {
                        usage.subsets.push(name);
                    }
                }
            }
        }
    }
    for definition in definitions {
        if let Some(indexed) = augmented.definition_index.get(&definition.qualified_name) {
            definition
                .implicit_specializations
                .clone_from(&indexed.implicit_specializations);
        }
        for member in &mut definition.members {
            sync_usage(member, &augmented);
        }
    }
    for usage in usages {
        sync_usage(usage, &augmented);
    }
    Ok(Some(augmented))
}
