//! Index construction and cached library lookup for lowering resolution.

use std::collections::{BTreeMap, BTreeSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

use mercurio_foundation::kir::KirDocument;
use mercurio_foundation::language_contracts::ast::QualifiedName;
use mercurio_foundation::language_contracts::diagnostics::Diagnostic;
use serde_json::Value;

use crate::language_frontend::lowering::collect::{
    CollectedAlias, CollectedDefinition, CollectedUsage,
};
use crate::language_frontend::lowering::emit::MappingBundle;

#[derive(Debug, Clone)]
pub(crate) struct LibraryIndexes {
    pub(crate) ids: Vec<String>,
    pub(crate) kinds: BTreeMap<String, String>,
    pub(crate) specializations: BTreeMap<String, Vec<String>>,
    pub(crate) feature_types: BTreeMap<String, Vec<String>>,
    pub(crate) feature_index: BTreeMap<String, BTreeMap<String, String>>,
    pub(crate) aliases: BTreeMap<String, String>,
    pub(crate) membership_visibility: Arc<BTreeMap<String, String>>,
    pub(crate) namespace_scope: Arc<LibraryNamespaceScope>,
}

/// Handwritten namespace traversal; Ecore memberships supply names, identities,
/// endpoint types and access. Root anchoring remains a library-format adapter.
#[derive(Debug, Default)]
pub(crate) struct LibraryNamespaceScope {
    members: LibraryMemberBindings,
    controlled: BTreeSet<String>,
    namespaces: BTreeSet<String>,
}

impl LibraryNamespaceScope {
    /// None means no explicit membership scope owns this lookup. Some(None)
    /// means a controlled lookup failed and compatibility spelling must not rescue it.
    pub(crate) fn resolve(
        &self,
        segments: &[String],
        aliases: &BTreeMap<String, String>,
    ) -> Option<Option<String>> {
        for count in 1..segments.len() {
            let prefix = segments[..count].join("::");
            let root = aliases.get(&prefix).unwrap_or(&prefix);
            if !self.controlled.contains(root) { continue; }
            let mut current = root.as_str();
            for name in &segments[count..] {
                if !self.namespaces.contains(current) { return Some(None); }
                let Some(Some(target)) = self.members.get(current).and_then(|members| members.get(name)) else {
                    return Some(None);
                };
                current = target;
            }
            return Some(Some(current.to_owned()));
        }
        None
    }
}

fn library_explicit_specializations(stdlib: &KirDocument) -> Result<BTreeMap<String, Vec<String>>, Diagnostic> {
    use super::relationship_declarations::metaclass_conforms;
    let mut result = BTreeMap::new();
    for element in stdlib.elements.iter().filter(|element| !is_library_synthetic(element)) {
        // Legacy materialized projection, distinct from canonical Ecore edges.
        let snapshot = element.properties.get("specializes");
        let mut parents = snapshot.and_then(Value::as_array).map(|values|
            values.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>()
        ).unwrap_or_default();
        if metaclass_conforms(&element.kind, "Type") && element.properties.contains_key("owned_relationship") {
            for parent in super::emit::stored_explicit_general_types(&stdlib.elements, element)? {
                if snapshot.is_some() && !parents.contains(&parent) {
                    return Err(Diagnostic::semantic(format!("Specialization snapshot disagrees with stored ownership for {}", element.id), None));
                }
                if !parents.contains(&parent) { parents.push(parent); }
            }
        }
        result.insert(element.id.clone(), parents);
    }
    Ok(result)
}

/// Shared bounded inheritance composition. Iteration reaches a fixed point
/// even for legal cyclic generalization; different inherited identities remain
/// ambiguous. Direct bindings take precedence. Full distinguishability and
/// redefinition filtering remain separate semantic dependencies.
fn compose_library_members(
    direct: &LibraryMemberBindings,
    parents: &BTreeMap<String, Vec<String>>,
) -> LibraryMemberBindings {
    let mut current = direct.clone();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (owner, generals) in parents {
        for general in generals { dependents.entry(general).or_default().push(owner); }
    }
    let mut pending = parents.keys().map(String::as_str).collect::<std::collections::VecDeque<_>>();
    let mut queued = parents.keys().map(String::as_str).collect::<BTreeSet<_>>();
    while let Some(owner) = pending.pop_front() {
        queued.remove(owner);
        let mut members = direct.get(owner).cloned().unwrap_or_default();
        for general in parents.get(owner).into_iter().flatten() {
            for (name, target) in current.get(general).into_iter().flatten() {
                if direct.get(owner).is_some_and(|owned| owned.contains_key(name)) { continue; }
                let binding = members.entry(name.clone()).or_insert_with(|| target.clone());
                if binding != target { *binding = None; }
            }
        }
        if current.get(owner) != Some(&members) {
            current.insert(owner.to_owned(), members);
            for child in dependents.get(owner).into_iter().flatten() {
                if queued.insert(child) { pending.push_back(child); }
            }
        }
    }
    current
}

fn build_library_namespace_scope(stdlib: &KirDocument) -> Result<LibraryNamespaceScope, Diagnostic> {
    use super::relationship_declarations::metaclass_conforms;
    let elements = stdlib.elements.iter().map(|element| (element.id.as_str(), element))
        .collect::<BTreeMap<_, _>>();
    let mut controlled = BTreeSet::new();
    for membership in stdlib.elements.iter().filter(|element| metaclass_conforms(&element.kind, "Membership")) {
        if let Some(owner) = library_membership_reference(stdlib, &elements, membership, "membership_owning_namespace")? {
            controlled.insert(owner.to_owned());
        }
    }
    for namespace in &stdlib.elements {
        if metaclass_conforms(&namespace.kind, "Package")
            && namespace.properties.get("owned_relationship").and_then(Value::as_array)
                .is_some_and(|ids| ids.iter().filter_map(Value::as_str)
                    .filter_map(|id| elements.get(id)).any(|e| metaclass_conforms(&e.kind, "Import"))) {
            controlled.insert(namespace.id.clone());
        }
    }
    let parents = library_explicit_specializations(stdlib)?;
    controlled.extend(parents.iter().filter(|(_, parents)| !parents.is_empty()).map(|(id, _)| id.clone()));
    Ok(LibraryNamespaceScope {
        members: compose_library_members(&library_direct_members(stdlib)?, &parents),
        controlled,
        namespaces: stdlib.elements.iter().filter(|element| metaclass_conforms(&element.kind, "Namespace"))
            .map(|element| element.id.clone()).collect(),
    })
}

fn is_library_membership(element: &mercurio_foundation::kir::KirElement) -> bool {
    element.id.starts_with("LibraryMembership::") || element.kind.ends_with("Membership")
}

fn is_library_synthetic(element: &mercurio_foundation::kir::KirElement) -> bool {
    is_library_membership(element) || element.id.starts_with("LibraryAnonymous::")
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
    let mut resolved = BTreeMap::new();

    for alias in aliases {
        let mut names = Vec::new();
        if !alias.declared_name.is_empty() { names.push(alias.declared_name.as_str()); }
        if let Some(declaration) = &alias.declaration {
            names.extend(declaration.modifiers.iter().filter_map(|m| m.strip_prefix("short_name=")));
        }
        for name in names {
            let qualified = if alias.owner_qualified_name == "root" { name.to_string() } else { format!("{}.{}", alias.owner_qualified_name, name) };
            resolved.insert(qualified, alias.target.clone());

        }
    }

    resolved
}

/// Prefer explicit Ecore membership identities over compatibility ID spelling.
/// This is an external/public member index; full import and conflict algorithms
/// remain separate scope services.
// None is an ambiguous binding, retained until inheritance/type composition
// finishes so a conflict cannot be rescued by an inherited or typed member.
type LibraryMemberBindings = BTreeMap<String, BTreeMap<String, Option<String>>>;

/// Read a projected or canonical membership reference using the most-specific
/// imported Ecore contract. Singleton lists are a library projection adapter,
/// not the canonical Ecore scalar shape. Redefined aliases must agree.
fn library_membership_reference<'a>(
    graph: &'a KirDocument,
    elements: &BTreeMap<&str, &'a mercurio_foundation::kir::KirElement>,
    membership: &'a mercurio_foundation::kir::KirElement,
    requested: &str,
) -> Result<Option<&'a str>, Diagnostic> {
    use super::ecore_model;
    let error = |message: String| Diagnostic::semantic(
        format!("Library membership {}: {message}", membership.id), None);
    let contract = ecore_model::redefined_feature(&membership.kind, requested).map_err(error)?;
    let mut selected = None;
    for (field, value) in &membership.properties {
        if ecore_model::feature(&membership.kind, field).is_none() { continue; }
        let candidate = ecore_model::redefined_feature(&membership.kind, field).map_err(error)?;
        if candidate.field != contract.field { continue; }
        let scalar = match value {
            Value::Array(values) if values.len() == 1 => &values[0],
            value => value,
        };
        ecore_model::validate_value(&membership.kind, contract.field, scalar).map_err(error)?;
        let Some(id) = scalar.as_str() else { continue; };
        if selected.is_some_and(|previous| previous != id) {
            return Err(error(format!("conflicting redefined reference {requested}")));
        }
        let endpoint = elements.get(id)
            .ok_or_else(|| error(format!("unresolved {requested} target {id}")))?;
        ecore_model::validate_reference_endpoint(&membership.kind, contract.field, &endpoint.kind)
            .map_err(error)?;
        selected = Some(id);
    }
    use super::emit::StoredMembershipEndpoint;
    match super::emit::stored_membership_endpoint(&graph.elements, membership, requested)? {
        StoredMembershipEndpoint::Resolved(endpoint) => {
            ecore_model::validate_reference_endpoint(&membership.kind, contract.field, &endpoint.kind)
                .map_err(error)?;
            if selected.is_some_and(|snapshot| snapshot != endpoint.id) {
                return Err(error(format!("stored ownership disagrees with {requested} snapshot")));
            }
            selected = Some(endpoint.id.as_str());
        }
        StoredMembershipEndpoint::Absent if selected.is_some() => {
            return Err(error(format!("absent stored ownership disagrees with {requested} snapshot")));
        }
        StoredMembershipEndpoint::Absent | StoredMembershipEndpoint::Unavailable => {}
    }
    Ok(selected)
}

