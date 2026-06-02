use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::Value;

use mercurio_kir::KirDocument;
use mercurio_language_contracts::ast::{
    Expr, LiteralExpr, ParsedModule as SysmlModule, QualifiedName, SourceSpan,
};
use mercurio_language_contracts::diagnostics::Diagnostic;

use crate::logging::{compile_timer_start, log_compile_timed_event};
use crate::lowering::collect::{
    CollectedDefinition, CollectedImport, CollectedUsage, ImportAliases, collect_module,
    collect_modules,
};
use crate::lowering::elaborate::{
    shorthand_reference_target, should_use_implicit_reference_redefinition_target,
};
use crate::lowering::emit::MappingBundle;
use crate::lowering::imports::build_import_alias_map;
use crate::lowering::indexes::{
    LibraryIndexes, build_local_alias_map, build_local_definition_map, build_local_feature_index,
    build_local_usage_map, cached_library_indexes,
};
pub use crate::lowering::ir::{
    ResolvedDefinition, ResolvedExpr, ResolvedImport, ResolvedModule, ResolvedPackage,
    ResolvedPathSegment, ResolvedUsage,
};
use crate::lowering::names::expand_import_namespace_prefix;
use crate::lowering::policy::{KERML_RESOLVE_POLICY, ResolvePolicy, STRICT_RESOLVE_POLICY};

