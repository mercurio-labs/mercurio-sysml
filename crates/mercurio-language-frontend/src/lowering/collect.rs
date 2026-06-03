//! AST collection phase.

use std::collections::{BTreeMap, BTreeSet};

use mercurio_language_contracts::ast::{
    AliasDecl, Declaration, Expr, GenericDefinitionDecl, GenericUsageDecl, ImportDecl,
    MultiplicityRange, PackageDecl, ParsedModule as SysmlModule, QualifiedName, SourceSpan,
};
use mercurio_language_contracts::diagnostics::Diagnostic;

use crate::lowering::elaborate::should_annotate_connection_end_direction;
use crate::lowering::emit::MappingBundle;
use crate::lowering::ir::ResolvedPackage;
use crate::lowering::rules::LoweringRule;

#[derive(Debug, Clone, Default)]
pub(crate) struct CollectedModule {
    pub(crate) packages: Vec<ResolvedPackage>,
    pub(crate) imports: Vec<CollectedImport>,
    pub(crate) definitions: Vec<CollectedDefinition>,
    pub(crate) usages: Vec<CollectedUsage>,
    pub(crate) aliases: Vec<CollectedAlias>,
}

#[derive(Debug, Clone)]
pub(crate) struct CollectedImport {
    pub(crate) owner_package_qualified_name: Option<String>,
    pub(crate) decl: ImportDecl,
}

#[derive(Debug, Clone)]
pub(crate) struct CollectedDefinition {
    pub(crate) construct: String,
    pub(crate) qualified_name: String,
    pub(crate) declared_name: String,
    pub(crate) is_abstract: bool,
    pub(crate) specializes: Vec<QualifiedName>,
    pub(crate) members: Vec<CollectedUsage>,
    pub(crate) docs: Vec<String>,
    pub(crate) span: SourceSpan,
}

#[derive(Debug, Clone)]
pub(crate) struct CollectedUsage {
    pub(crate) construct: String,
    pub(crate) owner_construct: String,
    pub(crate) owner_qualified_name: String,
    pub(crate) qualified_name: String,
    pub(crate) declared_name: String,
    pub(crate) is_implicit_name: bool,
    pub(crate) ty: Option<QualifiedName>,
    pub(crate) additional_types: Vec<QualifiedName>,
    pub(crate) reference_target: Option<QualifiedName>,
    pub(crate) allocation_source: Option<QualifiedName>,
    pub(crate) allocation_target: Option<QualifiedName>,
    pub(crate) metadata_properties: BTreeMap<String, String>,
    pub(crate) multiplicity: Option<MultiplicityRange>,
    pub(crate) expression: Option<Expr>,
    pub(crate) specializes: Vec<QualifiedName>,
    pub(crate) subsets: Vec<QualifiedName>,
    pub(crate) redefines: Vec<QualifiedName>,
    pub(crate) members: Vec<CollectedUsage>,
    pub(crate) modifiers: Vec<String>,
    pub(crate) docs: Vec<String>,
    pub(crate) span: SourceSpan,
}

