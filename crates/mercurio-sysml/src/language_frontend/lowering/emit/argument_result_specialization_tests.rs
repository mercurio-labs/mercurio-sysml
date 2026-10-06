// Scoped prepared prototypes support focused producer controls; they do not
// replace the 94 unprepared pinned library resources in release qualification.
fn argument_rule_fixture(kind:&str,shape:&str,collection:bool)->Vec<KirElement> {
    let mut graph=relative_fixture(&json!({"kind":kind,"shape":shape,"direction":"in"}));
    graph.iter_mut().find(|node|node.id=="function").unwrap().properties.insert("declared_name".into(),json!(if kind=="IndexExpression" {"#"}else{"select"}));
    graph.iter_mut().find(|node|node.id=="namespace").unwrap().properties.insert("is_implied_included".into(),json!(false));
    append_owned_expression_result(&mut graph,"namespace","own-result.member","own-result").unwrap();
    for (id,name) in [("base","Base"),("collections","Collections"),("performances","Performances")] {
        let mut node=fresh_definition_element(id.into(),"LibraryPackage").unwrap();
        node.properties.insert("declared_name".into(),json!(name));node.properties.insert("is_standard".into(),json!(true));
        node.properties.insert("is_implied_included".into(),json!(true));graph.push(node);
    }
    for (id,kind,parent,name) in [("evaluations","Feature","performances","evaluations"),("anything","Classifier","base","Anything"),("things","Feature","base","things"),
        ("collection","DataType","collections","Collection"),("first-type","DataType","base","T"),("second-type","DataType","base","S")] {
        let mut node=fresh_definition_element(id.into(),kind).unwrap();node.properties.insert("declared_name".into(),json!(name));
        node.properties.insert("is_implied_included".into(),json!(true));graph.push(node);
        relative_fixture_member(&mut graph,parent,id,"OwningMembership");
    }
    for (id,target) in [("things","anything"),("result0.0",if collection {"collection"}else{"first-type"}),("result1.0","second-type")] {
        if graph.iter().any(|node|node.id==id) {
            let name=format!("{id}.explicit-typing");let mut relation=fresh_definition_element(name.clone(),"FeatureTyping").unwrap();
            relation.properties.insert("typed_feature".into(),json!(id));relation.properties.insert("type".into(),json!(target));
            graph.push(relation);relative_fixture_attach(&mut graph,id,&name,true);
        }
    }
    for id in ["first-type","second-type","collection"] {
        let name=format!("{id}.super");let mut relation=fresh_definition_element(name.clone(),"Subclassification").unwrap();
        relation.properties.insert("subclassifier".into(),json!(id));relation.properties.insert("superclassifier".into(),json!("anything"));
        graph.push(relation);relative_fixture_attach(&mut graph,id,&name,true);
    }
    assert!(ecore_model::validate_publication(&graph,ecore_model::ReferenceCompleteness::Closed).is_empty());
    graph
}

#[test]
fn definition_argument_result_rule_builds_index_select_types_and_replays_from_canonical_context() {
    for kind in ["IndexExpression","SelectExpression"] {
        let mut graph=argument_rule_fixture(kind,"normal",false);let original=graph.clone();
        let roots=vec!["namespace".to_owned()];
        assert_eq!(argument_result_specialization::materialize_batch(&mut graph,&roots).unwrap(),1);
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&graph,&index);
        let relation=index["namespace.implicit.argument-result-specialization"];
        assert_eq!(argument_result_specialization::assessed_relation(&query,index["own-result"],relation).unwrap(),Some(true));
        assert_eq!(query_reference_in_view(&query,relation,"general").unwrap().id,"result0.0");
        assert_eq!(definition_reference_targets_typed_query(&query,index["own-result"],"type").unwrap().iter().map(|node|node.id.as_str()).collect::<Vec<_>>(),["first-type"]);
        for node in &original {assert_eq!(graph.iter().find(|new|new.id==node.id).unwrap().properties.get("is_implied_included"),node.properties.get("is_implied_included"));}
        let mut restored:Vec<KirElement>=serde_json::from_str(&serde_json::to_string(&graph).unwrap()).unwrap();restored.reverse();
        let saved=serde_json::to_value(&restored).unwrap();
        assert_eq!(argument_result_specialization::materialize_batch(&mut restored,&roots).unwrap(),0);
        assert_eq!(saved,serde_json::to_value(&restored).unwrap());
        // Stable identity is not an admission token: a different identity with
        // the same canonical endpoints remains the same normative contribution.
        let new="unrelated-spelling";
        let mut value=serde_json::to_value(&graph).unwrap();
        fn rename(value:&mut Value,old:&str,new:&str) {match value {
            Value::String(text) if text==old=>*text=new.into(),Value::Array(rows)=>{for row in rows {rename(row,old,new);}},
            Value::Object(fields)=>{for field in fields.values_mut() {rename(field,old,new);}},_=>{}
        }}
        rename(&mut value,"namespace.implicit.argument-result-specialization",new);
        let mut renamed=serde_json::from_value(value).unwrap();
        assert_eq!(argument_result_specialization::materialize_batch(&mut renamed,&roots).unwrap(),0);
    }
}