/// Reconcile effective-name snapshots only when canonical ownership is present.
/// Legacy materialized libraries retain their explicit snapshot adapter.
fn library_membership_names(
    graph: &KirDocument,
    membership: &mercurio_foundation::kir::KirElement,
    target: &mercurio_foundation::kir::KirElement,
) -> Result<[Option<String>; 2], Diagnostic> {
    use super::{ecore_model, relationship_declarations::metaclass_conforms};
    let error = |message: String| Diagnostic::semantic(
        format!("Library membership {}: {message}", membership.id), None);
    let mut snapshots: [Option<Option<String>>; 2] = [None, None];
    for (index, requested) in ["member_name", "member_short_name"].iter().enumerate() {
        let contract = ecore_model::redefined_feature(&membership.kind, requested).map_err(error)?;
        for (field, value) in &membership.properties {
            if ecore_model::feature(&membership.kind, field).is_none() { continue; }
            if ecore_model::redefined_feature(&membership.kind, field).map_err(error)?.field != contract.field { continue; }
            ecore_model::validate_value(&membership.kind, contract.field, value).map_err(error)?;
            let name = value.as_str().map(str::to_owned);
            if snapshots[index].as_ref().is_some_and(|previous| previous != &name) {
                return Err(error(format!("conflicting redefined name {requested}")));
            }
            snapshots[index] = Some(name);
        }
    }
    if metaclass_conforms(&membership.kind, "OwningMembership")
        && (membership.properties.contains_key("owned_related_element")
            || snapshots.iter().all(Option::is_none)) {
        let effective = super::emit::generated_effective_names(&graph.elements, target)?;
        for (index, snapshot) in snapshots.iter().enumerate() {
            if snapshot.as_ref().is_some_and(|name| name != &effective[index]) {
                return Err(error("name snapshot disagrees with owned element semantics".into()));
            }
        }
        Ok(effective)
    } else {
        Ok(snapshots.map(Option::flatten))
    }
}

