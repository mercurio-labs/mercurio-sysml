//! Handwritten canonical inputs/admission for generated Action/Transition predicates.
//! Default contributions are separate from complete Usage transformation/validation.
use super::*;
#[path="owner_typing_generated.rs"]
mod owner_typing_policy;
// Handwritten narrow projection proof over guarded, resolved producer bodies:
// direct receiver FeatureTyping is added by variation or owned-cross producers.
// Ordinary membership under a non-Feature namespace (or detached receiver) excludes both.
// Other contributions, inherited type closure and transformation are not assessed.
fn fresh_owner_typing_assessed(graph:&[KirElement],owner:&KirElement)->Result<bool,QueryFailure> {
 if !owner_typing_policy::supports(owner.kind.rsplit("::").next().unwrap_or(&owner.kind)) {return Ok(false);}
 if scope_boolean(owner,"is_end")? || owner.properties.get("direction").is_some_and(|v|!v.is_null()) {return Ok(false);}
 if metaclass_conforms(&owner.kind,"OccurrenceUsage") && !ecore_defaults::read_attribute(owner,"portion_kind").map_err(query_failure)?.is_null() {return Ok(false);}
 definition_projection::require_empty_metadata_bases(graph,owner)?;
 let index=graph.iter().map(|e|(e.id.as_str(),e)).collect::<BTreeMap<_,_>>();
 if index.len()!=graph.len() {return Err(query_failure("duplicate fresh owner typing identity"));}
 if let Some(member)=scope_container(&index,owner)? {
  if !matches!(member.kind.rsplit("::").next(),Some("FeatureMembership"|"OwningMembership")) {return Ok(false);}
  let parent=scope_container(&index,member)?.ok_or_else(||query_failure("fresh owner membership requires a canonical namespace"))?;
  // Independently assessed ancestor contexts; do not expand effect closure here.
  if !matches!(parent.kind.rsplit("::").next(),Some("Namespace"|"Package"|"Class"|"ActionDefinition"|"StateDefinition")) {return Ok(false);}
 }
 for r in stored_children(graph,owner,"owned_relationship")? {
  if scope_boolean(r,"is_implied")? {return Ok(false);}
  if !(metaclass_conforms(&r.kind,"FeatureTyping") || metaclass_conforms(&r.kind,"FeatureMembership") || ordinary_type_declaration(graph,r)? || definition_non_typing_featuring(graph,owner,r)? || definition_owned_non_typing_membership(graph,r)?) {return Ok(false);}
 }
 Ok(true)
}
#[path="action_contributions_generated.rs"]
mod policy;
#[path="state_contributions_generated.rs"]
mod state_policy;
pub(super) fn supports(kind:&str)->bool {let kind=kind.rsplit("::").next().unwrap_or(kind);policy::supports(kind) || kind=="StateUsage"}
// Handwritten selected Ecore invocation algorithm. The pinned operation dispatch,
// resolved delegate body and independent eInvoke controls are imported separately.
// This query reads canonical ownership/defaults; it does not transform its inputs.
fn state_is_substate_usage(graph:&[KirElement],receiver:&KirElement,parallel:bool)->Result<bool,QueryFailure> {
 if receiver.kind.rsplit("::").next()!=Some("StateUsage") {return Err(query_failure("unassessed State operation receiver"));}
 if graph.iter().map(|e|e.id.as_str()).collect::<BTreeSet<_>>().len()!=graph.len() {return Err(query_failure("duplicate State operation identity"));}
 let canonical=graph.iter().find(|e|e.id==receiver.id).ok_or_else(||query_failure("missing State operation receiver"))?;
 if canonical!=receiver {return Err(query_failure("foreign State operation receiver"));}
 if !scope_boolean(receiver,"is_composite")? {return Ok(false);}
 let Some((membership,owner))=definition_feature_owner(graph,receiver)? else {return Ok(false);};
 if !(metaclass_conforms(&owner.kind,"StateDefinition") || metaclass_conforms(&owner.kind,"StateUsage")) {return Ok(false);}
 Ok(scope_boolean(owner,"is_parallel")?==parallel && !metaclass_conforms(&membership.kind,"StateSubactionMembership"))
}
fn state_entry_exit(graph:&[KirElement],receiver:&KirElement)->Result<bool,QueryFailure> {
 let Some((membership,_))=definition_feature_owner(graph,receiver)? else {return Ok(false);};
 if !metaclass_conforms(&membership.kind,"StateSubactionMembership") {return Ok(false);}
 let kind=ecore_defaults::read_attribute(membership,"kind").map_err(query_failure)?;
 ecore_model::validate_value(&membership.kind,"kind",&kind).map_err(query_failure)?;
 Ok(kind=="entry" || kind=="exit")
}
fn typing(graph:&[KirElement],owner:&KirElement)->Result<(bool,bool,bool),QueryFailure> {
 let mut structure=false;let mut class=false;let mut data=false;
 for relation in stored_children(graph,owner,"owned_relationship")? {
  if !metaclass_conforms(&relation.kind,"FeatureTyping") {continue;}
  if scope_boolean(relation,"is_implied")? {return Err(query_failure("Action/Transition owner typing requires implicit FeatureTyping assessment"));}
  let target=query_reference(graph,relation,"type")?;
  structure |= metaclass_conforms(&target.kind,"Structure");class |= metaclass_conforms(&target.kind,"Class");data |= metaclass_conforms(&target.kind,"DataType");
 }
 Ok((structure,class,data))
}
// Imported containment/ownedTyping subsets distinguish a declared Type inside
// an exact OwningMembership from the receiver's direct FeatureTyping. Metadata
// declarations retain their separate semantic-base dependency.
fn ordinary_type_declaration(graph:&[KirElement],relation:&KirElement)->Result<bool,Diagnostic> {
 if relation.kind.rsplit("::").next()!=Some("OwningMembership") {return Ok(false);}
 let members=stored_children(graph,relation,"owned_related_element")?;
 if members.len()!=1 {return Err(error("Action declaration requires one canonical owned member"));}
 Ok(metaclass_conforms(&members[0].kind,"Type") && !metaclass_conforms(&members[0].kind,"MetadataFeature"))
}
pub(super) fn names_query(graph:&[KirElement],owner:&KirElement)->Result<Vec<&'static str>,QueryFailure> {
 if !supports(&owner.kind) {return Err(query_failure("unassessed Action/Transition contribution binding"));}
 if scope_boolean(owner,"is_end")? || owner.properties.get("direction").is_some_and(|v|!v.is_null()) {
  return Err(query_failure("Action/Transition contributions require non-end non-parameter context"));
 }
 definition_projection::require_empty_metadata_bases(graph,owner)?;
 let index=graph.iter().map(|e|(e.id.as_str(),e)).collect::<BTreeMap<_,_>>();
 if let Some(member)=scope_container(&index,owner)? {
  if metaclass_conforms(&member.kind,"VariantMembership") {return Err(query_failure("Action/Transition contribution variation typing remains required"));}
  if !matches!(member.kind.rsplit("::").next(),Some("FeatureMembership"|"OwningMembership")) && !(owner.kind.rsplit("::").next()==Some("StateUsage") && metaclass_conforms(&member.kind,"StateSubactionMembership")) {
   return Err(query_failure("Action/Transition contributions require ordinary canonical owning membership"));
  }
 }
 for r in stored_children(graph,owner,"owned_relationship")? {
  if metaclass_conforms(&r.kind,"Conjugation") || metaclass_conforms(&r.kind,"FeatureChaining") || metaclass_conforms(&r.kind,"CrossSubsetting") || metaclass_conforms(&r.kind,"FeatureValue") {
   return Err(query_failure("Action/Transition contribution requires valuation/chaining/crossing lifecycle assessment"));
  }
  if !(metaclass_conforms(&r.kind,"FeatureTyping") || metaclass_conforms(&r.kind,"Subsetting") || metaclass_conforms(&r.kind,"FeatureMembership") || r.kind.rsplit("::").next()==Some("Membership") || metaclass_conforms(&r.kind,"NamespaceImport") || definition_non_typing_featuring(graph,owner,r)? || definition_owned_non_typing_membership(graph,r)? || ordinary_type_declaration(graph,r)?) {
   return Err(query_failure(format!("unassessed Action/Transition contribution relationship {}",r.kind)));
  }
 }
 let (structure,_,data)=typing(graph,owner)?;
 let context=definition_feature_owner(graph,owner)?.map(|(_,type_)|type_);
 let mut owner_class=false;let mut owner_structure=false;
 if let Some(context)=context {
  if metaclass_conforms(&context.kind,"Feature") {
   if definition_variation_general(graph,context)?.is_some() {return Err(query_failure("Action/Transition owner variation typing remains required"));}
   if !scope_boolean(context,"is_implied_included")? && !fresh_owner_typing_assessed(graph,context)? {
     return Err(query_failure("Action/Transition requires assessed owner implicit typing"));
   }
   if stored_children(graph,context,"owned_relationship")?.iter().any(|r|metaclass_conforms(&r.kind,"Subsetting") || metaclass_conforms(&r.kind,"FeatureChaining") || metaclass_conforms(&r.kind,"Conjugation")) {
    return Err(query_failure("Action/Transition owner inherited/chained typing requires normative assessment"));
   }
   let (s,c,_)=typing(graph,context)?;owner_structure=s;owner_class=c;
  }
 }
 let portion=ecore_defaults::read_attribute(owner,"portion_kind").map_err(query_failure)?;
 ecore_model::validate_value(&owner.kind,"portion_kind",&portion).map_err(query_failure)?;
 let portion=portion.as_str().unwrap_or("none");
 let composite=scope_boolean(owner,"is_composite")?;
 let source_state=if owner.kind.rsplit("::").next()==Some("TransitionUsage") && composite
     && context.is_some_and(|c|metaclass_conforms(&c.kind,"ActionDefinition") || metaclass_conforms(&c.kind,"ActionUsage")) {
     transition_source_query::source_feature_for_defaults_query(graph,owner)?.is_some_and(|source|metaclass_conforms(&source.kind,"StateUsage"))
 } else {false};
 let inputs=policy::Inputs {source_state,entry_exit:false,composite:scope_boolean(owner,"is_composite")?,structure,data,owner_kind:context.map_or("",|e|e.kind.as_str()),owner_class,owner_structure,portion};
 let selected=if owner.kind.rsplit("::").next()==Some("StateUsage") {
  let state_inputs=state_policy::Inputs {entry_exit:state_entry_exit(graph,owner)?,composite:inputs.composite,structure:inputs.structure,data:inputs.data,owner_kind:inputs.owner_kind,owner_class:inputs.owner_class,owner_structure:inputs.owner_structure,portion:inputs.portion,exclusive_state:state_is_substate_usage(graph,owner,false)?,substate:state_is_substate_usage(graph,owner,true)?};
  state_policy::names(&state_inputs)
 } else {policy::names(owner.kind.rsplit("::").next().unwrap_or(&owner.kind),&inputs)}.ok_or_else(||query_failure("missing generated Action/State/Transition default map"))?;
 for relation in stored_children(graph,owner,"owned_relationship")? {
  if !scope_boolean(relation,"is_implied")? || definition_non_typing_featuring(graph,owner,relation)? {continue;}
  if relation.kind.rsplit("::").next()!=Some("Subsetting") || query_reference(graph,relation,"specific")?.id!=owner.id {return Err(query_failure("unassessed partial Action contribution"));}
  let target=query_reference(graph,relation,"general")?;
  let mut known=false;
  for name in &selected {if let GeneratedLinkTarget::Local(id)=standard_default_binding(graph,name)? {known|=target.id==id;}}
  if !known {return Err(query_failure("unassessed partial Action contribution target"));}
 }
 Ok(selected)
}
pub(super) fn names(graph:&[KirElement],owner:&KirElement)->Result<Vec<&'static str>,Diagnostic> {names_query(graph,owner).map_err(QueryFailure::into_diagnostic)}
pub(super) fn contributions(graph:&[KirElement],owner:&KirElement)->Result<Vec<(&'static str,String)>,Diagnostic> {
 let mut result=Vec::new();
 for name in names(graph,owner)? {
  let GeneratedLinkTarget::Local(id)=standard_default_binding(graph,name)? else {return Err(error("Action/Transition default requires resolved library resources"));};
  let target=graph.iter().find(|e|e.id==id).ok_or_else(||error("missing Action/Transition default target"))?;
  ecore_model::validate_reference_endpoint("Subsetting","general",&target.kind).map_err(error)?;
  if id!=owner.id {result.push(("Subsetting",id));}
 }
 Ok(result)
}


