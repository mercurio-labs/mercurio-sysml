//! Authored ownership only: inherited/imported membership is intentionally not synthesized.
use super::*;

pub(super) fn project(
    document: &KirDocument,
    ids: &BTreeMap<String, String>,
    elements: &mut Vec<Value>,
    origins: &mut Map<String, Value>,
) -> Result<(), String> {
    let by_id: BTreeMap<_, _> = document
        .elements
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect();
    if by_id.len() != document.elements.len() {
        return Err("Duplicate authored KIR identity".into());
    }
    let mut parents = BTreeMap::new();
    let mut children = BTreeMap::new();
    for e in &document.elements {
        if let Some(owner) = e.properties.get("owner") {
            let owner = owner.as_str().ok_or("Owner must be one KIR ID")?;
            if !by_id.contains_key(owner) {
                return Err("Authored owner is outside the document".into());
            }
            parents.insert(e.id.as_str(), owner);
        }
        let members = links(e, "members")?;
        if members.iter().collect::<BTreeSet<_>>().len() != members.len() {
            return Err("Duplicate authored member".into());
        }
        children.insert(e.id.as_str(), members);
    }
    for e in &document.elements {
        if let Some(parent) = parents.get(e.id.as_str()) {
            if !children[parent].contains(&e.id) {
                return Err("Owner/member links disagree".into());
            }
        }
        for child in &children[e.id.as_str()] {
            if parents.get(child.as_str()).copied() != Some(e.id.as_str()) {
                return Err("Member/owner links disagree".into());
            }
        }
        let mut cursor = e.id.as_str();
        let mut seen = BTreeSet::new();
        while let Some(parent) = parents.get(cursor) {
            if !seen.insert(cursor) {
                return Err("Cyclic authored ownership".into());
            }
            cursor = parent;
        }
        // Redundant lowered feature ownership must agree before generating an edge.
        for feature in links(e, "features")? {
            if !children[e.id.as_str()].contains(&feature)
                || by_id.get(feature.as_str()).map(|f| metaclass_name(&f.kind)) != Some("PartUsage")
            {
                return Err("Owned feature/member links disagree".into());
            }
        }
        for key in ["owning_namespace", "owning_type", "owning_definition"] {
            if let Some(value) = e.properties.get(key) {
                if value.as_str() != parents.get(e.id.as_str()).copied() {
                    return Err(format!("{key} disagrees with authored owner"));
                }
                if key != "owning_namespace"
                    && parents
                        .get(e.id.as_str())
                        .and_then(|p| by_id.get(p))
                        .map(|p| metaclass_name(&p.kind))
                        != Some("PartDefinition")
                {
                    return Err(format!(
                        "{key} requires a part definition owner in this preview"
                    ));
                }
            }
        }
    }
    let mut used: BTreeSet<_> = elements
        .iter()
        .filter_map(|e| e["@id"].as_str().map(str::to_owned))
        .collect();
    let mut memberships = BTreeMap::new();
    let reference = |id: &str| json!({"@id":ids[id]});
    for (child, parent) in &parents {
        let feature = metaclass_name(&by_id[child].kind) == "PartUsage"
            && metaclass_name(&by_id[parent].kind) != "Package";
        let kind = if feature {
            "FeatureMembership"
        } else {
            "OwningMembership"
        };
        let id = deterministic_exchange_uuid(&json!([PROFILE, kind, parent, child]).to_string());
        if !used.insert(id.clone()) {
            return Err("Generated membership identity collision".into());
        }
        let mut relation = package_projection::base(&id, kind);
        // Lowering does not retain enough syntax to prove public/private or implied status.
        relation["visibility"] = Value::Null;
        relation["isImplied"] = Value::Null;
        relation["owningRelatedElement"] = reference(parent);
        relation["membershipOwningNamespace"] = reference(parent);
        relation["memberElement"] = reference(child);
        relation["ownedMemberElement"] = reference(child);
        for key in ["memberElementId", "ownedMemberElementId"] {
            relation[key] = json!(ids[*child]);
        }
        for key in ["memberName", "ownedMemberName"] {
            relation[key] = by_id[child]
                .properties
                .get("declared_name")
                .cloned()
                .unwrap_or(Value::Null);
        }
        for key in ["memberShortName", "ownedMemberShortName"] {
            relation[key] = Value::Null;
        }
        relation["source"] = json!([reference(parent)]);
        relation["target"] = json!([reference(child)]);
        relation["relatedElement"] = json!([reference(parent), reference(child)]);
        relation["ownedRelatedElement"] = json!([reference(child)]);
        if feature {
            relation["ownedMemberFeature"] = reference(child);
            relation["owningType"] = reference(parent);
        }
        origins.insert(
            id.clone(),
            json!({"kind":"derived", "rule":"kir-authored-ownership",
            "sourceElements":[ids[*parent],ids[*child]], "sourceKirId":parent,
            "sourceProperty":"members", "targetKirId":child, "targetElementId":ids[*child],
            "implied":"unknown", "visibility":"unknown"}),
        );
        memberships.insert(*child, (id, feature));
        elements.push(relation);
    }
    for e in &document.elements {
        let record = elements
            .iter_mut()
            .find(|v| v["@id"] == ids[&e.id])
            .ok_or("Authored record missing")?;
        if let Some(parent) = parents.get(e.id.as_str()) {
            record["owner"] = reference(parent);
            record["owningNamespace"] = reference(parent);
            record["owningMembership"] = json!({"@id":memberships[e.id.as_str()].0});
            record["owningRelationship"] = record["owningMembership"].clone();
            if memberships[e.id.as_str()].1 {
                record["owningType"] = reference(parent);
            }
        }
        let members: Vec<_> = children[e.id.as_str()]
            .iter()
            .map(|id| reference(id))
            .collect();
        let relations: Vec<_> = children[e.id.as_str()]
            .iter()
            .map(|id| json!({"@id":memberships[id.as_str()].0}))
            .collect();
        record["ownedMember"] = json!(members);
        record["ownedElement"] = json!(members);
        record["ownedMembership"] = json!(relations);
        let owned = record
            .as_object_mut()
            .ok_or("Record must be an object")?
            .entry("ownedRelationship")
            .or_insert(json!([]))
            .as_array_mut()
            .ok_or("ownedRelationship must be an array")?;
        owned.extend(relations);
        if metaclass_name(&e.kind) != "Package" {
            let features: Vec<_> = children[e.id.as_str()]
                .iter()
                .filter(|id| memberships[id.as_str()].1)
                .collect();
            record["ownedFeature"] =
                json!(features.iter().map(|id| reference(id)).collect::<Vec<_>>());
            record["ownedFeatureMembership"] = json!(
                features
                    .iter()
                    .map(|id| json!({"@id":memberships[id.as_str()].0}))
                    .collect::<Vec<_>>()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tree() -> KirDocument {
        let element = |id: &str, kind: &str, owner: Option<&str>, members: Vec<&str>| {
            let mut properties = BTreeMap::from([
                ("declared_name".into(), json!(id)),
                ("members".into(), json!(members)),
            ]);
            if let Some(owner) = owner {
                properties.insert("owner".into(), json!(owner));
            }
            KirElement {
                id: id.into(),
                kind: kind.into(),
                layer: 2,
                properties,
            }
        };
        KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                element("root", "Package", None, vec!["Rover"]),
                element("Rover", "PartDefinition", Some("root"), vec!["wheel"]),
                element("wheel", "PartUsage", Some("Rover"), vec![]),
            ],
        }
    }
    #[test]
    fn ownership_is_stable_and_feature_membership_is_distinct() {
        let doc = tree();
        let ids = exchange_id_map(&doc, &mut vec![]);
        let mut elements: Vec<_> = doc
            .elements
            .iter()
            .map(|e| json!({"@id":ids[&e.id]}))
            .collect();
        let origins = super::super::project(&doc, &ids, &mut elements).unwrap();
        let wheel = elements.iter().find(|e| e["@id"] == ids["wheel"]).unwrap();
        let relation = elements
            .iter()
            .find(|e| e["@id"] == wheel["owningMembership"]["@id"])
            .unwrap();
        assert_eq!(relation["@type"], "FeatureMembership");
        assert_eq!(relation["owningType"]["@id"], ids["Rover"]);
        assert!(relation["visibility"].is_null());
        assert_eq!(
            origins[relation["@id"].as_str().unwrap()]["rule"],
            "kir-authored-ownership"
        );
        let mut again: Vec<_> = doc
            .elements
            .iter()
            .map(|e| json!({"@id":ids[&e.id]}))
            .collect();
        super::super::project(&doc, &ids, &mut again).unwrap();
        assert_eq!(again, elements);
    }
    #[test]
    fn broken_ownership_is_rejected_without_partial_export() {
        for mutation in 0..6 {
            let mut doc = tree();
            match mutation {
                0 => {
                    doc.elements[2]
                        .properties
                        .insert("owner".into(), json!("missing"));
                }
                1 => {
                    doc.elements[1]
                        .properties
                        .insert("members".into(), json!([]));
                }
                2 => {
                    doc.elements[1]
                        .properties
                        .insert("members".into(), json!(["wheel", "wheel"]));
                }
                3 => {
                    doc.elements[1]
                        .properties
                        .insert("features".into(), json!(["root"]));
                }
                4 => {
                    doc.elements[2]
                        .properties
                        .insert("owning_type".into(), json!("root"));
                }
                _ => {
                    doc.elements[0]
                        .properties
                        .insert("owner".into(), json!("wheel"));
                    doc.elements[2]
                        .properties
                        .insert("members".into(), json!(["root"]));
                }
            }
            let ids = exchange_id_map(&doc, &mut vec![]);
            let mut elements: Vec<_> = doc
                .elements
                .iter()
                .map(|e| json!({"@id":ids[&e.id]}))
                .collect();
            let before = elements.clone();
            assert!(super::super::project(&doc, &ids, &mut elements).is_err());
            assert_eq!(elements, before);
        }
    }
}
