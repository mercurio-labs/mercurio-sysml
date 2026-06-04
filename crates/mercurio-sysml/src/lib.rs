//! SysML language facade.
//!
//! This crate is the public SysML language implementation boundary: parsing,
//! recovery/reporting, compilation to KIR, and the SysML baseline library.

pub mod assessment;
pub mod authoring;
pub mod behavior;
pub mod constraints;
pub mod metamodel;
pub mod mutation;
pub mod parser;
pub mod semantic_profile;
pub mod session;

pub use assessment::sysml_parsed_module_assessment_facts;
pub use authoring::load_authoring_project_from_sysml;
pub use behavior::{
    CriticalSimulationEvent, HybridSimulationReport, HybridSimulationScenario,
    HybridSimulationStatus, HybridSimulationTraceEntry, SimulationError, SimulationSubject,
    StateMachineExecutionReport, StateMachineExecutionStatus, StateMachineModel,
    StateMachineScenario, StateMachineScenarioEvent, StateMachineTraceStep,
    StateMachineValidationFinding, StateMachineValidationSeverity, StateNode,
    StateTransitionTriggerKind, SysmlDynamicBehaviorCapability, TransitionNode,
    project_state_machines, project_state_machines_from_graph, register_sysml_behavior_capability,
    run_hybrid_simulation, run_hybrid_simulation_with_overlay,
};
pub use constraints::{
    ConstraintDiagnosticDto, ConstraintError, ConstraintExplanationDto, ConstraintGraphEdgeDto,
    ConstraintGraphRequestDto, ConstraintGraphViewDto, ConstraintRecordDto,
    ConstraintSolveRequestDto, ConstraintSolveResultDto, ConstraintStatusDto,
    ConstraintVariableDto, ConstraintVariableStatusDto, RequirementCheckDto, RequirementStatusDto,
    SysmlConstraintAnalysisCapability, execution_context_from_nested_values,
    register_sysml_constraint_analysis_capability, render_constraint_graph, solve_constraints,
};
pub use mercurio_kir::{KirDocument, KirError};
pub use mercurio_language_contracts::SemanticConcept;
pub use mercurio_language_contracts::ast::{
    ParsedModule, ParsedModule as SysmlModule, QualifiedName, SourceSpan,
};
pub use mercurio_language_contracts::diagnostics::Diagnostic;
pub use mercurio_language_contracts::reports::{ParseReport, SemanticCompileStatus};
pub use mercurio_language_contracts::service::{CompileContext, LanguageService};
pub use mercurio_language_frontend::SourceLanguage;
pub use metamodel::{
    LATEST_SYSML_METAMODEL_ID, LEGACY_SYSML_2_0_PILOT_057_ID, SYSML_2_0_METAMODEL_057_ID,
    SysmlEnvironment, SysmlEnvironmentError, SysmlMetamodel, SysmlMetamodelResource,
    SysmlMetamodelStatus, available_metamodels, latest_metamodel, metamodel_resource,
};
pub use mutation::{
    SYSML_MUTATION_GUIDANCE, SYSML_MUTATION_PROFILE_ID, SysmlMutationFeasibilityService,
    enrich_sysml_semantic_reasoning_context_with_child_affordances,
    sysml_mutation_feasibility_service, sysml_semantic_mutation_capability_context,
    sysml_semantic_reasoning_context_from_authoring_project,
};
pub use parser::{
    SemanticCompileReport, SysmlError, compile_sysml_module, compile_sysml_module_with_context,
    compile_sysml_module_with_context_report, compile_sysml_module_with_context_report_with_limit,
    compile_sysml_module_with_resolver_context,
    compile_sysml_module_with_resolver_context_report_with_limit, compile_sysml_text,
    compile_sysml_text_with_context, compile_sysml_text_with_context_report,
    default_sysml_delta_library_path, load_sysml_baseline, load_sysml_document,
    load_sysml_document_with_stdlib, parse_sysml, parse_sysml_recovering,
};
pub use semantic_profile::{
    SYSML_DEFINITION_KEYWORDS, SYSML_LANGUAGE_PROFILE_ID, SYSML_RELATIONSHIP_KINDS,
    SYSML_USAGE_KEYWORDS, SysmlSemanticCapabilityOracle, normalize_definition_keyword,
    sysml_definition_keyword_for_usage, sysml_definition_kind, sysml_is_container_kind,
    sysml_is_definition_keyword, sysml_is_satisfy_relationship, sysml_is_usage_keyword,
    sysml_language_profile, sysml_relationship_usage_keyword,
    sysml_trace_relationship_uses_owner_source, sysml_usage_kind,
};
pub use session::{
    SYSML_PART_USAGE_KIND, SYSML_REQUIREMENT_USAGE_KIND, SYSML_SATISFY_KEYWORD,
    SYSML_VERIFY_KEYWORD, SysmlModelForkExt,
};

