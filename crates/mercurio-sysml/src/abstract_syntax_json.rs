//! Importer for OMG SysML/KerML abstract syntax JSON and Systems Modeling API
//! element payloads.
//!
//! Version 2 imports pinned Ecore features under their generated snake-case
//! names. In particular `owned_relationship`, `owned_related_element`, and
//! `related_element` remain distinct. Reference cardinality follows the owner
//! metaclass: Feature.type is a list while FeatureTyping.type is singular.
//! Callers must inspect report diagnostics. External references are retained
//! but unresolved; embedded objects and automatic inverse/delegate evaluation
//! are not supported. Legacy native field aliases remain exportable.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use mercurio_foundation::kir::{
    KIR_SCHEMA_VERSION, KIR_SCHEMA_VERSION_METADATA_KEY, KirDocument, KirElement, KirError,
    KirFieldKind, KirFieldRegistry, inferred_layer,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::language_frontend::lowering::ecore_model::{self, FeatureKind};
use crate::language_frontend::lowering::relationship_declarations::metaclass_conforms;
use crate::sysml_field_specs;

pub const SYSML_JSON_IMPORTER_VERSION: &str = concat!(
    "mercurio-sysml/",
    env!("CARGO_PKG_VERSION"),
    "/sysml-json-v2"
);
pub const SYSML_JSON_EXPORTER_VERSION: &str = concat!(
    "mercurio-sysml/",
    env!("CARGO_PKG_VERSION"),
    "/sysml-json-export-v2"
);

fn default_include_mercurio_extensions() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SysmlJsonImportOptions {
    #[serde(default)]
    pub source_uri: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub project_name: Option<String>,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub branch_name: Option<String>,
    #[serde(default)]
    pub commit_id: Option<String>,
    #[serde(default)]
    pub schema_profile: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SysmlJsonExportOptions {
    #[serde(default)]
    pub source_uri: Option<String>,
    #[serde(default)]
    pub schema_profile: Option<String>,
    #[serde(default = "default_include_mercurio_extensions")]
    pub include_mercurio_extensions: bool,
}

impl Default for SysmlJsonExportOptions {
    fn default() -> Self {
        Self {
            source_uri: None,
            schema_profile: None,
            include_mercurio_extensions: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SysmlJsonImportReport {
    pub document: KirDocument,
    pub diagnostics: Vec<SysmlJsonImportDiagnostic>,
    pub metadata: BTreeMap<String, Value>,
}

/// One canonical stored-property update. None removes an optional property;
/// Some(Value::Null) sets an explicit null where Ecore permits it.
#[derive(Debug, Clone, PartialEq)]
pub struct SysmlStoredFeatureEdit {
    pub element_id: String,
    pub field: String,
    pub value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SysmlJsonExportReport {
    pub value: Value,
    pub diagnostics: Vec<SysmlJsonExportDiagnostic>,
    pub metadata: BTreeMap<String, Value>,
}

impl SysmlJsonExportReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == SysmlJsonExportSeverity::Error)
    }
}

impl SysmlJsonImportReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == SysmlJsonImportSeverity::Error)
    }

    /// Check the current document at the shared structural publication boundary.
    /// Closed mode additionally rejects unresolved reference IDs. Missing required
    /// values and executable semantic dependencies still need separate assessment.
    /// Import ingestion retains inconsistent snapshots for inspection and repair.
    pub fn publication_diagnostics(&self, require_resolved_references: bool) -> Vec<SysmlJsonImportDiagnostic> {
        let references = if require_resolved_references {
            ecore_model::ReferenceCompleteness::Closed
        } else {
            ecore_model::ReferenceCompleteness::Partial
        };
        ecore_model::validate_publication(&self.document.elements, references).into_iter()
            .map(|issue| diagnostic(SysmlJsonImportSeverity::Error, "ecore.publication.violation",
                issue.message, Some(issue.element_id), Some(issue.field))).collect()
    }

    /// Stage stored-property edits and commit the entire batch only if all checks
    /// pass. Imported mutability, value and reference contracts govern writes.
    /// Ownership changes, identity changes, unsettable state and graphs containing
    /// derived snapshots require their dedicated services; failure preserves all
    /// properties and metadata. Reference updates require in-graph targets.
    pub fn apply_stored_feature_edits(&mut self, edits: &[SysmlStoredFeatureEdit]) -> Result<(), SysmlJsonImportError> {
        if self.has_errors() {
            return Err(SysmlJsonImportError::Shape("Cannot mutate an import with error diagnostics".into()));
        }
        let edits: Vec<_> = edits.iter().map(|edit| ecore_model::StoredFeatureEdit {
            element_id: &edit.element_id, field: &edit.field, value: edit.value.clone(),
        }).collect();
        ecore_model::apply_stored_feature_edits(&mut self.document.elements, &edits).map_err(SysmlJsonImportError::Shape)
    }

    /// Move a contained element using canonical KIR field names and pinned Ecore
    /// opposites. Both ownership directions change atomically; list order is
    /// preserved and the moved element is appended at its new owner. Partial or
    /// external old ownership and containment cycles reject without mutation.
    /// Graphs with explicit derived/volatile properties require a recomputation
    /// service and are rejected; this API supports stored containment only.
    pub fn move_contained_element(
        &mut self,
        element_id: &str,
        owner_id: &str,
        field: &str,
    ) -> Result<(), SysmlJsonImportError> {
        if self.has_errors() {
            return Err(SysmlJsonImportError::Shape(
                "Cannot mutate an import with error diagnostics".into(),
            ));
        }
        ecore_model::move_contained_element(
            &mut self.document.elements,
            element_id,
            owner_id,
            field,
        )
        .map_err(SysmlJsonImportError::Shape)
    }

    /// Evaluate a pinned derived union from resolved subset inputs.
    /// Canonical snake_case fields are required. Reciprocal owned relationships
    /// derive owned memberships and prove empty imports when no Import exists.
    /// Closed canonical namespace imports use the shared scope service. Type
    /// targets derive inherited memberships when implied relationships are fully
    /// materialized; otherwise nonrecursive access requires resolved snapshots.
    /// Filtered imports, missing implicit semantics and external graphs remain bounded.
    /// This does not mutate storage or trust a cached union.
    /// The current bounded service supports Namespace.membership and subtypes.
    pub fn evaluate_explicit_union(
        &self,
        element_id: &str,
        field: &str,
    ) -> Result<Value, SysmlJsonImportError> {
        if self.has_errors() {
            return Err(SysmlJsonImportError::Shape("Cannot evaluate an import with error diagnostics".into()));
        }
        let element = self.document.elements.iter().find(|e| e.id == element_id)
            .ok_or_else(|| SysmlJsonImportError::Shape(format!("Unknown element {element_id}")))?;
        ecore_model::evaluate_explicit_union(&self.document.elements, element, field)
            .map_err(SysmlJsonImportError::Shape)
    }

    /// Evaluate ordered explicit and default general types for the five plain
    /// Type/Classifier/Class/DataType/Structure metaclasses in a closed graph.
    /// Library bindings map canonical qualified library names to graph IDs.
    /// They are an explicit resolution dependency, not a global lookup service.
    /// Incomplete annotated graphs, conjugation and conditional subclasses reject.
    /// Complete materialized inputs use stored specializations only. This is a
    /// read-only computation: it neither inserts relationships nor marks the
    /// model semantically complete. Ordered duplicate contributions are retained.
    pub fn evaluate_plain_type_generalizations(&self, element_id: &str,
        library_bindings: &BTreeMap<String, String>) -> Result<Vec<String>, SysmlJsonImportError> {
        if self.has_errors() {
            return Err(SysmlJsonImportError::Shape("Cannot evaluate an import with error diagnostics".into()));
        }
        ecore_model::evaluate_plain_type_generalizations(&self.document.elements, element_id, library_bindings)
            .map_err(SysmlJsonImportError::Shape)
    }

    /// Evaluate membership after computing supported plain-Type defaults on a
    /// private graph copy. Every incomplete Type in the supplied closed graph
    /// must be supported by the bounded default provider. Library bindings are
    /// explicit dependencies; no global lookup or metadata computation is implied.
    /// The original model and its completeness flags are never modified.
    pub fn evaluate_plain_type_membership(&self, element_id: &str,
        library_bindings: &BTreeMap<String, String>) -> Result<Value, SysmlJsonImportError> {
        if self.has_errors() {
            return Err(SysmlJsonImportError::Shape("Cannot evaluate an import with error diagnostics".into()));
        }
        ecore_model::evaluate_plain_type_membership(&self.document.elements, element_id, library_bindings)
            .map_err(SysmlJsonImportError::Shape)
    }

    /// Resolve plain-Type defaults from canonical, root-level standard
    /// LibraryPackages in this closed graph, then evaluate membership natively.
    /// Public effective membership names and import visibility govern lookup;
    /// ambiguous roots, missing dependencies and ID-spelling fallbacks reject.
    /// This does not load external resources or change the original graph.
    pub fn evaluate_plain_type_membership_from_libraries(&self, element_id: &str) -> Result<Value, SysmlJsonImportError> {
        if self.has_errors() {
            return Err(SysmlJsonImportError::Shape("Cannot evaluate an import with error diagnostics".into()));
        }
        ecore_model::evaluate_plain_type_membership_from_libraries(&self.document.elements, element_id)
            .map_err(SysmlJsonImportError::Shape)
    }

    /// Assess required Ecore values without materializing defaults or mutating
    /// the document. Errors identify definite stored-value violations; warnings
    /// identify unverified semantic services or external targets. An empty list
    /// establishes only these required-feature checks, not full conformance.
    /// Partial-model persistence deliberately remains a separate operation.
    pub fn required_feature_diagnostics(&self) -> Vec<SysmlJsonImportDiagnostic> {
        ecore_model::assess_required_features(&self.document.elements)
            .into_iter()
            .map(|issue| SysmlJsonImportDiagnostic {
                severity: if issue.unverified {
                    SysmlJsonImportSeverity::Warning
                } else {
                    SysmlJsonImportSeverity::Error
                },
                code: if issue.unverified {
                    "ecore.required.unverified"
                } else {
                    "ecore.required.violation"
                }
                .into(),
                message: issue.message,
                element_id: Some(issue.element_id),
                path: Some(issue.field),
            })
            .collect()
    }

    /// Prepare a validated import for KIR persistence using the pinned Ecore
    /// field shapes. Unresolved external references remain warnings. This does
    /// not evaluate delegates or turn a partial document into a complete model.
    pub fn persistable_document(&self) -> Result<KirDocument, SysmlJsonImportError> {
        if self.has_errors() {
            return Err(SysmlJsonImportError::Shape(
                "Cannot persist a SysML JSON import with error diagnostics".into(),
            ));
        }
        // The report exposes a mutable document: validate current contents,
        // not just the diagnostics captured when it was originally imported.
        if let Some(issue) = ecore_model::validate_publication(&self.document.elements, ecore_model::ReferenceCompleteness::Partial)
            .into_iter()
            .next()
        {
            return Err(SysmlJsonImportError::Shape(format!(
                "{}.{}: {}",
                issue.element_id, issue.field, issue.message
            )));
        }
        let registry = sysml_json_field_registry(&self.document);
        let document = self
            .document
            .clone()
            .normalized_for_persistence_with_registry(registry.clone());
        document.validate_persisted_with_registry(registry)?;
        Ok(document)
    }
}

/// The field registry for v2 JSON-imported KIR, including inherited Ecore
/// contracts scoped to each exact kind in this document. Use the same registry
/// for normalization and strict persisted validation; global field names cannot
/// represent every Ecore shape. This function performs no semantic validation.
pub fn sysml_json_field_registry(document: &KirDocument) -> KirFieldRegistry {
    let mut registry = KirFieldRegistry::from_document(document);
    registry.register_fields(sysml_field_specs().iter().copied());
    let kinds: BTreeSet<_> = document.elements.iter().map(|e| e.kind.as_str()).collect();
    for kind in kinds {
        ecore_model::register_field_contracts(&mut registry, kind);
    }
    registry
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SysmlJsonImportDiagnostic {
    pub severity: SysmlJsonImportSeverity,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SysmlJsonExportDiagnostic {
    pub severity: SysmlJsonExportSeverity,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SysmlJsonImportSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SysmlJsonExportSeverity {
    Warning,
    Error,
}

#[derive(Debug)]
pub enum SysmlJsonImportError {
    Json(serde_json::Error),
    Kir(KirError),
    Shape(String),
    DuplicateId(String),
}

#[derive(Debug)]
pub enum SysmlJsonExportError {
    Json(serde_json::Error),
    Kir(KirError),
}

impl fmt::Display for SysmlJsonImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(err) => write!(f, "failed to parse SysML JSON: {err}"),
            Self::Kir(err) => write!(f, "imported KIR document is invalid: {err}"),
            Self::Shape(message) => write!(f, "{message}"),
            Self::DuplicateId(id) => write!(f, "duplicate SysML JSON element id: {id}"),
        }
    }
}

impl std::error::Error for SysmlJsonImportError {}

impl fmt::Display for SysmlJsonExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(err) => write!(f, "failed to serialize SysML JSON: {err}"),
            Self::Kir(err) => write!(f, "KIR document is not exportable as SysML JSON: {err}"),
        }
    }
}

impl std::error::Error for SysmlJsonExportError {}

impl From<serde_json::Error> for SysmlJsonImportError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<KirError> for SysmlJsonImportError {
    fn from(value: KirError) -> Self {
        Self::Kir(value)
    }
}

impl From<serde_json::Error> for SysmlJsonExportError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<KirError> for SysmlJsonExportError {
    fn from(value: KirError) -> Self {
        Self::Kir(value)
    }
}

pub fn import_sysml_abstract_syntax_json(
    input: &str,
    options: SysmlJsonImportOptions,
) -> Result<SysmlJsonImportReport, SysmlJsonImportError> {
    let value: Value = serde_json::from_str(input)?;
    import_sysml_abstract_syntax_value(value, options)
}

pub fn import_sysml_abstract_syntax_value(
    value: Value,
    options: SysmlJsonImportOptions,
) -> Result<SysmlJsonImportReport, SysmlJsonImportError> {
    match value {
        Value::Array(elements) => import_sysml_api_elements(elements, options),
        Value::Object(mut object) => {
            if let Some(Value::Array(elements)) = object.remove("elements") {
                import_elements(elements, options, Some(Value::Object(object)))
            } else if object.contains_key("@id") || object.contains_key("elementId") {
                import_elements(vec![Value::Object(object)], options, None)
            } else {
                Err(SysmlJsonImportError::Shape(
                    "SysML abstract syntax JSON must be an element object, an element array, or an object with an `elements` array".to_string(),
                ))
            }
        }
        _ => Err(SysmlJsonImportError::Shape(
            "SysML abstract syntax JSON root must be an object or array".to_string(),
        )),
    }
}

