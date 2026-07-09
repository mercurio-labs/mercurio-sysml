use std::path::{Path, PathBuf};

use mercurio_core::DatalogError;
use mercurio_kerml::KermlLanguageModule;
use mercurio_kir::{KirDocument, KirError};
use mercurio_language_contracts::{LanguageRegistry, SemanticCompileStatus};
use serde::{Deserialize, Serialize};

use crate::SysmlLanguageModule;
use crate::parser;

pub const SYSML_2_0_METAMODEL_057_ID: &str = "sysml-2.0-metamodel-0.57.0";
pub const LEGACY_SYSML_2_0_PILOT_057_ID: &str = "sysml-2.0-pilot-0.57.0";
pub const SYSML_2_0_PILOT_2026_04_ID: &str = "sysml-2.0-pilot-2026-04";
pub const LATEST_SYSML_METAMODEL_ID: &str = SYSML_2_0_METAMODEL_057_ID;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SysmlMetamodelStatus {
    Latest,
    Supported,
    Deprecated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SysmlMetamodel {
    pub id: String,
    #[serde(default)]
    pub release: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
    pub display_name: String,
    pub sysml_version: String,
    pub kerml_version: String,
    pub metamodel_version: String,
    pub status: SysmlMetamodelStatus,
    #[serde(default)]
    pub legacy_ids: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub bundle: Option<ReleaseBundleDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseBundleDescriptor {
    #[serde(default)]
    pub profile: ReleaseBundleProfile,
    #[serde(default)]
    pub stdlib: ReleaseBundleStdlib,
    #[serde(default)]
    pub mappings: ReleaseBundleMappings,
    #[serde(default)]
    pub conformance: ReleaseBundleConformance,
    #[serde(default)]
    pub python: ReleaseBundlePython,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseBundleProfile {
    #[serde(default = "default_profile_path")]
    pub path: String,
}

impl Default for ReleaseBundleProfile {
    fn default() -> Self {
        Self {
            path: default_profile_path(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseBundleStdlib {
    #[serde(default = "default_stdlib_locator")]
    pub locator: String,
    #[serde(default = "default_rulepack_path")]
    pub rulepack: String,
}

impl Default for ReleaseBundleStdlib {
    fn default() -> Self {
        Self {
            locator: default_stdlib_locator(),
            rulepack: default_rulepack_path(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseBundleMappings {
    #[serde(default = "default_mappings_path")]
    pub path: String,
    #[serde(default = "default_metamodel_constructs_path")]
    pub metamodel_constructs: String,
    #[serde(default = "default_kir_emission_path")]
    pub kir_emission: String,
    #[serde(default = "default_lowering_rules_path")]
    pub lowering_rules: String,
    #[serde(default = "default_semantic_defaults_path")]
    pub semantic_defaults: String,
}

impl Default for ReleaseBundleMappings {
    fn default() -> Self {
        Self {
            path: default_mappings_path(),
            metamodel_constructs: default_metamodel_constructs_path(),
            kir_emission: default_kir_emission_path(),
            lowering_rules: default_lowering_rules_path(),
            semantic_defaults: default_semantic_defaults_path(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReleaseBundleConformance {
    #[serde(default)]
    pub accepted_differences: Option<String>,
    #[serde(default)]
    pub trace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReleaseBundlePython {
    #[serde(default)]
    pub wrapper_module: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReleaseBundleResource {
    pub release: Option<String>,
    pub selector: String,
    pub profile_id: String,
    pub status: SysmlMetamodelStatus,
    pub pilot_release_tag: Option<String>,
    pub pilot_implementation_version: Option<String>,
    pub sysml_version: String,
    pub aliases: Vec<String>,
    pub root: PathBuf,
    pub profile_path: PathBuf,
    pub mappings_path: PathBuf,
    pub metamodel_constructs_path: PathBuf,
    pub kir_emission_path: PathBuf,
    pub lowering_rules_path: PathBuf,
    pub semantic_defaults_path: PathBuf,
    pub stdlib_locator: String,
    pub stdlib_path: PathBuf,
    pub rulepack_path: PathBuf,
    pub accepted_differences_path: Option<PathBuf>,
    pub conformance_trace_path: Option<PathBuf>,
    pub python_wrapper_module: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SysmlMetamodelResource {
    pub info: SysmlMetamodel,
    pub root: PathBuf,
    pub profile_path: PathBuf,
    pub mappings_path: PathBuf,
    pub metamodel_constructs_path: PathBuf,
    pub kir_emission_path: PathBuf,
    pub lowering_rules_path: PathBuf,
    pub semantic_defaults_path: PathBuf,
    pub stdlib_path: PathBuf,
    pub sysml_delta_path: PathBuf,
    pub provenance_path: PathBuf,
    pub release_bundle: ReleaseBundleResource,
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
    Datalog(DatalogError),
    Diagnostic(mercurio_language_contracts::diagnostics::Diagnostic),
}

impl std::fmt::Display for SysmlEnvironmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownMetamodel(id) => write!(f, "unknown SysML metamodel `{id}`"),
            Self::Json(message) => write!(f, "{message}"),
            Self::Kir(err) => write!(f, "{err}"),
            Self::Datalog(err) => write!(f, "{err}"),
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

impl From<DatalogError> for SysmlEnvironmentError {
    fn from(value: DatalogError) -> Self {
        Self::Datalog(value)
    }
}

impl From<mercurio_language_contracts::diagnostics::Diagnostic> for SysmlEnvironmentError {
    fn from(value: mercurio_language_contracts::diagnostics::Diagnostic) -> Self {
        Self::Diagnostic(value)
    }
}

impl SysmlEnvironment {
    pub fn latest() -> Result<Self, SysmlEnvironmentError> {
        Self::for_release("latest")
    }

    pub fn latest_metamodel() -> Result<Self, SysmlEnvironmentError> {
        Self::latest()
    }

    pub fn for_release(selector: &str) -> Result<Self, SysmlEnvironmentError> {
        let bundle = release_bundle(selector)?;
        Self::for_metamodel(&bundle.profile_id)
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
    serde_json::from_str(crate::embedded_resources::METAMODEL_REGISTRY).map_err(|err| {
        SysmlEnvironmentError::Json(format!("failed to parse SysML metamodel registry: {err}"))
    })
}

pub fn latest_metamodel() -> Result<SysmlMetamodel, SysmlEnvironmentError> {
    let metamodels = available_metamodels()?;
    let latest = metamodels
        .into_iter()
        .filter(|metamodel| metamodel.status == SysmlMetamodelStatus::Latest)
        .collect::<Vec<_>>();
    match latest.as_slice() {
        [metamodel] => Ok(metamodel.clone()),
        [] => Err(SysmlEnvironmentError::UnknownMetamodel(
            "latest".to_string(),
        )),
        _ => Err(SysmlEnvironmentError::Json(
            "metamodel registry has more than one latest SysML metamodel".to_string(),
        )),
    }
}

pub fn metamodel_resource(id: &str) -> Result<SysmlMetamodelResource, SysmlEnvironmentError> {
    let descriptor = metamodel_descriptor(id)?;
    if !descriptor_matches_selector(&descriptor, id) {
        return Err(SysmlEnvironmentError::UnknownMetamodel(id.to_string()));
    }

    let raw = metamodel_descriptor_raw(&descriptor.id)?;
    let root = parser::repo_path(&format!("resources/metamodels/{}", descriptor.id));
    let release_bundle = release_bundle_from_descriptor(&descriptor, &raw)?;
    Ok(SysmlMetamodelResource {
        info: descriptor,
        profile_path: root.join(raw.profile_path),
        mappings_path: root.join(raw.mappings_path),
        metamodel_constructs_path: release_bundle.metamodel_constructs_path.clone(),
        kir_emission_path: release_bundle.kir_emission_path.clone(),
        lowering_rules_path: release_bundle.lowering_rules_path.clone(),
        semantic_defaults_path: release_bundle.semantic_defaults_path.clone(),
        stdlib_path: root.join(raw.stdlib_path),
        sysml_delta_path: root.join(raw.sysml_delta_path),
        provenance_path: root.join(raw.provenance_path),
        release_bundle,
        root,
    })
}

pub fn release_bundle(selector: &str) -> Result<ReleaseBundleResource, SysmlEnvironmentError> {
    let descriptor = release_bundle_descriptor(selector)?;
    let raw = metamodel_descriptor_raw(&descriptor.id)?;
    release_bundle_from_descriptor(&descriptor, &raw)
}

pub fn available_release_bundles() -> Result<Vec<ReleaseBundleResource>, SysmlEnvironmentError> {
    available_metamodels()?
        .into_iter()
        .filter(|descriptor| descriptor.bundle.is_some())
        .map(|descriptor| {
            let raw = metamodel_descriptor_raw(&descriptor.id)?;
            release_bundle_from_descriptor(&descriptor, &raw)
        })
        .collect()
}

pub fn load_baseline_for_metamodel(
    metamodel: &SysmlMetamodelResource,
) -> Result<KirDocument, KirError> {
    let kernel = mercurio_kerml::load_kernel_baseline()?;
    let sysml_delta = KirDocument::from_path_with_registered_fields(
        &metamodel.sysml_delta_path,
        crate::sysml_field_specs().iter().copied(),
    )?;
    KirDocument::merge_with_registered_fields(
        [kernel, sysml_delta],
        crate::sysml_field_specs().iter().copied(),
    )
}

#[cfg(feature = "embed-stdlib")]
pub(crate) fn embedded_bytes_for_metamodel(id: &str) -> Option<(&'static [u8], &'static [u8])> {
    match id {
        SYSML_2_0_METAMODEL_057_ID | LEGACY_SYSML_2_0_PILOT_057_ID => Some((
            crate::embedded_resources::EMBEDDED_KERNEL,
            crate::embedded_resources::EMBEDDED_SYSML_LIBRARY,
        )),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct RawMetamodelDescriptor {
    profile_path: String,
    mappings_path: String,
    stdlib_path: String,
    sysml_delta_path: String,
    provenance_path: String,
}

fn metamodel_descriptor(id: &str) -> Result<SysmlMetamodel, SysmlEnvironmentError> {
    available_metamodels()?
        .into_iter()
        .find(|metamodel| descriptor_matches_selector(metamodel, id))
        .ok_or_else(|| SysmlEnvironmentError::UnknownMetamodel(id.to_string()))
}

fn release_bundle_descriptor(selector: &str) -> Result<SysmlMetamodel, SysmlEnvironmentError> {
    if selector == "latest" {
        return latest_metamodel();
    }

    let matches = available_metamodels()?
        .into_iter()
        .filter(|metamodel| descriptor_matches_selector(metamodel, selector))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [metamodel] => Ok(metamodel.clone()),
        [] => Err(SysmlEnvironmentError::UnknownMetamodel(
            selector.to_string(),
        )),
        _ => Err(SysmlEnvironmentError::Json(format!(
            "release selector `{selector}` is ambiguous"
        ))),
    }
}

fn descriptor_matches_selector(metamodel: &SysmlMetamodel, selector: &str) -> bool {
    metamodel.id == selector
        || metamodel.release.as_deref() == Some(selector)
        || metamodel.selector.as_deref() == Some(selector)
        || metamodel
            .legacy_ids
            .iter()
            .any(|legacy_id| legacy_id == selector)
        || metamodel.aliases.iter().any(|alias| alias == selector)
}

fn release_bundle_from_descriptor(
    descriptor: &SysmlMetamodel,
    raw: &RawMetamodelDescriptor,
) -> Result<ReleaseBundleResource, SysmlEnvironmentError> {
    let root = parser::repo_path(&format!("resources/metamodels/{}", descriptor.id));
    let bundle = descriptor.bundle.clone().unwrap_or_default_bundle(raw);
    let stdlib_path = match bundle.stdlib.locator.strip_prefix("file:") {
        Some(relative) => root.join(relative),
        None => root.join(&bundle.stdlib.locator),
    };
    Ok(ReleaseBundleResource {
        release: descriptor.release.clone(),
        selector: descriptor
            .selector
            .clone()
            .unwrap_or_else(|| descriptor.id.clone()),
        profile_id: descriptor.id.clone(),
        status: descriptor.status.clone(),
        pilot_release_tag: descriptor.selector.clone(),
        pilot_implementation_version: Some(descriptor.metamodel_version.clone()),
        sysml_version: descriptor.sysml_version.clone(),
        aliases: descriptor
            .aliases
            .iter()
            .chain(descriptor.legacy_ids.iter())
            .cloned()
            .collect(),
        profile_path: root.join(&bundle.profile.path),
        mappings_path: root.join(&bundle.mappings.path),
        metamodel_constructs_path: root.join(&bundle.mappings.metamodel_constructs),
        kir_emission_path: root.join(&bundle.mappings.kir_emission),
        lowering_rules_path: root.join(&bundle.mappings.lowering_rules),
        semantic_defaults_path: root.join(&bundle.mappings.semantic_defaults),
        stdlib_locator: bundle.stdlib.locator,
        stdlib_path,
        rulepack_path: root.join(&bundle.stdlib.rulepack),
        accepted_differences_path: bundle
            .conformance
            .accepted_differences
            .map(|path| root.join(path)),
        conformance_trace_path: bundle.conformance.trace.map(|path| root.join(path)),
        python_wrapper_module: bundle.python.wrapper_module,
        root,
    })
}

trait ReleaseBundleDescriptorExt {
    fn unwrap_or_default_bundle(self, raw: &RawMetamodelDescriptor) -> ReleaseBundleDescriptor;
}

impl ReleaseBundleDescriptorExt for Option<ReleaseBundleDescriptor> {
    fn unwrap_or_default_bundle(self, raw: &RawMetamodelDescriptor) -> ReleaseBundleDescriptor {
        self.unwrap_or_else(|| ReleaseBundleDescriptor {
            profile: ReleaseBundleProfile {
                path: raw.profile_path.clone(),
            },
            stdlib: ReleaseBundleStdlib {
                locator: format!("file:{}", raw.stdlib_path),
                rulepack: default_rulepack_path(),
            },
            mappings: ReleaseBundleMappings {
                path: raw.mappings_path.clone(),
                ..ReleaseBundleMappings::default()
            },
            conformance: ReleaseBundleConformance::default(),
            python: ReleaseBundlePython::default(),
        })
    }
}

fn metamodel_descriptor_raw(id: &str) -> Result<RawMetamodelDescriptor, SysmlEnvironmentError> {
    let raw = match id {
        SYSML_2_0_METAMODEL_057_ID | LEGACY_SYSML_2_0_PILOT_057_ID => {
            include_str!("../../../resources/metamodels/sysml-2.0-metamodel-0.57.0/metamodel.json")
        }
        SYSML_2_0_PILOT_2026_04_ID => {
            include_str!("../../../resources/metamodels/sysml-2.0-pilot-2026-04/metamodel.json")
        }
        _ => return Err(SysmlEnvironmentError::UnknownMetamodel(id.to_string())),
    };
    serde_json::from_str(raw).map_err(|err| {
        SysmlEnvironmentError::Json(format!("failed to parse SysML metamodel descriptor: {err}"))
    })
}

fn default_profile_path() -> String {
    "profile.json".to_string()
}

fn default_stdlib_locator() -> String {
    "file:stdlib/stdlib.full.kir.json".to_string()
}

fn default_rulepack_path() -> String {
    "stdlib/stdlib.rulepack.json".to_string()
}

fn default_mappings_path() -> String {
    "mappings".to_string()
}

fn default_metamodel_constructs_path() -> String {
    "mappings/metamodel_constructs.seed.json".to_string()
}

fn default_kir_emission_path() -> String {
    "mappings/kir_emission.seed.json".to_string()
}

fn default_lowering_rules_path() -> String {
    "mappings/lowering_rules.seed.json".to_string()
}

fn default_semantic_defaults_path() -> String {
    "mappings/semantic_defaults.seed.json".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StdlibLocator;

    #[test]
    fn release_selector_resolves_2026_04_bundle() {
        let by_selector = release_bundle("2026-04").unwrap();
        let by_alias = release_bundle("pilot-2026-04").unwrap();

        assert_eq!(by_selector.profile_id, SYSML_2_0_PILOT_2026_04_ID);
        assert_eq!(by_selector.selector, "2026-04");
        assert_eq!(by_alias.profile_id, by_selector.profile_id);
        assert!(
            by_selector
                .conformance_trace_path
                .as_ref()
                .is_some_and(|path| path.ends_with("conformance/conformance-trace.json"))
        );
    }

    #[test]
    fn metamodel_resource_accepts_2026_04_selector_and_aliases() {
        let by_profile = metamodel_resource(SYSML_2_0_PILOT_2026_04_ID).unwrap();
        let by_selector = metamodel_resource("2026-04").unwrap();
        let by_alias = metamodel_resource("pilot-2026-04").unwrap();

        assert_eq!(by_profile.info.id, SYSML_2_0_PILOT_2026_04_ID);
        assert_eq!(by_selector.info.id, by_profile.info.id);
        assert_eq!(by_alias.info.id, by_profile.info.id);
        assert!(
            by_profile
                .sysml_delta_path
                .ends_with("sysml-library.kir.json")
        );
    }

    #[test]
    fn stdlib_locator_resolves_2026_04_release_selector() {
        let locator = StdlibLocator::for_release("2026-04").unwrap();

        assert!(matches!(locator, StdlibLocator::File { .. }));
        assert!(
            locator
                .as_uri()
                .contains("resources/metamodels/sysml-2.0-pilot-2026-04")
        );
    }
}