fn library_direct_members(stdlib: &KirDocument) -> Result<LibraryMemberBindings, Diagnostic> {
    use super::relationship_declarations::metaclass_conforms;
    let elements = stdlib.elements.iter().map(|element| (element.id.as_str(), element))
        .collect::<BTreeMap<_, _>>();
    if elements.len() != stdlib.elements.len() {
        return Err(Diagnostic::semantic("Duplicate library model identity", None));
    }
    let mut memberships = Vec::new();
    let mut controlled = BTreeSet::new();
    for membership in stdlib.elements.iter()
        .filter(|element| metaclass_conforms(&element.kind, "Membership")) {
        let owner = library_membership_reference(stdlib, &elements, membership, "membership_owning_namespace")?;
        let target = library_membership_reference(stdlib, &elements, membership, "member_element")?;
        if metaclass_conforms(&membership.kind, "OwningMembership") {
            if let Some(target) = target { controlled.insert(target); }
        }
        // Endpoints derive from reciprocal stored ownership when present.
        // Incomplete external graphs still require a separate resolution service.
        if let (Some(owner), Some(target)) = (owner, target) {
            memberships.push((membership, owner, target));
        }
    }
    let mut direct = LibraryMemberBindings::new();
    for element in &stdlib.elements {
        if is_library_synthetic(element) || controlled.contains(element.id.as_str()) { continue; }
        if element.properties.get("metadata").and_then(|v| v.get("owning_membership_visibility"))
            .and_then(Value::as_str).is_some_and(|access| access != "public") { continue; }
        if let Some((owner, name)) = element.id.rsplit_once("::") {
            direct.entry(owner.to_owned()).or_default().entry(name.to_owned())
                .or_insert_with(|| Some(element.id.clone()));
        }
    }
    let mut explicit = LibraryMemberBindings::new();
    for (membership, owner, target) in memberships {
        if library_membership_visibility(membership)? != "public" { continue; }
        let names = library_membership_names(stdlib, membership, elements[target])?;
        for name in names.into_iter().flatten().filter(|name| !name.is_empty()) {
            let binding = explicit.entry(owner.to_owned()).or_default()
                .entry(name).or_insert_with(|| Some(target.to_owned()));
            if binding.as_deref() != Some(target) { *binding = None; }
        }
    }
    for (owner, bindings) in explicit {
        direct.entry(owner).or_default().extend(bindings);
    }
    // Reuse the graph scope service: retain membership identities through
    // imports and conflict hiding before projecting names to element targets.
    for namespace in &stdlib.elements {
        let Some(memberships) = super::emit::stored_package_import_members(&stdlib.elements, namespace)? else { continue; };
        let mut bindings = BTreeMap::new();
        for membership in memberships {
            let target = library_membership_reference(stdlib, &elements, membership, "member_element")?
                .ok_or_else(|| Diagnostic::semantic("Imported membership requires a resolved member", None))?;
            for name in library_membership_names(stdlib, membership, elements[target])?.into_iter().flatten() {
                let binding = bindings.entry(name).or_insert_with(|| Some(target.to_owned()));
                if binding.as_deref() != Some(target) { *binding = None; }
            }
        }
        direct.insert(namespace.id.clone(), bindings);
    }
    Ok(direct)
}

pub(crate) fn build_stdlib_feature_index(
    stdlib: &KirDocument,
    mappings: &MappingBundle,
) -> Result<BTreeMap<String, BTreeMap<String, String>>, Diagnostic> {
    let direct_features = library_direct_members(stdlib)?;
    let feature_types = stdlib
        .elements
        .iter()
        .filter(|element| !is_library_synthetic(element))
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
    let mut specializations = library_explicit_specializations(stdlib)?;
    for element in stdlib.elements.iter().filter(|element| !is_library_synthetic(element)) {
        let parents = specializations.entry(element.id.clone()).or_default();
        for parent in mappings.semantic_specializations_for_definition(&element.kind) {
            if parent != element.id && !parents.contains(&parent) { parents.push(parent); }
        }
    }
    let mut resolved = compose_library_members(&direct_features, &specializations);
    for (feature_id, types) in feature_types {
        let mut features = resolved.get(&feature_id).cloned().unwrap_or_default();
        for ty in types {
            for (name, target) in resolved.get(&ty).cloned().unwrap_or_default() {
                let binding = features.entry(name.clone()).or_insert_with(|| target.clone());
                if target.is_none() && !direct_features.get(&feature_id).is_some_and(|owned| owned.contains_key(&name)) {
                    *binding = None;
                }
            }
        }
        if !features.is_empty() {
            resolved.insert(feature_id, features);
        }
    }
    Ok(resolved.into_iter().map(|(owner, members)| (owner, members.into_iter()
        .filter_map(|(name, target)| target.map(|target| (name, target))).collect())).collect())
}