pub fn import_sysml_api_elements(
    elements: Vec<Value>,
    metadata: SysmlJsonImportOptions,
) -> Result<SysmlJsonImportReport, SysmlJsonImportError> {
    import_elements(elements, metadata, None)
}

pub fn export_sysml_abstract_syntax_json(
    document: &KirDocument,
    options: SysmlJsonExportOptions,
) -> Result<String, SysmlJsonExportError> {
    let report = export_sysml_abstract_syntax_value(document, options)?;
    Ok(serde_json::to_string_pretty(&report.value)?)
}

pub fn export_sysml_abstract_syntax_value(
    document: &KirDocument,
    options: SysmlJsonExportOptions,
) -> Result<SysmlJsonExportReport, SysmlJsonExportError> {
    document.validate()?;

    let mut diagnostics = Vec::new();
    let mut registry = KirFieldRegistry::structural();
    registry.register_fields(sysml_field_specs().iter().copied());
    registry.extend_from_document(document);
    for issue in ecore_model::validate_explicit_reference_graph(&document.elements) {
        diagnostics.push(export_diagnostic(
            SysmlJsonExportSeverity::Error,
            "sysml_json_export.reference.ecore_graph",
            issue.message,
            Some(issue.element_id),
            Some(issue.field),
        ));
    }

    let exchange_ids = exchange_id_map(document, &mut diagnostics);
    let kind_by_id = document
        .elements
        .iter()
        .map(|element| (element.id.as_str(), element.kind.as_str()))
        .collect::<BTreeMap<_, _>>();
    let mut elements = Vec::with_capacity(document.elements.len());
    for (index, element) in document.elements.iter().enumerate() {
        let path = format!("elements[{index}]");
        elements.push(export_element(
            element,
            &registry,
            &exchange_ids,
            &kind_by_id,
            &options,
            &path,
            &mut diagnostics,
        ));
    }

    let mut metadata = BTreeMap::new();
    metadata.insert(
        "source_format".to_string(),
        Value::String("sysml-abstract-syntax-json".to_string()),
    );
    metadata.insert(
        "exporter_version".to_string(),
        Value::String(SYSML_JSON_EXPORTER_VERSION.to_string()),
    );
    metadata.insert("element_count".to_string(), json!(elements.len()));
    insert_optional_btree(&mut metadata, "source_uri", options.source_uri.as_deref());
    insert_optional_btree(
        &mut metadata,
        "schema_profile",
        options.schema_profile.as_deref(),
    );

    let mut value = Map::new();
    value.insert(
        "format".to_string(),
        Value::String("sysml-abstract-syntax-json".to_string()),
    );
    value.insert(
        "exporterVersion".to_string(),
        Value::String(SYSML_JSON_EXPORTER_VERSION.to_string()),
    );
    if let Some(source_uri) = options.source_uri {
        value.insert("sourceUri".to_string(), Value::String(source_uri));
    }
    if let Some(schema_profile) = options.schema_profile {
        value.insert("schemaProfile".to_string(), Value::String(schema_profile));
    }
    value.insert("elements".to_string(), Value::Array(elements));

    Ok(SysmlJsonExportReport {
        value: Value::Object(value),
        diagnostics,
        metadata,
    })
}

fn export_element(
    element: &KirElement,
    registry: &KirFieldRegistry,
    exchange_ids: &BTreeMap<String, String>,
    kind_by_id: &BTreeMap<&str, &str>,
    options: &SysmlJsonExportOptions,
    path: &str,
    diagnostics: &mut Vec<SysmlJsonExportDiagnostic>,
) -> Value {
    let mut object = Map::new();
    let exchange_id = exchange_ids
        .get(&element.id)
        .cloned()
        .unwrap_or_else(|| deterministic_exchange_uuid(&element.id));
    object.insert("@id".to_string(), Value::String(exchange_id));
    object.insert("@type".to_string(), Value::String(element.kind.clone()));

    let mut extension_properties = Map::new();
    let mut extension_metadata = Map::new();

    for (property, value) in &element.properties {
        if property == "element_id" {
            continue;
        }
        if property == "metadata" {
            if let Some(metadata) = value.as_object() {
                extension_metadata.extend(metadata.clone());
            } else {
                extension_metadata.insert("metadata".to_string(), value.clone());
            }
            continue;
        }
        if property == "x_sysml_api" {
            if let Some(extension) = value.as_object() {
                for (key, value) in extension {
                    object.insert(
                        snake_to_camel(key),
                        denormalize_extension_value(value, exchange_ids),
                    );
                }
            }
            continue;
        }

        // Feature identities and shapes come from Ecore, including distinct
        // derived and owned references and same-named fields on different kinds.
        if let Some(contract) = ecore_model::feature(&element.kind, property) {
            if let Err(message) = ecore_model::validate_value(&element.kind, property, value) {
                diagnostics.push(export_diagnostic(
                    SysmlJsonExportSeverity::Error,
                    "sysml_json_export.property.ecore_value",
                    message,
                    Some(element.id.clone()),
                    Some(property.clone()),
                ));
                object.insert(contract.name.to_string(), value.clone());
                continue;
            }
            let exported = match contract.kind {
                FeatureKind::Attribute => value.clone(),
                FeatureKind::Reference => {
                    for id in reference_ids(value) {
                        let (severity, code, message) = match kind_by_id.get(id) {
                            Some(target_kind) => match ecore_model::validate_reference_endpoint(
                                &element.kind,
                                property,
                                target_kind,
                            ) {
                                Ok(()) => continue,
                                Err(message) => (
                                    SysmlJsonExportSeverity::Error,
                                    "sysml_json_export.reference.ecore_target",
                                    message,
                                ),
                            },
                            None => (
                                SysmlJsonExportSeverity::Warning,
                                "sysml_json_export.reference.unresolved",
                                format!("Reference {id} is external; target type is unverified"),
                            ),
                        };
                        diagnostics.push(export_diagnostic(
                            severity,
                            code,
                            message,
                            Some(element.id.clone()),
                            Some(property.clone()),
                        ));
                    }
                    if contract.upper == 1 {
                        export_reference_value(value, exchange_ids, kind_by_id)
                    } else {
                        export_reference_list_value(value, exchange_ids, kind_by_id)
                    }
                }
            };
            object.insert(contract.name.to_string(), exported);
            continue;
        }

        let Some(target_property) = sysml_json_property_name(property) else {
            extension_properties.insert(property.clone(), value.clone());
            diagnostics.push(export_diagnostic(
                SysmlJsonExportSeverity::Warning,
                "sysml_json_export.property.extension",
                format!(
                    "KIR property `{property}` has no standard SysML JSON mapping and was preserved under `xMercurio.properties`"
                ),
                Some(element.id.clone()),
                Some(property.clone()),
            ));
            continue;
        };

        let exported_value = match registry
            .field_for(&element.kind, property)
            .map(|spec| spec.kind)
        {
            Some(KirFieldKind::Reference) => {
                export_reference_value(value, exchange_ids, kind_by_id)
            }
            Some(KirFieldKind::ReferenceList) => {
                export_reference_list_value(value, exchange_ids, kind_by_id)
            }
            Some(KirFieldKind::Expression | KirFieldKind::Metadata) => {
                extension_properties.insert(property.clone(), value.clone());
                diagnostics.push(export_diagnostic(
                    SysmlJsonExportSeverity::Warning,
                    "sysml_json_export.property.extension",
                    format!(
                        "KIR structured property `{property}` has no standard SysML JSON scalar/reference mapping and was preserved under `xMercurio.properties`"
                    ),
                    Some(element.id.clone()),
                    Some(property.clone()),
                ));
                continue;
            }
            Some(KirFieldKind::Scalar | KirFieldKind::ScalarList) | None => value.clone(),
        };
        object.insert(target_property, exported_value);
    }

    if options.include_mercurio_extensions {
        let mut extension = Map::new();
        extension.insert("kirId".to_string(), Value::String(element.id.clone()));
        extension.insert("kirKind".to_string(), Value::String(element.kind.clone()));
        extension.insert(
            "exporterVersion".to_string(),
            Value::String(SYSML_JSON_EXPORTER_VERSION.to_string()),
        );
        extension.insert("path".to_string(), Value::String(path.to_string()));
        if !extension_properties.is_empty() {
            extension.insert(
                "properties".to_string(),
                Value::Object(extension_properties),
            );
        }
        if !extension_metadata.is_empty() {
            extension.insert("metadata".to_string(), Value::Object(extension_metadata));
        }
        object.insert("xMercurio".to_string(), Value::Object(extension));
    }

    Value::Object(object)
}

fn import_elements(
    elements: Vec<Value>,
    options: SysmlJsonImportOptions,
    source_document_metadata: Option<Value>,
) -> Result<SysmlJsonImportReport, SysmlJsonImportError> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    let mut imported = Vec::new();
    let mut registry = KirFieldRegistry::structural();
    registry.register_fields(sysml_field_specs().iter().copied());

    // Resolve local identities before reading properties, so forward references
    // and Mercurio exchange-ID round trips have the same behavior as backward ones.
    let mut reference_targets = BTreeMap::new();
    for object in elements.iter().filter_map(Value::as_object) {
        if let (Some(external), Some(internal), Some(kind)) = (
            element_external_id(object),
            element_id(object),
            object.get("@type").and_then(Value::as_str),
        ) {
            for alias in [external, internal] {
                let target = (internal.to_string(), kind.to_string());
                if reference_targets
                    .insert(alias.to_string(), target.clone())
                    .is_some_and(|previous| previous != target)
                {
                    return Err(SysmlJsonImportError::DuplicateId(alias.to_string()));
                }
            }
        }
    }

    for (index, value) in elements.into_iter().enumerate() {
        let path = format!("elements[{index}]");
        let Value::Object(object) = value else {
            diagnostics.push(diagnostic(
                SysmlJsonImportSeverity::Error,
                "sysml_json.element.shape",
                "SysML JSON element must be an object",
                None,
                Some(path),
            ));
            continue;
        };

        if let Some(element) = import_element(
            object,
            &options,
            &registry,
            &reference_targets,
            &path,
            &mut diagnostics,
        )? {
            if !seen.insert(element.id.clone()) {
                return Err(SysmlJsonImportError::DuplicateId(element.id));
            }
            imported.push(element);
        }
    }

    let mut document = KirDocument {
        metadata: import_metadata(&options, source_document_metadata),
        elements: imported,
    };
    for issue in ecore_model::validate_explicit_reference_graph(&document.elements) {
        diagnostics.push(diagnostic(
            SysmlJsonImportSeverity::Error,
            "sysml_json.reference.ecore_graph",
            issue.message,
            Some(issue.element_id),
            Some(issue.field),
        ));
    }
    document.validate()?;
    document.set_schema_version();

    let metadata = document.metadata.clone();
    Ok(SysmlJsonImportReport {
        document,
        diagnostics,
        metadata,
    })
}

fn import_element(
    object: Map<String, Value>,
    options: &SysmlJsonImportOptions,
    registry: &KirFieldRegistry,
    reference_targets: &BTreeMap<String, (String, String)>,
    path: &str,
    diagnostics: &mut Vec<SysmlJsonImportDiagnostic>,
) -> Result<Option<KirElement>, SysmlJsonImportError> {
    let external_id = match element_external_id(&object) {
        Some(id) => id.to_string(),
        None => {
            diagnostics.push(diagnostic(
                SysmlJsonImportSeverity::Error,
                "sysml_json.element.missing_id",
                "SysML JSON element is missing `@id`",
                None,
                Some(path.to_string()),
            ));
            return Ok(None);
        }
    };

    let id = match element_id(&object) {
        Some(id) => id.to_string(),
        None => {
            diagnostics.push(diagnostic(
                SysmlJsonImportSeverity::Error,
                "sysml_json.element.missing_id",
                "SysML JSON element is missing a usable `@id` or Mercurio KIR id extension",
                None,
                Some(path.to_string()),
            ));
            return Ok(None);
        }
    };

    if !object.contains_key("@id") && object.contains_key("elementId") {
        diagnostics.push(diagnostic(
            SysmlJsonImportSeverity::Warning,
            "sysml_json.element.fallback_id",
            "SysML JSON element used `elementId` because `@id` was absent",
            Some(id.clone()),
            Some(path.to_string()),
        ));
    }

    let Some(kind) = object.get("@type").and_then(Value::as_str) else {
        diagnostics.push(diagnostic(
            SysmlJsonImportSeverity::Error,
            "sysml_json.element.missing_type",
            "SysML JSON element is missing `@type`",
            Some(id.clone()),
            Some(path.to_string()),
        ));
        return Ok(None);
    };
    let kind = kind.to_string();

    let mut properties = BTreeMap::new();
    let mut extension = Map::new();

    for (source_key, value) in &object {
        if source_key == "@id" || source_key == "@type" || source_key == "xMercurio" {
            continue;
        }

        let ecore_field = camel_to_snake(source_key);
        if let Some(contract) = ecore_model::feature(&kind, &ecore_field) {
            // Validate the original value: an object containing @id must not
            // be coerced into a valid string-valued attribute.
            let converted = match contract.kind {
                FeatureKind::Attribute => ecore_model::validate_value(&kind, contract.field, value)
                    .map(|()| (value.clone(), BTreeSet::new())),
                FeatureKind::Reference => {
                    import_ecore_reference(&kind, contract.field, value, reference_targets)
                }
            };
            let (converted, unresolved) = match converted {
                Ok(result) => result,
                Err(message) => {
                    diagnostics.push(diagnostic(
                        SysmlJsonImportSeverity::Error,
                        "sysml_json.property.ecore_value",
                        message,
                        Some(id.clone()),
                        Some(format!("{path}.{source_key}")),
                    ));
                    extension.insert(extension_key(source_key), value.clone());
                    continue;
                }
            };
            for target in unresolved {
                diagnostics.push(diagnostic(SysmlJsonImportSeverity::Warning,
                    "sysml_json.reference.unresolved",
                    format!("Reference {target} is external; target existence and type remain unverified"),
                    Some(id.clone()), Some(format!("{path}.{source_key}"))));
            }
            if properties
                .insert(contract.field.to_string(), converted)
                .is_some()
            {
                diagnostics.push(diagnostic(
                    SysmlJsonImportSeverity::Warning,
                    "sysml_json.property.duplicate_mapping",
                    format!(
                        "multiple SysML JSON properties mapped to `{}`",
                        contract.field
                    ),
                    Some(id.clone()),
                    Some(path.to_string()),
                ));
            }
            continue;
        }

        let normalized = normalize_value(value.clone());
        if let Some(property_name) = kir_property_name(source_key, &kind, registry) {
            if should_preserve_as_extension(&property_name, &kind, &normalized, registry) {
                diagnostics.push(diagnostic(
                    SysmlJsonImportSeverity::Warning,
                    "sysml_json.property.structured_scalar",
                    format!(
                        "SysML JSON property `{source_key}` was preserved under `x_sysml_api` because KIR property `{property_name}` expects a scalar"
                    ),
                    Some(id.clone()),
                    Some(path.to_string()),
                ));
                extension.insert(extension_key(source_key), normalized);
                continue;
            }
            if properties
                .insert(property_name.clone(), normalized)
                .is_some()
            {
                diagnostics.push(diagnostic(
                    SysmlJsonImportSeverity::Warning,
                    "sysml_json.property.duplicate_mapping",
                    format!("multiple SysML JSON properties mapped to `{property_name}`"),
                    Some(id.clone()),
                    Some(path.to_string()),
                ));
            }
        } else {
            extension.insert(extension_key(source_key), normalized);
        }
    }

    if !extension.is_empty() {
        properties.insert("x_sysml_api".to_string(), Value::Object(extension));
    }

    merge_source_provenance(
        &mut properties,
        source_provenance(options, &external_id, &kind, path),
    );

    let layer = imported_layer(&object, &id, &kind, &properties);
    Ok(Some(KirElement {
        id,
        kind,
        layer,
        properties,
    }))
}

