//! KerML language facade.
//!
//! This crate is the public KerML-facing boundary while the parser/compiler
//! implementation is still hosted in `mercurio-foundation`. Keep this surface narrow:
//! parsing KerML, compiling KerML to KIR, and loading the KerML/Kernel baseline.

pub mod compiler;
pub mod parser;

pub use compiler::{
    BaselineLibrary, KermlError, KermlLanguageModule, compile_kerml_module,
    compile_kerml_module_with_context, compile_kerml_module_with_resolver_context,
    compile_kerml_text, compile_kerml_text_with_context, compile_kerml_text_with_empty_context,
    compile_text, compile_text_with_context, default_kernel_library_path, load_kerml_document,
    load_kerml_document_with_stdlib, load_kernel_baseline,
};
pub use mercurio_kir::{KirDocument, KirError};
pub use mercurio_language_contracts::SemanticConcept;
pub use mercurio_language_contracts::ast::{ParsedModule, QualifiedName, SourceSpan};
pub use mercurio_language_contracts::diagnostics::Diagnostic;
pub use mercurio_language_contracts::service::{CompileContext, LanguageService};
pub use mercurio_language_frontend::SourceLanguage;
pub use parser::{parse, parse_kerml};

impl LanguageService for KermlLanguageModule {
    fn language_id(&self) -> &str {
        "kerml"
    }

    fn extensions(&self) -> &[&str] {
        &["kerml"]
    }

    fn compile(
        &self,
        source: &str,
        context: CompileContext<'_>,
    ) -> mercurio_language_contracts::SemanticCompileReport<KirDocument> {
        match compile_text(source, context.source_name, context.library_context) {
            Ok(document) => mercurio_language_contracts::SemanticCompileReport {
                status: mercurio_language_contracts::SemanticCompileStatus::Ok,
                diagnostics: Vec::new(),
                document: Some(document),
            },
            Err(diagnostic) => mercurio_language_contracts::SemanticCompileReport {
                status: mercurio_language_contracts::SemanticCompileStatus::Failed,
                diagnostics: vec![diagnostic],
                document: None,
            },
        }
    }
}

#[cfg(any())]
pub use mercurio_core::language::kerml::parser::{
    KermlError, compile_kerml_module, compile_kerml_module_with_context, compile_kerml_text,
    compile_kerml_text_with_context, compile_kerml_text_with_empty_context, compile_text,
    compile_text_with_context, load_kerml_document, load_kerml_document_with_stdlib,
};

#[cfg(test)]
mod tests {
    use super::*;
    use mercurio_language_contracts::LanguageRegistry;
    use std::path::Path;

    #[test]
    fn facade_parses_minimal_kerml() {
        let module = parse("package Demo { classifier Vehicle; }").unwrap();

        assert!(module.package.is_some());
    }

    #[test]
    fn language_service_compiles_registered_kerml() {
        let mut registry = LanguageRegistry::new();
        registry.register(KermlLanguageModule);
        let library_context = KirDocument {
            metadata: Default::default(),
            elements: Vec::new(),
        };

        let report = registry.compile_path(
            Path::new("demo.kerml"),
            "package Demo { classifier Vehicle; }",
            &library_context,
        );

        assert_eq!(
            report.status,
            mercurio_language_contracts::SemanticCompileStatus::Ok
        );
        assert!(report.document.is_some());
    }
}
