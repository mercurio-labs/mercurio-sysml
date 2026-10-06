mod identities;
mod namespace_checks;
mod qualified_visibility;
mod implicit_defaults;
mod connectors;
mod semantic_metadata;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::Value;

use mercurio_foundation::kir::KirDocument;
use mercurio_foundation::language_contracts::ast::{
    Expr, LiteralExpr, ParsedModule as SysmlModule, QualifiedName, SourceSpan,
};
use mercurio_foundation::language_contracts::diagnostics::Diagnostic;

use crate::language_frontend::logging::{compile_timer_start, log_compile_timed_event};
use crate::language_frontend::lowering::collect::{
    CollectedDefinition, CollectedImport, CollectedUsage, ImportAliases, collect_module,
    collect_modules,
};
use crate::language_frontend::lowering::elaborate::{
    shorthand_reference_target, should_use_implicit_reference_redefinition_target,
};
use crate::language_frontend::lowering::emit::MappingBundle;
use crate::language_frontend::lowering::imports::build_import_alias_map;
use crate::language_frontend::lowering::indexes::{
    LibraryIndexes, build_local_alias_map, build_local_definition_map, build_local_feature_index,
    build_local_usage_map, cached_library_indexes,
};
pub use crate::language_frontend::lowering::ir::{
    ResolvedAlias, ResolvedAnnotationTarget, ResolvedDefinition, ResolvedExpr, ResolvedImport, ResolvedModule, ResolvedPackage,
    ResolvedPathSegment, ResolvedUsage,
};
use crate::language_frontend::lowering::names::expand_import_namespace_prefix;
use crate::language_frontend::lowering::policy::{
    KERML_RESOLVE_POLICY, ResolvePolicy, STRICT_RESOLVE_POLICY,
};

