//! Experimental projection of lowered part type links to relationship records.
//! KerML 1.0 §§8.3.3.1.8, 8.3.3.2.3, 8.3.3.3.7 and 8.3.3.3.10.
//! This profile is deliberately NOT publication-qualified. It does not invent
//! library records or inherited values. KIR does not preserve explicit/implied
//! attribution on all these links, so isImplied remains unknown (JSON null).
use super::*;
mod ownership;
pub const PROFILE: &str = "omg-parts-preview-v2";

fn links(element: &KirElement, property: &str) -> Result<Vec<String>, String> {
    let Some(value) = element.properties.get(property) else {
        return Ok(Vec::new());
    };
    match value {
        Value::String(id) => Ok(vec![id.clone()]),
        Value::Array(values) => values
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{}.{property} must contain KIR IDs", element.id))
            })
            .collect(),
        _ => Err(format!(
            "{}.{property} must be a KIR ID or ID array",
            element.id
        )),
    }
}

pub(super) fn project(
    document: &KirDocument,
    ids: &BTreeMap<String, String>,
    elements: &mut Vec<Value>,
) -> Result<Value, String> {
    // Stage every mutation: an unsupported link never leaves a partially rewritten export.
    let mut projected = elements.clone();
    let mut origins = Map::new();
    let mut used: BTreeSet<_> = ids.values().cloned().collect();
    let reference = |kir: &str| reference_object(kir, ids, None);
    for element in &document.elements {
        let kind = metaclass_name(&element.kind);
        if !["Package", "PartDefinition", "PartUsage"].contains(&kind) {
            return Err(format!("{kind} is outside the parts preview"));
        }
        let id = ids.get(&element.id).ok_or("Missing exchange identity")?;
        origins.insert(
            id.clone(),
            json!({"kind":"authored","kirId":element.id,
            "sourceFile":element.properties.get("metadata").unwrap_or(&Value::Null)["source_file"],
            "span":element.properties.get("metadata").unwrap_or(&Value::Null)["source_span"]}),
        );
        let mut relationships: Vec<(String, String, String)> = Vec::new();
        if kind == "PartDefinition" {
            for target in links(element, "specializes")? {
                relationships.push(("Subclassification".into(), target, "specializes".into()));
            }
        } else if kind == "PartUsage" {
            // These are lowered target links, not already reified OMG relationships.
            for key in [
                "feature_typings",
                "redefines",
                "redefined_features",
                "subsets",
                "specialized_features",
            ] {
                if !links(element, key)?.is_empty() {
                    return Err(format!(
                        "{key} is not yet supported by the parts relationship preview"
                    ));
                }
            }
            let types = links(element, "type")?;
            let definitions = links(element, "definition")?;
            if definitions.iter().any(|id| !types.contains(id)) {
                return Err("Part definition and type links disagree".into());
            }
            let subsets = links(element, "subsetted_features")?;
            for target in links(element, "specializes")? {
                if !types.contains(&target) && !subsets.contains(&target) {
                    return Err(format!("Unclassified part specialization target: {target}"));
                }
            }
            for target in types {
                relationships.push(("FeatureTyping".into(), target, "type".into()));
            }
            for target in subsets {
                relationships.push(("Subsetting".into(), target, "subsetted_features".into()));
            }
        }
        let mut by_kind = BTreeMap::<String, Vec<Value>>::new();
        let mut all = Vec::new();
        let mut seen = BTreeSet::new();
        for (relationship_kind, target, property) in relationships {
            if !seen.insert((relationship_kind.clone(), target.clone())) {
                continue;
            }
            if let Some(endpoint) = document.elements.iter().find(|e| e.id == target) {
                let expected = if relationship_kind == "Subsetting" {
                    "PartUsage"
                } else {
                    "PartDefinition"
                };
                if metaclass_name(&endpoint.kind) != expected {
                    return Err(format!(
                        "{relationship_kind} target {target} must be a {expected}"
                    ));
                }
            }
            // Canonical tuple encoding avoids delimiter ambiguities in caller-supplied IDs.
            let relation_id = deterministic_exchange_uuid(
                &json!([PROFILE, relationship_kind, element.id, target]).to_string(),
            );
            if !used.insert(relation_id.clone()) {
                return Err("Generated relationship identity collision".into());
            }
            let specific = reference(&element.id);
            let general = reference(&target);
            let mut relation = package_projection::base(&relation_id, &relationship_kind);
            relation["isImplied"] = Value::Null;
            relation["specific"] = specific.clone();
            relation["general"] = general.clone();
            relation["source"] = json!([specific]);
            relation["target"] = json!([general]);
            relation["relatedElement"] = json!([specific, general]);
            relation["ownedRelatedElement"] = json!([]);
            relation["owningRelatedElement"] = specific.clone();
            relation["owningType"] = specific.clone();
            match relationship_kind.as_str() {
                "Subclassification" => {
                    relation["subclassifier"] = specific.clone();
                    relation["superclassifier"] = general;
                    relation["owningClassifier"] = specific;
                }
                "FeatureTyping" => {
                    relation["typedFeature"] = specific.clone();
                    relation["type"] = general;
                    relation["owningFeature"] = specific;
                }
                "Subsetting" => {
                    relation["subsettingFeature"] = specific.clone();
                    relation["subsettedFeature"] = general;
                    relation["owningFeature"] = specific;
                }
                _ => return Err("Unsupported relationship kind".into()),
            }
            origins.insert(relation_id.clone(), json!({"kind":"derived","rule":"kir-part-relationship",
                "sourceElements":[id],"sourceKirId":element.id,"sourceProperty":property,
                "targetKirId":target,"targetElementId":reference(&target)["@id"],"implied":"unknown"}));
            let reference = json!({"@id":relation_id});
            by_kind
                .entry(relationship_kind)
                .or_default()
                .push(reference.clone());
            all.push(reference);
            projected.push(relation);
        }
        let record = projected
            .iter_mut()
            .find(|e| e["@id"] == *id)
            .ok_or("Export record missing")?;
        if let Some(object) = record.as_object_mut() {
            // Metaclass identity is already in @type; metatype is a compiler-internal link.
            object.remove("metatype");
            if kind != "Package" {
                object.remove("subsettedFeature");
                object.insert("ownedSpecialization".into(), json!(all));
                let owned = object
                    .entry("ownedRelationship")
                    .or_insert(json!([]))
                    .as_array_mut()
                    .ok_or("ownedRelationship must be an array")?;
                owned.extend(all);
                for (class, key) in [
                    ("Subclassification", "ownedSubclassification"),
                    ("FeatureTyping", "ownedTyping"),
                    ("Subsetting", "ownedSubsetting"),
                ] {
                    if (kind == "PartDefinition") == (class == "Subclassification") {
                        object.insert(
                            key.into(),
                            json!(by_kind.get(class).cloned().unwrap_or_default()),
                        );
                    }
                }
            }
        }
    }
    ownership::project(document, ids, &mut projected, &mut origins)?;
    *elements = projected;
    Ok(Value::Object(origins))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> KirDocument {
        let e = |id: &str, kind: &str, properties| KirElement {
            id: id.into(),
            kind: kind.into(),
            layer: 2,
            properties,
        };
        KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                e(
                    "rover",
                    "PartUsage",
                    BTreeMap::from([
                        ("type".into(), json!("Rover")),
                        ("definition".into(), json!("Rover")),
                        ("specializes".into(), json!(["Rover", "Parts::parts"])),
                        ("subsetted_features".into(), json!(["Parts::parts"])),
                    ]),
                ),
                e(
                    "Rover",
                    "PartDefinition",
                    BTreeMap::from([("specializes".into(), json!(["Parts::Part"]))]),
                ),
            ],
        }
    }
    #[test]
    fn reifies_links_and_marks_external_targets_and_unknown_implied_status() {
        let report = export_sysml_abstract_syntax_value(
            &document(),
            SysmlJsonExportOptions {
                schema_profile: Some(PROFILE.into()),
                include_mercurio_extensions: false,
                ..Default::default()
            },
        )
        .unwrap();
        let elements = report.value["elements"].as_array().unwrap();
        assert_eq!(elements.len(), 5);
        let usage = elements.iter().find(|e| e["@type"] == "PartUsage").unwrap();
        let typing = elements
            .iter()
            .find(|e| e["@type"] == "FeatureTyping")
            .unwrap();
        assert_eq!(usage["ownedTyping"][0]["@id"], typing["@id"]);
        assert_eq!(typing["typedFeature"]["@id"], usage["@id"]);
        assert_eq!(typing["type"]["@id"], usage["type"][0]["@id"]);
        assert_eq!(typing["owningRelatedElement"]["@id"], usage["@id"]);
        assert!(typing["owner"].is_null());
        assert!(typing["isImplied"].is_null());
        assert_eq!(
            report.metadata["reference_closure"]["unresolved"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(report.metadata["origins"].as_object().unwrap().len(), 5);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "sysml_json_export.preview_only")
        );
    }
    #[test]
    fn rejects_malformed_unclassified_and_wrong_kind_links_without_partial_mutation() {
        for (key, value) in [
            ("type", json!(7)),
            ("specializes", json!(["unknown"])),
            ("type", json!("rover")),
        ] {
            let mut doc = document();
            doc.elements[0].properties.insert(key.into(), value);
            let mut diagnostics = vec![];
            let ids = exchange_id_map(&doc, &mut diagnostics);
            let mut elements = vec![json!({"@id":ids["rover"]}), json!({"@id":ids["Rover"]})];
            let before = elements.clone();
            assert!(project(&doc, &ids, &mut elements).is_err());
            assert_eq!(elements, before);
        }
    }
}
