//! Index construction and cached library lookup for lowering resolution.

use std::collections::{BTreeMap, BTreeSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

use mercurio_kir::KirDocument;
use mercurio_language_contracts::ast::QualifiedName;
use mercurio_language_contracts::diagnostics::Diagnostic;
use serde_json::Value;

use crate::lowering::collect::{CollectedAlias, CollectedDefinition, CollectedUsage};
use crate::lowering::emit::MappingBundle;

#[derive(Debug, Clone)]
pub(crate) struct LibraryIndexes {
    pub(crate) ids: Vec<String>,
    pub(crate) feature_index: BTreeMap<String, BTreeMap<String, String>>,
    pub(crate) aliases: BTreeMap<String, String>,
}

pub(crate) fn build_local_definition_map(
    definitions: &[CollectedDefinition],
    mappings: &MappingBundle,
) -> Result<BTreeMap<String, String>, Diagnostic> {
    let mut simple_names = BTreeMap::<String, String>::new();
    let mut duplicates = BTreeSet::new();
    let mut resolved = BTreeMap::new();

    for definition in definitions {
        let id = format!("type.{}", definition.qualified_name);
        resolved.insert(definition.qualified_name.clone(), id.clone());
        if mappings
            .generated_companion_construct_for_definition(&definition.construct)
            .is_some()
        {
            let conjugated_name = format!("~{}", definition.declared_name);
            let conjugated_id = format!("type.{}.{}", definition.qualified_name, conjugated_name);
            resolved.insert(conjugated_name.clone(), conjugated_id.clone());
            if let Some((owner, _)) = definition.qualified_name.rsplit_once('.') {
                resolved.insert(format!("{owner}.{conjugated_name}"), conjugated_id);
            }
        }
        match simple_names.entry(definition.declared_name.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(id);
            }
            std::collections::btree_map::Entry::Occupied(_) => {
                duplicates.insert(definition.declared_name.clone());
            }
        }
    }

    for duplicate in duplicates {
        simple_names.remove(&duplicate);
    }

    for (simple, id) in simple_names {
        resolved.entry(simple).or_insert(id);
    }

    Ok(resolved)
}

pub(crate) fn build_local_alias_map(aliases: &[CollectedAlias]) -> BTreeMap<String, QualifiedName> {
    let mut simple_aliases = BTreeMap::<String, QualifiedName>::new();
    let mut duplicates = BTreeSet::new();
    let mut resolved = BTreeMap::new();

    for alias in aliases {
        resolved.insert(alias.qualified_name.clone(), alias.target.clone());
        match simple_aliases.entry(alias.declared_name.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(alias.target.clone());
            }
            std::collections::btree_map::Entry::Occupied(_) => {
                duplicates.insert(alias.declared_name.clone());
            }
        }
    }

    for duplicate in duplicates {
        simple_aliases.remove(&duplicate);
    }

    for (simple, target) in simple_aliases {
        resolved.entry(simple).or_insert(target);
    }

    resolved
}

pub(crate) fn build_stdlib_feature_index(
    stdlib: &KirDocument,
    mappings: &MappingBundle,
) -> BTreeMap<String, BTreeMap<String, String>> {
    let direct_features = stdlib.elements.iter().fold(
        BTreeMap::<String, BTreeMap<String, String>>::new(),
        |mut acc, element| {
            if let Some((owner, feature_name)) = element.id.rsplit_once("::") {
                acc.entry(owner.to_string())
                    .or_default()
                    .entry(feature_name.to_string())
                    .or_insert_with(|| element.id.clone());
            }
            acc
        },
    );
    let feature_types = stdlib
        .elements
        .iter()
        .filter_map(|element| {
            let types = element
                .properties
                .get("type")
                .and_then(Value::as_array)?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            (!types.is_empty()).then(|| (element.id.clone(), types))
        })
        .collect::<BTreeMap<_, _>>();
    let specializations = stdlib
        .elements
        .iter()
        .map(|element| {
            let mut parents = element
                .properties
                .get("specializes")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            for semantic_parent in mappings.semantic_specializations_for_definition(&element.kind) {
                if semantic_parent != element.id
                    && !parents.iter().any(|parent| parent == &semantic_parent)
                {
                    parents.push(semantic_parent);
                }
            }
            (element.id.clone(), parents)
        })
        .collect::<BTreeMap<_, _>>();

    let mut resolved = BTreeMap::new();
    let mut resolving = BTreeSet::new();
    for owner in specializations.keys() {
        collect_stdlib_owner_features(
            owner,
            &direct_features,
            &specializations,
            &mut resolved,
            &mut resolving,
        );
    }
    for (feature_id, types) in feature_types {
        let mut features = resolved.get(&feature_id).cloned().unwrap_or_default();
        for ty in types {
            for (name, target) in collect_stdlib_owner_features(
                &ty,
                &direct_features,
                &specializations,
                &mut resolved,
                &mut resolving,
            ) {
                features.entry(name).or_insert(target);
            }
        }
        if !features.is_empty() {
            resolved.insert(feature_id, features);
        }
    }
    resolved
}