#[test]
fn definition_argument_result_rule_obeys_collection_guard_actual_argument_order_and_empty_case() {
    let roots=vec!["namespace".to_owned()];
    let mut index=argument_rule_fixture("IndexExpression","normal",true);let saved=serde_json::to_value(&index).unwrap();
    assert_eq!(argument_result_specialization::materialize_batch(&mut index,&roots).unwrap(),0);
    assert_eq!(saved,serde_json::to_value(&index).unwrap());
    // Collection specialization can be indirect; checking a datatype name or
    // just the first explicit typing is insufficient.
    index.iter_mut().find(|node|node.id=="result0.0.explicit-typing").unwrap().properties.insert("type".into(),json!("first-type"));
    index.iter_mut().find(|node|node.id=="first-type.super").unwrap().properties.insert("superclassifier".into(),json!("collection"));
    assert_eq!(argument_result_specialization::materialize_batch(&mut index,&roots).unwrap(),0);
    let mut select=argument_rule_fixture("SelectExpression","normal",true);
    assert_eq!(argument_result_specialization::materialize_batch(&mut select,&roots).unwrap(),1);
    let mut reversed=argument_rule_fixture("IndexExpression","reverse_inputs",false);
    assert_eq!(argument_result_specialization::materialize_batch(&mut reversed,&roots).unwrap(),1);
    assert_eq!(reversed.iter().find(|node|node.id=="namespace.implicit.argument-result-specialization").unwrap().properties["subsetted_feature"],json!("result1.0"));
    for kind in ["IndexExpression","SelectExpression"] {
        let mut empty=argument_rule_fixture(kind,"empty",false);
        // No argument means no collection guard dependency and no edge.
        for node in &mut empty {if node.id=="collections" {node.properties.insert("declared_name".into(),json!("UnavailableCollections"));}}
        let saved=serde_json::to_value(&empty).unwrap();assert_eq!(argument_result_specialization::materialize_batch(&mut empty,&roots).unwrap(),0);
        assert_eq!(saved,serde_json::to_value(&empty).unwrap());
    }
    assert!(!argument_result_specialization::supports("CollectExpression"));
}

#[test]
fn definition_argument_result_rule_preserves_typed_missing_reads_and_constructs_results_through_scheduler() {
    let mut graph=argument_rule_fixture("IndexExpression","normal",false);let roots=vec!["namespace".to_owned()];
    graph.iter_mut().find(|node|node.id=="redefinition0.0").unwrap().properties.remove("redefined_feature");
    let saved=serde_json::to_value(&graph).unwrap();
    assert!(matches!(argument_result_specialization::materialize_batch(&mut graph,&roots),Err(QueryFailure::Required(Prerequisite::ReadField{owner_id,field})) if owner_id=="redefinition0.0"&&field=="redefined_feature"));
    assert_eq!(saved,serde_json::to_value(&graph).unwrap());
    let mut graph=argument_rule_fixture("SelectExpression","normal",false);
    graph.retain(|node|!matches!(node.id.as_str(),"own-result"|"own-result.member"));
    graph.iter_mut().find(|node|node.id=="namespace").unwrap().properties.get_mut("owned_relationship").unwrap().as_array_mut().unwrap().retain(|id|id!=&json!("own-result.member"));
    let saved=serde_json::to_value(&graph).unwrap();
    assert!(matches!(argument_result_specialization::materialize_batch(&mut graph,&roots),Err(QueryFailure::Required(Prerequisite::AdditionalMembers{owner_id})) if owner_id=="namespace"));
    assert_eq!(saved,serde_json::to_value(&graph).unwrap());
    let jobs=[Prerequisite::ArgumentResultSpecializationBatch{owner_ids:roots}];
    assert_eq!(serde_json::from_str::<Prerequisite>(&serde_json::to_string(&jobs[0]).unwrap()).unwrap(),jobs[0]);
    let (graph,pending)=link_generated_graph_planned(graph,Vec::new(),&|_|Err(error("no lexical reads expected")),true,&[],&|_|{},Some(&jobs)).unwrap();
    assert!(pending.is_empty());
    assert_eq!(graph.iter().find(|node|node.id=="namespace.implicit.argument-result-specialization").unwrap().properties["subsetting_feature"],json!("namespace.implicit.additional-members.result"));
    assert!(!scope_boolean(graph.iter().find(|node|node.id=="namespace").unwrap(),"is_implied_included").unwrap());
}

