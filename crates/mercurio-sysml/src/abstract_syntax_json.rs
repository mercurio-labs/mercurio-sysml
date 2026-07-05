//! Importer for OMG SysML/KerML abstract syntax JSON and Systems Modeling API
//! element payloads.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use mercurio_kir::{
    KIR_SCHEMA_VERSION, KIR_SCHEMA_VERSION_METADATA_KEY, KirDocument, KirElement, KirError,
    KirFieldKind, KirFieldRegistry, inferred_layer,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::sysml_field_specs;

pub const SYSML_JSON_IMPORTER_VERSION: &str = concat!(
    "mercurio-sysml/",
    env!("CARGO_PKG_VERSION"),
    "/sysml-json-v1"
);
pub const SYSML_JSON_EXPORTER_VERSION: &str = concat!(
    "mercurio-sysml/",
    env!("CARGO_PKG_VERSION"),
    "/sysml-json-export-v1"
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

        let exported_value = match registry.field(property).map(|spec| spec.kind) {
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
            Some(KirFieldKind::Scalar) | None => value.clone(),
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

        if let Some(element) = import_element(object, &options, &registry, &path, &mut diagnostics)?
        {
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

        let normalized = normalize_value(value.clone());
        if let Some(property_name) = kir_property_name(source_key, registry) {
            if should_preserve_as_extension(&property_name, &normalized, registry) {
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

fn kir_property_name(source_key: &str, registry: &KirFieldRegistry) -> Option<String> {
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

    if registry.field(candidate).is_some() {
        return Some(candidate.to_string());
    }

    let snake = camel_to_snake(source_key);
    registry.field(&snake).map(|_| snake)
}

fn should_preserve_as_extension(
    property_name: &str,
    value: &Value,
    registry: &KirFieldRegistry,
) -> bool {
    matches!(
        registry.field(property_name).map(|spec| spec.kind),
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
                .unwrap_or_else(|| deterministic_exchange_uuid(kir_id)),
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

        assert!(report.diagnostics.is_empty());
        assert_eq!(report.document.elements.len(), 1);
        let element = &report.document.elements[0];
        assert_eq!(element.id, "pkg.demo");
        assert_eq!(element.kind, "Package");
        assert_eq!(element.properties["declared_name"], json!("Demo"));
        assert_eq!(
            element.properties["relationships"],
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
                    "type": {"@id": "type.Vehicle"}
                }
            ]
        });

        let report =
            import_sysml_abstract_syntax_value(input, SysmlJsonImportOptions::default()).unwrap();

        assert_eq!(report.document.elements.len(), 1);
        let element = &report.document.elements[0];
        assert_eq!(element.properties["declared_name"], json!("vehicle"));
        assert_eq!(element.properties["type"], json!("type.Vehicle"));
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
