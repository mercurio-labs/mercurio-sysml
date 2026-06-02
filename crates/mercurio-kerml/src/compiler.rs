use std::path::{Path, PathBuf};

use mercurio_kir::{KirDocument, KirError};
use mercurio_language_contracts::diagnostics::Diagnostic;
use mercurio_language_contracts::{SourceLanguage, ast::SysmlModule};
use mercurio_language_frontend::lowering::mappings::{LanguageProfile, MappingBundle};
use mercurio_language_frontend::resolver::{
    ResolverContext, resolve_kerml_module_with_context, resolve_kerml_module_with_resolver_context,
};
use mercurio_language_frontend::transpile::transpile_module_with_source;

use crate::parser::parse_kerml;

#[derive(Debug)]
pub enum KermlError {
    Io(std::io::Error),
    Diagnostic(Diagnostic),
    Kir(KirError),
}

impl std::fmt::Display for KermlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "failed to read kerml file: {err}"),
            Self::Diagnostic(err) => write!(f, "{err}"),
            Self::Kir(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for KermlError {}

impl From<std::io::Error> for KermlError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<Diagnostic> for KermlError {
    fn from(value: Diagnostic) -> Self {
        Self::Diagnostic(value)
    }
}

impl From<KirError> for KermlError {
    fn from(value: KirError) -> Self {
        Self::Kir(value)
    }
}

#[derive(Debug, Clone)]
pub enum BaselineLibrary {
    Empty,
    Kernel,
    Custom(KirDocument),
}

impl BaselineLibrary {
    pub fn load(&self) -> Result<KirDocument, KirError> {
        match self {
            Self::Empty => Ok(KirDocument {
                metadata: Default::default(),
                elements: Vec::new(),
            }),
            Self::Kernel => KirDocument::from_path(&default_kernel_library_path()),
            Self::Custom(document) => Ok(document.clone()),
        }
    }
}

#[derive(Debug)]
pub struct KermlLanguageModule;

pub fn default_kernel_library_path() -> PathBuf {
    if let Ok(path) = std::env::var("MERCURIO_KERNEL_LIBRARY_PATH") {
        return PathBuf::from(path);
    }

    repo_path("resources/kernel/kerml-kernel.kir.json")
}

pub fn load_kernel_baseline() -> Result<KirDocument, KirError> {
    BaselineLibrary::Kernel.load()
}

pub fn load_kerml_document(path: &Path) -> Result<KirDocument, KermlError> {
    let library_context = load_kernel_baseline()?;
    load_kerml_document_with_stdlib(path, &library_context)
}

pub fn load_kerml_document_with_stdlib(
    path: &Path,
    stdlib: &KirDocument,
) -> Result<KirDocument, KermlError> {
    let input = std::fs::read_to_string(path)?;
    compile_kerml_text(&input, &path.display().to_string(), stdlib).map_err(Into::into)
}

pub fn compile_kerml_text(
    input: &str,
    source_name: &str,
    library_context: &KirDocument,
) -> Result<KirDocument, Diagnostic> {
    let module = parse_kerml(input)?;
    compile_kerml_module(&module, source_name, library_context)
}

pub fn compile_kerml_text_with_empty_context(
    input: &str,
    source_name: &str,
) -> Result<KirDocument, Diagnostic> {
    compile_kerml_text(
        input,
        source_name,
        &KirDocument {
            metadata: Default::default(),
            elements: Vec::new(),
        },
    )
}

pub fn compile_kerml_text_with_context(
    input: &str,
    source_name: &str,
    context_modules: &[SysmlModule],
    library_context: &KirDocument,
) -> Result<KirDocument, Diagnostic> {
    let module = parse_kerml(input)?;
    compile_kerml_module_with_context(&module, source_name, context_modules, library_context)
}

pub fn compile_kerml_module(
    module: &SysmlModule,
    source_name: &str,
    library_context: &KirDocument,
) -> Result<KirDocument, Diagnostic> {
    let profile = LanguageProfile::load(SourceLanguage::Kerml)?;
    let mappings = profile.mappings;
    let resolved = resolve_kerml_module_with_context(
        module,
        std::slice::from_ref(module),
        library_context,
        mappings,
    )?;
    transpile_module_with_source(&resolved, source_name, "kerml", mappings)
}

pub fn compile_kerml_module_with_context(
    module: &SysmlModule,
    source_name: &str,
    context_modules: &[SysmlModule],
    library_context: &KirDocument,
) -> Result<KirDocument, Diagnostic> {
    let profile = LanguageProfile::load(SourceLanguage::Kerml)?;
    let mappings = profile.mappings;
    let resolved =
        resolve_kerml_module_with_context(module, context_modules, library_context, mappings)?;
    transpile_module_with_source(&resolved, source_name, "kerml", mappings)
}

pub fn compile_kerml_module_with_resolver_context(
    module: &SysmlModule,
    source_name: &str,
    resolver_context: &ResolverContext,
    mappings: &MappingBundle,
) -> Result<KirDocument, Diagnostic> {
    let resolved = resolve_kerml_module_with_resolver_context(module, resolver_context, mappings)?;
    transpile_module_with_source(&resolved, source_name, "kerml", mappings)
}

pub fn compile_text(
    input: &str,
    source_name: &str,
    library_context: &KirDocument,
) -> Result<KirDocument, Diagnostic> {
    compile_kerml_text(input, source_name, library_context)
}

pub fn compile_text_with_context(
    input: &str,
    source_name: &str,
    context_modules: &[SysmlModule],
    library_context: &KirDocument,
) -> Result<KirDocument, Diagnostic> {
    compile_kerml_text_with_context(input, source_name, context_modules, library_context)
}

fn repo_path(relative: &str) -> PathBuf {
    repo_root().join(relative)
}

fn repo_root() -> PathBuf {
    let mut current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    loop {
        if current.join("Cargo.toml").is_file()
            && current
                .join("resources/kernel/kerml-kernel.kir.json")
                .is_file()
        {
            return current;
        }

        if !current.pop() {
            break;
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn compiles_minimal_classifier_and_feature_to_kir() {
        let document = compile_kerml_text_with_empty_context(
            "package Demo {
                classifier Vehicle {
                    feature engine : Engine;
                }
                classifier Engine;
            }",
            "inline.kerml",
        )
        .unwrap();

        assert!(document.elements.iter().any(|element| {
            element.id == "type.Demo.Vehicle" && element.kind == "KerML::Core::Type"
        }));
        assert!(document.elements.iter().any(|element| {
            element.id == "feature.Demo.Vehicle.engine"
                && element.kind == "KerML::Core::Feature"
                && element.properties.get("type")
                    == Some(&Value::String("type.Demo.Engine".to_string()))
        }));
    }
}
