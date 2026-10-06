// Focused shared lifecycle controls. Fixtures supply structure; production
// producers establish each admitted canonical role before the lifecycle job.
pub(super) fn chain_lifecycle_value_fixture()->(crate::KirDocument,Vec<String>,Vec<String>) {
    let source="standard library package Base { abstract classifier Anything; feature things { feature that; } }
        standard library package Links { assoc Link { feature participant; } feature selfLinks; }
        standard library package Performances { abstract expr evaluations { return libraryResult; } }
        standard library package Occurrences { class Occurrence { feature startShot; } }
        package P { classifier T; class A { feature y : T; out feature x : T = y; out feature z : T = y; } }";
    let mut doc=crate::definition_document::parse_and_link(source,crate::SourceLanguage::Kerml).unwrap();
    let mut expressions=doc.elements.iter().filter(|node|node.kind.rsplit("::").next()==Some("FeatureReferenceExpression"))
        .map(|node|node.id.clone()).collect::<Vec<_>>();expressions.sort();
    for (n,id) in expressions.iter().enumerate() {
        crate::definition_document::materialize_expression_result(&mut doc,id,&format!("return.member.{n}"),&format!("return.{n}")).unwrap();
        reference_result_scope::materialize(&mut doc.elements,id,&format!("return.subsetting.{n}")).unwrap();
    }
    let mut owners=doc.elements.iter().filter(|node|["x","z"].iter().any(|name|node.properties.get("declared_name")==Some(&json!(name))))
        .map(|node|node.id.clone()).collect::<Vec<_>>();owners.sort();
    let values={let index=doc.elements.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&doc.elements,&index);
        owners.iter().map(|id|general_value_provider::inputs(&query,index[id.as_str()]).unwrap()[0].valuation.id.clone()).collect::<Vec<_>>()};
    doc.elements.iter_mut().find(|node|node.id==values[0]).unwrap().properties.insert("is_initial".into(),json!(true));
    general_value_binding::materialize_batch(&mut doc.elements,&owners).unwrap();
    let mut chains=values.iter().map(|id|format!("{id}.implicit.value-binding.source")).collect::<Vec<_>>();
    chains.push(format!("{}.implicit.value-binding.initial-context",values[0]));chains.sort();
    (doc,owners,chains)
}

#[test]
fn definition_chain_lifecycle_value_contexts_complete_and_replay_after_persistence() {
    let (mut doc,owners,chains)=chain_lifecycle_value_fixture();let original=doc.elements.clone();
    let job=Prerequisite::ChainLifecycleBatch{owner_ids:chains.clone()};
    assert_eq!(serde_json::from_value::<Prerequisite>(serde_json::to_value(&job).unwrap()).unwrap(),job);
    let (graph,pending)=link_generated_graph_planned(doc.elements,Vec::new(),&|_|Err(error("unexpected lexical read")),true,&[],&|_|{},Some(&[job])).unwrap();
    assert!(pending.is_empty());doc.elements=graph;
    for node in &original {
        let after=doc.elements.iter().find(|after|after.id==node.id).unwrap();
        if chains.contains(&node.id) {
            assert!(!scope_boolean(node,"is_implied_included").unwrap());assert!(scope_boolean(after,"is_implied_included").unwrap());
            let mut expected=node.clone();expected.properties.insert("is_implied_included".into(),json!(true));
            let relationships=after.properties["owned_relationship"].as_array().unwrap();assert_eq!(relationships.len(),3);
            expected.properties.insert("owned_relationship".into(),json!(relationships));assert_eq!(&expected,after);
        }else{assert_eq!(node,after);}
    }
    assert_eq!(doc.elements.len(),original.len()+3);
    assert!(ecore_model::validate_publication(&doc.elements,ecore_model::ReferenceCompleteness::Closed).is_empty());
    let exported=crate::abstract_syntax_json::export_sysml_abstract_syntax_value(&doc,Default::default()).unwrap();assert!(!exported.has_errors());
    let imported=crate::abstract_syntax_json::import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!imported.has_errors());
    let mut restored=imported.persistable_document().unwrap();restored.elements.reverse();
    let before=serde_json::to_value(&restored).unwrap();
    assert_eq!(canonical_chain_lifecycle::materialize_batch(&mut restored.elements,&chains).unwrap(),0);
    assert_eq!(general_value_binding::materialize_batch(&mut restored.elements,&owners).unwrap(),0);
    assert_eq!(serde_json::to_value(&restored).unwrap(),before);
}