fn reference_ids(value: &Value) -> Vec<&str> {
    match value {
        Value::String(id) => vec![id.as_str()],
        Value::Array(values) => values.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

/// Handwritten exchange-identity resolution driven by the generated EReference
/// contract. This does not implement namespace scope, proxy loading, inverse
/// maintenance, containment construction, or delegate evaluation.
fn import_ecore_reference(
    kind: &str,
    field: &str,
    value: &Value,
    targets: &BTreeMap<String, (String, String)>,
) -> Result<(Value, BTreeSet<String>), String> {
    let normalized = normalize_value(value.clone());
    ecore_model::validate_value(kind, field, &normalized)?;
    let mut unresolved = BTreeSet::new();
    let mut resolve = |item: &Value| -> Result<Value, String> {
        if item.is_null() {
            return Ok(Value::Null);
        }
        let (id, claimed_type) = match item {
            Value::String(id) => (id.as_str(), None),
            Value::Object(object) => {
                let id = simple_reference_id(object).ok_or_else(|| {
                    format!("Embedded objects are not supported for Ecore reference {kind}.{field}")
                })?;
                let claimed = match object.get("@type") {
                    None => None,
                    Some(Value::String(kind)) => Some(kind.as_str()),
                    Some(_) => return Err(format!("Invalid reference @type for {kind}.{field}")),
                };
                (id, claimed)
            }
            _ => return Err(format!("Invalid reference for {kind}.{field}")),
        };
        if let Some(claimed) = claimed_type {
            ecore_model::validate_reference_endpoint(kind, field, claimed)?;
        }
        if let Some((internal, actual)) = targets.get(id) {
            ecore_model::validate_reference_endpoint(kind, field, actual)?;
            if claimed_type.is_some_and(|claimed| {
                !metaclass_conforms(actual, claimed.rsplit("::").next().unwrap_or(claimed))
            }) {
                return Err(format!(
                    "Reference {id} claims an incompatible type for actual {actual}"
                ));
            }
            Ok(Value::String(internal.clone()))
        } else {
            unresolved.insert(id.to_string());
            Ok(Value::String(id.to_string()))
        }
    };
    let converted = match value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(&mut resolve)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        _ => resolve(value)?,
    };
    // Different exchange aliases may resolve to the same native identity.
    ecore_model::validate_value(kind, field, &converted)?;
    Ok((converted, unresolved))
}

fn element_id(object: &Map<String, Value>) -> Option<&str> {
    mercurio_kir_id(object).or_else(|| element_external_id(object))
}

fn element_external_id(object: &Map<String, Value>) -> Option<&str> {
    object
        .get("@id")
        .or_else(|| object.get("elementId"))
        .and_then(Value::as_str)
}

fn mercurio_kir_id(object: &Map<String, Value>) -> Option<&str> {
    object
        .get("xMercurio")
        .and_then(Value::as_object)
        .and_then(|extension| extension.get("kirId"))
        .and_then(Value::as_str)
        .or_else(|| object.get("xMercurioKirId").and_then(Value::as_str))
}

fn imported_layer(
    object: &Map<String, Value>,
    id: &str,
    kind: &str,
    properties: &BTreeMap<String, Value>,
) -> u8 {
    if object
        .get("isLibraryElement")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return 1;
    }
    if kind == "LibraryPackage" || kind.starts_with("Library") {
        return 1;
    }
    inferred_layer(id, kind, properties)
}

fn kir_property_name(source_key: &str, kind: &str, registry: &KirFieldRegistry) -> Option<String> {
    let candidate = match source_key {
        "elementId" => "element_id",
        "declaredName" => "declared_name",
        "declaredShortName" => "declared_short_name",
        "qualifiedName" => "qualified_name",
        "isAbstract" => "is_abstract",
        "isConjugated" => "is_conjugated",
        "isDerived" => "is_derived",
        "isEnd" => "is_end",
        "isVariable" => "is_variable",
        "isReadOnly" | "isReadonly" => "is_readonly",
        "isOrdered" => "is_ordered",
        "isUnique" => "is_unique",
        "owningType" => "owning_type",
        "owningDefinition" => "owning_definition",
        "owningNamespace" => "owning_namespace",
        "featuringType" => "featuring_type",
        "chainingFeature" => "chaining_feature",
        "sourceFeature" => "source_feature",
        "ownedRelationship" => "relationships",
        "ownedRelatedElement" | "relatedElement" => "related",
        "ownedFeature" => "owned_features",
        "ownedTyping" | "featureTyping" => "feature_typings",
        "ownedSpecialization" => "specializes",
        "ownedImport" => "imports",
        "ownedMember" | "member" => "members",
        "feature" => "features",
        "ownedFeatureMembership" | "featureMembership" => "features",
        "parameter" => "parameters",
        "argument" => "arguments",
        _ => source_key,
    };

    if registry.field_for(kind, candidate).is_some() {
        return Some(candidate.to_string());
    }

    let snake = camel_to_snake(source_key);
    registry.field_for(kind, &snake).map(|_| snake)
}

fn should_preserve_as_extension(
    property_name: &str,
    kind: &str,
    value: &Value,
    registry: &KirFieldRegistry,
) -> bool {
    matches!(
        registry
            .field_for(kind, property_name)
            .map(|spec| spec.kind),
        Some(KirFieldKind::Scalar)
    ) && (value.is_object() || value.is_array())
}

fn normalize_value(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            if let Some(id) = simple_reference_id(&object) {
                return Value::String(id.to_string());
            }

            let mut normalized = Map::new();
            for (key, value) in object {
                normalized.insert(extension_key(&key), normalize_value(value));
            }
            Value::Object(normalized)
        }
        Value::Array(items) => {
            if let Some(reference_ids) = simple_reference_array(&items) {
                return Value::Array(reference_ids.into_iter().map(Value::String).collect());
            }
            Value::Array(items.into_iter().map(normalize_value).collect())
        }
        other => other,
    }
}

fn denormalize_extension_value(value: &Value, exchange_ids: &BTreeMap<String, String>) -> Value {
    match value {
        Value::String(id) => reference_object(id, exchange_ids, None),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| denormalize_extension_value(item, exchange_ids))
                .collect(),
        ),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    (
                        snake_to_camel(key),
                        denormalize_extension_value(value, exchange_ids),
                    )
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

fn exchange_id_map(
    document: &KirDocument,
    diagnostics: &mut Vec<SysmlJsonExportDiagnostic>,
) -> BTreeMap<String, String> {
    let mut by_kir_id = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for element in &document.elements {
        let exchange_id = element
            .properties
            .get("element_id")
            .and_then(Value::as_str)
            .filter(|id| is_uuid_like(id))
            .map(str::to_string)
            .unwrap_or_else(|| deterministic_exchange_uuid(&element.id));
        if !seen.insert(exchange_id.clone()) {
            diagnostics.push(export_diagnostic(
                SysmlJsonExportSeverity::Error,
                "sysml_json_export.element.exchange_id_duplicate",
                format!(
                    "KIR element `{}` produced duplicate SysML JSON exchange id `{exchange_id}`",
                    element.id
                ),
                Some(element.id.clone()),
                None,
            ));
        }
        by_kir_id.insert(element.id.clone(), exchange_id);
    }
    by_kir_id
}

fn export_reference_value(
    value: &Value,
    exchange_ids: &BTreeMap<String, String>,
    kind_by_id: &BTreeMap<&str, &str>,
) -> Value {
    match value {
        Value::String(id) => {
            reference_object(id, exchange_ids, kind_by_id.get(id.as_str()).copied())
        }
        Value::Array(items) => items
            .iter()
            .find_map(Value::as_str)
            .map(|id| reference_object(id, exchange_ids, kind_by_id.get(id).copied()))
            .unwrap_or(Value::Null),
        Value::Null => Value::Null,
        other => other.clone(),
    }
}

fn export_reference_list_value(
    value: &Value,
    exchange_ids: &BTreeMap<String, String>,
    kind_by_id: &BTreeMap<&str, &str>,
) -> Value {
    match value {
        Value::String(id) => Value::Array(vec![reference_object(
            id,
            exchange_ids,
            kind_by_id.get(id.as_str()).copied(),
        )]),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|id| reference_object(id, exchange_ids, kind_by_id.get(id).copied()))
                .collect(),
        ),
        Value::Null => Value::Array(Vec::new()),
        other => other.clone(),
    }
}

fn reference_object(
    kir_id: &str,
    exchange_ids: &BTreeMap<String, String>,
    kind: Option<&str>,
) -> Value {
    let mut object = Map::new();
    object.insert(
        "@id".to_string(),
        Value::String(
            exchange_ids
                .get(kir_id)
                .cloned()
                .unwrap_or_else(|| kir_id.to_string()),
        ),
    );
    if let Some(kind) = kind {
        object.insert("@type".to_string(), Value::String(kind.to_string()));
    }
    Value::Object(object)
}

fn sysml_json_property_name(property: &str) -> Option<String> {
    Some(
        match property {
            "declared_name" => "declaredName",
            "declared_short_name" => "declaredShortName",
            "qualified_name" => "qualifiedName",
            "short_name" => "shortName",
            "is_abstract" => "isAbstract",
            "is_conjugated" => "isConjugated",
            "is_derived" => "isDerived",
            "is_end" => "isEnd",
            "is_variable" => "isVariable",
            "is_readonly" => "isReadOnly",
            "is_ordered" => "isOrdered",
            "is_unique" => "isUnique",
            "is_library_element" => "isLibraryElement",
            "is_implied" => "isImplied",
            "owning_type" => "owningType",
            "owning_definition" => "owningDefinition",
            "owning_namespace" => "owningNamespace",
            "source_feature" => "sourceFeature",
            "members" => "ownedMember",
            "features" => "feature",
            "owned_features" | "owned_feature" => "ownedFeature",
            "specializes" => "ownedSpecialization",
            "subsets" => "ownedSubsetting",
            "subsetted_features" => "subsettedFeature",
            "redefines" => "ownedRedefinition",
            "redefined_features" => "redefinedFeature",
            "specialized_features" => "specializedFeature",
            "feature_typings" => "ownedTyping",
            "featuring_type" => "featuringType",
            "chaining_feature" => "chainingFeature",
            "relationships" => "ownedRelationship",
            "related" => "relatedElement",
            "imports" => "ownedImport",
            "parameters" => "parameter",
            "arguments" => "argument",
            "type"
            | "owner"
            | "source"
            | "target"
            | "definition"
            | "metatype"
            | "name"
            | "language"
            | "body"
            | "text"
            | "locale"
            | "direction"
            | "multiplicity"
            | "multiplicity_lower"
            | "multiplicity_upper"
            | "declared_multiplicity"
            | "operator"
            | "trigger"
            | "trigger_kind"
            | "effect"
            | "requirement_id" => {
                return Some(property.to_string());
            }
            _ => return None,
        }
        .to_string(),
    )
}

fn export_diagnostic(
    severity: SysmlJsonExportSeverity,
    code: impl Into<String>,
    message: impl Into<String>,
    element_id: Option<String>,
    property: Option<String>,
) -> SysmlJsonExportDiagnostic {
    SysmlJsonExportDiagnostic {
        severity,
        code: code.into(),
        message: message.into(),
        element_id,
        property,
    }
}

fn is_uuid_like(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && [8, 13, 18, 23].iter().all(|index| bytes[*index] == b'-')
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| [8, 13, 18, 23].contains(&index) || byte.is_ascii_hexdigit())
}

