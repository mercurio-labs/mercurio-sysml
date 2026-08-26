//! SysML language facade.
//!
//! This crate is the public SysML language implementation boundary: parsing,
//! recovery/reporting, compilation to KIR, and the SysML baseline library.

pub mod abstract_syntax_json;
pub mod analysis;
pub mod assessment;
pub mod authoring;
pub mod behavior;
pub mod builder;
pub mod constraints;
pub mod dsl;
mod embedded_resources;
pub mod kerml;
pub mod language_frontend;
pub mod metamodel;
pub mod mutation;
pub mod parse_session;
pub mod parser;
pub mod requirements;
pub mod resources;
pub mod semantic_profile;
pub mod session;
pub mod simulation;

pub use crate::language_frontend::SourceLanguage;
pub use abstract_syntax_json::{
    SYSML_JSON_EXPORTER_VERSION, SYSML_JSON_IMPORTER_VERSION, SysmlJsonExportDiagnostic,
    SysmlJsonExportError, SysmlJsonExportOptions, SysmlJsonExportReport, SysmlJsonExportSeverity,
    SysmlJsonImportDiagnostic, SysmlJsonImportError, SysmlJsonImportOptions, SysmlJsonImportReport,
    SysmlJsonImportSeverity, export_sysml_abstract_syntax_json, export_sysml_abstract_syntax_value,
    import_sysml_abstract_syntax_json, import_sysml_abstract_syntax_value,
    import_sysml_api_elements,
};
pub use analysis::{
    AnalysisClockConfig, AnalysisDynamicBehaviorBinding, AnalysisDynamicBehaviorKind,
    AnalysisExecutionContext, AnalysisExecutionPlan, AnalysisExecutionStep,
    AnalysisExecutionStepKind, AnalysisExpectedArtifact, AnalysisReadinessDiagnostic,
    AnalysisReadinessSeverity, AnalysisReadinessStatus, AnalysisSpec, AnalysisSpecError,
    AnalysisTechnique, list_analysis_specs, project_analysis_spec,
};
pub use assessment::sysml_parsed_module_assessment_facts;
pub use authoring::load_authoring_project_from_sysml;
pub use behavior::{
    StateMachineExecutionReport, StateMachineExecutionStatus, StateMachineModel,
    StateMachineScenario, StateMachineScenarioEvent, StateMachineTraceStep,
    StateMachineValidationFinding, StateMachineValidationSeverity, StateNode,
    StateTransitionTriggerKind, SysmlDynamicBehaviorCapability, TransitionNode,
    project_state_machines, project_state_machines_from_graph, register_sysml_behavior_capability,
};
pub use builder::{
    ActionDefinition, AttributeDefinition, AttributeUsage, BuilderError, ConnectionDefinition,
    ConnectionUsage, InterfaceDefinition, IntoDeclaration, IntoRef, ItemDefinition, ItemUsage,
    ModelBuilder, PartDefinition, PartUsage, PortDefinition, PortUsage, RequirementDefinition,
    StateDefinition, StateUsage, StdlibRef,
};
pub use constraints::{
    ConstraintDiagnosticDto, ConstraintError, ConstraintExplanationDto, ConstraintGraphEdgeDto,
    ConstraintGraphRequestDto, ConstraintGraphViewDto, ConstraintRecordDto,
    ConstraintSolveRequestDto, ConstraintSolveResultDto, ConstraintStatusDto,
    ConstraintVariableDto, ConstraintVariableStatusDto, RequirementCheckDto, RequirementStatusDto,
    SysmlConstraintAnalysisCapability, execution_context_from_nested_values,
    register_sysml_constraint_analysis_capability, render_constraint_graph, solve_constraints,
};
pub use dsl::sysml_dsl_extension;
/// Language-neutral Mercurio APIs used by the SysML implementation.
pub use mercurio_foundation as foundation;
pub use mercurio_foundation::kir::{KirDocument, KirError};
pub use mercurio_foundation::language_contracts::Concept;
pub use mercurio_foundation::language_contracts::ast::{
    ParsedModule, ParsedModule as SysmlModule, QualifiedName, SourceSpan,
};
pub use mercurio_foundation::language_contracts::diagnostics::Diagnostic;
pub use mercurio_foundation::language_contracts::editor::{
    ParseSessionError, ParseSessionStatus, ParseSnapshot, TextEdit, TextRange,
};
pub use mercurio_foundation::language_contracts::reports::{ParseReport, SemanticCompileStatus};
pub use mercurio_foundation::language_contracts::service::{CompileContext, LanguageService};
use mercurio_foundation::language_contracts::workbench::{
    LanguageAnalysis, SourceDocument, analysis_from_compile_report,
};
pub use metamodel::{
    CANONICAL_SYSML_STDLIB_RELEASE, LATEST_SYSML_METAMODEL_ID, LEGACY_SYSML_2_0_PILOT_057_ID,
    ReleaseBundleConformance, ReleaseBundleDescriptor, ReleaseBundleMappings, ReleaseBundleProfile,
    ReleaseBundlePython, ReleaseBundleResource, ReleaseBundleStdlib, SYSML_2_0_METAMODEL_057_ID,
    SysmlEnvironment, SysmlEnvironmentError, SysmlMetamodel, SysmlMetamodelResource,
    SysmlMetamodelStatus, available_metamodels, available_release_bundles,
    canonical_sysml_stdlib_address, canonical_sysml_stdlib_digest,
    canonical_sysml_stdlib_runtime_source_bytes, canonical_sysml_stdlib_version, latest_metamodel,
    metamodel_resource, release_bundle,
};
pub use mutation::{
    SYSML_MUTATION_GUIDANCE, SYSML_MUTATION_PROFILE_ID, SysmlMutationFeasibilityService,
    SysmlSemanticLegalityService, SysmlSemanticNextActionsService,
    enrich_sysml_semantic_reasoning_context_with_child_affordances,
    sysml_mutation_feasibility_service, sysml_semantic_legality_base_rulepack,
    sysml_semantic_legality_rulepack, sysml_semantic_legality_rulepack_for_release,
    sysml_semantic_legality_rulepacks_for_release, sysml_semantic_legality_service,
    sysml_semantic_legality_service_for_release, sysml_semantic_mutation_capability_context,
    sysml_semantic_next_actions_service, sysml_semantic_next_actions_service_for_release,
    sysml_semantic_reasoning_context_from_authoring_project,
};
pub use parse_session::{SysmlParseSession, build_sysml_syntax_outline};
pub use parser::{
    SemanticCompileReport, StdlibLocator, SysmlError, compile_sysml_module,
    compile_sysml_module_with_context, compile_sysml_module_with_context_report,
    compile_sysml_module_with_context_report_with_limit,
    compile_sysml_module_with_resolver_context,
    compile_sysml_module_with_resolver_context_report_with_limit, compile_sysml_text,
    compile_sysml_text_with_context, compile_sysml_text_with_context_report,
    default_sysml_delta_library_path, load_sysml_baseline, load_sysml_baseline_from_locator,
    load_sysml_document, load_sysml_document_with_stdlib, parse_sysml, parse_sysml_recovering,
    resolve_default_stdlib_locator, shared_sysml_baseline, shared_sysml_baseline_from_locator,
};
pub use semantic_profile::{
    SYSML_LANGUAGE_PROFILE_ID, SysmlSemanticCapabilityOracle, normalize_definition_keyword,
    sysml_definition_element_kinds, sysml_definition_keyword_for_usage, sysml_definition_keywords,
    sysml_definition_kind, sysml_extension_relationship_keywords, sysml_field_specs,
    sysml_is_container_kind, sysml_is_definition_keyword, sysml_is_requirement_kind,
    sysml_is_satisfy_relationship, sysml_is_usage_keyword, sysml_language_profile,
    sysml_metamodel_adapter_from_graph, sysml_relationship_keywords,
    sysml_relationship_usage_keyword, sysml_trace_relationship_role,
    sysml_trace_relationship_uses_owner_source, sysml_trace_rulepack, sysml_usage_element_kinds,
    sysml_usage_keywords, sysml_usage_kind,
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
    ) -> mercurio_foundation::language_contracts::SemanticCompileReport<KirDocument> {
        compile_sysml_text_with_context_report(
            source,
            context.source_name,
            &[],
            context.library_context,
        )
    }

    fn analyze_workspace(
        &self,
        documents: &[SourceDocument],
        source_name: &str,
        library_context: &KirDocument,
    ) -> Option<LanguageAnalysis> {
        let target = documents
            .iter()
            .find(|document| document.source_name == source_name)?;
        let parsed = documents
            .iter()
            .filter_map(|document| {
                parse_sysml_recovering(&document.text)
                    .ok()
                    .map(|report| (document, report.module))
            })
            .collect::<Vec<_>>();
        let context_modules = parsed
            .iter()
            .map(|(_, module)| module.clone())
            .collect::<Vec<_>>();
        let mut resolution_context = library_context.clone();
        for (document, module) in &parsed {
            if let Ok(compiled) = compile_sysml_module_with_context(
                module,
                &document.source_name,
                &context_modules,
                library_context,
            ) {
                resolution_context.elements.extend(compiled.elements);
            }
        }
        let report = compile_sysml_text_with_context_report(
            &target.text,
            &target.source_name,
            &context_modules,
            library_context,
        );
        Some(analysis_from_compile_report(
            &target.text,
            &target.source_name,
            target.revision,
            &resolution_context,
            report,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mercurio_foundation::Runtime;
    use mercurio_foundation::kir::DiagnosticKind;
    use mercurio_foundation::language_contracts::LanguageRegistry;
    use std::path::Path;

    #[test]
    fn facade_parses_minimal_sysml() {
        let module = parse("package Demo { part def Vehicle; }").unwrap();

        assert!(module.package.is_some());
    }

    #[test]
    fn recovering_parse_diagnostics_are_syntax_kind() {
        let report = parse_sysml_recovering("package Demo { } }").unwrap();

        assert!(!report.diagnostics.is_empty());
        assert!(
            report
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.kind == DiagnosticKind::Syntax)
        );
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
    fn direct_compile_semantic_error_is_validation_kind() {
        let stdlib = load_sysml_baseline().unwrap();
        let diagnostic = compile_sysml_text(
            "package Demo { part vehicle: Missing; }",
            "inline.sysml",
            &stdlib,
        )
        .unwrap_err();

        assert_eq!(diagnostic.kind, DiagnosticKind::Validation);
        assert!(diagnostic.message.contains("unresolved type `Missing`"));
    }

    #[test]
    fn partial_compile_diagnostics_include_semantic_subjects() {
        let stdlib = load_sysml_baseline().unwrap();
        let report = compile_sysml_text_with_context_report(
            "package Demo { part def Good; part vehicle { part good: Good; part bad: Missing; } }",
            "inline.sysml",
            &[],
            &stdlib,
        );

        assert_eq!(report.status, SemanticCompileStatus::Partial);
        assert!(
            report
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.message.contains("unresolved type `Missing`"))
                .any(|diagnostic| diagnostic.kind == DiagnosticKind::Validation
                    && !diagnostic.subjects.is_empty())
        );
    }

    #[test]
    fn individual_part_compiles_to_individual_usage() {
        let stdlib = load_sysml_baseline().unwrap();
        let document = compile_sysml_text(
            "package Demo { part def Printer; individual part printer : Printer; }",
            "inline.sysml",
            &stdlib,
        )
        .unwrap();

        let printer = document
            .elements
            .iter()
            .find(|element| element.id == "individual.Demo.printer")
            .unwrap();

        assert_eq!(
            printer.properties["metatype"],
            serde_json::json!("SysML::IndividualUsage")
        );
        assert_eq!(
            printer.properties["type"],
            serde_json::json!("type.Demo.Printer")
        );
    }

    #[test]
    fn explicit_transition_usage_projects_state_machine_transition() {
        let stdlib = load_sysml_baseline().unwrap();
        let document = compile_sysml_text(
            "package Demo {
                part def Printer {
                    state lifecycle {
                        state idle;
                        state homing;
                        transition idle_go first idle accept start then homing;
                    }
                }
            }",
            "inline.sysml",
            &stdlib,
        )
        .unwrap();

        let runtime = Runtime::from_document(document).unwrap();
        let machines = project_state_machines(&runtime);
        let lifecycle = machines
            .iter()
            .find(|machine| machine.label == "lifecycle")
            .unwrap();

        assert!(lifecycle.transitions.iter().any(|transition| {
            transition.source == "state.Demo.Printer.lifecycle.idle"
                && transition.target == "state.Demo.Printer.lifecycle.homing"
                && transition.trigger.as_deref() == Some("start")
                && transition.trigger_kind == StateTransitionTriggerKind::Event
        }));
        assert!(
            lifecycle.states.iter().any(|state| {
                state.id == "state.Demo.Printer.lifecycle.idle" && state.is_initial
            })
        );
    }

    #[test]
    fn ai_style_transition_target_markers_compile_to_state_machine_transitions() {
        let stdlib = load_sysml_baseline().unwrap();
        let document = compile_sysml_text(
            "package Demo {
                part def Printer {
                    state lifecycle {
                        state idle;
                        state printing;
                        transition start first idle to printing;
                        transition stop from printing to idle;
                        transition resume idle -> printing;
                    }
                }
            }",
            "inline.sysml",
            &stdlib,
        )
        .unwrap();

        let runtime = Runtime::from_document(document).unwrap();
        let machines = project_state_machines(&runtime);
        let lifecycle = machines
            .iter()
            .find(|machine| machine.label == "lifecycle")
            .unwrap();

        assert!(lifecycle.transitions.iter().any(|transition| {
            transition.source == "state.Demo.Printer.lifecycle.idle"
                && transition.target == "state.Demo.Printer.lifecycle.printing"
        }));
        assert!(lifecycle.transitions.iter().any(|transition| {
            transition.source == "state.Demo.Printer.lifecycle.printing"
                && transition.target == "state.Demo.Printer.lifecycle.idle"
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
    fn release_bundle_resolves_latest_and_aliases() {
        let latest = release_bundle("latest").unwrap();
        let by_selector = release_bundle("2026-01").unwrap();
        let by_version_alias = release_bundle("0.57.0").unwrap();
        let by_alias = release_bundle("pilot-0.57.0").unwrap();
        let by_legacy = release_bundle(LEGACY_SYSML_2_0_PILOT_057_ID).unwrap();

        assert_eq!(latest.profile_id, SYSML_2_0_METAMODEL_057_ID);
        assert_eq!(latest.release.as_deref(), Some("2026-01"));
        assert_eq!(latest.selector, "2026-01");
        assert_eq!(latest.pilot_release_tag.as_deref(), Some("2026-01"));
        assert_eq!(
            latest.pilot_implementation_version.as_deref(),
            Some("0.57.0")
        );
        assert_eq!(by_selector.profile_id, latest.profile_id);
        assert_eq!(by_version_alias.profile_id, latest.profile_id);
        assert_eq!(by_alias.profile_id, latest.profile_id);
        assert_eq!(by_legacy.profile_id, latest.profile_id);
        assert!(latest.stdlib_path.ends_with("stdlib.full.kir.json"));
        assert!(latest.rulepack_path.ends_with("stdlib.rulepack.json"));
        assert!(
            latest
                .lowering_rules_path
                .ends_with("lowering_rules.seed.json")
        );
        assert!(
            latest
                .semantic_defaults_path
                .ends_with("semantic_defaults.seed.json")
        );
    }

    #[test]
    fn available_release_bundles_expose_user_facing_release_names() {
        let bundles = available_release_bundles().unwrap();

        assert!(bundles.iter().any(|bundle| {
            bundle.release.as_deref() == Some("2026-01")
                && bundle.selector == "2026-01"
                && bundle.profile_id == SYSML_2_0_METAMODEL_057_ID
                && bundle.aliases.iter().any(|alias| alias == "0.57.0")
        }));
    }

    #[test]
    fn stdlib_locator_resolves_release_selector() {
        let locator = StdlibLocator::for_release("2026-01").unwrap();

        assert!(matches!(locator, StdlibLocator::File { .. }));
        assert!(locator.as_uri().contains("stdlib.full.kir.json"));
    }

    #[cfg(feature = "embed-stdlib")]
    #[test]
    fn embedded_build_defaults_to_embedded_stdlib() {
        let locator = resolve_default_stdlib_locator();

        assert!(matches!(locator, StdlibLocator::Embedded { .. }));
        assert!(locator.as_uri().contains(SYSML_2_0_METAMODEL_057_ID));
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
    fn environment_compiles_with_release_selector() {
        let env = SysmlEnvironment::for_release("2026-01").unwrap();

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

    #[test]
    fn shared_baseline_reuses_cached_document() {
        let first = shared_sysml_baseline().unwrap();
        let second = shared_sysml_baseline().unwrap();

        assert!(std::sync::Arc::ptr_eq(&first, &second));
    }
}
