//! Semantic elaboration phase.

use std::collections::BTreeSet;

use mercurio_language_contracts::ast::QualifiedName;

use crate::lowering::emit::MappingBundle;
use crate::lowering::ir::ResolvedUsage;

#[derive(Debug, Clone, Default)]
pub(crate) struct ReferenceUsageSemantics {
    pub(crate) type_refs: Vec<String>,
    pub(crate) semantic_specializations: Vec<String>,
    pub(crate) subsetted_feature_refs: Vec<String>,
    pub(crate) specialized_feature_refs: Vec<String>,
    pub(crate) redefined_feature_refs: Vec<String>,
    pub(crate) direction: Option<String>,
}

pub(crate) fn has_elaboration_rule(
    mappings: &MappingBundle,
    construct: &str,
    rule_id: &str,
) -> bool {
    mappings
        .lowering_rule_for_construct(construct)
        .is_some_and(|rule| rule.elaborate.iter().any(|step| step.id == rule_id))
}

pub(crate) fn should_annotate_connection_end_direction(
    mappings: &MappingBundle,
    definition_construct: &str,
) -> bool {
    definition_construct == "ConnectionDefinition"
        && has_elaboration_rule(mappings, definition_construct, "connection-end-direction")
}

pub(crate) fn should_use_implicit_reference_redefinition_target(
    mappings: &MappingBundle,
    usage: &crate::lowering::collect::CollectedUsage,
) -> bool {
    has_elaboration_rule(mappings, &usage.construct, "implicit-ref-redefines-target")
        && usage.is_implicit_name
        && usage.declared_name == "ref"
        && usage.ty.is_none()
        && usage.reference_target.is_none()
        && usage.redefines.len() == 1
}

pub(crate) fn shorthand_reference_target(
    mappings: &MappingBundle,
    usage: &crate::lowering::collect::CollectedUsage,
) -> Option<QualifiedName> {
    let rule_id = match usage.construct.as_str() {
        "SatisfyUsage" => "satisfy-name-as-reference-target",
        "VerifyUsage" => "verify-name-as-reference-target",
        _ => return None,
    };
    if has_elaboration_rule(mappings, &usage.construct, rule_id)
        && usage.reference_target.is_none()
        && !usage.declared_name.is_empty()
    {
        return Some(QualifiedName {
            segments: vec![usage.declared_name.clone()],
            span: usage.span.clone(),
        });
    }
    None
}

pub(crate) fn usage_all_type_refs(usage: &ResolvedUsage) -> Vec<String> {
    let mut refs = Vec::new();
    if let Some(type_ref) = &usage.type_ref {
        refs.push(type_ref.clone());
    }
    refs.extend(usage.additional_type_refs.clone());
    dedupe_refs(refs)
}

pub(crate) struct UsageFamilyDefaults {
    pub(crate) type_ref: String,
    pub(crate) subsetted_feature_refs: Vec<String>,
    pub(crate) is_variable: bool,
}

pub(crate) fn dedupe_refs(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use mercurio_language_contracts::ast::SourceSpan;

    use super::*;
    use crate::lowering::collect::CollectedUsage;

    fn span() -> SourceSpan {
        SourceSpan {
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 1,
        }
    }

    fn usage(construct: &str, declared_name: &str) -> CollectedUsage {
        CollectedUsage {
            construct: construct.to_string(),
            owner_construct: "Package".to_string(),
            owner_qualified_name: "root".to_string(),
            qualified_name: format!("root.{declared_name}"),
            declared_name: declared_name.to_string(),
            is_implicit_name: false,
            ty: None,
            additional_types: Vec::new(),
            reference_target: None,
            allocation_source: None,
            allocation_target: None,
            metadata_properties: BTreeMap::new(),
            multiplicity: None,
            expression: None,
            specializes: Vec::new(),
            subsets: Vec::new(),
            redefines: Vec::new(),
            members: Vec::new(),
            modifiers: Vec::new(),
            docs: Vec::new(),
            span: span(),
        }
    }

    #[test]
    fn connection_end_direction_is_rule_backed() {
        let mappings = MappingBundle::load().unwrap();

        assert!(should_annotate_connection_end_direction(
            &mappings,
            "ConnectionDefinition"
        ));
        assert!(!should_annotate_connection_end_direction(
            &mappings,
            "PartDefinition"
        ));
    }

    #[test]
    fn implicit_reference_redefinition_target_is_rule_backed() {
        let mappings = MappingBundle::load().unwrap();
        let mut reference = usage("ReferenceUsage", "ref");
        reference.is_implicit_name = true;
        reference.redefines.push(QualifiedName {
            segments: vec!["target".to_string()],
            span: span(),
        });

        assert!(should_use_implicit_reference_redefinition_target(
            &mappings, &reference
        ));
    }

    #[test]
    fn satisfy_verify_shorthand_targets_are_rule_backed() {
        let mappings = MappingBundle::load().unwrap();
        let satisfy = usage("SatisfyUsage", "requirementA");
        let verify = usage("VerifyUsage", "requirementB");
        let part = usage("PartUsage", "partA");

        assert_eq!(
            shorthand_reference_target(&mappings, &satisfy)
                .unwrap()
                .segments,
            vec!["requirementA"]
        );
        assert_eq!(
            shorthand_reference_target(&mappings, &verify)
                .unwrap()
                .segments,
            vec!["requirementB"]
        );
        assert!(shorthand_reference_target(&mappings, &part).is_none());
    }
}
