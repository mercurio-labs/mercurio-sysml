use std::collections::BTreeSet;

use mercurio_language_contracts::diagnostics::Diagnostic;

use crate::SourceLanguage;
pub use crate::lowering::emit::{
    DefaultSpecializationAnchorsSeed, EmissionRule, EmissionSpec, KirEmissionSeed, MappingBundle,
    MetamodelConstructEntry, MetamodelConstructSeed, PilotConstructEntry, PilotConstructSeed,
    SemanticSpecializationDefaultsSeed, StdlibAliasSeed, UsageSemanticSpecializationOverrideSeed,
};
pub use crate::lowering::rules::{
    LoweringAstPattern, LoweringCollectRule, LoweringElaborationRule, LoweringEmitRule,
    LoweringPilotSources, LoweringRule, LoweringRuleSeed, has_runtime_collect_expression,
    has_runtime_elaboration_hook,
};

#[derive(Clone)]
pub struct LanguageProfile {
    pub id: String,
    pub language: SourceLanguage,
    pub mappings: &'static MappingBundle,
    pub lowering_rules: Option<&'static LoweringRuleSeed>,
}

impl LanguageProfile {
    pub fn load(language: SourceLanguage) -> Result<Self, Diagnostic> {
        let id = match language {
            SourceLanguage::Kerml => "kerml-bootstrap".to_string(),
            SourceLanguage::Sysml => "sysml-2.0-metamodel-0.57.0".to_string(),
        };
        let mappings = MappingBundle::load_for_language(language)?;
        let lowering_rules = LoweringRuleSeed::load_for_language(language)?;
        validate_lowering_rules_against_mappings(lowering_rules, mappings)?;
        Ok(Self {
            id,
            language,
            mappings,
            lowering_rules,
        })
    }

    pub fn load_for_profile(id: impl Into<String>) -> Result<Self, Diagnostic> {
        let id = canonical_profile_id(&id.into()).to_string();
        let mappings = MappingBundle::load_for_profile(&id)?;
        let lowering_rules = LoweringRuleSeed::load_for_profile(&id)?;
        validate_lowering_rules_against_mappings(lowering_rules, mappings)?;
        Ok(Self {
            id,
            language: SourceLanguage::Sysml,
            mappings,
            lowering_rules,
        })
    }
}

fn canonical_profile_id(id: &str) -> &str {
    match id {
        "sysml-2.0-pilot-0.57.0" => "sysml-2.0-metamodel-0.57.0",
        other => other,
    }
}

fn validate_lowering_rules_against_mappings(
    lowering_rules: Option<&LoweringRuleSeed>,
    mappings: &MappingBundle,
) -> Result<(), Diagnostic> {
    let Some(lowering_rules) = lowering_rules else {
        return Ok(());
    };

    for rule in &lowering_rules.rules {
        for (slot, expression) in collect_rule_expressions(rule) {
            if !has_runtime_collect_expression(&expression) {
                return Err(Diagnostic::new(
                    format!(
                        "lowering rule `{}` collect expression `{}` in `{}` has no runtime support",
                        rule.construct, expression, slot
                    ),
                    None,
                ));
            }
        }
        match rule.collect.element.as_str() {
            "definition" => {
                if let Some(keyword) = rule.ast.keyword.as_deref()
                    && rule.ast.node == "GenericDefinitionDecl"
                    && mappings.definition_construct_for(keyword) != rule.construct
                {
                    return Err(Diagnostic::new(
                        format!(
                            "lowering rule `{}` definition keyword `{}` does not resolve to its construct",
                            rule.construct, keyword
                        ),
                        None,
                    ));
                }
            }
            "usage" => {
                if let Some(keyword) = rule.ast.keyword.as_deref()
                    && rule.ast.node == "GenericUsageDecl"
                    && mappings.usage_construct_for(keyword) != rule.construct
                {
                    return Err(Diagnostic::new(
                        format!(
                            "lowering rule `{}` usage keyword `{}` does not resolve to its construct",
                            rule.construct, keyword
                        ),
                        None,
                    ));
                }
            }
            _ => {}
        }
        let emission = mappings.emission_for(&rule.metaclass)?;
        if rule.emit.id_template != emission.id_template {
            return Err(Diagnostic::new(
                format!(
                    "lowering rule `{}` id template `{}` does not match emission mapping `{}` template `{}`",
                    rule.construct, rule.emit.id_template, rule.metaclass, emission.id_template
                ),
                None,
            ));
        }
        let emission_properties = emission
            .emit
            .properties
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for property in rule.emit.properties.keys() {
            if !emission_properties.contains(property.as_str()) {
                return Err(Diagnostic::new(
                    format!(
                        "lowering rule `{}` property `{}` is missing from emission mapping `{}`",
                        rule.construct, property, rule.metaclass
                    ),
                    None,
                ));
            }
        }
        for step in &rule.elaborate {
            if !has_runtime_elaboration_hook(&step.id) {
                return Err(Diagnostic::new(
                    format!(
                        "lowering rule `{}` elaboration `{}` has no runtime hook",
                        rule.construct, step.id
                    ),
                    None,
                ));
            }
        }
    }

    Ok(())
}

