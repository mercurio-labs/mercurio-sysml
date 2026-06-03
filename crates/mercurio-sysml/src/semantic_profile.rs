use std::collections::BTreeMap;

use mercurio_core::{
    AttributePolicyAnswer, CapabilityAnswer, SemanticCapabilityOracle, SemanticConcept,
    SemanticMutationCapabilityContext, SourceLanguage, language::profile::LanguageProfile,
};

#[derive(Debug, Clone, Default)]
pub struct SysmlSemanticCapabilityOracle;

impl SemanticCapabilityOracle for SysmlSemanticCapabilityOracle {
    fn can_contain(&self, container_kind: &str, child_kind: &str) -> CapabilityAnswer {
        if container_kind.is_empty() || child_kind.is_empty() {
            CapabilityAnswer::Unknown("missing kind information".to_string())
        } else if !sysml_is_container_kind(container_kind) {
            CapabilityAnswer::Denied(format!(
                "`{container_kind}` cannot own `{child_kind}` declarations"
            ))
        } else if sysml_is_definition_keyword(child_kind)
            || sysml_is_usage_keyword(child_kind)
            || child_kind == "package"
        {
            CapabilityAnswer::Allowed
        } else {
            CapabilityAnswer::Unknown(format!("unknown child kind `{child_kind}`"))
        }
    }

    fn can_specialize(&self, source_kind: &str, target_kind: &str) -> CapabilityAnswer {
        if source_kind.is_empty() || target_kind.is_empty() {
            CapabilityAnswer::Unknown("missing kind information".to_string())
        } else {
            CapabilityAnswer::Allowed
        }
    }

    fn can_type_usage(&self, usage_kind: &str, definition_kind: &str) -> CapabilityAnswer {
        if usage_kind.is_empty() || definition_kind.is_empty() {
            CapabilityAnswer::Unknown("missing kind information".to_string())
        } else if !sysml_is_usage_keyword(usage_kind) {
            CapabilityAnswer::Denied(format!("`{usage_kind}` is not a usage kind"))
        } else if !definition_kind.to_ascii_lowercase().contains("def") {
            CapabilityAnswer::Denied(format!("`{definition_kind}` is not a definition-like type"))
        } else if usage_kind == "part" && !definition_kind.to_ascii_lowercase().contains("part") {
            CapabilityAnswer::Denied(format!(
                "part usages should be typed by part definitions, got `{definition_kind}`"
            ))
        } else {
            CapabilityAnswer::Allowed
        }
    }

    fn can_relate(
        &self,
        relationship_kind: &str,
        source_kind: &str,
        target_kind: &str,
    ) -> CapabilityAnswer {
        let relation = relationship_kind.to_ascii_lowercase();
        if !sysml_is_container_kind(source_kind) {
            return CapabilityAnswer::Denied(format!(
                "relationship source `{source_kind}` is not element-like"
            ));
        }
        let target = target_kind.to_ascii_lowercase();
        if relation.contains("satisfy") && !target.contains("requirement") {
            return CapabilityAnswer::Denied(
                "satisfy relationships must target a requirement-like element".to_string(),
            );
        }
        if relation.contains("verify") && !target.contains("requirement") {
            return CapabilityAnswer::Denied(
                "verify relationships must target a requirement-like element".to_string(),
            );
        }
        if !(relation.contains("satisfy") || relation.contains("verify")) {
            return CapabilityAnswer::Unknown(format!(
                "relationship kind `{relationship_kind}` is not yet governed"
            ));
        }
        CapabilityAnswer::Allowed
    }

    fn attribute_policy(&self, kind: &str, attribute: &str) -> AttributePolicyAnswer {
        let attribute = attribute.to_ascii_lowercase();
        let writable = matches!(
            attribute.as_str(),
            "declared_name"
                | "specializes"
                | "type"
                | "is_abstract"
                | "is_end"
                | "direction"
                | "target"
                | "imports"
                | "expression"
                | "doc"
                | "text"
                | "id"
                | "requirement_id"
        );
        AttributePolicyAnswer {
            writable,
            reason: (!writable).then(|| {
                format!("attribute `{attribute}` is not writable on `{kind}` by this service")
            }),
        }
    }

    fn relationship_uses_owner_as_source(&self, relationship_kind: &str) -> bool {
        sysml_trace_relationship_uses_owner_source(relationship_kind)
    }

