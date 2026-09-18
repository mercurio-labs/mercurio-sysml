//! A deliberately bounded, closed package-tree projection for the OMG REST representation.
//! Derivations follow KerML 1.0 §§8.3.2.1 and 8.3.2.4. No imports, documentation,
//! library packages or other metaclasses are silently approximated by this profile.
use super::*;
use mercurio_foundation::language_contracts::ast::{Declaration, PackageDecl};
use mercurio_foundation::language_contracts::lexer::{TokenKind, lex};

pub const PROFILE: &str = "omg-package-tree-v1";

/// Check authored syntax as well as KIR: lowering may omit unsupported modifiers.
pub fn validate_source(source: &str) -> Result<(), String> {
    let module = crate::parse_sysml(source).map_err(|error| error.to_string())?;
    fn package(p: &PackageDecl) -> Result<(), String> {
        if !p.imports.is_empty()
            || !p.definitions.is_empty()
            || !p.docs.is_empty()
            || p.modifiers.iter().any(|m| m != "public")
            || p.name.segments.len() != 1
        {
            return Err("Package publication supports plain packages only; imports, documentation, qualified declarations and modifiers are not yet qualified".into());
        }
        for child in &p.members {
            declaration(child)?;
        }
        Ok(())
    }
    fn declaration(d: &Declaration) -> Result<(), String> {
        match d {
            Declaration::Package(p) => package(p),
            _ => Err("Package publication supports only package declarations".into()),
        }
    }
    if !module.imports.is_empty() || !module.definitions.is_empty() {
        return Err("Package publication cannot include imports or definitions".into());
    }
    if let Some(p) = &module.package {
        package(p)?;
    }
    for d in &module.members {
        declaration(d)?;
    }
    Ok(())
}

