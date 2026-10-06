//! Translate resolver symbol keys to the identities chosen by emission templates.
//! Only typed reference slots are rewritten; names, text and literal values are not.
use super::*;
use crate::language_frontend::lowering::emit::{render_collected_usage_id, render_usage_id};

pub(super) fn finalize(
    module: &mut ResolvedModule,
    context: &ResolverContext,
    mappings: &MappingBundle,
) -> Result<(), Diagnostic> {
    let mut namespaces = context.local_definitions.clone();
    for package in context.packages.iter().chain(&module.packages) {
        namespaces.insert(
            package.qualified_name.clone(),
            format!("pkg.{}", package.qualified_name),
        );
    }
    let mut pending = context.local_usage_map.values().collect::<Vec<_>>();
    pending.sort_by_key(|usage| usage.qualified_name.matches('.').count());
    let mut ids = BTreeMap::new();
    for usage in pending {
        let owner = namespaces
            .get(&usage.owner_qualified_name)
            .map(String::as_str)
            .unwrap_or("pkg.root");
        let id = render_collected_usage_id(usage, owner, mappings)?;
        namespaces.insert(usage.qualified_name.clone(), id.clone());
        ids.insert(feature_id_from_qualified_name(&usage.qualified_name), id);
    }
    // The declaration being compiled wins over a same-named support declaration.
    let mut local = module.usages.iter().collect::<Vec<_>>();
    local.extend(
        module
            .definitions
            .iter()
            .flat_map(|definition| &definition.members),
    );
    let mut all = Vec::new();
    while let Some(usage) = local.pop() {
        local.extend(&usage.members);
        all.push(usage);
    }
    all.sort_by_key(|usage| usage.qualified_name.matches('.').count());
    for usage in all {
        let owner = namespaces
            .get(&usage.owner_qualified_name)
            .map(String::as_str)
            .unwrap_or("pkg.root");
        let id = render_usage_id(usage, owner, mappings)?;
        namespaces.insert(usage.qualified_name.clone(), id.clone());
        ids.insert(feature_id_from_qualified_name(&usage.qualified_name), id);
    }
    for import in &mut module.imports {
        replace(&mut import.target_id, &ids);
        if let Some(target) = &mut import.referenced_element_id { replace(target, &ids); }
        for usage in &mut import.members { remap_usage(usage, &ids)?; }
    }
    for alias in &mut module.aliases {
        replace(&mut alias.target, &ids);
        for usage in &mut alias.members { remap_usage(usage, &ids)?; }
    }
    for definition in &mut module.definitions {
        replace_many(&mut definition.specializes, &ids);
        for usage in &mut definition.members {
            remap_usage(usage, &ids)?;
        }
    }
    for usage in &mut module.usages {
        remap_usage(usage, &ids)?;
    }
    Ok(())
}

fn replace(value: &mut String, ids: &BTreeMap<String, String>) {
    if let Some(id) = ids.get(value) {
        *value = id.clone();
    }
}
fn replace_many(values: &mut [String], ids: &BTreeMap<String, String>) {
    for value in values {
        replace(value, ids);
    }
}
fn remap_usage(usage: &mut ResolvedUsage, ids: &BTreeMap<String, String>) -> Result<(), Diagnostic> {
    if let Some(mut endpoints) = crate::language_frontend::lowering::relationship_declarations::load::<crate::language_frontend::lowering::relationship_declarations::Operand<String>>(&usage.metadata_properties, &usage.span)? {
        for operand in endpoints.sources.iter_mut().chain(&mut endpoints.targets) {
            replace_many(&mut operand.steps, ids);
            if let Some(ty) = &mut operand.type_ref { replace(ty, ids); }
        }
        crate::language_frontend::lowering::relationship_declarations::store(&mut usage.metadata_properties, &endpoints, &usage.span)?;
    }
    for value in [
        &mut usage.type_ref,
        &mut usage.reference_target,
        &mut usage.allocation_source,
        &mut usage.allocation_target,
    ]
    .into_iter()
    .flatten()
    {
        replace(value, ids);
    }
    for values in [
        &mut usage.additional_type_refs,
        &mut usage.related_features,
        &mut usage.specializes,
        &mut usage.specialized_features,
        &mut usage.subsetted_features,
        &mut usage.redefined_features,
    ] {
        replace_many(values, ids);
    }
    if let Some(expression) = &mut usage.expression {
        remap_expression(expression, ids);
    }
    for member in &mut usage.members {
        remap_usage(member, ids)?;
    }
    Ok(())
}
fn remap_expression(expression: &mut ResolvedExpr, ids: &BTreeMap<String, String>) {
    match expression {
        ResolvedExpr::TypeReference { target } => replace(target, ids),
        ResolvedExpr::FeaturePath { segments } => {
            for segment in segments {
                replace(&mut segment.feature_id, ids);
            }
        }
        ResolvedExpr::Select { root, segments } => {
            remap_expression(root, ids);
            for segment in segments {
                replace(&mut segment.feature_id, ids);
            }
        }
        ResolvedExpr::Operation { operands, .. } | ResolvedExpr::Tuple { items: operands } => {
            for operand in operands {
                remap_expression(operand, ids);
            }
        }
        ResolvedExpr::Call { function, args } => {
            replace(function, ids);
            for arg in args {
                remap_expression(arg, ids);
            }
        }
        ResolvedExpr::Unary { expr, .. } | ResolvedExpr::NamedArgument { value: expr, .. } => {
            remap_expression(expr, ids)
        }
        ResolvedExpr::Binary { left, right, .. } => {
            remap_expression(left, ids);
            remap_expression(right, ids);
        }
        ResolvedExpr::Lambda { parameters, body } => {
            for parameter in parameters {
                if let Some(ty) = &mut parameter.type_ref {
                    replace(ty, ids);
                }
                if let Some(default) = &mut parameter.default {
                    remap_expression(default, ids);
                }
            }
            remap_expression(body, ids);
        }
        ResolvedExpr::Literal(_) | ResolvedExpr::Variable { .. } | ResolvedExpr::SelfRef => {}
    }
}
