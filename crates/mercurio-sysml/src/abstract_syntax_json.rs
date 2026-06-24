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

pub const SYSML_JSON_IMPORTER_VERSION: &str = concat!(
    "mercurio-sysml/",
    env!("CARGO_PKG_VERSION"),
    "/sysml-json-v1"
);

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SysmlJsonImportReport {
    pub document: KirDocument,
    pub diagnostics: Vec<SysmlJsonImportDiagnostic>,
    pub metadata: BTreeMap<String, Value>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SysmlJsonImportSeverity {
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

fn import_elements(
    elements: Vec<Value>,
    options: SysmlJsonImportOptions,
    source_document_metadata: Option<Value>,
) -> Result<SysmlJsonImportReport, SysmlJsonImportError> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    let mut imported = Vec::new();
    let registry = KirFieldRegistry::standard();

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
    let id = match element_id(&object) {
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
            Some(id),
            Some(path.to_string()),
        ));
        return Ok(None);
    };
    let kind = kind.to_string();

    let mut properties = BTreeMap::new();
    let mut extension = Map::new();

    for (source_key, value) in &object {
        if source_key == "@id" || source_key == "@type" {
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
        source_provenance(options, &id, &kind, path),
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
    object
        .get("@id")
        .or_else(|| object.get("elementId"))
        .and_then(Value::as_str)
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
}
