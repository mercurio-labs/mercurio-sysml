//! AST collection phase.

use std::collections::{BTreeMap, BTreeSet};

use mercurio_foundation::language_contracts::ast::{
    AliasDecl, Declaration, Expr, GenericDefinitionDecl, GenericUsageDecl, ImportDecl,
    MultiplicityRange, PackageDecl, ParsedModule as SysmlModule, QualifiedName, SourceSpan,
};
use mercurio_foundation::language_contracts::diagnostics::Diagnostic;

use crate::language_frontend::lowering::elaborate::should_annotate_connection_end_direction;
use crate::language_frontend::lowering::emit::MappingBundle;
use crate::language_frontend::lowering::ir::ResolvedPackage;
use crate::language_frontend::lowering::rules::LoweringRule;

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
    /// Qualified name of the owning namespace. **Not always a package**:
    /// `collect_nested_member_imports` also walks usage and definition bodies,
    /// so a `view v { expose x::**; }` records the *view usage* here. Emission
    /// must therefore resolve this against usage and definition ids too, not
    /// just package ids — assuming a package silently reparented every query
    /// declared inside a view onto the document root (save-as-view SV-2).
    pub(crate) owner_qualified_name: Option<String>,
    pub(crate) decl: ImportDecl,
}

#[derive(Debug, Clone)]
pub(crate) struct CollectedDefinition {
    pub(crate) modifiers: Vec<String>,
    pub(crate) construct: String,
    pub(crate) qualified_name: String,
    pub(crate) declared_name: String,
    pub(crate) is_abstract: bool,
    pub(crate) is_variation: bool,
    pub(crate) is_public: bool,
    pub(crate) implicit_specializations: Vec<String>,
    pub(crate) specializes: Vec<QualifiedName>,
    pub(crate) members: Vec<CollectedUsage>,
    pub(crate) docs: Vec<String>,
    pub(crate) span: SourceSpan,
}

