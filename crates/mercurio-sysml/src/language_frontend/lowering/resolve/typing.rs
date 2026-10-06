//! Native execution of mechanically extracted Pilot type-family predicates.
//! Selection is explicit in the generator; this is not the full validator set.
mod feature_checks;
mod scalar_checks;
mod type_family_checks;
use super::*;
use crate::language_frontend::lowering::relationship_declarations::metaclass_conforms as kind_conforms_to;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct TypeChecks {
    checks: Vec<TypeCheck>,
    reference_checks: Vec<ReferenceCheck>,
    owned_end_checks: Vec<OwnedEndCheck>,
}
#[derive(Deserialize)]
struct TypeCheck {
    quantifier: String,
    context: String,
    #[serde(default)]
    excluded_contexts: Vec<String>,
    required_type: String,
    message: String,
    issue: String,
}
#[derive(Deserialize)]
struct ReferenceCheck {
    context: String,
    #[serde(default)]
    excluded_contexts: Vec<String>,
    required_type: String,
    message: String,
    issue: String,
}
#[derive(Deserialize)]
struct OwnedEndCheck {
    context: String,
    maximum: usize,
    message: String,
    issue: String,
}
fn checks() -> Result<&'static TypeChecks, Diagnostic> {
    static CHECKS: OnceLock<Result<TypeChecks, String>> = OnceLock::new();
    CHECKS
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../resources/metamodels/sysml-2.0-pilot-2026-08/type-checks.extract.json"
            ))
            .map_err(|error| format!("invalid extracted usage type checks: {error}"))
        })
        .as_ref()
        .map_err(|error| Diagnostic::new(error.clone(), None))
}

