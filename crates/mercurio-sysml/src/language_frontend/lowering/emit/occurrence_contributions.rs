//! Handwritten canonical inputs/admission for generated Occurrence predicates.
//! Default contributions are separate from complete Usage transformation/validation.
use super::*;
#[path="occurrence_contributions_generated.rs"]
mod policy;
pub(super) fn supports(kind:&str)->bool {policy::supports(kind.rsplit("::").next().unwrap_or(kind))}
fn typing(graph:&[KirElement],owner:&KirElement)->Result<(bool,bool,bool),QueryFailure> {
 let mut structure=false;let mut class=false;let mut data=false;
 for relation in stored_children(graph,owner,"owned_relationship")? {
  if !metaclass_conforms(&relation.kind,"FeatureTyping") {continue;}
  if scope_boolean(relation,"is_implied")? {return Err(query_failure("Occurrence owner typing requires implicit FeatureTyping assessment"));}
  let target=query_reference(graph,relation,"type")?;
  structure |= metaclass_conforms(&target.kind,"Structure");class |= metaclass_conforms(&target.kind,"Class");data |= metaclass_conforms(&target.kind,"DataType");
 }
 Ok((structure,class,data))
}
pub(super) fn names_query(graph:&[KirElement],owner:&KirElement)->Result<Vec<&'static str>,QueryFailure> {
 if !supports(&owner.kind) {return Err(query_failure("unassessed Occurrence contribution binding"));}
 if scope_boolean(owner,"is_end")? || owner.properties.get("direction").is_some_and(|v|!v.is_null()) {
  return Err(query_failure("Occurrence contributions require non-end non-parameter context"));
 }
 definition_projection::require_empty_metadata_bases(graph,owner)?;
 let index=graph.iter().map(|e|(e.id.as_str(),e)).collect::<BTreeMap<_,_>>();
 if let Some(member)=scope_container(&index,owner)? {
  if metaclass_conforms(&member.kind,"VariantMembership") {return Err(query_failure("Occurrence contribution variation typing remains required"));}
  if metaclass_conforms(&member.kind,"ParameterMembership") || (!metaclass_conforms(&member.kind,"FeatureMembership") && member.kind.rsplit("::").next()!=Some("OwningMembership")) {
   return Err(query_failure("Occurrence contributions require ordinary canonical owning membership"));
  }
 }
 for r in stored_children(graph,owner,"owned_relationship")? {
  if metaclass_conforms(&r.kind,"Conjugation") || metaclass_conforms(&r.kind,"FeatureChaining") || metaclass_conforms(&r.kind,"CrossSubsetting") || metaclass_conforms(&r.kind,"FeatureValue") {
   return Err(query_failure("Occurrence contribution requires valuation/chaining/crossing lifecycle assessment"));
  }
  if !(metaclass_conforms(&r.kind,"FeatureTyping") || metaclass_conforms(&r.kind,"Subsetting") || metaclass_conforms(&r.kind,"FeatureMembership") || r.kind.rsplit("::").next()==Some("Membership") || metaclass_conforms(&r.kind,"NamespaceImport") || definition_non_typing_featuring(graph,owner,r)? || definition_owned_non_typing_membership(graph,r)?) {
   return Err(query_failure(format!("unassessed Occurrence contribution relationship {}",r.kind)));
  }
 }
 let (structure,_,data)=typing(graph,owner)?;
 let context=definition_feature_owner(graph,owner)?.map(|(_,type_)|type_);
 let mut owner_class=false;let mut owner_structure=false;
 if let Some(context)=context {
  if metaclass_conforms(&context.kind,"Feature") {
   if definition_variation_general(graph,context)?.is_some() {return Err(query_failure("Occurrence owner variation typing remains required"));}
   if !scope_boolean(context,"is_implied_included")? && !(context.kind.rsplit("::").next()==Some("Feature") || supports(&context.kind)) {
    return Err(query_failure("Occurrence requires assessed owner implicit typing"));
   }
   if stored_children(graph,context,"owned_relationship")?.iter().any(|r|metaclass_conforms(&r.kind,"Subsetting") || metaclass_conforms(&r.kind,"FeatureChaining") || metaclass_conforms(&r.kind,"Conjugation")) {
    return Err(query_failure("Occurrence owner inherited/chained typing requires normative assessment"));
   }
   let (s,c,_)=typing(graph,context)?;owner_structure=s;owner_class=c;
  }
 }
 let portion=ecore_defaults::read_attribute(owner,"portion_kind").map_err(query_failure)?;
 ecore_model::validate_value(&owner.kind,"portion_kind",&portion).map_err(query_failure)?;
 let portion=portion.as_str().unwrap_or("none");
 let inputs=policy::Inputs {composite:scope_boolean(owner,"is_composite")?,structure,data,owner_kind:context.map_or("",|e|e.kind.as_str()),owner_class,owner_structure,portion};
 policy::names(owner.kind.rsplit("::").next().unwrap_or(&owner.kind),&inputs).ok_or_else(||query_failure("missing generated Occurrence default map"))
}
pub(super) fn names(graph:&[KirElement],owner:&KirElement)->Result<Vec<&'static str>,Diagnostic> {names_query(graph,owner).map_err(QueryFailure::into_diagnostic)}
pub(super) fn contributions(graph:&[KirElement],owner:&KirElement)->Result<Vec<(&'static str,String)>,Diagnostic> {
 let mut result=Vec::new();
 for name in names(graph,owner)? {
  let GeneratedLinkTarget::Local(id)=standard_default_binding(graph,name)? else {return Err(error("Occurrence default requires resolved library resources"));};
  let target=graph.iter().find(|e|e.id==id).ok_or_else(||error("missing Occurrence default target"))?;
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
   let mut owner=fresh_definition_element("owner".into(),if context.starts_with("Feature_"){"Feature"}else{context}).unwrap();if metaclass_conforms(&owner.kind,"Type") {owner.properties.insert("is_implied_included".into(),json!(true));}graph.push(owner);
   if context.starts_with("Feature_") {typed(&mut graph,"owner","owner.type",if context=="Feature_class"{"Class"}else{"Structure"});}
   own(&mut graph,"owner",target,"FeatureMembership");
  }
  let mask=c["typing_mask"].as_u64().unwrap();for (bit,kind) in ["DataType","Class","Structure"].iter().enumerate() {if mask&(1<<bit)!=0 {typed(&mut graph,"target",&format!("type.{bit}"),kind);}}
  graph
 }
 fn library()->Vec<KirElement> {
  // Explicit factory dependency environment only; this is not actual stdlib closure.
  let mut graph=crate::definition_document::parse_and_link("standard library package Base { feature things; feature dataValues; } standard library package Items { feature items; class Item { feature subitems; feature subparts; } } standard library package Occurrences { feature occurrences; class Occurrence { feature suboccurrences; feature snapshots; feature timeSlices; } } standard library package Objects { feature objects; class Object { feature subobjects; } } standard library package Views { feature views; class View { feature subviews; } } standard library package Metadata { feature metadataItems; }",crate::SourceLanguage::Kerml).unwrap().elements;
  for e in &mut graph {if metaclass_conforms(&e.kind,"Type") {e.properties.insert("is_implied_included".into(),json!(true));}}
  graph
 }
 #[test]
 fn definition_occurrence_contributions_match_all_pilot_controls() {
  let data:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/conformance/2026-08-support/occurrence-contribution-pilot-controls.json"))).unwrap();assert_eq!(data["controls"].as_array().unwrap().len(),2304);
  for c in data["controls"].as_array().unwrap() {
   let mut graph=fixture(c);let owner=graph.iter().find(|e|e.id=="target").unwrap();let observed=names(&graph,owner).unwrap().into_iter().map(|n|json!({"kind":"Subsetting","target":n})).collect::<Vec<_>>();assert_eq!(json!(observed),c["generals"],"{c}");
   graph.reverse();let graph:Vec<KirElement>=serde_json::from_value(serde_json::to_value(graph).unwrap()).unwrap();assert_eq!(names(&graph,graph.iter().find(|e|e.id=="target").unwrap()).unwrap().len(),observed.len());
  }
 }
 #[test]
 fn definition_occurrence_contribution_queries_and_transactions_replay() {
  let data:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/conformance/2026-08-support/occurrence-contribution-pilot-controls.json"))).unwrap();let library=library();
  for c in data["controls"].as_array().unwrap().iter().filter(|c|c["composite"]==true && c["typing_mask"]==7 && c["portion"]=="snapshot") {
   let mut graph=fixture(c);graph.extend(library.clone());let owner=graph.iter().find(|e|e.id=="target").unwrap();let selected=contributions(&graph,owner).unwrap();assert_eq!(selected.len(),c["generals"].as_array().unwrap().len(),"{c}");
   let DefinitionGeneralTypes::Ready(ids)=definition_general_type_inputs(&graph,owner).unwrap() else {panic!("unexpected deferred input");};assert_eq!(ids,[vec!["type.0".into(),"type.1".into(),"type.2".into()],selected.iter().map(|(_,id)|id.clone()).collect::<Vec<_>>()].concat());
   assert_eq!(definition_materialize_feature_defaults(&mut graph,&[("target","contribution")]).unwrap(),selected.len());
   graph.reverse();let mut graph:Vec<KirElement>=serde_json::from_value(serde_json::to_value(graph).unwrap()).unwrap();let before=serde_json::to_value(&graph).unwrap();assert_eq!(definition_materialize_feature_defaults(&mut graph,&[("target","replay")]).unwrap(),0);assert_eq!(serde_json::to_value(graph).unwrap(),before);
  }
 }
 #[test]
 fn definition_occurrence_contributions_preserve_dependencies_and_reject_partial_lifecycles() {
  let mut graph=fixture(&json!({"kind":"ViewUsage","context":"Class","composite":true,"typing_mask":1,"portion":"none"}));
  graph.iter_mut().find(|e|e.id=="type.0.typing").unwrap().properties.remove("type");
  assert!(matches!(names_query(&graph,graph.iter().find(|e|e.id=="target").unwrap()),Err(QueryFailure::Required(Prerequisite::ReadField{field,..})) if field=="type"));
  for change in ["end","parameter","crossing","variant"] {
   let mut graph=fixture(&json!({"kind":"ViewUsage","context":"ViewUsage","composite":true,"typing_mask":0,"portion":"none"}));
   match change {"end"=>{graph.iter_mut().find(|e|e.id=="target").unwrap().properties.insert("is_end".into(),json!(true));},"parameter"=>{graph.iter_mut().find(|e|e.id=="target").unwrap().properties.insert("direction".into(),json!("in"));},"variant"=>{graph.iter_mut().find(|e|e.id=="target.member").unwrap().kind="SysML::VariantMembership".into();},_=>{let mut r=fresh_definition_element("cross".into(),"CrossSubsetting").unwrap();ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id=="target").unwrap(),&mut r).unwrap();graph.push(r);}}
   let before=serde_json::to_value(&graph).unwrap();assert!(names(&graph,graph.iter().find(|e|e.id=="target").unwrap()).is_err(),"{change}");assert!(definition_materialize_feature_defaults(&mut graph,&[("target","failed")]).is_err());assert_eq!(serde_json::to_value(graph).unwrap(),before);
  }
 }
}