#[cfg(test)]
mod tests {
 use super::*;
 fn own(graph:&mut Vec<KirElement>,owner:&str,mut child:KirElement,kind:&str) {
  let mut member=fresh_definition_element(format!("{}.member",child.id),kind).unwrap();
  ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut member,&mut child).unwrap();ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id==owner).unwrap(),&mut member).unwrap();graph.extend([member,child]);
 }
 fn typed(graph:&mut Vec<KirElement>,owner:&str,name:&str,kind:&str) {
  let mut target=fresh_definition_element(name.into(),kind).unwrap();target.properties.insert("is_implied_included".into(),json!(true));graph.push(target);
  let mut r=fresh_definition_element(format!("{name}.typing"),"FeatureTyping").unwrap();r.properties.insert("typed_feature".into(),json!(owner));r.properties.insert("type".into(),json!(name));ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id==owner).unwrap(),&mut r).unwrap();graph.push(r);
 }
 fn fixture(c:&Value)->Vec<KirElement> {
  let mut target=fresh_definition_element("target".into(),c["kind"].as_str().unwrap()).unwrap();target.properties.insert("is_composite".into(),c["composite"].clone());target.properties.insert("portion_kind".into(),if c["portion"]=="none" {Value::Null}else{c["portion"].clone()});
  let mut graph=Vec::new();let context=c["context"].as_str().unwrap();
  if context=="detached" {graph.push(target);}else {
   let mut owner=fresh_definition_element("owner".into(),if context.starts_with("state_"){"StateUsage"}else if context.starts_with("Feature_"){"Feature"}else{context}).unwrap();if metaclass_conforms(&owner.kind,"Type") {owner.properties.insert("is_implied_included".into(),json!(true));}if metaclass_conforms(&owner.kind,"StateDefinition") || metaclass_conforms(&owner.kind,"StateUsage") {owner.properties.insert("is_parallel".into(),c.get("owner_parallel").cloned().unwrap_or(json!(false)));}graph.push(owner);
   if context.starts_with("Feature_") {typed(&mut graph,"owner","owner.type",if context=="Feature_class"{"Class"}else{"Structure"});}
   own(&mut graph,"owner",target,if context.starts_with("state_"){"StateSubactionMembership"}else{c["membership"].as_str().unwrap_or("FeatureMembership")});
   if context.starts_with("state_"){graph.iter_mut().find(|e|e.id=="target.member").unwrap().properties.insert("kind".into(),json!(&context[6..]));}
  }
  let mask=c["typing_mask"].as_u64().unwrap();for (bit,kind) in ["DataType","Class","Structure"].iter().enumerate() {if mask&(1<<bit)!=0 {typed(&mut graph,"target",&format!("type.{bit}"),kind);}}
  if c["source_kind"]!="none" {
   let mut source=fresh_definition_element("source".into(),c["source_kind"].as_str().unwrap()).unwrap();source.properties.insert("is_implied_included".into(),json!(true));graph.push(source);
   let mut alias=fresh_definition_element("source.alias".into(),"Membership").unwrap();alias.properties.insert("member_element".into(),json!("source"));
   ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id=="target").unwrap(),&mut alias).unwrap();graph.push(alias);
   let target=graph.iter_mut().find(|e|e.id=="target").unwrap();let mut relations=target.properties["owned_relationship"].as_array().unwrap().clone();relations.retain(|v|v!="source.alias");relations.insert(0,json!("source.alias"));target.properties.insert("owned_relationship".into(),json!(relations));
  }
  graph
 }

 fn data()->Value {serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/conformance/2026-08-support/action-contribution-pilot-controls.json"))).unwrap()}
 fn source_pending(c:&Value)->bool {c["kind"]=="TransitionUsage" && c["composite"]==true && c["source_kind"]=="none" && ["ActionDefinition","ActionUsage","StateDefinition","StateUsage"].iter().any(|k|c["context"]==*k)}
 fn library()->Vec<KirElement> {library_for(&data())}
 fn library_for(data:&Value)->Vec<KirElement> {let mut tree=BTreeMap::<String,BTreeMap<String,BTreeSet<String>>>::new();
  for binding in data["bindings"].as_object().unwrap().values() {for name in binding["names"].as_object().unwrap().values().filter_map(Value::as_str) {
   let parts=name.split("::").collect::<Vec<_>>();let package=tree.entry(parts[0].into()).or_default();
   if parts.len()==2 {package.entry(String::new()).or_default().insert(parts[1].into());}else{package.entry(parts[1].into()).or_default().insert(parts[2].into());}
  }}
  let mut source=String::new();for (package,classes) in tree {source+=&format!("standard library package {package} {{ ");for (class,features) in classes {if !class.is_empty(){source+=&format!("class {class} {{ ");}for name in features {source+=&format!("feature {name}; ");}if !class.is_empty(){source+="} ";}}source+="} ";}
  let mut graph=crate::definition_document::parse_and_link(&source,crate::SourceLanguage::Kerml).unwrap().elements;
  // Same supplied complete fixture-library stage as Pilot, not real resource closure.
  for e in &mut graph {if metaclass_conforms(&e.kind,"Type") {e.properties.insert("is_implied_included".into(),json!(true));}}graph
 }
 fn state_data()->Value {serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/conformance/2026-08-support/state-contribution-pilot-controls.json"))).unwrap()}
 #[test]
 fn definition_state_ecore_invocation_matches_selected_pilot_delegate() {
  let data=state_data();let cases=data["operation_controls"].as_array().unwrap();assert_eq!(cases.len(),320);
  for c in cases {
   let context=if c["membership"]!="ordinary" && c["context"]=="StateUsage" {format!("state_{}",c["membership"].as_str().unwrap())}else{c["context"].as_str().unwrap().to_string()};
   let mut graph=fixture(&json!({"kind":"StateUsage","context":context,"composite":c["composite"],"typing_mask":0,"portion":"none","source_kind":"none","owner_parallel":c["owner_parallel"]}));
   if c["membership"]!="ordinary" && graph.iter().any(|e|e.id=="target.member") {let m=graph.iter_mut().find(|e|e.id=="target.member").unwrap();m.kind="SysML::StateSubactionMembership".into();m.properties.insert("kind".into(),c["membership"].clone());}
   graph.iter_mut().find(|e|e.id=="target").unwrap().properties.insert("is_parallel".into(),c["receiver_parallel"].clone());
   let restored:Vec<KirElement>=serde_json::from_value(serde_json::to_value(&graph).unwrap()).unwrap();
   for mut g in [graph,restored] {for _ in 0..2 {let target=g.iter().find(|e|e.id=="target").unwrap();assert_eq!(state_is_substate_usage(&g,target,c["argument"].as_bool().unwrap()).unwrap(),c["result"].as_bool().unwrap(),"{c}");g.reverse();}}
  }
 }
 #[test]
 fn definition_state_contributions_match_pilot_owner_and_subaction_contexts() {
  let data=state_data();let cases=data["controls"].as_array().unwrap();assert_eq!(cases.len(),1632);
  for c in cases {
   let graph=fixture(c);let restored:Vec<KirElement>=serde_json::from_value(serde_json::to_value(&graph).unwrap()).unwrap();
   for mut g in [graph,restored] {for _ in 0..2 {let target=g.iter().find(|e|e.id=="target").unwrap();let result=names_query(&g,target).unwrap();assert_eq!(json!(result.iter().map(|n|json!({"kind":"Subsetting","target":n})).collect::<Vec<_>>()),c["generals"],"{c}");assert!(!scope_boolean(target,"is_implied_included").unwrap());g.reverse();}}
  }
 }
 #[test]
 fn definition_state_defaults_materialize_and_round_trip_without_completion_flags() {
  let data=state_data();let library=library_for(&data);let mut count=0;
  for c in data["controls"].as_array().unwrap().iter().filter(|c|c["typing_mask"]==0 && c["portion"]=="none") {
   let mut graph=fixture(c);graph.extend(library.clone());let expected=contributions(&graph,graph.iter().find(|e|e.id=="target").unwrap()).unwrap();
   assert_eq!(definition_materialize_feature_defaults(&mut graph,&[("target","state-default")]).unwrap(),expected.len(),"{c}");
   assert_eq!(definition_materialize_feature_defaults(&mut graph,&[("target","state-replay")]).unwrap(),0,"{c}");
   assert!(!scope_boolean(graph.iter().find(|e|e.id=="target").unwrap(),"is_implied_included").unwrap());
   let doc=crate::KirDocument{metadata:BTreeMap::new(),elements:graph};let exported=crate::abstract_syntax_json::export_sysml_abstract_syntax_value(&doc,Default::default()).unwrap();assert!(!exported.has_errors(),"{c}: {:?}",exported.diagnostics);
   let imported=crate::abstract_syntax_json::import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!imported.has_errors(),"{c}: {:?}",imported.diagnostics);
   let mut restored=imported.persistable_document().unwrap();restored.elements.reverse();assert_eq!(definition_materialize_feature_defaults(&mut restored.elements,&[("target","state-restored")]).unwrap(),0,"{c}");
   assert_eq!(contributions(&restored.elements,restored.elements.iter().find(|e|e.id=="target").unwrap()).unwrap(),expected,"{c}");count+=1;
  }
  assert_eq!(count,68);
 }
 #[test]
 fn definition_state_operation_rejects_malformed_inputs_without_mutation() {
  let base=json!({"kind":"StateUsage","context":"StateUsage","composite":true,"typing_mask":0,"portion":"none","source_kind":"none","owner_parallel":false});
  for (id,field,value) in [("owner","is_parallel",json!("false")),("target","is_composite",Value::Null),("target.member","kind",json!("unknown"))] {
   let mut graph=fixture(&base);if field=="kind" {graph.iter_mut().find(|e|e.id==id).unwrap().kind="SysML::StateSubactionMembership".into();}
   graph.iter_mut().find(|e|e.id==id).unwrap().properties.insert(field.into(),value);let before=serde_json::to_value(&graph).unwrap();let receiver=graph.iter().find(|e|e.id=="target").unwrap();assert!(names_query(&graph,receiver).is_err());assert_eq!(before,serde_json::to_value(&graph).unwrap());
  }
  let graph=fixture(&base);let mut foreign=graph.iter().find(|e|e.id=="target").unwrap().clone();foreign.properties.insert("is_parallel".into(),json!(true));assert!(state_is_substate_usage(&graph,&foreign,false).is_err());
 }
 #[test]
 fn definition_fresh_owner_typing_matches_resolved_pilot_producers() {
  let data:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/conformance/2026-08-support/owner-typing-pilot-controls.json"))).unwrap();
  let controls=data["controls"].as_array().unwrap();assert_eq!(controls.len(),768);
  for case in controls {
   let mut graph=fixture(&json!({"kind":case["kind"],"context":case["context"],"membership":case["membership"],"composite":case["composite"],"typing_mask":case["typing_mask"],"portion":"none","source_kind":"none"}));
   let before=serde_json::to_value(&graph).unwrap();let target=graph.iter().find(|e|e.id=="target").unwrap();
   assert!(fresh_owner_typing_assessed(&graph,target).unwrap(),"{case}");assert_eq!(scope_boolean(target,"is_implied_included").unwrap(),case["is_implied_included"]);
   let direct=stored_children(&graph,target,"owned_relationship").unwrap().into_iter().filter(|r|metaclass_conforms(&r.kind,"FeatureTyping")).map(|r|query_reference(&graph,r,"type").unwrap().id.clone()).collect::<Vec<_>>();
   assert_eq!(json!(direct),case["explicit"]);assert_eq!(case["implicit_after"],json!([]));assert_eq!(serde_json::to_value(&graph).unwrap(),before);
   // Downstream default contributions can use a fresh receiver as their owner.
   own(&mut graph,"target",fresh_definition_element("nested".into(),"ActionUsage").unwrap(),"FeatureMembership");
   let nested=graph.iter().find(|e|e.id=="nested").unwrap();assert!(names(&graph,nested).is_ok(),"{case}: {:?}",names(&graph,nested));
   let restored:Vec<KirElement>=serde_json::from_value(serde_json::to_value(&graph).unwrap()).unwrap();assert!(names(&restored,restored.iter().find(|e|e.id=="nested").unwrap()).is_ok());
  }
 }
 #[test]
 fn definition_fresh_owner_typing_drives_physical_defaults_and_persistence() {
  let data:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/conformance/2026-08-support/owner-typing-pilot-controls.json"))).unwrap();
  let libraries=library();let mut count=0;
  for case in data["controls"].as_array().unwrap().iter().filter(|c|c["typing_mask"]==0 && c["composite"]==false) {
   let mut graph=fixture(&json!({"kind":case["kind"],"context":case["context"],"membership":case["membership"],"composite":false,"typing_mask":0,"portion":"none","source_kind":"none"}));
   let owner=graph.iter_mut().find(|e|e.id=="target").unwrap();if !metaclass_conforms(&owner.kind,"OccurrenceUsage") {owner.properties.remove("portion_kind");}
   own(&mut graph,"target",fresh_definition_element("nested".into(),"ActionUsage").unwrap(),"FeatureMembership");graph.extend(libraries.clone());
   let target=graph.iter().find(|e|e.id=="nested").unwrap();let expected=contributions(&graph,target).unwrap();assert!(!expected.is_empty());
   assert_eq!(definition_materialize_feature_defaults(&mut graph,&[("nested","fresh-owner")]).unwrap(),expected.len(),"{case}");
   assert!(!scope_boolean(graph.iter().find(|e|e.id=="target").unwrap(),"is_implied_included").unwrap());
   let doc=crate::KirDocument {metadata:BTreeMap::new(),elements:graph};let exported=crate::abstract_syntax_json::export_sysml_abstract_syntax_value(&doc,Default::default()).unwrap();assert!(!exported.has_errors(),"{case}: {:?}",exported.diagnostics);
   let imported=crate::abstract_syntax_json::import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!imported.has_errors(),"{case}: {:?}",imported.diagnostics);
   let mut replay=imported.persistable_document().unwrap();replay.elements.reverse();assert_eq!(definition_materialize_feature_defaults(&mut replay.elements,&[("nested","repeat")]).unwrap(),0);assert!(!scope_boolean(replay.elements.iter().find(|e|e.id=="target").unwrap(),"is_implied_included").unwrap());count+=1;
  }
  assert_eq!(count,48);
 }
 #[test]
 fn definition_fresh_owner_typing_preserves_unassessed_producers() {
  let base=json!({"kind":"StateUsage","context":"Package","composite":false,"typing_mask":0,"portion":"none","source_kind":"none"});
  for failure in ["end","portion","variant","chaining","implied"] {
   let mut graph=fixture(&base);
   match failure {
    "end"=>{graph.iter_mut().find(|e|e.id=="target").unwrap().properties.insert("is_end".into(),json!(true));},
    "composite"=>{graph.iter_mut().find(|e|e.id=="target").unwrap().properties.insert("is_composite".into(),json!(true));},
    "portion"=>{graph.iter_mut().find(|e|e.id=="target").unwrap().properties.insert("portion_kind".into(),json!("snapshot"));},
    "variant"=>{graph.iter_mut().find(|e|e.id=="target.member").unwrap().kind="SysML::VariantMembership".into();},
    _=>{let mut r=fresh_definition_element("unassessed".into(),if failure=="chaining" {"FeatureChaining"}else{"FeatureTyping"}).unwrap();if failure=="implied" {r.properties.insert("is_implied".into(),json!(true));}ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id=="target").unwrap(),&mut r).unwrap();graph.push(r);},
   }
   let before=serde_json::to_value(&graph).unwrap();assert!(!fresh_owner_typing_assessed(&graph,graph.iter().find(|e|e.id=="target").unwrap()).unwrap(),"{failure}");assert_eq!(before,serde_json::to_value(&graph).unwrap());
  }
 }
 #[test]
 fn definition_transition_alias_prerequisites_resume_to_pilot_contributions() {
  let data=data();let libraries=library();let mut count=0;
  for c in data["controls"].as_array().unwrap().iter().filter(|c|c["kind"]=="TransitionUsage" && c["composite"]==true && c["source_kind"]!="none" && ["ActionDefinition","ActionUsage","StateDefinition","StateUsage"].iter().any(|k|c["context"]==*k)) {
   let mut graph=fixture(c);graph.extend(libraries.clone());let alias=graph.iter_mut().find(|e|e.id=="source.alias").unwrap();let value=alias.properties.remove("member_element").unwrap();
   let before=serde_json::to_value(&graph).unwrap();let target=graph.iter().find(|e|e.id=="target").unwrap();
   assert!(matches!(names_query(&graph,target),Err(QueryFailure::Required(Prerequisite::ReadField{owner_id,field})) if owner_id=="source.alias" && field=="member_element"),"{c}");
   assert!(matches!(definition_general_type_inputs(&graph,target).unwrap(),DefinitionGeneralTypes::Deferred(GeneratedLinkTarget::Deferred{owner_id,field}) if owner_id=="source.alias" && field=="member_element"),"{c}");
   assert_eq!(before,serde_json::to_value(&graph).unwrap());graph.iter_mut().find(|e|e.id=="source.alias").unwrap().properties.insert("member_element".into(),value);
   let target=graph.iter().find(|e|e.id=="target").unwrap();let resolved=names_query(&graph,target).unwrap();assert_eq!(json!(resolved.iter().map(|n|json!({"kind":"Subsetting","target":n})).collect::<Vec<_>>()),c["generals"],"{c}");
   let expected=contributions(&graph,target).unwrap();assert_eq!(definition_materialize_feature_defaults(&mut graph,&[("target","resumed")]).unwrap(),expected.len());assert_eq!(definition_materialize_feature_defaults(&mut graph,&[("target","replayed")]).unwrap(),0);count+=1;
  }
  assert_eq!(count,192);
 }
 #[test]
 fn definition_transition_defaults_use_stable_source_while_connector_producer_remains_pending() {
  let data=data();let libraries=library();let mut count=0;
  for c in data["controls"].as_array().unwrap().iter().filter(|c|c["kind"]=="TransitionUsage" && c["composite"]==true && c["source_kind"]!="none" && ["ActionDefinition","ActionUsage","StateDefinition","StateUsage"].iter().any(|k|c["context"]==*k)) {
   let mut graph=fixture(c);graph.extend(libraries.clone());let mut succession=fresh_definition_element("succession".into(),"SuccessionAsUsage").unwrap();succession.properties.insert("is_portion".into(),json!(true));own(&mut graph,"target",succession,"OwningMembership");
   let before=serde_json::to_value(&graph).unwrap();let target=graph.iter().find(|e|e.id=="target").unwrap();
   assert_eq!(transition_source_query::ready(&graph,target).unwrap(),Some(false));
   assert!(transition_source_query::source_feature_query(&graph,target).is_err());
   let names=names_query(&graph,target).unwrap();assert_eq!(json!(names.iter().map(|n|json!({"kind":"Subsetting","target":n})).collect::<Vec<_>>()),c["generals"],"{c}");
   assert_eq!(serde_json::to_value(&graph).unwrap(),before);
   let selected=contributions(&graph,target).unwrap();definition_materialize_feature_defaults(&mut graph,&[("target","source-stage")]).unwrap();
   assert!(!scope_boolean(graph.iter().find(|e|e.id=="target").unwrap(),"is_implied_included").unwrap());
   assert!(!graph.iter().any(|e|metaclass_conforms(&e.kind,"BindingConnector") || e.id.contains(".link")));
   assert_eq!(definition_materialize_feature_defaults(&mut graph,&[("target","replay")]).unwrap(),0);
   assert_eq!(selected.len(),c["generals"].as_array().unwrap().len());count+=1;
  }
  assert_eq!(count,192);
 }
 #[test]
 fn definition_action_contributions_match_resolved_pilot_chain() {
  let data=data();let controls=data["controls"].as_array().unwrap();assert_eq!(controls.len(),2688);let mut matched=0;let mut pending=0;
  for case in controls {
   let mut graph=fixture(case);let target=graph.iter().find(|e|e.id=="target").unwrap();
   if source_pending(case) {assert!(names(&graph,target).unwrap_err().message.contains("additional-member construction"),"{case}");pending+=1;continue;}
   let read=|graph:&[KirElement]|json!(names(graph,graph.iter().find(|e|e.id=="target").unwrap()).unwrap().iter().map(|n|json!({"kind":"Subsetting","target":n})).collect::<Vec<_>>());
   let before=serde_json::to_value(&graph).unwrap();assert_eq!(read(&graph),case["generals"],"{case}");assert_eq!(serde_json::to_value(&graph).unwrap(),before);
   graph.reverse();let restored:Vec<KirElement>=serde_json::from_value(serde_json::to_value(&graph).unwrap()).unwrap();assert_eq!(read(&restored),case["generals"],"{case}");matched+=1;
  }
  assert_eq!((matched,pending),(2592,96));
 }
 #[test]
 fn definition_action_contributions_distinguish_nested_type_declarations() {
  let data=data();let controls=data["owned_member_controls"].as_array().unwrap();assert_eq!(controls.len(),8);
  for case in controls {
   let mut graph=fixture(&json!({"kind":case["kind"],"context":"detached","composite":false,"typing_mask":0,"portion":"none","source_kind":"none"}));
   own(&mut graph,"target",fresh_definition_element("declared".into(),case["child_kind"].as_str().unwrap()).unwrap(),"OwningMembership");
   let observed=names(&graph,graph.iter().find(|e|e.id=="target").unwrap()).unwrap();assert_eq!(json!(observed.iter().map(|n|json!({"kind":"Subsetting","target":n})).collect::<Vec<_>>()),case["generals"],"{case}");
  }
 }
 #[test]
 fn definition_action_contribution_transactions_persist_and_replay() {
  let data=data();let library=library();let mut count=0;
  for case in data["controls"].as_array().unwrap().iter().filter(|c|c["composite"]==true && c["typing_mask"]==7 && c["portion"]=="snapshot" && (c["kind"]=="ActionUsage" || c["source_kind"]!="none")) {
   let mut graph=fixture(case);graph.extend(library.clone());let target=graph.iter().find(|e|e.id=="target").unwrap();let expected=contributions(&graph,target).unwrap();
   let DefinitionGeneralTypes::Ready(ids)=definition_general_type_inputs(&graph,target).unwrap() else {panic!("deferred {case}");};assert_eq!(ids,[vec!["type.0".into(),"type.1".into(),"type.2".into()],expected.iter().map(|(_,id)|id.clone()).collect::<Vec<_>>()].concat(),"{case}");
   assert_eq!(definition_materialize_feature_defaults(&mut graph,&[("target","generated")]).unwrap(),expected.len(),"{case}");
   let before=serde_json::to_value(&graph).unwrap();assert_eq!(definition_materialize_feature_defaults(&mut graph,&[("target","replay")]).unwrap(),0);assert_eq!(serde_json::to_value(&graph).unwrap(),before);
   let doc=crate::KirDocument {metadata:BTreeMap::new(),elements:graph};let exported=crate::abstract_syntax_json::export_sysml_abstract_syntax_value(&doc,Default::default()).unwrap();assert!(!exported.has_errors(),"{case}: {:?}",exported.diagnostics);
   let imported=crate::abstract_syntax_json::import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!imported.has_errors());let mut replay=imported.persistable_document().unwrap();replay.elements.reverse();assert_eq!(definition_materialize_feature_defaults(&mut replay.elements,&[("target","replayed")]).unwrap(),0);
   let read=names(&replay.elements,replay.elements.iter().find(|e|e.id=="target").unwrap()).unwrap();assert_eq!(json!(read.iter().map(|n|json!({"kind":"Subsetting","target":n})).collect::<Vec<_>>()),case["generals"],"{case}");count+=1;
  }
  assert_eq!(count,42);
 }
 #[test]
 fn definition_action_contributions_preserve_missing_semantic_dependencies() {
  let base=json!({"kind":"TransitionUsage","context":"StateUsage","composite":true,"typing_mask":0,"portion":"none","source_kind":"StateUsage"});
  for failure in ["owner_typing","end","parameter","state_entry","valuation","implied_typing","partial"] {
   let mut graph=fixture(&base);graph.extend(library());
   match failure {
    "owner_typing"=>{let owner=graph.iter_mut().find(|e|e.id=="owner").unwrap();owner.properties.insert("is_implied_included".into(),json!(false));owner.kind="SysML::PartUsage".into();},
    "end"=>{graph.iter_mut().find(|e|e.id=="target").unwrap().properties.insert("is_end".into(),json!(true));},
    "parameter"=>{graph.iter_mut().find(|e|e.id=="target.member").unwrap().kind="SysML::ParameterMembership".into();},
    "state_entry"=>{graph.iter_mut().find(|e|e.id=="target.member").unwrap().kind="SysML::StateSubactionMembership".into();},
    "valuation"=>{let mut r=fresh_definition_element("value".into(),"FeatureValue").unwrap();ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id=="target").unwrap(),&mut r).unwrap();graph.push(r);},
    "implied_typing"=>{let mut r=fresh_definition_element("typing".into(),"FeatureTyping").unwrap();r.properties.insert("typed_feature".into(),json!("target"));r.properties.insert("type".into(),json!("owner"));r.properties.insert("is_implied".into(),json!(true));ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id=="target").unwrap(),&mut r).unwrap();graph.push(r);},
    _=>{let mut r=fresh_definition_element("unknown".into(),"Subsetting").unwrap();r.properties.insert("subsetting_feature".into(),json!("target"));r.properties.insert("subsetted_feature".into(),json!("source"));r.properties.insert("is_implied".into(),json!(true));ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id=="target").unwrap(),&mut r).unwrap();graph.push(r);},
   }
   let before=serde_json::to_value(&graph).unwrap();assert!(names(&graph,graph.iter().find(|e|e.id=="target").unwrap()).is_err(),"{failure}");assert!(definition_materialize_feature_defaults(&mut graph,&[("target","failed")]).is_err(),"{failure}");assert_eq!(serde_json::to_value(&graph).unwrap(),before);
  }
 }
}
