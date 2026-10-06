//! Import alias construction for lowering resolution.

use std::collections::{BTreeMap, BTreeSet};

use mercurio_foundation::language_contracts::ast::{QualifiedName, SourceSpan};
use mercurio_foundation::language_contracts::diagnostics::Diagnostic;

use crate::language_frontend::lowering::collect::{
    CollectedDefinition, CollectedUsage, ImportAliases,
};
use crate::language_frontend::lowering::ir::{ResolvedImport, ResolvedPackage};
use crate::language_frontend::lowering::names::{
    direct_child_name, dotted_name_to_qualified_name, import_namespace_prefix,
    qualified_names_match, resolve_local_namespace_dot,
};
use crate::language_frontend::lowering::policy::ResolvePolicy;
pub(crate) fn build_import_alias_map(
    imports: &[ResolvedImport],
    packages: &[ResolvedPackage],
    definitions: &[CollectedDefinition],
    usages: &BTreeMap<String, CollectedUsage>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    non_public_members: &BTreeSet<String>,
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    policy: ResolvePolicy,
) -> Result<ImportAliases, Diagnostic> {
    let mut aliases = ImportAliases::default();
    let root_package = packages
        .iter()
        .find(|package| package.owner_package_qualified_name.is_none())
        .or_else(|| packages.first())
        .map(|package| package.qualified_name.clone());

    // Close public re-exports before binding aliases. Each (owner, target)
    // pair is visited once, so mutually importing namespaces terminate.
    let mut expanded = imports.to_vec();
    let mut index = 0;
    while index < expanded.len() {
        let import = expanded[index].clone();
        index += 1;
        if !(import.target_id.ends_with("::*") || import.target_id.ends_with("::**"))
        {
            continue;
        }
        let namespace = import_namespace_prefix(&import.target_id);
        let Some(namespace) = resolve_local_namespace_dot(
            &namespace,
            import.owner_qualified_name.as_deref().unwrap_or(""),
            &root_package,
            packages,
        ) else {
            continue;
        };
        let mut forwarded_imports = Vec::new();
        for exported in imports.iter().filter(|candidate| {
            (candidate.is_public || import.is_import_all)
                && candidate.owner_qualified_name.as_deref() == Some(namespace.as_str())
        }) {
            let mut forwarded = exported.clone();
            // Resolve a relative re-export in its declaring scope before
            // forwarding it into the consumer's namespace.
            if exported.target_id.ends_with("::*") || exported.target_id.ends_with("::**") {
                let target = import_namespace_prefix(&exported.target_id);
                if let Some(target) = resolve_local_namespace_dot(
                    &target,
                    exported.owner_qualified_name.as_deref().unwrap_or(""),
                    &root_package,
                    packages,
                ) {
                    let wildcard = if exported.target_id.ends_with("::**") {
                        "**"
                    } else {
                        "*"
                    };
                    forwarded.target_id = format!("{}::{wildcard}", target.replace('.', "::"));
                }
            }
            forwarded.owner_qualified_name = import.owner_qualified_name.clone();
            forwarded.is_public = import.is_public;
            forwarded.is_import_all |= import.is_import_all;
            if !expanded.iter().any(|existing| {
                existing.owner_qualified_name == forwarded.owner_qualified_name
                    && existing.target_id == forwarded.target_id
                    && existing.imported_name == forwarded.imported_name
            }) {
                forwarded_imports.push(forwarded);
            }
        }
        expanded.splice(index..index, forwarded_imports);
    }
    for import in &expanded {
        if import.target_id.ends_with("::*") || import.target_id.ends_with("::**") {
            let namespace = import_namespace_prefix(&import.target_id);
            add_wildcard_import_aliases(
                &namespace,
                import.target_id.ends_with("::**"),
                import.is_import_all,
                import.owner_qualified_name.as_deref().unwrap_or(""),
                &root_package,
                packages,
                definitions,
                usages,
                local_aliases,
                non_public_members,
                stdlib_ids,
                stdlib_aliases,
                &mut aliases,
                &import.span,
                policy,
            )?;
            continue;
        }

        if let Some(namespace) = import.target_id.strip_prefix("package.") {
            if let Some(alias) = import.imported_name.as_deref() {
                bind_namespace_alias(
                    &mut aliases,
                    alias,
                    dotted_name_to_qualified_name(namespace, &import.span),
                    &import.span,
                    policy,
                )?;
            }
        }
        if let Some(alias) = import
            .imported_name
            .as_deref()
            .or_else(|| import.target_id.rsplit("::").next())
        {
            bind_value_alias(
                &mut aliases,
                alias,
                import.target_id.clone(),
                import.owner_qualified_name.as_deref().unwrap_or("root"),
                &import.span,
                policy,
            )?;
            bind_owner_qualified_value_aliases(
                &mut aliases,
                import.owner_qualified_name.as_deref().unwrap_or(""),
                alias,
                import.target_id.clone(),
                &import.span,
                policy,
            )?;
        }
    }
    Ok(aliases)
}