#[test]
fn definition_chain_lifecycle_result_and_crossing_contexts_share_the_proof() {
    let mut graph=vec![fresh_definition_element("chain".into(),"FeatureChainExpression").unwrap(),fresh_definition_element("library".into(),"LibraryPackage").unwrap()];
    graph[1].properties.insert("declared_name".into(),json!("ControlFunctions"));graph[1].properties.insert("is_standard".into(),json!(true));
    cross_test_own(&mut graph,"library","OwningMembership",fresh_definition_element("operator".into(),"Function").unwrap());
    cross_test_own(&mut graph,"operator","ParameterMembership",fresh_definition_element("source".into(),"Feature").unwrap());
    cross_test_own(&mut graph,"source","FeatureMembership",fresh_definition_element("role".into(),"Feature").unwrap());
    for (id,name) in [("operator","."),("source","source"),("role","target")] {graph.iter_mut().find(|node|node.id==id).unwrap().properties.insert("declared_name".into(),json!(name));}
    cross_test_own(&mut graph,"chain","ParameterMembership",fresh_definition_element("input".into(),"Feature").unwrap());
    cross_test_own(&mut graph,"input","FeatureMembership",fresh_definition_element("source.target".into(),"Feature").unwrap());
    cross_test_own(&mut graph,"chain","ReturnParameterMembership",fresh_definition_element("result".into(),"Feature").unwrap());
    cross_test_own(&mut graph,"chain","OwningMembership",fresh_definition_element("target".into(),"Feature").unwrap());
    graph.iter_mut().find(|node|node.id=="source").unwrap().properties.insert("direction".into(),json!("in"));
    graph.iter_mut().find(|node|node.id=="input").unwrap().properties.insert("direction".into(),json!("in"));
    graph.iter_mut().find(|node|node.id=="result").unwrap().properties.insert("direction".into(),json!("out"));
    let library=crate::definition_document::parse_and_link("standard library package Base { feature things; } standard library package Performances { function Evaluation; feature evaluations; }",crate::SourceLanguage::Kerml).unwrap();graph.extend(library.elements);
    // Exact target-role containment admits selector reads, while an unrelated
    // Feature in the same expression namespace retains its dependency boundary.
    {let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&graph,&index);
        assert_eq!(definition_feature_default_name_in_view(&query,index["target"]).unwrap(),"Base::things");}
    let mut unrelated=graph.clone();cross_test_own(&mut unrelated,"chain","OwningMembership",fresh_definition_element("unrelated".into(),"Feature").unwrap());
    {let index=unrelated.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&unrelated,&index);
        assert!(matches!(definition_feature_default_name_in_view(&query,index["unrelated"]),Err(QueryFailure::Rejected(_))));}
    chain_specialization::materialize(&mut graph,"chain","result-stage").unwrap();
    assert_eq!(canonical_chain_lifecycle::materialize_batch(&mut graph,&["result-stage.result-chain".into()]).unwrap(),1);
    let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&graph,&index);
    assert!(chain_specialization::adopted_result_chain(&query,index["result-stage.result-chain"]).unwrap());
    assert!(!scope_boolean(index["chain"],"is_implied_included").unwrap());
    let before=serde_json::to_value(&graph).unwrap();assert!(!chain_specialization::materialize(&mut graph,"chain","other-stage").unwrap());assert_eq!(serde_json::to_value(&graph).unwrap(),before);
    let mut cross=cross_test_graph(&[true,true],true);
    // Full default/type reads use a supported Class-owned end context;
    // the earlier crossing-only fixture deliberately used a bare Classifier.
    cross[0].kind="SysML::Class".into();
    cross_test_own(&mut cross,"end0","OwningMembership",fresh_definition_element("cross".into(),"Feature").unwrap());
    // Reuse the complete focused library bundle, including the owning Class
    // default/featuring dependency on Occurrences, rather than a crossing-only
    // Base fragment that cannot support full type/featuring reads.
    cross.extend(chain_lifecycle_value_fixture().0.elements);
    definition_materialize_owned_cross_specialization_query(&mut cross,"cross","cross-generals").unwrap();
    definition_materialize_binary_crossing_query(&mut cross,"end0","cross-stage").unwrap();
    assert_eq!(canonical_chain_lifecycle::materialize_batch(&mut cross,&["cross-stage.chain".into()]).unwrap(),1);
    let before=serde_json::to_value(&cross).unwrap();assert!(!definition_materialize_binary_crossing_query(&mut cross,"end0","other-stage").unwrap());assert_eq!(serde_json::to_value(&cross).unwrap(),before);
}

