//! Deterministic retention of the library KIR records reached from authored KIR.
//! Scope is explicit: registered reference properties, excluding the compiler's
//! metatype link. This is input evidence, not a qualified OMG exchange projection.
use mercurio_foundation::kir::{KirDocument, KirElement, KirFieldKind, KirFieldRegistry};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct DependencyReference {
    pub source_id: String,
    pub property: String,
    pub target_id: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDependencyEvidence {
    pub profile: &'static str,
    pub excluded_properties: Vec<&'static str>,
    pub complete_within_scope: bool,
    pub roots: Vec<DependencyReference>,
    pub resolutions: Vec<DependencyReference>,
    pub unresolved: Vec<DependencyReference>,
    pub malformed: Vec<String>,
    pub document: KirDocument,
}

fn references(
    element: &KirElement,
    registry: &KirFieldRegistry,
    malformed: &mut Vec<String>,
) -> Vec<DependencyReference> {
    let mut result = Vec::new();
    for (key, value) in &element.properties {
        if key == "metatype"
            || !matches!(
                registry.field(key).map(|f| f.kind),
                Some(KirFieldKind::Reference | KirFieldKind::ReferenceList)
            )
        {
            continue;
        }
        let valid = match value {
            Value::String(_) | Value::Null => true,
            Value::Array(items) => items.iter().all(Value::is_string),
            _ => false,
        };
        if !valid {
            malformed.push(format!("{}.{key}: expected KIR ID or ID array", element.id));
            continue;
        }
        for target in registry.reference_ids(key, value) {
            result.push(DependencyReference {
                source_id: element.id.clone(),
                property: key.clone(),
                target_id: target.into(),
            });
        }
    }
    result
}

/// Retain exact records and every edge used to discover them. No aliases, suffix
/// matching, fake records or I/O are involved. Unknown IDs remain explicit.
pub fn retain_library_dependencies(
    authored: &KirDocument,
    library: &KirDocument,
) -> Result<LibraryDependencyEvidence, String> {
    let mut registry = KirFieldRegistry::structural();
    registry.register_fields(crate::sysml_field_specs().iter().copied());
    let local: BTreeSet<_> = authored.elements.iter().map(|e| e.id.as_str()).collect();
    let by_id: BTreeMap<_, _> = library
        .elements
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect();
    if local.len() != authored.elements.len() || by_id.len() != library.elements.len() {
        return Err("Duplicate KIR identity in dependency inputs".into());
    }
    if local.iter().any(|id| by_id.contains_key(id)) {
        return Err("Authored and library KIR identities collide".into());
    }
    let mut malformed = Vec::new();
    let mut roots = BTreeSet::new();
    for element in &authored.elements {
        for reference in references(element, &registry, &mut malformed) {
            if !local.contains(reference.target_id.as_str()) {
                roots.insert(reference);
            }
        }
    }
    let mut pending: BTreeSet<String> = roots.iter().map(|r| r.target_id.clone()).collect();
    let mut visited = BTreeSet::new();
    let mut resolutions = BTreeSet::new();
    let mut unresolved = BTreeSet::new();
    let mut classify = |reference: DependencyReference| {
        if by_id.contains_key(reference.target_id.as_str()) {
            resolutions.insert(reference);
        } else {
            unresolved.insert(reference);
        }
    };
    for reference in &roots {
        classify(reference.clone());
    }
    let mut retained = Vec::new();
    while let Some(id) = pending.pop_first() {
        if !visited.insert(id.clone()) {
            continue;
        }
        let Some(element) = by_id.get(id.as_str()) else {
            continue;
        };
        if retained.len() >= 20000 {
            return Err("Library dependency retention exceeds 20000 records".into());
        }
        for reference in references(element, &registry, &mut malformed) {
            // Library input is independent of authored source. Never silently let
            // an authored element satisfy a missing library-internal reference.
            pending.insert(reference.target_id.clone());
            classify(reference);
        }
        retained.push((*element).clone());
    }
    retained.sort_by(|a, b| a.id.cmp(&b.id));
    malformed.sort();
    malformed.dedup();
    Ok(LibraryDependencyEvidence {
        profile: "registered-kir-library-closure-v1",
        excluded_properties: vec!["metatype"],
        complete_within_scope: unresolved.is_empty() && malformed.is_empty(),
        roots: roots.into_iter().collect(),
        resolutions: resolutions.into_iter().collect(),
        unresolved: unresolved.into_iter().collect(),
        malformed,
        document: KirDocument {
            metadata: library.metadata.clone(),
            elements: retained,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn e(id: &str, properties: Value) -> KirElement {
        KirElement {
            id: id.into(),
            kind: "PartDefinition".into(),
            layer: 1,
            properties: serde_json::from_value(properties).unwrap(),
        }
    }
    fn doc(elements: Vec<KirElement>) -> KirDocument {
        KirDocument {
            metadata: BTreeMap::new(),
            elements,
        }
    }
    #[test]
    fn retains_exact_transitive_records_handles_cycles_and_ignores_scalar_lookalikes() {
        let authored = doc(vec![e(
            "rover",
            json!({"type":"A","declared_name":"unused","metatype":"compiler-only"}),
        )]);
        let library = doc(vec![
            e("unused", json!({})),
            e("B", json!({"specializes":["A"]})),
            e(
                "A",
                json!({"specializes":["B"],"metadata":{"retained":true}}),
            ),
        ]);
        let evidence = retain_library_dependencies(&authored, &library).unwrap();
        assert!(evidence.complete_within_scope);
        assert_eq!(evidence.document.elements.len(), 2);
        assert_eq!(evidence.roots.len(), 1);
        assert_eq!(evidence.resolutions.len(), 3);
        assert_eq!(
            evidence.document.elements[0].properties["metadata"]["retained"],
            true
        );
        let mut reordered = library.clone();
        reordered.elements.reverse();
        assert_eq!(
            serde_json::to_value(evidence).unwrap(),
            serde_json::to_value(retain_library_dependencies(&authored, &reordered).unwrap())
                .unwrap()
        );
    }
    #[test]
    fn reports_missing_and_malformed_links_and_rejects_identity_collisions() {
        let authored = doc(vec![e("local", json!({"type":"A"}))]);
        let library = doc(vec![e(
            "A",
            json!({"specializes":["missing","local"],"owner":7}),
        )]);
        let evidence = retain_library_dependencies(&authored, &library).unwrap();
        assert!(!evidence.complete_within_scope);
        assert_eq!(evidence.unresolved.len(), 2);
        assert_eq!(evidence.malformed.len(), 1);
        assert!(retain_library_dependencies(&authored, &authored).is_err());
        assert!(
            retain_library_dependencies(
                &authored,
                &doc(vec![e("A", json!({})), e("A", json!({}))])
            )
            .is_err()
        );
    }
}