#[allow(clippy::too_many_arguments)]
fn add_wildcard_import_aliases(
    namespace: &str,
    recursive: bool,
    include_all: bool,
    owner_package_qualified_name: &str,
    root_package: &Option<String>,
    packages: &[ResolvedPackage],
    definitions: &[CollectedDefinition],
    usages: &BTreeMap<String, CollectedUsage>,
    local_aliases: &BTreeMap<String, QualifiedName>,
    non_public_members: &BTreeSet<String>,
    stdlib_ids: &[String],
    stdlib_aliases: &BTreeMap<String, String>,
    aliases: &mut ImportAliases,
    span: &SourceSpan,
    policy: ResolvePolicy,
) -> Result<(), Diagnostic> {
    let local_namespace = resolve_local_namespace_dot(
        namespace,
        owner_package_qualified_name,
        root_package,
        packages,
    );

    if let Some(namespace_dot) = local_namespace {
        if let Some(alias) = namespace_dot.rsplit('.').next() {
            bind_namespace_alias(
                aliases,
                alias,
                dotted_name_to_qualified_name(&namespace_dot, span),
                span,
                policy,
            )?;
        }

        let namespace_prefix = format!("{namespace_dot}.");
        // Recursive traversal must not enter a private/protected namespace.
        // `import all` admits every membership and every traversed namespace.
        let visible = |qualified: &str| {
            if include_all { return true; }
            let mut cursor = qualified;
            while cursor != namespace_dot {
                if non_public_members.contains(cursor) { return false; }
                let Some((parent, _)) = cursor.rsplit_once('.') else { break; };
                cursor = parent;
            }
            true
        };


        for package in packages {
            if !visible(&package.qualified_name) { continue; }
            if let Some(child) =
                imported_child_name(&package.qualified_name, &namespace_prefix, recursive)
            {
                bind_namespace_alias(
                    aliases,
                    child,
                    dotted_name_to_qualified_name(&package.qualified_name, span),
                    span,
                    policy,
                )?;
            }
        }

        let mut candidates: Vec<(&str, String, &str, SourceSpan)> = Vec::new();
        for definition in definitions {
            if !visible(&definition.qualified_name) {
                continue;
            }
            if let Some(child) =
                imported_child_name(&definition.qualified_name, &namespace_prefix, recursive)
            {
                let target = format!("type.{}", definition.qualified_name);
                candidates.push((
                    child,
                    target,
                    &definition.qualified_name,
                    definition.span.clone(),
                ));
            }
        }

        for (qualified_alias, name) in local_aliases {
            if !visible(qualified_alias) { continue; }
            let Some(child) = imported_child_name(qualified_alias, &namespace_prefix, recursive)
            else {
                continue;
            };
            let mut name = name.clone();
            let mut seen = std::collections::BTreeSet::new();
            while let Some(next) = local_aliases.get(&name.as_dot_string()) {
                if !seen.insert(name.as_dot_string()) {
                    break;
                }
                name = next.clone();
            }
            let colon = name.as_colon_string();
            let dotted = name.as_dot_string();
            let target = stdlib_aliases
                .get(&colon)
                .cloned()
                .or_else(|| stdlib_ids.iter().find(|id| *id == &colon).cloned())
                .or_else(|| {
                    definitions
                        .iter()
                        .find(|definition| definition.qualified_name == dotted)
                        .map(|definition| format!("type.{}", definition.qualified_name))
                })
                .or_else(|| {
                    usages
                        .get(&dotted)
                        .map(|usage| format!("feature.{}", usage.qualified_name))
                });
            if let Some(target) = target {
                candidates.push((child, target, qualified_alias, name.span.clone()));
            }
        }
        for usage in usages.values() {
            if !visible(&usage.qualified_name) { continue; }
            if let Some(child) =
                imported_child_name(&usage.qualified_name, &namespace_prefix, recursive)
            {
                let target = format!("feature.{}", usage.qualified_name);
                candidates.push((child, target, &usage.qualified_name, usage.span.clone()));
            }
        }
        if recursive {
            candidates.sort_by_key(|(_, _, qualified, member_span)| {
                let relative = qualified
                    .strip_prefix(&namespace_prefix)
                    .unwrap_or(qualified);
                let segments = relative.split('.').collect::<Vec<_>>();
                let mut owner = namespace_dot.clone();
                let mut order = Vec::new();
                for segment in segments.iter().take(segments.len().saturating_sub(1)) {
                    owner.push('.');
                    owner.push_str(segment);
                    let owner_span = packages
                        .iter()
                        .find(|p| p.qualified_name == owner)
                        .map(|p| &p.span)
                        .or_else(|| {
                            definitions
                                .iter()
                                .find(|d| d.qualified_name == owner)
                                .map(|d| &d.span)
                        })
                        .or_else(|| usages.get(&owner).map(|u| &u.span));
                    order.extend([
                        1,
                        owner_span.map_or(0, |span| span.start_line),
                        owner_span.map_or(0, |span| span.start_col),
                    ]);
                }
                order.extend([0, member_span.start_line, member_span.start_col]);
                order
            });
        }
        for (child, target, _, _) in candidates {
            bind_value_alias(aliases, child, target.clone(), owner_package_qualified_name, span, policy)?;
            bind_owner_qualified_value_aliases(
                aliases,
                owner_package_qualified_name,
                child,
                target,
                span,
                policy,
            )?;
        }
    } else {
        if let Some(alias) = namespace.rsplit("::").next() {
            bind_namespace_alias(
                aliases,
                alias,
                dotted_name_to_qualified_name(&namespace.replace("::", "."), span),
                span,
                policy,
            )?;
        }

        let namespace_prefix = format!("{namespace}::");
        for id in stdlib_ids {
            if let Some(child) = direct_child_name(id, &namespace_prefix) {
                bind_value_alias(aliases, child, id.clone(), owner_package_qualified_name, span, policy)?;
                bind_owner_qualified_value_aliases(
                    aliases,
                    owner_package_qualified_name,
                    child,
                    id.clone(),
                    span,
                    policy,
                )?;
            }
        }
        let namespace_alias_prefix = format!("{namespace_prefix}");
        for (alias, target) in stdlib_aliases {
            let Some(short_alias) = alias.strip_prefix(&namespace_alias_prefix) else {
                continue;
            };
            if short_alias.contains("::") {
                continue;
            }
            bind_value_alias(aliases, short_alias, target.clone(), owner_package_qualified_name, span, policy)?;
            bind_owner_qualified_value_aliases(
                aliases,
                owner_package_qualified_name,
                short_alias,
                target.clone(),
                span,
                policy,
            )?;
        }
    }

    Ok(())
}