fn deterministic_exchange_uuid(input: &str) -> String {
    let left = fnv1a64(0xcbf29ce484222325, input.as_bytes());
    let right = fnv1a64(0x84222325cbf29ce4, input.as_bytes());
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&left.to_be_bytes());
    bytes[8..].copy_from_slice(&right.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

fn fnv1a64(seed: u64, bytes: &[u8]) -> u64 {
    let mut hash = seed;
    for byte in b"dev.mercurio.sysml-json" {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn simple_reference_array(items: &[Value]) -> Option<Vec<String>> {
    if items.is_empty() {
        return None;
    }

    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let Value::Object(object) = item else {
            return None;
        };
        let Some(id) = simple_reference_id(object) else {
            return None;
        };
        ids.push(id.to_string());
    }
    Some(ids)
}

fn simple_reference_id(object: &Map<String, Value>) -> Option<&str> {
    if object.keys().all(|key| key == "@id" || key == "@type") {
        return object.get("@id").and_then(Value::as_str);
    }
    None
}

fn merge_source_provenance(properties: &mut BTreeMap<String, Value>, provenance: Value) {
    let mut metadata = match properties.remove("metadata") {
        Some(Value::Object(metadata)) => metadata,
        Some(raw_metadata) => {
            let mut metadata = Map::new();
            metadata.insert("raw_metadata".to_string(), raw_metadata);
            metadata
        }
        None => Map::new(),
    };
    metadata.insert("source_provenance".to_string(), provenance);
    properties.insert("metadata".to_string(), Value::Object(metadata));
}

fn source_provenance(
    options: &SysmlJsonImportOptions,
    element_id: &str,
    element_type: &str,
    path: &str,
) -> Value {
    let mut provenance = Map::new();
    provenance.insert(
        "source_format".to_string(),
        json!("sysml-abstract-syntax-json"),
    );
    provenance.insert(
        "importer_version".to_string(),
        json!(SYSML_JSON_IMPORTER_VERSION),
    );
    provenance.insert("external_id".to_string(), json!(element_id));
    provenance.insert("external_type".to_string(), json!(element_type));
    provenance.insert("path".to_string(), json!(path));
    insert_optional(&mut provenance, "source_uri", options.source_uri.as_deref());
    insert_optional(&mut provenance, "base_url", options.base_url.as_deref());
    insert_optional(&mut provenance, "project_id", options.project_id.as_deref());
    insert_optional(
        &mut provenance,
        "project_name",
        options.project_name.as_deref(),
    );
    insert_optional(&mut provenance, "branch_id", options.branch_id.as_deref());
    insert_optional(
        &mut provenance,
        "branch_name",
        options.branch_name.as_deref(),
    );
    insert_optional(&mut provenance, "commit_id", options.commit_id.as_deref());
    insert_optional(
        &mut provenance,
        "schema_profile",
        options.schema_profile.as_deref(),
    );
    insert_optional(
        &mut provenance,
        "source_kind",
        options.source_kind.as_deref(),
    );
    Value::Object(provenance)
}

fn import_metadata(
    options: &SysmlJsonImportOptions,
    source_document_metadata: Option<Value>,
) -> BTreeMap<String, Value> {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        KIR_SCHEMA_VERSION_METADATA_KEY.to_string(),
        Value::String(KIR_SCHEMA_VERSION.to_string()),
    );
    metadata.insert(
        "source_format".to_string(),
        Value::String("sysml-abstract-syntax-json".to_string()),
    );
    metadata.insert(
        "importer_version".to_string(),
        Value::String(SYSML_JSON_IMPORTER_VERSION.to_string()),
    );
    insert_optional_btree(&mut metadata, "source_uri", options.source_uri.as_deref());
    insert_optional_btree(&mut metadata, "base_url", options.base_url.as_deref());
    insert_optional_btree(&mut metadata, "project_id", options.project_id.as_deref());
    insert_optional_btree(
        &mut metadata,
        "project_name",
        options.project_name.as_deref(),
    );
    insert_optional_btree(&mut metadata, "branch_id", options.branch_id.as_deref());
    insert_optional_btree(&mut metadata, "branch_name", options.branch_name.as_deref());
    insert_optional_btree(&mut metadata, "commit_id", options.commit_id.as_deref());
    insert_optional_btree(
        &mut metadata,
        "schema_profile",
        options.schema_profile.as_deref(),
    );
    insert_optional_btree(&mut metadata, "source_kind", options.source_kind.as_deref());
    if let Some(value) = source_document_metadata {
        metadata.insert("x_sysml_json_document".to_string(), normalize_value(value));
    }
    metadata
}

fn insert_optional(map: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        map.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn insert_optional_btree(map: &mut BTreeMap<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        map.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn diagnostic(
    severity: SysmlJsonImportSeverity,
    code: impl Into<String>,
    message: impl Into<String>,
    element_id: Option<String>,
    path: Option<String>,
) -> SysmlJsonImportDiagnostic {
    SysmlJsonImportDiagnostic {
        severity,
        code: code.into(),
        message: message.into(),
        element_id,
        path,
    }
}

fn extension_key(source_key: &str) -> String {
    let key = source_key
        .strip_prefix('@')
        .map(|key| format!("at_{key}"))
        .unwrap_or_else(|| source_key.to_string());
    let snake = camel_to_snake(&key);
    snake
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn camel_to_snake(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut previous_was_underscore = false;

    for (index, ch) in input.chars().enumerate() {
        if ch == '-' || ch == ' ' {
            if !previous_was_underscore && !output.is_empty() {
                output.push('_');
                previous_was_underscore = true;
            }
            continue;
        }

        if ch.is_ascii_uppercase() {
            if index > 0 && !previous_was_underscore {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
            previous_was_underscore = false;
        } else {
            output.push(ch);
            previous_was_underscore = ch == '_';
        }
    }

    output
}

fn snake_to_camel(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut uppercase_next = false;
    for ch in input.chars() {
        if ch == '_' {
            uppercase_next = true;
            continue;
        }
        if uppercase_next {
            output.push(ch.to_ascii_uppercase());
            uppercase_next = false;
        } else {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn containment_moves_update_imported_opposites_and_persist_atomically() {
        let mut report = import_sysml_api_elements(vec![
            json!({"@id":"p", "@type":"Package", "ownedRelationship":[{"@id":"a"},{"@id":"b"}]}),
            json!({"@id":"q", "@type":"Package", "ownedRelationship":[{"@id":"c"}]}),
            json!({"@id":"a", "@type":"OwningMembership", "owningRelatedElement":{"@id":"p"}, "ownedRelatedElement":[{"@id":"x"}]}),
            json!({"@id":"b", "@type":"OwningMembership", "owningRelatedElement":{"@id":"p"}}),
            json!({"@id":"c", "@type":"OwningMembership", "owningRelatedElement":{"@id":"q"}}),
            json!({"@id":"x", "@type":"Class", "owningRelationship":{"@id":"a"}}),
        ], Default::default()).unwrap();
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        report
            .move_contained_element("a", "q", "owned_relationship")
            .unwrap();
        let value = |report: &SysmlJsonImportReport, id: &str, field: &str| {
            report
                .document
                .elements
                .iter()
                .find(|e| e.id == id)
                .unwrap()
                .properties[field]
                .clone()
        };
        assert_eq!(value(&report, "p", "owned_relationship"), json!(["b"]));
        assert_eq!(value(&report, "q", "owned_relationship"), json!(["c", "a"]));
        assert_eq!(value(&report, "a", "owning_related_element"), json!("q"));
        let before = report.document.clone();
        report
            .move_contained_element("a", "q", "owned_relationship")
            .unwrap();
        assert_eq!(before, report.document);
        report
            .move_contained_element("x", "c", "owned_related_element")
            .unwrap();
        assert_eq!(value(&report, "a", "owned_related_element"), json!([]));
        assert_eq!(value(&report, "c", "owned_related_element"), json!(["x"]));
        assert_eq!(value(&report, "x", "owning_relationship"), json!("c"));
        // A Relationship can move between the two Ecore containment families.
        report
            .move_contained_element("a", "c", "owned_related_element")
            .unwrap();
        assert_eq!(value(&report, "q", "owned_relationship"), json!(["c"]));
        assert_eq!(
            value(&report, "c", "owned_related_element"),
            json!(["x", "a"])
        );
        let moved = report
            .document
            .elements
            .iter()
            .find(|e| e.id == "a")
            .unwrap();
        assert!(!moved.properties.contains_key("owning_related_element"));
        assert_eq!(moved.properties["owning_relationship"], json!("c"));
        report.persistable_document().unwrap();
        let before = report.document.clone();
        // q -> c -> x already exists: inserting c beneath x would cycle.
        assert!(
            report
                .move_contained_element("c", "x", "owned_relationship")
                .is_err()
        );
        assert_eq!(before, report.document);
        assert!(
            report
                .move_contained_element("x", "q", "owned_relationship")
                .is_err()
        );
        assert_eq!(before, report.document);
        assert!(
            report
                .move_contained_element("x", "missing", "owned_related_element")
                .is_err()
        );
        assert_eq!(before, report.document);
    }

    #[test]
    fn containment_moves_refuse_partial_ownership_and_stale_derived_values() {
        for child in [
            json!({"@id":"x","@type":"Class"}),
            json!({"@id":"x","@type":"Class","owningRelationship":{"@id":"external"}}),
        ] {
            let mut report = import_sysml_api_elements(vec![
                json!({"@id":"a","@type":"OwningMembership","ownedRelatedElement":[{"@id":"x"}]}),
                json!({"@id":"b","@type":"OwningMembership"}), child,
            ],Default::default()).unwrap();
            let before = report.document.clone();
            assert!(
                report
                    .move_contained_element("x", "b", "owned_related_element")
                    .is_err()
            );
            assert_eq!(before, report.document);
        }
        let mut report = import_sysml_api_elements(
            vec![
                json!({"@id":"a","@type":"OwningMembership"}),
                json!({"@id":"x","@type":"Class","isLibraryElement":false}),
            ],
            Default::default(),
        )
        .unwrap();
        let before = report.document.clone();
        assert!(
            report
                .move_contained_element("x", "a", "owned_related_element")
                .is_err()
        );
        assert_eq!(before, report.document);
    }

    #[test]
    fn required_feature_assessment_separates_missing_defaults_and_dependencies() {
        let mut report = import_sysml_api_elements(
            vec![
                json!({"@id":"comment", "@type":"Comment"}),
                json!({"@id":"part", "@type":"PartDefinition"}),
                json!({"@id":"usage", "@type":"PartUsage"}),
                json!({"@id":"package", "@type":"Package", "isLibraryElement":false}),
            ],
            Default::default(),
        )
        .unwrap();
        let before = report.document.clone();
        let diagnostics = report.required_feature_diagnostics();
        assert_eq!(before, report.document);
        let has = |id: &str, field: &str, code: &str| {
            diagnostics.iter().any(|d| {
                d.element_id.as_deref() == Some(id)
                    && d.path.as_deref() == Some(field)
                    && d.code == code
            })
        };
        assert!(has("comment", "body", "ecore.required.violation"));
        assert!(has("usage", "is_variable", "ecore.required.unverified"));
        // A serialized derived value does not qualify its defining algorithm.
        assert!(has(
            "package",
            "is_library_element",
            "ecore.required.unverified"
        ));
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.path.as_deref() == Some("element_id"))
        );
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.path.as_deref() == Some("is_variation"))
        );
        // Assessment reads the current model, independently of import-time diagnostics.
        let comment = report
            .document
            .elements
            .iter_mut()
            .find(|e| e.id == "comment")
            .unwrap();
        comment.properties.insert("body".into(), json!(""));
        assert!(
            !report
                .required_feature_diagnostics()
                .iter()
                .any(|d| d.element_id.as_deref() == Some("comment")
                    && d.path.as_deref() == Some("body"))
        );
        report
            .document
            .elements
            .iter_mut()
            .find(|e| e.id == "comment")
            .unwrap()
            .properties
            .insert("body".into(), Value::Null);
        assert!(
            report
                .required_feature_diagnostics()
                .iter()
                .any(|d| d.element_id.as_deref() == Some("comment")
                    && d.path.as_deref() == Some("body")
                    && d.code == "ecore.required.violation")
        );
    }

    #[test]
    fn required_reference_assessment_checks_bounds_types_and_resolution() {
        let mut report = import_sysml_api_elements(
            vec![
                json!({"@id":"dependency", "@type":"Dependency"}),
                json!({"@id":"package", "@type":"Package"}),
                json!({"@id":"typing", "@type":"FeatureTyping"}),
            ],
            Default::default(),
        )
        .unwrap();
        let missing = report.required_feature_diagnostics();
        for field in ["client", "supplier"] {
            assert!(
                missing
                    .iter()
                    .any(|d| d.element_id.as_deref() == Some("dependency")
                        && d.path.as_deref() == Some(field)
                        && d.code == "ecore.required.violation")
            );
        }
        let dependency = report
            .document
            .elements
            .iter_mut()
            .find(|e| e.id == "dependency")
            .unwrap();
        dependency
            .properties
            .insert("client".into(), json!(["package"]));
        dependency
            .properties
            .insert("supplier".into(), json!(["external"]));
        let typing = report
            .document
            .elements
            .iter_mut()
            .find(|e| e.id == "typing")
            .unwrap();
        typing.properties.insert("type".into(), json!("package"));
        let diagnostics = report.required_feature_diagnostics();
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.element_id.as_deref() == Some("dependency")
                    && d.path.as_deref() == Some("client"))
        );
        assert!(diagnostics.iter().any(
            |d| d.path.as_deref() == Some("supplier") && d.code == "ecore.required.unverified"
        ));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.element_id.as_deref() == Some("typing")
                    && d.path.as_deref() == Some("type")
                    && d.code == "ecore.required.violation")
        );
        report
            .document
            .elements
            .iter_mut()
            .find(|e| e.id == "dependency")
            .unwrap()
            .properties
            .insert("client".into(), json!([]));
        assert!(
            report
                .required_feature_diagnostics()
                .iter()
                .any(
                    |d| d.path.as_deref() == Some("client") && d.code == "ecore.required.violation"
                )
        );
    }

    #[test]
    fn ecore_attributes_roundtrip_with_inherited_and_kind_specific_shapes() {
        let input = vec![
            json!({"@id":"req", "@type":"SysML::ConcernUsage", "text":["second", "first"], "aliasIds":["z", "a"], "reqId":"R-1"}),
            json!({"@id":"issue", "@type":"Issue", "text":"scalar issue"}),
            json!({"@id":"member", "@type":"OwningMembership", "visibility":"protected"}),
            json!({"@id":"integer", "@type":"LiteralInteger", "value":2147483647}),
            json!({"@id":"boolean", "@type":"LiteralBoolean", "value":true}),
            json!({"@id":"string", "@type":"LiteralString", "value":"literal"}),
            json!({"@id":"rational", "@type":"LiteralRational", "value":1.25}),
        ];
        let imported = import_sysml_api_elements(input.clone(), Default::default()).unwrap();
        assert!(!imported.has_errors(), "{:?}", imported.diagnostics);
        assert_eq!(
            imported.document.elements[0].properties["text"],
            json!(["second", "first"])
        );
        assert_eq!(
            imported.document.elements[0].properties["alias_ids"],
            json!(["z", "a"])
        );
        assert_eq!(
            imported.document.elements[1].properties["text"],
            json!("scalar issue")
        );
        let persisted = imported.persistable_document().unwrap();
        let reloaded: KirDocument =
            serde_json::from_str(&serde_json::to_string(&persisted).unwrap()).unwrap();
        reloaded
            .validate_persisted_with_registry(sysml_json_field_registry(&reloaded))
            .unwrap();
        let exported = export_sysml_abstract_syntax_value(&reloaded, Default::default()).unwrap();
        assert!(!exported.has_errors(), "{:?}", exported.diagnostics);
        let roundtrip =
            import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(!roundtrip.has_errors());
        for (before, after) in imported
            .document
            .elements
            .iter()
            .zip(&roundtrip.document.elements)
        {
            assert_eq!(before.id, after.id);
            for (field, value) in &before.properties {
                if field != "metadata" {
                    assert_eq!(
                        after.properties.get(field),
                        Some(value),
                        "{}.{}",
                        before.id,
                        field
                    );
                }
            }
        }
    }

    #[test]
    fn invalid_ecore_attributes_are_reported_and_preserved_without_coercion() {
        for (kind, field, value) in [
            ("OwningMembership", "visibility", json!("expose")),
            ("PartUsage", "direction", json!("public")),
            ("PartUsage", "isUnique", json!("true")),
            ("LiteralInteger", "value", json!(2147483648_i64)),
            ("LiteralBoolean", "value", json!(1)),
            ("LiteralString", "value", json!({"@id":"not_a_string"})),
            ("Package", "aliasIds", json!(["duplicate", "duplicate"])),
            ("RequirementUsage", "text", json!("not_a_list")),
            ("Comment", "body", Value::Null),
        ] {
            let mut element = json!({"@id":"test", "@type":kind});
            element[field] = value.clone();
            let report = import_sysml_api_elements(vec![element], Default::default()).unwrap();
            assert!(report.has_errors(), "{kind}.{field}");
            assert_eq!(
                report.diagnostics[0].code,
                "sysml_json.property.ecore_value"
            );
            let properties = &report.document.elements[0].properties;
            assert!(!properties.contains_key(&camel_to_snake(field)));
            assert_eq!(properties["x_sysml_api"][extension_key(field)], value);
        }
    }

    #[test]
    fn invalid_ecore_export_values_produce_errors() {
        let document = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![KirElement {
                id: "member".into(),
                kind: "OwningMembership".into(),
                layer: 2,
                properties: BTreeMap::from([("visibility".into(), json!("wrong"))]),
            }],
        };
        let report = export_sysml_abstract_syntax_value(&document, Default::default()).unwrap();
        assert!(report.has_errors());
        assert_eq!(
            report.diagnostics[0].code,
            "sysml_json_export.property.ecore_value"
        );
    }

    #[test]
    fn ecore_reference_roundtrip_resolves_forward_ids_and_preserves_feature_identity() {
        let input = vec![
            json!({"@id":"pkg", "@type":"Package", "ownedRelationship":[{"@id":"m"}]}),
            json!({"@id":"m", "@type":"OwningMembership", "ownedRelatedElement":[{"@id":"p"}],
                "relatedElement":[{"@id":"pkg"},{"@id":"p"},{"@id":"p"}], "source":[{"@id":"pkg"}], "target":[{"@id":"p"}]}),
            json!({"@id":"p", "@type":"PartUsage", "type":[{"@id":"class"}]}),
            json!({"@id":"typing", "@type":"FeatureTyping", "type":{"@id":"class"}}),
            json!({"@id":"class", "@type":"PartDefinition"}),
            json!({"@id":"transition", "@type":"TransitionUsage", "source":{"@id":"action"}, "target":{"@id":"action"}}),
            json!({"@id":"action", "@type":"ActionUsage"}),
        ];
        let imported = import_sysml_api_elements(input, Default::default()).unwrap();
        assert!(
            imported.diagnostics.is_empty(),
            "{:?}",
            imported.diagnostics
        );
        let membership = &imported.document.elements[1].properties;
        assert_eq!(membership["owned_related_element"], json!(["p"]));
        assert_eq!(membership["related_element"], json!(["pkg", "p", "p"]));
        assert!(!membership.contains_key("related"));
        assert_eq!(
            imported.document.elements[2].properties["type"],
            json!(["class"])
        );
        assert_eq!(
            imported.document.elements[3].properties["type"],
            json!("class")
        );
        let persisted = imported.persistable_document().unwrap();
        let reloaded: KirDocument =
            serde_json::from_str(&serde_json::to_string(&persisted).unwrap()).unwrap();
        reloaded
            .validate_persisted_with_registry(sysml_json_field_registry(&reloaded))
            .unwrap();
        assert_eq!(reloaded.elements[2].properties["type"], json!(["class"]));
        assert_eq!(reloaded.elements[3].properties["type"], json!("class"));
        let mut invalid = reloaded.clone();
        invalid.elements[3]
            .properties
            .insert("type".into(), json!(["class"]));
        assert!(
            invalid
                .validate_persisted_with_registry(sysml_json_field_registry(&invalid))
                .is_err()
        );
        let exported = export_sysml_abstract_syntax_value(&reloaded, Default::default()).unwrap();
        assert!(
            exported.diagnostics.is_empty(),
            "{:?}",
            exported.diagnostics
        );
        let roundtrip =
            import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(
            roundtrip.diagnostics.is_empty(),
            "{:?}",
            roundtrip.diagnostics
        );
        for (before, after) in imported
            .document
            .elements
            .iter()
            .zip(&roundtrip.document.elements)
        {
            assert_eq!(before.id, after.id);
            for (field, value) in &before.properties {
                if field != "metadata" {
                    assert_eq!(
                        after.properties.get(field),
                        Some(value),
                        "{}.{}",
                        before.id,
                        field
                    );
                }
            }
        }
    }

    #[test]
    fn stored_feature_transactions_publish_and_round_trip_atomically() {
        let mut report = import_sysml_api_elements(vec![
            json!({"@id":"p", "@type":"Package", "declaredName":"Before", "aliasIds":["z","a"]}),
            json!({"@id":"c", "@type":"Class"}),
            json!({"@id":"f", "@type":"Feature"}),
            json!({"@id":"ft", "@type":"FeatureTyping", "type":{"@id":"c"}, "typedFeature":{"@id":"f"}}),
        ], Default::default()).unwrap();
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let edit = |id: &str, field: &str, value| SysmlStoredFeatureEdit {
            element_id: id.into(), field: field.into(), value,
        };
        let metadata = report.document.metadata.clone();
        report.apply_stored_feature_edits(&[
            edit("p", "declared_name", Some(json!("After"))),
            edit("p", "alias_ids", Some(json!(["a","z","b"]))),
            edit("f", "is_ordered", Some(json!(true))),
            edit("ft", "type", Some(json!("f"))),
        ]).unwrap();
        assert_eq!(report.document.metadata, metadata);
        assert!(report.publication_diagnostics(true).is_empty());
        let persisted = report.persistable_document().unwrap();
        let reloaded: KirDocument = serde_json::from_value(serde_json::to_value(&persisted).unwrap()).unwrap();
        assert_eq!(reloaded.elements.iter().find(|e| e.id == "p").unwrap().properties["alias_ids"], json!(["a","z","b"]));
        assert_eq!(reloaded.elements.iter().find(|e| e.id == "f").unwrap().properties["is_ordered"], json!(true));
        let exported = export_sysml_abstract_syntax_value(&reloaded, Default::default()).unwrap();
        assert!(!exported.has_errors(), "{:?}", exported.diagnostics);
        let imported = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(!imported.has_errors());
        assert!(imported.publication_diagnostics(true).is_empty());
        assert_eq!(imported.document.elements.iter().find(|e| e.id == "p").unwrap().properties["declared_name"], json!("After"));
        report.apply_stored_feature_edits(&[edit("p", "declared_name", Some(Value::Null))]).unwrap();
        assert_eq!(report.document.elements[0].properties["declared_name"], Value::Null);
        report.apply_stored_feature_edits(&[edit("p", "declared_name", None)]).unwrap();
        assert!(!report.document.elements[0].properties.contains_key("declared_name"));
    }

    #[test]
    fn stored_feature_transactions_reject_unsupported_or_invalid_batches_without_changes() {
        let original = import_sysml_api_elements(vec![
            json!({"@id":"p", "@type":"Package", "declaredName":"Before"}),
            json!({"@id":"c", "@type":"Class"}),
            json!({"@id":"ft", "@type":"FeatureTyping", "type":{"@id":"c"}}),
            json!({"@id":"m", "@type":"Membership"}),
        ], Default::default()).unwrap();
        let edit = |id: &str, field: &str, value| SysmlStoredFeatureEdit {
            element_id: id.into(), field: field.into(), value,
        };
        for invalid in [
            edit("p", "alias_ids", Some(json!(["duplicate", "duplicate"]))),
            edit("p", "owned_relationship", Some(json!([]))),
            edit("p", "membership", Some(json!([]))),
            edit("ft", "general", Some(json!("c"))),
            edit("p", "element_id", Some(json!("renamed"))),
            edit("p", "unknown_field", Some(json!(true))),
            edit("ft", "type", Some(json!("absent"))),
            edit("ft", "type", Some(json!("m"))),
            edit("ft", "type", None),
            edit("missing", "declared_name", Some(json!("New"))),
            edit("c", "is_abstract", Some(json!("not boolean"))),
        ] {
            let mut report = original.clone();
            let before = serde_json::to_value(&report).unwrap();
            assert!(report.apply_stored_feature_edits(&[
                edit("p", "declared_name", Some(json!("Must roll back"))), invalid,
            ]).is_err());
            assert_eq!(serde_json::to_value(&report).unwrap(), before);
        }
        let mut report = original.clone();
        assert!(report.apply_stored_feature_edits(&[
            edit("p", "declared_name", Some(json!("One"))),
            edit("p", "declared_name", Some(json!("Two"))),
        ]).is_err());
        assert_eq!(report, original);
        report.document.elements[0].properties.insert("name".into(), json!("Before"));
        let before = report.clone();
        assert!(report.apply_stored_feature_edits(&[edit("p", "declared_name", Some(json!("Stale")))]).is_err());
        assert_eq!(report, before);
        // Read-only publication and a true no-op preserve snapshots; they do not
        // claim to implement their derivation algorithms.
        assert!(report.publication_diagnostics(true).is_empty());
        report.apply_stored_feature_edits(&[edit("p", "declared_name", Some(json!("Before")))]).unwrap();
        assert_eq!(report, before);
    }

    #[test]
    fn publication_separates_partial_external_graphs_and_preserves_snapshots() {
        let mut report = import_sysml_api_elements(vec![
            json!({"@id":"ft", "@type":"FeatureTyping", "type":{"@id":"external"}}),
        ], Default::default()).unwrap();
        let before = report.clone();
        assert!(!report.has_errors());
        assert!(report.publication_diagnostics(false).is_empty());
        assert_eq!(report.publication_diagnostics(true).len(), 1);
        assert!(report.persistable_document().is_ok());
        assert_eq!(report, before);
        report.document.elements.push(report.document.elements[0].clone());
        assert!(report.publication_diagnostics(false).iter().any(|d| d.message.contains("unique nonempty")));
        assert!(report.persistable_document().is_err());
        let contradictory = import_sysml_api_elements(vec![
            json!({"@id":"n", "@type":"Package", "ownedMembership":[], "importedMembership":[], "membership":[{"@id":"m"}]}),
            json!({"@id":"m", "@type":"Membership"}),
        ], Default::default()).unwrap();
        assert!(!contradictory.has_errors());
        let before = contradictory.clone();
        assert!(contradictory.publication_diagnostics(false).iter().any(|d| d.message.contains("Explicit union")));
        assert!(contradictory.persistable_document().is_err());
        assert_eq!(contradictory, before);
    }

    #[test]
    fn ecore_persistence_rechecks_current_model_and_refuses_import_errors() {
        let mut report = import_sysml_api_elements(
            vec![json!({
                "@id":"m", "@type":"OwningMembership", "visibility":"private"
            })],
            Default::default(),
        )
        .unwrap();
        report.document.elements[0]
            .properties
            .insert("visibility".into(), json!("invalid"));
        assert!(report.persistable_document().is_err());
        let invalid = import_sysml_api_elements(
            vec![json!({
                "@id":"m", "@type":"OwningMembership", "visibility":"invalid"
            })],
            Default::default(),
        )
        .unwrap();
        assert!(invalid.persistable_document().is_err());
    }

    #[test]
    fn ecore_persistence_rejects_contradictory_subset_snapshots() {
        let mut report = import_sysml_api_elements(vec![
            json!({"@id":"r", "@type":"Relationship", "source":[{"@id":"a"}],
                   "relatedElement":[{"@id":"a"},{"@id":"b"}]}),
            json!({"@id":"a", "@type":"Class"}),
            json!({"@id":"b", "@type":"Class"}),
        ], Default::default()).unwrap();
        assert!(report.persistable_document().is_ok());
        let relationship = report.document.elements.iter_mut().find(|e| e.id == "r").unwrap();
        relationship.properties.insert("related_element".into(), json!(["b"]));
        let before = report.document.clone();
        let error = report.persistable_document().unwrap_err();
        assert!(format!("{error:?}").contains("Explicit subset"));
        assert_eq!(serde_json::to_value(&report.document).unwrap(), serde_json::to_value(&before).unwrap());
        report.document.elements.iter_mut().find(|e| e.id == "r").unwrap()
            .properties.remove("related_element");
        assert!(report.persistable_document().is_ok());
    }

    #[test]
    fn ecore_persistence_checks_union_completeness_without_inventing_inputs() {
        let mut report = import_sysml_api_elements(vec![
            json!({"@id":"n", "@type":"Package", "ownedMembership":[],
                   "importedMembership":[], "membership":[{"@id":"m"}]}),
            json!({"@id":"m", "@type":"Membership"}),
        ], Default::default()).unwrap();
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let error = report.persistable_document().unwrap_err();
        assert!(format!("{error:?}").contains("Explicit union"));
        report.document.elements.iter_mut().find(|e| e.id == "n").unwrap()
            .properties.insert("imported_membership".into(), json!(["m"]));
        assert!(report.persistable_document().is_ok());
        report.document.elements.iter_mut().find(|e| e.id == "n").unwrap()
            .properties.remove("imported_membership");
        assert!(report.persistable_document().is_ok());
    }

    #[test]
    fn explicit_union_evaluation_is_ordered_typed_and_non_mutating() {
        let mut report = import_sysml_api_elements(vec![
            json!({"@id":"n", "@type":"Class", "importedMembership":[{"@id":"b"},{"@id":"a"}],
                   "ownedMembership":[{"@id":"a"}], "inheritedMembership":[{"@id":"c"},{"@id":"b"}]}),
            json!({"@id":"a", "@type":"Membership"}),
            json!({"@id":"b", "@type":"Membership"}),
            json!({"@id":"c", "@type":"Membership"}),
        ], Default::default()).unwrap();
        let before = serde_json::to_value(&report.document).unwrap();
        assert_eq!(report.evaluate_explicit_union("n", "membership").unwrap(), json!(["b", "a", "c"]));
        assert_eq!(serde_json::to_value(&report.document).unwrap(), before);
        report.document.elements.iter_mut().find(|e| e.id == "n").unwrap()
            .properties.insert("membership".into(), json!(["stale"]));
        assert_eq!(report.evaluate_explicit_union("n", "membership").unwrap(), json!(["b", "a", "c"]));
        let mut malformed = report.clone();
        malformed.document.elements.iter_mut().find(|e| e.id == "n").unwrap()
            .properties.insert("owned_membership".into(), json!("a"));
        assert!(malformed.evaluate_explicit_union("n", "membership").is_err());
        let mut duplicate = report.clone();
        duplicate.document.elements.push(duplicate.document.elements[1].clone());
        assert!(format!("{:?}", duplicate.evaluate_explicit_union("n", "membership").unwrap_err()).contains("Duplicate model identity"));
        assert!(report.evaluate_explicit_union("n", "member").is_err());
        assert!(report.evaluate_explicit_union("missing", "membership").is_err());
        report.document.elements.iter_mut().find(|e| e.id == "c").unwrap().kind = "SysML::Class".into();
        assert!(report.evaluate_explicit_union("n", "membership").is_err());
        report.document.elements.retain(|e| e.id != "c");
        assert!(format!("{:?}", report.evaluate_explicit_union("n", "membership").unwrap_err()).contains("resolved target"));
        report.document.elements.iter_mut().find(|e| e.id == "n").unwrap().properties.remove("inherited_membership");
        assert!(format!("{:?}", report.evaluate_explicit_union("n", "membership").unwrap_err()).contains("resolved input"));
    }

    #[test]
    fn union_evaluation_derives_canonical_owned_memberships() {
        let mut report = import_sysml_api_elements(vec![
            json!({"@id":"n", "@type":"Package", "ownedRelationship":[{"@id":"b"},{"@id":"r"},{"@id":"a"}]}),
            json!({"@id":"a", "@type":"Membership", "owningRelatedElement":{"@id":"n"}}),
            json!({"@id":"b", "@type":"OwningMembership", "owningRelatedElement":{"@id":"n"}, "visibility":"private"}),
            json!({"@id":"r", "@type":"Relationship", "owningRelatedElement":{"@id":"n"}}),
        ], Default::default()).unwrap();
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let before = serde_json::to_value(&report.document).unwrap();
        assert_eq!(report.evaluate_explicit_union("n", "membership").unwrap(), json!(["b", "a"]));
        assert_eq!(serde_json::to_value(&report.document).unwrap(), before);
        report.document.elements.reverse();
        assert_eq!(report.evaluate_explicit_union("n", "membership").unwrap(), json!(["b", "a"]));
        let namespace = report.document.elements.iter_mut().find(|e| e.id == "n").unwrap();
        namespace.properties.insert("owned_membership".into(), json!(["a", "b"]));
        assert!(format!("{:?}", report.evaluate_explicit_union("n", "membership").unwrap_err()).contains("snapshot disagrees"));
        report.document.elements.iter_mut().find(|e| e.id == "n").unwrap().properties.remove("owned_membership");
        report.document.elements.iter_mut().find(|e| e.id == "r").unwrap().kind = "SysML::NamespaceImport".into();
        assert!(format!("{:?}", report.evaluate_explicit_union("n", "membership").unwrap_err()).contains("resolved endpoint"));
        report.document.elements.iter_mut().find(|e| e.id == "r").unwrap().kind = "SysML::Relationship".into();
        report.document.elements.iter_mut().find(|e| e.id == "a").unwrap().properties.remove("owning_related_element");
        assert!(report.evaluate_explicit_union("n", "membership").is_err());
    }

    #[test]
    fn union_evaluation_derives_ordered_package_imports() {
        let mut report = import_sysml_api_elements(vec![
            json!({"@id":"n", "@type":"Package", "ownedRelationship":[{"@id":"i"},{"@id":"j"}]}),
            json!({"@id":"i", "@type":"NamespaceImport", "owningRelatedElement":{"@id":"n"},
                   "importedNamespace":{"@id":"s"}, "visibility":"private"}),
            json!({"@id":"j", "@type":"MembershipImport", "owningRelatedElement":{"@id":"n"},
                   "importedMembership":{"@id":"z"}}),
            json!({"@id":"s", "@type":"Package", "ownedRelationship":[{"@id":"z"},{"@id":"a"}]}),
            json!({"@id":"z", "@type":"Membership", "owningRelatedElement":{"@id":"s"},
                   "memberName":"Z", "memberElement":{"@id":"target"}}),
            json!({"@id":"a", "@type":"Membership", "owningRelatedElement":{"@id":"s"},
                   "memberName":"A", "memberElement":{"@id":"target"}, "visibility":"private"}),
            json!({"@id":"target", "@type":"Class"}),
        ], Default::default()).unwrap();
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert_eq!(report.evaluate_explicit_union("n", "membership").unwrap(), json!(["z"]));
        report.document.elements.iter_mut().find(|e| e.id == "i").unwrap().properties.insert("is_import_all".into(), json!(true));
        // Source declaration order, not sorted identity; duplicate imports keep one membership.
        assert_eq!(report.evaluate_explicit_union("n", "membership").unwrap(), json!(["z", "a"]));
        report.document.elements.reverse();
        assert_eq!(report.evaluate_explicit_union("n", "membership").unwrap(), json!(["z", "a"]));
        report.document.elements.iter_mut().find(|e| e.id == "n").unwrap().properties.insert("imported_membership".into(), json!(["a", "z"]));
        assert!(format!("{:?}", report.evaluate_explicit_union("n", "membership").unwrap_err()).contains("snapshot disagrees"));
        report.document.elements.iter_mut().find(|e| e.id == "n").unwrap().properties.remove("imported_membership");
        report.document.elements.push(KirElement { id: "local".into(), kind: "SysML::Membership".into(), layer: 0,
            properties: BTreeMap::from([
                ("owning_related_element".into(), json!("n")),
                ("member_element".into(), json!("target")),
                ("member_name".into(), json!("Local")),
                ("member_short_name".into(), json!("Z")),
            ]) });
        report.document.elements.iter_mut().find(|e| e.id == "n").unwrap().properties.get_mut("owned_relationship").unwrap()
            .as_array_mut().unwrap().push(json!("local"));
        assert_eq!(report.evaluate_explicit_union("n", "membership").unwrap(), json!(["a", "local"]));
        report.document.elements.iter_mut().find(|e| e.id == "s").unwrap().properties.remove("owned_relationship");
        assert!(format!("{:?}", report.evaluate_explicit_union("n", "membership").unwrap_err()).contains("complete canonical"));
    }

    #[test]
    fn canonical_package_union_derivation_matches_pilot_runtime() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/union-pilot-controls.json"
        )).unwrap();
        let controls = evidence["canonical_controls"].as_array().unwrap();
        assert_eq!(controls.len(), 27);
        for control in controls {
            let mut report = import_sysml_api_elements(control["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
            assert!(!report.has_errors(), "{:?}", report.diagnostics);
            let root = control["root"].as_str().unwrap();
            let before = serde_json::to_value(&report.document).unwrap();
            assert_eq!(report.evaluate_explicit_union(root, "membership").unwrap(), control["membership"], "case {}", control["case"]);
            assert_eq!(serde_json::to_value(&report.document).unwrap(), before);
            if control["case"].as_u64().unwrap() >= 19 {
                assert_eq!(report.evaluate_explicit_union("source", "membership").unwrap(), control["source_membership"]);
            }
            // Reconcile independently observed subsets against native derivation.
            let namespace = report.document.elements.iter_mut().find(|e| e.id == root).unwrap();
            namespace.properties.insert("owned_membership".into(), control["owned_membership"].clone());
            namespace.properties.insert("imported_membership".into(), control["imported_membership"].clone());
            assert_eq!(report.evaluate_explicit_union(root, "membership").unwrap(), control["membership"]);
            report.document.elements.reverse();
            assert_eq!(report.evaluate_explicit_union(root, "membership").unwrap(), control["membership"]);
        }
    }

    #[test]
    fn feature_chaining_inheritance_matches_pilot_without_derived_snapshots() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/union-pilot-controls.json"
        )).unwrap();
        let controls = evidence["chaining_controls"].as_array().unwrap();
        assert_eq!(controls.len(), 9);
        for control in controls {
            let mut report = import_sysml_api_elements(control["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
            assert!(!report.has_errors(), "{:?}", report.diagnostics);
            assert!(report.document.elements.iter().all(|e| !e.properties.contains_key("inherited_membership")));
            let before = serde_json::to_value(&report.document).unwrap();
            for (root, expected) in [("source", "source_membership"), ("destination", "membership")] {
                assert_eq!(report.evaluate_explicit_union(root, "membership").unwrap(), control[expected], "case {}", control["case"]);
            }
            assert_eq!(serde_json::to_value(&report.document).unwrap(), before);
            report.document.elements.reverse();
            assert_eq!(report.evaluate_explicit_union("destination", "membership").unwrap(), control["membership"]);
        }
    }

    #[test]
    fn feature_chaining_inheritance_rejects_incomplete_or_inconsistent_endpoints() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/union-pilot-controls.json"
        )).unwrap();
        let clean = import_sysml_api_elements(evidence["chaining_controls"][0]["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
        for (id, field, value) in [
            ("chain0", "chaining_feature", json!("missing")),
            ("chain1", "chaining_feature", json!("target")),
            ("chain1", "chaining_feature", json!([])),
            ("chain1", "owning_related_element", json!("first")),
            ("chain1", "feature_chained", json!("first")),
            ("last", "is_implied_included", json!(false)),
        ] {
            let mut report = clean.clone();
            report.document.elements.iter_mut().find(|e| e.id == id).unwrap().properties.insert(field.into(), value);
            let before = serde_json::to_value(&report.document).unwrap();
            assert!(report.evaluate_explicit_union("destination", "membership").is_err(), "{id}.{field}");
            assert_eq!(serde_json::to_value(&report.document).unwrap(), before);
        }
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "source").unwrap().properties.insert("owned_relationship".into(), json!(["chain1", "chain0"]));
        assert_eq!(report.evaluate_explicit_union("source", "membership").unwrap(), json!(["First"]));
        report.document.elements.iter_mut().find(|e| e.id == "first").unwrap().properties.remove("owned_relationship");
        assert!(report.evaluate_explicit_union("source", "membership").is_err());
    }

    #[test]
    fn type_import_scope_requires_valid_resolved_inherited_memberships() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/union-pilot-controls.json"
        )).unwrap();
        let control = &evidence["canonical_controls"][5];
        let mut report = import_sysml_api_elements(control["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert_eq!(report.evaluate_explicit_union("destination", "membership").unwrap(), control["membership"]);
        report.document.elements.iter_mut().find(|e| e.id == "a").unwrap().properties.insert("visibility".into(), json!("private"));
        assert_eq!(report.evaluate_explicit_union("destination", "membership").unwrap(), control["membership"]);
        report.document.elements.iter_mut().find(|e| e.id == "a").unwrap().properties.insert("visibility".into(), json!("protected"));
        report.document.elements.iter_mut().find(|e| e.id == "source").unwrap().properties.insert("inherited_membership".into(), json!(["target"]));
        assert!(report.evaluate_explicit_union("destination", "membership").is_err());
        report.document.elements.iter_mut().find(|e| e.id == "source").unwrap().properties.remove("inherited_membership");
        assert!(format!("{:?}", report.evaluate_explicit_union("destination", "membership").unwrap_err()).contains("resolved inherited_membership"));
        // A complete full-inheritance snapshot cannot stand in for the different
        // exclude-implied semantics required by a recursive type import.
        report.document.elements.iter_mut().find(|e| e.id == "source").unwrap().properties.insert("inherited_membership".into(), json!(["z", "a"]));
        report.document.elements.iter_mut().find(|e| e.id == "import").unwrap().properties.insert("is_recursive".into(), json!(true));
        assert_eq!(report.evaluate_explicit_union("destination", "membership").unwrap(), control["membership"]);
    }

    #[test]
    fn recursive_type_import_derives_explicit_inheritance_and_rejects_missing_dependencies() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/union-pilot-controls.json"
        )).unwrap();
        let control = &evidence["canonical_controls"][12];
        let clean = import_sysml_api_elements(control["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
        assert!(!clean.has_errors(), "{:?}", clean.diagnostics);
        assert!(clean.document.elements.iter().all(|e| !e.properties.contains_key("inherited_membership")));
        assert_eq!(clean.evaluate_explicit_union("destination", "membership").unwrap(), control["membership"]);
        // Snapshot corruption does not override exclude-implied derivation.
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "source").unwrap().properties.insert("inherited_membership".into(), json!(["local"]));
        assert_eq!(report.evaluate_explicit_union("destination", "membership").unwrap(), control["membership"]);
        for (id, field, value, message) in [
            ("specialization", "superclassifier", json!("absent"), ""),
            ("specialization", "superclassifier", json!("local"), ""),
            ("specialization", "subclassifier", json!("base"), "specific endpoint"),
            ("specialization", "is_implied", json!("true"), "invalid Ecore value"),
            ("base", "owned_relationship", Value::Null, "containment list"),
        ] {
            let mut report = clean.clone();
            report.document.elements.iter_mut().find(|e| e.id == id).unwrap().properties.insert(field.into(), value);
            let failure = format!("{:?}", report.evaluate_explicit_union("destination", "membership").unwrap_err());
            assert!(failure.contains(message), "{id}.{field}: {failure}");
        }
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "base").unwrap().properties.remove("owned_relationship");
        assert!(report.evaluate_explicit_union("destination", "membership").is_err());
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "target").unwrap().kind = "Feature".into();
        // Ordinary Feature filtering is now computed. These two memberships
        // alias one Feature, so the normative membership predicate removes both
        // inherited aliases (the separate Pilot disagreement remains recorded).
        assert_eq!(report.evaluate_explicit_union("destination", "membership").unwrap(), json!(["local"]));
        report.document.elements.iter_mut().find(|e| e.id == "target").unwrap().kind = "ReferenceUsage".into();
        assert!(format!("{:?}", report.evaluate_explicit_union("destination", "membership").unwrap_err()).contains("unsupported adapter strategy"));
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "specialization").unwrap().properties.insert("is_implied".into(), json!(true));
        assert_eq!(report.evaluate_explicit_union("destination", "membership").unwrap(), json!(["local"]));
        // The normative Type.supertypes(true) rejects stored implied edges.
        // Pinned Pilot still traverses them; preserve this independent disagreement.
        let disagreement = &evidence["implied_disagreement_control"];
        assert_eq!(disagreement["membership"], json!(["z", "a", "local"]));
        let report = import_sysml_api_elements(disagreement["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
        assert!(!report.has_errors());
        assert_eq!(report.evaluate_explicit_union("destination", "membership").unwrap(), json!(["local"]));
    }

    #[test]
    fn materialized_inheritance_requires_complete_typed_redefinitions_and_reconciles_snapshots() {
        let oracle: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/union-pilot-controls.json"
        )).unwrap();
        let control = oracle["canonical_controls"].as_array().unwrap().iter().find(|c| c["case"] == 23).unwrap();
        let clean = import_sysml_api_elements(control["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
        assert!(!clean.has_errors(), "{:?}", clean.diagnostics);
        assert!(clean.document.elements.iter().all(|e| !e.properties.contains_key("inherited_membership")));
        assert_eq!(clean.evaluate_explicit_union("source", "membership").unwrap(), control["source_membership"]);
        for (id, field, value) in [
            ("source", "is_implied_included", json!(false)),
            ("base", "is_implied_included", json!(false)),
            ("redefine", "redefined_feature", json!("absent")),
            ("redefine", "redefined_feature", json!("base")),
            ("redefine", "redefining_feature", json!("fa")),
            ("source", "inherited_membership", json!(["z"])),
        ] {
            let mut report = clean.clone();
            report.document.elements.iter_mut().find(|e| e.id == id).unwrap().properties.insert(field.into(), value);
            let before = serde_json::to_value(&report.document).unwrap();
            assert!(report.evaluate_explicit_union("source", "membership").is_err(), "{id}.{field}");
            assert_eq!(serde_json::to_value(&report.document).unwrap(), before);
        }
        // Ordinary Feature redefinitions now compute their empty implicit
        // contribution; no blanket materialization assertion is needed on them.
        let mut computed = clean.clone();
        for id in ["fz", "fa", "own"] {
            computed.document.elements.iter_mut().find(|e| e.id == id).unwrap().properties.insert("is_implied_included".into(), json!(false));
            assert_eq!(computed.evaluate_explicit_union("source", "membership").unwrap(), control["source_membership"]);
        }
        let before = serde_json::to_value(&computed.document).unwrap();
        computed.document.elements.reverse();
        assert_eq!(computed.evaluate_explicit_union("source", "membership").unwrap(), control["source_membership"]);
        computed.document.elements.reverse();
        assert_eq!(serde_json::to_value(&computed.document).unwrap(), before);
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "fa").unwrap().properties.remove("owned_relationship");
        assert!(report.evaluate_explicit_union("source", "membership").is_err());
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "source").unwrap().properties.insert("inherited_membership".into(), json!([]));
        assert_eq!(report.evaluate_explicit_union("source", "membership").unwrap(), control["source_membership"]);
    }

    #[test]
    fn materialized_feature_alias_filter_follows_normative_membership_identity() {
        let oracle: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/union-pilot-controls.json"
        )).unwrap();
        let control = &oracle["redefinition_alias_disagreement_control"];
        assert_eq!(control["membership"], json!(["z", "a", "local"]));
        let report = import_sysml_api_elements(control["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
        assert!(!report.has_errors());
        // allRedefinedFeaturesOf includes the Feature itself. The normative
        // predicate compares another Membership, even for aliases of one Feature.
        assert_eq!(report.evaluate_explicit_union("destination", "membership").unwrap(), json!(["local"]));
    }

    #[test]
    fn plain_type_defaults_match_independent_pilot_generalization_controls() {
        let oracle: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/default-general-pilot-controls.json"
        )).unwrap();
        let controls = oracle["controls"].as_array().unwrap();
        assert_eq!(controls.len(), 98);
        for control in controls {
            let mut report = import_sysml_api_elements(control["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
            assert!(!report.has_errors(), "{:?}", report.diagnostics);
            let bindings: BTreeMap<String,String> = serde_json::from_value(control["library_bindings"].clone()).unwrap();
            let before = serde_json::to_value(&report.document).unwrap();
            let actual = report.evaluate_plain_type_generalizations("owner", &bindings).unwrap();
            assert_eq!(serde_json::to_value(actual).unwrap(), control["general_types"], "{} mode {}",control["kind"],control["mode"]);
            assert_eq!(report.evaluate_plain_type_membership("owner", &bindings).unwrap(), control["membership"]);
            assert_eq!(serde_json::to_value(&report.document).unwrap(), before);
            let snapshot=serde_json::to_value(&report.document).unwrap();
            report.document=serde_json::from_value(snapshot).unwrap();
            assert!(!report.document.elements.iter().find(|e|e.id=="owner").unwrap().properties.get("is_implied_included").is_some_and(|v|v==&json!(true)));
            report.document.elements.reverse();
            assert_eq!(report.evaluate_plain_type_membership("owner", &bindings).unwrap(), control["membership"]);
            assert_eq!(serde_json::to_value(report.evaluate_plain_type_generalizations("owner", &bindings).unwrap()).unwrap(), control["general_types"]);
        }
    }

    #[test]
    fn plain_type_shared_strategy_contributions_replay_all_bindings() {
        let oracle:Value=serde_json::from_str(include_str!("../resources/metamodels/sysml-2.0-pilot-2026-08/default-general-pilot-controls.json")).unwrap();
        assert_eq!(oracle["bindings"].as_array().unwrap().len(),14);
        for control in oracle["controls"].as_array().unwrap() {
            let mut graph=plain_default_graph_with_standard_library(control);
            let (qualified,target)=control["library_bindings"].as_object().unwrap().iter().next().unwrap();
            let target=target.as_str().unwrap();
            // Keep the explicit lookup alias from Pilot's supplied provider.
            // Canonical containment is independent of derived effective names
            // (notably ConjugatedPortDefinition's tilde-prefixed name).
            let library=graph.iter_mut().find(|e|e["@id"]=="opaque-library").unwrap();
            library["ownedRelationship"].as_array_mut().unwrap().push(json!({"@id":"library-owner"}));
            graph.push(json!({"@id":"library-owner","@type":"OwningMembership","owningRelatedElement":{"@id":"opaque-library"},"ownedRelatedElement":[{"@id":target}]}));
            let endpoint=graph.iter_mut().find(|e|e["@id"]==target).unwrap();
            endpoint["owningRelationship"]=json!({"@id":"library-owner"});
            endpoint["declaredName"]=json!(qualified.split_once("::").unwrap().1);
            let mut report=import_sysml_api_elements(graph,Default::default()).unwrap();
            assert!(!report.has_errors(),"{:?}",report.diagnostics);
            let bindings:BTreeMap<String,String>=serde_json::from_value(control["library_bindings"].clone()).unwrap();
            let before=report.evaluate_plain_type_generalizations("owner",&bindings).unwrap();
            let inserted=crate::definition_document::materialize_type_defaults(&mut report.document,&[("owner","contribution")]).unwrap();
            assert!(inserted<=1,"{control}");
            let actual=report.evaluate_plain_type_generalizations("owner",&bindings).unwrap_or_else(|e|panic!("{control}: {e:?}"));
            let mut expected=before.clone();
            // Pilot's explicit partial-state controls retain a stored and a
            // recomputed copy. Physical insertion is not full transformation.
            if inserted>0 {
                let partial=oracle["controls"].as_array().unwrap().iter().find(|c|c["kind"]==control["kind"] && c["mode"]==6).unwrap();
                assert_eq!(partial["general_types"],json!(["default","default"]));
                expected.push(bindings.values().next().unwrap().clone());
            }
            assert_eq!(actual,expected,"{control}");
            assert_eq!(report.evaluate_plain_type_membership("owner",&bindings).unwrap(),control["membership"],"{control}");
            assert!(!report.document.elements.iter().find(|e|e.id=="owner").unwrap().properties.get("is_implied_included").is_some_and(|v|v==&json!(true)));
            let snapshot=serde_json::to_value(&report.document).unwrap();
            assert_eq!(crate::definition_document::materialize_type_defaults(&mut report.document,&[("owner","retry")]).unwrap(),0);
            assert_eq!(serde_json::to_value(&report.document).unwrap(),snapshot);
            report.document=serde_json::from_value(snapshot).unwrap();report.document.elements.reverse();
            assert_eq!(crate::definition_document::materialize_type_defaults(&mut report.document,&[("owner","reload")]).unwrap(),0);
            assert_eq!(report.evaluate_plain_type_membership("owner",&bindings).unwrap(),control["membership"]);
        }
    }

    #[test]
    fn plain_type_default_dependencies_are_explicit_and_non_mutating() {
        let oracle: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/default-general-pilot-controls.json"
        )).unwrap();
        let control = oracle["controls"].as_array().unwrap().iter().find(|c|c["kind"]=="Class" && c["mode"]==1).unwrap(); // Class with an explicit general.
        let clean = import_sysml_api_elements(control["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
        let bindings: BTreeMap<String,String> = serde_json::from_value(control["library_bindings"].clone()).unwrap();
        assert!(clean.evaluate_plain_type_generalizations("owner", &BTreeMap::new()).is_err());
        let before = serde_json::to_value(&clean.document).unwrap();
        assert!(clean.evaluate_plain_type_membership("owner", &BTreeMap::new()).is_err());
        assert_eq!(serde_json::to_value(&clean.document).unwrap(), before);
        // Private staging must not collide with a pre-existing graph identity.
        let mut collision = clean.clone();
        let mut extra = collision.document.elements.iter().find(|e| e.id == "own").unwrap().clone();
        extra.id = "__native_default_general_0".into();
        extra.properties.clear();
        collision.document.elements.push(extra);
        let before = serde_json::to_value(&collision.document).unwrap();
        assert_eq!(collision.evaluate_plain_type_membership("owner", &bindings).unwrap(), control["membership"]);
        assert_eq!(serde_json::to_value(&collision.document).unwrap(), before);
        for target in ["missing", "specialization"] {
            let bad = bindings.keys().map(|k| (k.clone(),target.into())).collect();
            assert!(clean.evaluate_plain_type_generalizations("owner", &bad).is_err());
        }
        for (id, field, value) in [
            ("owner", "owned_relationship", Value::Null),
            ("owner", "is_implied_included", json!("true")),
            ("specialization", "subclassifier", json!("explicit")),
            ("specialization", "is_implied", json!(true)),
        ] {
            let mut report = clean.clone();
            report.document.elements.iter_mut().find(|e| e.id == id).unwrap().properties.insert(field.into(), value);
            let before = serde_json::to_value(&report.document).unwrap();
            assert!(report.evaluate_plain_type_generalizations("owner", &bindings).is_err());
            assert_eq!(serde_json::to_value(&report.document).unwrap(), before);
        }
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "owner").unwrap().properties.insert("is_implied_included".into(), json!(true));
        assert_eq!(report.evaluate_plain_type_generalizations("owner", &BTreeMap::new()).unwrap(), vec!["explicit"]);
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "default").unwrap().kind = "DataType".into();
        assert!(report.evaluate_plain_type_generalizations("owner", &bindings).is_err());
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "default").unwrap().kind = "MetadataFeature".into();
        assert!(report.evaluate_plain_type_generalizations("owner", &bindings).is_err()); // Wrong-category endpoint; owner-specific metadata assessment is separate.
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "owner").unwrap().kind = "PartDefinition".into();
        assert!(report.evaluate_plain_type_generalizations("owner", &bindings).is_err());
    }

    fn plain_default_graph_with_standard_library(control: &Value) -> Vec<Value> {
        let mut graph = control["graph"].as_array().unwrap().clone();
        let (name, target) = control["library_bindings"].as_object().unwrap().iter().next().unwrap();
        let (root, member) = name.split_once("::").unwrap();
        graph.push(json!({"@id":"opaque-library", "@type":"LibraryPackage", "declaredName":root,
            "isStandard":true, "ownedRelationship":[{"@id":"library-member"}]}));
        graph.push(json!({"@id":"library-member", "@type":"Membership", "memberName":member,
            "memberElement":{"@id":target}, "owningRelatedElement":{"@id":"opaque-library"}}));
        graph
    }

    #[test]
    fn standard_library_memberships_drive_plain_type_defaults_without_id_spelling() {
        let oracle: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/default-general-pilot-controls.json"
        )).unwrap();
        for control in oracle["controls"].as_array().unwrap() {
            let mut report = import_sysml_api_elements(plain_default_graph_with_standard_library(control), Default::default()).unwrap();
            assert!(!report.has_errors(), "{:?}", report.diagnostics);
            let before = serde_json::to_value(&report.document).unwrap();
            assert_eq!(report.evaluate_plain_type_membership_from_libraries("owner").unwrap(), control["membership"]);
            assert_eq!(serde_json::to_value(&report.document).unwrap(), before);
            report.document.elements.reverse();
            assert_eq!(report.evaluate_plain_type_membership_from_libraries("owner").unwrap(), control["membership"]);
        }
    }

    #[test]
    fn standard_default_lookup_rejects_private_ambiguous_and_incomplete_roots() {
        let oracle: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/default-general-pilot-controls.json"
        )).unwrap();
        let control = oracle["controls"].as_array().unwrap().iter().find(|c|c["kind"]=="Class" && c["mode"]==0).unwrap();
        let clean = import_sysml_api_elements(plain_default_graph_with_standard_library(control), Default::default()).unwrap();
        for (id, field, value) in [
            ("opaque-library", "is_standard", json!(false)),
            ("opaque-library", "is_standard", json!("true")),
            ("opaque-library", "declared_name", json!("Wrong")),
            ("opaque-library", "owned_relationship", json!([])),
            ("opaque-library", "owning_relationship", json!(17)),
            ("opaque-library", "owning_relationship", json!("missing-container")),
            ("library-member", "visibility", json!("private")),
            ("library-member", "member_element", json!("missing")),
        ] {
            let mut report = clean.clone();
            report.document.elements.iter_mut().find(|e| e.id == id).unwrap().properties.insert(field.into(), value);
            assert!(report.evaluate_plain_type_membership_from_libraries("owner").is_err(), "{id}.{field}");
        }
        let mut report = clean.clone();
        let mut duplicate = report.document.elements.iter().find(|e| e.id == "opaque-library").unwrap().clone();
        duplicate.id = "second-library".into();duplicate.properties.insert("owned_relationship".into(), json!([]));
        report.document.elements.push(duplicate);
        assert!(report.evaluate_plain_type_membership_from_libraries("owner").is_err());
        let mut report = clean.clone();
        report.document.elements.iter_mut().find(|e| e.id == "library-member").unwrap().properties.insert("visibility".into(), json!("private"));
        let mut decoy = report.document.elements.iter().find(|e| e.id == "default").unwrap().clone();
        decoy.id = "Occurrences::Occurrence".into();decoy.properties.insert("is_implied_included".into(), json!(true));
        report.document.elements.push(decoy);
        assert!(report.evaluate_plain_type_membership_from_libraries("owner").is_err());
    }

    #[test]
    fn standard_default_lookup_reuses_import_visibility_and_owned_effective_names() {
        let oracle: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/default-general-pilot-controls.json"
        )).unwrap();
        let control = oracle["controls"].as_array().unwrap().iter().find(|c|c["kind"]=="Class" && c["mode"]==0).unwrap();
        let mut graph = plain_default_graph_with_standard_library(control);
        graph.iter_mut().find(|e| e["@id"] == "opaque-library").unwrap()["ownedRelationship"] = json!([{"@id":"library-import"}]);
        graph.iter_mut().find(|e| e["@id"] == "library-member").unwrap()["owningRelatedElement"] = json!({"@id":"relay"});
        graph.iter_mut().find(|e| e["@id"] == "library-member").unwrap()["visibility"] = json!("private");
        graph.push(json!({"@id":"relay", "@type":"Package", "ownedRelationship":[{"@id":"library-member"}]}));
        graph.push(json!({"@id":"library-import", "@type":"NamespaceImport", "owningRelatedElement":{"@id":"opaque-library"},
            "importedNamespace":{"@id":"relay"}, "isImportAll":true, "visibility":"public"}));
        let mut report = import_sysml_api_elements(graph, Default::default()).unwrap();
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert_eq!(report.evaluate_plain_type_membership_from_libraries("owner").unwrap(), control["membership"]);
        report.document.elements.iter_mut().find(|e| e.id == "library-import").unwrap().properties.insert("visibility".into(), json!("private"));
        assert!(report.evaluate_plain_type_membership_from_libraries("owner").is_err());
        report.document.elements.iter_mut().find(|e| e.id == "library-import").unwrap().properties.insert("visibility".into(), json!("public"));
        report.document.elements.iter_mut().find(|e| e.id == "library-import").unwrap().properties.insert("is_import_all".into(), json!(false));
        assert!(report.evaluate_plain_type_membership_from_libraries("owner").is_err());
        let mut graph = plain_default_graph_with_standard_library(control);
        let membership = graph.iter_mut().find(|e| e["@id"] == "library-member").unwrap();
        membership["@type"] = json!("OwningMembership");
        membership.as_object_mut().unwrap().remove("memberName");membership.as_object_mut().unwrap().remove("memberElement");
        membership["ownedRelatedElement"] = json!([{"@id":"default"}]);
        let target = graph.iter_mut().find(|e| e["@id"] == "default").unwrap();
        target["owningRelationship"] = json!({"@id":"library-member"});target["declaredName"] = json!("Occurrence");
        let mut report = import_sysml_api_elements(graph, Default::default()).unwrap();
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        assert_eq!(report.evaluate_plain_type_membership_from_libraries("owner").unwrap(), control["membership"]);
        report.document.elements.iter_mut().find(|e| e.id == "default").unwrap().properties.insert("declared_name".into(), json!("Renamed"));
        assert!(report.evaluate_plain_type_membership_from_libraries("owner").is_err());
    }

    #[test]
    fn ecore_reference_errors_preserve_original_payloads() {
        for (kind, field, value) in [
            ("Package", "ownedRelationship", json!([{"@id":"class"}])),
            (
                "Package",
                "ownedRelationship",
                json!([{"@id":"m"}, {"@id":"m"}]),
            ),
            ("FeatureTyping", "type", json!([{"@id":"class"}])),
            ("PartUsage", "type", json!({"@id":"class"})),
            ("FeatureTyping", "type", Value::Null),
            (
                "PartUsage",
                "type",
                json!([{"@id":"class", "@type":"DataType"}]),
            ),
            ("PartUsage", "type", json!([{"@id":"class", "@type":4}])),
            (
                "PartUsage",
                "type",
                json!([{"@id":"class", "declaredName":"embedded"}]),
            ),
        ] {
            let mut element = json!({"@id":"test", "@type":kind});
            element[field] = value.clone();
            let report = import_sysml_api_elements(
                vec![
                    element,
                    json!({"@id":"class", "@type":"Class"}),
                    json!({"@id":"m", "@type":"OwningMembership"}),
                ],
                Default::default(),
            )
            .unwrap();
            assert!(report.has_errors(), "{kind}.{field}: {value}");
            let properties = &report.document.elements[0].properties;
            assert!(!properties.contains_key(&camel_to_snake(field)));
            assert_eq!(properties["x_sysml_api"][extension_key(field)], value);
        }
    }

    #[test]
    fn ecore_graph_conflicts_are_reported_on_json_import_and_export() {
        let imported = import_sysml_api_elements(
            vec![
                json!({"@id":"p", "@type":"Package", "ownedRelationship":[{"@id":"r"}]}),
                json!({"@id":"r", "@type":"OwningMembership", "ownedRelatedElement":[{"@id":"p"}]}),
            ],
            Default::default(),
        )
        .unwrap();
        assert!(imported.has_errors());
        assert!(
            imported.diagnostics.iter().any(
                |d| d.code == "sysml_json.reference.ecore_graph" && d.message.contains("cycle")
            )
        );
        let exported =
            export_sysml_abstract_syntax_value(&imported.document, Default::default()).unwrap();
        assert!(exported.has_errors());
        assert!(
            exported
                .diagnostics
                .iter()
                .any(|d| d.code == "sysml_json_export.reference.ecore_graph")
        );
    }

    #[test]
    fn invalid_reference_export_preserves_values_and_reports_target_errors() {
        let document = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                KirElement {
                    id: "typing".into(),
                    kind: "FeatureTyping".into(),
                    layer: 2,
                    properties: BTreeMap::from([("type".into(), json!(["a", "b"]))]),
                },
                KirElement {
                    id: "part".into(),
                    kind: "PartUsage".into(),
                    layer: 2,
                    properties: BTreeMap::from([("type".into(), json!(["comment"]))]),
                },
                KirElement {
                    id: "comment".into(),
                    kind: "Comment".into(),
                    layer: 2,
                    properties: BTreeMap::new(),
                },
            ],
        };
        let report = export_sysml_abstract_syntax_value(&document, Default::default()).unwrap();
        assert!(report.has_errors());
        assert_eq!(report.value["elements"][0]["type"], json!(["a", "b"]));
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "sysml_json_export.property.ecore_value")
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "sysml_json_export.reference.ecore_target")
        );
    }

    #[test]
    fn external_references_remain_unresolved_with_stable_identity() {
        let imported = import_sysml_api_elements(vec![json!({
            "@id":"part", "@type":"PartUsage", "type":[{"@id":"urn:external:type", "@type":"Class"}]
        })], Default::default()).unwrap();
        assert!(!imported.has_errors());
        assert_eq!(
            imported.diagnostics[0].code,
            "sysml_json.reference.unresolved"
        );
        let exported =
            export_sysml_abstract_syntax_value(&imported.document, Default::default()).unwrap();
        assert_eq!(
            exported.value["elements"][0]["type"][0]["@id"],
            "urn:external:type"
        );
        assert_eq!(
            exported.diagnostics[0].code,
            "sysml_json_export.reference.unresolved"
        );
    }

    #[test]
    fn exchange_alias_collisions_and_duplicate_resolved_references_are_rejected() {
        let collision = import_sysml_api_elements(
            vec![
                json!({"@id":"external", "@type":"Class", "xMercurio":{"kirId":"a"}}),
                json!({"@id":"external", "@type":"Class", "xMercurio":{"kirId":"b"}}),
            ],
            Default::default(),
        );
        assert!(matches!(
            collision,
            Err(SysmlJsonImportError::DuplicateId(_))
        ));
        let report = import_sysml_api_elements(vec![
            json!({"@id":"part", "@type":"PartUsage", "type":[{"@id":"external"},{"@id":"native"}]}),
            json!({"@id":"external", "@type":"Class", "xMercurio":{"kirId":"native"}}),
        ], Default::default()).unwrap();
        assert!(report.has_errors());
        assert!(
            report.diagnostics[0]
                .message
                .contains("duplicate Ecore value")
        );
    }

    #[test]
    fn imports_api_elements_to_kir() {
        let elements = vec![json!({
            "@id": "pkg.demo",
            "@type": "Package",
            "declaredName": "Demo",
            "ownedRelationship": [{"@id": "membership.vehicle", "@type": "Membership"}],
            "unknownCamel": {"nestedRef": {"@id": "part.vehicle", "@type": "PartUsage"}}
        })];

        let report = import_sysml_api_elements(
            elements,
            SysmlJsonImportOptions {
                source_uri: Some(
                    "sysmlapi://example/projects/project-1/commits/commit-1".to_string(),
                ),
                project_id: Some("project-1".to_string()),
                commit_id: Some("commit-1".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(!report.has_errors());
        assert_eq!(
            report.diagnostics[0].code,
            "sysml_json.reference.unresolved"
        );
        assert_eq!(report.document.elements.len(), 1);
        let element = &report.document.elements[0];
        assert_eq!(element.id, "pkg.demo");
        assert_eq!(element.kind, "Package");
        assert_eq!(element.properties["declared_name"], json!("Demo"));
        assert_eq!(
            element.properties["owned_relationship"],
            json!(["membership.vehicle"])
        );
        assert_eq!(
            element.properties["x_sysml_api"]["unknown_camel"]["nested_ref"],
            json!("part.vehicle")
        );
        assert_eq!(
            element.properties["metadata"]["source_provenance"]["commit_id"],
            json!("commit-1")
        );
    }

    #[test]
    fn imports_object_with_elements_array() {
        let input = json!({
            "name": "Snapshot",
            "elements": [
                {
                    "@id": "part.vehicle",
                    "@type": "PartUsage",
                    "declaredName": "vehicle",
                    "type": [{"@id": "type.Vehicle"}]
                }
            ]
        });

        let report =
            import_sysml_abstract_syntax_value(input, SysmlJsonImportOptions::default()).unwrap();

        assert_eq!(report.document.elements.len(), 1);
        let element = &report.document.elements[0];
        assert_eq!(element.properties["declared_name"], json!("vehicle"));
        assert_eq!(element.properties["type"], json!(["type.Vehicle"]));
        assert_eq!(
            report.metadata["x_sysml_json_document"]["name"],
            json!("Snapshot")
        );
    }

    #[test]
    fn preserves_structured_api_expression_as_extension() {
        let report = import_sysml_api_elements(
            vec![json!({
                "@id": "expr.structured",
                "@type": "AttributeUsage",
                "declaredName": "limit",
                "expression": {
                    "@id": "expr.literal",
                    "@type": "LiteralInteger",
                    "value": 5
                }
            })],
            SysmlJsonImportOptions::default(),
        )
        .unwrap();

        let element = &report.document.elements[0];
        assert!(element.properties.get("expression").is_none());
        assert_eq!(
            element.properties["x_sysml_api"]["expression"]["at_type"],
            json!("LiteralInteger")
        );
        assert_eq!(
            report.diagnostics[0].code,
            "sysml_json.property.structured_scalar"
        );
    }

    #[test]
    fn reports_missing_type_without_importing_element() {
        let report = import_sysml_api_elements(
            vec![json!({
                "@id": "missing.type",
                "declaredName": "MissingType"
            })],
            SysmlJsonImportOptions::default(),
        )
        .unwrap();

        assert!(report.has_errors());
        assert!(report.document.elements.is_empty());
        assert_eq!(
            report.diagnostics[0].code,
            "sysml_json.element.missing_type"
        );
    }

    #[test]
    fn rejects_duplicate_ids() {
        let err = import_sysml_api_elements(
            vec![
                json!({"@id": "dup", "@type": "Package"}),
                json!({"@id": "dup", "@type": "Package"}),
            ],
            SysmlJsonImportOptions::default(),
        )
        .unwrap_err();

        assert!(matches!(err, SysmlJsonImportError::DuplicateId(id) if id == "dup"));
    }

    #[test]
    fn uses_element_id_as_fallback_with_warning() {
        let report = import_sysml_api_elements(
            vec![json!({
                "elementId": "fallback",
                "@type": "Package"
            })],
            SysmlJsonImportOptions::default(),
        )
        .unwrap();

        assert_eq!(report.document.elements[0].id, "fallback");
        assert_eq!(report.diagnostics[0].code, "sysml_json.element.fallback_id");
    }

    #[test]
    fn exports_kir_as_sysml_json_with_uuid_exchange_ids() {
        let document = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                KirElement {
                    id: "pkg.Demo".to_string(),
                    kind: "Package".to_string(),
                    layer: 2,
                    properties: BTreeMap::from([
                        ("declared_name".to_string(), json!("Demo")),
                        ("members".to_string(), json!(["type.Demo.Vehicle"])),
                    ]),
                },
                KirElement {
                    id: "type.Demo.Vehicle".to_string(),
                    kind: "PartDefinition".to_string(),
                    layer: 2,
                    properties: BTreeMap::from([
                        ("declared_name".to_string(), json!("Vehicle")),
                        ("owner".to_string(), json!("pkg.Demo")),
                    ]),
                },
            ],
        };

        let report =
            export_sysml_abstract_syntax_value(&document, SysmlJsonExportOptions::default())
                .unwrap();

        assert!(!report.has_errors());
        let elements = report.value["elements"].as_array().unwrap();
        let package = elements
            .iter()
            .find(|element| element["xMercurio"]["kirId"] == json!("pkg.Demo"))
            .unwrap();
        assert!(is_uuid_like(package["@id"].as_str().unwrap()));
        assert_eq!(package["@type"], json!("Package"));
        assert_eq!(package["declaredName"], json!("Demo"));
        assert_eq!(package["ownedMember"][0]["@type"], json!("PartDefinition"));
        assert!(is_uuid_like(
            package["ownedMember"][0]["@id"].as_str().unwrap()
        ));
    }

    #[test]
    fn exports_scoped_scalar_lists_without_reference_conversion() {
        let document = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                KirElement {
                    id: "meta.issue.text".into(),
                    kind: "MetamodelFeature".into(),
                    layer: 2,
                    properties: BTreeMap::from([
                        ("kir_property".into(), json!("text")),
                        ("feature_kind".into(), json!("attribute")),
                    ]),
                },
                KirElement {
                    id: "meta.requirement.text".into(),
                    kind: "MetamodelFeature".into(),
                    layer: 2,
                    properties: BTreeMap::from([
                        ("kir_property".into(), json!("text")),
                        ("feature_kind".into(), json!("attribute")),
                        ("upper".into(), json!(-1)),
                        ("kir_owner_kinds".into(), json!(["ConcernDefinition"])),
                    ]),
                },
                KirElement {
                    id: "issue".into(),
                    kind: "Issue".into(),
                    layer: 2,
                    properties: BTreeMap::from([("text".into(), json!("one"))]),
                },
                KirElement {
                    id: "requirement".into(),
                    kind: "ConcernDefinition".into(),
                    layer: 2,
                    properties: BTreeMap::from([("text".into(), json!(["one", "two"]))]),
                },
            ],
        };
        let report =
            export_sysml_abstract_syntax_value(&document, SysmlJsonExportOptions::default())
                .unwrap();
        let elements = report.value["elements"].as_array().unwrap();
        let issue = elements
            .iter()
            .find(|element| element["xMercurio"]["kirId"] == "issue")
            .unwrap();
        let requirement = elements
            .iter()
            .find(|element| element["xMercurio"]["kirId"] == "requirement")
            .unwrap();
        assert_eq!(issue["text"], "one");
        assert_eq!(requirement["text"], json!(["one", "two"]));
    }

    #[test]
    fn imports_mercurio_export_with_original_kir_ids() {
        let original = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![KirElement {
                id: "pkg.Demo".to_string(),
                kind: "Package".to_string(),
                layer: 2,
                properties: BTreeMap::from([("declared_name".to_string(), json!("Demo"))]),
            }],
        };
        let exported =
            export_sysml_abstract_syntax_value(&original, SysmlJsonExportOptions::default())
                .unwrap();
        let imported =
            import_sysml_abstract_syntax_value(exported.value, SysmlJsonImportOptions::default())
                .unwrap();

        assert_eq!(imported.document.elements[0].id, "pkg.Demo");
        assert!(is_uuid_like(
            imported.document.elements[0].properties["metadata"]["source_provenance"]
                ["external_id"]
                .as_str()
                .unwrap()
        ));
    }
}