#[derive(Debug, Clone)]
pub(crate) struct CollectedUsage {
    pub(crate) annotation_targets: Vec<QualifiedName>,
    pub(crate) construct: String,
    pub(crate) owner_construct: String,
    pub(crate) owner_qualified_name: String,
    pub(crate) qualified_name: String,
    pub(crate) declared_name: String,
    pub(crate) is_implicit_name: bool,
    pub(crate) has_explicit_specialization: bool,
    pub(crate) ty: Option<QualifiedName>,
    pub(crate) implicit_type: Option<String>,
    pub(crate) implicit_subsets: Vec<String>,
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
    pub(crate) declaration: Option<AliasDecl>,
    pub(crate) owner_qualified_name: String,
    pub(crate) qualified_name: String,
    pub(crate) declared_name: String,
    pub(crate) target: QualifiedName,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ImportAliases {
    /// Membership access metadata shared by scope consumers alongside import bindings.
    /// Indexed once per resolver context; cloning bindings does not clone this table.
    pub(crate) membership_visibility: std::sync::Arc<BTreeMap<String, String>>,
    /// Source-derived visibility of directly owned library members.
    pub(crate) library_membership_visibility: std::sync::Arc<BTreeMap<String, String>>,
    pub(crate) library_namespace_scope: std::sync::Arc<super::indexes::LibraryNamespaceScope>,
    pub(crate) value_aliases: BTreeMap<String, String>,
    /// Declaring scopes for compatibility short bindings; lookup must still be lexical.
    pub(crate) value_alias_owners: BTreeMap<String, BTreeSet<String>>,
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

    // Relationship bodies are containment scopes, not namespaces. Their
    // definitions still need the ordinary definition indexes and semantic passes.
    // Keep usage roots in the relationship body; collect nested definitions here.
    let mut visited_bodies = BTreeSet::new();
    loop {
        let bodies = aliases.iter().filter_map(|a| a.declaration.as_ref().map(|d|
            (a.qualified_name.clone(), d.body_members.clone())))
            .chain(imports.iter().map(|i| (import_body_scope(i), i.decl.body_members.clone())))
            .filter(|(scope, _)| visited_bodies.insert(scope.clone()))
            .collect::<Vec<_>>();
        if bodies.is_empty() { break; }
        for (scope, body) in bodies {
            let segments = scope.split('.').map(str::to_string).collect::<Vec<_>>();
            collect_nested_owned_definitions(&body, &segments, &mut definitions, mappings)?;
            collect_nested_owned_packages(&body, &segments, &mut packages, &mut imports,
                &mut definitions, &mut usages, &mut aliases, mappings)?;
            collect_nested_member_imports(&body, &scope, &mut imports);
            collect_nested_aliases(&body, &segments, Some(&scope), &mut aliases);
        }
    }


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
            let qualified_name = anonymous_usage_qualified_name(owner, &usage);
            collect_nested_member_imports(&usage.body_members, &qualified_name, imports);
            let segments = qualified_name
                .split('.')
                .map(str::to_string)
                .collect::<Vec<_>>();
            collect_nested_owned_definitions(
                &usage.body_members,
                &segments,
                definitions,
                mappings,
            )?;
            collect_nested_owned_packages(
                &usage.body_members,
                &segments,
                packages,
                imports,
                definitions,
                usages,
                aliases,
                mappings,
            )?;
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
                owner_qualified_name: owner_package_qualified_name.map(str::to_string),
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
        } else if let Some(definition) = declaration.as_definition_like() {
            let segments = qualify_segments(owner_package_segments, &[definition.name.clone()]);
            collect_nested_owned_packages(
                &definition.members,
                &segments,
                packages,
                imports,
                definitions,
                usages,
                aliases,
                mappings,
            )?;
        } else if let Some(usage) = declaration.as_usage_like() {
            let name = anonymous_usage_qualified_name(&owner_package_segments.join("."), &usage);
            let segments = name.split('.').map(str::to_string).collect::<Vec<_>>();
            collect_nested_owned_packages(
                &usage.body_members,
                &segments,
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
        } else if let Some(usage) = declaration.as_usage_like() {
            let name = anonymous_usage_qualified_name(&owner_package_segments.join("."), &usage);
            let segments = name.split('.').map(str::to_string).collect::<Vec<_>>();
            collect_nested_owned_definitions(
                &usage.body_members,
                &segments,
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
            let qualified_name = anonymous_usage_qualified_name(owner_qualified_name, &usage);
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
                owner_qualified_name: Some(owner_qualified_name.to_string()),
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
        modifiers: package.modifiers.clone(),
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
    let end_count = members.iter().filter(|member| member.modifiers.iter().any(|m| m == "end" || m.starts_with("end-"))).count();
    let connection_defaults = mappings.connection_definition_defaults(&construct, end_count)?;
    // Derived library parents are not explicit source references. Resolve adds
    // them after checking declared parents; partial-library services can still
    // diagnose and recover individual source members without those libraries.
    let specializes = if connection_defaults.is_some() && definition.specializes.is_empty() {
        Vec::new()
    } else { plan.specializes };
    let implicit_specializations = connection_defaults
        .unwrap_or_else(|| mappings.semantic_specializations_for_definition(&construct));

    Ok(CollectedDefinition {
        modifiers: definition.modifiers.clone(),
        implicit_specializations,
        construct: construct.clone(),
        qualified_name,
        declared_name: plan.declared_name,
        is_abstract: plan.is_abstract,
        // Pilot EnumerationDefinitionImpl initializes isVariation to true.
        // This model default is handwritten; the generated validator only reads it.
        is_variation: construct == "EnumerationDefinition"
            || definition.modifiers.iter().any(|m| m == "variation"),
        is_public: !definition
            .modifiers
            .iter()
            .any(|m| matches!(m.as_str(), "private" | "protected")),
        specializes,
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
            .any(|modifier| matches!(modifier.as_str(), "abstract" | "variation")),
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
                    .any(|modifier| matches!(modifier.as_str(), "abstract" | "variation"));
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
    let mut construct = mappings.usage_construct_in_context(&usage.keyword, owner_construct);
    // A state entry/do/exit action (`do action X`) performs the named action;
    // the pilot models it as PerformActionUsage, not a plain ActionUsage.
    if matches!(construct.as_str(), "ActionUsage" | "ReferenceUsage")
        && usage
            .modifiers
            .iter()
            .any(|modifier| matches!(modifier.as_str(), "entry" | "do" | "exit"))
    {
        construct = "PerformActionUsage".to_string();
    }
    let qualified_name = anonymous_usage_qualified_name(owner_qualified_name, &usage);
    let plan = collect_generic_usage_plan(
        mappings.lowering_rule_for_construct(&construct),
        usage,
        &qualified_name,
        &construct,
        mappings,
    )?;
    let end_count = usage.body_members.iter().filter(|member| matches!(member,
        Declaration::GenericUsage(end) if end.modifiers.iter().any(|m| m == "end" || m.starts_with("end-")))).count();
    let implicit_type = mappings.usage_type_for_end_count(&construct, end_count)
        .or_else(|| mappings.usage_family_default(&construct, owner_construct).map(|default| default.type_ref))
        .or_else(|| mappings.constant_usage_type_default(&construct));
    Ok(CollectedUsage {
        annotation_targets: plan.annotation_targets,
        implicit_subsets: mappings.constant_usage_subset_defaults(&construct),
        implicit_type,
        construct,
        owner_construct: owner_construct.to_string(),
        owner_qualified_name: owner_qualified_name.to_string(),
        qualified_name,
        declared_name: plan.declared_name,
        is_implicit_name: usage.is_implicit_name,
        has_explicit_specialization: usage.ty.is_some()
            || !usage.additional_types.is_empty()
            || !usage.specializes.is_empty()
            || !usage.subsets.is_empty()
            || !usage.redefines.is_empty()
            || usage.reference_target.is_some(),
        ty: plan.ty.or_else(|| {
            // Pilot derives a variant's type from its owning variation definition.
            (usage.reference_target.is_none()
                && usage.modifiers.iter().any(|m| m == "variant")
                && owner_construct.ends_with("Definition"))
            .then(|| QualifiedName {
                segments: owner_qualified_name
                    .split('.')
                    .map(str::to_string)
                    .collect(),
                span: usage.span.clone(),
            })
        }),
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
    annotation_targets: Vec<QualifiedName>,
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
        annotation_targets: usage.annotation_targets.clone(),
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
            ("expression", "$ast.expression") => {
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
                    (!usage.name.is_empty() && usage.ty.is_none()).then(|| QualifiedName {
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

fn alias_scope_name(alias: &AliasDecl) -> String {
    if !alias.name.is_empty() { alias.name.clone() }
    else { format!("__alias_{}_{}", alias.span.start_line, alias.span.start_col) }
}

pub(crate) fn import_body_scope(import: &CollectedImport) -> String {
    format!("{}.__import_{}_{}", import.owner_qualified_name.as_deref().unwrap_or("root"), import.decl.span.start_line, import.decl.span.start_col)
}

pub(crate) fn collect_relationship_body(
    members: &[Declaration], scope: &str, construct: &str, mappings: &MappingBundle,
) -> Result<Vec<CollectedUsage>, Diagnostic> {
    members.iter().filter_map(|declaration| {
        // Definitions and packages are collected into their ordinary module
        // indexes above. Do not reinterpret them as usage-like annotations.
        if declaration.as_definition_like().is_some() || matches!(declaration, Declaration::Package(_)) {
            return None;
        }
        Some(declaration.as_usage_like().ok_or_else(|| Diagnostic::new("unsupported relationship body declaration", None))
            .and_then(|usage| collect_generic_usage(&usage, scope, construct, mappings)))
    }).collect()
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
        declaration: Some(alias.clone()),
        owner_qualified_name: if owner_package_segments.is_empty() { "root".into() } else { owner_package_segments.join(".") },
        qualified_name: qualify_name(owner_package_segments, &alias_scope_name(alias)),
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
        declaration: Some(alias.clone()),
        owner_qualified_name: owner_qualified_name.into(),
        qualified_name: usage_qualified_name(owner_qualified_name, &alias_scope_name(alias)),
        declared_name: alias.name.clone(),
        target,
    }
}

fn collect_definition_short_names(
    definition: &GenericDefinitionDecl,
    qualified_name: &str,
    aliases: &mut Vec<CollectedAlias>,
) {
    for short_name in definition
        .modifiers
        .iter()
        .filter_map(|m| m.strip_prefix("short_name="))
    {
        let owner = qualified_name
            .rsplit_once('.')
            .map(|(owner, _)| owner)
            .unwrap_or("root");
        aliases.push(CollectedAlias {
            declaration: None,
            owner_qualified_name: owner.into(),
            qualified_name: usage_qualified_name(owner, short_name),
            declared_name: short_name.to_string(),
            target: QualifiedName {
                segments: qualified_name.split('.').map(str::to_string).collect(),
                span: definition.span.clone(),
            },
        });
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
            collect_definition_short_names(&definition, &qualified_name, aliases);
            collect_nested_member_aliases(&definition.members, &qualified_name, aliases);
            continue;
        }
        if let Some(usage) = declaration.as_usage_like() {
            let qualified_name =
                anonymous_usage_qualified_name(owner_qualified_name.unwrap_or("root"), &usage);
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
            let qualified_name = anonymous_usage_qualified_name(owner_qualified_name, &usage);
            collect_nested_member_aliases(&usage.body_members, &qualified_name, aliases);
            continue;
        }
        if let Some(definition) = declaration.as_definition_like() {
            let qualified_name = usage_qualified_name(owner_qualified_name, &definition.name);
            collect_definition_short_names(&definition, &qualified_name, aliases);
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

// Anonymous declarations may share a derived display name. Their namespace
// identities must remain distinct before indexes and nested ownership are built.
fn anonymous_usage_qualified_name(owner: &str, usage: &GenericUsageDecl) -> String {
    let name = usage_qualified_name(owner, &usage.name);
    if usage.is_implicit_name && usage.redefines.is_empty() {
        format!("{name}.@{}_{}", usage.span.start_line, usage.span.start_col)
    } else {
        name
    }
}