fn collect_rule_expressions(rule: &LoweringRule) -> Vec<(&str, &str)> {
    let mut expressions = vec![
        ("element", rule.collect.element.as_str()),
        ("name", rule.collect.name.as_str()),
        ("owner", rule.collect.owner.as_str()),
    ];
    expressions.extend(
        rule.collect
            .fields
            .iter()
            .map(|(field, expression)| (field.as_str(), expression.as_str())),
    );
    expressions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::ir::{ResolvedDefinition, ResolvedUsage};

    #[test]
    fn sysml_profile_loads_declarative_lowering_rules() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();
        let rules = profile.lowering_rules.expect("sysml lowering rules");

        assert_eq!(rules.schema_version, 1);
        assert!(rules.rules.iter().any(|rule| rule.construct == "PartUsage"));
    }

    #[test]
    fn sysml_mappings_expose_reviewed_package_lowering_rule() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();
        let rule = profile
            .mappings
            .lowering_rule_for_construct("Package")
            .expect("package lowering rule");

        assert_eq!(rule.status.as_deref(), Some("reviewed"));
        assert_eq!(rule.metaclass, "SysML::Package");
        assert_eq!(rule.emit.id_template, "pkg.{qualified_name}");
    }

    #[test]
    fn sysml_mappings_expose_reviewed_import_lowering_rule() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();
        let rule = profile
            .mappings
            .lowering_rule_for_construct("Import")
            .expect("import lowering rule");

        assert_eq!(rule.status.as_deref(), Some("reviewed"));
        assert_eq!(rule.metaclass, "SysML::Import");
        assert_eq!(rule.emit.id_template, "import.{owner_id}.{ordinal}");
    }

    #[test]
    fn sysml_mappings_expose_reviewed_definition_lowering_rule() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();
        let rule = profile
            .mappings
            .lowering_rule_for_construct("PartDefinition")
            .expect("part definition lowering rule");

        assert_eq!(rule.status.as_deref(), Some("reviewed"));
        assert_eq!(rule.metaclass, "SysML::PartDefinition");
        assert_eq!(rule.emit.id_template, "type.{qualified_name}");
    }

    #[test]
    fn sysml_mappings_expose_reviewed_usage_lowering_rule() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();
        let rule = profile
            .mappings
            .lowering_rule_for_construct("PartUsage")
            .expect("part usage lowering rule");

        assert_eq!(rule.status.as_deref(), Some("reviewed"));
        assert_eq!(rule.metaclass, "SysML::PartUsage");
        assert_eq!(
            rule.emit.id_template,
            "feature.{owner_path}.{declared_name}"
        );
    }

    #[test]
    fn sysml_mappings_use_lowering_rules_for_generic_ast_keywords() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();

        assert_eq!(
            profile.mappings.definition_construct_for("connection"),
            "ConnectionDefinition"
        );
        assert_eq!(
            profile.mappings.usage_construct_for("satisfy"),
            "SatisfyUsage"
        );
    }

    #[test]
    fn sysml_mappings_load_usage_family_semantic_defaults() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();

        let package_action = profile
            .mappings
            .usage_family_default("ActionUsage", "Package")
            .expect("package action usage defaults");
        assert_eq!(package_action.type_ref, "Actions::Action");
        assert_eq!(
            package_action.subsetted_feature_refs,
            vec!["Actions::actions"]
        );
        assert!(!package_action.is_variable);

        let nested_action = profile
            .mappings
            .usage_family_default("ActionUsage", "PartDefinition")
            .expect("nested action usage defaults");
        assert_eq!(
            nested_action.subsetted_feature_refs,
            vec!["Parts::Part::ownedActions"]
        );
    }

    #[test]
    fn sysml_mappings_load_usage_type_semantic_defaults() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();
        let usage = test_usage("PartUsage", "Package");

        assert_eq!(
            profile.mappings.usage_type_default(&usage).as_deref(),
            Some("Parts::Part")
        );
    }

    #[test]
    fn sysml_mappings_load_usage_subset_semantic_defaults() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();
        let mut usage = test_usage("PortUsage", "PortDefinition");

        assert_eq!(
            profile.mappings.usage_subset_default(&usage),
            vec!["Ports::Port::subports"]
        );

        usage.modifiers.push("ref".to_string());
        assert_eq!(
            profile.mappings.usage_subset_default(&usage),
            vec!["Ports::ports"]
        );
    }

    #[test]
    fn sysml_mappings_load_specialized_feature_subset_defaults() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();
        let mut usage = test_usage("PartUsage", "Package");
        usage.specialized_features = vec!["feature.root.p".to_string()];

        assert_eq!(
            profile
                .mappings
                .specialized_feature_subset_default(&usage)
                .expect("part specialized feature subset defaults"),
            vec!["feature.root.p", "Parts::parts"]
        );

        usage.has_explicit_type = true;
        assert_eq!(
            profile
                .mappings
                .specialized_feature_subset_default(&usage)
                .expect("typed part specialized feature subset defaults"),
            vec!["feature.root.p"]
        );
    }

    #[test]
    fn sysml_mappings_load_usage_context_defaults() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();
        let package_usage = test_usage("PartUsage", "Package");
        let nested_usage = test_usage("PartUsage", "PartDefinition");
        let connection_usage = test_usage("ConnectionUsage", "Package");
        let mut directed_usage = test_usage("PartUsage", "PartDefinition");
        directed_usage.modifiers = vec!["out".to_string(), "inout".to_string()];

        assert!(!profile.mappings.usage_is_variable(&package_usage));
        assert!(!profile.mappings.usage_has_type_context(&package_usage));
        assert!(profile.mappings.usage_is_variable(&nested_usage));
        assert!(profile.mappings.usage_has_type_context(&nested_usage));
        assert_eq!(
            profile
                .mappings
                .usage_direction_from_modifiers(&directed_usage),
            Some("inout")
        );
        assert!(
            !profile
                .mappings
                .usage_counts_as_owned_member(&connection_usage)
        );
    }

    #[test]
    fn sysml_mappings_load_usage_property_defaults() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();
        let usage = test_usage("PartUsage", "ItemDefinition");
        let mut ref_usage = test_usage("PartUsage", "ItemDefinition");
        ref_usage.modifiers.push("ref".to_string());

        let defaults = profile.mappings.usage_property_defaults(&usage);
        assert_eq!(defaults.len(), 1);
        assert_eq!(defaults[0].property_refs["definition"], vec!["Parts::Part"]);
        assert!(
            profile
                .mappings
                .usage_property_defaults(&ref_usage)
                .is_empty()
        );
    }

    #[test]
    fn sysml_mappings_load_definition_context_defaults() {
        let profile = LanguageProfile::load_for_profile("sysml-2.0-metamodel-0.57.0").unwrap();
        let enumeration = test_definition("EnumerationDefinition");
        let part = test_definition("PartDefinition");

        assert!(profile.mappings.definition_is_abstract(&enumeration));
        assert!(!profile.mappings.definition_is_abstract(&part));
    }

    #[test]
    fn kerml_profile_has_no_sysml_lowering_rules() {
        let profile = LanguageProfile::load(SourceLanguage::Kerml).unwrap();

        assert!(profile.lowering_rules.is_none());
    }

    fn test_usage(construct: &str, owner_construct: &str) -> ResolvedUsage {
        ResolvedUsage {
            construct: construct.to_string(),
            owner_construct: owner_construct.to_string(),
            owner_qualified_name: "root".to_string(),
            qualified_name: "root.x".to_string(),
            declared_name: "x".to_string(),
            is_implicit_name: false,
            has_explicit_type: false,
            type_ref: None,
            additional_type_refs: Vec::new(),
            reference_target: None,
            allocation_source: None,
            allocation_target: None,
            metadata_properties: Default::default(),
            multiplicity: None,
            expression: None,
            is_derived: false,
            specializes: Vec::new(),
            specialized_features: Vec::new(),
            subsetted_features: Vec::new(),
            redefined_features: Vec::new(),
            members: Vec::new(),
            modifiers: Vec::new(),
            docs: Vec::new(),
            span: mercurio_language_contracts::ast::SourceSpan {
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 1,
            },
        }
    }

    fn test_definition(construct: &str) -> ResolvedDefinition {
        ResolvedDefinition {
            construct: construct.to_string(),
            qualified_name: "root.X".to_string(),
            declared_name: "X".to_string(),
            is_abstract: false,
            specializes: Vec::new(),
            members: Vec::new(),
            docs: Vec::new(),
            span: mercurio_language_contracts::ast::SourceSpan {
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 1,
            },
        }
    }
}