#[derive(Debug, Clone)]
pub(crate) struct CollectedAlias {
    pub(crate) qualified_name: String,
    pub(crate) declared_name: String,
    pub(crate) target: QualifiedName,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ImportAliases {
    pub(crate) value_aliases: BTreeMap<String, String>,
    pub(crate) namespace_aliases: BTreeMap<String, QualifiedName>,
    pub(crate) ambiguous_value_aliases: BTreeSet<String>,
    pub(crate) ambiguous_namespace_aliases: BTreeSet<String>,
}

pub(crate) fn collect_module(
    module: &SysmlModule,
    mappings: &MappingBundle,
) -> Result<CollectedModule, Diagnostic> {
    let mut packages = Vec::new();
    let mut imports = Vec::new();
    let mut definitions = Vec::new();
    let mut usages = Vec::new();
    let mut aliases = Vec::new();

    let root_members = if !module.members.is_empty() {
        module.members.clone()
    } else if let Some(package) = &module.package {
        vec![Declaration::Package(package.clone())]
    } else {
        Vec::new()
    };

    collect_declarations(
        &root_members,
        &[],
        None,
        &mut packages,
        &mut imports,
        &mut definitions,
        &mut usages,
        &mut aliases,
        mappings,
    )?;
    collect_nested_aliases(&root_members, &[], None, &mut aliases);

    Ok(CollectedModule {
        packages,
        imports,
        definitions,
        usages,
        aliases,
    })
}

pub(crate) fn collect_modules(
    modules: &[SysmlModule],
    mappings: &MappingBundle,
) -> Result<CollectedModule, Diagnostic> {
    let mut collected = CollectedModule::default();

    for module in modules {
        let module = collect_module(module, mappings)?;
        collected.packages.extend(module.packages);
        collected.imports.extend(module.imports);
        collected.definitions.extend(module.definitions);
        collected.usages.extend(module.usages);
        collected.aliases.extend(module.aliases);
    }

    Ok(collected)
}

#[allow(clippy::too_many_arguments)]
fn collect_declarations(
    declarations: &[Declaration],
    owner_package_segments: &[String],
    owner_package_qualified_name: Option<&str>,
    packages: &mut Vec<ResolvedPackage>,
    imports: &mut Vec<CollectedImport>,
    definitions: &mut Vec<CollectedDefinition>,
    usages: &mut Vec<CollectedUsage>,
    aliases: &mut Vec<CollectedAlias>,
    mappings: &MappingBundle,
) -> Result<(), Diagnostic> {
    for declaration in declarations {
        if let Some(definition) = declaration.as_definition_like() {
            let qualified_segments =
                qualify_segments(owner_package_segments, &[definition.name.clone()]);
            definitions.push(collect_generic_definition(
                &definition,
                owner_package_segments,
                mappings,
            )?);
            collect_nested_owned_definitions(
                &definition.members,
                &qualified_segments,
                definitions,
                mappings,
            )?;
            collect_nested_member_imports(
                &definition.members,
                &qualified_segments.join("."),
                imports,
            );
            collect_nested_owned_packages(
                &definition.members,
                &qualified_segments,
                packages,
                imports,
                definitions,
                usages,
                aliases,
                mappings,
            )?;
            continue;
        }
        if let Some(usage) = declaration.as_usage_like() {
            let owner = owner_package_qualified_name.unwrap_or("root");
            usages.push(collect_generic_usage(&usage, owner, "Package", mappings)?);
            let qualified_name = usage_qualified_name(owner, &usage.name);
            collect_nested_member_imports(&usage.body_members, &qualified_name, imports);
            continue;
        }

        match declaration {
            Declaration::Package(package) => collect_package(
                package,
                owner_package_segments,
                packages,
                imports,
                definitions,
                usages,
                aliases,
                mappings,
            )?,
            Declaration::Import(import_decl) => imports.push(CollectedImport {
                owner_package_qualified_name: owner_package_qualified_name.map(str::to_string),
                decl: import_decl.clone(),
            }),
            Declaration::Alias(alias) => aliases.push(collect_alias(alias, owner_package_segments)),
            _ => unreachable!("definition-like and usage-like declarations are handled above"),
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn collect_nested_owned_packages(
    declarations: &[Declaration],
    owner_package_segments: &[String],
    packages: &mut Vec<ResolvedPackage>,
    imports: &mut Vec<CollectedImport>,
    definitions: &mut Vec<CollectedDefinition>,
    usages: &mut Vec<CollectedUsage>,
    aliases: &mut Vec<CollectedAlias>,
    mappings: &MappingBundle,
) -> Result<(), Diagnostic> {
    for declaration in declarations {
        if let Declaration::Package(package) = declaration {
            collect_package(
                package,
                owner_package_segments,
                packages,
                imports,
                definitions,
                usages,
                aliases,
                mappings,
            )?;
        }
    }

    Ok(())
}

fn collect_nested_owned_definitions(
    declarations: &[Declaration],
    owner_package_segments: &[String],
    definitions: &mut Vec<CollectedDefinition>,
    mappings: &MappingBundle,
) -> Result<(), Diagnostic> {
    for declaration in declarations {
        if let Some(definition) = declaration.as_definition_like() {
            definitions.push(collect_generic_definition(
                &definition,
                owner_package_segments,
                mappings,
            )?);
            collect_nested_owned_definitions(
                &definition.members,
                &qualify_segments(owner_package_segments, &[definition.name.clone()]),
                definitions,
                mappings,
            )?;
        }
    }

    Ok(())
}

fn collect_nested_member_imports(
    declarations: &[Declaration],
    owner_qualified_name: &str,
    imports: &mut Vec<CollectedImport>,
) {
    for declaration in declarations {
        if let Some(usage) = declaration.as_usage_like() {
            let qualified_name = usage_qualified_name(owner_qualified_name, &usage.name);
            collect_nested_member_imports(&usage.body_members, &qualified_name, imports);
            continue;
        }
        if let Some(definition) = declaration.as_definition_like() {
            let qualified_name = usage_qualified_name(owner_qualified_name, &definition.name);
            collect_nested_member_imports(&definition.members, &qualified_name, imports);
            continue;
        }

        match declaration {
            Declaration::Import(import_decl) => imports.push(CollectedImport {
                owner_package_qualified_name: Some(owner_qualified_name.to_string()),
                decl: import_decl.clone(),
            }),
            Declaration::Package(package) => {
                let qualified_name =
                    usage_qualified_name(owner_qualified_name, &package.name.as_dot_string());
                collect_nested_member_imports(&package.members, &qualified_name, imports);
            }
            Declaration::Alias(_) => {}
            _ => unreachable!("definition-like and usage-like declarations are handled above"),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_package(
    package: &PackageDecl,
    owner_package_segments: &[String],
    packages: &mut Vec<ResolvedPackage>,
    imports: &mut Vec<CollectedImport>,
    definitions: &mut Vec<CollectedDefinition>,
    usages: &mut Vec<CollectedUsage>,
    aliases: &mut Vec<CollectedAlias>,
    mappings: &MappingBundle,
) -> Result<(), Diagnostic> {
    let package_segments = qualify_segments(owner_package_segments, &package.name.segments);
    let qualified_name = package_segments.join(".");

    packages.push(ResolvedPackage {
        owner_package_qualified_name: (!owner_package_segments.is_empty())
            .then(|| owner_package_segments.join(".")),
        qualified_name: qualified_name.clone(),
        declared_name: package
            .name
            .segments
            .last()
            .cloned()
            .unwrap_or_else(|| qualified_name.clone()),
        docs: package.docs.clone(),
        span: package.span.clone(),
    });

    collect_declarations(
        &package.members,
        &package_segments,
        Some(&qualified_name),
        packages,
        imports,
        definitions,
        usages,
        aliases,
        mappings,
    )
}

fn collect_generic_definition(
    definition: &GenericDefinitionDecl,
    owner_package_segments: &[String],
    mappings: &MappingBundle,
) -> Result<CollectedDefinition, Diagnostic> {
    let qualified_name = qualify_name(owner_package_segments, &definition.name);
    let construct = mappings.definition_construct_for(&definition.keyword);
    let plan = collect_generic_definition_plan(
        mappings.lowering_rule_for_construct(&construct),
        definition,
        &qualified_name,
        &construct,
        mappings,
    )?;
    let mut members = plan.members;
    annotate_connection_definition_members(&construct, &mut members, mappings);

    Ok(CollectedDefinition {
        construct,
        qualified_name,
        declared_name: plan.declared_name,
        is_abstract: plan.is_abstract,
        specializes: plan.specializes,
        members,
        docs: plan.docs,
        span: definition.span.clone(),
    })
}

struct GenericDefinitionCollectPlan {
    declared_name: String,
    is_abstract: bool,
    specializes: Vec<QualifiedName>,
    members: Vec<CollectedUsage>,
    docs: Vec<String>,
}

fn collect_generic_definition_plan(
    rule: Option<&LoweringRule>,
    definition: &GenericDefinitionDecl,
    qualified_name: &str,
    construct: &str,
    mappings: &MappingBundle,
) -> Result<GenericDefinitionCollectPlan, Diagnostic> {
    let mut plan = GenericDefinitionCollectPlan {
        declared_name: definition.name.clone(),
        is_abstract: definition
            .modifiers
            .iter()
            .any(|modifier| modifier == "abstract"),
        specializes: definition_specializations_with_default(
            construct,
            &definition.specializes,
            mappings,
        ),
        members: collect_usage_members(&definition.members, qualified_name, construct, mappings)?,
        docs: definition.docs.clone(),
    };

    let Some(rule) = rule else {
        return Ok(plan);
    };
    require_collect_expression(rule, "name", "$ast.name")?;
    for (field, expression) in &rule.collect.fields {
        match (field.as_str(), expression.as_str()) {
            ("is_abstract", "$ast.modifiers contains abstract") => {
                plan.is_abstract = definition
                    .modifiers
                    .iter()
                    .any(|modifier| modifier == "abstract");
            }
            ("specializes", "$ast.specializes or semantic_default") => {
                plan.specializes = definition_specializations_with_default(
                    construct,
                    &definition.specializes,
                    mappings,
                );
            }
            ("specializes", "$ast.specializes") => {
                plan.specializes = definition.specializes.clone();
            }
            ("members", "$ast.members[usage]") | ("end_members", "$ast.members[modifier=end]") => {
                plan.members = collect_usage_members(
                    &definition.members,
                    qualified_name,
                    construct,
                    mappings,
                )?;
            }
            ("docs", "$ast.docs") => {
                plan.docs = definition.docs.clone();
            }
            _ => return Err(unsupported_collect_expression(rule, field, expression)),
        }
    }
    Ok(plan)
}

fn definition_specializations_with_default(
    construct: &str,
    explicit: &[QualifiedName],
    mappings: &MappingBundle,
) -> Vec<QualifiedName> {
    if !explicit.is_empty() {
        return explicit.to_vec();
    }

    let zero_span = SourceSpan {
        start_line: 0,
        start_col: 0,
        end_line: 0,
        end_col: 0,
    };
    let mut specializations = Vec::new();
    for semantic_specialization in mappings.semantic_specializations_for_definition(construct) {
        specializations.push(QualifiedName {
            segments: semantic_specialization
                .split("::")
                .map(str::to_string)
                .collect(),
            span: zero_span.clone(),
        });
    }
    specializations
}

fn collect_generic_usage(
    usage: &GenericUsageDecl,
    owner_qualified_name: &str,
    owner_construct: &str,
    mappings: &MappingBundle,
) -> Result<CollectedUsage, Diagnostic> {
    let construct = mappings.usage_construct_for(&usage.keyword);
    let qualified_name = usage_qualified_name(owner_qualified_name, &usage.name);
    let plan = collect_generic_usage_plan(
        mappings.lowering_rule_for_construct(&construct),
        usage,
        &qualified_name,
        &construct,
        mappings,
    )?;
    Ok(CollectedUsage {
        construct,
        owner_construct: owner_construct.to_string(),
        owner_qualified_name: owner_qualified_name.to_string(),
        qualified_name,
        declared_name: plan.declared_name,
        is_implicit_name: usage.is_implicit_name,
        ty: plan.ty,
        additional_types: usage.additional_types.clone(),
        reference_target: plan.reference_target,
        allocation_source: plan.allocation_source,
        allocation_target: plan.allocation_target,
        metadata_properties: usage.metadata_properties.clone(),
        multiplicity: plan.multiplicity,
        expression: plan.expression,
        specializes: plan.specializes,
        subsets: plan.subsets,
        redefines: plan.redefines,
        members: plan.members,
        modifiers: plan.modifiers,
        docs: plan.docs,
        span: usage.span.clone(),
    })
}

struct GenericUsageCollectPlan {
    declared_name: String,
    ty: Option<QualifiedName>,
    reference_target: Option<QualifiedName>,
    allocation_source: Option<QualifiedName>,
    allocation_target: Option<QualifiedName>,
    multiplicity: Option<MultiplicityRange>,
    expression: Option<Expr>,
    specializes: Vec<QualifiedName>,
    subsets: Vec<QualifiedName>,
    redefines: Vec<QualifiedName>,
    members: Vec<CollectedUsage>,
    modifiers: Vec<String>,
    docs: Vec<String>,
}

fn generic_usage_plan_from_ast(
    usage: &GenericUsageDecl,
    qualified_name: &str,
    construct: &str,
    mappings: &MappingBundle,
) -> Result<GenericUsageCollectPlan, Diagnostic> {
    Ok(GenericUsageCollectPlan {
        declared_name: usage.name.clone(),
        ty: usage.ty.clone(),
        reference_target: usage.reference_target.clone(),
        allocation_source: usage.allocation_source.clone(),
        allocation_target: usage.allocation_target.clone(),
        multiplicity: usage.multiplicity.clone(),
        expression: usage.expression.clone(),
        specializes: usage.specializes.clone(),
        subsets: usage.subsets.clone(),
        redefines: usage.redefines.clone(),
        members: collect_usage_members(&usage.body_members, qualified_name, construct, mappings)?,
        modifiers: usage.modifiers.clone(),
        docs: usage.docs.clone(),
    })
}

fn collect_generic_usage_plan(
    rule: Option<&LoweringRule>,
    usage: &GenericUsageDecl,
    qualified_name: &str,
    construct: &str,
    mappings: &MappingBundle,
) -> Result<GenericUsageCollectPlan, Diagnostic> {
    let mut plan = generic_usage_plan_from_ast(usage, qualified_name, construct, mappings)?;
    let Some(rule) = rule else {
        return Ok(plan);
    };
    require_collect_expression(rule, "name", "$ast.name")?;
    for (field, expression) in &rule.collect.fields {
        match (field.as_str(), expression.as_str()) {
            ("allocation_source", "$ast.allocation_source") => {
                plan.allocation_source = usage.allocation_source.clone();
            }
            ("allocation_target", "$ast.allocation_target") => {
                plan.allocation_target = usage.allocation_target.clone();
            }
            ("annotation_target", "$ast.reference_target")
            | ("reference_target", "$ast.reference_target") => {
                plan.reference_target = usage.reference_target.clone();
            }
            ("body", "$ast.expression") => {
                plan.expression = usage.expression.clone();
            }
            ("docs", "$ast.docs") => {
                plan.docs = usage.docs.clone();
            }
            ("members", "$ast.body_members[usage]") => {
                plan.members = collect_usage_members(
                    &usage.body_members,
                    qualified_name,
                    construct,
                    mappings,
                )?;
            }
            ("modifiers", "$ast.modifiers + end") => {
                plan.modifiers = usage.modifiers.clone();
                if !plan.modifiers.iter().any(|modifier| modifier == "end") {
                    plan.modifiers.push("end".to_string());
                }
            }
            ("multiplicity", "$ast.multiplicity") => {
                plan.multiplicity = usage.multiplicity.clone();
            }
            ("reference_target", "$ast.reference_target or $ast.name") => {
                plan.reference_target = usage.reference_target.clone().or_else(|| {
                    (!usage.name.is_empty()).then(|| QualifiedName {
                        segments: vec![usage.name.clone()],
                        span: usage.span.clone(),
                    })
                });
            }
            ("redefines", "$ast.redefines") => {
                plan.redefines = usage.redefines.clone();
            }
            ("specializes", "$ast.specializes or semantic_default")
            | ("specializes", "$ast.specializes") => {
                plan.specializes = usage.specializes.clone();
            }
            ("subsets", "$ast.subsets") => {
                plan.subsets = usage.subsets.clone();
            }
            ("type", "$ast.ty") => {
                plan.ty = usage.ty.clone();
            }
            _ => return Err(unsupported_collect_expression(rule, field, expression)),
        }
    }
    Ok(plan)
}

fn collect_usage_members(
    declarations: &[Declaration],
    qualified_name: &str,
    construct: &str,
    mappings: &MappingBundle,
) -> Result<Vec<CollectedUsage>, Diagnostic> {
    let mut usages = Vec::new();
    for member in declarations {
        if let Some(usage) = member.as_usage_like() {
            usages.push(collect_generic_usage(
                &usage,
                qualified_name,
                construct,
                mappings,
            )?);
        }
    }
    Ok(usages)
}

fn require_collect_expression(
    rule: &LoweringRule,
    slot: &str,
    expected: &str,
) -> Result<(), Diagnostic> {
    let actual = match slot {
        "name" => rule.collect.name.as_str(),
        "owner" => rule.collect.owner.as_str(),
        "element" => rule.collect.element.as_str(),
        _ => "",
    };
    if actual == expected {
        Ok(())
    } else {
        Err(unsupported_collect_expression(rule, slot, actual))
    }
}

fn unsupported_collect_expression(rule: &LoweringRule, slot: &str, expression: &str) -> Diagnostic {
    Diagnostic::new(
        format!(
            "lowering rule `{}` collect expression `{}` in `{}` is not executable here",
            rule.construct, expression, slot
        ),
        None,
    )
}

fn annotate_connection_definition_members(
    definition_construct: &str,
    members: &mut [CollectedUsage],
    mappings: &MappingBundle,
) {
    if !should_annotate_connection_end_direction(mappings, definition_construct) {
        return;
    }

    let mut end_index = 0usize;
    for member in members {
        if member.construct == "PartUsage"
            && member.modifiers.iter().any(|modifier| modifier == "end")
        {
            let directional_modifier = if end_index == 0 {
                "end-source"
            } else {
                "end-target"
            };
            member.modifiers.push(directional_modifier.to_string());
            end_index += 1;
        }
    }
}

fn collect_alias(alias: &AliasDecl, owner_package_segments: &[String]) -> CollectedAlias {
    let target = if alias.target.segments.len() == 1 && !owner_package_segments.is_empty() {
        QualifiedName {
            segments: qualify_segments(owner_package_segments, &alias.target.segments),
            span: alias.target.span.clone(),
        }
    } else {
        alias.target.clone()
    };
    CollectedAlias {
        qualified_name: qualify_name(owner_package_segments, &alias.name),
        declared_name: alias.name.clone(),
        target,
    }
}

fn collect_alias_in_owner(alias: &AliasDecl, owner_qualified_name: &str) -> CollectedAlias {
    let target = if alias.target.segments.len() == 1 && owner_qualified_name != "root" {
        let mut segments = owner_qualified_name
            .split('.')
            .map(str::to_string)
            .collect::<Vec<_>>();
        segments.extend(alias.target.segments.clone());
        QualifiedName {
            segments,
            span: alias.target.span.clone(),
        }
    } else {
        alias.target.clone()
    };
    CollectedAlias {
        qualified_name: usage_qualified_name(owner_qualified_name, &alias.name),
        declared_name: alias.name.clone(),
        target,
    }
}

fn collect_nested_aliases(
    declarations: &[Declaration],
    owner_package_segments: &[String],
    owner_qualified_name: Option<&str>,
    aliases: &mut Vec<CollectedAlias>,
) {
    for declaration in declarations {
        if let Some(definition) = declaration.as_definition_like() {
            let qualified_name = qualify_name(owner_package_segments, &definition.name);
            collect_nested_member_aliases(&definition.members, &qualified_name, aliases);
            continue;
        }
        if let Some(usage) = declaration.as_usage_like() {
            let qualified_name =
                usage_qualified_name(owner_qualified_name.unwrap_or("root"), &usage.name);
            collect_nested_member_aliases(&usage.body_members, &qualified_name, aliases);
            continue;
        }

        match declaration {
            Declaration::Package(package) => {
                let package_segments =
                    qualify_segments(owner_package_segments, &package.name.segments);
                let package_qualified_name = package_segments.join(".");
                collect_nested_aliases(
                    &package.members,
                    &package_segments,
                    Some(&package_qualified_name),
                    aliases,
                );
            }
            Declaration::Import(_) | Declaration::Alias(_) => {}
            _ => unreachable!("definition-like and usage-like declarations are handled above"),
        }
    }
}

fn collect_nested_member_aliases(
    declarations: &[Declaration],
    owner_qualified_name: &str,
    aliases: &mut Vec<CollectedAlias>,
) {
    for declaration in declarations {
        if let Some(usage) = declaration.as_usage_like() {
            let qualified_name = usage_qualified_name(owner_qualified_name, &usage.name);
            collect_nested_member_aliases(&usage.body_members, &qualified_name, aliases);
            continue;
        }
        if let Some(definition) = declaration.as_definition_like() {
            let qualified_name = usage_qualified_name(owner_qualified_name, &definition.name);
            collect_nested_member_aliases(&definition.members, &qualified_name, aliases);
            continue;
        }

        match declaration {
            Declaration::Alias(alias) => {
                aliases.push(collect_alias_in_owner(alias, owner_qualified_name))
            }
            Declaration::Package(package) => {
                let qualified_name =
                    usage_qualified_name(owner_qualified_name, &package.name.as_dot_string());
                collect_nested_member_aliases(&package.members, &qualified_name, aliases);
            }
            Declaration::Import(_) => {}
            _ => unreachable!("definition-like and usage-like declarations are handled above"),
        }
    }
}

fn qualify_name(owner_package_segments: &[String], name: &str) -> String {
    let mut segments = owner_package_segments.to_vec();
    segments.push(name.to_string());
    segments.join(".")
}

fn usage_qualified_name(owner_qualified_name: &str, declared_name: &str) -> String {
    if owner_qualified_name == "root" {
        declared_name.to_string()
    } else {
        format!("{owner_qualified_name}.{declared_name}")
    }
}

fn qualify_segments(
    owner_package_segments: &[String],
    declared_segments: &[String],
) -> Vec<String> {
    let mut segments = owner_package_segments.to_vec();
    segments.extend(declared_segments.iter().cloned());
    segments
}