pub(crate) fn cached_library_indexes(
    library_context: &KirDocument,
    mappings: &MappingBundle,
) -> Result<Arc<LibraryIndexes>, Diagnostic> {
    static CACHE: OnceLock<Mutex<BTreeMap<(usize, usize, u64), Arc<LibraryIndexes>>>> =
        OnceLock::new();

    let key = library_context_instance_key(library_context, mappings);
    let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    {
        let guard = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(indexes) = guard.get(&key) {
            return Ok(indexes.clone());
        }
    }

    let indexes = Arc::new(LibraryIndexes {
        feature_types: library_context
            .elements
            .iter()
            .filter(|element| !is_library_synthetic(element))
            .filter_map(|element| {
                let value = element.properties.get("type")?;
                let types = match value {
                    Value::String(id) => vec![id.clone()],
                    Value::Array(values) => values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect(),
                    _ => Vec::new(),
                };
                Some((element.id.clone(), types))
            })
            .collect(),
        specializations: library_explicit_specializations(library_context)?,
        kinds: library_context
            .elements
            .iter()
            .filter(|element| !is_library_synthetic(element))
            .map(|e| (e.id.clone(), e.kind.clone()))
            .collect(),
        ids: library_context
            .elements
            .iter()
            .filter(|element| !is_library_synthetic(element))
            .map(|element| element.id.clone())
            .collect::<Vec<_>>(),
        feature_index: build_stdlib_feature_index(library_context, mappings)?,
        namespace_scope: Arc::new(build_library_namespace_scope(library_context)?),
        aliases: build_stdlib_alias_map(library_context, mappings)?,
        membership_visibility: Arc::new(build_library_membership_visibility(library_context)?),
    });

    let mut guard = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Ok(guard.entry(key).or_insert_with(|| indexes.clone()).clone())
}

/// Effective visibility uses the imported enum/default contract. The metadata
/// location is an explicit adapter for older library snapshots.
fn library_membership_visibility(
    membership: &mercurio_foundation::kir::KirElement,
) -> Result<String, Diagnostic> {
    let error = |message| Diagnostic::semantic(message, None);
    let value = if let Some(value) = membership.properties.get("visibility")
        .or_else(|| membership.properties.get("metadata")?.get("visibility")) {
        super::ecore_model::validate_value(&membership.kind, "visibility", value).map_err(error)?;
        value.clone()
    } else {
        super::ecore_defaults::read_attribute(membership, "visibility").map_err(error)?
    };
    value.as_str().map(str::to_owned)
        .ok_or_else(|| error("Membership visibility is not an enum literal".into()))
}

pub(crate) fn build_library_membership_visibility(
    stdlib: &KirDocument,
) -> Result<BTreeMap<String, String>, Diagnostic> {
    use super::relationship_declarations::metaclass_conforms;
    let elements = stdlib.elements.iter().map(|element| (element.id.as_str(), element))
        .collect::<BTreeMap<_, _>>();
    let mut ownership = BTreeMap::new();
    for membership in stdlib.elements.iter()
        .filter(|element| metaclass_conforms(&element.kind, "OwningMembership")) {
        let owner = library_membership_reference(stdlib, &elements, membership, "membership_owning_namespace")?;
        let target = library_membership_reference(stdlib, &elements, membership, "member_element")?;
        if let (Some(_owner), Some(target)) = (owner, target) {
            let value = library_membership_visibility(membership)?;
            if ownership.insert(target, value).is_some() {
                return Err(Diagnostic::semantic(format!("Multiple owning memberships for {target}"), None));
            }
        }
    }
    let mut visibility = BTreeMap::new();
    for element in &stdlib.elements {
        if is_library_synthetic(element) { continue; }
        let Some((owner, name)) = element.id.rsplit_once("::") else { continue; };
        let graph_value = ownership.get(element.id.as_str())
            .map(String::as_str);
        let metadata_value = element.properties.get("metadata")
            .and_then(|metadata| metadata.get("owning_membership_visibility"))
            .and_then(Value::as_str);
        if let Some(value) = graph_value.or(metadata_value) {
            visibility.insert(format!("{owner}.{name}"), value.to_string());
        }
    }
    Ok(visibility)
}

