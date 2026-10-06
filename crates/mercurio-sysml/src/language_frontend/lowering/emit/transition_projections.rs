//! Handwritten ordered membership projection for resolved Transition getters.
//! Ecore supplies endpoint ranges, ancestry, ownership, delegate bindings and enum domains.
//! Getter results neither complete adapter construction nor validate transition constraints.
use super::*;
#[path = "transition_projections_generated.rs"]
mod policy;
pub(super) fn targets<'g>(
    graph: &'g [KirElement],
    owner: &'g KirElement,
    contract: &ecore_model::FeatureContract,
) -> Result<Option<Vec<&'g KirElement>>, Diagnostic> {
    let Some((role, target, delegate)) = policy::rule(contract.field) else {
        return Ok(None);
    };
    if owner.kind.rsplit("::").next() != Some("TransitionUsage") {
        return Ok(None);
    }
    let binding = contract
        .setting_delegate
        .as_ref()
        .ok_or_else(|| error("missing transition getter delegate"))?;
    if contract.owner != "TransitionUsage"
        || binding.uri != "http://www.omg.org/spec/SysML"
        || binding.status != "custom_setting_delegate_source"
        || binding.candidates != [delegate]
        || contract.target != target
        || !contract.derived
    {
        return Err(error("unassessed transition getter contract"));
    }
    if owner.properties.contains_key(contract.field) {
        return Err(error(
            "transition getter requires recomputation, not a stored derived snapshot",
        ));
    }
    if !owner.properties.contains_key("owned_relationship") {
        return Err(error(
            "transition projection requires canonical relationship storage",
        ));
    }
    let mut result = Vec::new();
    for m in stored_children(graph, owner, "owned_relationship")? {
        if !metaclass_conforms(&m.kind, "OwningMembership") {
            continue;
        }
        let StoredMembershipEndpoint::Resolved(e) =
            stored_membership_endpoint(graph, m, "member_element")?
        else {
            return Err(error("unresolved transition owned member"));
        };
        let endpoint =
            ecore_model::redefined_feature(&m.kind, "owned_member_element").map_err(error)?;
        ecore_model::validate_reference_endpoint(&m.kind, endpoint.field, &e.kind)
            .map_err(error)?;
        if role.is_empty() {
            if metaclass_conforms(&e.kind, target) {
                return Ok(Some(vec![e]));
            }
            continue;
        }
        if !metaclass_conforms(&m.kind, "TransitionFeatureMembership") {
            continue;
        }
        let kind=m.properties.get("kind").ok_or_else(||error("transition membership kind requires an explicit resolved value; fresh Pilot null is not an enum default"))?;
        ecore_model::validate_value(&m.kind, "kind", kind).map_err(error)?;
        if kind.as_str() == Some(role)
            && metaclass_conforms(&e.kind, target)
            && !result.iter().any(|prior: &&KirElement| prior.id == e.id)
        {
            result.push(e);
        }
    }
    Ok(Some(result))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn own(g: &mut Vec<KirElement>, mut m: KirElement, mut e: KirElement) {
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut m, &mut e)
            .unwrap();
        ecore_ownership::attach(
            &ecore_ownership_generated::OWNED_RELATIONSHIPS,
            g.iter_mut().find(|e| e.id == "transition").unwrap(),
            &mut m,
        )
        .unwrap();
        g.extend([m, e]);
    }
    fn fixture(c: &Value) -> Vec<KirElement> {
        let mut g = vec![fresh_definition_element("transition".into(), "TransitionUsage").unwrap()];
        for id in if c["reverse"] == true {
            ["c", "b", "a"]
        } else {
            ["a", "b", "c"]
        } {
            let mut e = fresh_definition_element(id.into(), c["kind"].as_str().unwrap()).unwrap();
            e.properties.insert("declared_name".into(), json!(id));
            let mut m =
                fresh_definition_element(format!("{id}.member"), c["membership"].as_str().unwrap())
                    .unwrap();
            if c["membership"] == "TransitionFeatureMembership" {
                if c["role"] == "unset" {
                    m.properties.remove("kind");
                } else {
                    m.properties.insert("kind".into(), c["role"].clone());
                }
            }
            if c["membership"] == "Membership" {
                m.properties.insert("member_element".into(), json!(id));
                ecore_ownership::attach(
                    &ecore_ownership_generated::OWNED_RELATIONSHIPS,
                    &mut g[0],
                    &mut m,
                )
                .unwrap();
                g.extend([m, e]);
            } else {
                own(&mut g, m, e);
            }
        }
        g
    }
    #[test]
    fn definition_transition_projections_match_all_pilot_controls() {
        let data: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/conformance/2026-08-support/transition-projection-pilot-controls.json"
        )))
        .unwrap();
        assert_eq!(data["controls"].as_array().unwrap().len(), 2528);
        for c in data["controls"].as_array().unwrap() {
            let mut g = fixture(c);
            for replay in [false, true] {
                if replay {
                    g.reverse();
                    g = serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
                }
                let owner = g.iter().find(|e| e.id == "transition").unwrap();
                for field in [
                    "trigger_action",
                    "guard_expression",
                    "effect_action",
                    "succession",
                ] {
                    let result = definition_reference_targets(&g, owner, field);
                    if c["rejected"] == true {
                        assert!(result.is_err(), "{c}: {field}");
                        continue;
                    }
                    if c["membership"] == "TransitionFeatureMembership"
                        && c["role"] == "unset"
                        && field != "succession"
                    {
                        assert!(result
                            .unwrap_err()
                            .message
                            .contains("explicit resolved value"));
                        continue;
                    }
                    let ids = result
                        .unwrap_or_else(|e| panic!("{c}: {field}: {e:?}"))
                        .iter()
                        .map(|e| e.properties["declared_name"].clone())
                        .collect::<Vec<_>>();
                    assert_eq!(Value::Array(ids), c["result"][field], "{c}: {field}");
                }
            }
        }
    }
    #[test]
    fn definition_transition_projections_preserve_mixed_role_order_and_dependencies() {
        let c = json!({"kind":"AcceptActionUsage","membership":"TransitionFeatureMembership","role":"trigger","reverse":false});
        let mut g = fixture(&c);
        g.iter_mut()
            .find(|e| e.id == "b.member")
            .unwrap()
            .properties
            .insert("kind".into(), json!("effect"));
        let owner = g.iter().find(|e| e.id == "transition").unwrap();
        let ids = definition_reference_targets(&g, owner, "trigger_action")
            .unwrap()
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["a", "c"]);
        assert_eq!(
            definition_reference_targets(&g, owner, "effect_action").unwrap()[0].id,
            "b"
        );
        assert!(definition_reference_targets(&g, owner, "guard_expression")
            .unwrap()
            .is_empty());
        let before = serde_json::to_value(&g).unwrap();
        definition_reference_targets(&g, owner, "succession").unwrap();
        assert_eq!(serde_json::to_value(&g).unwrap(), before);
        let mut bad = g.clone();
        bad.iter_mut()
            .find(|e| e.id == "b.member")
            .unwrap()
            .properties
            .insert("kind".into(), json!("invalid"));
        assert!(definition_reference_targets(
            &bad,
            bad.iter().find(|e| e.id == "transition").unwrap(),
            "trigger_action"
        )
        .is_err());
        let mut bad = g.clone();
        bad.iter_mut()
            .find(|e| e.id == "transition")
            .unwrap()
            .properties
            .insert("trigger_action".into(), json!(["b"]));
        assert!(definition_reference_targets(
            &bad,
            bad.iter().find(|e| e.id == "transition").unwrap(),
            "trigger_action"
        )
        .is_err());
        let mut bad = g.clone();
        bad.iter_mut()
            .find(|e| e.id == "transition")
            .unwrap()
            .properties
            .remove("owned_relationship");
        assert!(definition_reference_targets(
            &bad,
            bad.iter().find(|e| e.id == "transition").unwrap(),
            "trigger_action"
        )
        .is_err());
    }
    #[test]
    fn definition_transition_projections_reach_public_queries_and_abstract_syntax_roundtrip() {
        use crate::abstract_syntax_json::{
            export_sysml_abstract_syntax_value, import_sysml_abstract_syntax_value,
        };
        let mut graph = fixture(
            &json!({"kind":"AcceptActionUsage","membership":"TransitionFeatureMembership","role":"trigger","reverse":false}),
        );
        graph
            .iter_mut()
            .find(|e| e.id == "b.member")
            .unwrap()
            .properties
            .insert("kind".into(), json!("effect"));
        let mut succession = fresh_definition_element("succession".into(), "Succession").unwrap();
        succession
            .properties
            .insert("declared_name".into(), json!("succession"));
        own(
            &mut graph,
            fresh_definition_element("succession.member".into(), "OwningMembership").unwrap(),
            succession,
        );
        let document = crate::KirDocument {
            elements: graph,
            metadata: [("semantic_validation".into(), json!("not_assessed"))]
                .into_iter()
                .collect(),
        };
        let read = |doc: &crate::KirDocument, field: &str| {
            crate::definition_document::reference_targets(doc, "transition", field)
                .unwrap()
                .into_iter()
                .map(|e| e.id.clone())
                .collect::<Vec<_>>()
        };
        let fields = [
            "trigger_action",
            "guard_expression",
            "effect_action",
            "succession",
        ];
        let expected = fields.map(|field| read(&document, field));
        assert_eq!(
            expected,
            [
                vec!["a".to_owned(), "c".to_owned()],
                vec![],
                vec!["b".to_owned()],
                vec!["succession".to_owned()]
            ]
        );
        let exported = export_sysml_abstract_syntax_value(&document, Default::default()).unwrap();
        assert!(!exported.has_errors());
        let imported =
            import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(!imported.has_errors());
        let mut restored = imported.persistable_document().unwrap();
        restored.elements.reverse();
        assert_eq!(fields.map(|field| read(&restored, field)), expected);
        assert!(restored
            .elements
            .iter()
            .all(|e| !e.properties.contains_key("trigger_action")
                && !e.properties.contains_key("guard_expression")
                && !e.properties.contains_key("effect_action")
                && !e.properties.contains_key("succession")));
    }
}