pub(crate) fn cached_library_indexes(
    library_context: &KirDocument,
    mappings: &MappingBundle,
) -> Arc<LibraryIndexes> {
    static CACHE: OnceLock<Mutex<BTreeMap<(usize, usize, u64), Arc<LibraryIndexes>>>> =
        OnceLock::new();

    let key = library_context_instance_key(library_context);
    let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    {
        let guard = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(indexes) = guard.get(&key) {
            return indexes.clone();
        }
    }

    let indexes = Arc::new(LibraryIndexes {
        ids: library_context
            .elements
            .iter()
            .map(|element| element.id.clone())
            .collect::<Vec<_>>(),
        feature_index: build_stdlib_feature_index(library_context, mappings),
        aliases: build_stdlib_alias_map(library_context, mappings),
    });

    let mut guard = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.entry(key).or_insert_with(|| indexes.clone()).clone()
}

fn library_context_instance_key(library_context: &KirDocument) -> (usize, usize, u64) {
    let mut hasher = DefaultHasher::new();
    for element in &library_context.elements {
        element.id.hash(&mut hasher);
    }
    (
        library_context.elements.as_ptr() as usize,
        library_context.elements.len(),
        hasher.finish(),
    )
}

fn collect_stdlib_owner_features(
    owner: &str,
    direct_features: &BTreeMap<String, BTreeMap<String, String>>,
    specializations: &BTreeMap<String, Vec<String>>,
    resolved: &mut BTreeMap<String, BTreeMap<String, String>>,
    resolving: &mut BTreeSet<String>,
) -> BTreeMap<String, String> {
    if let Some(existing) = resolved.get(owner) {
        return existing.clone();
    }
    if !resolving.insert(owner.to_string()) {
        return direct_features.get(owner).cloned().unwrap_or_default();
    }

    let mut features = BTreeMap::new();
    if let Some(parents) = specializations.get(owner) {
        for parent in parents {
            for (name, feature_id) in collect_stdlib_owner_features(
                parent,
                direct_features,
                specializations,
                resolved,
                resolving,
            ) {
                features.entry(name).or_insert(feature_id);
            }
        }
    }
    if let Some(local) = direct_features.get(owner) {
        for (name, feature_id) in local {
            features.insert(name.clone(), feature_id.clone());
        }
    }

    resolving.remove(owner);
    resolved.insert(owner.to_string(), features.clone());
    features
}

pub(crate) fn build_local_feature_index(
    definitions: &[CollectedDefinition],
    usages: &[CollectedUsage],
) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut index = BTreeMap::new();
    for definition in definitions {
        collect_feature_scope(&definition.members, &mut index);
    }
    collect_feature_scope(usages, &mut index);
    index
}

pub(crate) fn build_local_usage_map(
    definitions: &[CollectedDefinition],
    usages: &[CollectedUsage],
) -> BTreeMap<String, CollectedUsage> {
    let mut map = BTreeMap::new();
    for definition in definitions {
        collect_usage_map(&definition.members, &mut map);
    }
    collect_usage_map(usages, &mut map);
    map
}

fn collect_feature_scope(
    usages: &[CollectedUsage],
    index: &mut BTreeMap<String, BTreeMap<String, String>>,
) {
    for usage in usages {
        index
            .entry(usage.owner_qualified_name.clone())
            .or_default()
            .insert(usage.declared_name.clone(), usage.qualified_name.clone());
        collect_feature_scope(&usage.members, index);
    }
}

fn collect_usage_map(usages: &[CollectedUsage], map: &mut BTreeMap<String, CollectedUsage>) {
    for usage in usages {
        map.insert(usage.qualified_name.clone(), usage.clone());
        collect_usage_map(&usage.members, map);
    }
}

pub(crate) fn build_stdlib_alias_map(
    stdlib: &KirDocument,
    mappings: &MappingBundle,
) -> BTreeMap<String, String> {
    let mut aliases = mappings
        .stdlib_aliases()
        .iter()
        .map(|(alias, target)| (alias.clone(), target.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut bare_short_name_targets = BTreeMap::<String, String>::new();
    let mut duplicate_bare_short_names = BTreeSet::<String>::new();

    for element in &stdlib.elements {
        let Some((namespace, _)) = element.id.rsplit_once("::") else {
            continue;
        };
        let Some(metadata) = element
            .properties
            .get("metadata")
            .and_then(Value::as_object)
        else {
            continue;
        };
        let Some(short_name) = metadata.get("declared_short_name").and_then(Value::as_str) else {
            continue;
        };
        aliases
            .entry(format!("{namespace}::{short_name}"))
            .or_insert_with(|| element.id.clone());
        match bare_short_name_targets.entry(short_name.to_string()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(element.id.clone());
            }
            std::collections::btree_map::Entry::Occupied(existing)
                if existing.get() != &element.id =>
            {
                duplicate_bare_short_names.insert(short_name.to_string());
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
    }

    for duplicate in duplicate_bare_short_names {
        bare_short_name_targets.remove(&duplicate);
    }

    for (short_name, target) in bare_short_name_targets {
        aliases.entry(short_name).or_insert(target);
    }

    for (alias, target) in mappings.compatibility_library_aliases() {
        add_compat_stdlib_alias(&mut aliases, stdlib, alias, target);
    }

    aliases
}

fn add_compat_stdlib_alias(
    aliases: &mut BTreeMap<String, String>,
    stdlib: &KirDocument,
    alias: &str,
    target: &str,
) {
    if stdlib.elements.iter().any(|element| element.id == target) {
        aliases
            .entry(alias.to_string())
            .or_insert(target.to_string());
    }
}