fn expression_span(expr: &Expr) -> SourceSpan {
    match expr {
        Expr::Literal(_) => SourceSpan {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        },
        Expr::Name(name) => name.span.clone(),
        Expr::SelfRef(span) => span.clone(),
        Expr::Tuple { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Path { span, .. }
        | Expr::Call { span, .. } => span.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct ResolverContext {
    module_count: usize,
    packages: Vec<ResolvedPackage>,
    definitions: Vec<CollectedDefinition>,
    local_definitions: BTreeMap<String, String>,
    definition_index: BTreeMap<String, CollectedDefinition>,
    local_feature_index: BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: BTreeMap<String, CollectedUsage>,
    library_indexes: Arc<LibraryIndexes>,
}

impl ResolverContext {
    pub fn module_count(&self) -> usize {
        self.module_count
    }

    pub fn from_modules(
        context_modules: &[SysmlModule],
        library_context: &KirDocument,
        mappings: &MappingBundle,
    ) -> Result<Self, Diagnostic> {
        let collect_context_start = compile_timer_start();
        let collected_context = collect_modules(context_modules, mappings)?;
        let packages = collected_context.packages;
        let definitions = collected_context.definitions;
        let usages = collected_context.usages;
        log_compile_timed_event(
            "resolver.collect_context_modules",
            collect_context_start,
            "ok",
            format!(
                "context_modules={} packages={} definitions={} usages={}",
                context_modules.len(),
                packages.len(),
                definitions.len(),
                usages.len()
            ),
        );

        let local_index_start = compile_timer_start();
        let local_definitions = build_local_definition_map(&definitions, mappings)?;
        let definition_index = definitions
            .iter()
            .cloned()
            .map(|definition| (definition.qualified_name.clone(), definition))
            .collect::<BTreeMap<_, _>>();
        let local_feature_index = build_local_feature_index(&definitions, &usages);
        let local_usage_map = build_local_usage_map(&definitions, &usages);
        log_compile_timed_event(
            "resolver.build_local_indexes",
            local_index_start,
            "ok",
            format!("definitions={} usages={}", definitions.len(), usages.len()),
        );

        let stdlib_index_start = compile_timer_start();
        let library_indexes = cached_library_indexes(library_context, mappings);
        log_compile_timed_event(
            "resolver.build_library_indexes",
            stdlib_index_start,
            "ok",
            format!(
                "library_elements={} aliases={} cache=instance_keyed",
                library_context.elements.len(),
                library_indexes.aliases.len()
            ),
        );

        Ok(Self {
            module_count: context_modules.len(),
            packages,
            definitions,
            local_definitions,
            definition_index,
            local_feature_index,
            local_usage_map,
            library_indexes,
        })
    }
}

pub fn resolve_module(
    module: &SysmlModule,
    library_context: &KirDocument,
    mappings: &MappingBundle,
) -> Result<ResolvedModule, Diagnostic> {
    resolve_module_with_context(
        module,
        std::slice::from_ref(module),
        library_context,
        mappings,
    )
}

pub fn resolve_module_with_context(
    module: &SysmlModule,
    context_modules: &[SysmlModule],
    library_context: &KirDocument,
    mappings: &MappingBundle,
) -> Result<ResolvedModule, Diagnostic> {
    resolve_module_with_policy(
        module,
        context_modules,
        library_context,
        mappings,
        STRICT_RESOLVE_POLICY,
    )
}

pub fn resolve_module_with_resolver_context(
    module: &SysmlModule,
    context: &ResolverContext,
    mappings: &MappingBundle,
) -> Result<ResolvedModule, Diagnostic> {
    resolve_module_with_policy_context(module, context, mappings, STRICT_RESOLVE_POLICY)
}

pub fn resolve_kerml_module_with_context(
    module: &SysmlModule,
    context_modules: &[SysmlModule],
    library_context: &KirDocument,
    mappings: &MappingBundle,
) -> Result<ResolvedModule, Diagnostic> {
    resolve_module_with_policy(
        module,
        context_modules,
        library_context,
        mappings,
        KERML_RESOLVE_POLICY,
    )
}

pub fn resolve_kerml_module_with_resolver_context(
    module: &SysmlModule,
    context: &ResolverContext,
    mappings: &MappingBundle,
) -> Result<ResolvedModule, Diagnostic> {
    resolve_module_with_policy_context(module, context, mappings, KERML_RESOLVE_POLICY)
}

fn resolve_module_with_policy(
    module: &SysmlModule,
    context_modules: &[SysmlModule],
    library_context: &KirDocument,
    mappings: &MappingBundle,
    policy: ResolvePolicy,
) -> Result<ResolvedModule, Diagnostic> {
    let context = ResolverContext::from_modules(context_modules, library_context, mappings)?;
    resolve_module_with_policy_context(module, &context, mappings, policy)
}

fn resolve_module_with_policy_context(
    module: &SysmlModule,
    context: &ResolverContext,
    mappings: &MappingBundle,
    policy: ResolvePolicy,
) -> Result<ResolvedModule, Diagnostic> {
    let collect_module_start = compile_timer_start();
    let collected_module = collect_module(module, mappings)?;
    let packages = collected_module.packages;
    let imports = collected_module.imports;
    let definitions = collected_module.definitions;
    let usages = collected_module.usages;
    let aliases = collected_module.aliases;
    log_compile_timed_event(
        "resolver.collect_module",
        collect_module_start,
        "ok",
        format!(
            "packages={} imports={} definitions={} usages={} aliases={}",
            packages.len(),
            imports.len(),
            definitions.len(),
            usages.len(),
            aliases.len()
        ),
    );

    let local_aliases = build_local_alias_map(&aliases);

    let resolve_import_start = compile_timer_start();
    let resolved_imports = resolve_imports(
        &imports,
        &context.library_indexes.ids,
        &context.library_indexes.aliases,
        &context.local_definitions,
        &local_aliases,
    )?;
    log_compile_timed_event(
        "resolver.resolve_imports",
        resolve_import_start,
        "ok",
        format!("imports={}", resolved_imports.len()),
    );

    let import_alias_start = compile_timer_start();
    let import_aliases = build_import_alias_map(
        &resolved_imports,
        &context.packages,
        &context.definitions,
        &context.local_usage_map,
        &context.library_indexes.ids,
        &context.library_indexes.aliases,
        policy,
    )?;
    log_compile_timed_event(
        "resolver.build_import_aliases",
        import_alias_start,
        "ok",
        format!(
            "namespace_aliases={} value_aliases={} ambiguous_namespace_aliases={} ambiguous_value_aliases={}",
            import_aliases.namespace_aliases.len(),
            import_aliases.value_aliases.len(),
            import_aliases.ambiguous_namespace_aliases.len(),
            import_aliases.ambiguous_value_aliases.len()
        ),
    );

    let resolve_definition_start = compile_timer_start();
    let resolved_definitions = definitions
        .into_iter()
        .map(|definition| {
            resolve_definition(
                definition,
                &context.library_indexes.ids,
                &context.library_indexes.feature_index,
                &context.library_indexes.aliases,
                &context.local_definitions,
                &local_aliases,
                &import_aliases,
                &context.definition_index,
                &context.local_feature_index,
                &context.local_usage_map,
                mappings,
                policy,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    log_compile_timed_event(
        "resolver.resolve_definitions",
        resolve_definition_start,
        "ok",
        format!("definitions={}", resolved_definitions.len()),
    );

    let resolve_usage_start = compile_timer_start();
    let resolved_usages = usages
        .into_iter()
        .map(|usage| {
            resolve_usage(
                usage,
                &context.library_indexes.ids,
                &context.library_indexes.feature_index,
                &context.library_indexes.aliases,
                &context.local_definitions,
                &local_aliases,
                &import_aliases,
                &context.definition_index,
                &context.local_feature_index,
                &context.local_usage_map,
                mappings,
                policy,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    log_compile_timed_event(
        "resolver.resolve_usages",
        resolve_usage_start,
        "ok",
        format!("usages={}", resolved_usages.len()),
    );

    Ok(ResolvedModule {
        packages,
        imports: resolved_imports,
        definitions: resolved_definitions,
        usages: resolved_usages,
    })
}

fn resolve_imports(
    imports: &[CollectedImport],
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
) -> Result<Vec<ResolvedImport>, Diagnostic> {
    let mut resolved = Vec::new();

    for import in imports {
        let target_id = resolve_import_target(
            &import.decl.path,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
        )
        .ok_or_else(|| {
            Diagnostic::new(
                format!("unresolved import `{}`", import.decl.path.as_colon_string()),
                Some(import.decl.span.clone()),
            )
        })?;

        resolved.push(ResolvedImport {
            owner_package_qualified_name: import.owner_package_qualified_name.clone(),
            target_id,
            imported_name: import
                .decl
                .path
                .segments
                .last()
                .cloned()
                .filter(|name| name != "*" && name != "**"),
            docs: import.decl.docs.clone(),
            span: import.decl.span.clone(),
            ordinal: resolved.len() + 1,
        });
    }

    Ok(resolved)
}

fn resolve_definition(
    definition: CollectedDefinition,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    mappings: &MappingBundle,
    policy: ResolvePolicy,
) -> Result<ResolvedDefinition, Diagnostic> {
    let specializes = definition
        .specializes
        .iter()
        .map(|name| {
            unresolved_or_error(
                resolve_type_reference_in_scope(
                    name,
                    &definition.qualified_name,
                    stdlib_ids,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    import_aliases,
                ),
                name,
                "specialization",
                policy,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let members = definition
        .members
        .into_iter()
        .map(|usage| {
            resolve_usage(
                usage,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
                mappings,
                policy,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ResolvedDefinition {
        construct: definition.construct,
        qualified_name: definition.qualified_name,
        declared_name: definition.declared_name,
        is_abstract: definition.is_abstract,
        specializes,
        members,
        docs: definition.docs,
        span: definition.span,
    })
}

fn resolve_usage(
    usage: CollectedUsage,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    mappings: &MappingBundle,
    policy: ResolvePolicy,
) -> Result<ResolvedUsage, Diagnostic> {
    let mut effective_reference_target = usage.reference_target.clone();
    let mut effective_redefines = usage.redefines.clone();
    if should_use_implicit_reference_redefinition_target(mappings, &usage) {
        effective_reference_target = effective_redefines.first().cloned();
        effective_redefines.clear();
    }
    if effective_reference_target.is_none() {
        effective_reference_target = shorthand_reference_target(mappings, &usage);
    }
    let expression = usage
        .expression
        .as_ref()
        .map(|expr| {
            resolve_expression(
                &usage,
                expr,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            )
        })
        .transpose()?;
    let mut specialized_features = Vec::new();
    let mut type_ref = match &usage.ty {
        Some(name) => {
            if let Some(target) = resolve_type_reference_in_scope(
                name,
                &usage.owner_qualified_name,
                stdlib_ids,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
            ) {
                Some(target)
            } else if let Some(target) = resolve_feature_reference(
                &usage,
                name,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            ) {
                specialized_features.push(target);
                None
            } else {
                Some(unresolved_or_error(None, name, "type", policy)?)
            }
        }
        None => None,
    };
    let reference_target = effective_reference_target
        .as_ref()
        .map(|name| {
            let reference_target_policy =
                mappings.usage_reference_target_resolution_policy(&usage.construct);
            if reference_target_policy == Some("annotation_target_then_type_then_reference") {
                resolve_comment_annotation_target(&usage, name, local_definitions, local_usage_map)
                    .or_else(|| {
                        resolve_type_reference_in_scope(
                            name,
                            &usage.owner_qualified_name,
                            stdlib_ids,
                            stdlib_aliases,
                            local_definitions,
                            local_aliases,
                            import_aliases,
                        )
                    })
                    .or_else(|| {
                        resolve_reference_usage_target(
                            &usage,
                            name,
                            stdlib_ids,
                            stdlib_feature_index,
                            stdlib_aliases,
                            local_definitions,
                            local_aliases,
                            import_aliases,
                            definition_index,
                            local_feature_index,
                            local_usage_map,
                        )
                    })
            } else if reference_target_policy == Some("type_then_reference") {
                resolve_type_reference_in_scope(
                    name,
                    &usage.owner_qualified_name,
                    stdlib_ids,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    import_aliases,
                )
                .or_else(|| {
                    resolve_reference_usage_target(
                        &usage,
                        name,
                        stdlib_ids,
                        stdlib_feature_index,
                        stdlib_aliases,
                        local_definitions,
                        local_aliases,
                        import_aliases,
                        definition_index,
                        local_feature_index,
                        local_usage_map,
                    )
                })
            } else {
                resolve_reference_usage_target(
                    &usage,
                    name,
                    stdlib_ids,
                    stdlib_feature_index,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    import_aliases,
                    definition_index,
                    local_feature_index,
                    local_usage_map,
                )
            }
            .ok_or_else(|| {
                Diagnostic::new(
                    format!("unresolved reference target `{}`", name.as_colon_string()),
                    Some(name.span.clone()),
                )
            })
        })
        .transpose()?;
    let allocation_source = usage
        .allocation_source
        .as_ref()
        .map(|name| {
            resolve_allocation_endpoint(
                &usage,
                name,
                false,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            )
            .ok_or_else(|| {
                Diagnostic::new(
                    format!("unresolved allocation source `{}`", name.as_colon_string()),
                    Some(name.span.clone()),
                )
            })
        })
        .transpose()?;
    let allocation_target = usage
        .allocation_target
        .as_ref()
        .map(|name| {
            resolve_allocation_endpoint(
                &usage,
                name,
                true,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            )
            .ok_or_else(|| {
                Diagnostic::new(
                    format!("unresolved allocation target `{}`", name.as_colon_string()),
                    Some(name.span.clone()),
                )
            })
        })
        .transpose()?;
    let additional_type_refs = usage
        .additional_types
        .iter()
        .map(|name| {
            unresolved_or_error(
                resolve_type_reference_in_scope(
                    name,
                    &usage.owner_qualified_name,
                    stdlib_ids,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    import_aliases,
                ),
                name,
                "type",
                policy,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut specializes = Vec::new();
    for name in &usage.specializes {
        if let Some(target) = resolve_feature_reference(
            &usage,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
        ) {
            specialized_features.push(target);
        } else if is_self_feature_reference(&usage, name) {
            specialized_features.push(feature_id_from_qualified_name(&usage.qualified_name));
        } else if let Some(target) = resolve_type_reference_in_scope(
            name,
            &usage.owner_qualified_name,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        ) {
            specializes.push(target);
        } else {
            specializes.push(unresolved_or_error(None, name, "specialization", policy)?);
        }
    }
    let subsetted_features = usage
        .subsets
        .iter()
        .map(|name| {
            unresolved_or_error(
                resolve_feature_reference(
                    &usage,
                    name,
                    stdlib_ids,
                    stdlib_feature_index,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    import_aliases,
                    definition_index,
                    local_feature_index,
                    local_usage_map,
                ),
                name,
                "subset target",
                policy,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let redefined_features = effective_redefines
        .iter()
        .map(|name| {
            let modifier_alias_target = || {
                unique_feature_modifier_alias_match_excluding(
                    name.segments.first()?,
                    local_feature_index,
                    local_usage_map,
                    &usage.qualified_name,
                )
                .map(|qualified_name| feature_id_from_qualified_name(&qualified_name))
            };
            unresolved_or_error(
                resolve_redefinition_feature_reference(
                    &usage,
                    name,
                    stdlib_ids,
                    stdlib_feature_index,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    import_aliases,
                    definition_index,
                    local_feature_index,
                    local_usage_map,
                )
                .or_else(|| {
                    (name.segments.len() == 1)
                        .then(modifier_alias_target)
                        .flatten()
                }),
                name,
                "redefinition target",
                policy,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    if let Some((connection_end_policy, parent_construct)) =
        mappings.usage_connection_end_specialization_policy(&usage.construct)
        && connection_end_policy == "from_parent_connection_type_member"
    {
        if let Some(parent_feature) = resolve_connection_end_specialization(
            &usage,
            parent_construct,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_usage_map,
        ) {
            specialized_features.push(parent_feature);
        }
        if type_ref.is_none() {
            type_ref = reference_target.as_deref().and_then(|target| {
                infer_usage_type_from_feature_id(
                    target,
                    stdlib_ids,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    import_aliases,
                    local_usage_map,
                )
            });
        }
    }
    let type_ref = type_ref
        .or_else(|| {
            inferred_usage_type_ref(
                &usage,
                &redefined_features,
                &subsetted_features,
                &specialized_features,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            )
        })
        .or_else(|| infer_named_definition_type_ref(&usage, local_definitions));
    let members = usage
        .members
        .into_iter()
        .map(|member| {
            resolve_usage(
                member,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
                mappings,
                policy,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ResolvedUsage {
        construct: usage.construct,
        owner_construct: usage.owner_construct,
        owner_qualified_name: usage.owner_qualified_name,
        qualified_name: usage.qualified_name,
        declared_name: usage.declared_name,
        is_implicit_name: usage.is_implicit_name,
        has_explicit_type: usage.ty.is_some() || !usage.additional_types.is_empty(),
        type_ref,
        additional_type_refs,
        reference_target,
        allocation_source,
        allocation_target,
        metadata_properties: usage.metadata_properties,
        multiplicity: usage.multiplicity,
        expression,
        is_derived: usage.modifiers.iter().any(|modifier| modifier == "derived")
            || (usage.expression.is_some() && effective_redefines.is_empty()),
        specializes,
        specialized_features,
        subsetted_features,
        redefined_features,
        members,
        modifiers: usage.modifiers,
        docs: usage.docs,
        span: usage.span,
    })
}

#[allow(clippy::too_many_arguments)]
fn resolve_expression(
    usage: &CollectedUsage,
    expr: &Expr,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Result<ResolvedExpr, Diagnostic> {
    match expr {
        Expr::Literal(LiteralExpr::Integer(value)) => {
            Ok(ResolvedExpr::Literal(Value::from(*value)))
        }
        Expr::Literal(LiteralExpr::Real(value)) => {
            let parsed = value.parse::<f64>().map_err(|_| {
                Diagnostic::new("invalid real literal", Some(expression_span(expr)))
            })?;
            Ok(ResolvedExpr::Literal(Value::from(parsed)))
        }
        Expr::Literal(LiteralExpr::Boolean(value)) => {
            Ok(ResolvedExpr::Literal(Value::from(*value)))
        }
        Expr::Literal(LiteralExpr::String(value)) => {
            Ok(ResolvedExpr::Literal(Value::from(value.clone())))
        }
        Expr::SelfRef(_) => Ok(ResolvedExpr::SelfRef),
        Expr::Tuple { items, .. } => Ok(ResolvedExpr::Tuple {
            items: items
                .iter()
                .map(|item| {
                    resolve_expression(
                        usage,
                        item,
                        stdlib_ids,
                        stdlib_feature_index,
                        stdlib_aliases,
                        local_definitions,
                        local_aliases,
                        import_aliases,
                        definition_index,
                        local_feature_index,
                        local_usage_map,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        }),
        Expr::Name(name) => resolve_expression_name(
            usage,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
        ),
        Expr::Path { .. } => resolve_expression_path(
            usage,
            expr,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
        ),
        Expr::Unary { op, expr, .. } => Ok(ResolvedExpr::Unary {
            op: op.clone(),
            expr: Box::new(resolve_expression(
                usage,
                expr,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            )?),
        }),
        Expr::Binary {
            left, op, right, ..
        } => Ok(ResolvedExpr::Binary {
            left: Box::new(resolve_expression(
                usage,
                left,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            )?),
            op: op.clone(),
            right: Box::new(resolve_expression(
                usage,
                right,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            )?),
        }),
        Expr::Call { function, args, .. } => Ok(ResolvedExpr::Call {
            function: function.clone(),
            args: args
                .iter()
                .map(|arg| {
                    resolve_expression(
                        usage,
                        arg,
                        stdlib_ids,
                        stdlib_feature_index,
                        stdlib_aliases,
                        local_definitions,
                        local_aliases,
                        import_aliases,
                        definition_index,
                        local_feature_index,
                        local_usage_map,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_expression_name(
    usage: &CollectedUsage,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Result<ResolvedExpr, Diagnostic> {
    if let Some(feature_id) = resolve_feature_reference(
        usage,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
    ) {
        let first = name
            .segments
            .last()
            .cloned()
            .unwrap_or_else(|| name.as_dot_string());
        return Ok(ResolvedExpr::FeaturePath {
            segments: vec![ResolvedPathSegment {
                name: first,
                feature_id,
            }],
        });
    }

    if let Some(feature_id) = resolve_qualified_reference(
        name,
        stdlib_ids,
        stdlib_aliases,
        local_definitions,
        local_aliases,
    ) {
        let first = name
            .segments
            .last()
            .cloned()
            .unwrap_or_else(|| name.as_dot_string());
        return Ok(ResolvedExpr::FeaturePath {
            segments: vec![ResolvedPathSegment {
                name: first,
                feature_id,
            }],
        });
    }

    if let Some(path) = resolve_qualified_expression_name_as_path(
        usage,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
    ) {
        return Ok(path);
    }

    if name.segments.len() > 1 {
        let tail = QualifiedName {
            segments: vec![name.segments.last().cloned().unwrap_or_default()],
            span: name.span.clone(),
        };
        if let Some(feature_id) = resolve_feature_reference(
            usage,
            &tail,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
        ) {
            let first = tail
                .segments
                .last()
                .cloned()
                .unwrap_or_else(|| tail.as_dot_string());
            return Ok(ResolvedExpr::FeaturePath {
                segments: vec![ResolvedPathSegment {
                    name: first,
                    feature_id,
                }],
            });
        }
    }

    if let Some(feature_id) = resolve_type_reference(
        name,
        stdlib_ids,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
    ) {
        let first = name
            .segments
            .last()
            .cloned()
            .unwrap_or_else(|| name.as_dot_string());
        return Ok(ResolvedExpr::FeaturePath {
            segments: vec![ResolvedPathSegment {
                name: first,
                feature_id,
            }],
        });
    }

    Err(Diagnostic::new(
        format!("unresolved expression name `{}`", name.as_colon_string()),
        Some(name.span.clone()),
    ))
}

#[allow(clippy::too_many_arguments)]
fn resolve_qualified_expression_name_as_path(
    usage: &CollectedUsage,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<ResolvedExpr> {
    if name.segments.len() <= 1 {
        return None;
    }

    let root_name = QualifiedName {
        segments: vec![name.segments.first()?.clone()],
        span: name.span.clone(),
    };
    let mut current_feature_id = resolve_feature_reference(
        usage,
        &root_name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
    )
    .or_else(|| {
        resolve_type_reference(
            &root_name,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        )
    })?;

    let mut bound_segments = vec![ResolvedPathSegment {
        name: root_name
            .segments
            .first()
            .cloned()
            .unwrap_or_else(|| root_name.as_dot_string()),
        feature_id: current_feature_id.clone(),
    }];

    for segment in name.segments.iter().skip(1) {
        let feature_id = resolve_feature_reference_from_feature_type(
            &current_feature_id,
            segment,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
        )?;
        bound_segments.push(ResolvedPathSegment {
            name: segment.clone(),
            feature_id: feature_id.clone(),
        });
        current_feature_id = feature_id;
    }

    Some(ResolvedExpr::FeaturePath {
        segments: bound_segments,
    })
}

#[allow(clippy::too_many_arguments)]
fn resolve_expression_path(
    usage: &CollectedUsage,
    expr: &Expr,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Result<ResolvedExpr, Diagnostic> {
    let (root, segments, span) = flatten_expression_path(expr).ok_or_else(|| {
        Diagnostic::new(
            "expression path must be rooted in `self` or a feature name",
            None,
        )
    })?;

    let mut bound_segments = Vec::new();
    let (mut current_feature_id, mut current_type_id) = match root {
        ExpressionPathRoot::SelfRef => (None, None),
        ExpressionPathRoot::Name(name) => {
            let feature_id = resolve_feature_reference(
                usage,
                &name,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            )
            .ok_or_else(|| {
                Diagnostic::new(
                    format!("unresolved expression root `{}`", name.as_colon_string()),
                    Some(name.span.clone()),
                )
            })?;
            let first_name = name
                .segments
                .last()
                .cloned()
                .unwrap_or_else(|| name.as_dot_string());
            bound_segments.push(ResolvedPathSegment {
                name: first_name,
                feature_id: feature_id.clone(),
            });
            (Some(feature_id), None)
        }
        ExpressionPathRoot::CastType(name) => {
            let type_id = resolve_type_reference(
                &name,
                stdlib_ids,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
            )
            .or_else(|| {
                resolve_feature_reference(
                    usage,
                    &name,
                    stdlib_ids,
                    stdlib_feature_index,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    import_aliases,
                    definition_index,
                    local_feature_index,
                    local_usage_map,
                )
                .and_then(|feature_id| {
                    infer_usage_type_from_feature_id(
                        &feature_id,
                        stdlib_ids,
                        stdlib_aliases,
                        local_definitions,
                        local_aliases,
                        import_aliases,
                        local_usage_map,
                    )
                })
            })
            .ok_or_else(|| {
                Diagnostic::new(
                    format!(
                        "unresolved expression cast type `{}`",
                        name.as_colon_string()
                    ),
                    Some(name.span.clone()),
                )
            })?;
            (None, Some(type_id))
        }
    };

    for segment in segments {
        let feature_id = if let Some(current_type_id) = &current_type_id {
            resolve_feature_reference_from_type_id(
                current_type_id,
                &segment,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
            )
        } else if let Some(current_feature_id) = &current_feature_id {
            resolve_feature_reference_from_feature_type(
                current_feature_id,
                &segment,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            )
        } else {
            let qualified = QualifiedName {
                segments: vec![segment.clone()],
                span: span.clone(),
            };
            resolve_feature_reference(
                usage,
                &qualified,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            )
        }
        .ok_or_else(|| {
            Diagnostic::new(
                format!("unresolved expression path segment `{segment}`"),
                Some(span.clone()),
            )
        })?;

        bound_segments.push(ResolvedPathSegment {
            name: segment.clone(),
            feature_id: feature_id.clone(),
        });
        current_feature_id = Some(feature_id);
        current_type_id = None;
    }

    Ok(ResolvedExpr::FeaturePath {
        segments: bound_segments,
    })
}

#[derive(Debug, Clone)]
enum ExpressionPathRoot {
    SelfRef,
    Name(QualifiedName),
    CastType(QualifiedName),
}

fn flatten_expression_path(expr: &Expr) -> Option<(ExpressionPathRoot, Vec<String>, SourceSpan)> {
    match expr {
        Expr::SelfRef(span) => Some((ExpressionPathRoot::SelfRef, Vec::new(), span.clone())),
        Expr::Name(name) => Some((
            ExpressionPathRoot::Name(name.clone()),
            Vec::new(),
            name.span.clone(),
        )),
        Expr::Path {
            root,
            segment,
            span,
        } => {
            let (base, mut segments, _) = flatten_expression_path(root)?;
            segments.push(segment.clone());
            Some((base, segments, span.clone()))
        }
        Expr::Call {
            function,
            args,
            span,
        } if args.len() == 1 && function.starts_with("as ") => {
            let type_name = function.strip_prefix("as ")?.trim();
            let segments = if type_name.contains("::") {
                type_name.split("::").map(str::to_string).collect()
            } else {
                type_name.split('.').map(str::to_string).collect()
            };
            Some((
                ExpressionPathRoot::CastType(QualifiedName {
                    segments,
                    span: span.clone(),
                }),
                Vec::new(),
                span.clone(),
            ))
        }
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn resolve_feature_reference(
    usage: &CollectedUsage,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    if name.segments.len() == 1
        && let Some(local) = unique_feature_modifier_alias_match_excluding(
            name.segments.first()?,
            local_feature_index,
            local_usage_map,
            &usage.qualified_name,
        )
    {
        return Some(feature_id_from_qualified_name(&local));
    }

    let mut seen_usages = BTreeSet::new();
    let mut seen_definitions = BTreeSet::new();
    resolve_feature_reference_with_seen(
        usage,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
        &mut seen_usages,
        &mut seen_definitions,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_feature_reference_with_seen(
    usage: &CollectedUsage,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    seen_usages: &mut BTreeSet<String>,
    seen_definitions: &mut BTreeSet<String>,
) -> Option<String> {
    if let Some(scoped_local) = resolve_local_scoped_feature_reference(
        usage,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
        seen_usages,
        seen_definitions,
    ) {
        return Some(scoped_local);
    }

    if let Some(scoped) = resolve_ancestor_scoped_feature_reference(
        usage,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
        seen_usages,
        seen_definitions,
    ) {
        return Some(scoped);
    }

    if let Some(scoped) = resolve_enclosing_scope_sibling_feature_reference(
        usage,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
        seen_usages,
        seen_definitions,
    ) {
        return Some(scoped);
    }

    if let Some(exact) =
        resolve_local_feature_name(name, local_aliases, import_aliases, local_feature_index)
    {
        if exact != usage.qualified_name {
            return Some(feature_id_from_qualified_name(&exact));
        }
    }

    if let Some(local) = resolve_owner_feature_name(
        &usage.owner_qualified_name,
        name,
        local_aliases,
        import_aliases,
        local_feature_index,
    ) {
        if local != usage.qualified_name {
            return Some(feature_id_from_qualified_name(&local));
        }
    }

    if let Some(ancestor_local) = resolve_enclosing_usage_feature_reference(
        &usage.owner_qualified_name,
        name,
        local_aliases,
        import_aliases,
        local_feature_index,
        local_usage_map,
    ) {
        if ancestor_local != usage.qualified_name {
            return Some(feature_id_from_qualified_name(&ancestor_local));
        }
    }

    if let Some(ancestor_inherited) = resolve_enclosing_usage_inherited_feature_reference(
        &usage.owner_qualified_name,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
        seen_usages,
        seen_definitions,
    ) {
        if ancestor_inherited != usage.qualified_name {
            return Some(feature_id_from_qualified_name(&ancestor_inherited));
        }
    }

    if let Some(local_usage) =
        resolve_local_usage_qualified_name(name, local_aliases, import_aliases, local_usage_map)
    {
        if local_usage != usage.qualified_name {
            return Some(feature_id_from_qualified_name(&local_usage));
        }
    }

    if usage.owner_construct.ends_with("Definition") {
        if let Some(inherited) = resolve_inherited_definition_feature_reference(
            &usage.owner_qualified_name,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            seen_definitions,
        ) {
            return Some(feature_id_from_qualified_name(&inherited));
        }
    }

    if let Some(type_name) = &usage.ty
        && let Some(type_id) = resolve_type_reference(
            type_name,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        )
        && let Some(definition_qualified_name) = type_id.strip_prefix("type.")
        && let Some(local) = resolve_owner_feature_name(
            definition_qualified_name,
            name,
            local_aliases,
            import_aliases,
            local_feature_index,
        )
        && local != usage.qualified_name
    {
        return Some(feature_id_from_qualified_name(&local));
    }

    if let Some(inherited) = resolve_owner_usage_feature_reference(
        &usage.owner_qualified_name,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
        seen_usages,
        seen_definitions,
    ) {
        if inherited != usage.qualified_name {
            return Some(normalize_feature_target_id(&inherited));
        }
    }

    None
}

fn resolve_local_usage_qualified_name(
    name: &QualifiedName,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    if let Some(expanded) = expand_import_namespace_prefix(name, local_aliases, import_aliases) {
        return resolve_local_usage_qualified_name(
            &expanded,
            local_aliases,
            import_aliases,
            local_usage_map,
        );
    }

    let dotted = name.as_dot_string();
    local_usage_map.contains_key(&dotted).then_some(dotted)
}

#[allow(clippy::too_many_arguments)]
fn resolve_local_scoped_feature_reference(
    usage: &CollectedUsage,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    seen_usages: &mut BTreeSet<String>,
    seen_definitions: &mut BTreeSet<String>,
) -> Option<String> {
    if name.segments.len() < 2 {
        return None;
    }

    let scope = local_feature_index.get(&usage.owner_qualified_name)?;
    let head = name.segments.first()?;
    let scoped_target = scope.get(head)?;
    let scoped_usage = local_usage_map.get(scoped_target)?;
    let tail = QualifiedName {
        segments: name.segments[1..].to_vec(),
        span: name.span.clone(),
    };
    if let Some(local) = resolve_owner_feature_name(
        &scoped_usage.qualified_name,
        &tail,
        local_aliases,
        import_aliases,
        local_feature_index,
    ) {
        return Some(feature_id_from_qualified_name(&local));
    }
    if let Some(inherited) = resolve_owner_usage_feature_reference(
        &scoped_usage.qualified_name,
        &tail,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
        seen_usages,
        seen_definitions,
    ) {
        return Some(feature_id_from_qualified_name(&inherited));
    }
    resolve_feature_reference_with_seen(
        scoped_usage,
        &tail,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
        seen_usages,
        seen_definitions,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_ancestor_scoped_feature_reference(
    usage: &CollectedUsage,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    seen_usages: &mut BTreeSet<String>,
    seen_definitions: &mut BTreeSet<String>,
) -> Option<String> {
    if name.segments.len() < 2 {
        return None;
    }

    let mut owner_cursor = usage.owner_qualified_name.clone();
    while let Some(owner_usage) = local_usage_map.get(&owner_cursor) {
        if name.segments.first() == Some(&owner_usage.declared_name) {
            let tail = QualifiedName {
                segments: name.segments[1..].to_vec(),
                span: name.span.clone(),
            };
            return resolve_feature_reference_with_seen(
                owner_usage,
                &tail,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
                seen_usages,
                seen_definitions,
            );
        }
        owner_cursor = owner_usage.owner_qualified_name.clone();
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn resolve_enclosing_scope_sibling_feature_reference(
    usage: &CollectedUsage,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    seen_usages: &mut BTreeSet<String>,
    seen_definitions: &mut BTreeSet<String>,
) -> Option<String> {
    if name.segments.len() < 2 {
        return None;
    }

    let head = QualifiedName {
        segments: vec![name.segments.first()?.clone()],
        span: name.span.clone(),
    };
    let tail = QualifiedName {
        segments: name.segments[1..].to_vec(),
        span: name.span.clone(),
    };

    let mut scope_cursor = usage.owner_qualified_name.clone();
    loop {
        let mut scoped_target = resolve_owner_feature_name(
            &scope_cursor,
            &head,
            local_aliases,
            import_aliases,
            local_feature_index,
        );
        if scoped_target.is_none()
            && let Some(scope_usage) = local_usage_map.get(&scope_cursor)
            && let Some(inherited) = resolve_owner_usage_feature_reference(
                &scope_usage.qualified_name,
                &head,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
                seen_usages,
                seen_definitions,
            )
        {
            scoped_target = inherited
                .strip_prefix("feature.")
                .map(str::to_string)
                .or_else(|| (!inherited.contains("::")).then_some(inherited));
        }

        if let Some(scoped_target) = scoped_target {
            let scoped_usage = local_usage_map.get(&scoped_target)?;
            if let Some(local) = resolve_owner_feature_name(
                &scoped_usage.qualified_name,
                &tail,
                local_aliases,
                import_aliases,
                local_feature_index,
            ) {
                return Some(feature_id_from_qualified_name(&local));
            }
            if let Some(inherited) = resolve_owner_usage_feature_reference(
                &scoped_usage.qualified_name,
                &tail,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
                seen_usages,
                seen_definitions,
            ) {
                return Some(normalize_feature_target_id(&inherited));
            }
            if let Some(resolved) = resolve_feature_reference_with_seen(
                scoped_usage,
                &tail,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
                seen_usages,
                seen_definitions,
            ) {
                return Some(resolved);
            }
        }

        let Some(owner_usage) = local_usage_map.get(&scope_cursor) else {
            break;
        };
        scope_cursor = owner_usage.owner_qualified_name.clone();
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn resolve_redefinition_feature_reference(
    usage: &CollectedUsage,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    let mut seen_usages = BTreeSet::new();
    let mut seen_definitions = BTreeSet::new();
    resolve_redefinition_feature_reference_with_seen(
        usage,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
        &mut seen_usages,
        &mut seen_definitions,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_redefinition_feature_reference_with_seen(
    usage: &CollectedUsage,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    seen_usages: &mut BTreeSet<String>,
    seen_definitions: &mut BTreeSet<String>,
) -> Option<String> {
    if name.segments.len() == 1
        && let Some(local) = unique_feature_modifier_alias_match_excluding(
            name.segments.first()?,
            local_feature_index,
            local_usage_map,
            &usage.qualified_name,
        )
    {
        return Some(feature_id_from_qualified_name(&local));
    }

    if usage.owner_construct.ends_with("Definition") {
        if let Some(inherited) = resolve_inherited_definition_feature_reference(
            &usage.owner_qualified_name,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            seen_definitions,
        ) {
            return Some(normalize_feature_target_id(&inherited));
        }
    }

    if let Some(target) = resolve_feature_reference_with_seen(
        usage,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
        seen_usages,
        seen_definitions,
    ) {
        return Some(target);
    }

    let mut owner_cursor = usage.owner_qualified_name.clone();
    let mut owner_seen_usages = BTreeSet::new();
    let mut owner_seen_definitions = BTreeSet::new();
    while let Some(owner_usage) = local_usage_map.get(&owner_cursor) {
        if let Some(type_name) = &owner_usage.ty
            && let Some(type_id) = resolve_type_reference_in_scope(
                type_name,
                &owner_usage.owner_qualified_name,
                stdlib_ids,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
            )
            && let Some(definition_qualified_name) = type_id.strip_prefix("type.")
            && let Some(local) = resolve_owner_feature_name(
                definition_qualified_name,
                name,
                local_aliases,
                import_aliases,
                local_feature_index,
            )
        {
            return Some(feature_id_from_qualified_name(&local));
        }
        if let Some(type_name) = &owner_usage.ty
            && let Some(type_id) = resolve_type_reference_in_scope(
                type_name,
                &owner_usage.owner_qualified_name,
                stdlib_ids,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
            )
            && let Some(definition_qualified_name) = type_id.strip_prefix("type.")
            && let Some(local) = resolve_owner_feature_modifier_alias(
                definition_qualified_name,
                name,
                local_feature_index,
                local_usage_map,
            )
        {
            return Some(feature_id_from_qualified_name(&local));
        }
        if let Some(inherited) = resolve_owner_usage_feature_reference(
            &owner_usage.qualified_name,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
            &mut owner_seen_usages,
            &mut owner_seen_definitions,
        ) {
            return Some(normalize_feature_target_id(&inherited));
        }
        owner_cursor = owner_usage.owner_qualified_name.clone();
    }

    if name.segments.len() == 1 {
        if let Some(local) = unique_definition_owned_feature_match_excluding(
            name.segments.first()?,
            local_feature_index,
            definition_index,
            &usage.qualified_name,
        ) {
            return Some(feature_id_from_qualified_name(&local));
        }
        if let Some(local) = unique_feature_match_excluding(
            name.segments.first()?,
            local_feature_index,
            &usage.qualified_name,
        ) {
            return Some(feature_id_from_qualified_name(&local));
        }
        if let Some(local) = unique_feature_modifier_alias_match_excluding(
            name.segments.first()?,
            local_feature_index,
            local_usage_map,
            &usage.qualified_name,
        ) {
            return Some(feature_id_from_qualified_name(&local));
        }
        if let Some(base_feature) =
            semantic_base_feature_fallback(name.segments.first()?, stdlib_ids)
        {
            return Some(base_feature);
        }
        unique_suffix_match(name.segments.first()?, stdlib_ids)
    } else {
        resolve_qualified_reference(
            name,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
        )
    }
}

fn semantic_base_feature_fallback(name: &str, stdlib_ids: &[String]) -> Option<String> {
    let target = match name {
        "mRefs" => "MeasurementReferences::TensorMeasurementReference::mRefs",
        "num" => "Quantities::TensorQuantityValue::num",
        "planeAngle" => "ISQSpaceTime::angularMeasure",
        "quantityPowerFactors" => "Quantities::QuantityDimension::quantityPowerFactors",
        "coordinateFrame" => "SpatialItems::SpatialItem::coordinateFrame",
        "shape" => "Items::Item::shape",
        "elements" => "Occurrences::Occurrence::differencesOf::elements",
        "radius" => "ShapeItems::CircularCylinder::radius",
        "transformation" => "MeasurementReferences::CoordinateFrame::transformation",
        _ => return None,
    };
    stdlib_ids
        .iter()
        .any(|id| id == target)
        .then(|| target.to_string())
}

fn resolve_local_feature_name(
    name: &QualifiedName,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
) -> Option<String> {
    if let Some(expanded) = expand_import_namespace_prefix(name, local_aliases, import_aliases) {
        return resolve_local_feature_name(
            &expanded,
            local_aliases,
            import_aliases,
            local_feature_index,
        );
    }

    let dotted = name.as_dot_string();
    if let Some(imported) = import_aliases.value_aliases.get(&dotted)
        && imported.starts_with("feature.")
    {
        return imported.strip_prefix("feature.").map(str::to_string);
    }
    if let Some(exact) = unique_feature_match(&dotted, local_feature_index) {
        return Some(exact);
    }

    if name.segments.len() == 1 {
        let simple = name.segments.first()?;
        if let Some(imported) = import_aliases.value_aliases.get(simple)
            && imported.starts_with("feature.")
        {
            return imported.strip_prefix("feature.").map(str::to_string);
        }
        unique_feature_match(simple, local_feature_index)
    } else {
        None
    }
}

fn resolve_owner_feature_name(
    owner_qualified_name: &str,
    name: &QualifiedName,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
) -> Option<String> {
    let expanded = expand_import_namespace_prefix(name, local_aliases, import_aliases);
    let resolved = expanded.as_ref().unwrap_or(name);
    let scope = local_feature_index.get(owner_qualified_name)?;
    if resolved.segments.len() == 1 {
        scope.get(resolved.segments.first()?).cloned()
    } else {
        let dotted = resolved.as_dot_string();
        scope
            .values()
            .find(|qualified| *qualified == &dotted)
            .cloned()
    }
}

fn resolve_owner_feature_modifier_alias(
    owner_qualified_name: &str,
    name: &QualifiedName,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    if name.segments.len() != 1 {
        return None;
    }
    let alias = name.segments.first()?;
    local_feature_index
        .get(owner_qualified_name)?
        .values()
        .find(|qualified_name| {
            local_usage_map
                .get(*qualified_name)
                .is_some_and(|usage| usage.modifiers.iter().any(|modifier| modifier == alias))
        })
        .cloned()
}

fn resolve_enclosing_usage_feature_reference(
    owner_qualified_name: &str,
    name: &QualifiedName,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    let mut cursor = owner_qualified_name.to_string();
    while let Some(owner_usage) = local_usage_map.get(&cursor) {
        if let Some(local) = resolve_owner_feature_name(
            &owner_usage.qualified_name,
            name,
            local_aliases,
            import_aliases,
            local_feature_index,
        ) {
            return Some(local);
        }
        cursor = owner_usage.owner_qualified_name.clone();
    }
    resolve_owner_feature_name(
        &cursor,
        name,
        local_aliases,
        import_aliases,
        local_feature_index,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_enclosing_usage_inherited_feature_reference(
    owner_qualified_name: &str,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    seen_usages: &mut BTreeSet<String>,
    seen_definitions: &mut BTreeSet<String>,
) -> Option<String> {
    let mut cursor = owner_qualified_name.to_string();
    while let Some(owner_usage) = local_usage_map.get(&cursor) {
        if let Some(inherited) = resolve_owner_usage_feature_reference(
            &owner_usage.qualified_name,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
            seen_usages,
            seen_definitions,
        ) {
            return Some(inherited);
        }
        cursor = owner_usage.owner_qualified_name.clone();
    }
    resolve_inherited_definition_feature_reference(
        &cursor,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        seen_definitions,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_inherited_definition_feature_reference(
    owner_qualified_name: &str,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    seen: &mut BTreeSet<String>,
) -> Option<String> {
    if !seen.insert(owner_qualified_name.to_string()) {
        return None;
    }

    let definition = definition_index.get(owner_qualified_name)?;
    for parent in &definition.specializes {
        let Some(parent_id) = resolve_type_reference(
            parent,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        ) else {
            continue;
        };
        let Some(parent_qualified_name) = parent_id.strip_prefix("type.") else {
            if let Some(inherited) =
                resolve_stdlib_owned_feature_reference(&parent_id, name, stdlib_feature_index)
            {
                return Some(inherited);
            }
            continue;
        };
        if let Some(local) = resolve_owner_feature_name(
            parent_qualified_name,
            name,
            local_aliases,
            import_aliases,
            local_feature_index,
        ) {
            return Some(local);
        }
        if let Some(inherited) = resolve_inherited_definition_feature_reference(
            parent_qualified_name,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            seen,
        ) {
            return Some(inherited);
        }
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn resolve_owner_usage_feature_reference(
    owner_qualified_name: &str,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    seen_usages: &mut BTreeSet<String>,
    seen_definitions: &mut BTreeSet<String>,
) -> Option<String> {
    if !seen_usages.insert(owner_qualified_name.to_string()) {
        return None;
    }

    let owner_usage = local_usage_map.get(owner_qualified_name)?;
    let mut candidate_definitions = BTreeSet::new();
    let mut stdlib_owner_ids = BTreeSet::new();
    if let Some(stdlib_owner) = usage_construct_stdlib_owner(&owner_usage.construct) {
        stdlib_owner_ids.insert(stdlib_owner.to_string());
    }

    if let Some(type_name) = &owner_usage.ty
        && let Some(type_id) = resolve_type_reference(
            type_name,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        )
    {
        if let Some(local_definition) = type_id.strip_prefix("type.") {
            candidate_definitions.insert(local_definition.to_string());
        } else {
            stdlib_owner_ids.insert(type_id);
        }
    }

    if let Some(type_id) = infer_named_definition_type_ref(owner_usage, local_definitions)
        && let Some(local_definition) = type_id.strip_prefix("type.")
    {
        candidate_definitions.insert(local_definition.to_string());
    }

    if let Some(type_qualified_name) = resolve_collected_usage_type_qualified_name(
        owner_usage,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
        seen_usages,
        seen_definitions,
    ) {
        candidate_definitions.insert(type_qualified_name);
    }

    for target_name in &owner_usage.redefines {
        let Some(target_id) = resolve_redefinition_feature_reference_with_seen(
            owner_usage,
            target_name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
            seen_usages,
            seen_definitions,
        ) else {
            continue;
        };
        let Some(target_usage) = target_id
            .strip_prefix("feature.")
            .and_then(|qualified_name| local_usage_map.get(qualified_name))
        else {
            if let Some(inherited) =
                resolve_stdlib_owned_feature_reference(&target_id, name, stdlib_feature_index)
            {
                return Some(inherited);
            }
            continue;
        };
        if let Some(local) = resolve_owner_feature_name(
            &target_usage.qualified_name,
            name,
            local_aliases,
            import_aliases,
            local_feature_index,
        ) {
            return Some(local);
        }
        if let Some(inherited) = resolve_owner_usage_feature_reference(
            &target_usage.qualified_name,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
            seen_usages,
            seen_definitions,
        ) {
            return Some(inherited);
        }
    }

    for target_name in owner_usage
        .subsets
        .iter()
        .chain(owner_usage.specializes.iter())
    {
        let Some(target_id) = resolve_feature_reference_with_seen(
            owner_usage,
            target_name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
            seen_usages,
            seen_definitions,
        ) else {
            continue;
        };
        let Some(target_usage) = target_id
            .strip_prefix("feature.")
            .and_then(|qualified_name| local_usage_map.get(qualified_name))
        else {
            if let Some(inherited) =
                resolve_stdlib_owned_feature_reference(&target_id, name, stdlib_feature_index)
            {
                return Some(inherited);
            }
            continue;
        };
        if let Some(local) = resolve_owner_feature_name(
            &target_usage.qualified_name,
            name,
            local_aliases,
            import_aliases,
            local_feature_index,
        ) {
            return Some(local);
        }
        if let Some(inherited) = resolve_owner_usage_feature_reference(
            &target_usage.qualified_name,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
            seen_usages,
            seen_definitions,
        ) {
            return Some(inherited);
        }
    }

    for definition_qualified_name in candidate_definitions {
        if let Some(local) = resolve_owner_feature_name(
            &definition_qualified_name,
            name,
            local_aliases,
            import_aliases,
            local_feature_index,
        ) {
            return Some(local);
        }
        if let Some(local) = resolve_owner_feature_modifier_alias(
            &definition_qualified_name,
            name,
            local_feature_index,
            local_usage_map,
        ) {
            return Some(local);
        }
        if let Some(inherited) = resolve_inherited_definition_feature_reference(
            &definition_qualified_name,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            seen_definitions,
        ) {
            return Some(inherited);
        }
        let stdlib_owner = format!("type.{definition_qualified_name}");
        if let Some(inherited) =
            resolve_stdlib_owned_feature_reference(&stdlib_owner, name, stdlib_feature_index)
        {
            return Some(inherited);
        }
    }

    for owner_type_id in stdlib_owner_ids {
        if let Some(inherited) =
            resolve_stdlib_owned_feature_reference(&owner_type_id, name, stdlib_feature_index)
        {
            return Some(inherited);
        }
    }

    None
}

fn resolve_stdlib_owned_feature_reference(
    owner_type_id: &str,
    name: &QualifiedName,
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
) -> Option<String> {
    if name.segments.len() != 1 {
        return None;
    }
    let feature_name = name.segments.first()?;
    let owner_type_id = owner_type_id
        .strip_prefix("type.")
        .or_else(|| owner_type_id.strip_prefix("feature."))
        .unwrap_or(owner_type_id);
    stdlib_feature_index
        .get(owner_type_id)
        .and_then(|features| features.get(feature_name))
        .cloned()
}

fn unique_feature_match(
    dotted_name: &str,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
) -> Option<String> {
    let matches = local_feature_index
        .values()
        .flat_map(BTreeMap::values)
        .filter(|qualified_name| {
            *qualified_name == dotted_name || qualified_name.ends_with(&format!(".{dotted_name}"))
        })
        .cloned()
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn unique_feature_match_excluding(
    dotted_name: &str,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    excluded_qualified_name: &str,
) -> Option<String> {
    let matches = local_feature_index
        .values()
        .flat_map(BTreeMap::values)
        .filter(|qualified_name| {
            qualified_name.as_str() != excluded_qualified_name
                && (*qualified_name == dotted_name
                    || qualified_name.ends_with(&format!(".{dotted_name}")))
        })
        .cloned()
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn unique_definition_owned_feature_match_excluding(
    dotted_name: &str,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    excluded_qualified_name: &str,
) -> Option<String> {
    let matches = local_feature_index
        .values()
        .flat_map(BTreeMap::values)
        .filter(|qualified_name| {
            qualified_name.as_str() != excluded_qualified_name
                && (*qualified_name == dotted_name
                    || qualified_name.ends_with(&format!(".{dotted_name}")))
                && qualified_name
                    .rsplit_once('.')
                    .is_some_and(|(owner, _)| definition_index.contains_key(owner))
        })
        .cloned()
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn unique_feature_modifier_alias_match_excluding(
    alias: &str,
    _local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    excluded_qualified_name: &str,
) -> Option<String> {
    let matches = local_usage_map
        .values()
        .filter(|usage| usage.qualified_name != excluded_qualified_name)
        .filter(|usage| usage.modifiers.iter().any(|modifier| modifier == alias))
        .map(|usage| usage.qualified_name.clone())
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn usage_construct_stdlib_owner(construct: &str) -> Option<&'static str> {
    match construct {
        "SendUsage" => Some("Actions::SendAction"),
        "AcceptActionUsage" => Some("Actions::AcceptAction"),
        _ => None,
    }
}

fn feature_id_from_qualified_name(qualified_name: &str) -> String {
    format!("feature.{qualified_name}")
}

fn resolve_comment_annotation_target(
    usage: &CollectedUsage,
    name: &QualifiedName,
    local_definitions: &BTreeMap<String, String>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    let target_name = name.as_dot_string();
    for candidate in scoped_reference_candidates(&target_name, &usage.owner_qualified_name) {
        if let Some(target_usage) = local_usage_map.get(&candidate) {
            return Some(collected_usage_element_id(target_usage));
        }
        if let Some(target_definition) = local_definitions.get(&candidate) {
            return Some(target_definition.clone());
        }
    }

    if usage.owner_qualified_name == target_name
        || usage
            .owner_qualified_name
            .strip_prefix(&target_name)
            .is_some_and(|rest| rest.starts_with('.'))
    {
        return Some(format!("pkg.{target_name}"));
    }

    None
}

fn scoped_reference_candidates(target_name: &str, owner_qualified_name: &str) -> Vec<String> {
    if target_name.contains('.') {
        return vec![target_name.to_string()];
    }

    let mut candidates = Vec::new();
    let mut scope = Some(owner_qualified_name);
    while let Some(current) = scope {
        candidates.push(format!("{current}.{target_name}"));
        scope = current.rsplit_once('.').map(|(parent, _)| parent);
    }
    candidates.push(target_name.to_string());
    candidates
}

fn collected_usage_element_id(usage: &CollectedUsage) -> String {
    match usage.construct.as_str() {
        "CommentUsage" => format!(
            "comment.{}.{}.{}.{}",
            usage.owner_qualified_name,
            usage.declared_name,
            usage.span.start_line,
            usage.span.start_col
        ),
        _ => feature_id_from_qualified_name(&usage.qualified_name),
    }
}

fn normalize_feature_target_id(target: &str) -> String {
    if target.starts_with("feature.") || target.contains("::") {
        target.to_string()
    } else {
        feature_id_from_qualified_name(target)
    }
}

fn infer_named_definition_type_ref(
    usage: &CollectedUsage,
    local_definitions: &BTreeMap<String, String>,
) -> Option<String> {
    if usage.ty.is_some() || usage.declared_name.is_empty() {
        return None;
    }

    let action_like = matches!(
        usage.construct.as_str(),
        "ActionUsage" | "PerformActionUsage" | "AcceptActionUsage"
    );
    if !action_like {
        return None;
    }

    let mut candidates = vec![usage.declared_name.clone()];
    let mut chars = usage.declared_name.chars();
    if let Some(first) = chars.next() {
        let mut pascal = String::new();
        pascal.extend(first.to_uppercase());
        pascal.push_str(chars.as_str());
        if pascal != usage.declared_name {
            candidates.push(pascal);
        }
    }

    candidates
        .into_iter()
        .find_map(|candidate| local_definitions.get(&candidate).cloned())
}

fn is_self_feature_reference(usage: &CollectedUsage, name: &QualifiedName) -> bool {
    let dotted = name.as_dot_string();
    dotted == usage.declared_name || dotted == usage.qualified_name
}

#[allow(clippy::too_many_arguments)]
fn resolve_collected_usage_type_qualified_name(
    usage: &CollectedUsage,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    seen_usages: &mut BTreeSet<String>,
    seen_definitions: &mut BTreeSet<String>,
) -> Option<String> {
    if let Some(type_name) = &usage.ty {
        return resolve_type_reference(
            type_name,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        )
        .and_then(|target| target.strip_prefix("type.").map(str::to_string));
    }

    if let Some(target_name) = &usage.reference_target
        && let Some(target_id) = resolve_reference_usage_target(
            usage,
            target_name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
        )
        && let Some(target_type_id) = infer_usage_type_from_feature_id(
            &target_id,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            local_usage_map,
        )
    {
        return target_type_id.strip_prefix("type.").map(str::to_string);
    }

    if !usage.declared_name.is_empty() {
        let inferred_name = QualifiedName {
            segments: vec![usage.declared_name.clone()],
            span: usage.span.clone(),
        };
        if let Some(target_id) = resolve_feature_reference(
            usage,
            &inferred_name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
        ) && let Some(target_type_id) = infer_usage_type_from_feature_id(
            &target_id,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            local_usage_map,
        ) {
            return target_type_id.strip_prefix("type.").map(str::to_string);
        }
    }

    for name in &usage.redefines {
        let target = resolve_redefinition_feature_reference_with_seen(
            usage,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
            seen_usages,
            seen_definitions,
        )?;
        if let Some(target_usage) = target
            .strip_prefix("feature.")
            .and_then(|qualified_name| local_usage_map.get(qualified_name))
        {
            if !seen_usages.insert(target_usage.qualified_name.clone()) {
                continue;
            }
            if let Some(type_qualified_name) = resolve_collected_usage_type_qualified_name(
                target_usage,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
                seen_usages,
                seen_definitions,
            ) {
                return Some(type_qualified_name);
            }
        }
    }

    for name in usage.subsets.iter().chain(usage.specializes.iter()) {
        let Some(target) = resolve_feature_reference_with_seen(
            usage,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
            seen_usages,
            seen_definitions,
        ) else {
            continue;
        };
        let Some(target_usage) = target
            .strip_prefix("feature.")
            .and_then(|qualified_name| local_usage_map.get(qualified_name))
        else {
            continue;
        };
        if !seen_usages.insert(target_usage.qualified_name.clone()) {
            continue;
        }
        if let Some(type_qualified_name) = resolve_collected_usage_type_qualified_name(
            target_usage,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
            seen_usages,
            seen_definitions,
        ) {
            return Some(type_qualified_name);
        }
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn inferred_usage_type_ref(
    usage: &CollectedUsage,
    redefined_features: &[String],
    subsetted_features: &[String],
    specialized_features: &[String],
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    redefined_features
        .iter()
        .chain(subsetted_features.iter())
        .chain(specialized_features.iter())
        .find_map(|feature_id| {
            let target = feature_id.strip_prefix("feature.")?;
            let target_usage = local_usage_map.get(target)?;
            if let Some(target_type) = target_usage.ty.as_ref() {
                return resolve_type_reference(
                    target_type,
                    stdlib_ids,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    import_aliases,
                );
            }

            let mut seen_usages = BTreeSet::from([usage.qualified_name.clone()]);
            let mut seen_definitions = BTreeSet::new();
            resolve_collected_usage_type_qualified_name(
                target_usage,
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
                &mut seen_usages,
                &mut seen_definitions,
            )
            .map(|qualified_name| format!("type.{qualified_name}"))
        })
}

fn infer_usage_type_from_feature_id(
    feature_id: &str,
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    let target = feature_id.strip_prefix("feature.")?;
    let usage = local_usage_map.get(target)?;
    let ty = usage.ty.as_ref()?;
    resolve_type_reference_in_scope(
        ty,
        &usage.owner_qualified_name,
        stdlib_ids,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_feature_reference_from_type_id(
    type_id: &str,
    segment: &str,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
) -> Option<String> {
    let name = QualifiedName {
        segments: vec![segment.to_string()],
        span: SourceSpan {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        },
    };

    if let Some(qualified_name) = type_id.strip_prefix("type.") {
        if let Some(local) = resolve_owner_feature_name(
            qualified_name,
            &name,
            local_aliases,
            import_aliases,
            local_feature_index,
        ) {
            return Some(feature_id_from_qualified_name(&local));
        }

        let mut seen = BTreeSet::new();
        return resolve_inherited_definition_feature_reference(
            qualified_name,
            &name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            &mut seen,
        )
        .map(|qualified| feature_id_from_qualified_name(&qualified));
    }

    resolve_stdlib_owned_feature_reference(type_id, &name, stdlib_feature_index)
}

#[allow(clippy::too_many_arguments)]
fn resolve_feature_reference_from_feature_type(
    feature_id: &str,
    segment: &str,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    if let Some(target_qualified_name) = feature_id.strip_prefix("feature.") {
        let name = QualifiedName {
            segments: vec![segment.to_string()],
            span: SourceSpan {
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
            },
        };
        if let Some(local) = resolve_owner_feature_name(
            target_qualified_name,
            &name,
            local_aliases,
            import_aliases,
            local_feature_index,
        ) {
            return Some(feature_id_from_qualified_name(&local));
        }
    }

    let type_id = infer_usage_type_from_feature_id(
        feature_id,
        stdlib_ids,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        local_usage_map,
    );
    if type_id.is_none()
        && let Some(target_id) = resolve_usage_feature_type_from_feature_id(
            feature_id,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
        )
        && let Some(target_qualified_name) = target_id.strip_prefix("feature.")
    {
        let name = QualifiedName {
            segments: vec![segment.to_string()],
            span: SourceSpan {
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
            },
        };

        if let Some(local) = resolve_owner_feature_name(
            target_qualified_name,
            &name,
            local_aliases,
            import_aliases,
            local_feature_index,
        ) {
            return Some(feature_id_from_qualified_name(&local));
        }

        let mut seen_usages = BTreeSet::new();
        let mut seen_definitions = BTreeSet::new();
        if let Some(inherited) = resolve_owner_usage_feature_reference(
            target_qualified_name,
            &name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
            &mut seen_usages,
            &mut seen_definitions,
        ) {
            return Some(normalize_feature_target_id(&inherited));
        }
    }

    resolve_feature_reference_from_type_id(
        &type_id?,
        segment,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_usage_feature_type_from_feature_id(
    feature_id: &str,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    let target = feature_id.strip_prefix("feature.")?;
    let usage = local_usage_map.get(target)?;
    let ty = usage.ty.as_ref()?;
    resolve_feature_reference(
        usage,
        ty,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_reference_usage_target(
    usage: &CollectedUsage,
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    let mut scoped_usage = usage.clone();
    while !matches!(
        scoped_usage.owner_construct.as_str(),
        "Package" | "PartDefinition" | "PartUsage" | "ActionUsage" | "PerformActionUsage"
    ) && !scoped_usage.owner_construct.ends_with("Definition")
    {
        let Some(owner_usage) = local_usage_map.get(&scoped_usage.owner_qualified_name) else {
            break;
        };
        scoped_usage.owner_construct = owner_usage.owner_construct.clone();
        scoped_usage.owner_qualified_name = owner_usage.owner_qualified_name.clone();
    }

    if let Some(target) = resolve_feature_reference(
        &scoped_usage,
        name,
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
    ) {
        return Some(target);
    }

    let mut namespace_cursor = scoped_usage.owner_qualified_name.clone();
    while let Some((parent, _)) = namespace_cursor.rsplit_once('.') {
        namespace_cursor = parent.to_string();
        if let Some(local) = resolve_owner_feature_name(
            &namespace_cursor,
            name,
            local_aliases,
            import_aliases,
            local_feature_index,
        ) {
            return Some(feature_id_from_qualified_name(&local));
        }
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn resolve_allocation_endpoint(
    usage: &CollectedUsage,
    name: &QualifiedName,
    prefer_type: bool,
    stdlib_ids: &[String],
    stdlib_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_feature_index: &BTreeMap<String, BTreeMap<String, String>>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    let type_ref = || {
        resolve_type_reference_in_scope(
            name,
            &usage.owner_qualified_name,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        )
    };
    let feature_ref = || {
        resolve_reference_usage_target(
            usage,
            name,
            stdlib_ids,
            stdlib_feature_index,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
            definition_index,
            local_feature_index,
            local_usage_map,
        )
    };

    if prefer_type {
        type_ref().or_else(feature_ref)
    } else {
        feature_ref().or_else(type_ref)
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_connection_end_specialization(
    usage: &CollectedUsage,
    parent_construct: Option<&str>,
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
    definition_index: &BTreeMap<String, CollectedDefinition>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    let parent_usage = local_usage_map.get(&usage.owner_qualified_name)?;
    if parent_construct.is_some_and(|construct| parent_usage.construct != construct) {
        return None;
    }

    let parent_type_name = parent_usage.ty.as_ref()?;
    let parent_type_id = resolve_type_reference(
        parent_type_name,
        stdlib_ids,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
    )?;
    let parent_definition = definition_index.get(parent_type_id.strip_prefix("type.")?)?;
    let member = parent_definition
        .members
        .iter()
        .find(|member| member.declared_name == usage.declared_name)?;
    Some(format!("feature.{}", member.qualified_name))
}

fn resolve_import_target(
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
) -> Option<String> {
    let as_colon = name.as_colon_string();
    if as_colon.contains('*') {
        return Some(as_colon);
    }

    resolve_qualified_reference(
        name,
        stdlib_ids,
        stdlib_aliases,
        local_definitions,
        local_aliases,
    )
    .or_else(|| {
        if name.segments.len() > 1 {
            unique_suffix_match(name.segments.last()?, stdlib_ids)
        } else {
            None
        }
    })
}

fn resolve_type_reference(
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
) -> Option<String> {
    if let Some(unconjugated) = unconjugated_type_name(name) {
        return resolve_type_reference(
            &unconjugated,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        );
    }

    if name.segments.len() == 1 {
        let simple = &name.segments[0];
        if let Some(local) = local_definitions.get(simple) {
            return Some(local.clone());
        }
        if let Some(alias_target) = local_aliases.get(simple) {
            return resolve_type_reference(
                alias_target,
                stdlib_ids,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
            );
        }
        if let Some(imported) = import_aliases.value_aliases.get(simple) {
            return Some(imported.clone());
        }
        if let Some(alias) = stdlib_aliases.get(simple) {
            return Some(alias.clone());
        }
        return unique_suffix_match(simple, stdlib_ids);
    }

    if let Some(expanded) = expand_import_namespace_prefix(name, local_aliases, import_aliases) {
        return resolve_type_reference(
            &expanded,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        );
    }

    if let Some(alias_target) = local_aliases.get(&name.as_dot_string()) {
        return resolve_type_reference(
            alias_target,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        );
    }

    if let Some(alias_target) =
        unique_local_alias_suffix_match(&name.as_dot_string(), local_aliases)
    {
        return resolve_type_reference(
            &alias_target,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        );
    }

    if let Some(imported) = import_aliases.value_aliases.get(&name.as_dot_string()) {
        return Some(imported.clone());
    }

    resolve_qualified_reference(
        name,
        stdlib_ids,
        stdlib_aliases,
        local_definitions,
        local_aliases,
    )
}

fn resolve_type_reference_in_scope(
    name: &QualifiedName,
    owner_qualified_name: &str,
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
) -> Option<String> {
    if let Some(unconjugated) = unconjugated_type_name(name) {
        return resolve_type_reference_in_scope(
            &unconjugated,
            owner_qualified_name,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        );
    }

    resolve_scoped_local_type_reference(name, owner_qualified_name, local_definitions)
        .or_else(|| resolve_scoped_import_value_alias(name, owner_qualified_name, import_aliases))
        .or_else(|| {
            resolve_visible_type_reference(
                name,
                stdlib_ids,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
            )
        })
}

fn resolve_scoped_import_value_alias(
    name: &QualifiedName,
    owner_qualified_name: &str,
    import_aliases: &ImportAliases,
) -> Option<String> {
    if name.segments.len() != 1 {
        return None;
    }

    let simple = name.segments.first()?;
    let mut cursor = owner_qualified_name;
    loop {
        let key = format!("{cursor}.{simple}");
        if let Some(imported) = import_aliases.value_aliases.get(&key) {
            return Some(imported.clone());
        }
        let Some((parent, _)) = cursor.rsplit_once('.') else {
            break;
        };
        cursor = parent;
    }
    None
}

fn resolve_visible_type_reference(
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
) -> Option<String> {
    if name.segments.len() == 1 {
        let simple = &name.segments[0];
        if let Some(alias_target) = local_aliases.get(simple) {
            return resolve_visible_type_reference(
                alias_target,
                stdlib_ids,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
            );
        }
        if let Some(imported) = import_aliases.value_aliases.get(simple) {
            return Some(imported.clone());
        }
        if let Some(alias) = stdlib_aliases.get(simple) {
            return Some(alias.clone());
        }
        return unique_suffix_match(simple, stdlib_ids);
    }

    if let Some(expanded) = expand_import_namespace_prefix(name, local_aliases, import_aliases) {
        return resolve_visible_type_reference(
            &expanded,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        );
    }

    if let Some(imported) = import_aliases.value_aliases.get(&name.as_dot_string()) {
        return Some(imported.clone());
    }

    resolve_explicit_type_reference(name, stdlib_ids, stdlib_aliases, local_definitions)
}

fn unconjugated_type_name(name: &QualifiedName) -> Option<QualifiedName> {
    let first = name.segments.first()?;
    let stripped = first.strip_prefix('~')?;
    if stripped.is_empty() {
        return None;
    }
    let mut segments = name.segments.clone();
    segments[0] = stripped.to_string();
    Some(QualifiedName {
        segments,
        span: name.span.clone(),
    })
}

fn resolve_explicit_type_reference(
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
) -> Option<String> {
    let colon = name.as_colon_string();
    if let Some(alias) = stdlib_aliases.get(&colon) {
        return Some(alias.clone());
    }
    if stdlib_ids.iter().any(|id| id == &colon) {
        return Some(colon);
    }

    local_definitions.get(&name.as_dot_string()).cloned()
}

fn resolve_scoped_local_type_reference(
    name: &QualifiedName,
    owner_qualified_name: &str,
    local_definitions: &BTreeMap<String, String>,
) -> Option<String> {
    let dotted_name = name.as_dot_string();
    let mut cursor = owner_qualified_name;
    loop {
        let candidate = format!("{cursor}.{dotted_name}");
        if let Some(local) = local_definitions.get(&candidate) {
            return Some(local.clone());
        }
        let Some((parent, _)) = cursor.rsplit_once('.') else {
            break;
        };
        cursor = parent;
    }
    None
}

fn resolve_qualified_reference(
    name: &QualifiedName,
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
) -> Option<String> {
    let colon = name.as_colon_string();
    if let Some(alias) = stdlib_aliases.get(&colon) {
        return Some(alias.clone());
    }
    if stdlib_ids.iter().any(|id| id == &colon) {
        return Some(colon);
    }

    if let Some(local) = local_definitions.get(&name.as_dot_string()) {
        return Some(local.clone());
    }

    if let Some(local) = unique_local_suffix_match(&name.as_dot_string(), local_definitions) {
        return Some(local);
    }

    if let Some(alias_target) = local_aliases.get(&name.as_dot_string()) {
        return resolve_qualified_reference(
            alias_target,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
        );
    }

    if let Some(alias_target) =
        unique_local_alias_suffix_match(&name.as_dot_string(), local_aliases)
    {
        return resolve_qualified_reference(
            &alias_target,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
        );
    }

    if name.segments.len() == 1 {
        unique_suffix_match(name.segments.last()?, stdlib_ids)
    } else {
        None
    }
}

fn unique_local_alias_suffix_match(
    dotted_name: &str,
    local_aliases: &BTreeMap<String, QualifiedName>,
) -> Option<QualifiedName> {
    let matches = local_aliases
        .iter()
        .filter(|(qualified_name, _)| {
            *qualified_name == dotted_name || qualified_name.ends_with(&format!(".{dotted_name}"))
        })
        .map(|(_, target)| target.clone())
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn unique_suffix_match(name: &str, stdlib_ids: &[String]) -> Option<String> {
    let matches = stdlib_ids
        .iter()
        .filter(|id| id.rsplit("::").next() == Some(name))
        .cloned()
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn unique_local_suffix_match(
    dotted_name: &str,
    local_definitions: &BTreeMap<String, String>,
) -> Option<String> {
    let matches = local_definitions
        .iter()
        .filter(|(key, _)| *key == dotted_name || key.ends_with(&format!(".{dotted_name}")))
        .map(|(_, value)| value.clone())
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn unresolved_or_error(
    resolved: Option<String>,
    name: &QualifiedName,
    reference_kind: &str,
    policy: ResolvePolicy,
) -> Result<String, Diagnostic> {
    if let Some(resolved) = resolved {
        return Ok(resolved);
    }
    if policy.preserve_unresolved_references {
        return Ok(name.as_colon_string());
    }
    Err(Diagnostic::new(
        format!("unresolved {reference_kind} `{}`", name.as_colon_string()),
        Some(name.span.clone()),
    ))
}

#[cfg(any())]
mod tests {
    use super::*;
    use mercurio_kir::KirElement;
    use mercurio_language_contracts::ast::SourceSpan;

    use crate::lowering::emit::MappingBundle;

    #[test]
    fn expand_import_namespace_prefix_ignores_noop_expansion() {
        let span = SourceSpan {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        };
        let name = QualifiedName {
            segments: vec!["Packets".to_string(), "packet data field".to_string()],
            span: span.clone(),
        };
        let import_aliases = ImportAliases {
            value_aliases: BTreeMap::new(),
            namespace_aliases: BTreeMap::from([(
                "Packets".to_string(),
                QualifiedName {
                    segments: vec!["Packets".to_string()],
                    span,
                },
            )]),
            ambiguous_value_aliases: BTreeSet::new(),
            ambiguous_namespace_aliases: BTreeSet::new(),
        };

        assert_eq!(
            expand_import_namespace_prefix(&name, &BTreeMap::new(), &import_aliases),
            None
        );
    }

    #[test]
    fn expand_import_namespace_prefix_still_expands_real_aliases() {
        let span = SourceSpan {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        };
        let name = QualifiedName {
            segments: vec!["P".to_string(), "packet data field".to_string()],
            span: span.clone(),
        };
        let import_aliases = ImportAliases {
            value_aliases: BTreeMap::new(),
            namespace_aliases: BTreeMap::from([(
                "P".to_string(),
                QualifiedName {
                    segments: vec!["Packets".to_string()],
                    span: span.clone(),
                },
            )]),
            ambiguous_value_aliases: BTreeSet::new(),
            ambiguous_namespace_aliases: BTreeSet::new(),
        };

        assert_eq!(
            expand_import_namespace_prefix(&name, &BTreeMap::new(), &import_aliases),
            Some(QualifiedName {
                segments: vec!["Packets".to_string(), "packet data field".to_string()],
                span,
            })
        );
    }

    #[test]
    fn definition_defaults_use_semantic_specialization_without_metatype_anchor() {
        let mappings = MappingBundle::load().unwrap();
        let specializations =
            definition_specializations_with_default("ItemDefinition", &[], mappings);

        assert_eq!(specializations.len(), 1);
        assert_eq!(specializations[0].as_colon_string(), "Items::Item");
    }

    #[test]
    fn resolve_type_reference_prefers_local_definition_over_stdlib_alias() {
        let span = SourceSpan {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        };
        let name = QualifiedName {
            segments: vec!["A".to_string()],
            span,
        };
        let stdlib_aliases = BTreeMap::from([("A".to_string(), "ISQBase::ampere".to_string())]);
        let local_definitions = BTreeMap::from([("A".to_string(), "type.ItemTest.A".to_string())]);

        let resolved = resolve_type_reference(
            &name,
            &["ISQBase::ampere".to_string()],
            &stdlib_aliases,
            &local_definitions,
            &BTreeMap::new(),
            &ImportAliases::default(),
        );

        assert_eq!(resolved.as_deref(), Some("type.ItemTest.A"));
    }

    #[test]
    fn expression_path_resolves_cast_target_features() {
        let module = parse_sysml(
            r#"
            package Demo {
                part def VehiclePart {
                    attribute m;
                }

                part def Vehicle :> VehiclePart;
                part vehicle : Vehicle;
                part vehicles[*] = (vehicle, vehicle);
                attribute masses1[*] = (vehicles as VehiclePart).m;
                attribute masses2[*] = (vehicles as vehicle).m;
            }
            "#,
        )
        .unwrap();
        let stdlib = fake_stdlib([]);
        let mappings = MappingBundle::load().unwrap();
        let (_, _, definitions, _, _) =
            collect_modules(std::slice::from_ref(&module), &mappings).unwrap();
        let mass_feature = definitions
            .iter()
            .find(|definition| definition.declared_name == "VehiclePart")
            .and_then(|definition| {
                definition
                    .members
                    .iter()
                    .find(|member| member.declared_name == "m")
            })
            .expect("expected VehiclePart.m");
        let local_feature_index = build_local_feature_index(&definitions, &[]);
        let local_usage_map = build_local_usage_map(&definitions, &[]);
        let local_definitions = build_local_definition_map(&definitions, &mappings).unwrap();
        assert_eq!(mass_feature.owner_qualified_name, "Demo.VehiclePart");
        assert_eq!(
            local_feature_index
                .get("Demo.VehiclePart")
                .and_then(|features| features.get("m"))
                .map(String::as_str),
            Some("Demo.VehiclePart.m")
        );
        assert!(local_usage_map.contains_key("Demo.VehiclePart.m"));
        assert_eq!(
            resolve_type_reference_in_scope(
                &QualifiedName {
                    segments: vec!["VehiclePart".to_string()],
                    span: SourceSpan {
                        start_line: 0,
                        start_col: 0,
                        end_line: 0,
                        end_col: 0,
                    },
                },
                "Demo",
                &stdlib
                    .elements
                    .iter()
                    .map(|element| element.id.clone())
                    .collect::<Vec<_>>(),
                &build_stdlib_alias_map(&stdlib, &mappings),
                &local_definitions,
                &BTreeMap::new(),
                &ImportAliases::default(),
            )
            .as_deref(),
            Some("type.Demo.VehiclePart")
        );

        let resolved = resolve_module(&module, &stdlib, &mappings).unwrap();

        let expressions = resolved
            .usages
            .iter()
            .filter(|usage| usage.declared_name == "masses1" || usage.declared_name == "masses2")
            .map(|usage| usage.expression.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(expressions.len(), 2);
        assert!(expressions.iter().all(|expression| expression.is_some()));
    }

    #[test]
    fn expression_path_resolves_calculation_return_feature() {
        let module = parse_sysml(
            r#"
            package Demo {
                calc def Acceleration {
                    return a;
                }

                action dyn {
                    calc acc : Acceleration;
                    bind out = acc.a;
                }
            }
            "#,
        )
        .unwrap();
        let stdlib = fake_stdlib([]);
        let mappings = MappingBundle::load().unwrap();
        let collected = collect_modules(std::slice::from_ref(&module), &mappings).unwrap();
        let local_feature_index =
            build_local_feature_index(&collected.definitions, &collected.usages);
        let local_usage_map = build_local_usage_map(&collected.definitions, &collected.usages);
        let return_feature = collected
            .definitions
            .iter()
            .find(|definition| definition.declared_name == "Acceleration")
            .and_then(|definition| {
                definition
                    .members
                    .iter()
                    .find(|member| member.declared_name == "a")
            })
            .expect("expected return feature a");
        assert_eq!(return_feature.owner_qualified_name, "Demo.Acceleration");
        assert_eq!(
            local_feature_index
                .get("Demo.Acceleration")
                .and_then(|features| features.get("a"))
                .map(String::as_str),
            Some("Demo.Acceleration.a")
        );
        assert!(local_usage_map.contains_key("Demo.Acceleration.a"));

        let resolved = resolve_module(&module, &stdlib, &mappings).unwrap();

        let bind = resolved
            .usages
            .iter()
            .find_map(|usage| find_resolved_usage_by_declared_name(usage, "out"))
            .expect("expected bind usage");
        assert!(bind.expression.is_some());
    }

    #[test]
    fn stdlib_feature_index_adds_semantic_definition_base_features() {
        let mappings = MappingBundle::load().unwrap();
        let stdlib = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                kir_element("Items::Item", "ItemDefinition", []),
                kir_element("Items::Item::shape", "ItemUsage", []),
                kir_element(
                    "SpatialItems::SpatialItem",
                    "ItemDefinition",
                    [(
                        "specializes",
                        serde_json::json!(["SpatialFrames::SpatialFrame"]),
                    )],
                ),
                kir_element("SpatialFrames::SpatialFrame", "Structure", []),
            ],
        };

        let index = build_stdlib_feature_index(&stdlib, &mappings);

        assert_eq!(
            index
                .get("SpatialItems::SpatialItem")
                .and_then(|features| features.get("shape"))
                .map(String::as_str),
            Some("Items::Item::shape")
        );
    }

    #[test]
    fn stdlib_feature_index_adds_features_from_feature_type() {
        let mappings = MappingBundle::load().unwrap();
        let stdlib = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                kir_element("Base::DataValue", "DataType", []),
                kir_element("Quantities::QuantityDimension", "AttributeDefinition", []),
                kir_element(
                    "Quantities::QuantityDimension::quantityPowerFactors",
                    "ReferenceUsage",
                    [],
                ),
                kir_element(
                    "MeasurementReferences::ScalarMeasurementReference::quantityDimension",
                    "AttributeUsage",
                    [("type", serde_json::json!(["Quantities::QuantityDimension"]))],
                ),
            ],
        };

        let index = build_stdlib_feature_index(&stdlib, &mappings);

        assert_eq!(
            index
                .get("MeasurementReferences::ScalarMeasurementReference::quantityDimension")
                .and_then(|features| features.get("quantityPowerFactors"))
                .map(String::as_str),
            Some("Quantities::QuantityDimension::quantityPowerFactors")
        );
    }

    #[test]
    fn stdlib_feature_index_adds_inherited_features_from_feature_type() {
        let mappings = MappingBundle::load().unwrap();
        let stdlib = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                kir_element("Base::DataValue", "DataType", []),
                kir_element(
                    "MeasurementReferences::TensorMeasurementReference",
                    "AttributeDefinition",
                    [],
                ),
                kir_element(
                    "MeasurementReferences::TensorMeasurementReference::mRefs",
                    "AttributeUsage",
                    [],
                ),
                kir_element(
                    "MeasurementReferences::VectorMeasurementReference",
                    "AttributeDefinition",
                    [(
                        "specializes",
                        serde_json::json!(["MeasurementReferences::TensorMeasurementReference"]),
                    )],
                ),
                kir_element(
                    "MeasurementReferences::CoordinateFrame",
                    "AttributeDefinition",
                    [(
                        "specializes",
                        serde_json::json!(["MeasurementReferences::VectorMeasurementReference"]),
                    )],
                ),
                kir_element(
                    "MeasurementReferences::'3dCoordinateFrame'",
                    "AttributeDefinition",
                    [(
                        "specializes",
                        serde_json::json!(["MeasurementReferences::CoordinateFrame"]),
                    )],
                ),
                kir_element(
                    "SpatialItems::SpatialItem::coordinateFrame",
                    "AttributeUsage",
                    [(
                        "type",
                        serde_json::json!(["MeasurementReferences::'3dCoordinateFrame'"]),
                    )],
                ),
            ],
        };

        let index = build_stdlib_feature_index(&stdlib, &mappings);

        assert_eq!(
            index
                .get("SpatialItems::SpatialItem::coordinateFrame")
                .and_then(|features| features.get("mRefs"))
                .map(String::as_str),
            Some("MeasurementReferences::TensorMeasurementReference::mRefs")
        );
    }

    #[test]
    fn nested_redefinition_follows_stdlib_feature_type_features() {
        let module = parse_sysml(
            r#"
            package Demo {
                private import SpatialItems::*;

                part def Car :> SpatialItem {
                    attribute datum :>> coordinateFrame {
                        :>> mRefs = ();
                    }
                }
            }
            "#,
        )
        .unwrap();
        let mappings = MappingBundle::load().unwrap();
        let stdlib = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                kir_element("Base::DataValue", "DataType", []),
                kir_element("Parts::Part", "PartDefinition", []),
                kir_element("SpatialItems::SpatialItem", "ItemDefinition", []),
                kir_element(
                    "SpatialItems::SpatialItem::coordinateFrame",
                    "AttributeUsage",
                    [(
                        "type",
                        serde_json::json!(["MeasurementReferences::'3dCoordinateFrame'"]),
                    )],
                ),
                kir_element(
                    "MeasurementReferences::TensorMeasurementReference",
                    "AttributeDefinition",
                    [],
                ),
                kir_element(
                    "MeasurementReferences::TensorMeasurementReference::mRefs",
                    "AttributeUsage",
                    [],
                ),
                kir_element(
                    "MeasurementReferences::VectorMeasurementReference",
                    "AttributeDefinition",
                    [(
                        "specializes",
                        serde_json::json!(["MeasurementReferences::TensorMeasurementReference"]),
                    )],
                ),
                kir_element(
                    "MeasurementReferences::CoordinateFrame",
                    "AttributeDefinition",
                    [(
                        "specializes",
                        serde_json::json!(["MeasurementReferences::VectorMeasurementReference"]),
                    )],
                ),
                kir_element(
                    "MeasurementReferences::'3dCoordinateFrame'",
                    "AttributeDefinition",
                    [(
                        "specializes",
                        serde_json::json!(["MeasurementReferences::CoordinateFrame"]),
                    )],
                ),
            ],
        };

        let resolved = resolve_module(&module, &stdlib, &mappings).unwrap();
        let mrefs = resolved
            .definitions
            .iter()
            .flat_map(|definition| &definition.members)
            .find_map(|usage| find_resolved_usage_by_declared_name(usage, "mRefs"))
            .expect("expected mRefs usage");
        assert_eq!(
            mrefs.redefined_features.first().map(String::as_str),
            Some("feature.MeasurementReferences::TensorMeasurementReference::mRefs")
        );
    }

    #[test]
    fn geometry_coordinate_frame_mrefs_resolves_with_full_stdlib() {
        let module = parse_sysml(
            r#"
            package Demo {
                private import SpatialItems::*;
                private import SI::*;

                part def Car :> SpatialItem {
                    attribute datum :>> coordinateFrame {
                        :>> mRefs = (mm, mm, mm);
                    }
                }
            }
            "#,
        )
        .unwrap();
        let stdlib =
            KirDocument::from_path(&crate::paths::default_stdlib_path()).expect("stdlib loads");
        let mappings = MappingBundle::load().unwrap();
        let index = build_stdlib_feature_index(&stdlib, &mappings);
        assert_eq!(
            index
                .get("SpatialItems::SpatialItem::coordinateFrame")
                .and_then(|features| features.get("mRefs"))
                .map(String::as_str),
            Some("MeasurementReferences::TensorMeasurementReference::mRefs")
        );

        let resolved = resolve_module(&module, &stdlib, &mappings).unwrap();
        let mrefs = resolved
            .definitions
            .iter()
            .flat_map(|definition| &definition.members)
            .find_map(|usage| find_resolved_usage_by_declared_name(usage, "mRefs"))
            .expect("expected mRefs usage");
        assert!(!mrefs.redefined_features.is_empty());
    }

    #[test]
    fn redefinition_resolves_feature_angle_adornment_alias() {
        let module = parse_sysml(
            r#"
            package Demo {
                private import SpatialItems::*;
                private import MeasurementReferences::TranslationRotationSequence;

                part def Engine :> SpatialItem {
                    attribute <ecf> engineCoordinateFrame :>> coordinateFrame;
                }

                part def Car {
                    part powerSource : Engine {
                        :>> ecf {
                            :>> transformation : TranslationRotationSequence;
                        }
                    }
                }
            }
            "#,
        )
        .unwrap();
        let stdlib =
            KirDocument::from_path(&crate::paths::default_stdlib_path()).expect("stdlib loads");
        let mappings = MappingBundle::load().unwrap();
        let collected = collect_modules(std::slice::from_ref(&module), &mappings).unwrap();
        let local_feature_index =
            build_local_feature_index(&collected.definitions, &collected.usages);
        let local_usage_map = build_local_usage_map(&collected.definitions, &collected.usages);
        assert_eq!(
            unique_feature_modifier_alias_match_excluding(
                "ecf",
                &local_feature_index,
                &local_usage_map,
                "Demo.Car.powerSource.ecf",
            )
            .as_deref(),
            Some("Demo.Engine.engineCoordinateFrame")
        );

        let resolved = resolve_module(&module, &stdlib, &mappings).unwrap();
        let ecf = resolved
            .definitions
            .iter()
            .flat_map(|definition| &definition.members)
            .find_map(|usage| find_resolved_usage_by_declared_name(usage, "ecf"))
            .expect("expected ecf redefinition");
        assert_eq!(
            ecf.redefined_features.first().map(String::as_str),
            Some("feature.Demo.Engine.engineCoordinateFrame")
        );
        let transformation = resolved
            .definitions
            .iter()
            .flat_map(|definition| &definition.members)
            .find_map(|usage| find_resolved_usage_by_declared_name(usage, "transformation"))
            .expect("expected transformation redefinition");
        assert_eq!(
            transformation
                .redefined_features
                .first()
                .map(String::as_str),
            Some("MeasurementReferences::CoordinateFrame::transformation")
        );
    }

    #[test]
    fn succession_flow_resolves_action_output_endpoint() {
        let module = parse_sysml(
            r#"
            package Demo {
                attribute def OnOffCmd;

                action illuminateRegion {
                    action sendOnOffCmd {
                        out onOffCmd: OnOffCmd;
                    }

                    action produceDirectedLight {
                        in onOffCmd;
                    }

                    succession flow onOffCmdFlow from sendOnOffCmd.onOffCmd to produceDirectedLight.onOffCmd;
                }
            }
            "#,
        )
        .unwrap();
        let stdlib = fake_stdlib([]);
        let mappings = MappingBundle::load().unwrap();

        let resolved = resolve_module(&module, &stdlib, &mappings).unwrap();

        let flow = resolved
            .usages
            .iter()
            .find_map(|usage| find_resolved_usage_by_declared_name(usage, "onOffCmdFlow"))
            .expect("expected succession flow");
        let source = flow
            .members
            .iter()
            .find(|member| {
                member
                    .modifiers
                    .iter()
                    .any(|modifier| modifier == "source-output")
            })
            .expect("expected source endpoint");
        assert!(source.reference_target.is_some());
    }

    fn find_resolved_usage_by_declared_name<'a>(
        usage: &'a ResolvedUsage,
        declared_name: &str,
    ) -> Option<&'a ResolvedUsage> {
        if usage.declared_name == declared_name {
            return Some(usage);
        }
        usage
            .members
            .iter()
            .find_map(|member| find_resolved_usage_by_declared_name(member, declared_name))
    }

    fn fake_stdlib<const N: usize>(ids: [&str; N]) -> KirDocument {
        let default_ids = [
            "Actions::Action",
            "Base::DataValue",
            "BinaryConnection",
            "Connections::BinaryConnection",
            "Items::Item",
            "Parts::Part",
            "Ports::Port",
        ];
        KirDocument {
            metadata: BTreeMap::new(),
            elements: default_ids
                .into_iter()
                .chain(ids)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(|id| KirElement {
                    id: id.to_string(),
                    kind: id.to_string(),
                    layer: 1,
                    properties: BTreeMap::new(),
                })
                .collect(),
        }
    }

    fn kir_element<const N: usize>(
        id: &str,
        kind: &str,
        properties: [(&str, Value); N],
    ) -> KirElement {
        KirElement {
            id: id.to_string(),
            kind: kind.to_string(),
            layer: 1,
            properties: properties
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        }
    }
}