#[derive(Debug)]
pub struct SysmlLanguageModule;

pub fn parse(input: &str) -> Result<ParsedModule, Diagnostic> {
    parse_sysml(input)
}

pub fn compile_text(
    input: &str,
    source_name: &str,
    library_context: &KirDocument,
) -> Result<KirDocument, Diagnostic> {
    compile_sysml_text(input, source_name, library_context)
}

pub fn compile_text_with_context(
    input: &str,
    source_name: &str,
    context_modules: &[ParsedModule],
    library_context: &KirDocument,
) -> Result<KirDocument, Diagnostic> {
    compile_sysml_text_with_context(input, source_name, context_modules, library_context)
}

pub fn default_sysml_library_path() -> std::path::PathBuf {
    default_sysml_delta_library_path()
}

pub fn legacy_monolithic_sysml_library_path() -> std::path::PathBuf {
    parser::default_sysml_library_path()
}

impl LanguageService for SysmlLanguageModule {
    fn language_id(&self) -> &str {
        "sysml"
    }

    fn extensions(&self) -> &[&str] {
        &["sysml"]
    }

    fn compile(
        &self,
        source: &str,
        context: CompileContext<'_>,
    ) -> mercurio_language_contracts::SemanticCompileReport<KirDocument> {
        compile_sysml_text_with_context_report(
            source,
            context.source_name,
            &[],
            context.library_context,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mercurio_language_contracts::LanguageRegistry;
    use std::path::Path;

    #[test]
    fn facade_parses_minimal_sysml() {
        let module = parse("package Demo { part def Vehicle; }").unwrap();

        assert!(module.package.is_some());
    }

    #[test]
    fn facade_compiles_minimal_sysml() {
        let stdlib = load_sysml_baseline().unwrap();
        let document = compile_sysml_text(
            "package Demo { part def Vehicle; part vehicle : Vehicle; }",
            "inline.sysml",
            &stdlib,
        )
        .unwrap();

        assert!(document.elements.iter().any(|element| {
            element.id == "part_definition.Demo.Vehicle"
                || element.id == "definition.Demo.Vehicle"
                || element.properties.get("declared_name")
                    == Some(&serde_json::Value::String("Vehicle".to_string()))
        }));
    }

    #[test]
    fn lists_latest_sysml_metamodel() {
        let metamodels = available_metamodels().unwrap();

        assert!(metamodels.iter().any(|metamodel| {
            metamodel.id == SYSML_2_0_METAMODEL_057_ID
                && metamodel.status == SysmlMetamodelStatus::Latest
        }));
        assert_eq!(latest_metamodel().unwrap().id, SYSML_2_0_METAMODEL_057_ID);
    }

    #[test]
    fn resolves_legacy_pilot_id_to_metamodel() {
        let metamodel = metamodel_resource(LEGACY_SYSML_2_0_PILOT_057_ID).unwrap();

        assert_eq!(metamodel.info.id, SYSML_2_0_METAMODEL_057_ID);
        assert!(
            metamodel
                .sysml_delta_path
                .ends_with("sysml-library.kir.json")
        );
    }

    #[test]
    fn environment_compiles_with_latest_metamodel() {
        let env = SysmlEnvironment::latest_metamodel().unwrap();

        let document = env
            .compile_text("package Demo { part def Vehicle; }", "inline.sysml")
            .unwrap();

        assert_eq!(env.metamodel().id, SYSML_2_0_METAMODEL_057_ID);
        assert!(!document.elements.is_empty());
    }

    #[test]
    fn language_service_compiles_registered_sysml() {
        let mut registry = LanguageRegistry::new();
        registry.register(SysmlLanguageModule);
        let stdlib = load_sysml_baseline().unwrap();

        let report = registry.compile_path(
            Path::new("demo.sysml"),
            "package Demo { part def Vehicle; }",
            &stdlib,
        );

        assert_eq!(report.status, SemanticCompileStatus::Ok);
        assert!(report.document.is_some());
    }

    #[test]
    fn body_doc_is_owned_by_containing_part_definition() {
        let source = "package Demo { part def A { doc /* doc from A */ } part def B; }";
        let module = parse_sysml(source).unwrap();
        let package = module.package.as_ref().unwrap();

        let definition_docs = |name: &str| {
            package
                .members
                .iter()
                .find_map(|member| {
                    let definition = member.as_definition_like()?;
                    (definition.name == name).then_some(definition.docs)
                })
                .unwrap()
        };

        assert_eq!(definition_docs("A"), vec!["doc from A".to_string()]);
        assert!(definition_docs("B").is_empty());

        let stdlib = load_sysml_baseline().unwrap();
        let document = compile_sysml_text(source, "inline.sysml", &stdlib).unwrap();
        let a = document
            .elements
            .iter()
            .find(|element| element.id == "type.Demo.A")
            .unwrap();
        let b = document
            .elements
            .iter()
            .find(|element| element.id == "type.Demo.B")
            .unwrap();
        let documentation = document
            .elements
            .iter()
            .find(|element| element.kind == "KerML::Root::Documentation")
            .unwrap();

        assert!(!a.properties.contains_key("doc"));
        assert!(!b.properties.contains_key("doc"));
        assert!(!a.properties.contains_key("ownedElement"));
        assert!(!a.properties.contains_key("documentation"));
        assert_eq!(
            documentation.properties["body"],
            serde_json::json!("doc from A")
        );
        assert_eq!(
            documentation.properties["owner"],
            serde_json::json!("type.Demo.A")
        );
        assert!(!documentation.properties.contains_key("documentedElement"));
        assert!(!documentation.properties.contains_key("annotatedElement"));
    }

    #[test]
    fn comment_usage_trailing_doc_is_body() {
        let stdlib = load_sysml_baseline().unwrap();
        let document = compile_sysml_text(
            "package Demo { comment cmt /* Named Comment */ comment about C /* About Definition */ part def C { comment /* Inner Comment */ comment about cmt locale \"en_US\" /* About Named */ comment about Demo /* About Package */ } }",
            "inline.sysml",
            &stdlib,
        )
        .unwrap();
        let comment = document
            .elements
            .iter()
            .find(|element| {
                element.properties.get("metatype")
                    == Some(&serde_json::json!("SysML::CommentUsage"))
                    && element.properties.get("declared_name") == Some(&serde_json::json!("cmt"))
            })
            .unwrap();

        assert_eq!(
            comment.properties["body"],
            serde_json::json!("Named Comment")
        );

        let inner_comment = document
            .elements
            .iter()
            .find(|element| {
                element.properties.get("metatype")
                    == Some(&serde_json::json!("SysML::CommentUsage"))
                    && element.properties.get("owner") == Some(&serde_json::json!("type.Demo.C"))
            })
            .unwrap();

        assert_eq!(
            inner_comment.properties["body"],
            serde_json::json!("Inner Comment")
        );

        let about_comment = document
            .elements
            .iter()
            .find(|element| {
                element.properties.get("metatype")
                    == Some(&serde_json::json!("SysML::CommentUsage"))
                    && element.properties.get("body") == Some(&serde_json::json!("About Named"))
            })
            .unwrap();

        assert_eq!(
            about_comment.properties["annotatedElement"],
            serde_json::json!(comment.id)
        );
        assert_eq!(
            about_comment.properties["locale"],
            serde_json::json!("en_US")
        );

        let definition = document
            .elements
            .iter()
            .find(|element| element.id == "type.Demo.C")
            .unwrap();
        let about_definition = document
            .elements
            .iter()
            .find(|element| {
                element.properties.get("metatype")
                    == Some(&serde_json::json!("SysML::CommentUsage"))
                    && element.properties.get("body")
                        == Some(&serde_json::json!("About Definition"))
            })
            .unwrap();
        assert_eq!(
            about_definition.properties["annotatedElement"],
            serde_json::json!(definition.id)
        );

        let about_package = document
            .elements
            .iter()
            .find(|element| {
                element.properties.get("metatype")
                    == Some(&serde_json::json!("SysML::CommentUsage"))
                    && element.properties.get("body") == Some(&serde_json::json!("About Package"))
            })
            .unwrap();
        assert_eq!(
            about_package.properties["annotatedElement"],
            serde_json::json!("pkg.Demo")
        );
    }

    #[test]
    fn baseline_is_kernel_plus_sysml_delta() {
        let baseline = load_sysml_baseline().unwrap();

        assert!(
            baseline
                .elements
                .iter()
                .any(|element| { element.id.contains("Kernel") || element.kind.contains("KerML") })
        );
        assert!(
            baseline
                .elements
                .iter()
                .any(|element| { element.id.contains("SysML") || element.kind.contains("SysML") })
        );
    }
}