    fn doc_id_attribute_aliases(&self) -> &'static [&'static str] {
        &["id", "requirement_id"]
    }
}

pub const SYSML_MUTATION_PROFILE_ID: &str = "model-v2-writable-mutation-v1";
pub const SYSML_LANGUAGE_PROFILE_ID: &str = "sysml-v2";

pub const SYSML_DEFINITION_KEYWORDS: &[&str] = &[
    "part",
    "attribute",
    "requirement",
    "item",
    "connection",
    "port",
    "action",
    "constraint",
    "calc",
    "state",
    "view",
    "verification",
];

pub const SYSML_USAGE_KEYWORDS: &[&str] = &[
    "part",
    "attribute",
    "requirement",
    "item",
    "connection",
    "port",
    "action",
    "constraint",
    "calc",
    "state",
    "satisfy",
    "verify",
    "ref",
    "reference",
];

pub const SYSML_RELATIONSHIP_KINDS: &[&str] = &["satisfy", "verify", "trace", "refine"];

pub const SYSML_MUTATION_GUIDANCE: &[&str] = &[
    "Use Model v2 textual concepts, not Model v1 block terminology.",
    "Never use keyword `block`; use `part` for part definitions and part usages.",
    "Requirement definitions should carry explicit `id` and `text` semantic attributes; use SetAttribute on existing requirement elements when those fields are missing.",
    "Return semantic mutations, not source text edits.",
    "Core feasibility remains authoritative for contextual legality.",
];

pub fn sysml_language_profile() -> LanguageProfile {
    LanguageProfile {
        id: SYSML_LANGUAGE_PROFILE_ID.to_string(),
        language: SourceLanguage::Model,
        language_version: "2.0".to_string(),
        metamodel_version: "sysml-2.0".to_string(),
        stdlib_version: "sysml-2.0".to_string(),
        stdlib_path: "resources/sysml/sysml-library.kir.json".to_string(),
        kir_schema_version: mercurio_core::ir::KIR_SCHEMA_VERSION.to_string(),
        canonical_kinds: BTreeMap::from([
            (
                SemanticConcept::Package,
                "KerML::Kernel::Package".to_string(),
            ),
            (SemanticConcept::Type, "KerML::Kernel::Type".to_string()),
        ]),
        semantic_anchors: BTreeMap::from([
            (
                "attribute_usage".to_string(),
                "SysML::Systems::AttributeUsage".to_string(),
            ),
            (
                "constraint_usage".to_string(),
                "SysML::Systems::ConstraintUsage".to_string(),
            ),
            (
                "part_definition".to_string(),
                "SysML::Systems::PartDefinition".to_string(),
            ),
            (
                "part_usage".to_string(),
                "SysML::Systems::PartUsage".to_string(),
            ),
            (
                "requirement_usage".to_string(),
                "SysML::Requirements::RequirementUsage".to_string(),
            ),
            (
                "verification_case_usage".to_string(),
                "SysML::Verification::VerificationCaseUsage".to_string(),
            ),
        ]),
        aliases: BTreeMap::from([
            (
                "Model::PartDefinition".to_string(),
                "SysML::Systems::PartDefinition".to_string(),
            ),
            (
                "Model::PartUsage".to_string(),
                "SysML::Systems::PartUsage".to_string(),
            ),
            (
                "Model::RequirementUsage".to_string(),
                "SysML::Requirements::RequirementUsage".to_string(),
            ),
        ]),
    }
}

pub fn sysml_semantic_mutation_capability_context() -> SemanticMutationCapabilityContext {
    SemanticMutationCapabilityContext {
        metamodel_version: SYSML_MUTATION_PROFILE_ID.to_string(),
        supported_operations: vec![
            "AddPackage".to_string(),
            "AddDefinition".to_string(),
            "AddUsage".to_string(),
            "AddRelationship".to_string(),
            "AddMetadataAnnotation".to_string(),
            "RenameDeclaration".to_string(),
            "UpdateUsageType".to_string(),
            "SetExpression".to_string(),
            "UpdateSpecializations".to_string(),
            "MoveDeclaration".to_string(),
            "SetAttribute".to_string(),
        ],
        definition_keywords: SYSML_DEFINITION_KEYWORDS
            .iter()
            .map(ToString::to_string)
            .collect(),
        usage_keywords: SYSML_USAGE_KEYWORDS
            .iter()
            .map(ToString::to_string)
            .collect(),
        relationship_kinds: SYSML_RELATIONSHIP_KINDS
            .iter()
            .map(ToString::to_string)
            .collect(),
        guidance: SYSML_MUTATION_GUIDANCE
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}

pub fn sysml_trace_relationship_uses_owner_source(keyword: &str) -> bool {
    matches!(
        keyword.to_ascii_lowercase().as_str(),
        "satisfy" | "verify" | "refine"
    )
}

pub fn sysml_is_satisfy_relationship(kind: &str) -> bool {
    matches!(
        kind.trim().to_ascii_lowercase().as_str(),
        "satisfy" | "satisfies"
    )
}

pub fn sysml_relationship_usage_keyword(kind: &str) -> Option<&'static str> {
    sysml_is_satisfy_relationship(kind).then_some("satisfy")
}