#[test]
fn definition_chain_lifecycle_rejects_unknown_contributions_and_forged_completion_atomically() {
    let (original,_,chains)=chain_lifecycle_value_fixture();
    for mutation in ["completion","unknown","wrong-default","shortened","reordered","self","flags"] {
        let mut doc=original.clone();let owner=&chains[1];
        match mutation {
            "completion"=>{doc.elements.iter_mut().find(|node|node.id==*owner).unwrap().properties.insert("is_implied_included".into(),json!(true));},
            "unknown"=>{let mut child=fresh_definition_element("unexpected".into(),"OwningMembership").unwrap();ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,doc.elements.iter_mut().find(|node|node.id==*owner).unwrap(),&mut child).unwrap();doc.elements.push(child);},
            "wrong-default"=>{definition_append_implied_specialization(&mut doc.elements,"wrong-general","Subsetting",owner,&chains[0]).unwrap();},
            "shortened"=>{doc.elements.iter_mut().find(|node|node.id==*owner).unwrap().properties.insert("owned_relationship".into(),json!([format!("{owner}.chain.0")]));},
            "reordered"=>{let child=doc.elements.iter_mut().find(|node|node.id==*owner).unwrap();child.properties.get_mut("owned_relationship").unwrap().as_array_mut().unwrap().reverse();},
            "self"=>{doc.elements.iter_mut().find(|node|node.id==format!("{owner}.chain.1")).unwrap().properties.insert("chaining_feature".into(),json!(owner));},
            _=>{doc.elements.iter_mut().find(|node|node.id==*owner).unwrap().properties.insert("is_variable".into(),json!(true));},
        }
        let before=serde_json::to_value(&doc).unwrap();assert!(matches!(canonical_chain_lifecycle::materialize_batch(&mut doc.elements,&chains),Err(QueryFailure::Rejected(_))),"{mutation}");assert_eq!(serde_json::to_value(&doc).unwrap(),before,"{mutation}");
    }
}

#[test]
fn definition_chain_lifecycle_pending_endpoints_remain_typed_and_no_partial_flags_are_written() {
    let (original,_,chains)=chain_lifecycle_value_fixture();let missing=format!("{}.chain.1",chains[2]);let mut doc=original.clone();
    let target=doc.elements.iter_mut().find(|node|node.id==missing).unwrap().properties.remove("chaining_feature").unwrap();
    let before=serde_json::to_value(&doc).unwrap();
    assert!(matches!(canonical_chain_lifecycle::materialize_batch(&mut doc.elements,&chains),Err(QueryFailure::Required(Prerequisite::ReadField{owner_id,field})) if owner_id==missing&&field=="chaining_feature"));
    assert_eq!(serde_json::to_value(&doc).unwrap(),before);
    doc.elements.iter_mut().find(|node|node.id==missing).unwrap().properties.insert("chaining_feature".into(),target);
    assert_eq!(canonical_chain_lifecycle::materialize_batch(&mut doc.elements,&chains).unwrap(),3);
}

#[test]
fn definition_chain_lifecycle_late_identity_collision_rolls_back_the_complete_group() {
    let (mut doc,_,chains)=chain_lifecycle_value_fixture();
    doc.elements.push(fresh_definition_element(format!("{}.implicit.chain-default.0",chains[2]),"Feature").unwrap());
    let before=serde_json::to_value(&doc).unwrap();
    let error=canonical_chain_lifecycle::materialize_batch(&mut doc.elements,&chains).unwrap_err();
    assert!(matches!(error,QueryFailure::Rejected(ref issue) if issue.message.contains("identity")||issue.message.contains("fresh")||issue.message.contains("collision")),"{error:?}");
    assert_eq!(serde_json::to_value(&doc).unwrap(),before);
}


#[test]
fn definition_chain_lifecycle_binary_cross_featuring_reads_actual_types_and_retains_pending_and_nary_boundaries() {
    let mut graph=cross_test_graph(&[true,true],true);graph[0].kind="SysML::Class".into();
    cross_test_own(&mut graph,"end0","OwningMembership",fresh_definition_element("cross".into(),"Feature").unwrap());
    graph.extend(chain_lifecycle_value_fixture().0.elements);
    let target=graph.iter().find(|node|node.properties.get("declared_name")==Some(&json!("T"))).unwrap().id.clone();
    definition_append_implied_specialization(&mut graph,"other-end.type","FeatureTyping","end1",&target).unwrap();
    // It is an explicit input, independently of any lifecycle flag.
    graph.iter_mut().find(|node|node.id=="other-end.type").unwrap().properties.insert("is_implied".into(),json!(false));
    let before=serde_json::to_value(&graph).unwrap();
    {let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&graph,&index);
        assert_eq!(definition_featuring_types_in_view(&query,index["cross"]).unwrap().iter().map(|node|node.id.clone()).collect::<Vec<_>>(),vec![target.clone()]);}
    assert_eq!(serde_json::to_value(&graph).unwrap(),before);
    let mut pending=graph.clone();pending.iter_mut().find(|node|node.id=="other-end.type").unwrap().properties.remove("type");
    let before=serde_json::to_value(&pending).unwrap();
    {let index=pending.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&pending,&index);
        assert!(matches!(definition_featuring_types_in_view(&query,index["cross"]),Err(QueryFailure::Required(Prerequisite::ReadField{owner_id,field})) if owner_id=="other-end.type"&&field=="type"));}
    assert_eq!(serde_json::to_value(&pending).unwrap(),before);
    let mut nary=graph.clone();let mut extra=fresh_definition_element("end2".into(),"Feature").unwrap();extra.properties.insert("is_end".into(),json!(true));cross_test_own(&mut nary,"context","FeatureMembership",extra);
    {let index=nary.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&nary,&index);
        assert!(matches!(definition_featuring_types_in_view(&query,index["cross"]),Err(QueryFailure::Rejected(ref issue)) if issue.message.contains("Cartesian-product")));}
}
