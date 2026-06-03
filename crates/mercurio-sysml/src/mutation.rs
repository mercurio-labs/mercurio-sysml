use mercurio_core::{
    AuthoringProject, CoreMutationFeasibilityService, ElementRef,
    SemanticMutationCapabilityContext, SemanticReasoningContext, WorkspaceRevision,
    enrich_semantic_reasoning_context_with_child_affordances_for_capability,
    semantic_reasoning_context_from_authoring_project_with_oracle,
};

use crate::semantic_profile::{
    SYSML_DEFINITION_KEYWORDS, SYSML_RELATIONSHIP_KINDS, SYSML_USAGE_KEYWORDS,
    SysmlSemanticCapabilityOracle,
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

pub fn sysml_mutation_feasibility_service() -> SysmlMutationFeasibilityService {
    CoreMutationFeasibilityService::with_oracle(SysmlSemanticCapabilityOracle)
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
    let capability_context = sysml_semantic_mutation_capability_context();
    enrich_semantic_reasoning_context_with_child_affordances_for_capability(
        context,
        max_affordances,
        &capability_context,
    );
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use mercurio_core::{
        ElementRef, FeasibilityStatus, MutationContext, MutationFeasibilityService,
        MutationProposal, SemanticMutation, WorkspaceRevision,
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
        assert!(
            context
                .guidance
                .iter()
                .any(|item| item.contains("Never use keyword `block`"))
        );
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
            affordance.operation == "AddDefinition" && affordance.child_kind == "part"
        }));
        assert!(context.affordances.iter().any(|affordance| {
            affordance.operation == "AddUsage" && affordance.child_kind == "satisfy"
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
