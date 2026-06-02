//! Import alias construction for lowering resolution.

use std::collections::BTreeMap;

use mercurio_language_contracts::ast::{QualifiedName, SourceSpan};
use mercurio_language_contracts::diagnostics::Diagnostic;

use crate::lowering::collect::{CollectedDefinition, CollectedUsage, ImportAliases};
use crate::lowering::ir::{ResolvedImport, ResolvedPackage};
use crate::lowering::names::{
    direct_child_name, dotted_name_to_qualified_name, import_namespace_prefix,
    qualified_names_match, resolve_local_namespace_dot,
};
use crate::lowering::policy::ResolvePolicy;
pub(crate) fn build_import_alias_map(
    imports: &[ResolvedImport],
    packages: &[ResolvedPackage],
    definitions: &[CollectedDefinition],
    usages: &BTreeMap<String, CollectedUsage>,
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

    for import in imports {
        if import.target_id.ends_with("::*") || import.target_id.ends_with("::**") {
            let namespace = import_namespace_prefix(&import.target_id);
            add_wildcard_import_aliases(
                &namespace,
                import.owner_package_qualified_name.as_deref().unwrap_or(""),
                &root_package,
                packages,
                definitions,
                usages,
                stdlib_ids,
                stdlib_aliases,
                &mut aliases,
                &import.span,
                policy,
            )?;
            continue;
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
                &import.span,
                policy,
            )?;
            bind_owner_qualified_value_aliases(
                &mut aliases,
                import.owner_package_qualified_name.as_deref().unwrap_or(""),
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
    owner_package_qualified_name: &str,
    root_package: &Option<String>,
    packages: &[ResolvedPackage],
    definitions: &[CollectedDefinition],
    usages: &BTreeMap<String, CollectedUsage>,
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

        for package in packages {
            if let Some(child) = direct_child_name(&package.qualified_name, &namespace_prefix) {
                bind_namespace_alias(
                    aliases,
                    child,
                    dotted_name_to_qualified_name(&package.qualified_name, span),
                    span,
                    policy,
                )?;
            }
        }

        for definition in definitions {
            if let Some(child) = direct_child_name(&definition.qualified_name, &namespace_prefix) {
                let target = format!("type.{}", definition.qualified_name);
                bind_value_alias(aliases, child, target.clone(), span, policy)?;
                bind_owner_qualified_value_aliases(
                    aliases,
                    owner_package_qualified_name,
                    child,
                    target,
                    span,
                    policy,
                )?;
            }
        }

        for usage in usages.values() {
            if let Some(child) = direct_child_name(&usage.qualified_name, &namespace_prefix) {
                let target = format!("feature.{}", usage.qualified_name);
                bind_value_alias(aliases, child, target.clone(), span, policy)?;
                bind_owner_qualified_value_aliases(
                    aliases,
                    owner_package_qualified_name,
                    child,
                    target,
                    span,
                    policy,
                )?;
            }
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
                bind_value_alias(aliases, child, id.clone(), span, policy)?;
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
            bind_value_alias(aliases, short_alias, target.clone(), span, policy)?;
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

fn bind_value_alias(
    aliases: &mut ImportAliases,
    alias: &str,
    target: String,
    _span: &SourceSpan,
    _policy: ResolvePolicy,
) -> Result<(), Diagnostic> {
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
    span: &SourceSpan,
    policy: ResolvePolicy,
) -> Result<(), Diagnostic> {
    if owner_package_qualified_name.is_empty() {
        return Ok(());
    }

    let segments = owner_package_qualified_name.split('.').collect::<Vec<_>>();
    for start in 0..segments.len() {
        let qualified_alias = format!("{}.{}", segments[start..].join("."), alias);
        bind_value_alias(aliases, &qualified_alias, target.clone(), span, policy)?;
    }
    Ok(())
}