pub fn sysml_definition_kind(keyword: &str) -> &'static str {
    match keyword {
        "requirement" => "model.RequirementDefinition",
        "action" => "model.ActionDefinition",
        "metadata" => "model.MetadataDefinition",
        "attribute" => "model.AttributeDefinition",
        _ => "model.PartDefinition",
    }
}

pub fn sysml_usage_kind(keyword: &str) -> &'static str {
    match keyword {
        "requirement" => "model.RequirementUsage",
        "attribute" => "model.AttributeUsage",
        "satisfy" => "model.SatisfyRelationship",
        "action" => "model.ActionUsage",
        _ => "model.PartUsage",
    }
}

pub fn sysml_is_container_kind(kind: &str) -> bool {
    let lower = kind.to_ascii_lowercase();
    lower == "package" || lower.contains("def") || lower.contains("usage") || lower == "part"
}

pub fn sysml_is_definition_keyword(kind: &str) -> bool {
    SYSML_DEFINITION_KEYWORDS.contains(&kind) || kind.ends_with(" def")
}

pub fn sysml_is_usage_keyword(kind: &str) -> bool {
    SYSML_USAGE_KEYWORDS.contains(&kind)
}

pub fn normalize_definition_keyword(keyword: &str) -> String {
    keyword
        .strip_suffix(" def")
        .unwrap_or(keyword)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysml_capability_context_exposes_writable_model_v2_vocabulary() {
        let context = sysml_semantic_mutation_capability_context();

        assert_eq!(context.metamodel_version, "model-v2-writable-mutation-v1");
        assert!(
            context
                .supported_operations
                .contains(&"AddDefinition".to_string())
        );
        assert!(context.definition_keywords.contains(&"part".to_string()));
        assert!(!context.definition_keywords.contains(&"block".to_string()));
        assert!(context.relationship_kinds.contains(&"satisfy".to_string()));
        assert!(
            context
                .guidance
                .iter()
                .any(|item| item.contains("Never use keyword `block`"))
        );
    }

    #[test]
    fn sysml_profile_owns_domain_semantic_anchors() {
        let profile = sysml_language_profile();

        assert_eq!(profile.id, SYSML_LANGUAGE_PROFILE_ID);
        assert_eq!(
            profile.semantic_anchors["requirement_usage"],
            "SysML::Requirements::RequirementUsage"
        );
        assert_eq!(
            profile.canonical_kinds[&SemanticConcept::Package],
            "KerML::Kernel::Package"
        );
    }

    #[test]
    fn sysml_oracle_blocks_satisfy_to_non_requirement() {
        let oracle = SysmlSemanticCapabilityOracle;

        let answer = oracle.can_relate("satisfy", "part", "part");

        assert!(matches!(
            answer,
            CapabilityAnswer::Denied(message) if message.contains("must target a requirement")
        ));
    }

    #[test]
    fn sysml_oracle_blocks_part_usage_typed_by_requirement_definition() {
        let oracle = SysmlSemanticCapabilityOracle;

        let answer = oracle.can_type_usage("part", "requirement def");

        assert!(matches!(
            answer,
            CapabilityAnswer::Denied(message)
                if message.contains("part usages should be typed by part definitions")
        ));
    }

    #[test]
    fn sysml_oracle_allows_requirement_id_and_text_attributes() {
        let oracle = SysmlSemanticCapabilityOracle;

        assert!(oracle.attribute_policy("requirement", "id").writable);
        assert!(oracle.attribute_policy("requirement", "text").writable);
        assert!(
            oracle
                .attribute_policy("requirement", "requirement_id")
                .writable
        );
    }
}