/// Pilot FeatureAdapter.getAllTypes: gather typing features with a visited set,
/// then remove types generalized by another collected type. The graph here is
/// the native declared/default typing and subsetting/redefinition graph; computed
/// conjugation, cross features and implicit expression-result edges still need
/// their own lowering support.
pub(super) fn validate_usage_types(
    module: &ResolvedModule,
    context: &ResolverContext,
    mappings: &MappingBundle,
    lookup: &FeatureLookup<'_>,
) -> Result<(), Diagnostic> {
    let checks = checks()?;
    for definition in &module.definitions {
        let kind = mappings.metaclass_for(&definition.construct)?;
        for check in &checks.owned_end_checks {
            if !kind_conforms_to(kind, &check.context) {
                continue;
            }
            let extra = definition
                .members
                .iter()
                .filter(|member| {
                    member
                        .modifiers
                        .iter()
                        .any(|m| m == "end" || m.starts_with("end-"))
                })
                .nth(check.maximum);
            if let Some(end) = extra {
                return Err(Diagnostic::new(
                    format!("{}: {}", check.issue, check.message),
                    Some(end.span.clone()),
                ));
            }
        }
    }
    let mut pending = module.usages.iter().collect::<Vec<_>>();
    pending.extend(module.definitions.iter().flat_map(|d| &d.members));
    let mut usages = BTreeMap::new();
    while let Some(usage) = pending.pop() {
        pending.extend(&usage.members);
        usages.insert(feature_id_from_qualified_name(&usage.qualified_name), usage);
    }
    for usage in usages.values() {
        let metaclass = mappings.metaclass_for(&usage.construct)?;
        if let Some(target) = &usage.reference_target {
            for check in &checks.reference_checks {
                if !kind_conforms_to(metaclass, &check.context)
                    || check.excluded_contexts.iter().any(|excluded| {
                        kind_conforms_to(metaclass, excluded)
                    })
                {
                    continue;
                }
                let target_kind = if let Some(resolved) = usages.get(target) {
                    Some(mappings.metaclass_for(&resolved.construct)?)
                } else if let Some(collected) = target
                    .strip_prefix("feature.")
                    .and_then(|name| context.local_usage_map.get(name))
                {
                    Some(mappings.metaclass_for(&collected.construct)?)
                } else {
                    context
                        .library_indexes
                        .kinds
                        .get(target)
                        .map(String::as_str)
                };
                if !target_kind.is_some_and(|kind| {
                    kind_conforms_to(kind, &check.required_type)
                }) {
                    return Err(Diagnostic::new(
                        format!(
                            "{}: {} Referenced target `{target}`.",
                            check.issue, check.message
                        ),
                        Some(usage.span.clone()),
                    ));
                }
            }
        }
        let types = all_types(usage, &usages, context, mappings, lookup);
        type_family_checks::validate(usage, &types, context, mappings)?;
        for check in &checks.checks {
            if type_family_checks::owns_context(&check.context) { continue; }
            if !kind_conforms_to(metaclass, &check.context)
                || check
                    .excluded_contexts
                    .iter()
                    .any(|excluded| kind_conforms_to(metaclass, excluded))
            {
                continue;
            }
            let mut valid = check.quantifier == "all" || types.len() == 1;
            for ty in &types {
                let kind = if let Some(name) = ty.strip_prefix("type.") {
                    context
                        .definition_index
                        .get(name)
                        .map(|d| mappings.metaclass_for(&d.construct))
                        .transpose()?
                } else {
                    context.library_indexes.kinds.get(ty).map(String::as_str)
                };
                if let Some(kind) = kind {
                    valid &= kind_conforms_to(kind, &check.required_type);
                }
            }
            if !valid {
                return Err(Diagnostic::new(
                    format!(
                        "{}: {} Usage `{}` has effective types [{}].",
                        check.issue,
                        check.message,
                        usage.declared_name,
                        types.join(", ")
                    ),
                    Some(usage.span.clone()),
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn all_types(
    usage: &ResolvedUsage,
    resolved: &BTreeMap<String, &ResolvedUsage>,
    context: &ResolverContext,
    mappings: &MappingBundle,
    lookup: &FeatureLookup<'_>,
) -> Vec<String> {
    feature_types(&feature_id_from_qualified_name(&usage.qualified_name), resolved, context, mappings, lookup)
}

pub(super) fn feature_types(
    feature: &str,
    resolved: &BTreeMap<String, &ResolvedUsage>,
    context: &ResolverContext,
    mappings: &MappingBundle,
    lookup: &FeatureLookup<'_>,
) -> Vec<String> {
    let mut pending = vec![feature.to_string()];
    let mut visited = BTreeSet::new();
    let mut types = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        let collected = id
            .strip_prefix("feature.")
            .and_then(|name| context.local_usage_map.get(name));
        if let Some(usage) = resolved.get(&id) {
            // FeatureAdapter.addOwnedCrossFeatureSpecialization types an owned
            // crossing feature from its owning end; checked by the default extractor.
            if usage.modifiers.iter().any(|m| m == "owned_crossing_feature") {
                pending.push(feature_id_from_qualified_name(&usage.owner_qualified_name));
            }
            types.extend(usage.type_ref.iter().cloned());
            // A resolved declaration belongs to the module being compiled.
            // A context index can contain a same-named declaration from another
            // source, so its implicit default must not be mixed into this one.
            if let Some(default) =
                mappings.usage_family_default(&usage.construct, &usage.owner_construct)
            {
                types.insert(default.type_ref);
            }
            if let Some(default) = mappings.usage_type_default(usage) {
                types.insert(
                    context
                        .local_definitions
                        .get(&default)
                        .cloned()
                        .unwrap_or(default),
                );
            }
            types.extend(usage.additional_type_refs.iter().cloned());
            types.extend(usage.specializes.iter().cloned());
            pending.extend(usage.specialized_features.iter().cloned());
            pending.extend(usage.subsetted_features.iter().cloned());
            pending.extend(usage.redefined_features.iter().cloned());
            // An undirected value feature inherits the result's typing. Direct
            // feature-path values provide a resolved result without guessing a
            // type from a spelling or a literal's host representation.
            if !usage.has_explicit_specialization
                && !usage
                    .modifiers
                    .iter()
                    .any(|m| m == "feature_value_is_default")
                && !usage
                    .modifiers
                    .iter()
                    .any(|m| matches!(m.as_str(), "in" | "out" | "inout" | "return"))
                && let Some(ResolvedExpr::FeaturePath { segments }) = &usage.expression
                && let Some(last) = segments.last()
            {
                pending.push(last.feature_id.clone());
            }
            if usage.construct != "MetadataUsage"
                && let Some(target) = &usage.reference_target
            {
                if target.starts_with("feature.")
                    || context.library_indexes.feature_types.contains_key(target)
                {
                    pending.push(target.clone());
                }
            }
        } else if let Some(usage) = collected {
            if usage.modifiers.iter().any(|m| m == "owned_crossing_feature") {
                pending.push(feature_id_from_qualified_name(&usage.owner_qualified_name));
            }
            types.extend(usage.implicit_type.iter().cloned());
            // Support-file features retain their own lexical scope and imports.
            for name in usage.ty.iter().chain(&usage.additional_types) {
                if let Some(ty) = resolve_type_reference_in_scope(
                    name,
                    &usage.owner_qualified_name,
                    lookup.stdlib_ids,
                    lookup.stdlib_aliases,
                    lookup.local_definitions,
                    lookup.local_aliases,
                    lookup.import_aliases,
                ) {
                    types.insert(ty);
                }
            }
            for name in usage
                .subsets
                .iter()
                .chain(&usage.redefines)
                .chain(&usage.specializes)
                .chain(&usage.reference_target)
            {
                if let Some(target) =
                    lookup.resolve(name, &usage.owner_qualified_name, "", &mut BTreeSet::new())
                {
                    if target.starts_with("type.") {
                        types.insert(target);
                    } else {
                        pending.push(target);
                    }
                }
            }
        } else {
            if let Some(direct) = context.library_indexes.feature_types.get(&id) {
                types.extend(direct.iter().cloned());
            }
            if let Some(parents) = context.library_indexes.specializations.get(&id) {
                pending.extend(
                    parents
                        .iter()
                        .filter(|parent| {
                            context.library_indexes.feature_types.contains_key(*parent)
                        })
                        .cloned(),
                );
            }
        }
    }
    types
        .iter()
        .filter(|general| {
            !types.iter().any(|specific| {
                specific != *general
                    && lookup.type_specializes(
                        specific,
                        general,
                        Some(&context.library_indexes.specializations),
                        &mut BTreeSet::new(),
                    )
            })
        })
        .cloned()
        .collect()
}

pub(super) fn kind_conforms(kind: &str, general: &str) -> Result<bool, Diagnostic> {
    Ok(kind_conforms_to(kind, general))
}

#[derive(Deserialize)]
struct UsageFlags {
    non_variable_contexts: Vec<String>,
    owner_type: String,
    excluded_types: Vec<String>,
    composite_excluded_type: String,
    non_composite_contexts: Vec<String>,
}

pub(super) fn derive_composite_flags(
    module: &mut ResolvedModule,
    context: &ResolverContext,
    mappings: &MappingBundle,
    lookup: &FeatureLookup<'_>,
) -> Result<(), Diagnostic> {
    derive_flags(module, context, mappings, lookup, true)
}

pub(super) fn derive_usage_flags(
    module: &mut ResolvedModule,
    context: &ResolverContext,
    mappings: &MappingBundle,
    lookup: &FeatureLookup<'_>,
) -> Result<(), Diagnostic> {
    derive_flags(module, context, mappings, lookup, false)
}

fn derive_flags(
    module: &mut ResolvedModule,
    context: &ResolverContext,
    mappings: &MappingBundle,
    lookup: &FeatureLookup<'_>,
    composite_only: bool,
) -> Result<(), Diagnostic> {
    static FLAGS: OnceLock<Result<UsageFlags, String>> = OnceLock::new();
    let flags = FLAGS
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../resources/metamodels/sysml-2.0-pilot-2026-08/usage-flags.extract.json"
            ))
            .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| {
            Diagnostic::new(format!("invalid usage flag extraction: {error}"), None)
        })?;
    // ReferenceUsage compositeness is fixed by its metaclass, independent of
    // occurrence-library ancestry. Partial fixtures can derive these two flags;
    // keep all ancestry-dependent flags unassessed in those fixtures.
    let partial_library = !context.library_indexes.kinds.contains_key(&flags.owner_type);
    let mut pending = module.usages.iter().collect::<Vec<_>>();
    pending.extend(
        module
            .definitions
            .iter()
            .flat_map(|definition| &definition.members),
    );
    pending.extend(module.aliases.iter().flat_map(|alias| &alias.members));
    pending.extend(module.imports.iter().flat_map(|import| &import.members));
    let mut usages = BTreeMap::new();
    let mut occurrences = Vec::new();
    while let Some(usage) = pending.pop() {
        pending.extend(&usage.members);
        occurrences.push(usage);
        usages.insert(feature_id_from_qualified_name(&usage.qualified_name), usage);
    }
    let mut properties: BTreeMap<(String, String), BTreeMap<String, bool>> = BTreeMap::new();
    for usage in occurrences {
        let kind = mappings.metaclass_for(&usage.construct)?;
        if !kind_conforms_to(kind, "Usage")
            || (partial_library && !kind_conforms_to(kind, "ReferenceUsage")) {
            continue;
        }
        let specializes = |specific: &str, general: &str| {
            lookup.type_specializes(
                specific,
                general,
                Some(&context.library_indexes.specializations),
                &mut BTreeSet::new(),
            )
        };
        let types = if composite_only || partial_library {
            Vec::new()
        } else {
            all_types(usage, &usages, context, mappings, lookup)
        };
        let typed_by = |general: &str| types.iter().any(|specific| specializes(specific, general));
        let owner_usage = usages.get(&feature_id_from_qualified_name(&usage.owner_qualified_name));
        let owner_definition = module
            .definitions
            .iter()
            .find(|definition| definition.qualified_name == usage.owner_qualified_name);
        let variant = usage.modifiers.iter().any(|m| m == "variant");
        // A VariantMembership does not make its member an owned feature.
        // A variant can inherit its featuring type through a variation usage.
        let mut featuring_usage = usage;
        let mut seen_owners = BTreeSet::new();
        while featuring_usage.modifiers.iter().any(|m| m == "variant") {
            if !seen_owners.insert(&featuring_usage.qualified_name) {
                break;
            }
            let Some(owner) = usages.get(&feature_id_from_qualified_name(
                &featuring_usage.owner_qualified_name,
            )) else {
                break;
            };
            if !owner.modifiers.iter().any(|m| m == "variation") {
                break;
            }
            featuring_usage = owner;
        }
        let has_owner_type = !featuring_usage.modifiers.iter().any(|m| m == "variant")
            && (usages.contains_key(&feature_id_from_qualified_name(
                &featuring_usage.owner_qualified_name,
            )) || module
                .definitions
                .iter()
                .any(|d| d.qualified_name == featuring_usage.owner_qualified_name));
        let owner_is_occurrence = if composite_only || partial_library {
            false
        } else if let Some(owner) = owner_usage {
            all_types(owner, &usages, context, mappings, lookup)
                .iter()
                .any(|ty| specializes(ty, &flags.owner_type))
        } else if let Some(owner) = owner_definition {
            owner
                .specializes
                .iter()
                .any(|ty| specializes(ty, &flags.owner_type))
        } else {
            false
        };
        let owner_kind = mappings.metaclass_for(&featuring_usage.owner_construct)?;
        let owner_attribute = ["AttributeDefinition", "AttributeUsage"]
            .iter()
            .any(|general| kind_conforms_to(owner_kind, general));
        let owner_port = ["PortDefinition", "PortUsage"]
            .iter()
            .any(|general| kind_conforms_to(owner_kind, general));
        let directed = usage.modifiers.iter().any(|m| {
            matches!(
                m.as_str(),
                "in" | "out" | "inout" | "return" | "end" | "end-source" | "end-target" | "ref"
            )
        });
        let composite = has_owner_type
            && !owner_attribute
            && !directed
            && !flags
                .non_composite_contexts
                .iter()
                .any(|general| kind_conforms_to(kind, general))
            && (!kind_conforms_to(kind, "PortUsage") || owner_port);
        let portion = usage
            .modifiers
            .iter()
            .any(|m| matches!(m.as_str(), "snapshot" | "timeslice"));
        // Owned crossing features have an OwningMembership, not a FeatureMembership;
        // their owner is the end, while owningType is absent (UsageAdapter.mayTimeVary).
        let variable = !variant
            && !usage.modifiers.iter().any(|m| m == "owned_crossing_feature")
            && !flags.non_variable_contexts.iter().any(|general| kind_conforms_to(kind, general))
            && owner_is_occurrence
            && !portion
            && !flags.excluded_types.iter().any(|ty| typed_by(ty))
            && !(composite && typed_by(&flags.composite_excluded_type));
        properties.insert(
            (usage.qualified_name.clone(), usage.construct.clone()),
            BTreeMap::from([
                (
                    "is_ordered".into(),
                    usage.modifiers.iter().any(|m| m == "ordered"),
                ),
                (
                    "is_unique".into(),
                    !usage.modifiers.iter().any(|m| m == "nonunique"),
                ),
                ("is_variable".into(), variable),
                ("may_time_vary".into(), variable),
                ("is_composite".into(), composite),
                ("is_reference".into(), !composite),
                ("is_portion".into(), portion),
                (
                    "is_constant".into(),
                    usage.modifiers.iter().any(|m| m == "constant")
                        || (variable
                            && usage
                                .modifiers
                                .iter()
                                .any(|m| m == "end" || m.starts_with("end-"))),
                ),
                (
                    "is_abstract".into(),
                    usage
                        .modifiers
                        .iter()
                        .any(|m| matches!(m.as_str(), "abstract" | "variation"))
                        || (usage.construct == "Message"
                            && !usage.members.iter().any(|member| {
                                member
                                    .modifiers
                                    .iter()
                                    .any(|m| m == "end" || m.starts_with("end-"))
                            })),
                ),
                (
                    "is_variation".into(),
                    usage.modifiers.iter().any(|m| m == "variation"),
                ),
            ]),
        );
    }
    if partial_library {
        for derived in properties.values_mut() {
            derived.retain(|key, _| matches!(key.as_str(), "is_composite" | "is_reference"));
        }
    }
    fn assign(
        usage: &mut ResolvedUsage,
        properties: &BTreeMap<(String, String), BTreeMap<String, bool>>,
    ) {
        // Resolution can retain several representations of one feature. Do not
        // consume the flags after the first occurrence or apply them to another
        // metaclass sharing the same qualified name (for example PayloadFeature).
        if let Some(flags) = properties.get(&(usage.qualified_name.clone(), usage.construct.clone())) {
            usage.derived_properties.extend(flags.clone());
        }
        for member in &mut usage.members {
            assign(member, properties);
        }
    }
    for usage in &mut module.usages {
        assign(usage, &properties);
    }
    for definition in &mut module.definitions {
        for usage in &mut definition.members {
            assign(usage, &properties);
        }
    }
    for usage in module.aliases.iter_mut().flat_map(|alias| &mut alias.members)
        .chain(module.imports.iter_mut().flat_map(|import| &mut import.members)) {
        assign(usage, &properties);
    }
    Ok(())
}

pub(super) fn validate_scalar_checks(module: &ResolvedModule, mappings: &MappingBundle) -> Result<(), Diagnostic> {
    scalar_checks::validate(module, mappings)
}

pub(super) fn validate_feature_flags(
    module: &mut ResolvedModule,
    context: &ResolverContext,
    mappings: &MappingBundle,
    lookup: &FeatureLookup<'_>,
) -> Result<(), Diagnostic> {
    feature_checks::apply(module, context, mappings, lookup)
}
