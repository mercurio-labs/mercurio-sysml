//! Controls for immutable syntax-tree sharing; no model occurrence is interned.
use super::*;
use serde_json::json;

#[test]
fn shared_completed_subtrees_keep_root_writes_and_detached_edits_private() {
    let child = Arc::new(Match {
        object_kind: Some("LiteralInteger".into()),
        fields: BTreeMap::from([("value".into(), json!(2))]),
        ..Match::default()
    });
    let member = Arc::new(Match {
        object_kind: Some("OwningMembership".into()),
        children: BTreeMap::from([("owned_related_element".into(), vec![child])]),
        construction_order: Box::new(vec![ConstructionStep::Child {field:"owned_related_element".into(),index:0}]),
        ..Match::default()
    });
    let original = Match {
        object_kind: Some("MultiplicityRange".into()),
        children: BTreeMap::from([("owned_relationship".into(), vec![member])]),
        construction_order: Box::new(vec![ConstructionStep::Child {field:"owned_relationship".into(),index:0}]),
        ..Match::default()
    };
    let mut branch = original.clone();
    assert!(Arc::ptr_eq(&original.children["owned_relationship"][0], &branch.children["owned_relationship"][0]));
    assert!(Arc::ptr_eq(&original.children["owned_relationship"][0].children["owned_related_element"][0],
        &branch.children["owned_relationship"][0].children["owned_related_element"][0]));
    branch.fields.insert("is_ordered".into(), json!(true));
    branch.construction_order.clear();
    assert!(original.fields.is_empty());
    assert_eq!(original.construction_order.len(), 1);
    let member = Arc::make_mut(&mut branch.children.get_mut("owned_relationship").unwrap()[0]);
    let literal = Arc::make_mut(&mut member.children.get_mut("owned_related_element").unwrap()[0]);
    literal.fields.insert("value".into(),json!(7));
    assert_eq!(original.children["owned_relationship"][0].children["owned_related_element"][0].fields["value"],2);
    assert_eq!(branch.children["owned_relationship"][0].children["owned_related_element"][0].fields["value"],7);
    assert!(!Arc::ptr_eq(&original.children["owned_relationship"][0], &branch.children["owned_relationship"][0]));
}

#[test]
fn definition_shared_occurrences_keep_canonical_identity_and_persistence() {
    let source = "package P { class T; feature a : T[2..2]; }";
    let tree=candidate_document(source,true).unwrap();
    fn literal_occurrences<'a>(tree:&'a Match,out:&mut Vec<&'a Arc<Match>>) {
        for child in tree.children.values().flatten().chain(tree.captured_operands.iter()) {
            if child.object_kind.as_deref()==Some("LiteralInteger") {out.push(child);}
            literal_occurrences(child,out);
        }
    }
    let mut occurrences=Vec::new();literal_occurrences(&tree,&mut occurrences);
    assert_eq!(occurrences.len(),2);
    assert!(!Arc::ptr_eq(occurrences[0],occurrences[1]),"equal spelling is not object identity");
    let snapshot=tree.clone();
    assert!(Arc::ptr_eq(&tree.children["owned_relationship"][0],&snapshot.children["owned_relationship"][0]));
    let document=crate::definition_document::parse_and_link(source,crate::SourceLanguage::Kerml).unwrap();
    let literals:Vec<_>=document.elements.iter().filter(|e|e.kind.ends_with("::LiteralInteger")).collect();
    assert_eq!(literals.len(),2);
    assert_ne!(literals[0].id,literals[1].id);
    assert_ne!(literals[0].properties["owning_relationship"],literals[1].properties["owning_relationship"]);
    assert!(literals.iter().all(|e|e.properties["value"]==2));
    let encoded=serde_json::to_string(&document).unwrap();
    let fresh:crate::KirDocument=serde_json::from_str(&encoded).unwrap();
    assert_eq!(serde_json::to_value(fresh).unwrap(),serde_json::to_value(document).unwrap());
    assert_eq!(tree,snapshot,"construction and publication must not mutate shared syntax");
}

#[test]
fn failed_capture_unwinds_shared_snapshot_without_leaks_or_model_writes() {
    let program:Programs=serde_json::from_value(json!({"rules":{"capture":{
        "owner":"https://www.omg.org/spec/SysML/20250201#//Expression","construct":false,"scalar":false,
        "body":{"id":"capture/body","kind":"sequence","cardinality":"","elements":[
            {"id":"capture/action","kind":"capture","cardinality":"",
             "classifier":"https://www.omg.org/spec/SysML/20250201#//OperatorExpression",
             "feature_id":ecore_model::feature("OperatorExpression","operand").unwrap().id,
             "captured_types":["https://www.omg.org/spec/SysML/20250201#//Expression"]},
            {"id":"capture/x","kind":"keyword","cardinality":"","value":"x"},
            {"id":"capture/y","kind":"keyword","cardinality":"","value":"y"}]}}}})).unwrap();
    let tokens=crate::xtext_terminal::lex("x z").unwrap();
    let stack=RefCell::new(Vec::new());let cache=RefCell::new(BTreeMap::new());
    let executor=Executor{programs:&program,tokens:&tokens,kind:"Expression",kerml:true,
        speculative:false,call_stack:&stack,prediction_cache:&cache};
    let operand=Arc::new(Match{object_kind:Some("LiteralInteger".into()),
        fields:BTreeMap::from([("value".into(),json!(2))]),..Match::default()});
    let original=Match{object_kind:Some("OperatorExpression".into()),
        captured_operands:vec![operand],construction_order:Box::new(vec![ConstructionStep::Operand(0)]),
        ..Match::default()};
    let before=original.clone();
    let references=Arc::strong_count(&original.captured_operands[0]);
    assert_eq!(executor.rule("capture",&original).unwrap_err(),"Incomplete Xtext group");
    assert_eq!(original,before);
    assert_eq!(Arc::strong_count(&original.captured_operands[0]),references);
    assert!(stack.borrow().is_empty());
    assert!(cache.borrow().is_empty());
}