fn imported_child_name<'a>(
    qualified_name: &'a str,
    namespace_prefix: &str,
    recursive: bool,
) -> Option<&'a str> {
    if recursive {
        qualified_name
            .strip_prefix(namespace_prefix)
            .filter(|name| !name.is_empty())?
            .rsplit('.')
            .next()
    } else {
        direct_child_name(qualified_name, namespace_prefix)
    }
}

fn bind_value_alias(
    aliases: &mut ImportAliases,
    alias: &str,
    target: String,
    owner: &str,
    _span: &SourceSpan,
    _policy: ResolvePolicy,
) -> Result<(), Diagnostic> {
    aliases.value_alias_owners.entry(alias.to_string()).or_default().insert(owner.to_string());
    if aliases.ambiguous_value_aliases.contains(alias) {
        return Ok(());
    }

    match aliases.value_aliases.get(alias) {
        Some(existing) if existing != &target => {
            aliases.value_aliases.remove(alias);
            aliases.ambiguous_value_aliases.insert(alias.to_string());
            Ok(())
        }
        _ => {
            aliases.value_aliases.insert(alias.to_string(), target);
            Ok(())
        }
    }
}

fn bind_namespace_alias(
    aliases: &mut ImportAliases,
    alias: &str,
    target: QualifiedName,
    _span: &SourceSpan,
    _policy: ResolvePolicy,
) -> Result<(), Diagnostic> {
    if aliases.ambiguous_namespace_aliases.contains(alias) {
        return Ok(());
    }

    match aliases.namespace_aliases.get(alias) {
        Some(existing) if !qualified_names_match(existing, &target) => {
            aliases.namespace_aliases.remove(alias);
            aliases
                .ambiguous_namespace_aliases
                .insert(alias.to_string());
            Ok(())
        }
        _ => {
            aliases.namespace_aliases.insert(alias.to_string(), target);
            Ok(())
        }
    }
}

fn bind_owner_qualified_value_aliases(
    aliases: &mut ImportAliases,
    owner_package_qualified_name: &str,
    alias: &str,
    target: String,
    _span: &SourceSpan,
    _policy: ResolvePolicy,
) -> Result<(), Diagnostic> {
    if owner_package_qualified_name.is_empty() {
        return Ok(());
    }

    let segments = owner_package_qualified_name.split('.').collect::<Vec<_>>();
    for start in 0..segments.len() {
        let qualified_alias = format!("{}.{}", segments[start..].join("."), alias);
        aliases
            .value_aliases
            .entry(qualified_alias)
            .or_insert_with(|| target.clone());
    }
    Ok(())
}
