//! Two extracted branches of KerMLValidator.checkFeature, not the whole check.
use super::*;

#[derive(Deserialize)]
struct FeatureChecks {
    constant_implies_variable: bool,
    checks: Vec<FeatureCheck>,
}
#[derive(Deserialize)]
struct FeatureCheck {
    operation: String,
    issue: String,
    message: String,
    owner_type: Option<String>,
}

pub(super) fn apply(
    module: &mut ResolvedModule,
    context: &ResolverContext,
    mappings: &MappingBundle,
    lookup: &FeatureLookup<'_>,
) -> Result<(), Diagnostic> {
    static EXTRACTED: OnceLock<Result<FeatureChecks, String>> = OnceLock::new();
    let extracted = EXTRACTED.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../../resources/metamodels/sysml-2.0-pilot-2026-08/feature-checks.extract.json"
        )).map_err(|error| error.to_string())
    }).as_ref().map_err(|error| Diagnostic::new(format!("invalid feature check extraction: {error}"), None))?;
    fn derive(
        usage: &mut ResolvedUsage,
        mappings: &MappingBundle,
        constant_implies_variable: bool,
    ) -> Result<(), Diagnostic> {
        let kind = mappings.metaclass_for(&usage.construct)?;
        // SysML Usage flags have their own derived semantics. KerML Feature
        // flags are declared, with the extracted constant postprocessor.
        if kind_conforms_to(kind, "Feature") && !kind_conforms_to(kind, "Usage") {
            let constant = usage.modifiers.iter().any(|m| m == "const");
            let variable = usage.modifiers.iter().any(|m| m == "var")
                || (constant && constant_implies_variable);
            usage
                .derived_properties
                .insert("is_constant".into(), constant);
            usage
                .derived_properties
                .insert("is_variable".into(), variable);
            usage.derived_properties.insert(
                "is_portion".into(),
                usage.modifiers.iter().any(|m| m == "portion"),
            );
        }
        for member in &mut usage.members {
            derive(member, mappings, constant_implies_variable)?;
        }
        Ok(())
    }
    for usage in module
        .usages
        .iter_mut()
        .chain(module.definitions.iter_mut().flat_map(|d| &mut d.members))
    {
        derive(
            usage,
            mappings,
            extracted.constant_implies_variable,
        )?;
    }
    let mut pending = module
        .usages
        .iter()
        .chain(module.definitions.iter().flat_map(|d| &d.members))
        .collect::<Vec<_>>();
    let mut usages = BTreeMap::new();
    while let Some(usage) = pending.pop() {
        pending.extend(&usage.members);
        usages.insert(feature_id_from_qualified_name(&usage.qualified_name), usage);
    }
    for usage in usages.values() {
        let kind = mappings.metaclass_for(&usage.construct)?;
        if !kind_conforms_to(kind, "Feature")
            || !usage
                .derived_properties
                .get("is_variable")
                .copied()
                .unwrap_or(false)
        {
            continue;
        }
        for check in &extracted.checks {
            let invalid = match check.operation.as_str() {
                "variable_occurrence_owner" => {
                    let Some(anchor) = &check.owner_type else {
                        return Err(Diagnostic::new("missing extracted occurrence anchor", None));
                    };
                    // Partial library fixtures cannot establish owner ancestry.
                    if !context.library_indexes.kinds.contains_key(anchor) {
                        continue;
                    }
                    let specializes = |specific: &str| {
                        lookup.type_specializes(
                            specific,
                            anchor,
                            Some(&context.library_indexes.specializations),
                            &mut BTreeSet::new(),
                        )
                    };
                    // OwningMembership/VariantMembership do not establish an
                    // owning type, even when the namespace is itself a type.
                    let has_feature_membership = !usage
                        .modifiers
                        .iter()
                        .any(|m| matches!(m.as_str(), "owned_crossing_feature" | "variant"));
                    let owner_is_occurrence = if !has_feature_membership {
                        false
                    } else if let Some(owner) =
                        usages.get(&feature_id_from_qualified_name(&usage.owner_qualified_name))
                    {
                        all_types(owner, &usages, context, mappings, lookup)
                            .iter()
                            .any(|ty| specializes(ty))
                    } else if let Some(owner) = module
                        .definitions
                        .iter()
                        .find(|d| d.qualified_name == usage.owner_qualified_name)
                    {
                        owner.specializes.iter().any(|ty| specializes(ty))
                    } else {
                        false
                    };
                    !owner_is_occurrence
                }
                "portion_not_variable" => usage
                    .derived_properties
                    .get("is_portion")
                    .copied()
                    .unwrap_or(false),
                other => {
                    return Err(Diagnostic::new(
                        format!("unsupported extracted feature check: {other}"),
                        None,
                    ));
                }
            };
            if invalid {
                return Err(Diagnostic::new(
                    format!("{}: {}", check.issue, check.message),
                    Some(usage.span.clone()),
                ));
            }
        }
    }
    Ok(())
}
