//! Handwritten read-only execution of the resolved Transition source program.
//! Additional-member readiness is proved only when the actual producer makes no edits.
//! Connector-producing branches remain unsupported; no completion flag is inserted.
use super::*;
#[path = "transition_source_query_generated.rs"]
mod policy;
pub(super) fn ready(graph:&[KirElement],owner:&KirElement)->Result<Option<bool>,Diagnostic> {
    ready_query(graph,owner).map_err(QueryFailure::into_diagnostic)
}
pub(super) fn ready_query(graph: &[KirElement], owner: &KirElement) -> Result<Option<bool>, QueryFailure> {
    let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
    ready_in_view(&DefinitionNameQuery::new(graph,&index),owner)
}

pub(super) fn ready_in_view(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement) -> Result<Option<bool>, QueryFailure> {
    let graph=query.graph;
    if owner.kind.rsplit("::").next() != Some("TransitionUsage") {
        return Ok(None);
    }
    if !transition_source::membership_stable_in_view(query,owner)? {return Ok(Some(false));}
    if definition_transition_link_feature(graph, owner)?.is_some() {
        return Ok(Some(true));
    }
    // Without a link, the producer is a no-op only if both connector branches are absent.
    if !definition_reference_targets_typed_query(query, owner, "succession")?.is_empty() {
        return Ok(Some(false));
    }
    if !definition_chain_reference_targets_typed_query(query, owner, "chaining_feature")?.is_empty() {
        return Ok(Some(false));
    }
    for f in definition_owned_features_query(query, owner, false)? {
        if definition_has_direction(f)? {
            return Ok(Some(false));
        }
    }
    Ok(Some(true))

}
pub(crate) fn definition_transition_source_feature<'g>(graph:&'g [KirElement],owner:&'g KirElement)->Result<Option<&'g KirElement>,Diagnostic> {
    source_feature_query(graph,owner).map_err(QueryFailure::into_diagnostic)
}
pub(super) fn source_feature_query<'g>(
    graph: &'g [KirElement],
    owner: &'g KirElement,
) -> Result<Option<&'g KirElement>, QueryFailure> {
    let _resolved_program = policy::RESOLVED_PROGRAM_SHA;
    if ready_query(graph, owner)? != Some(true) {
        return Err(query_failure("Transition source query requires additional-member construction; connector-producing branches remain unsupported"));
    }
    source_feature_after_source_stage(graph,owner)
}
// Pure source projection for default predicates. Connector producers cannot
// change the resolved first source membership. The public source delegate keeps
// its complete additional-member requirement; no readiness flag is fabricated.
pub(super) fn source_feature_for_defaults_query<'g>(graph:&'g [KirElement],owner:&'g KirElement)->Result<Option<&'g KirElement>,QueryFailure> {
    if !transition_source::membership_stable_query(graph,owner)? {
        return Err(query_failure("Transition default source requires additional-member construction of the source membership"));
    }
    source_feature_after_source_stage(graph,owner)
}
// Private producer-stage navigation. Callers must construct/resolve the source
// membership first; this does not assert connector or transformation completion.
pub(super) fn source_feature_after_source_stage<'g>(graph:&'g [KirElement],owner:&'g KirElement)
    -> Result<Option<&'g KirElement>,QueryFailure> {
    for m in stored_children(graph, owner, "owned_relationship")? {
        if !metaclass_conforms(&m.kind, "Membership")
            || metaclass_conforms(&m.kind, "FeatureMembership")
        {
            continue;
        }
        let Some(e) = transition_source::member_query(graph, m)? else {
            continue;
        };
        if !metaclass_conforms(&e.kind, "Feature") {
            continue;
        }
        let target = definition_chain_reference_targets(graph, e, "feature_target")?;
        if target
            .first()
            .is_some_and(|t| metaclass_conforms(&t.kind, policy::FILTER_TARGET))
        {
            return Ok(Some(e));
        }
    }
    Ok(None)
}
pub(super) fn source<'g>(
    graph: &'g [KirElement],
    owner: &'g KirElement,
    contract: &ecore_model::FeatureContract,
) -> Result<Vec<&'g KirElement>, Diagnostic> {
    let b = contract
        .setting_delegate
        .as_ref()
        .ok_or_else(|| error("missing Transition source delegate"))?;
    if contract.owner != "TransitionUsage"
        || !contract.derived
        || contract.target != policy::FILTER_TARGET
        || b.uri != "http://www.omg.org/spec/SysML"
        || b.status != "custom_setting_delegate_source"
        || b.candidates != [policy::SOURCE_DELEGATE]
    {
        return Err(error("unassessed Transition source delegate"));
    }
    if owner.properties.contains_key(contract.field) {
        return Err(error(
            "Transition source requires recomputation, not a derived snapshot",
        ));
    }
    let Some(raw) = definition_transition_source_feature(graph, owner)? else {
        return Ok(vec![]);
    };
    definition_chain_reference_targets(graph, raw, "feature_target")
}
#[cfg(test)]
mod tests {
    use super::*;
    fn own(g: &mut Vec<KirElement>, owner: &str, mut m: KirElement, mut e: KirElement) {
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut m, &mut e)
            .unwrap();
        ecore_ownership::attach(
            &ecore_ownership_generated::OWNED_RELATIONSHIPS,
            g.iter_mut().find(|e| e.id == owner).unwrap(),
            &mut m,
        )
        .unwrap();
        g.extend([m, e]);
    }
    fn fixture(c: &Value) -> Vec<KirElement> {
        let mut g = vec![fresh_definition_element("transition".into(), "TransitionUsage").unwrap()];
        let mut e =
            fresh_definition_element("candidate".into(), c["kind"].as_str().unwrap()).unwrap();
        e.properties
            .insert("declared_name".into(), json!("candidate"));
        let mut m =
            fresh_definition_element("candidate.member".into(), c["membership"].as_str().unwrap())
                .unwrap();
        if c["membership"] == "Membership" {
            m.properties
                .insert("member_element".into(), json!("candidate"));
            ecore_ownership::attach(
                &ecore_ownership_generated::OWNED_RELATIONSHIPS,
                &mut g[0],
                &mut m,
            )
            .unwrap();
            g.extend([m, e]);
        } else {
            own(&mut g, "transition", m, e);
        }
        if c["chain"] != "none" {
            g.push(
                fresh_definition_element(
                    "last".into(),
                    if c["chain"] == "action_last" {
                        "StateUsage"
                    } else {
                        "Feature"
                    },
                )
                .unwrap(),
            );
            let mut r = fresh_definition_element("chain".into(), "FeatureChaining").unwrap();
            r.properties
                .insert("chaining_feature".into(), json!("last"));
            ecore_ownership::attach(
                &ecore_ownership_generated::OWNED_RELATIONSHIPS,
                g.iter_mut().find(|e| e.id == "candidate").unwrap(),
                &mut r,
            )
            .unwrap();
            g.push(r);
        }
        g.push(fresh_definition_element("fallback".into(), "StateUsage").unwrap());
        let mut m = fresh_definition_element("fallback.member".into(), "Membership").unwrap();
        m.properties
            .insert("member_element".into(), json!("fallback"));
        ecore_ownership::attach(
            &ecore_ownership_generated::OWNED_RELATIONSHIPS,
            &mut g[0],
            &mut m,
        )
        .unwrap();
        g.push(m);
        if c["link"] == true {
            own(
                &mut g,
                "transition",
                fresh_definition_element("link.member".into(), "FeatureMembership").unwrap(),
                fresh_definition_element("link".into(), "ReferenceUsage").unwrap(),
            );
        }
        g
    }
    #[test]
    fn definition_transition_source_queries_match_pilot_noop_and_boundary_controls() {
        let d: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/conformance/2026-08-support/transition-source-query-pilot-controls.json"
        )))
        .unwrap();
        assert_eq!(d["controls"].as_array().unwrap().len(), 1422);
        for c in d["controls"].as_array().unwrap() {
            let mut g = fixture(c);
            for replay in [false, true] {
                if replay {
                    g.reverse();
                    g = serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
                }
                let before = serde_json::to_value(&g).unwrap();
                let owner = g.iter().find(|e| e.id == "transition").unwrap();
                assert_eq!(
                    definition_additional_members_ready(&g, owner).unwrap(),
                    c["noop"].as_bool().unwrap(),
                    "{c}"
                );
                let raw = definition_transition_source_feature(&g, owner);
                let source = definition_reference_targets(&g, owner, "source");
                if c["noop"] == false {
                    assert!(raw
                        .unwrap_err()
                        .message
                        .contains("additional-member construction"));
                    assert!(source.is_err());
                } else {
                    assert_eq!(
                        raw.unwrap().map(|e| json!(e.id)).unwrap_or(Value::Null),
                        c["raw"],
                        "{c}"
                    );
                    assert_eq!(
                        source
                            .unwrap()
                            .first()
                            .map(|e| json!(e.id))
                            .unwrap_or(Value::Null),
                        c["source"],
                        "{c}"
                    );
                }
                assert_eq!(serde_json::to_value(&g).unwrap(), before);
            }
        }
    }
    #[test]
    fn definition_transition_default_source_projection_matches_pilot_without_completing_connectors() {
        let d:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/conformance/2026-08-support/transition-source-query-pilot-controls.json"))).unwrap();let mut pending_connectors=0;
        for c in d["controls"].as_array().unwrap() {
            let mut graph=fixture(c);
            for replay in [false,true] {
                if replay {graph.reverse();graph=serde_json::from_value(serde_json::to_value(&graph).unwrap()).unwrap();}
                let before=serde_json::to_value(&graph).unwrap();let owner=graph.iter().find(|e|e.id=="transition").unwrap();
                assert!(transition_source::membership_stable_query(&graph,owner).unwrap());
                assert_eq!(json!(source_feature_for_defaults_query(&graph,owner).unwrap().map(|e|e.id.as_str())),c["raw"],"{c}");
                assert_eq!(ready(&graph,owner).unwrap(),Some(c["noop"].as_bool().unwrap()));
                if c["noop"]==false {assert!(definition_transition_source_feature(&graph,owner).is_err());pending_connectors+=usize::from(!replay);}
                assert_eq!(serde_json::to_value(&graph).unwrap(),before);
            }
        }
        assert_eq!(pending_connectors,12);
    }
    #[test]
    fn definition_transition_noop_readiness_rejects_real_producers_and_snapshots() {
        let c = json!({"kind":"StateUsage","membership":"Membership","chain":"none","link":false});
        let g = fixture(&c);
        let mut parameter = g.clone();
        let mut e = fresh_definition_element("parameter".into(), "ReferenceUsage").unwrap();
        e.properties.insert("direction".into(), json!("in"));
        own(
            &mut parameter,
            "transition",
            fresh_definition_element("parameter.member".into(), "ParameterMembership").unwrap(),
            e,
        );
        assert_eq!(ready(&parameter, &parameter[0]).unwrap(), Some(false));
        let mut incomplete = g.clone();
        incomplete
            .iter_mut()
            .find(|e| e.id == "candidate.member")
            .unwrap()
            .properties
            .insert("member_element".into(), Value::Null);
        assert_eq!(ready(&incomplete, &incomplete[0]).unwrap(), Some(false));
        let mut pending = g.clone();
        pending
            .iter_mut()
            .find(|e| e.id == "candidate.member")
            .unwrap()
            .properties
            .remove("member_element");
        assert!(ready(&pending, &pending[0]).is_err());
        let mut snapshot = g.clone();
        snapshot[0]
            .properties
            .insert("source".into(), json!("fallback"));
        assert!(definition_reference_targets(&snapshot, &snapshot[0], "source").is_err());
        assert!(g
            .iter()
            .all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
    }
    #[test]
    fn definition_transition_source_queries_reach_public_api_and_persisted_chains() {
        use crate::abstract_syntax_json::{
            export_sysml_abstract_syntax_value, import_sysml_abstract_syntax_value,
        };
        let g = fixture(
            &json!({"kind":"Feature","membership":"Membership","chain":"action_last","link":true}),
        );
        let doc = crate::KirDocument {
            elements: g,
            metadata: [("semantic_validation".into(), json!("not_assessed"))]
                .into_iter()
                .collect(),
        };
        assert_eq!(
            crate::definition_document::transition_source_feature(&doc, "transition")
                .unwrap()
                .unwrap()
                .id,
            "candidate"
        );
        assert_eq!(
            crate::definition_document::reference_targets(&doc, "transition", "source").unwrap()[0]
                .id,
            "last"
        );
        let exported = export_sysml_abstract_syntax_value(&doc, Default::default()).unwrap();
        assert!(!exported.has_errors());
        let imported =
            import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(!imported.has_errors());
        let mut restored = imported.persistable_document().unwrap();
        restored.elements.reverse();
        assert_eq!(
            crate::definition_document::transition_source_feature(&restored, "transition")
                .unwrap()
                .unwrap()
                .id,
            "candidate"
        );
        assert_eq!(
            crate::definition_document::reference_targets(&restored, "transition", "source")
                .unwrap()[0]
                .id,
            "last"
        );
        assert!(restored
            .elements
            .iter()
            .all(|e| !e.properties.contains_key("source")
                && e.properties.get("is_implied_included") != Some(&json!(true))));
        let mut duplicate = doc.clone();
        duplicate.elements.push(duplicate.elements[0].clone());
        assert!(
            crate::definition_document::transition_source_feature(&duplicate, "transition")
                .is_err()
        );
    }
}
