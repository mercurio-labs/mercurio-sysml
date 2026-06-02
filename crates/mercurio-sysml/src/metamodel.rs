use std::path::{Path, PathBuf};

use mercurio_kerml::KermlLanguageModule;
use mercurio_kir::{KirDocument, KirError};
use mercurio_language_contracts::{LanguageRegistry, SemanticCompileStatus};
use serde::Deserialize;

use crate::SysmlLanguageModule;
use crate::parser;

pub const SYSML_2_0_METAMODEL_057_ID: &str = "sysml-2.0-metamodel-0.57.0";
pub const LEGACY_SYSML_2_0_PILOT_057_ID: &str = "sysml-2.0-pilot-0.57.0";
pub const LATEST_SYSML_METAMODEL_ID: &str = SYSML_2_0_METAMODEL_057_ID;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SysmlMetamodelStatus {
    Latest,
    Supported,
    Deprecated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysmlMetamodel {
    pub id: String,
    pub display_name: String,
    pub sysml_version: String,
    pub kerml_version: String,
    pub metamodel_version: String,
    pub status: SysmlMetamodelStatus,
    pub legacy_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SysmlMetamodelResource {
    pub info: SysmlMetamodel,
    pub root: PathBuf,
    pub profile_path: PathBuf,
    pub mappings_path: PathBuf,
    pub stdlib_path: PathBuf,
    pub sysml_delta_path: PathBuf,
    pub provenance_path: PathBuf,
}

pub struct SysmlEnvironment {
    metamodel: SysmlMetamodelResource,
    registry: LanguageRegistry,
    baseline: KirDocument,
}

#[derive(Debug)]
pub enum SysmlEnvironmentError {
    UnknownMetamodel(String),
    Json(String),
    Kir(KirError),
    Diagnostic(mercurio_language_contracts::diagnostics::Diagnostic),
}

impl std::fmt::Display for SysmlEnvironmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownMetamodel(id) => write!(f, "unknown SysML metamodel `{id}`"),
            Self::Json(message) => write!(f, "{message}"),
            Self::Kir(err) => write!(f, "{err}"),
            Self::Diagnostic(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SysmlEnvironmentError {}

impl From<KirError> for SysmlEnvironmentError {
    fn from(value: KirError) -> Self {
        Self::Kir(value)
    }
}

impl From<mercurio_language_contracts::diagnostics::Diagnostic> for SysmlEnvironmentError {
    fn from(value: mercurio_language_contracts::diagnostics::Diagnostic) -> Self {
        Self::Diagnostic(value)
    }
}

impl SysmlEnvironment {
    pub fn latest() -> Result<Self, SysmlEnvironmentError> {
        Self::for_metamodel(LATEST_SYSML_METAMODEL_ID)
    }

    pub fn latest_metamodel() -> Result<Self, SysmlEnvironmentError> {
        Self::latest()
    }

    pub fn for_metamodel(id: &str) -> Result<Self, SysmlEnvironmentError> {
        let metamodel = metamodel_resource(id)?;
        let mut registry = LanguageRegistry::new();
        registry.register(KermlLanguageModule);
        registry.register(SysmlLanguageModule);
        let baseline = load_baseline_for_metamodel(&metamodel)?;

        Ok(Self {
            metamodel,
            registry,
            baseline,
        })
    }

    pub fn available_metamodels() -> Result<Vec<SysmlMetamodel>, SysmlEnvironmentError> {
        available_metamodels()
    }

    pub fn latest_metamodel_info() -> Result<SysmlMetamodel, SysmlEnvironmentError> {
        latest_metamodel()
    }

    pub fn metamodel(&self) -> &SysmlMetamodel {
        &self.metamodel.info
    }

    pub fn metamodel_resource(&self) -> &SysmlMetamodelResource {
        &self.metamodel
    }

    pub fn registry(&self) -> &LanguageRegistry {
        &self.registry
    }

    pub fn baseline(&self) -> &KirDocument {
        &self.baseline
    }

    pub fn compile_text(
        &self,
        source: &str,
        source_name: &str,
    ) -> Result<KirDocument, SysmlEnvironmentError> {
        let report = self
            .registry
            .compile_path(Path::new(source_name), source, &self.baseline);
        if report.status != SemanticCompileStatus::Ok {
            let diagnostic = report.diagnostics.into_iter().next().unwrap_or_else(|| {
                mercurio_language_contracts::diagnostics::Diagnostic::new(
                    "SysML compile failed without diagnostics",
                    None,
                )
            });
            return Err(SysmlEnvironmentError::Diagnostic(diagnostic));
        }
        Ok(report
            .document
            .expect("successful compile returns a document"))
    }
}

pub fn available_metamodels() -> Result<Vec<SysmlMetamodel>, SysmlEnvironmentError> {
    Ok(vec![metamodel_descriptor()?])
}

pub fn latest_metamodel() -> Result<SysmlMetamodel, SysmlEnvironmentError> {
    let metamodels = available_metamodels()?;
    metamodels
        .into_iter()
        .find(|metamodel| metamodel.status == SysmlMetamodelStatus::Latest)
        .ok_or_else(|| SysmlEnvironmentError::UnknownMetamodel("latest".to_string()))
}

pub fn metamodel_resource(id: &str) -> Result<SysmlMetamodelResource, SysmlEnvironmentError> {
    let descriptor = metamodel_descriptor()?;
    if descriptor.id != id
        && !descriptor
            .legacy_ids
            .iter()
            .any(|legacy_id| legacy_id == id)
    {
        return Err(SysmlEnvironmentError::UnknownMetamodel(id.to_string()));
    }

    let raw = metamodel_descriptor_raw()?;
    let root = parser::repo_path(&format!("resources/metamodels/{}", descriptor.id));
    Ok(SysmlMetamodelResource {
        info: descriptor,
        profile_path: root.join(raw.profile_path),
        mappings_path: root.join(raw.mappings_path),
        stdlib_path: root.join(raw.stdlib_path),
        sysml_delta_path: root.join(raw.sysml_delta_path),
        provenance_path: root.join(raw.provenance_path),
        root,
    })
}

pub fn load_baseline_for_metamodel(
    metamodel: &SysmlMetamodelResource,
) -> Result<KirDocument, KirError> {
    let kernel = mercurio_kerml::load_kernel_baseline()?;
    let sysml_delta = KirDocument::from_path(&metamodel.sysml_delta_path)?;
    KirDocument::merge([kernel, sysml_delta])
}

#[derive(Debug, Deserialize)]
struct RawMetamodelDescriptor {
    id: String,
    display_name: String,
    sysml_version: String,
    kerml_version: String,
    metamodel_version: String,
    status: String,
    profile_path: String,
    mappings_path: String,
    stdlib_path: String,
    sysml_delta_path: String,
    provenance_path: String,
    #[serde(default)]
    legacy_ids: Vec<String>,
}

fn metamodel_descriptor() -> Result<SysmlMetamodel, SysmlEnvironmentError> {
    let raw = metamodel_descriptor_raw()?;
    Ok(SysmlMetamodel {
        id: raw.id,
        display_name: raw.display_name,
        sysml_version: raw.sysml_version,
        kerml_version: raw.kerml_version,
        metamodel_version: raw.metamodel_version,
        status: match raw.status.as_str() {
            "latest" => SysmlMetamodelStatus::Latest,
            "supported" => SysmlMetamodelStatus::Supported,
            "deprecated" => SysmlMetamodelStatus::Deprecated,
            other => {
                return Err(SysmlEnvironmentError::Json(format!(
                    "unknown SysML metamodel status `{other}`"
                )));
            }
        },
        legacy_ids: raw.legacy_ids,
    })
}

fn metamodel_descriptor_raw() -> Result<RawMetamodelDescriptor, SysmlEnvironmentError> {
    serde_json::from_str(include_str!(
        "../../../resources/metamodels/sysml-2.0-metamodel-0.57.0/metamodel.json"
    ))
    .map_err(|err| {
        SysmlEnvironmentError::Json(format!("failed to parse SysML metamodel descriptor: {err}"))
    })
}