fn library_context_instance_key(library_context: &KirDocument, mappings: &MappingBundle) -> (usize, usize, u64) {
    // IDs and allocation addresses alone are insufficient: callers may mutate a
    // library in place, reuse an allocation, or compile both source languages.
    // Hash every input consumed by the index builders, including mapping facts.
    let mut hasher = DefaultHasher::new();
    let mut kinds = BTreeSet::new();
    for element in &library_context.elements {
        element.id.hash(&mut hasher);
        element.kind.hash(&mut hasher);
        // Name delegates traverse ownership/redefinition/conjugation graphs;
        // hash all properties so indirect semantic edits invalidate the cache.
        element.properties.hash(&mut hasher);
        kinds.insert(element.kind.as_str());
    }
    for kind in kinds {
        kind.hash(&mut hasher);
        mappings.semantic_specializations_for_definition(kind).hash(&mut hasher);
    }
    mappings.stdlib_aliases().iter().collect::<BTreeMap<_, _>>().hash(&mut hasher);
    mappings.compatibility_library_aliases().hash(&mut hasher);
    (
        library_context.elements.as_ptr() as usize,
        library_context.elements.len(),
        hasher.finish(),
    )
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
        for short_name in usage
            .modifiers
            .iter()
            .filter_map(|modifier| modifier.strip_prefix("short_name="))
        {
            index
                .entry(usage.owner_qualified_name.clone())
                .or_default()
                .insert(short_name.to_string(), usage.qualified_name.clone());
        }
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
) -> Result<BTreeMap<String, String>, Diagnostic> {
    let mut aliases = mappings
        .stdlib_aliases()
        .iter()
        .map(|(alias, target)| (alias.clone(), target.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut bare_short_name_targets = BTreeMap::<String, String>::new();
    let mut duplicate_bare_short_names = BTreeSet::<String>::new();
    let membership_visibility = build_library_membership_visibility(stdlib)?;

    for element in &stdlib.elements {
        if is_library_synthetic(element) { continue; }
        if let Some(public) = element
            .properties
            .get("metadata")
            .and_then(|metadata| metadata.get("public_memberships"))
            .and_then(Value::as_object)
        {
            for (name, target) in public {
                if let Some(target) = target.as_str() {
                    aliases.insert(format!("{}::{name}", element.id), target.to_string());
                }
            }
        }
        let Some((namespace, member_name)) = element.id.rsplit_once("::") else {
            continue;
        };
        let Some(metadata) = element
            .properties
            .get("metadata")
            .and_then(Value::as_object)
        else {
            continue;
        };
        if membership_visibility.get(&format!("{namespace}.{member_name}"))
            .map(String::as_str)
            .is_some_and(|visibility| visibility != "public") {
            continue;
        }
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

    Ok(aliases)
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

#[cfg(test)]
mod cache_tests {
    use super::*;
    use mercurio_foundation::kir::KirElement;
    use serde_json::json;

    #[test]
    fn inheritance_composition_preserves_diamonds_conflicts_overrides_and_cycles() {
        let direct = BTreeMap::from([
            ("Base".into(), BTreeMap::from([("slot".into(), Some("a".into()))])),
            ("Other".into(), BTreeMap::from([("slot".into(), Some("b".into()))])),
            ("Local".into(), BTreeMap::from([("slot".into(), Some("c".into()))])),
        ]);
        let mut parents: BTreeMap<String, Vec<String>> = BTreeMap::from([
            ("Left".into(), vec!["Base".into()]),
            ("Right".into(), vec!["Base".into()]),
            ("Diamond".into(), vec!["Left".into(), "Right".into()]),
            ("Ambiguous".into(), vec!["Left".into(), "Other".into()]),
            ("Local".into(), vec!["Ambiguous".into()]),
            ("CycleA".into(), vec!["CycleB".into()]),
            ("CycleB".into(), vec!["CycleA".into(), "Left".into()]),
        ]);
        let result = compose_library_members(&direct, &parents);
        assert_eq!(result["Diamond"]["slot"].as_deref(), Some("a"));
        assert_eq!(result["Ambiguous"]["slot"], None);
        assert_eq!(result["Local"]["slot"].as_deref(), Some("c"));
        assert_eq!(result["CycleA"]["slot"].as_deref(), Some("a"));
        assert_eq!(result["CycleB"]["slot"].as_deref(), Some("a"));
        for values in parents.values_mut() { values.reverse(); }
        assert_eq!(compose_library_members(&direct, &parents), result);
    }

    #[test]
    fn canonical_name_snapshots_reconcile_each_field_and_explicit_absent_owner() {
        let element = |id: &str, kind: &str, properties| KirElement {
            id: id.into(), kind: kind.into(), layer: 0, properties,
        };
        let mut library = KirDocument { metadata: BTreeMap::new(), elements: vec![
            element("Owner", "Class", BTreeMap::from([("owned_relationship".into(), json!(["m"]))])),
            element("target", "Feature", BTreeMap::from([
                ("owning_relationship".into(), json!("m")),
                ("declared_name".into(), json!("long")),
                ("declared_short_name".into(), json!("short")),
            ])),
            element("m", "FeatureMembership", BTreeMap::from([
                ("owning_related_element".into(), json!("Owner")),
                ("owned_related_element".into(), json!(["target"])),
                ("owned_member_name".into(), json!("long")),
            ])),
        ] };
        let names = library_direct_members(&library).unwrap();
        assert_eq!(names["Owner"]["long"].as_deref(), Some("target"));
        assert_eq!(names["Owner"]["short"].as_deref(), Some("target"));
        for (field, value) in [
            ("owned_member_name", json!("stale")),
            ("member_name", json!("different")),
            ("member_short_name", json!(null)),
            ("member_short_name", json!(42)),
            ("member_short_name", json!(["short"])),
        ] {
            let saved = library.elements[2].properties.clone();
            library.elements[2].properties.insert(field.into(), value);
            assert!(library_direct_members(&library).is_err(), "{field}: invalid snapshot accepted");
            library.elements[2].properties = saved;
        }
        library.elements[2].properties.insert("member_short_name".into(), json!("short"));
        library.elements[1].properties.insert("declared_short_name".into(), json!("new_short"));
        assert!(library_direct_members(&library).is_err());
        library.elements[2].properties.remove("member_short_name");
        assert!(library_direct_members(&library).unwrap()["Owner"].contains_key("new_short"));
        library.elements[2].properties.insert("membership_owning_namespace".into(), json!("Owner"));
        library.elements[2].properties.insert("owning_related_element".into(), json!(null));
        let error = library_direct_members(&library).unwrap_err();
        assert!(format!("{error:?}").contains("absent stored ownership"), "{error:?}");
        library.elements[2].properties.remove("membership_owning_namespace");
        library.elements[0].properties.remove("owned_relationship");
        assert!(!library_direct_members(&library).unwrap().contains_key("Owner"));
    }

    #[test]
    fn stored_membership_ownership_derives_endpoints_and_rejects_stale_snapshots() {
        let element = |id: &str, kind: &str, properties| KirElement {
            id: id.into(), kind: kind.into(), layer: 0, properties,
        };
        let mut library = KirDocument { metadata: BTreeMap::new(), elements: vec![
            element("Owner", "Class", BTreeMap::from([
                ("owned_relationship".into(), json!(["membership"])),
            ])),
            element("opaque-member", "Feature", BTreeMap::from([
                ("declared_name".into(), json!("derived")),
                ("owning_relationship".into(), json!("membership")),
            ])),
            element("membership", "FeatureMembership", BTreeMap::from([
                ("owning_related_element".into(), json!("Owner")),
                ("owned_related_element".into(), json!(["opaque-member"])),
            ])),
            element("other", "Feature", BTreeMap::new()),
        ] };
        assert_eq!(library_direct_members(&library).unwrap()["Owner"]["derived"].as_deref(), Some("opaque-member"));
        library.elements[2].properties.insert("member_element".into(), json!("other"));
        let error = library_direct_members(&library).unwrap_err();
        assert!(format!("{error:?}").contains("disagrees"), "{error:?}");
        library.elements[2].properties.insert("member_element".into(), json!("opaque-member"));
        library.elements[1].properties.remove("owning_relationship");
        assert!(library_direct_members(&library).is_err());
        library.elements[1].properties.insert("owning_relationship".into(), json!("membership"));
        library.elements[0].properties.remove("owned_relationship");
        assert!(library_direct_members(&library).is_err());
        library.elements[0].properties.insert("owned_relationship".into(), json!(["membership"]));
        library.elements[2].properties.insert("owned_related_element".into(), json!([]));
        assert!(library_direct_members(&library).is_err());
        library.elements[2].properties.insert("owned_related_element".into(), json!(["opaque-member"]));
        library.elements[2].properties.insert("visibility".into(), json!("private"));
        assert!(!library_direct_members(&library).unwrap().get("Owner").is_some_and(|names| names.contains_key("derived")));
        library.elements[2].properties.insert("visibility".into(), json!("public"));
        library.elements.reverse();
        assert_eq!(library_direct_members(&library).unwrap()["Owner"]["derived"].as_deref(), Some("opaque-member"));
    }

    #[test]
    fn canonical_membership_visibility_controls_short_aliases_and_cache() {
        let element = |id: &str, kind: &str, properties| KirElement {
            id: id.into(), kind: kind.into(), layer: 0, properties,
        };
        let mut library = KirDocument { metadata: BTreeMap::new(), elements: vec![
            element("Lib", "Class", BTreeMap::new()),
            element("Lib::value", "Feature", BTreeMap::from([
                ("metadata".into(), json!({"declared_short_name":"v", "owning_membership_visibility":"public"})),
            ])),
            element("membership", "FeatureMembership", BTreeMap::from([
                ("owned_member_feature".into(), json!("Lib::value")),
                ("owning_type".into(), json!("Lib")),
                ("member_name".into(), json!("value")),
                ("visibility".into(), json!("private")),
                ("metadata".into(), json!({"visibility":"public"})),
            ])),
        ] };
        let mappings = MappingBundle::load().unwrap();
        for visibility in ["private", "protected", "public"] {
            library.elements[2].properties.insert("visibility".into(), json!(visibility));
            let indexes = cached_library_indexes(&library, mappings).unwrap();
            assert_eq!(indexes.membership_visibility["Lib.value"], visibility);
            assert_eq!(indexes.aliases.contains_key("Lib::v"), visibility == "public");
            assert_eq!(indexes.aliases.contains_key("v"), visibility == "public");
            assert_eq!(indexes.feature_index["Lib"].contains_key("value"), visibility == "public");
        }
        // Identity spelling does not determine ownership or visibility.
        library.elements[0].id = "OtherOwner".into();
        library.elements[2].properties.insert("owning_type".into(), json!("OtherOwner"));
        library.elements[2].properties.insert("visibility".into(), json!("private"));
        let indexes = cached_library_indexes(&library, mappings).unwrap();
        assert_eq!(indexes.membership_visibility["Lib.value"], "private");
        assert!(!indexes.aliases.contains_key("Lib::v"));
        assert!(!indexes.aliases.contains_key("v"));
        // The imported default is used only when neither representation exists.
        library.elements[2].properties.remove("visibility");
        library.elements[2].properties.insert("metadata".into(), json!({"visibility":"private"}));
        assert_eq!(cached_library_indexes(&library, mappings).unwrap().membership_visibility["Lib.value"], "private");
        library.elements[2].properties.remove("metadata");
        assert_eq!(cached_library_indexes(&library, mappings).unwrap().membership_visibility["Lib.value"], "public");
        for invalid in [json!("PUBLIC"), json!(true), json!(null), json!(["public"])] {
            library.elements[2].properties.insert("visibility".into(), invalid);
            assert!(cached_library_indexes(&library, mappings).is_err());
        }
        library.elements[2].properties.insert("visibility".into(), json!("public"));
        let mut duplicate = library.elements[2].clone();
        duplicate.id = "another-owner".into();
        library.elements.push(duplicate);
        assert!(cached_library_indexes(&library, mappings).is_err());
    }

    #[test]
    fn membership_references_use_resolved_ecore_narrowing_and_identity() {
        let element = |id: &str, kind: &str, properties| KirElement {
            id: id.into(), kind: kind.into(), layer: 0, properties,
        };
        for (kind, canonical, valid, invalid) in [
            ("OwningMembership", "owned_member_element", "Class", "UnknownKind"),
            ("FeatureMembership", "owned_member_feature", "MultiplicityRange", "Class"),
            ("ParameterMembership", "owned_member_parameter", "Feature", "Comment"),
            ("VariantMembership", "owned_variant_usage", "PartUsage", "Feature"),
        ] {
            let mut library = KirDocument { metadata: BTreeMap::new(), elements: vec![
                element("Owner", "Class", BTreeMap::new()),
                element("Owner::opaque", valid, BTreeMap::new()),
                element("membership", kind, BTreeMap::from([
                    ("membership_owning_namespace".into(), json!("Owner")),
                    ("member_name".into(), json!("visible")),
                ])),
            ] };
            for field in ["member_element", "owned_member_element", canonical] {
                library.elements[2].properties.insert(field.into(), json!("Owner::opaque"));
                let members = library_direct_members(&library).unwrap();
                assert_eq!(members["Owner"]["visible"].as_deref(), Some("Owner::opaque"));
                assert!(!members["Owner"].contains_key("opaque"));
            }
            library.elements[1].kind = invalid.into();
            let error = library_direct_members(&library).unwrap_err();
            assert!(format!("{error:?}").contains("target mismatch"), "{kind}: {error:?}");
            library.elements[1].kind = valid.into();
            for value in [json!([]), json!(["Owner::opaque", "Owner::opaque"]), json!(23), json!(null), json!("")] {
                library.elements[2].properties.insert(canonical.into(), value);
                assert!(library_direct_members(&library).is_err(), "{kind}: malformed reference accepted");
            }
            library.elements[2].properties.insert(canonical.into(), json!(["Owner::opaque"]));
            assert!(library_direct_members(&library).is_ok());
            library.elements[2].properties.insert("member_element".into(), json!("Owner"));
            assert!(library_direct_members(&library).is_err(), "{kind}: contradictory projections accepted");
            library.elements[2].properties.insert("member_element".into(), json!("missing"));
            assert!(library_direct_members(&library).is_err(), "{kind}: missing target accepted");
            library.elements[2].properties.insert("member_element".into(), json!("Owner::opaque"));
            library.elements[0].kind = "Comment".into();
            assert!(library_direct_members(&library).is_err(), "{kind}: non-namespace owner accepted");
            library.elements[0].kind = "Class".into();
            library.elements.push(library.elements[1].clone());
            assert!(library_direct_members(&library).is_err(), "{kind}: duplicate identity accepted");
        }
    }

    #[test]
    fn explicit_member_conflicts_block_inherited_and_typed_fallbacks() {
        let element = |id: &str, kind: &str, properties| KirElement {
            id: id.into(), kind: kind.into(), layer: 0, properties,
        };
        let membership = |id: &str, target: &str, field: &str| element(id, "Membership", BTreeMap::from([
            ("membership_owning_namespace".into(), json!("Lib::Derived")),
            ("member_element".into(), json!(target)),
            (field.into(), json!("slot")),
            ("visibility".into(), json!("public")),
        ]));
        let mut library = KirDocument { metadata: BTreeMap::new(), elements: vec![
            element("Lib::Base", "Class", BTreeMap::new()),
            element("Lib::Base::slot", "Feature", BTreeMap::new()),
            element("Lib::Derived", "Class", BTreeMap::from([("specializes".into(), json!(["Lib::Base"]))])),
            element("Lib::typed", "Feature", BTreeMap::from([("type".into(), json!(["Lib::Derived"]))])),
            element("Lib::a", "Feature", BTreeMap::new()),
            element("Lib::b", "Feature", BTreeMap::new()),
            membership("first", "Lib::a", "member_name"),
            membership("second", "Lib::b", "member_short_name"),
        ] };
        let mappings = MappingBundle::load().unwrap();
        for _ in 0..2 {
            let indexes = cached_library_indexes(&library, mappings).unwrap();
            assert!(!indexes.feature_index["Lib::Derived"].contains_key("slot"));
            assert!(!indexes.feature_index["Lib::typed"].contains_key("slot"));
            assert_eq!(indexes.feature_index["Lib::Base"]["slot"], "Lib::Base::slot");
            library.elements.reverse();
        }
        // Repeated names for one target retain its identity; changing an explicit
        // target also invalidates the shared cache without changing element IDs.
        library.elements.iter_mut().find(|element| element.id == "second").unwrap()
            .properties.insert("member_element".into(), json!("Lib::a"));
        let indexes = cached_library_indexes(&library, mappings).unwrap();
        assert_eq!(indexes.feature_index["Lib::Derived"]["slot"], "Lib::a");
        assert_eq!(indexes.feature_index["Lib::typed"]["slot"], "Lib::a");
    }

    #[test]
    fn concrete_library_membership_controls_visibility_without_becoming_a_feature() {
        let mut library = KirDocument { metadata: BTreeMap::new(), elements: vec![
            KirElement { id: "LibraryMembership::sample:://@members.0".into(),
                kind: "FeatureValue".into(), layer: 0,
                properties: BTreeMap::from([
                    ("metadata".into(), json!({"visibility":"private"})),
                    ("member_element".into(), json!(["Sample::hidden"])),
                    ("membership_owning_namespace".into(), json!(["Sample"])),
                ]) },
            KirElement { id: "Sample::hidden".into(), kind: "Expression".into(), layer: 0,
                properties: BTreeMap::from([
                    ("metadata".into(), json!({"owning_membership_visibility":"public", "declared_short_name":"h"})),
                    ("owning_membership".into(), json!(["LibraryMembership::sample:://@members.0"])),
                ]) },
            KirElement { id: "LibraryAnonymous::sample:://@members.1".into(),
                kind: "Expression".into(), layer: 0,
                properties: BTreeMap::from([("metadata".into(), json!({"declared_short_name":"a"}))]) },
            KirElement { id: "Sample".into(), kind: "Namespace".into(), layer: 0, properties: BTreeMap::new() },
        ]};
        let indexes = cached_library_indexes(&library, MappingBundle::load().unwrap()).unwrap();
        assert_eq!(indexes.membership_visibility["Sample.hidden"], "private");
        assert!(!indexes.ids.iter().any(|id| id.starts_with("LibraryMembership::")));
        assert!(!indexes.ids.iter().any(|id| id.starts_with("LibraryAnonymous::")));
        assert!(!indexes.feature_index.contains_key("LibraryMembership::sample"));
        assert!(!indexes.feature_index.contains_key("LibraryAnonymous::sample"));
        assert!(!indexes.aliases.contains_key("Sample::h"));
        assert!(!indexes.aliases.contains_key("LibraryAnonymous::sample::a"));
        library.elements[0].properties.insert("member_element".into(),
            json!(["LibraryAnonymous::sample:://@members.1"]));
        let changed = cached_library_indexes(&library, MappingBundle::load().unwrap()).unwrap();
        assert!(!Arc::ptr_eq(&indexes, &changed));
        assert_eq!(changed.membership_visibility["Sample.hidden"], "public");
    }

    #[test]
    fn cache_distinguishes_language_mappings_for_the_same_library() {
        let library = crate::load_sysml_baseline().unwrap();
        for language in [crate::SourceLanguage::Kerml, crate::SourceLanguage::Sysml, crate::SourceLanguage::Kerml] {
            let mappings = MappingBundle::load_for_language(language).unwrap();
            let indexes = cached_library_indexes(&library, mappings).unwrap();
            assert_eq!(indexes.feature_index, build_stdlib_feature_index(&library, mappings).unwrap());
            assert_eq!(indexes.aliases, build_stdlib_alias_map(&library, mappings).unwrap());
        }
    }

    #[test]
    fn cache_invalidates_semantic_edits_without_changing_element_ids_or_allocation() {
        let mappings = MappingBundle::load().unwrap();
        let mut library = KirDocument { metadata: BTreeMap::new(), elements: vec![
            KirElement {id:"CacheTest::A".into(), kind:"KerML::Classifier".into(), layer:0, properties:BTreeMap::new()},
            KirElement {id:"CacheTest::B".into(), kind:"KerML::Classifier".into(), layer:0, properties:BTreeMap::new()},
            KirElement {id:"CacheTest::C".into(), kind:"KerML::Classifier".into(), layer:0, properties:BTreeMap::new()},
            KirElement {id:"CacheTest::B::value".into(), kind:"KerML::Feature".into(), layer:0, properties:BTreeMap::new()},
        ]};
        let before = cached_library_indexes(&library, mappings).unwrap();
        let allocation = library.elements.as_ptr();
        library.elements[0].properties.insert("specializes".into(), json!(["CacheTest::C", "CacheTest::B", "CacheTest::B"]));
        library.elements[3].properties.insert("type".into(), json!(["CacheTest::A"]));
        library.elements[3].properties.insert("metadata".into(), json!({"declared_short_name":"v"}));
        library.elements[0].kind = "SysML::PartDefinition".into();
        assert_eq!(allocation, library.elements.as_ptr());
        let after = cached_library_indexes(&library, mappings).unwrap();
        assert!(!Arc::ptr_eq(&before, &after));
        assert_eq!(after.feature_types["CacheTest::B::value"], vec!["CacheTest::A"]);
        assert_eq!(after.specializations["CacheTest::A"], vec!["CacheTest::C", "CacheTest::B", "CacheTest::B"]);
        assert_eq!(after.kinds["CacheTest::A"], "SysML::PartDefinition");
        assert_eq!(after.aliases["CacheTest::B::v"], "CacheTest::B::value");
        assert_eq!(after.feature_index, build_stdlib_feature_index(&library, mappings).unwrap());
        assert!(Arc::ptr_eq(&after, &cached_library_indexes(&library, mappings).unwrap()));
    }
}
