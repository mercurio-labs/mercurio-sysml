use std::collections::{BTreeMap, BTreeSet};

use mercurio_core::{
    Atom, AuthoringProject, CoreMutationFeasibilityService, DiagnosticRule, ElementRef, Fact,
    RuleDiagnosticSeverity, RulePack, SemanticLegalityService, SemanticMutationCapabilityContext,
    SemanticNextActionsService, SemanticReasoningContext, SemanticRelationshipTargetRuleContext,
    SemanticUsageTypingRuleContext, Term, WorkspaceRevision,
    enrich_semantic_reasoning_context_with_next_action_affordances,
    semantic_reasoning_context_from_authoring_project_with_oracle,
};

use crate::SysmlEnvironmentError;
#[cfg(not(target_arch = "wasm32"))]
use crate::release_bundle;
use crate::semantic_profile::{
    SYSML_DEFINITION_KEYWORDS, SYSML_RELATIONSHIP_KINDS, SYSML_USAGE_KEYWORDS,
    SysmlSemanticCapabilityOracle, sysml_definition_keyword_for_usage,
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
        let _ = selector;
        Ok(sysml_semantic_legality_rulepack())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let bundle = release_bundle(selector)?;
        Ok(RulePack::from_path(&bundle.rulepack_path)?)
    }
}

pub fn sysml_semantic_legality_rulepack() -> RulePack {
    let mut facts = BTreeSet::new();
    facts.insert(Fact::new(
        "sysml_deprecated_v1_keyword",
        ["block".to_string()],
    ));
    for kind in SYSML_USAGE_KEYWORDS
        .iter()
        .copied()
        .filter(|kind| *kind != "requirement")
    {
        facts.insert(Fact::new(
            "sysml_non_requirement_relationship_target_kind",
            [kind.to_string()],
        ));
    }
    for kind in SYSML_DEFINITION_KEYWORDS
        .iter()
        .copied()
        .filter(|kind| *kind != "requirement")
    {
        let definition_kind = format!("{kind} def");
        facts.insert(Fact::new(
            "sysml_non_requirement_relationship_target_kind",
            [definition_kind.clone()],
        ));
    }
    for usage in SYSML_USAGE_KEYWORDS.iter().copied() {
        let Some(expected_definition) = sysml_definition_keyword_for_usage(usage) else {
            continue;
        };
        let expected_definition_kind = format!("{expected_definition} def");
        for definition in SYSML_DEFINITION_KEYWORDS.iter().copied() {
            let definition_kind = format!("{definition} def");
            if definition_kind == expected_definition_kind {
                continue;
            }
            facts.insert(Fact::new(
                "sysml_usage_typing_mismatch_kind",
                [
                    usage.to_string(),
                    expected_definition_kind.clone(),
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
    diagnostics.extend(SYSML_USAGE_KEYWORDS.iter().copied().filter_map(|usage| {
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
            "AddDefinition".to_string(),
            "AddUsage".to_string(),
            "AddRelationship".to_string(),
            "AddMetadataAnnotation".to_string(),
            "RemoveDeclaration".to_string(),
            "RemoveUsage".to_string(),
            "RemoveRelationship".to_string(),
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
        usage_typing_rules: sysml_usage_typing_rule_contexts(),
        relationship_target_rules: sysml_relationship_target_rule_contexts(),
        guidance: SYSML_MUTATION_GUIDANCE
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}

fn sysml_usage_typing_rule_contexts() -> Vec<SemanticUsageTypingRuleContext> {
    SYSML_USAGE_KEYWORDS
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
    use std::collections::BTreeMap;

    use mercurio_core::{
        ElementRef, FeasibilityStatus, MutationContext, MutationFeasibilityService,
        MutationProposal, SemanticLegalityDiagnosticSource, SemanticLegalityRequest,
        SemanticLegalityStatus, SemanticMutation, SemanticNextActionOperation,
        SemanticNextActionsRequest, WorkspaceRevision,
    };

    use super::*;
    use crate::load_authoring_project_from_sysml;

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
        assert!(!context.definition_keywords.contains(&"block".to_string()));
        assert!(context.relationship_kinds.contains(&"satisfy".to_string()));
        assert!(context.usage_typing_rules.iter().any(|rule| {
            rule.usage_keyword == "action" && rule.expected_definition_kind == "action def"
        }));
        assert!(context.relationship_target_rules.iter().any(|rule| {
            rule.relationship_kind == "satisfy" && rule.expected_target_kind == "requirement"
        }));
        assert!(
            context
                .guidance
                .iter()
                .any(|item| item.contains("Never use keyword `block`"))
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
            candidate_attributes: vec!["text".to_string()],
            facts: Vec::new(),
            max_actions: None,
        });

        assert!(report.actions.iter().any(|action| {
            action.operation
                == SemanticNextActionOperation::AddRelationship {
                    relationship_kind: "satisfy".to_string(),
                    target_kind: "requirement".to_string(),
                }
                && action.status == SemanticLegalityStatus::Allowed
        }));
        assert!(report.actions.iter().any(|action| {
            action.operation
                == SemanticNextActionOperation::AddRelationship {
                    relationship_kind: "satisfy".to_string(),
                    target_kind: "part".to_string(),
                }
                && action.status == SemanticLegalityStatus::Blocked
                && action.legality.diagnostics.iter().any(|diagnostic| {
                    diagnostic.source == SemanticLegalityDiagnosticSource::Rulepack
                        && diagnostic.code == "sysml.satisfy.target_requirement"
                })
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

        enrich_sysml_semantic_reasoning_context_with_child_affordances(&mut context, 64);

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
            affected_elements: vec![ElementRef::new("HybridVehicle.vehicle")],
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
            SemanticMutation::AddDefinition { keyword, name, .. }
                if keyword == "part" && name == "RegenerativeBrakingSystem"
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
            affected_elements: Vec::new(),
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
}
