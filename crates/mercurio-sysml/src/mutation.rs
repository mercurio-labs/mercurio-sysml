use std::collections::{BTreeMap, BTreeSet};

use mercurio_core::{
    Atom, AuthoringProject, CoreMutationFeasibilityService, DiagnosticRule, ElementRef, Fact,
    RuleDiagnosticSeverity, RulePack, SemanticLegalityService, SemanticMutationCapabilityContext,
    SemanticNextActionsService, SemanticReasoningContext, SemanticRelationshipTargetRuleContext,
    SemanticUsageTypingRuleContext, Term, WorkspaceRevision,
    default_semantic_variant_capability_context,
    enrich_semantic_reasoning_context_with_next_action_affordances,
    semantic_reasoning_context_from_authoring_project_with_oracle,
};

use crate::SysmlEnvironmentError;
use crate::metamodel::{
    LEGACY_SYSML_2_0_PILOT_057_ID, SYSML_2_0_METAMODEL_057_ID, SYSML_2_0_PILOT_2026_04_ID,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::release_bundle;
use crate::semantic_profile::{
    SysmlSemanticCapabilityOracle, sysml_allowed_definition_keywords_for_usage,
    sysml_definition_element_kinds, sysml_definition_keyword_for_usage, sysml_definition_keywords,
    sysml_extension_relationship_keywords, sysml_grammar_containment_mismatches,
    sysml_is_requirement_kind, sysml_relationship_keywords, sysml_usage_element_kinds,
    sysml_usage_keywords,
};

pub const SYSML_MUTATION_PROFILE_ID: &str = "sysml-v2-writable-mutation-v1";

pub const SYSML_MUTATION_GUIDANCE: &[&str] = &[
    "Use SysML v2 textual concepts, not SysML v1 block terminology.",
    "Never use keyword `block`; use `part` for part definitions and part usages.",
    "Requirement definitions should carry explicit `id` and `text` semantic attributes; use SetAttribute on existing requirement elements when those fields are missing.",
    "Return semantic mutations, not source text edits.",
    "Foundation feasibility remains authoritative for contextual legality.",
];

pub type SysmlMutationFeasibilityService =
    CoreMutationFeasibilityService<SysmlSemanticCapabilityOracle>;
pub type SysmlSemanticLegalityService = SemanticLegalityService<SysmlSemanticCapabilityOracle>;
pub type SysmlSemanticNextActionsService =
    SemanticNextActionsService<SysmlSemanticCapabilityOracle>;

pub fn sysml_mutation_feasibility_service() -> SysmlMutationFeasibilityService {
    match sysml_semantic_legality_rulepacks_for_release("latest") {
        Ok(rulepacks) => CoreMutationFeasibilityService::with_oracle_and_rulepacks(
            SysmlSemanticCapabilityOracle,
            rulepacks,
        ),
        Err(_) => CoreMutationFeasibilityService::with_oracle_and_rulepacks(
            SysmlSemanticCapabilityOracle,
            vec![sysml_semantic_legality_rulepack()],
        ),
    }
}

pub fn sysml_semantic_legality_service() -> SysmlSemanticLegalityService {
    match sysml_semantic_legality_service_for_release("latest") {
        Ok(service) => service,
        Err(_) => SemanticLegalityService::with_oracle_and_rulepacks(
            SysmlSemanticCapabilityOracle,
            vec![sysml_semantic_legality_rulepack()],
        ),
    }
}

pub fn sysml_semantic_next_actions_service() -> SysmlSemanticNextActionsService {
    match sysml_semantic_next_actions_service_for_release("latest") {
        Ok(service) => service,
        Err(_) => SemanticNextActionsService::with_legality(
            sysml_semantic_legality_service(),
            sysml_semantic_mutation_capability_context(),
        ),
    }
}

pub fn sysml_semantic_next_actions_service_for_release(
    selector: &str,
) -> Result<SysmlSemanticNextActionsService, SysmlEnvironmentError> {
    Ok(SemanticNextActionsService::with_legality(
        sysml_semantic_legality_service_for_release(selector)?,
        sysml_semantic_mutation_capability_context(),
    ))
}

pub fn sysml_semantic_legality_service_for_release(
    selector: &str,
) -> Result<SysmlSemanticLegalityService, SysmlEnvironmentError> {
    Ok(SemanticLegalityService::with_oracle_and_rulepacks(
        SysmlSemanticCapabilityOracle,
        sysml_semantic_legality_rulepacks_for_release(selector)?,
    ))
}

pub fn sysml_semantic_legality_rulepacks_for_release(
    selector: &str,
) -> Result<Vec<RulePack>, SysmlEnvironmentError> {
    Ok(vec![sysml_semantic_legality_rulepack_for_release(
        selector,
    )?])
}

pub fn sysml_semantic_legality_rulepack_for_release(
    selector: &str,
) -> Result<RulePack, SysmlEnvironmentError> {
    #[cfg(target_arch = "wasm32")]
    {
        sysml_embedded_semantic_legality_rulepack_for_release(selector)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let bundle = release_bundle(selector)?;
        Ok(RulePack::from_path(&bundle.rulepack_path)?)
    }
}

pub fn sysml_semantic_legality_rulepack() -> RulePack {
    sysml_embedded_semantic_legality_rulepack_for_release("latest")
        .expect("embedded latest SysML legality rulepack parses")
}

pub fn sysml_semantic_legality_base_rulepack() -> RulePack {
    let mut facts = BTreeSet::new();
    facts.insert(Fact::new(
        "sysml_deprecated_v1_keyword",
        ["block".to_string()],
    ));
    for (relationship, source) in sysml_extension_relationship_keywords() {
        facts.insert(Fact::new(
            "sysml_extension_relationship_kind",
            [relationship.to_string(), source.to_string()],
        ));
    }
    for (container, child) in sysml_grammar_containment_mismatches() {
        facts.insert(Fact::new(
            "sysml_grammar_containment_mismatch_kind",
            [container.to_string(), child.to_string()],
        ));
    }
    for (keyword, _) in sysml_usage_element_kinds()
        .iter()
        .copied()
        .filter(|(_, kind)| !sysml_is_requirement_kind(kind))
    {
        facts.insert(Fact::new(
            "sysml_non_requirement_relationship_target_kind",
            [keyword.to_string()],
        ));
    }
    for (keyword, _) in sysml_definition_element_kinds()
        .iter()
        .copied()
        .filter(|(_, kind)| !sysml_is_requirement_kind(kind))
    {
        let definition_kind = format!("{keyword} def");
        facts.insert(Fact::new(
            "sysml_non_requirement_relationship_target_kind",
            [definition_kind.clone()],
        ));
    }
    for usage in sysml_usage_keywords().iter().copied() {
        let Some(expected_definition) = sysml_definition_keyword_for_usage(usage) else {
            continue;
        };
        let allowed_definition_kinds = sysml_allowed_definition_keywords_for_usage(usage)
            .into_iter()
            .map(|definition| format!("{definition} def"))
            .collect::<BTreeSet<_>>();
        for definition in sysml_definition_keywords().iter().copied() {
            let definition_kind = format!("{definition} def");
            if allowed_definition_kinds.contains(&definition_kind) {
                continue;
            }
            facts.insert(Fact::new(
                "sysml_usage_typing_mismatch_kind",
                [
                    usage.to_string(),
                    format!("{expected_definition} def"),
                    definition_kind,
                ],
            ));
        }
    }

    let mut diagnostics = vec![
        relationship_target_diagnostic("satisfy"),
        relationship_target_diagnostic("verify"),
        deprecated_keyword_diagnostic(
            "sysml.deprecated.block.containment_container",
            "legality_containment_request",
            vec![
                Term::Var("DeprecatedKind".to_string()),
                Term::Var("ChildKind".to_string()),
            ],
        ),
        deprecated_keyword_diagnostic(
            "sysml.deprecated.block.containment_child",
            "legality_containment_request",
            vec![
                Term::Var("ContainerKind".to_string()),
                Term::Var("DeprecatedKind".to_string()),
            ],
        ),
        grammar_containment_warning_diagnostic(),
        deprecated_keyword_diagnostic(
            "sysml.deprecated.block.specialization_source",
            "legality_specialization_request",
            vec![
                Term::Var("DeprecatedKind".to_string()),
                Term::Var("TargetKind".to_string()),
            ],
        ),
        deprecated_keyword_diagnostic(
            "sysml.deprecated.block.specialization_target",
            "legality_specialization_request",
            vec![
                Term::Var("SourceKind".to_string()),
                Term::Var("DeprecatedKind".to_string()),
            ],
        ),
        deprecated_keyword_diagnostic(
            "sysml.deprecated.block.usage_typing_usage",
            "legality_usage_typing_request",
            vec![
                Term::Var("DeprecatedKind".to_string()),
                Term::Var("DefinitionKind".to_string()),
            ],
        ),
        deprecated_keyword_diagnostic(
            "sysml.deprecated.block.usage_typing_definition",
            "legality_usage_typing_request",
            vec![
                Term::Var("UsageKind".to_string()),
                Term::Var("DeprecatedKind".to_string()),
            ],
        ),
        deprecated_keyword_diagnostic(
            "sysml.deprecated.block.relationship_source",
            "legality_relationship_request",
            vec![
                Term::Var("RelationshipKind".to_string()),
                Term::Var("DeprecatedKind".to_string()),
                Term::Var("TargetKind".to_string()),
            ],
        ),
        deprecated_keyword_diagnostic(
            "sysml.deprecated.block.relationship_target",
            "legality_relationship_request",
            vec![
                Term::Var("RelationshipKind".to_string()),
                Term::Var("SourceKind".to_string()),
                Term::Var("DeprecatedKind".to_string()),
            ],
        ),
        deprecated_keyword_diagnostic(
            "sysml.deprecated.block.attribute_write",
            "legality_attribute_write_request",
            vec![
                Term::Var("DeprecatedKind".to_string()),
                Term::Var("Attribute".to_string()),
            ],
        ),
    ];
    diagnostics.extend(sysml_usage_keywords().iter().copied().filter_map(|usage| {
        sysml_definition_keyword_for_usage(usage)
            .map(|definition| usage_typing_diagnostic(usage, definition))
    }));

    RulePack {
        id: "sysml.semantic_legality".to_string(),
        version: "0.1.0".to_string(),
        metadata: BTreeMap::new(),
        facts: facts.into_iter().collect(),
        rules: Vec::new(),
        diagnostics,
    }
}

fn sysml_embedded_semantic_legality_rulepack_for_release(
    selector: &str,
) -> Result<RulePack, SysmlEnvironmentError> {
    let raw = match selector {
        "latest"
        | "2026-01"
        | "0.57.0"
        | "pilot-0.57.0"
        | SYSML_2_0_METAMODEL_057_ID
        | LEGACY_SYSML_2_0_PILOT_057_ID => {
            crate::embedded_resources::SYSML_2_0_METAMODEL_057_RULEPACK
        }
        "2026-04" | "pilot-2026-04" | SYSML_2_0_PILOT_2026_04_ID => {
            crate::embedded_resources::SYSML_2_0_PILOT_2026_04_RULEPACK
        }
        _ => {
            return Err(SysmlEnvironmentError::UnknownMetamodel(
                selector.to_string(),
            ));
        }
    };
    Ok(RulePack::from_str(raw)?)
}

fn deprecated_keyword_diagnostic(
    id: &str,
    request_predicate: &str,
    request_terms: Vec<Term>,
) -> DiagnosticRule {
    DiagnosticRule {
        id: id.to_string(),
        severity: RuleDiagnosticSeverity::Error,
        message: "SysML v2 does not use `block`; use `part` instead".to_string(),
        subjects: vec![Term::Var("DeprecatedKind".to_string())],
        when: vec![
            Atom {
                predicate: request_predicate.to_string(),
                terms: request_terms,
            },
            Atom {
                predicate: "sysml_deprecated_v1_keyword".to_string(),
                terms: vec![Term::Var("DeprecatedKind".to_string())],
            },
        ],
    }
}

fn relationship_target_diagnostic(kind: &str) -> DiagnosticRule {
    DiagnosticRule {
        id: format!("sysml.{kind}.target_requirement"),
        severity: RuleDiagnosticSeverity::Error,
        message: format!("{kind} relationships must target requirement-like elements"),
        subjects: vec![Term::Var("TargetKind".to_string())],
        when: vec![
            Atom {
                predicate: "legality_relationship_request".to_string(),
                terms: vec![
                    Term::Const(kind.to_string()),
                    Term::Var("SourceKind".to_string()),
                    Term::Var("TargetKind".to_string()),
                ],
            },
            Atom {
                predicate: "sysml_non_requirement_relationship_target_kind".to_string(),
                terms: vec![Term::Var("TargetKind".to_string())],
            },
        ],
    }
}

fn grammar_containment_warning_diagnostic() -> DiagnosticRule {
    DiagnosticRule {
        id: "sysml.grammar.containment.member_path".to_string(),
        severity: RuleDiagnosticSeverity::Warning,
        message: "containment pair is not present in the grammar-derived member paths".to_string(),
        subjects: vec![
            Term::Var("ContainerKind".to_string()),
            Term::Var("ChildKind".to_string()),
        ],
        when: vec![
            Atom {
                predicate: "legality_containment_request".to_string(),
                terms: vec![
                    Term::Var("ContainerKind".to_string()),
                    Term::Var("ChildKind".to_string()),
                ],
            },
            Atom {
                predicate: "sysml_grammar_containment_mismatch_kind".to_string(),
                terms: vec![
                    Term::Var("ContainerKind".to_string()),
                    Term::Var("ChildKind".to_string()),
                ],
            },
        ],
    }
}

fn usage_typing_diagnostic(usage: &str, expected_definition: &str) -> DiagnosticRule {
    DiagnosticRule {
        id: format!("sysml.{usage}.typing.{expected_definition}_definition"),
        severity: RuleDiagnosticSeverity::Error,
        message: format!("{usage} usages should be typed by {expected_definition} definitions"),
        subjects: vec![Term::Var("DefinitionKind".to_string())],
        when: vec![
            Atom {
                predicate: "legality_usage_typing_request".to_string(),
                terms: vec![
                    Term::Const(usage.to_string()),
                    Term::Var("DefinitionKind".to_string()),
                ],
            },
            Atom {
                predicate: "sysml_usage_typing_mismatch_kind".to_string(),
                terms: vec![
                    Term::Const(usage.to_string()),
                    Term::Const(format!("{expected_definition} def")),
                    Term::Var("DefinitionKind".to_string()),
                ],
            },
        ],
    }
}

pub fn sysml_semantic_mutation_capability_context() -> SemanticMutationCapabilityContext {
    SemanticMutationCapabilityContext {
        metamodel_version: SYSML_MUTATION_PROFILE_ID.to_string(),
        supported_operations: vec![
            "AddPackage".to_string(),
            "AddElement".to_string(),
            "AddDefinition".to_string(),
            "AddUsage".to_string(),
            "AddRelationship".to_string(),
            "AddMetadataAnnotation".to_string(),
            "Remove".to_string(),
            "RemoveRelationship".to_string(),
            "RenameDeclaration".to_string(),
            "UpdateUsageType".to_string(),
            "SetExpression".to_string(),
            "UpdateSpecializations".to_string(),
            "MoveDeclaration".to_string(),
            "SetAttribute".to_string(),
        ],
        variant_capabilities: default_semantic_variant_capability_context(),
        element_kinds: sysml_semantic_element_kinds(),
        definition_keywords: sysml_definition_keywords()
            .iter()
            .map(ToString::to_string)
            .collect(),
        usage_keywords: sysml_usage_keywords()
            .iter()
            .map(ToString::to_string)
            .collect(),
        relationship_kinds: sysml_relationship_keywords()
            .iter()
            .map(ToString::to_string)
            .collect(),
        usage_typing_rules: sysml_usage_typing_rule_contexts(),
        relationship_target_rules: sysml_relationship_target_rule_contexts(),
        guidance: SYSML_MUTATION_GUIDANCE
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}

fn sysml_semantic_element_kinds() -> Vec<String> {
    sysml_definition_element_kinds()
        .iter()
        .map(|(_, kind)| kind.to_string())
        .chain(
            sysml_usage_element_kinds()
                .iter()
                .map(|(_, kind)| kind.to_string()),
        )
        .collect()
}

fn sysml_usage_typing_rule_contexts() -> Vec<SemanticUsageTypingRuleContext> {
    sysml_usage_keywords()
        .iter()
        .copied()
        .filter_map(|usage| {
            let expected_definition = sysml_definition_keyword_for_usage(usage)?;
            Some(SemanticUsageTypingRuleContext {
                usage_keyword: usage.to_string(),
                expected_definition_keyword: expected_definition.to_string(),
                expected_definition_kind: format!("{expected_definition} def"),
                message: format!(
                    "{usage} usages should be typed by {expected_definition} definitions"
                ),
            })
        })
        .collect()
}

fn sysml_relationship_target_rule_contexts() -> Vec<SemanticRelationshipTargetRuleContext> {
    ["satisfy", "verify"]
        .into_iter()
        .map(|relationship_kind| SemanticRelationshipTargetRuleContext {
            relationship_kind: relationship_kind.to_string(),
            expected_target_kind: "requirement".to_string(),
            message: format!(
                "{relationship_kind} relationships must target requirement-like elements"
            ),
        })
        .collect()
}

pub fn sysml_semantic_reasoning_context_from_authoring_project(
    project: &AuthoringProject,
    workspace_revision: WorkspaceRevision,
    focus: Vec<ElementRef>,
    max_elements: usize,
) -> SemanticReasoningContext {
    let mut context = semantic_reasoning_context_from_authoring_project_with_oracle(
        project,
        workspace_revision,
        focus,
        max_elements,
        &SysmlSemanticCapabilityOracle,
    );
    context.metamodel_version = "sysml-v2-authoring-context-v1".to_string();
    context
}

pub fn enrich_sysml_semantic_reasoning_context_with_child_affordances(
    context: &mut SemanticReasoningContext,
    max_affordances: usize,
) {
    let service = sysml_semantic_next_actions_service();
    enrich_semantic_reasoning_context_with_next_action_affordances(
        context,
        max_affordances,
        &service,
    );
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use mercurio_core::{
        ElementRef, FeasibilityStatus, Graph, KirDocument, MutationApplicationResult,
        MutationContext, MutationFeasibilityService, MutationProposal, RulePack,
        SemanticElementKind, SemanticLegalityDiagnosticSource, SemanticLegalityRequest,
        SemanticLegalityStatus, SemanticMutation, SemanticNextActionOperation,
        SemanticNextActionsRequest, WorkspaceRevision, WriteBackMode,
    };
    use serde_json::{Value, json};

    use super::*;
    use crate::{available_release_bundles, load_authoring_project_from_sysml};

    #[test]
    fn sysml_capability_context_exposes_writable_sysml_v2_vocabulary() {
        let context = sysml_semantic_mutation_capability_context();

        assert_eq!(context.metamodel_version, SYSML_MUTATION_PROFILE_ID);
        assert!(
            context
                .supported_operations
                .contains(&"AddDefinition".to_string())
        );
        assert!(context.definition_keywords.contains(&"part".to_string()));
        assert!(
            context
                .definition_keywords
                .contains(&"analysis".to_string())
        );
        assert!(context.definition_keywords.contains(&"enum".to_string()));
        assert!(
            context
                .supported_operations
                .contains(&"AddElement".to_string())
        );
        assert!(context.element_kinds.contains(&"StateUsage".to_string()));
        assert!(
            context
                .element_kinds
                .contains(&"PartDefinition".to_string())
        );
        assert!(!context.definition_keywords.contains(&"block".to_string()));
        assert!(context.relationship_kinds.contains(&"satisfy".to_string()));
        assert!(context.relationship_kinds.contains(&"allocate".to_string()));
        assert!(!context.relationship_kinds.contains(&"trace".to_string()));
        assert!(!context.usage_keywords.contains(&"ref".to_string()));
        assert!(context.usage_typing_rules.iter().any(|rule| {
            rule.usage_keyword == "action" && rule.expected_definition_kind == "action def"
        }));
        assert!(context.relationship_target_rules.iter().any(|rule| {
            rule.relationship_kind == "satisfy" && rule.expected_target_kind == "requirement"
        }));
        assert!(
            context
                .variant_capabilities
                .supported_operations
                .contains(&"CreateExplorationVariant".to_string())
        );
        assert!(
            context
                .guidance
                .iter()
                .any(|item| item.contains("Never use keyword `block`"))
        );
    }

    #[test]
    fn sysml_capability_context_matches_generated_keyword_registry() {
        let context = sysml_semantic_mutation_capability_context();
        let expected_definition_keywords = sysml_definition_keywords()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let expected_usage_keywords = sysml_usage_keywords()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let expected_relationship_keywords = sysml_relationship_keywords()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let expected_element_kinds = sysml_definition_element_kinds()
            .iter()
            .chain(sysml_usage_element_kinds().iter())
            .map(|(_, kind)| kind.to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            context.definition_keywords, expected_definition_keywords,
            "mutation definitions must match the generated compiler registry"
        );
        assert_eq!(
            context.usage_keywords, expected_usage_keywords,
            "mutation usages must match the generated compiler registry"
        );
        assert_eq!(
            context.relationship_kinds, expected_relationship_keywords,
            "mutation relationships must be the generated relationship-family subset"
        );
        assert_eq!(
            context.element_kinds, expected_element_kinds,
            "mutation metaclasses must come from the generated registry"
        );
    }

    #[test]
    fn sysml_legality_service_reports_rulepack_diagnostics() {
        let report = sysml_semantic_legality_service().check(
            SemanticLegalityRequest::relationship("satisfy", "part", "part"),
        );

        assert_eq!(report.status, SemanticLegalityStatus::Blocked);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.source == SemanticLegalityDiagnosticSource::Rulepack
                && diagnostic.code == "sysml.satisfy.target_requirement"
        }));
    }

    #[test]
    fn sysml_legality_loads_generated_bundle_rulepack() {
        let rulepack = sysml_semantic_legality_rulepack_for_release("latest")
            .expect("latest SysML legality rulepack loads");

        assert_eq!(rulepack.id, "sysml.semantic_legality");
        assert_eq!(
            rulepack
                .metadata
                .get("stdlibPath")
                .and_then(|value| value.as_str()),
            Some("stdlib/stdlib.full.kir.json")
        );
        assert!(
            rulepack
                .facts
                .iter()
                .any(|fact| fact.predicate == "relationship_kind")
        );
        assert!(
            rulepack
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.id == "sysml.verify.target_requirement" })
        );
        assert!(rulepack.facts.iter().any(|fact| {
            fact.predicate == "sysml_non_requirement_relationship_target_kind"
                && fact.terms == vec!["analysis def".to_string()]
        }));
        assert!(!rulepack.facts.iter().any(|fact| {
            fact.predicate == "sysml_non_requirement_relationship_target_kind"
                && fact.terms == vec!["concern".to_string()]
        }));
        assert!(!rulepack.facts.iter().any(|fact| {
            fact.predicate == "sysml_non_requirement_relationship_target_kind"
                && fact.terms == vec!["viewpoint def".to_string()]
        }));
        assert!(rulepack.facts.iter().any(|fact| {
            fact.predicate == "sysml_usage_typing_mismatch_kind"
                && fact.terms
                    == vec![
                        "analysis".to_string(),
                        "analysis def".to_string(),
                        "part def".to_string(),
                    ]
        }));
        assert!(!rulepack.facts.iter().any(|fact| {
            fact.predicate == "sysml_usage_typing_mismatch_kind"
                && fact.terms
                    == vec![
                        "concern".to_string(),
                        "concern def".to_string(),
                        "requirement def".to_string(),
                    ]
        }));
        assert!(rulepack.facts.iter().any(|fact| {
            fact.predicate == "sysml_extension_relationship_kind"
                && fact.terms == vec!["trace".to_string(), "mercurio-extension".to_string()]
        }));
        assert!(rulepack.facts.iter().any(|fact| {
            fact.predicate == "sysml_grammar_containment_mismatch_kind"
                && fact.terms == vec!["PartUsage".to_string(), "transition".to_string()]
        }));
        assert!(!rulepack.facts.iter().any(|fact| {
            fact.predicate == "sysml_grammar_containment_mismatch_kind"
                && fact.terms == vec!["StateUsage".to_string(), "transition".to_string()]
        }));
    }

    #[test]
    fn sysml_embedded_rulepack_matches_latest_bundle_rulepack() {
        let embedded = sysml_semantic_legality_rulepack();
        let bundled = sysml_semantic_legality_rulepack_for_release("latest")
            .expect("latest SysML legality rulepack loads");

        assert_eq!(
            embedded, bundled,
            "embedded wasm/default fallback must match the shipped latest rulepack"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn sysml_bundle_rulepacks_match_fresh_registry_generation() {
        for bundle in available_release_bundles().expect("release bundles load") {
            let stdlib = KirDocument::from_path(&bundle.stdlib_path).expect("release stdlib loads");
            let adapter = crate::sysml_metamodel_adapter_from_graph(
                &Graph::from_document(stdlib).expect("stdlib graph builds"),
            );
            let regenerated = merge_rulepacks_for_test(
                adapter,
                sysml_semantic_legality_base_rulepack(),
                regenerated_rulepack_metadata_for_test(&bundle),
            );
            let shipped =
                RulePack::from_path(&bundle.rulepack_path).expect("shipped rulepack loads");

            assert_eq!(
                regenerated, shipped,
                "shipped rulepack for {} must match fresh registry generation",
                bundle.profile_id
            );
        }
    }

    #[test]
    fn sysml_legality_service_for_release_uses_bundle_rulepack() {
        let service = sysml_semantic_legality_service_for_release("latest")
            .expect("latest SysML legality service loads");
        let report = service.check(SemanticLegalityRequest::usage_typing(
            "part",
            "requirement def",
        ));

        assert_eq!(report.status, SemanticLegalityStatus::Blocked);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.source == SemanticLegalityDiagnosticSource::Rulepack
                && diagnostic.code == "sysml.part.typing.part_definition"
        }));
    }

    #[test]
    fn sysml_legality_rulepack_reports_usage_typing_family_mismatches() {
        let report = sysml_semantic_legality_service().check(
            SemanticLegalityRequest::usage_typing("action", "constraint def"),
        );

        assert_eq!(report.status, SemanticLegalityStatus::Blocked);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.source == SemanticLegalityDiagnosticSource::Rulepack
                && diagnostic.code == "sysml.action.typing.action_definition"
                && diagnostic.subjects == vec!["constraint def".to_string()]
                && diagnostic.source_facts.iter().any(|fact| {
                    fact.predicate == "sysml_usage_typing_mismatch_kind"
                        && fact.terms
                            == vec![
                                "action".to_string(),
                                "action def".to_string(),
                                "constraint def".to_string(),
                            ]
                })
        }));
    }

    #[test]
    fn sysml_legality_allows_usage_typing_by_definition_ancestors() {
        let report = sysml_semantic_legality_service().check(
            SemanticLegalityRequest::usage_typing("concern", "requirement def"),
        );

        assert_eq!(
            report.status,
            SemanticLegalityStatus::Allowed,
            "{report:#?}"
        );
        assert!(report.diagnostics.is_empty(), "{report:#?}");
    }

    #[test]
    fn sysml_legality_warns_on_grammar_containment_mismatch() {
        let report = sysml_semantic_legality_service().check(SemanticLegalityRequest::containment(
            "PartUsage",
            "transition",
        ));

        assert_eq!(
            report.status,
            SemanticLegalityStatus::AllowedWithWarnings,
            "{report:#?}"
        );
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.source == SemanticLegalityDiagnosticSource::Rulepack
                && diagnostic.code == "sysml.grammar.containment.member_path"
                && diagnostic.subjects == vec!["PartUsage".to_string(), "transition".to_string()]
        }));

        let allowed = sysml_semantic_legality_service().check(
            SemanticLegalityRequest::containment("StateUsage", "transition"),
        );
        assert_eq!(
            allowed.status,
            SemanticLegalityStatus::Allowed,
            "{allowed:#?}"
        );
    }

    #[test]
    fn sysml_legality_rulepack_blocks_deprecated_block_keyword() {
        let report = sysml_semantic_legality_service()
            .check(SemanticLegalityRequest::containment("package", "block"));

        assert_eq!(report.status, SemanticLegalityStatus::Blocked);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.source == SemanticLegalityDiagnosticSource::Rulepack
                && diagnostic.code == "sysml.deprecated.block.containment_child"
                && diagnostic.subjects == vec!["block".to_string()]
        }));
    }

    #[test]
    fn sysml_generated_rulepack_blocks_deprecated_block_keyword() {
        let service = sysml_semantic_legality_service_for_release("latest")
            .expect("latest SysML legality service loads");
        let report = service.check(SemanticLegalityRequest::specialization("block", "part"));

        assert_eq!(report.status, SemanticLegalityStatus::Blocked);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.source == SemanticLegalityDiagnosticSource::Rulepack
                && diagnostic.code == "sysml.deprecated.block.specialization_source"
                && diagnostic.subjects == vec!["block".to_string()]
        }));
    }

    #[test]
    fn sysml_next_actions_include_legality_for_allowed_and_blocked_candidates() {
        let service = sysml_semantic_next_actions_service();
        let report = service.next_actions(SemanticNextActionsRequest {
            element: Some(ElementRef::new("HybridVehicle.vehicle")),
            element_kind: "part".to_string(),
            candidate_target_kinds: vec!["requirement".to_string(), "part".to_string()],
            candidate_targets: Vec::new(),
            candidate_attributes: vec!["text".to_string()],
            facts: Vec::new(),
            max_actions: None,
        });

        assert!(report.actions.iter().any(|action| {
            action.operation
                == SemanticNextActionOperation::AddRelationship {
                    relationship_kind: "satisfy".to_string(),
                    target_kind: "requirement".to_string(),
                    target: None,
                }
                && action.status == SemanticLegalityStatus::Allowed
        }));
        assert!(report.actions.iter().any(|action| {
            action.operation
                == SemanticNextActionOperation::AddRelationship {
                    relationship_kind: "satisfy".to_string(),
                    target_kind: "part".to_string(),
                    target: None,
                }
                && action.status == SemanticLegalityStatus::Blocked
                && action.legality.diagnostics.iter().any(|diagnostic| {
                    diagnostic.source == SemanticLegalityDiagnosticSource::Rulepack
                        && diagnostic.code == "sysml.satisfy.target_requirement"
                })
        }));
        assert!(report.actions.iter().any(|action| {
            action.operation
                == SemanticNextActionOperation::AddRelationship {
                    relationship_kind: "allocate".to_string(),
                    target_kind: "part".to_string(),
                    target: None,
                }
                && action.status == SemanticLegalityStatus::Allowed
        }));
    }

    #[test]
    fn sysml_context_uses_owner_as_source_for_trace_relationships() {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "hybrid.sysml".to_string(),
            r#"
package HybridVehicle {
    part def Vehicle {
        action def RegenerativeBraking {
            satisfy EfficiencyRequirement references EfficiencyRequirement;
        }
    }

    requirement def EfficiencyRequirement;
}
"#
            .to_string(),
        )]))
        .expect("project parses");

        let context = sysml_semantic_reasoning_context_from_authoring_project(
            &project,
            WorkspaceRevision::unchecked(),
            Vec::new(),
            64,
        );

        assert!(context.relationships.iter().any(|relationship| {
            relationship.kind == "satisfy"
                && relationship.source.qualified_name == "HybridVehicle.Vehicle.RegenerativeBraking"
                && relationship
                    .target
                    .qualified_name
                    .ends_with("EfficiencyRequirement")
        }));
    }

    #[test]
    fn sysml_context_exposes_sysml_child_affordances() {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "vehicle.sysml".to_string(),
            r#"
package HybridVehicle {
    part HybridVehicle;
}
"#
            .to_string(),
        )]))
        .expect("project parses");
        let mut context = sysml_semantic_reasoning_context_from_authoring_project(
            &project,
            WorkspaceRevision::unchecked(),
            vec![ElementRef::new("HybridVehicle.HybridVehicle")],
            64,
        );

        enrich_sysml_semantic_reasoning_context_with_child_affordances(&mut context, 256);

        assert!(context.affordances.iter().any(|affordance| {
            affordance.operation == "AddElement"
                && affordance.child_kind == "PartDefinition"
                && affordance.status == "Allowed"
        }));
        assert!(context.affordances.iter().any(|affordance| {
            affordance.operation == "AddDefinition"
                && affordance.child_kind == "part"
                && affordance.status == "Allowed"
        }));
        assert!(context.affordances.iter().any(|affordance| {
            affordance.operation == "AddUsage"
                && affordance.child_kind == "satisfy"
                && affordance.status == "Allowed"
        }));
    }

    #[test]
    fn sysml_context_exposes_relationship_affordances_to_actual_requirements() {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "vehicle.sysml".to_string(),
            r#"
package HybridVehicle {
    part vehicle;
    requirement def EfficiencyRequirement;
    requirement efficiencyRequirement : EfficiencyRequirement;
}
"#
            .to_string(),
        )]))
        .expect("project parses");
        let mut context = sysml_semantic_reasoning_context_from_authoring_project(
            &project,
            WorkspaceRevision::unchecked(),
            vec![ElementRef::new("HybridVehicle.vehicle")],
            64,
        );

        enrich_sysml_semantic_reasoning_context_with_child_affordances(&mut context, 256);

        assert!(context.affordances.iter().any(|affordance| {
            affordance.operation == "AddRelationship"
                && affordance.child_kind == "satisfy -> HybridVehicle.efficiencyRequirement"
                && affordance.status == "Allowed"
        }));
    }

    #[test]
    fn sysml_feasibility_suggests_matching_definition_for_missing_usage_type() {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "hybrid.sysml".to_string(),
            r#"
package HybridVehicle {
    part def Vehicle;
    part vehicle : Vehicle;
}
"#
            .to_string(),
        )]))
        .expect("project parses");
        let context = MutationContext::from_project(project);
        let proposal = MutationProposal {
            intent: "Add missing regenerative braking usage".to_string(),
            operations: vec![SemanticMutation::AddUsage {
                container: ElementRef::new("HybridVehicle.vehicle"),
                keyword: "part".to_string(),
                name: "regenerativeBraking".to_string(),
                ty: Some(ElementRef::new("HybridVehicle.RegenerativeBrakingSystem")),
                specializes: Vec::new(),
            }],
            evidence: Vec::new(),
            rationale: None,
            workspace_revision: context.workspace_revision.clone(),
        };

        let report = sysml_mutation_feasibility_service().check(&context, &proposal);

        assert_eq!(
            report.status,
            FeasibilityStatus::RequiresSupportingChanges,
            "{report:#?}"
        );
        assert!(matches!(
            &report.suggested_supporting_changes[0],
            SemanticMutation::AddElement { kind, name, .. }
                if kind.metaclass == "PartDefinition" && name == "RegenerativeBrakingSystem"
        ));
    }

    #[test]
    fn sysml_semantic_add_element_state_usage_renders_state_source() {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "controller.sysml".to_string(),
            r#"
package Control {
    state def Controller {
        state operational;
    }
}
"#
            .to_string(),
        )]))
        .expect("project parses");
        let context = MutationContext::from_project(project);
        let proposal = MutationProposal {
            intent: "Add nested state semantically".to_string(),
            operations: vec![SemanticMutation::AddElement {
                container: ElementRef::new("Control.Controller.operational"),
                kind: SemanticElementKind::new("StateUsage"),
                name: "armed".to_string(),
                ty: None,
                specializes: Vec::new(),
                properties: BTreeMap::new(),
            }],
            evidence: Vec::new(),
            rationale: None,
            workspace_revision: context.workspace_revision.clone(),
        };
        let service = sysml_mutation_feasibility_service();
        let report = service.check(&context, &proposal);

        assert_eq!(report.status, FeasibilityStatus::Allowed, "{report:#?}");
        let plan = report.normalized_plan.as_ref().unwrap();
        assert!(matches!(
            &plan.normalized_operations[0],
            SemanticMutation::AddElement { kind, name, .. }
                if kind.metaclass == "StateUsage" && name == "armed"
        ));

        let application = service.apply_checked_plan(&context, plan).unwrap();
        let source = application.edited_files.get("controller.sysml").unwrap();
        assert!(source.contains("state armed;"));
        assert!(!source.contains("substate"));
    }

    #[test]
    fn sysml_legacy_state_usage_normalizes_to_semantic_add_element() {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "controller.sysml".to_string(),
            r#"
package Control {
    state def Controller {
        state operational;
    }
}
"#
            .to_string(),
        )]))
        .expect("project parses");
        let context = MutationContext::from_project(project);
        let proposal = MutationProposal {
            intent: "Legacy state usage proposal".to_string(),
            operations: vec![SemanticMutation::AddUsage {
                container: ElementRef::new("Control.Controller.operational"),
                keyword: "state".to_string(),
                name: "armed".to_string(),
                ty: None,
                specializes: Vec::new(),
            }],
            evidence: Vec::new(),
            rationale: None,
            workspace_revision: context.workspace_revision.clone(),
        };

        let report = sysml_mutation_feasibility_service().check(&context, &proposal);

        assert_eq!(report.status, FeasibilityStatus::Allowed, "{report:#?}");
        assert!(matches!(
            &report.normalized_plan.as_ref().unwrap().normalized_operations[0],
            SemanticMutation::AddElement { kind, name, .. }
                if kind.metaclass == "StateUsage" && name == "armed"
        ));
    }

    #[test]
    fn sysml_feasibility_normalizes_definition_suffix_for_writeback() {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "vehicle.sysml".to_string(),
            r#"
package HybridVehicle {
}
"#
            .to_string(),
        )]))
        .expect("project parses");
        let context = MutationContext::from_project(project);
        let proposal = MutationProposal {
            intent: "Add vehicle definition with SysML surface spelling".to_string(),
            operations: vec![SemanticMutation::AddDefinition {
                container: ElementRef::new("HybridVehicle"),
                keyword: "part def".to_string(),
                name: "Vehicle".to_string(),
                specializes: Vec::new(),
            }],
            evidence: Vec::new(),
            rationale: None,
            workspace_revision: context.workspace_revision.clone(),
        };
        let service = sysml_mutation_feasibility_service();
        let report = service.check(&context, &proposal);

        assert_eq!(report.status, FeasibilityStatus::Allowed, "{report:#?}");

        let application = service
            .apply_checked_plan(&context, report.normalized_plan.as_ref().unwrap())
            .unwrap();
        let source = application.edited_files.get("vehicle.sysml").unwrap();

        assert!(source.contains("part def Vehicle;"));
        assert!(!source.contains("part def def Vehicle;"));
    }

    /// DA-1 round-trip fixture: commented, hand-formatted source. Every
    /// fidelity test below asserts that an apply touches only the edited
    /// declaration and that every comment survives.
    const FIDELITY_FIXTURE: &str = "\
// Vehicle systems model - hand-maintained fixture.
// The odd formatting below is deliberate; write-back must not touch it.
package  Vehicle {

    /* Engine block: kept deliberately terse. */
    part def Engine;

    part def Chassis {
        attribute mass;
    }


    // The flagship configuration.
    part flagship : Chassis {
        // Primary drive.
        part engine : Engine;
    }

    requirement def MassLimit;

    /* Verified by the mass rollup. */
    requirement massLimit : MassLimit;

    // Wheels arrive in rev B.
    part def Wheel;
}
";

    const FIDELITY_FIXTURE_COMMENTS: &[&str] = &[
        "// Vehicle systems model - hand-maintained fixture.",
        "// The odd formatting below is deliberate; write-back must not touch it.",
        "/* Engine block: kept deliberately terse. */",
        "// The flagship configuration.",
        "// Primary drive.",
        "/* Verified by the mass rollup. */",
        "// Wheels arrive in rev B.",
    ];

    fn fidelity_context() -> MutationContext {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "vehicle.sysml".to_string(),
            FIDELITY_FIXTURE.to_string(),
        )]))
        .expect("fidelity fixture parses");
        MutationContext::from_project(project)
    }

    fn apply_fidelity_operation(operation: SemanticMutation) -> MutationApplicationResult {
        apply_fidelity_operations(vec![operation])
    }

    fn apply_fidelity_operations(operations: Vec<SemanticMutation>) -> MutationApplicationResult {
        let context = fidelity_context();
        let proposal = MutationProposal {
            intent: "Fidelity fixture edit".to_string(),
            operations,
            evidence: Vec::new(),
            rationale: None,
            workspace_revision: context.workspace_revision.clone(),
        };
        let service = sysml_mutation_feasibility_service();
        let report = service.check(&context, &proposal);
        assert!(
            matches!(
                report.status,
                FeasibilityStatus::Allowed | FeasibilityStatus::AllowedWithWarnings
            ),
            "{report:#?}"
        );
        service
            .apply_checked_plan(&context, report.normalized_plan.as_ref().expect("plan"))
            .expect("apply succeeds")
    }

    fn assert_fidelity_comments_survive(text: &str) {
        for comment in FIDELITY_FIXTURE_COMMENTS {
            assert!(
                text.contains(comment),
                "comment `{comment}` was lost:\n{text}"
            );
        }
    }

    /// Asserts everything before `start_marker` and from `end_marker` on is
    /// byte-identical to the fixture, i.e. only the declaration between the
    /// markers was rewritten.
    fn assert_bytes_identical_outside_edit(text: &str, start_marker: &str, end_marker: &str) {
        let prefix_end = FIDELITY_FIXTURE.find(start_marker).expect("start marker");
        let prefix = &FIDELITY_FIXTURE[..prefix_end];
        let suffix_start = FIDELITY_FIXTURE.find(end_marker).expect("end marker");
        let suffix = &FIDELITY_FIXTURE[suffix_start..];
        assert!(
            text.starts_with(prefix),
            "text before the edited declaration changed:\n{text}"
        );
        assert!(
            text.ends_with(suffix),
            "text after the edited declaration changed:\n{text}"
        );
    }

    #[test]
    fn sysml_add_usage_apply_patches_only_the_edited_declaration() {
        let application = apply_fidelity_operation(SemanticMutation::AddUsage {
            container: ElementRef::new("Vehicle.flagship"),
            keyword: "part".to_string(),
            name: "backupEngine".to_string(),
            ty: Some(ElementRef::new("Vehicle.Engine")),
            specializes: Vec::new(),
        });

        assert_eq!(
            application.write_back_mode,
            Some(WriteBackMode::LocalizedPatch),
            "{application:#?}"
        );
        let text = application.edited_files.get("vehicle.sysml").expect("edit");
        assert!(text.contains("backupEngine"));
        assert_fidelity_comments_survive(text);
        assert_bytes_identical_outside_edit(
            text,
            "    part flagship",
            "    requirement def MassLimit;",
        );
    }

    #[test]
    fn sysml_set_attribute_apply_patches_only_the_edited_declaration() {
        let application = apply_fidelity_operation(SemanticMutation::SetAttribute {
            element: ElementRef::new("Vehicle.massLimit"),
            attribute: "text".to_string(),
            value: json!("Total vehicle mass stays within the limit."),
        });

        assert_eq!(
            application.write_back_mode,
            Some(WriteBackMode::LocalizedPatch),
            "{application:#?}"
        );
        let text = application.edited_files.get("vehicle.sysml").expect("edit");
        assert!(text.contains("Total vehicle mass stays within the limit."));
        assert_fidelity_comments_survive(text);
        assert_bytes_identical_outside_edit(
            text,
            "    requirement massLimit",
            "    // Wheels arrive in rev B.",
        );
    }

    /// Two consecutive text edits on the same requirement must BOTH patch
    /// localized. The first splice introduces the doc-before form (`doc /*
    /// ... */` above the declaration keyword); on the fresh parse the
    /// recorded span must cover those doc lines, because the canonical
    /// printer re-renders them with the declaration — a keyword-anchored
    /// splice would duplicate the doc and force a whole-file rewrite.
    #[test]
    fn sysml_set_attribute_text_twice_patches_localized_both_times() {
        let first = apply_fidelity_operation(SemanticMutation::SetAttribute {
            element: ElementRef::new("Vehicle.massLimit"),
            attribute: "text".to_string(),
            value: json!("Total vehicle mass stays within the limit."),
        });
        assert_eq!(
            first.write_back_mode,
            Some(WriteBackMode::LocalizedPatch),
            "{first:#?}"
        );
        let first_text = first
            .edited_files
            .get("vehicle.sysml")
            .expect("first edit")
            .clone();

        // Fresh parse of the patched text, exactly as a live session reloads
        // between applies.
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "vehicle.sysml".to_string(),
            first_text.clone(),
        )]))
        .expect("patched fixture reparses");
        let context = MutationContext::from_project(project);
        let proposal = MutationProposal {
            intent: "Second text edit".to_string(),
            operations: vec![SemanticMutation::SetAttribute {
                element: ElementRef::new("Vehicle.massLimit"),
                attribute: "text".to_string(),
                value: json!("Mass stays within the revised limit."),
            }],
            evidence: Vec::new(),
            rationale: None,
            workspace_revision: context.workspace_revision.clone(),
        };
        let service = sysml_mutation_feasibility_service();
        let report = service.check(&context, &proposal);
        assert!(
            matches!(
                report.status,
                FeasibilityStatus::Allowed | FeasibilityStatus::AllowedWithWarnings
            ),
            "{report:#?}"
        );
        let second = service
            .apply_checked_plan(&context, report.normalized_plan.as_ref().expect("plan"))
            .expect("second apply succeeds");

        assert_eq!(
            second.write_back_mode,
            Some(WriteBackMode::LocalizedPatch),
            "{second:#?}"
        );
        let text = second.edited_files.get("vehicle.sysml").expect("edit");
        assert_eq!(text.matches("doc /*").count(), 1, "{text}");
        assert!(text.contains("Mass stays within the revised limit."), "{text}");
        assert!(
            !text.contains("Total vehicle mass stays within the limit."),
            "the first doc must be replaced, not accumulated:\n{text}"
        );
        assert_fidelity_comments_survive(text);
        // Everything outside the doc+declaration region is byte-identical to
        // the first apply's output.
        let prefix_end = first_text.find("doc /*").expect("doc marker");
        let suffix_start = first_text
            .find("    // Wheels arrive in rev B.")
            .expect("suffix marker");
        assert!(text.starts_with(&first_text[..prefix_end]), "{text}");
        assert!(text.ends_with(&first_text[suffix_start..]), "{text}");
    }

    #[test]
    fn sysml_rename_apply_patches_only_the_edited_declaration() {
        let application = apply_fidelity_operation(SemanticMutation::RenameDeclaration {
            element: ElementRef::new("Vehicle.Wheel"),
            new_name: "RoadWheel".to_string(),
        });

        assert_eq!(
            application.write_back_mode,
            Some(WriteBackMode::LocalizedPatch),
            "{application:#?}"
        );
        let text = application.edited_files.get("vehicle.sysml").expect("edit");
        assert!(text.contains("part def RoadWheel;"));
        assert!(!text.contains("part def Wheel;"));
        assert_fidelity_comments_survive(text);
        assert_bytes_identical_outside_edit(text, "    part def Wheel;", "\n}\n");
    }

    #[test]
    fn sysml_add_relationship_apply_patches_only_the_edited_declaration() {
        let application = apply_fidelity_operation(SemanticMutation::AddRelationship {
            kind: "satisfy".to_string(),
            source: ElementRef::new("Vehicle.flagship"),
            target: ElementRef::new("Vehicle.massLimit"),
        });

        assert_eq!(
            application.write_back_mode,
            Some(WriteBackMode::LocalizedPatch),
            "{application:#?}"
        );
        let text = application.edited_files.get("vehicle.sysml").expect("edit");
        assert!(text.contains("satisfy"));
        assert_fidelity_comments_survive(text);
        assert_bytes_identical_outside_edit(
            text,
            "    part flagship",
            "    requirement def MassLimit;",
        );
    }

    #[test]
    fn sysml_add_package_into_new_file_falls_back_to_canonical_rewrite() {
        let application = apply_fidelity_operation(SemanticMutation::AddPackage {
            target_file: "extensions.sysml".to_string(),
            name: "VehicleExtensions".to_string(),
        });

        assert_eq!(
            application.write_back_mode,
            Some(WriteBackMode::CanonicalRewrite),
            "{application:#?}"
        );
        let text = application
            .edited_files
            .get("extensions.sysml")
            .expect("new file");
        assert!(text.contains("package VehicleExtensions"));
        assert!(
            !application.edited_files.contains_key("vehicle.sysml"),
            "the fixture file must not be rewritten by an unrelated add"
        );
    }

    /// T6 keystone: a localized ReplaceNode re-renders the edited declaration
    /// canonically *inside* its span — interior member comments only survive
    /// because trivia is attached to the authoring model and re-rendered.
    /// The declaration's own leading comment stays in the untouched prefix
    /// and must not be duplicated by the splice.
    #[test]
    fn sysml_rename_of_container_keeps_interior_member_comments() {
        let application = apply_fidelity_operation(SemanticMutation::RenameDeclaration {
            element: ElementRef::new("Vehicle.flagship"),
            new_name: "flagshipConfig".to_string(),
        });

        assert_eq!(
            application.write_back_mode,
            Some(WriteBackMode::LocalizedPatch),
            "{application:#?}"
        );
        let text = application.edited_files.get("vehicle.sysml").expect("edit");
        // The edited declaration is re-rendered canonically inside its span
        // (`name: Type`, canonical spacing).
        assert!(text.contains("part flagshipConfig: Chassis"), "{text}");
        assert_fidelity_comments_survive(text);
        assert_eq!(
            text.matches("// Primary drive.").count(),
            1,
            "the interior comment must survive the canonical re-render exactly once:\n{text}"
        );
        assert_eq!(
            text.matches("// The flagship configuration.").count(),
            1,
            "the leading comment must not be duplicated by the splice:\n{text}"
        );
        assert_bytes_identical_outside_edit(
            text,
            "    part flagship",
            "    requirement def MassLimit;",
        );
    }

    /// T6 canonical-printer wiring: a plan that mixes a new-file AddPackage
    /// (FullFile — never localizable) with an edit to the fixture file forces
    /// the whole write-back onto the canonical path; every preserved comment
    /// must survive the file-wide re-render.
    #[test]
    fn sysml_forced_canonical_apply_keeps_leading_comments_file_wide() {
        let application = apply_fidelity_operations(vec![
            SemanticMutation::AddPackage {
                target_file: "extensions.sysml".to_string(),
                name: "VehicleExtensions".to_string(),
            },
            SemanticMutation::AddUsage {
                container: ElementRef::new("Vehicle.flagship"),
                keyword: "part".to_string(),
                name: "backupEngine".to_string(),
                ty: Some(ElementRef::new("Vehicle.Engine")),
                specializes: Vec::new(),
            },
        ]);

        assert_eq!(
            application.write_back_mode,
            Some(WriteBackMode::CanonicalRewrite),
            "{application:#?}"
        );
        let text = application
            .edited_files
            .get("vehicle.sysml")
            .expect("fixture rewrite");
        assert!(text.contains("backupEngine"));
        assert_fidelity_comments_survive(text);
        assert!(
            application
                .edited_files
                .get("extensions.sysml")
                .is_some_and(|new_file| new_file.contains("package VehicleExtensions")),
            "{application:#?}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn regenerated_rulepack_metadata_for_test(
        bundle: &crate::metamodel::ReleaseBundleResource,
    ) -> BTreeMap<String, Value> {
        let mut metadata = BTreeMap::from([
            (
                "description".to_string(),
                Value::String(
                    "Generated SysML metamodel adapter facts and semantic legality diagnostics"
                        .to_string(),
                ),
            ),
            (
                "profileId".to_string(),
                Value::String(bundle.profile_id.clone()),
            ),
            (
                "selector".to_string(),
                Value::String(bundle.selector.clone()),
            ),
            ("release".to_string(), json!(bundle.release)),
            (
                "stdlibPath".to_string(),
                Value::String(relative_path_string_for_test(
                    &bundle.root,
                    &bundle.stdlib_path,
                )),
            ),
        ]);
        if let Some(element_count) = bundle_element_count_for_test(&bundle.stdlib_path) {
            metadata.insert("elementCount".to_string(), element_count);
        }
        metadata
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn bundle_element_count_for_test(path: &std::path::Path) -> Option<Value> {
        let stdlib = KirDocument::from_path(path).ok()?;
        let adapter =
            crate::sysml_metamodel_adapter_from_graph(&Graph::from_document(stdlib).ok()?);
        adapter.metadata.get("elementCount").cloned()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn merge_rulepacks_for_test(
        adapter: RulePack,
        legality: RulePack,
        metadata: BTreeMap<String, Value>,
    ) -> RulePack {
        let mut facts = BTreeSet::new();
        facts.extend(adapter.facts);
        facts.extend(legality.facts);

        let mut rules = adapter.rules;
        rules.extend(legality.rules);

        RulePack {
            id: "sysml.semantic_legality".to_string(),
            version: legality.version,
            metadata,
            facts: facts.into_iter().collect(),
            rules,
            diagnostics: legality.diagnostics,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn relative_path_string_for_test(root: &std::path::Path, path: &std::path::Path) -> String {
        let stable_path = path.strip_prefix(root).unwrap_or(path);
        stable_path.to_string_lossy().replace('\\', "/")
    }
}