fn simple_name(name: &str) -> bool {
    match lex(name) {
        Ok(tokens) => {
            matches!(tokens.first().map(|token| &token.kind), Some(TokenKind::Identifier(value)) if value == name)
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        Err(_) => false,
    }
}

pub(super) fn project(document: &KirDocument) -> Result<(Vec<Value>, Value), String> {
    project_with_identity(document, None)
}

pub(super) fn project_with_identity(
    document: &KirDocument,
    identities: Option<&BTreeMap<String, package_identity::Entry>>,
) -> Result<(Vec<Value>, Value), String> {
    if document.elements.is_empty() {
        return Err("A package publication must contain a package".into());
    }
    let by_id: BTreeMap<_, _> = document
        .elements
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect();
    let mut diagnostics = Vec::new();
    let mut ids = exchange_id_map(document, &mut diagnostics);
    if let Some(entries) = identities {
        if entries.len() != document.elements.len()
            || entries.keys().any(|key| !by_id.contains_key(key.as_str()))
        {
            return Err("Identity mapping must cover exactly the current packages".into());
        }
        for (key, entry) in entries {
            ids.insert(key.clone(), entry.element_id.clone());
        }
        if ids.values().any(|id| !is_uuid_like(id))
            || ids.values().collect::<BTreeSet<_>>().len() != ids.len()
        {
            return Err("Invalid or duplicate package identity".into());
        }
    }
    if !diagnostics.is_empty() {
        return Err("Duplicate exchange identity".into());
    }
    let mut all_ids: BTreeSet<String> = ids.values().cloned().collect();
    let mut parents = BTreeMap::new();
    let mut children = BTreeMap::<&str, Vec<&str>>::new();
    for e in &document.elements {
        if metaclass_name(&e.kind) != "Package" {
            return Err(format!("{} is outside the package-tree profile", e.kind));
        }
        for key in e.properties.keys() {
            if ![
                "declared_name",
                "qualified_name",
                "members",
                "owner",
                "metadata",
                "metatype",
                "element_id",
            ]
            .contains(&key.as_str())
            {
                return Err(format!(
                    "Package property `{key}` is outside the qualified profile"
                ));
            }
        }
        let name = e
            .properties
            .get("declared_name")
            .and_then(Value::as_str)
            .ok_or("Packages must have a declared name")?;
        if !simple_name(name) {
            return Err("Package profile currently requires plain identifier names".into());
        }
        if let Some(owner) = e.properties.get("owner") {
            let owner = owner.as_str().ok_or("Package owner must be a KIR ID")?;
            if !by_id.contains_key(owner) {
                return Err("Package owner is outside the document".into());
            }
            parents.insert(e.id.as_str(), owner);
        }
        let mut members = Vec::new();
        if let Some(value) = e.properties.get("members") {
            for member in value.as_array().ok_or("Package members must be an array")? {
                let member = member.as_str().ok_or("Member must be a KIR ID")?;
                if !by_id.contains_key(member) || members.contains(&member) {
                    return Err("Missing or duplicate package member".into());
                }
                members.push(member);
            }
        }
        children.insert(e.id.as_str(), members);
    }
    for e in &document.elements {
        if let Some(owner) = parents.get(e.id.as_str()) {
            if !children[owner].contains(&e.id.as_str()) {
                return Err("Package owner/member links disagree".into());
            }
        }
        for child in &children[e.id.as_str()] {
            if parents.get(child).copied() != Some(e.id.as_str()) {
                return Err("Package member/owner links disagree".into());
            }
        }
        let mut seen = BTreeSet::new();
        let mut cursor = e.id.as_str();
        while let Some(parent) = parents.get(cursor) {
            if !seen.insert(cursor) {
                return Err("Cyclic package ownership".into());
            }
            cursor = parent;
        }
        let mut names = BTreeSet::new();
        for child in &children[e.id.as_str()] {
            if !names.insert(by_id[child].properties["declared_name"].as_str()) {
                return Err("Sibling package names must be distinguishable".into());
            }
        }
    }
    // Whole-project persistence normalizes KIR names using dot-separated paths.
    // Check that redundant field against ownership; derive OMG qualifiedName below.
    for e in &document.elements {
        if let Some(qualified) = e.properties.get("qualified_name") {
            let mut names = vec![
                e.properties["declared_name"]
                    .as_str()
                    .ok_or("Missing name")?,
            ];
            let mut cursor = e.id.as_str();
            while let Some(parent) = parents.get(cursor) {
                names.push(
                    by_id[parent].properties["declared_name"]
                        .as_str()
                        .ok_or("Missing parent name")?,
                );
                cursor = parent;
            }
            names.reverse();
            if qualified.as_str() != Some(names.join(".").as_str()) {
                return Err("Normalized package name disagrees with ownership".into());
            }
        }
    }
    let mut memberships = BTreeMap::new();
    for (child, parent) in &parents {
        let id = match identities {
            Some(entries) => entries[*child]
                .membership_id
                .clone()
                .ok_or("Renames cannot change package ownership")?,
            None => {
                deterministic_exchange_uuid(&format!("omg-package-membership:{parent}:{child}"))
            }
        };
        if !all_ids.insert(id.clone()) {
            return Err("Generated relationship identity collides with a package".into());
        }
        memberships.insert(*child, id);
    }
    let reference = |kir: &str| json!({"@id": ids[kir]});
    let mut elements = Vec::new();
    let mut origins = Map::new();
    for e in &document.elements {
        let name = e.properties["declared_name"].clone();
        let mut value = base(&ids[&e.id], "Package");
        value["declaredName"] = name.clone();
        value["name"] = name;
        // Root Namespace has no qualifiedName. Its direct members start at their own name.
        if parents.contains_key(e.id.as_str()) {
            let mut names = vec![
                e.properties["declared_name"]
                    .as_str()
                    .ok_or("Missing name")?,
            ];
            let mut cursor = e.id.as_str();
            while let Some(parent) = parents.get(cursor) {
                if !parents.contains_key(parent) {
                    break;
                }
                names.push(
                    by_id[parent].properties["declared_name"]
                        .as_str()
                        .ok_or("Missing parent name")?,
                );
                cursor = parent;
            }
            names.reverse();
            value["qualifiedName"] = json!(names.join("::"));
            let parent = parents[e.id.as_str()];
            value["owner"] = reference(parent);
            value["owningNamespace"] = reference(parent);
            value["owningMembership"] = json!({"@id": memberships[e.id.as_str()]});
            value["owningRelationship"] = value["owningMembership"].clone();
        }
        let members: Vec<_> = children[e.id.as_str()]
            .iter()
            .map(|id| reference(id))
            .collect();
        let relationships: Vec<_> = children[e.id.as_str()]
            .iter()
            .map(|id| json!({"@id":memberships[id]}))
            .collect();
        for key in ["ownedMember", "member", "ownedElement"] {
            value[key] = json!(members);
        }
        for key in ["ownedMembership", "membership", "ownedRelationship"] {
            value[key] = json!(relationships);
        }
        for key in ["filterCondition", "importedMembership", "ownedImport"] {
            value[key] = json!([]);
        }
        origins.insert(ids[&e.id].clone(), json!({"kind":"authored", "kirId":e.id,"sourceFile":e.properties.get("metadata").unwrap_or(&Value::Null)["source_file"],"span":e.properties.get("metadata").unwrap_or(&Value::Null)["source_span"]}));
        elements.push(value);
        if let Some(parent) = parents.get(e.id.as_str()) {
            let mut relation = base(&memberships[e.id.as_str()], "OwningMembership");
            relation["isImplied"] = json!(false);
            relation["owningRelatedElement"] = reference(parent);
            relation["membershipOwningNamespace"] = reference(parent);
            relation["memberElement"] = reference(&e.id);
            relation["ownedMemberElement"] = reference(&e.id);
            for key in ["memberElementId", "ownedMemberElementId"] {
                relation[key] = json!(ids[&e.id]);
            }
            for key in ["memberName", "ownedMemberName"] {
                relation[key] = e.properties["declared_name"].clone();
            }
            for key in ["memberShortName", "ownedMemberShortName"] {
                relation[key] = Value::Null;
            }
            relation["source"] = json!([reference(parent)]);
            relation["target"] = json!([reference(&e.id)]);
            relation["relatedElement"] = json!([reference(parent), reference(&e.id)]);
            relation["ownedRelatedElement"] = json!([reference(&e.id)]);
            relation["visibility"] = json!("public");
            origins.insert(memberships[e.id.as_str()].clone(), json!({"kind":"derived", "rule":"package-ownership", "sourceElements":[ids[*parent], ids[&e.id]]}));
            elements.push(relation);
        }
    }
    Ok((elements, Value::Object(origins)))
}
pub(super) fn base(id: &str, kind: &str) -> Value {
    json!({"@id":id,"@type":kind,"aliasIds":[],"elementId":id,"declaredName":null,"declaredShortName":null,
        "name":null,"shortName":null,"qualifiedName":null,"documentation":[],"ownedAnnotation":[],
        "ownedElement":[],"ownedRelationship":[],"owner":null,"owningMembership":null,"owningNamespace":null,
        "owningRelationship":null,"isImpliedIncluded":false,"isLibraryElement":false,"textualRepresentation":[]})
}
#[cfg(test)]
mod tests {
    use super::*;
    fn tree() -> KirDocument {
        let package = |id: &str, name: &str, owner: Option<&str>, members: Vec<&str>| {
            let mut properties = BTreeMap::from([
                ("declared_name".into(), json!(name)),
                ("members".into(), json!(members)),
            ]);
            if let Some(owner) = owner {
                properties.insert("owner".into(), json!(owner));
            }
            KirElement {
                id: id.into(),
                kind: "SysML::Package".into(),
                layer: 2,
                properties,
            }
        };
        KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                package("root", "Root", None, vec!["child"]),
                package("child", "Child", Some("root"), vec!["grandchild"]),
                package("grandchild", "Leaf", Some("child"), vec![]),
            ],
        }
    }
    #[test]
    fn explicit_rename_preserves_authored_and_derived_identity() {
        let original = tree();
        let (elements, origins) = project(&original).unwrap();
        let plan = package_identity::Continuity {
            format: "mercurio.sysml.package-identity.v1".into(),
            base_candidate_digest: "base".into(),
            base: package_identity::IdentityBase {
                elements: elements.clone(),
                origins: serde_json::from_value(origins).unwrap(),
            },
            changes: None,
            renames: vec![package_identity::Rename {
                from: "grandchild".into(),
                to: "renamed".into(),
            }],
        };
        let mut changed = original;
        changed.elements[1]
            .properties
            .insert("members".into(), json!(["renamed"]));
        changed.elements[2].id = "renamed".into();
        changed.elements[2]
            .properties
            .insert("declared_name".into(), json!("Renamed"));
        let report = package_identity::export(&changed, &plan).unwrap();
        assert!(!report.has_errors());
        assert_eq!(
            report.value["elements"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["@id"].clone())
                .collect::<Vec<_>>(),
            elements
                .iter()
                .map(|e| e["@id"].clone())
                .collect::<Vec<_>>()
        );
        assert!(
            report.value["elements"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["name"] == "Renamed")
        );
        let mut invalid = plan.clone();
        invalid.renames.clear();
        assert!(package_identity::export(&changed, &invalid).is_err());
        invalid = plan.clone();
        invalid.renames[0].to = "root".into();
        assert!(package_identity::export(&changed, &invalid).is_err());
        let current = package_identity::IdentityBase {
            elements: serde_json::from_value(report.value["elements"].clone()).unwrap(),
            origins: serde_json::from_value(report.metadata["origins"].clone()).unwrap(),
        };
        assert!(
            package_identity::validate_lineage(&plan, "other-base", &plan.base, &current).is_err()
        );
    }
    #[test]
    fn validates_normalized_names_from_multifile_compilation() {
        let mut doc = tree();
        for (element, name) in
            doc.elements
                .iter_mut()
                .zip(["Root", "Root.Child", "Root.Child.Leaf"])
        {
            element
                .properties
                .insert("qualified_name".into(), json!(name));
        }
        assert!(project(&doc).is_ok());
        doc.elements[1]
            .properties
            .insert("qualified_name".into(), json!("Wrong.Child"));
        assert!(project(&doc).unwrap_err().contains("disagrees"));
    }
    #[test]
    fn derives_membership_and_element_ownership_without_including_relationships_as_members() {
        let (elements, origins) = project(&tree()).unwrap();
        assert_eq!(elements.len(), 5);
        let root = elements.iter().find(|e| e["name"] == "Root").unwrap();
        let child = elements.iter().find(|e| e["name"] == "Child").unwrap();
        let leaf = elements.iter().find(|e| e["name"] == "Leaf").unwrap();
        let membership = elements
            .iter()
            .find(|e| e["@id"] == child["owningMembership"]["@id"])
            .unwrap();
        assert_eq!(root["ownedElement"], root["ownedMember"]);
        assert_eq!(child["owner"]["@id"], root["@id"]);
        assert_eq!(membership["owningRelatedElement"]["@id"], root["@id"]);
        assert_eq!(membership["ownedRelatedElement"][0]["@id"], child["@id"]);
        assert!(membership["owner"].is_null());
        assert!(root["qualifiedName"].is_null());
        assert_eq!(child["qualifiedName"], "Child");
        assert_eq!(leaf["qualifiedName"], "Child::Leaf");
        assert_eq!(
            origins[membership["@id"].as_str().unwrap()]["kind"],
            "derived"
        );
    }
    #[test]
    fn refuses_lost_semantics_and_broken_ownership() {
        for property in ["imports", "documentation", "is_library_element"] {
            let mut doc = tree();
            doc.elements[0]
                .properties
                .insert(property.into(), json!([]));
            assert!(project(&doc).is_err());
        }
        let mut doc = tree();
        doc.elements[1]
            .properties
            .insert("owner".into(), json!("missing"));
        assert!(project(&doc).is_err());
        assert!(validate_source("package P { private package Hidden; }").is_err());
        assert!(validate_source("package P { part def Vehicle; }").is_err());
        assert!(validate_source("package P { package Q; }").is_ok());
    }
    #[test]
    fn standard_discriminators_remove_only_known_language_qualification() {
        assert_eq!(
            metaclass_name("SysML::Systems::PartDefinition"),
            "PartDefinition"
        );
        assert_eq!(metaclass_name("KerML::Kernel::Package"), "Package");
        assert_eq!(metaclass_name("Vendor::Package"), "Vendor::Package");
    }
}