fn expression_span(expr: &Expr) -> SourceSpan {
    match expr {
        Expr::Literal(_) => SourceSpan {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        },
        Expr::Name(name) | Expr::TypeReference(name) => name.span.clone(),
        Expr::SelfRef(span) => span.clone(),
        Expr::Operation { span, .. }
        | Expr::NamedArgument { span, .. }
        | Expr::Lambda { span, .. }
        | Expr::Tuple { span, .. }
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
    imports: Vec<CollectedImport>,
    local_aliases: BTreeMap<String, QualifiedName>,
    alias_memberships: BTreeMap<String, String>,
    non_public_members: BTreeSet<String>,
    membership_visibility: Arc<BTreeMap<String, String>>,
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
        let imports = collected_context.imports;
        let local_aliases = build_local_alias_map(&collected_context.aliases);
        let mut alias_memberships = BTreeMap::new();
        for alias in &collected_context.aliases {
            if let Some(decl) = &alias.declaration {
                let id = format!("membership.alias.{}.{}.{}", alias.qualified_name, decl.span.start_line, decl.span.start_col);
                let mut names = decl.modifiers.iter().filter_map(|m| m.strip_prefix("short_name=")).collect::<Vec<_>>();
                if !decl.name.is_empty() { names.push(&decl.name); }
                for name in names {
                    let name = if alias.owner_qualified_name == "root" { name.to_string() } else { format!("{}.{}", alias.owner_qualified_name, name) };
                    alias_memberships.insert(name, id.clone());
                }
            }
        }

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
        let mut local_usage_map = build_local_usage_map(&definitions, &usages);
        for alias in &collected_context.aliases {
            if let Some(decl) = &alias.declaration {
                let body = super::collect::collect_relationship_body(&decl.body_members, &alias.qualified_name, "Membership", mappings)?;
                local_usage_map.extend(build_local_usage_map(&[], &body));
            }
        }
        for import in &imports {
            let body = super::collect::collect_relationship_body(&import.decl.body_members, &super::collect::import_body_scope(import), "Import", mappings)?;
            local_usage_map.extend(build_local_usage_map(&[], &body));
        }
        let non_public = |modifiers: &[String]| modifiers.iter().any(|m| matches!(m.as_str(), "private" | "protected"));
        let mut non_public_members = BTreeSet::new();
        for package in &packages {
            if non_public(&package.modifiers) { non_public_members.insert(package.qualified_name.clone()); }
        }
        for definition in &definitions {
            if !definition.is_public { non_public_members.insert(definition.qualified_name.clone()); }
        }
        for usage in local_usage_map.values() {
            if non_public(&usage.modifiers) { non_public_members.insert(usage.qualified_name.clone()); }
        }
        for import in &imports {
            if !import.decl.is_expose && non_public(&import.decl.modifiers) {
                if let Some(name) = import.decl.path.segments.last().filter(|name| !matches!(name.as_str(), "*" | "**")) {
                    let owner = import.owner_qualified_name.as_deref().unwrap_or("root");
                    let key = format!("{owner}.{name}");
                    if !local_definitions.contains_key(&key) && !local_usage_map.contains_key(&key)
                        && !packages.iter().any(|package| package.qualified_name == key)
                        && !local_aliases.contains_key(&key) {
                        non_public_members.insert(key);
                    }
                }
            }
        }
        for alias in &collected_context.aliases {
            let hidden = alias.declaration.as_ref().map_or_else(
                || non_public_members.contains(&alias.target.as_dot_string()), |decl| non_public(&decl.modifiers));
            if hidden {
                let mut names = vec![alias.declared_name.as_str()];
                if let Some(decl) = &alias.declaration { names.extend(decl.modifiers.iter().filter_map(|m| m.strip_prefix("short_name="))); }
                for name in names.into_iter().filter(|name| !name.is_empty()) {
                    non_public_members.insert(if alias.owner_qualified_name == "root" { name.to_string() } else { format!("{}.{}", alias.owner_qualified_name, name) });
                }
            }
        }
        let visibility = |modifiers: &[String]| modifiers.iter()
            .find(|m| matches!(m.as_str(), "private" | "protected" | "public"))
            .cloned().unwrap_or_else(|| "public".into());
        let mut membership_visibility = BTreeMap::new();
        for package in &packages { membership_visibility.insert(package.qualified_name.clone(), visibility(&package.modifiers)); }
        for definition in &definitions { membership_visibility.insert(definition.qualified_name.clone(), visibility(&definition.modifiers)); }
        for usage in local_usage_map.values() {
            membership_visibility.insert(usage.qualified_name.clone(), visibility(&usage.modifiers));
            for short in usage.modifiers.iter().filter_map(|m| m.strip_prefix("short_name=")) {
                membership_visibility.insert(format!("{}.{}", usage.owner_qualified_name, short), visibility(&usage.modifiers));
            }
        }
        for import in &imports {
            if !import.decl.is_expose {
                if let Some(name) = import.decl.path.segments.last().filter(|name| !matches!(name.as_str(), "*" | "**")) {
                    let owner = import.owner_qualified_name.as_deref().unwrap_or("root");
                    membership_visibility.entry(format!("{owner}.{name}")).or_insert_with(|| visibility(&import.decl.modifiers));
                }
            }
        }
        for alias in &collected_context.aliases {
            let access = alias.declaration.as_ref().map(|decl| visibility(&decl.modifiers))
                .or_else(|| membership_visibility.get(&alias.target.as_dot_string()).cloned())
                .unwrap_or_else(|| "public".into());
            let mut names = vec![alias.declared_name.as_str()];
            if let Some(decl) = &alias.declaration { names.extend(decl.modifiers.iter().filter_map(|m| m.strip_prefix("short_name="))); }
            for name in names.into_iter().filter(|name| !name.is_empty()) {
                let key = if alias.owner_qualified_name == "root" { name.to_string() } else { format!("{}.{}", alias.owner_qualified_name, name) };
                membership_visibility.insert(key, access.clone());
            }
        }
        let membership_visibility = Arc::new(membership_visibility);
        log_compile_timed_event(
            "resolver.build_local_indexes",
            local_index_start,
            "ok",
            format!("definitions={} usages={}", definitions.len(), usages.len()),
        );

        let stdlib_index_start = compile_timer_start();
        let library_indexes = cached_library_indexes(library_context, mappings)?;
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
            imports,
            local_aliases,
            alias_memberships,
            non_public_members,
            membership_visibility,
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
    qualified_visibility::QualifiedVisibility::new(context, &context.local_aliases).module(&collected_module)?;
    let packages = collected_module.packages;
    let imports = collected_module.imports;
    let mut definitions = collected_module.definitions;
    let mut usages = collected_module.usages;
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

    let mut local_aliases = context.local_aliases.clone();
    local_aliases.extend(build_local_alias_map(&aliases));
    for alias in &aliases {
        let mut name = alias.target.as_dot_string();
        let mut visited = BTreeSet::new();
        while !context.local_definitions.contains_key(&name) && !context.local_usage_map.contains_key(&name) {
            let Some(next) = local_aliases.get(&name) else { break; };
            if !visited.insert(name.clone()) {
                return Err(Diagnostic::semantic(format!("cyclic alias target `{name}`"), Some(alias.target.span.clone())));
            }
            name = next.as_dot_string();
        }
    }


    // Public membership imports are addressable through the importing
    // namespace. Resolve their canonical targets before binding consumers.
    loop {
        let mut added = false;
        for import in &context.imports {
            if import.decl.is_expose
                || import
                    .decl
                    .modifiers
                    .iter()
                    .any(|m| matches!(m.as_str(), "private" | "protected"))
            {
                continue;
            }
            let Some(last) = import.decl.path.segments.last() else {
                continue;
            };
            if matches!(last.as_str(), "*" | "**") {
                continue;
            }
            let Some(owner) = &import.owner_qualified_name else {
                continue;
            };
            let key = format!("{owner}.{last}");
            if local_aliases.contains_key(&key) {
                continue;
            }
            if let Some(target) = resolve_import_target(
                &import.decl.path,
                &context.library_indexes.ids,
                &context.library_indexes.aliases,
                &context.local_definitions,
                &local_aliases,
                Some(&context.local_usage_map),
            ) {
                let target = target
                    .strip_prefix("type.")
                    .or_else(|| target.strip_prefix("feature."))
                    .unwrap_or(&target);
                local_aliases.insert(
                    key,
                    QualifiedName {
                        segments: target
                            .replace("::", ".")
                            .split('.')
                            .map(str::to_string)
                            .collect(),
                        span: import.decl.span.clone(),
                    },
                );
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    // An import may name a member made visible by an enclosing namespace's
    // wildcard import. Bind those namespaces before resolving named imports.
    let wildcard_imports = context
        .imports
        .iter()
        .chain(imports.iter())
        .filter(|import| {
            import
                    .decl
                    .path
                    .segments
                    .last()
                    .is_some_and(|name| name == "*" || name == "**")
        })
        .map(|import| ResolvedImport {
            original_path: import.decl.path.clone(),
            alias_membership_id: None,
            referenced_element_id: None,
            members: Vec::new(),
            owner_qualified_name: import.owner_qualified_name.clone(),
            target_id: import.decl.path.as_colon_string(),
            is_expose: import.decl.is_expose,
            is_import_all: import.decl.is_expose || import.decl.modifiers.iter().any(|m| m == "import_all"),
            visibility: namespace_checks::import_visibility(import).to_string(),
            is_public: namespace_checks::import_visibility(import) == "public",
            filter: import.decl.filter.clone(),
            imported_name: None,
            docs: import.decl.docs.clone(),
            span: import.decl.span.clone(),
            ordinal: 0,
        })
        .collect::<Vec<_>>();
    let mut enclosing_import_aliases = build_import_alias_map(
        &wildcard_imports,
        &context.packages,
        &context.definitions,
        &context.local_usage_map,
        &local_aliases,
        &context.non_public_members,
        &context.library_indexes.ids,
        &context.library_indexes.aliases,
        policy,
    )?;
    enclosing_import_aliases.membership_visibility = context.membership_visibility.clone();
    enclosing_import_aliases.library_membership_visibility = context.library_indexes.membership_visibility.clone();
    enclosing_import_aliases.library_namespace_scope = context.library_indexes.namespace_scope.clone();
    let resolve_import_start = compile_timer_start();
    let mut resolved_imports = resolve_imports(
        &imports,
        &context.library_indexes.ids,
        &context.library_indexes.aliases,
        &context.local_definitions,
        &local_aliases,
        &context.local_usage_map,
        &context.packages,
        &enclosing_import_aliases,
    )?;
    log_compile_timed_event(
        "resolver.resolve_imports",
        resolve_import_start,
        "ok",
        format!("imports={}", resolved_imports.len()),
    );

    let import_alias_start = compile_timer_start();
    // Support definitions resolve in the namespaces where they were declared.
    // Do not turn unrelated support-file diagnostics into target diagnostics;
    // unresolved support imports still fail if a target actually uses them.
    let mut visible_imports = context
        .imports
        .iter()
        .filter_map(|import| {
            resolve_imports(
                std::slice::from_ref(import),
                &context.library_indexes.ids,
                &context.library_indexes.aliases,
                &context.local_definitions,
                &local_aliases,
                &context.local_usage_map,
                &context.packages,
                &enclosing_import_aliases,
            )
            .ok()
        })
        .flatten()
        .collect::<Vec<_>>();
    for import in &resolved_imports {
        if !visible_imports.iter().any(|other| {
            other.owner_qualified_name == import.owner_qualified_name
                && other.target_id == import.target_id
                && other.imported_name == import.imported_name
        }) {
            visible_imports.push(import.clone());
        }
    }
    let mut import_aliases = build_import_alias_map(
        &visible_imports,
        &context.packages,
        &context.definitions,
        &context.local_usage_map,
        &local_aliases,
        &context.non_public_members,
        &context.library_indexes.ids,
        &context.library_indexes.aliases,
        policy,
    )?;
    import_aliases.membership_visibility = context.membership_visibility.clone();
    import_aliases.library_membership_visibility = context.library_indexes.membership_visibility.clone();
    import_aliases.library_namespace_scope = context.library_indexes.namespace_scope.clone();
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

    // Metadata-derived parents must be visible in support-file indexes as well
    // as the module being emitted, before member lookup and usage flag derivation.
    let metadata_context = semantic_metadata::augment(
        context, &mut definitions, &mut usages, &local_aliases, &import_aliases, mappings,
    )?;
    let context = metadata_context.as_ref().unwrap_or(context);

    let resolve_definition_start = compile_timer_start();
    let resolved_definitions = definitions
        .into_iter()
        .map(|definition| {
            resolve_definition(
                definition,
                &context.packages,
                &context.library_indexes.kinds,
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
                &context.library_indexes.specializations,
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
                &context.packages,
                &context.library_indexes.kinds,
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

    let resolve_body = |body: &[mercurio_foundation::language_contracts::ast::Declaration], scope: &str, construct: &str| {
        let body = super::collect::collect_relationship_body(body, scope, construct, mappings)?;
        let access = qualified_visibility::QualifiedVisibility::new(context, &local_aliases);
        for usage in &body { access.usage(usage)?; }
        body.into_iter().map(|usage|
            resolve_usage(usage, &context.packages,
                &context.library_indexes.kinds, &context.library_indexes.ids,
                &context.library_indexes.feature_index, &context.library_indexes.aliases,
                &context.local_definitions, &local_aliases, &import_aliases, &context.definition_index,
                &context.local_feature_index, &context.local_usage_map, mappings, policy)
        ).collect::<Result<Vec<_>, Diagnostic>>()
    };
    let alias_lookup = FeatureLookup {
        stdlib_ids: &context.library_indexes.ids, stdlib_feature_index: &context.library_indexes.feature_index,
        stdlib_aliases: &context.library_indexes.aliases, local_definitions: &context.local_definitions,
        local_aliases: &local_aliases, import_aliases: &import_aliases,
        definition_index: &context.definition_index, local_feature_index: &context.local_feature_index,
        local_usage_map: &context.local_usage_map,
    };
    for (collected, resolved) in imports.iter().zip(&mut resolved_imports) {
        let mut target_name = collected.decl.path.clone();
        while target_name.segments.last().is_some_and(|s| s == "*" || s == "**") { target_name.segments.pop(); }
        let owner = collected.owner_qualified_name.as_deref().unwrap_or("root");
        resolved.alias_membership_id = scoped_reference_candidates(&target_name.as_dot_string(), owner)
            .into_iter().find_map(|name| context.alias_memberships.get(&name).cloned());
        // Expose query text is preserved for downstream query evaluation. Its
        // provisional binding must not resolve its own reference back to that text.
        let mut target_aliases = import_aliases.clone();
        if resolved.is_expose {
            target_aliases.value_aliases.retain(|_, value| value != &resolved.target_id);
        }
        let target_lookup = FeatureLookup { import_aliases: &target_aliases, ..alias_lookup };
        resolved.referenced_element_id = (resolved.is_expose.then(|| resolve_local_usage_reference(&target_name, &context.local_usage_map)).flatten())
            .or_else(|| target_lookup.resolve(&target_name, owner, "", &mut BTreeSet::new()))
            .or_else(|| (!resolved.is_expose && resolved.target_id.starts_with("feature.")).then(|| resolved.target_id.clone()))
            .or_else(|| super::names::resolve_local_namespace_dot(&target_name.as_colon_string(), owner, &None, &context.packages).map(|name| format!("pkg.{name}")))
            .or_else(|| resolve_local_usage_reference(&target_name, &context.local_usage_map));
        if resolved.referenced_element_id.is_none() {
            return Err(Diagnostic::semantic(format!("unresolved {} target `{}`", if collected.decl.is_expose { "expose" } else { "import" }, target_name.as_colon_string()), Some(collected.decl.span.clone())));
        }
        resolved.members = resolve_body(&collected.decl.body_members, &super::collect::import_body_scope(collected), "Import")?;
    }
    let resolved_aliases = aliases.iter().filter_map(|alias| alias.declaration.as_ref().map(|decl| (alias, decl))).map(|(alias, decl)| {
        let owner = &alias.owner_qualified_name;
        let target = resolve_type_reference_in_scope(&decl.target, owner, &context.library_indexes.ids,
                &context.library_indexes.aliases, &context.local_definitions, &local_aliases, &import_aliases)
            .or_else(|| alias_lookup.resolve(&decl.target, owner, "", &mut BTreeSet::new()))
            .or_else(|| super::names::resolve_local_namespace_dot(&decl.target.as_colon_string(), owner, &None, &context.packages).map(|name| format!("pkg.{name}")))
            .ok_or_else(|| Diagnostic::semantic(format!("unresolved alias target `{}`", decl.target.as_colon_string()), Some(decl.target.span.clone())))?;
        Ok(ResolvedAlias { qualified_name: alias.qualified_name.clone(), owner_qualified_name: owner.clone(),
            declared_name: decl.name.clone(), declared_short_name: decl.modifiers.iter().find_map(|m| m.strip_prefix("short_name=").map(str::to_string)),
            target, visibility: decl.modifiers.iter().find(|m| matches!(m.as_str(), "public" | "private" | "protected")).cloned().unwrap_or_else(|| "public".into()),
            members: resolve_body(&decl.body_members, &alias.qualified_name, "Membership")?, docs: decl.docs.clone(), span: decl.span.clone() })
    }).collect::<Result<Vec<_>, Diagnostic>>()?;
    let mut resolved = ResolvedModule {
        aliases: resolved_aliases,
        packages,
        imports: resolved_imports,
        definitions: resolved_definitions,
        usages: resolved_usages,
    };
    let lookup = FeatureLookup {
        stdlib_ids: &context.library_indexes.ids,
        stdlib_feature_index: &context.library_indexes.feature_index,
        stdlib_aliases: &context.library_indexes.aliases,
        local_definitions: &context.local_definitions,
        local_aliases: &local_aliases,
        import_aliases: &import_aliases,
        definition_index: &context.definition_index,
        local_feature_index: &context.local_feature_index,
        local_usage_map: &context.local_usage_map,
    };
    typing::derive_composite_flags(&mut resolved, context, mappings, &lookup)?;
    implicit_defaults::apply(&mut resolved, context, mappings, &lookup)?;
    typing::validate_usage_types(&resolved, context, mappings, &lookup)?;
    typing::derive_usage_flags(&mut resolved, context, mappings, &lookup)?;
    typing::validate_scalar_checks(&resolved, mappings)?;
    typing::validate_feature_flags(&mut resolved, context, mappings, &lookup)?;
    connectors::derive_relations(&mut resolved, mappings)?;
    identities::finalize(&mut resolved, context, mappings)?;
    Ok(resolved)
}

fn resolve_imports(
    imports: &[CollectedImport],
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    local_usage_map: &BTreeMap<String, CollectedUsage>,
    packages: &[ResolvedPackage],
    enclosing_import_aliases: &ImportAliases,
) -> Result<Vec<ResolvedImport>, Diagnostic> {
    let mut resolved = Vec::new();

    for import in imports {
        namespace_checks::validate_import(import)?;
        let scoped_target = import.owner_qualified_name.as_deref().and_then(|owner| {
            let mut scope = Some(owner);
            while let Some(prefix) = scope {
                let dotted = format!("{prefix}.{}", import.decl.path.as_dot_string());
                if let Some(target) = local_definitions.get(&dotted) {
                    return Some(target.clone());
                }
                if !import.decl.is_expose {
                    if let Some(target) = local_usage_map.get(&dotted) {
                        return Some(collected_usage_element_id(target));
                    }
                }
                scope = prefix.rsplit_once('.').map(|(parent, _)| parent);
            }
            None
        });
        let package_target = (!import.decl.is_expose)
            .then(|| {
                crate::language_frontend::lowering::names::resolve_local_namespace_dot(
                    &import.decl.path.as_colon_string(),
                    import.owner_qualified_name.as_deref().unwrap_or(""),
                    &None,
                    packages,
                )
                .map(|name| format!("package.{name}"))
            })
            .flatten();
        let resolved_target = scoped_target
            .or(package_target)
            .or_else(|| {
                if import
                    .decl
                    .path
                    .segments
                    .last()
                    .is_some_and(|name| name == "*" || name == "**")
                {
                    return None;
                }
                resolve_type_reference_in_scope(
                    &import.decl.path,
                    import.owner_qualified_name.as_deref().unwrap_or(""),
                    stdlib_ids,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    enclosing_import_aliases,
                )
            })
            .or_else(|| {
                resolve_import_target(
                    &import.decl.path,
                    stdlib_ids,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    // Usages are offered to `import` only. SV-2 deliberately made a
                    // non-wildcard `expose` scope fall through as verbatim text, so
                    // `exposed_elements` binds it against the graph; binding it to an
                    // element id here would take that back.
                    (!import.decl.is_expose).then_some(local_usage_map),
                )
            });

        // An `expose` scope is a *query*, not a name binding. It does not have
        // to name one element at compile time: `vehicle::**` already falls
        // through as verbatim text, because a wildcard cannot denote a single
        // target, and `exposed_elements` resolves it against the graph.
        //
        // A non-wildcard scope is the same kind of thing, so it falls through
        // the same way. Erroring instead made the pilot's own
        // `view vehicleSafetyFeatureView { expose vehicle; }` fail to compile,
        // since `vehicle` is a *usage* and only definitions are bound here.
        //
        // `import` keeps the hard error: an import really does bind a name into
        // a scope, and an unresolvable one is a defect the author must see.
        let target_id = match resolved_target {
            Some(target_id) => target_id,
            None if import.decl.is_expose => import.decl.path.as_colon_string(),
            None => {
                return Err(Diagnostic::new(
                    format!("unresolved import `{}`", import.decl.path.as_colon_string()),
                    Some(import.decl.span.clone()),
                ));
            }
        };

        resolved.push(ResolvedImport {
            original_path: import.decl.path.clone(),
            alias_membership_id: None,
            referenced_element_id: None,
            members: Vec::new(),
            owner_qualified_name: import.owner_qualified_name.clone(),
            target_id,
            is_expose: import.decl.is_expose,
            is_import_all: import.decl.is_expose || import.decl.modifiers.iter().any(|m| m == "import_all"),
            visibility: namespace_checks::import_visibility(import).to_string(),
            is_public: namespace_checks::import_visibility(import) == "public",
            filter: import.decl.filter.clone(),
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
    packages: &[ResolvedPackage],
    library_kinds: &BTreeMap<String, String>,
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
    library_parents: &BTreeMap<String, Vec<String>>,
) -> Result<ResolvedDefinition, Diagnostic> {
    let lookup = FeatureLookup {
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
    };
    let mut specializes = definition
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
                ).or_else(|| {
                    lookup.resolve(name, &definition.qualified_name, "", &mut BTreeSet::new())
                        .filter(|target| target.starts_with("type."))
                }),
                name,
                "specialization",
                policy,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    for implicit in &definition.implicit_specializations {
        if !specializes.iter().any(|parent| {
            lookup.type_specializes(
                parent,
                implicit,
                Some(library_parents),
                &mut BTreeSet::new(),
            )
        }) {
            specializes.push(implicit.clone());
        }
    }
    let members = definition
        .members
        .into_iter()
        .map(|usage| {
            resolve_usage(
                usage,
                packages,
                library_kinds,
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
        visibility: definition.modifiers.iter().find(|m| matches!(m.as_str(), "public" | "private" | "protected")).cloned().unwrap_or_else(|| "public".into()),
        declared_short_name: definition.modifiers.iter().find_map(|m| m.strip_prefix("short_name=").map(str::to_string)),
        is_anonymous: definition.modifiers.iter().any(|m| m == "anonymous_namespace"),
        construct: definition.construct,
        qualified_name: definition.qualified_name,
        declared_name: definition.declared_name,
        is_abstract: definition.is_abstract,
        is_variation: definition.is_variation,
        specializes,
        members,
        docs: definition.docs,
        span: definition.span,
    })
}

fn resolve_usage(
    mut usage: CollectedUsage,
    packages: &[ResolvedPackage],
    library_kinds: &BTreeMap<String, String>,
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
    if let Some(endpoints) = super::relationship_declarations::load::<super::relationship_declarations::Operand<QualifiedName>>(&usage.metadata_properties, &usage.span)? {
        let resolve_operand = |name: &QualifiedName| {
            resolve_allocation_endpoint(&usage, name, false, stdlib_ids, stdlib_feature_index,
                stdlib_aliases, local_definitions, local_aliases, import_aliases,
                definition_index, local_feature_index, local_usage_map)
                .or_else(|| (usage.construct == "Dependency").then(|| {
                    super::names::resolve_local_namespace_dot(&name.as_colon_string(),
                        &usage.owner_qualified_name, &None, packages).map(|name| format!("pkg.{name}"))
                }).flatten())
                .map(|id| id.strip_prefix("package.").map(|name| format!("pkg.{name}")).unwrap_or(id))
                .ok_or_else(|| Diagnostic::new(format!("unresolved relationship endpoint `{}`", name.as_colon_string()), Some(name.span.clone())))
        };
        let resolve_path = |operand: &super::relationship_declarations::Operand<QualifiedName>| {
            let mut steps: Vec<String> = Vec::with_capacity(operand.steps.len());
            for name in &operand.steps {
                let target = if let Some(previous) = steps.last() {
                    // Handwritten scope service, separate from imported syntax:
                    // Pilot scope_featureChaining scopes subsequent references in
                    // the preceding Feature, without lexical-parent fallback.
                    // Traverse qualification inside the previous Feature's scope,
                    // emitting only the final member as this grammar chain step.
                    // Intermediate qualified members are not FeatureChainings.
                    let mut namespace = previous.clone();
                    for (index, segment) in name.segments.iter().enumerate() {
                        let is_type = namespace.starts_with("type.");
                        let feature = if is_type {
                            resolve_feature_reference_from_type_id(
                                &namespace, segment, stdlib_ids, stdlib_feature_index,
                                stdlib_aliases, local_definitions, local_aliases, import_aliases,
                                definition_index, local_feature_index,
                            )
                        } else {
                            resolve_feature_reference_from_feature_type(
                                &namespace, segment, stdlib_ids, stdlib_feature_index,
                                stdlib_aliases, local_definitions, local_aliases, import_aliases,
                                definition_index, local_feature_index, local_usage_map,
                            )
                        };
                        // A qualified name may pass through a Class namespace,
                        // but the imported FeatureChaining reference still requires
                        // a Feature at its final segment. Exact owned definitions
                        // are consulted here; no global/suffix-name rescue applies.
                        let local_namespace_member = || {
                            let own_scope = namespace.strip_prefix("feature.")
                                .or_else(|| namespace.strip_prefix("type."))?;
                            let member_in_scope = |initial: &str| {
                                // The explicit generalization graph is shared by
                                // Feature and classifier namespace members. Visit
                                // owned members before parents, preserving declared
                                // parent order, and bound cycles by model identity.
                                let mut pending = vec![initial.to_owned()];
                                let mut visited = BTreeSet::new();
                                while let Some(scope) = pending.pop() {
                                    if !visited.insert(scope.clone()) { continue; }
                                    if let Some(member) = local_feature_index.get(&scope)
                                        .and_then(|members| members.get(segment)) {
                                        return Some(Ok((feature_id_from_qualified_name(member), false)));
                                    }
                                    if index + 1 < name.segments.len() {
                                        let key = format!("{scope}.{segment}");
                                        if definition_index.contains_key(&key) {
                                            return Some(Ok((format!("type.{key}"), false)));
                                        }
                                    }
                                    let alias_key = format!("{scope}.{segment}");
                                    if local_aliases.contains_key(&alias_key) {
                                        // Alias membership visibility controls access,
                                        // independently of its target's own membership.
                                        if import_aliases.membership_visibility.get(&alias_key)
                                            .is_none_or(|visibility| visibility != "public") {
                                            return Some(Err(()));
                                        }
                                        return Some(resolve_local_namespace_alias(
                                            &alias_key, local_aliases, definition_index, local_usage_map,
                                        ).map(|target| (target, true)).ok_or(()));
                                    }
                                    if let Some(definition) = definition_index.get(&scope) {
                                        for parent in definition.specializes.iter().rev() {
                                            if let Some(parent_id) = resolve_type_reference_in_scope(
                                                parent, &definition.qualified_name, stdlib_ids,
                                                stdlib_aliases, local_definitions, local_aliases, import_aliases,
                                            ) && let Some(parent_scope) = parent_id.strip_prefix("type.") {
                                                pending.push(parent_scope.to_owned());
                                            }
                                        }
                                    }
                                }
                                None
                            };
                            member_in_scope(own_scope).or_else(|| {
                                if is_type { return None; }
                                let ty = infer_usage_type_from_feature_id(
                                    &namespace, stdlib_ids, stdlib_aliases, local_definitions,
                                    local_aliases, import_aliases, local_usage_map,
                                )?;
                                member_in_scope(ty.strip_prefix("type.")?)
                            })
                        };
                        let visible = |target: &String| {
                            // Subsequent FeatureChaining references use an external
                            // namespace scope (scope_featureChaining passes false),
                            // even when the chain is textually inside the type.
                            let modifiers = target.strip_prefix("type.")
                                .and_then(|key| definition_index.get(key))
                                .map(|definition| &definition.modifiers)
                                .or_else(|| target.strip_prefix("feature.")
                                    .and_then(|key| local_usage_map.get(key))
                                    .map(|member| &member.modifiers));
                            !modifiers.is_some_and(|modifiers| modifiers.iter()
                                .any(|modifier| matches!(modifier.as_str(), "private" | "protected")))
                        };
                        namespace = local_namespace_member().or_else(|| feature.map(|target| Ok((target, false))))
                            .and_then(Result::ok)
                            .filter(|(target, via_alias)| *via_alias || visible(target))
                            .map(|(target, _)| target).ok_or_else(|| Diagnostic::new(
                            format!("unresolved qualified member `{segment}` in namespace `{namespace}`"),
                            Some(name.span.clone()),
                        ))?;
                    }
                    namespace
                } else {
                    resolve_operand(name)?
                };
                if operand.steps.len() > 1 {
                    // IDs distinguish identity, not metaclass compatibility. In
                    // particular, aliases can name Comments and Relationships.
                    // Use the imported Ecore reference target and ancestry for
                    // every known local object, including the first chain entry.
                    let contract = super::ecore_model::feature("FeatureChaining", "chaining_feature")
                        .ok_or_else(|| Diagnostic::new("missing imported FeatureChaining target contract", Some(name.span.clone())))?;
                    let construct = target.strip_prefix("type.")
                        .and_then(|key| definition_index.get(key))
                        .map(|definition| definition.construct.as_str())
                        .or_else(|| target.strip_prefix("feature.")
                            .and_then(|key| local_usage_map.get(key))
                            .map(|member| member.construct.as_str()))
                        .or_else(|| local_usage_map.values()
                            .find(|member| collected_usage_element_id(member) == target)
                            .map(|member| member.construct.as_str()));
                    let kind = if let Some(construct) = construct {
                        mappings.metaclass_for(construct)?
                    } else {
                        library_kinds.get(&target).map(String::as_str).ok_or_else(|| Diagnostic::new(
                            format!("missing chain endpoint metaclass for `{target}`"), Some(name.span.clone()),
                        ))?
                    };
                    if !super::relationship_declarations::metaclass_conforms(kind, contract.target) {
                        return Err(Diagnostic::new(
                            format!("chain step requires a {}, found {kind}", contract.target),
                            Some(name.span.clone()),
                        ));
                    }
                }
                steps.push(target);
            }
            let type_ref = if steps.len()>1 {
                inferred_usage_type_ref(&usage, &steps[steps.len()-1..], &[], &[], stdlib_ids,
                    stdlib_feature_index, stdlib_aliases, local_definitions, local_aliases,
                    import_aliases, definition_index, local_feature_index, local_usage_map)
            } else { None };
            Ok::<_,Diagnostic>(super::relationship_declarations::Operand { steps, type_ref })
        };
        let sources = endpoints.sources.iter().map(resolve_path).collect::<Result<Vec<_>,_>>()?;
        let targets = endpoints.targets.iter().map(resolve_path).collect::<Result<Vec<_>,_>>()?;
        super::relationship_declarations::store(&mut usage.metadata_properties,
            &super::relationship_declarations::Endpoints { sources, targets }, &usage.span)?;
    }
    if let Some(raw) = usage.metadata_properties.get("__flow_payload_type") {
        let name: QualifiedName = serde_json::from_str(raw).map_err(|error| Diagnostic::new(
            format!("invalid Flow payload type metadata: {error}"), Some(usage.span.clone()),
        ))?;
        let target = resolve_type_reference_in_scope(
            &name, &usage.owner_qualified_name, stdlib_ids, stdlib_aliases,
            local_definitions, local_aliases, import_aliases,
        ).ok_or_else(|| Diagnostic::new(
            format!("unresolved Flow payload type `{}`", name.as_colon_string()),
            Some(name.span.clone()),
        ))?;
        usage.metadata_properties.insert("__flow_payload_type_ref".into(), target);
    }
    for (source_key, target_key) in [
        ("__multiplicity_range_references", "__multiplicity_range_reference_ids"),
        ("__flow_payload_references", "__flow_payload_reference_ids"),
    ] {
    if let Some(raw) = usage.metadata_properties.get(source_key) {
        let names: Vec<Option<QualifiedName>> = serde_json::from_str(raw).map_err(|error| Diagnostic::new(
            format!("invalid MultiplicityRange reference metadata: {error}"), Some(usage.span.clone()),
        ))?;
        let targets = names.iter().map(|name| name.as_ref().map(|name| {
            resolve_feature_reference(
                &usage, name, stdlib_ids, stdlib_feature_index, stdlib_aliases,
                local_definitions, local_aliases, import_aliases, definition_index,
                local_feature_index, local_usage_map,
            ).ok_or_else(|| Diagnostic::new(
                format!("unresolved MultiplicityRange bound feature `{}`", name.as_colon_string()),
                Some(name.span.clone()),
            ))
        }).transpose()).collect::<Result<Vec<_>, Diagnostic>>()?;
        usage.metadata_properties.insert(target_key.into(),
            serde_json::to_string(&targets).expect("reference IDs serialize"));
    }
    }
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
    let inherited_lookup = FeatureLookup {
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
    };
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
            )
            .or_else(|| {
                inherited_lookup
                    .resolve(
                        name,
                        &usage.owner_qualified_name,
                        &feature_id_from_qualified_name(&usage.qualified_name),
                        &mut BTreeSet::new(),
                    )
                    .filter(|target| target.starts_with("type."))
            }) {
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
    let resolve_reference = |name: &QualifiedName| {
            let reference_target_policy =
                mappings.usage_reference_target_resolution_policy(&usage.construct);
            let resolved = if reference_target_policy
                == Some("annotation_target_then_type_then_reference")
            {
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
            };

            let resolved = resolved.or_else(|| {
                inherited_lookup
                    .resolve(
                        name,
                        &usage.owner_qualified_name,
                        &feature_id_from_qualified_name(&usage.qualified_name),
                        &mut BTreeSet::new(),
                    )
                    .filter(|target| target.starts_with("feature."))
            });
            match resolved {
                Some(target) => Ok(Some(target)),
                // A filter's `@Safety` is a metadata *predicate*, not a name
                // binding: `exposed_elements` evaluates it against each
                // element's own metadata features at render time, and a
                // predicate that currently matches nothing is a legitimate
                // filter, not an error. Requiring it to bind here made the
                // pilot's own `view def SafetyFeatureView { filter @Safety; }`
                // fail to compile unless the annotation happened to be applied
                // somewhere the reference index could already see
                // (save-as-view SV-2).
                None if reference_target_policy == Some("metadata_predicate") => Ok(None),
                None => Err(Diagnostic::new(
                    format!("unresolved reference target `{}`", name.as_colon_string()),
                    Some(name.span.clone()),
                )),
            }

    };
    let reference_target = effective_reference_target.as_ref().map(&resolve_reference).transpose()?.flatten();
    let annotation_targets = usage.annotation_targets.iter().map(|name| {
        let target = resolve_reference(name)?.ok_or_else(|| Diagnostic::new("unresolved annotation target", Some(name.span.clone())))?;
        Ok(ResolvedAnnotationTarget { target, span: name.span.clone() })
    }).collect::<Result<Vec<_>, _>>()?;
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
    let type_ref = type_ref.or_else(|| {
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
    });
    let members = usage
        .members
        .into_iter()
        .map(|member| {
            resolve_usage(
                member,
                packages,
                library_kinds,
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
        annotation_targets,
        derived_properties: BTreeMap::new(),
        construct: usage.construct,
        owner_construct: usage.owner_construct,
        owner_qualified_name: usage.owner_qualified_name,
        qualified_name: usage.qualified_name,
        declared_name: usage.declared_name,
        is_implicit_name: usage.is_implicit_name,
        has_explicit_type: usage.ty.is_some() || !usage.additional_types.is_empty(),
        has_explicit_specialization: usage.has_explicit_specialization,
        type_ref,
        additional_type_refs,
        reference_target,
        related_features: Vec::new(),
        allocation_source,
        allocation_target,
        metadata_properties: usage.metadata_properties,
        multiplicity: usage.multiplicity,
        expression,
        // `isDerived` is set only by the explicit `derived` modifier. A value
        // binding (`= expr`) is a FeatureValue, not a derivation, so it must
        // not imply `isDerived` (matches the pilot, which reports a bound but
        // non-`derived` feature as `isDerived = false`).
        is_derived: usage.modifiers.iter().any(|modifier| modifier == "derived"),
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
    resolve_expression_in_scope(
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
        &BTreeMap::new(),
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_expression_in_scope(
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
    bindings: &BTreeMap<String, Option<String>>,
) -> Result<ResolvedExpr, Diagnostic> {
    let resolve = |expr: &Expr| {
        resolve_expression_in_scope(
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
            bindings,
        )
    };
    let lookup = feature_lookup::FeatureLookup {
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
    };
    let resolve_type = |name: &QualifiedName| {
        resolve_type_reference_in_scope(
            name,
            &usage.owner_qualified_name,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        )
        .or_else(|| lookup.resolve(name, &usage.owner_qualified_name, "", &mut BTreeSet::new()))
        .ok_or_else(|| {
            Diagnostic::new(
                format!("unresolved expression type `{}`", name.as_colon_string()),
                Some(name.span.clone()),
            )
        })
    };
    match expr {
        Expr::Operation {
            operator, operands, ..
        } => Ok(ResolvedExpr::Operation {
            operator: operator.clone(),
            operands: operands
                .iter()
                .map(resolve)
                .collect::<Result<Vec<_>, _>>()?,
        }),
        Expr::TypeReference(name) => Ok(ResolvedExpr::TypeReference {
            target: resolve_type(name)?,
        }),
        Expr::NamedArgument {
            parameter, value, ..
        } => Ok(ResolvedExpr::NamedArgument {
            parameter: parameter.as_colon_string(),
            value: Box::new(resolve(value)?),
        }),
        Expr::Lambda {
            parameters, body, ..
        } => {
            let mut scope = bindings.clone();
            let mut resolved_parameters = Vec::new();
            for parameter in parameters {
                let type_ref = parameter.ty.as_ref().map(resolve_type).transpose()?;
                if resolved_parameters
                    .iter()
                    .any(|p: &super::ir::ResolvedExpressionParameter| p.name == parameter.name)
                {
                    return Err(Diagnostic::new(
                        format!("duplicate expression parameter `{}`", parameter.name),
                        Some(parameter.span.clone()),
                    ));
                }
                scope.insert(parameter.name.clone(), type_ref.clone());
                let mut properties = BTreeMap::new();
                properties.insert(
                    "direction".to_string(),
                    Value::String(parameter.keyword.clone()),
                );
                properties.insert(
                    "modifiers".to_string(),
                    serde_json::json!(parameter.modifiers),
                );
                if let Some(range) = &parameter.multiplicity {
                    properties.insert("multiplicity".to_string(), serde_json::json!(range));
                }
                for (key, value) in &parameter.metadata_properties {
                    properties.insert(key.clone(), Value::String(value.clone()));
                }
                resolved_parameters.push(super::ir::ResolvedExpressionParameter {
                    name: parameter.name.clone(),
                    type_ref,
                    properties,
                    default: None,
                });
            }
            let nested = |expr: &Expr| {
                resolve_expression_in_scope(
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
                    &scope,
                )
            };
            for (resolved, parameter) in resolved_parameters.iter_mut().zip(parameters) {
                resolved.default = parameter
                    .expression
                    .as_ref()
                    .map(nested)
                    .transpose()?
                    .map(Box::new);
            }
            Ok(ResolvedExpr::Lambda {
                parameters: resolved_parameters,
                body: Box::new(nested(body)?),
            })
        }
        Expr::Literal(LiteralExpr::Integer(value)) => {
            Ok(ResolvedExpr::Literal(Value::from(*value)))
        }
        Expr::Literal(LiteralExpr::Real(value)) => Ok(ResolvedExpr::Literal(Value::from(
            value.parse::<f64>().map_err(|_| {
                Diagnostic::new("invalid real literal", Some(expression_span(expr)))
            })?,
        ))),
        Expr::Literal(LiteralExpr::Boolean(value)) => {
            Ok(ResolvedExpr::Literal(Value::from(*value)))
        }
        Expr::Literal(LiteralExpr::String(value)) => {
            Ok(ResolvedExpr::Literal(Value::from(value.clone())))
        }
        Expr::SelfRef(_) => Ok(ResolvedExpr::SelfRef),
        Expr::Tuple { items, .. } => Ok(ResolvedExpr::Tuple {
            items: items.iter().map(resolve).collect::<Result<Vec<_>, _>>()?,
        }),
        Expr::Name(name)
            if name
                .segments
                .first()
                .is_some_and(|first| bindings.contains_key(first)) =>
        {
            lexical_expression_target(expr, bindings, &lookup)?;
            let mut result = ResolvedExpr::Variable {
                name: name.segments[0].clone(),
            };
            for segment in &name.segments[1..] {
                result = ResolvedExpr::Operation {
                    operator: "member".to_string(),
                    operands: vec![
                        result,
                        ResolvedExpr::Literal(Value::String(segment.clone())),
                    ],
                };
            }
            Ok(result)
        }
        // Initializers may refer to their own feature. Relationship resolution
        // excludes that feature to prevent reflexive specialization; applying
        // that exclusion here can bind an unrelated same-named declaration.
        Expr::Name(name) if is_self_feature_reference(usage, name) => {
            Ok(ResolvedExpr::FeaturePath {
                segments: vec![ResolvedPathSegment {
                    name: usage.declared_name.clone(),
                    feature_id: feature_id_from_qualified_name(&usage.qualified_name),
                }],
            })
        }
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
        )
        .or_else(|error| {
            lookup
                .resolve(name, &usage.owner_qualified_name, "", &mut BTreeSet::new())
                .map(|target| ResolvedExpr::FeaturePath {
                    segments: vec![ResolvedPathSegment {
                        name: name.as_dot_string(),
                        feature_id: target,
                    }],
                })
                .ok_or(error)
        }),
        Expr::Path { root, segment, .. } if expression_has_lexical_root(root, bindings) => {
            lexical_expression_target(expr, bindings, &lookup)?;
            Ok(ResolvedExpr::Operation {
                operator: "member".to_string(),
                operands: vec![
                    resolve(root)?,
                    ResolvedExpr::Literal(Value::String(segment.clone())),
                ],
            })
        }
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
            expr: Box::new(resolve(expr)?),
        }),
        Expr::Binary {
            left, op, right, ..
        } => Ok(ResolvedExpr::Binary {
            op: op.clone(),
            left: Box::new(resolve(left)?),
            right: Box::new(resolve(right)?),
        }),
        Expr::Call { function, args, .. } => Ok(ResolvedExpr::Call {
            function: function.clone(),
            args: args.iter().map(resolve).collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

// Resolve lexical members through the same inherited member lookup as model
// features. Untyped parameters remain dynamically bound; an explicit type must
// never allow a missing member to escape as an unchecked operation.
fn lexical_expression_target(
    expr: &Expr,
    bindings: &BTreeMap<String, Option<String>>,
    lookup: &FeatureLookup<'_>,
) -> Result<Option<String>, Diagnostic> {
    let member =
        |owner: Option<String>, name: &str| -> Result<Option<String>, Diagnostic> {
            owner
                .map(|owner| {
                    lookup.member(&owner, name, "", &mut BTreeSet::new()).ok_or_else(|| {
                Diagnostic::new(
                    format!("unresolved member `{name}` on expression parameter type `{owner}`"),
                    Some(expression_span(expr)),
                )
            })
                })
                .transpose()
        };
    match expr {
        Expr::Name(name) => {
            let mut target = name
                .segments
                .first()
                .and_then(|name| bindings.get(name))
                .cloned()
                .flatten();
            for segment in &name.segments[1..] {
                target = member(target, segment)?;
            }
            Ok(target)
        }
        Expr::Path { root, segment, .. } => {
            member(lexical_expression_target(root, bindings, lookup)?, segment)
        }
        Expr::Operation {
            operator, operands, ..
        } if matches!(operator.as_str(), "#" | ".?" | "all") => operands
            .first()
            .map(|root| lexical_expression_target(root, bindings, lookup))
            .transpose()
            .map(Option::flatten),
        _ => Ok(None),
    }
}

fn expression_has_lexical_root(expr: &Expr, bindings: &BTreeMap<String, Option<String>>) -> bool {
    match expr {
        Expr::Name(name) => name
            .segments
            .first()
            .is_some_and(|first| bindings.contains_key(first)),
        Expr::Path { root, .. } => expression_has_lexical_root(root, bindings),
        Expr::Operation { operands, .. } => operands
            .first()
            .is_some_and(|root| expression_has_lexical_root(root, bindings)),
        _ => false,
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

    if let Some(feature_id) = resolve_type_reference_in_scope(
        name,
        &usage.owner_qualified_name,
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
        resolve_type_reference_in_scope(
            &root_name,
            &usage.owner_qualified_name,
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
    if flatten_expression_path(expr).is_none() {
        if let Expr::Path {
            root,
            segment,
            span,
        } = expr
        {
            let lookup = feature_lookup::FeatureLookup {
                stdlib_ids,
                stdlib_feature_index,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
                definition_index,
                local_feature_index,
                local_usage_map,
            };
            let excluded = collected_usage_element_id(usage);
            let target = lookup
                .expression_target(
                    root,
                    &usage.owner_qualified_name,
                    &excluded,
                    &mut BTreeSet::new(),
                )
                .and_then(|target| lookup.member(&target, segment, &excluded, &mut BTreeSet::new()))
                .ok_or_else(|| {
                    Diagnostic::new(
                        format!("unresolved member `{segment}` of expression result"),
                        Some(span.clone()),
                    )
                })?;
            let root = resolve_expression(
                usage,
                root,
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
            return Ok(ResolvedExpr::Select {
                root: Box::new(root),
                segments: vec![ResolvedPathSegment {
                    name: segment.clone(),
                    feature_id: target,
                }],
            });
        }
    }
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
            let type_id = resolve_type_reference_in_scope(
                &name,
                &usage.owner_qualified_name,
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
fn feature_is_visible(
    target: &str,
    scope: &str,
    usages: &BTreeMap<String, CollectedUsage>,
) -> bool {
    let Some(usage) = target
        .strip_prefix("feature.")
        .and_then(|name| usages.get(name))
    else {
        return true;
    };
    !usage.modifiers.iter().any(|m| m == "private")
        || scope == usage.owner_qualified_name
        || scope.starts_with(&format!("{}.", usage.owner_qualified_name))
}

fn imported_feature_outside_scope(name: &QualifiedName, scope: &str, aliases: &ImportAliases) -> bool {
    let [simple] = name.segments.as_slice() else { return false; };
    aliases.value_aliases.get(simple).is_some_and(|target| target.starts_with("feature."))
        && aliases.value_alias_owners.get(simple).is_some_and(|owners| !owners.iter().any(|owner|
            owner.is_empty() || owner == "root" || scope == owner
                || scope.starts_with(&format!("{owner}."))))
}

/// Local namespaces take precedence over compatibility library root aliases.
fn has_local_reference_prefix(
    name: &QualifiedName,
    definitions: &BTreeMap<String, String>,
    features: &BTreeMap<String, BTreeMap<String, String>>,
    usages: &BTreeMap<String, CollectedUsage>,
    aliases: &BTreeMap<String, QualifiedName>,
) -> bool {
    (1..=name.segments.len()).any(|count| {
        let prefix = name.segments[..count].join(".");
        definitions.contains_key(&prefix) || features.contains_key(&prefix)
            || usages.contains_key(&prefix) || aliases.contains_key(&prefix)
    })
}

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
    // The selected membership controls access. In particular, a public alias
    // may expose a privately owned feature; do not filter that target again.
    let lookup = FeatureLookup {
        stdlib_ids, stdlib_feature_index, stdlib_aliases, local_definitions,
        local_aliases, import_aliases, definition_index, local_feature_index, local_usage_map,
    };
    if let Some(target) = lookup.resolve(name, &usage.owner_qualified_name,
        &collected_usage_element_id(usage), &mut BTreeSet::new())
        .filter(|target| target.starts_with("feature.")
            || stdlib_feature_index.values().any(|members| members.values().any(|id| id == target))) {
        return Some(target);
    }

    if !has_local_reference_prefix(name, local_definitions, local_feature_index, local_usage_map, local_aliases)
        && import_aliases.library_namespace_scope.resolve(&name.segments, stdlib_aliases) == Some(None) {
        return None;
    }

    if imported_feature_outside_scope(name, &usage.owner_qualified_name, import_aliases) {
        return None;
    }

    if name.segments.len() == 1
        && let Some(local) = unique_feature_modifier_alias_match_excluding(
            name.segments.first()?,
            local_feature_index,
            local_usage_map,
            &usage.qualified_name,
        )
    {
        let target = feature_id_from_qualified_name(&local);
        return feature_is_visible(&target, &usage.owner_qualified_name, local_usage_map)
            .then_some(target);
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
    .filter(|target| feature_is_visible(target, &usage.owner_qualified_name, local_usage_map))
    .filter(|target| {
        target.rsplit_once("::")
            .and_then(|(owner, member)| import_aliases.library_membership_visibility.get(&format!("{owner}.{member}")))
            .is_none_or(|visibility| visibility == "public")
    })
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
    let lookup = FeatureLookup {
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
    };
    let inherited_target = lookup.resolve(
        name,
        &usage.owner_qualified_name,
        &feature_id_from_qualified_name(&usage.qualified_name),
        &mut BTreeSet::new(),
    );
    if let Some(target) = inherited_target
        .as_ref()
        .filter(|target| target.starts_with("feature."))
    {
        return Some(target.clone());
    }

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
            return Some(normalize_feature_target_id(&ancestor_inherited));
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
            return Some(normalize_feature_target_id(&inherited));
        }
    }

    if let Some(type_name) = &usage.ty
        && let Some(type_id) = resolve_type_reference_in_scope(
            type_name,
            &usage.owner_qualified_name,
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

    inherited_target.filter(|target| !target.starts_with("type."))
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
        return Some(normalize_feature_target_id(&inherited));
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
    // The selected membership controls access. In particular, a public alias
    // may expose a privately owned feature; do not filter that target again.
    let lookup = FeatureLookup {
        stdlib_ids, stdlib_feature_index, stdlib_aliases, local_definitions,
        local_aliases, import_aliases, definition_index, local_feature_index, local_usage_map,
    };
    if let Some(target) = lookup.resolve(name, &usage.owner_qualified_name,
        &collected_usage_element_id(usage), &mut BTreeSet::new())
        .filter(|target| target.starts_with("feature.")
            || stdlib_feature_index.values().any(|members| members.values().any(|id| id == target))) {
        return Some(target);
    }

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
    .filter(|target| feature_is_visible(target, &usage.owner_qualified_name, local_usage_map))
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
    let lookup = FeatureLookup {
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
    };
    if let Some(target) = lookup
        .resolve(
            name,
            &usage.owner_qualified_name,
            &feature_id_from_qualified_name(&usage.qualified_name),
            &mut BTreeSet::new(),
        )
        .filter(|target| {
            target.starts_with("feature.")
                || stdlib_feature_index.values().any(|members| members.values().any(|id| id == target))
        })
    {
        return Some(target);
    }

    if imported_feature_outside_scope(name, &usage.owner_qualified_name, import_aliases) {
        return None;
    }

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
        let Some(parent_id) = resolve_type_reference_in_scope(
            parent,
            &definition.qualified_name,
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
        && let Some(type_id) = resolve_type_reference_in_scope(
            type_name,
            &owner_usage.owner_qualified_name,
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

/// Resolve aliases to exact local model identities. This bounded service uses
/// lexical candidates, never global suffix matches. External/imported targets
/// remain the responsibility of the full namespace service.
fn resolve_local_namespace_alias(
    alias: &str,
    aliases: &BTreeMap<String, QualifiedName>,
    definitions: &BTreeMap<String, CollectedDefinition>,
    usages: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    let mut current = alias.to_owned();
    let mut visited = BTreeSet::new();
    while visited.insert(current.clone()) {
        let target = aliases.get(&current)?;
        let owner = current.rsplit_once('.').map(|(owner, _)| owner).unwrap_or("root");
        let candidate = scoped_reference_candidates(&target.as_dot_string(), owner)
            .into_iter().find(|key| definitions.contains_key(key)
                || usages.contains_key(key) || aliases.contains_key(key))?;
        if let Some(member) = usages.get(&candidate) {
            return Some(collected_usage_element_id(member));
        }
        if definitions.contains_key(&candidate) { return Some(format!("type.{candidate}")); }
        current = candidate;
    }
    None
}

fn scoped_reference_candidates(target_name: &str, owner_qualified_name: &str) -> Vec<String> {
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
        "Documentation" => format!("doc.{}.{}.{}.{}", usage.owner_qualified_name, usage.declared_name, usage.span.start_line, usage.span.start_col),
        "TextualRepresentation" => format!("representation.{}.{}", usage.owner_qualified_name, usage.declared_name),
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
        return resolve_type_reference_in_scope(
            type_name,
            &usage.owner_qualified_name,
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
                return resolve_type_reference_in_scope(
                    target_type,
                    &target_usage.owner_qualified_name,
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
        .map(|qualified| normalize_feature_target_id(&qualified));
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
    let lookup = FeatureLookup {
        stdlib_ids,
        stdlib_feature_index,
        stdlib_aliases,
        local_definitions,
        local_aliases,
        import_aliases,
        definition_index,
        local_feature_index,
        local_usage_map,
    };
    if let Some(target) = lookup.resolve(
        name,
        &usage.owner_qualified_name,
        &collected_usage_element_id(usage),
        &mut BTreeSet::new(),
    ) {
        return Some(target);
    }
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
    let parent_type_id = resolve_type_reference_in_scope(
        parent_type_name,
        &parent_usage.owner_qualified_name,
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
    local_usages: Option<&BTreeMap<String, CollectedUsage>>,
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
    // A path may name a *usage* -- `import PartsTree::vehicle;` -- and only
    // definitions are bound above. Usages are consulted before the stdlib
    // last-segment fallback below, because a local declaration the path names
    // in full beats a loose suffix match against a library id.
    .or_else(|| local_usages.and_then(|usages| resolve_local_usage_reference(name, usages)))
    // An explicit public alias can name a usage. Resolve the alias membership
    // to its feature only after trying the written path itself.
    .or_else(|| local_usages.and_then(|usages| {
        local_aliases.get(&name.as_dot_string()).cloned()
            .or_else(|| unique_local_alias_suffix_match(&name.as_dot_string(), local_aliases))
            .and_then(|target| resolve_local_usage_reference(&target, usages))
    }))
    .or_else(|| {
        if name.segments.len() > 1 {
            unique_suffix_match(name.segments.last()?, stdlib_ids)
        } else {
            None
        }
    })
}

/// A qualified path against the usages declared in this module, exactly or by
/// unique suffix -- `Tree::vehicle` names `P.Tree.vehicle` -- mirroring how
/// [`unique_local_suffix_match`] treats definitions. A suffix matching more
/// than one usage is ambiguous and resolves to nothing.
fn resolve_local_usage_reference(
    name: &QualifiedName,
    local_usages: &BTreeMap<String, CollectedUsage>,
) -> Option<String> {
    let dotted = name.as_dot_string();
    if let Some(usage) = local_usages.get(&dotted) {
        return Some(collected_usage_element_id(usage));
    }

    let suffix = format!(".{dotted}");
    let mut matches = local_usages
        .iter()
        .filter(|(qualified_name, _)| qualified_name.ends_with(&suffix))
        .map(|(_, usage)| collected_usage_element_id(usage));
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
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
    // Generated defaults are already fully qualified library references.
    if name.span.start_line == 0 && name.segments.len() > 1 {
        if let Some(target) =
            resolve_explicit_type_reference(name, stdlib_ids, stdlib_aliases, local_definitions)
        {
            return Some(target);
        }
    }

    resolve_scoped_local_type_reference(name, owner_qualified_name, local_definitions)
        .or_else(|| {
            scoped_reference_candidates(&name.as_dot_string(), owner_qualified_name).into_iter()
                .find_map(|key| local_aliases.get(&key).and_then(|target|
                    resolve_visible_type_reference(target, owner_qualified_name, stdlib_ids, stdlib_aliases, local_definitions, local_aliases, import_aliases)))
        })
        .or_else(|| resolve_scoped_import_value_alias(name, owner_qualified_name, import_aliases))
        .or_else(|| {
            resolve_visible_type_reference(
                name,
                owner_qualified_name,
                stdlib_ids,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
            )
        })
        .or_else(|| {
            unconjugated_type_name(name).and_then(|unconjugated| {
                resolve_type_reference_in_scope(
                    &unconjugated,
                    owner_qualified_name,
                    stdlib_ids,
                    stdlib_aliases,
                    local_definitions,
                    local_aliases,
                    import_aliases,
                )
            })
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
    owner_qualified_name: &str,
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    local_definitions: &BTreeMap<String, String>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
) -> Option<String> {
    let mut expanded_name = name.clone();
    let mut seen_aliases = BTreeSet::new();
    while let Some(target) = local_aliases.get(&expanded_name.as_dot_string()) {
        if !seen_aliases.insert(expanded_name.as_dot_string()) {
            return None;
        }
        expanded_name = target.clone();
    }
    let name = &expanded_name;

    if name.segments.len() == 1 {
        let simple = &name.segments[0];
        if let Some(alias_target) = local_aliases.get(simple) {
            return resolve_visible_type_reference(
                alias_target,
                owner_qualified_name,
                stdlib_ids,
                stdlib_aliases,
                local_definitions,
                local_aliases,
                import_aliases,
            );
        }
        if let Some(imported) = import_aliases.value_aliases.get(simple) {
            if import_aliases.value_alias_owners.get(simple).is_some_and(|owners| owners.iter().any(|owner|
                owner == "root" || owner.is_empty() || owner_qualified_name == owner
                    || owner_qualified_name.starts_with(&format!("{owner}.")))) {
                return Some(imported.clone());
            }
        }
        if let Some(alias) = stdlib_aliases.get(simple) {
            return Some(alias.clone());
        }
        return unique_suffix_match(simple, stdlib_ids)
            .filter(|target| library_member_public(target, import_aliases));
    }

    if let Some(expanded) = expand_import_namespace_prefix(name, local_aliases, import_aliases) {
        return resolve_visible_type_reference(
            &expanded,
            owner_qualified_name,
            stdlib_ids,
            stdlib_aliases,
            local_definitions,
            local_aliases,
            import_aliases,
        );
    }

    if let Some(local) = local_definitions.get(&name.as_dot_string()) {
        return Some(local.clone());
    }

    if let Some(resolved) = import_aliases.library_namespace_scope.resolve(&name.segments, stdlib_aliases) {
        return resolved;
    }

    if let Some(imported) = import_aliases.value_aliases.get(&name.as_dot_string()) {
        return Some(imported.clone());
    }

    resolve_explicit_type_reference(name, stdlib_ids, stdlib_aliases, local_definitions)
        .filter(|target| stdlib_aliases.get(&name.as_colon_string()).is_some_and(|alias| alias == target)
            || library_member_public(target, import_aliases))
}

fn library_member_public(target: &str, import_aliases: &ImportAliases) -> bool {
    target.rsplit_once("::")
        .and_then(|(owner, member)| import_aliases.library_membership_visibility.get(&format!("{owner}.{member}")))
        .is_none_or(|visibility| visibility == "public")
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

#[cfg(test)]
mod tests {
    use super::*;
    use mercurio_foundation::kir::KirElement;
    use mercurio_foundation::language_contracts::ast::SourceSpan;

    use crate::language_frontend::lowering::emit::MappingBundle;
    use crate::language_frontend::lowering::indexes::build_stdlib_feature_index;

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
            membership_visibility: Default::default(),
            library_membership_visibility: Default::default(),
            library_namespace_scope: Default::default(),
            value_aliases: BTreeMap::new(),
            value_alias_owners: BTreeMap::new(),
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
            membership_visibility: Default::default(),
            library_membership_visibility: Default::default(),
            library_namespace_scope: Default::default(),
            value_aliases: BTreeMap::new(),
            value_alias_owners: BTreeMap::new(),
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
        let local_definitions = BTreeMap::from([("ItemTest.A".to_string(), "type.ItemTest.A".to_string())]);

        let resolved = resolve_type_reference_in_scope(
            &name,
            "ItemTest",
            &["ISQBase::ampere".to_string()],
            &stdlib_aliases,
            &local_definitions,
            &BTreeMap::new(),
            &ImportAliases::default(),
        );

        assert_eq!(resolved.as_deref(), Some("type.ItemTest.A"));
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

        let index = build_stdlib_feature_index(&stdlib, &mappings).unwrap();

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

        let index = build_stdlib_feature_index(&stdlib, &mappings).unwrap();

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

        let index = build_stdlib_feature_index(&stdlib, &mappings).unwrap();

        assert_eq!(
            index
                .get("SpatialItems::SpatialItem::coordinateFrame")
                .and_then(|features| features.get("mRefs"))
                .map(String::as_str),
            Some("MeasurementReferences::TensorMeasurementReference::mRefs")
        );
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

mod feature_lookup;
use feature_lookup::FeatureLookup;

mod typing;