#[test]
fn definition_argument_result_rule_rejects_forged_endpoint_and_completion_claims_atomically() {
    let roots=vec!["namespace".to_owned()];
    let mut graph=argument_rule_fixture("IndexExpression","normal",false);
    argument_result_specialization::materialize_batch(&mut graph,&roots).unwrap();
    graph.iter_mut().find(|node|node.id=="namespace.implicit.argument-result-specialization").unwrap().properties.insert("subsetted_feature".into(),json!("result1.0"));
    let saved=serde_json::to_value(&graph).unwrap();
    assert!(matches!(argument_result_specialization::materialize_batch(&mut graph,&roots),Err(QueryFailure::Rejected(_))));
    assert_eq!(saved,serde_json::to_value(&graph).unwrap());
    let mut graph=argument_rule_fixture("SelectExpression","normal",false);
    graph.iter_mut().find(|node|node.id=="namespace").unwrap().properties.insert("is_implied_included".into(),json!(true));
    let saved=serde_json::to_value(&graph).unwrap();
    assert!(matches!(argument_result_specialization::materialize_batch(&mut graph,&roots),Err(QueryFailure::Rejected(_))));
    assert_eq!(saved,serde_json::to_value(&graph).unwrap());
}

#[test]
fn definition_argument_result_rule_discards_a_late_collision_for_the_complete_receiver_batch() {
    let mut graph=argument_rule_fixture("SelectExpression","normal",false);
    let mut owned=BTreeSet::from(["namespace".to_owned()]);
    loop {let mut changed=false;for node in &graph {if node.properties.get("owning_relationship").and_then(Value::as_str).is_some_and(|id|owned.contains(id))
        || node.properties.get("owning_related_element").and_then(Value::as_str).is_some_and(|id|owned.contains(id)) {changed|=owned.insert(node.id.clone());}}
        if !changed {break;}}
    for node in graph.clone().into_iter().filter(|node|owned.contains(&node.id)) {
        let mut value=serde_json::to_value(node).unwrap();
        fn replace(value:&mut Value,owned:&BTreeSet<String>) {match value {
            Value::String(text) if owned.contains(text)=>*text=format!("second.{text}"),
            Value::Array(rows)=>{for row in rows {replace(row,owned);}},Value::Object(fields)=>{for field in fields.values_mut() {replace(field,owned);}},_=>{}
        }}
        replace(&mut value,&owned);graph.push(serde_json::from_value(value).unwrap());
    }
    graph.push(fresh_definition_element("second.namespace.implicit.argument-result-specialization".into(),"Feature").unwrap());
    let saved=serde_json::to_value(&graph).unwrap();
    let failure=argument_result_specialization::materialize_batch(&mut graph,&["namespace".into(),"second.namespace".into()]).unwrap_err();
    assert!(matches!(failure,QueryFailure::Rejected(ref diagnostic) if diagnostic.message.contains("fresh")||diagnostic.message.contains("collision")),"{failure:?}");
    assert_eq!(saved,serde_json::to_value(&graph).unwrap());
}

#[test]
fn definition_argument_result_rule_rejects_recursive_guard_dependencies_and_missing_collection_inputs() {
    let roots=vec!["namespace".to_owned()];
    let mut graph=argument_rule_fixture("IndexExpression","normal",false);
    definition_append_implied_specialization(&mut graph,"argument-back-edge","Subsetting","result0.0","own-result").unwrap();
    definition_append_implied_specialization(&mut graph,"return-back-edge","Subsetting","own-result","result0.0").unwrap();
    let saved=serde_json::to_value(&graph).unwrap();
    let failure=argument_result_specialization::materialize_batch(&mut graph,&roots).unwrap_err();
    assert!(matches!(failure,QueryFailure::Rejected(ref diagnostic) if diagnostic.message.contains("cyclic argument-result")),"{failure:?}");
    assert_eq!(saved,serde_json::to_value(&graph).unwrap());
    let mut graph=argument_rule_fixture("IndexExpression","normal",false);
    graph.iter_mut().find(|node|node.id=="collections").unwrap().properties.insert("declared_name".into(),json!("MissingCollections"));
    let saved=serde_json::to_value(&graph).unwrap();
    let failure=argument_result_specialization::materialize_batch(&mut graph,&roots).unwrap_err();
    assert!(matches!(failure,QueryFailure::Rejected(ref diagnostic) if diagnostic.message.contains("Collections")),"{failure:?}");
    assert_eq!(saved,serde_json::to_value(&graph).unwrap());
}
