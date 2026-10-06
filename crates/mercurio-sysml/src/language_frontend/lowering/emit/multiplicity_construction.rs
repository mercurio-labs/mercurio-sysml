//! Handwritten transactional execution and canonical projection for imported policy.
//! Bare multiplicity creation does not supply bounds, featuring or validation.
use super::*;
#[path="multiplicity_construction_generated.rs"]
mod policy;

pub(super) fn admission(graph: &[KirElement], owner: &KirElement) -> Result<bool, Diagnostic> {
 let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
 admission_in_view(&DefinitionNameQuery::new(graph,&index),owner).map_err(|failure|failure.into_diagnostic_at("multiplicity admission external boundary"))
}

pub(super) fn admission_in_view(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement) -> Result<bool,QueryFailure> {
 let mode=policy::uses_default_admission(owner.kind.rsplit("::").next().unwrap_or(&owner.kind)).ok_or_else(||error("unassessed multiplicity member provider"))?;
 let end=scope_boolean(owner,"is_end")?;
 if end || !mode { return Ok(end); }
 let owning=definition_feature_owner_indexed(query.graph.len(),query.index,owner)?.is_some();
 if !owning {return Ok(false);}
 let mut subset_owned=false;
 for relation in stored_children_projection_query(query,owner,"owned_relationship")? {
  if !metaclass_conforms(&relation.kind,"Subsetting") {continue;}
  let feature=query_reference_in_view(query,relation,"subsetted_feature")?;
  let chain=definition_chain_reference_targets_typed_query(query,feature,"chaining_feature")?;
  let basic=chain.last().copied().unwrap_or(feature);
  subset_owned |= definition_feature_owner_indexed(query.graph.len(),query.index,basic)?.is_some();
 }
 Ok(policy::admitted(mode,end,owning,subset_owned))

}
pub(super) fn supports(kind: &str) -> bool {policy::uses_default_admission(kind.rsplit("::").next().unwrap_or(kind)).is_some()}
pub(super) fn ready_in_view(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement) -> Result<Option<bool>,QueryFailure> {
 if !supports(&owner.kind) {return Ok(None);}
 Ok(Some(!admission_in_view(query,owner)? || existing_in_view(query,owner)?))

}
fn existing(graph: &[KirElement],owner: &KirElement) -> Result<bool,Diagnostic> {
 let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
 existing_in_view(&DefinitionNameQuery::new(graph,&index),owner).map_err(|failure|failure.into_diagnostic_at("multiplicity existing external boundary"))
}

fn existing_in_view(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement) -> Result<bool,QueryFailure> {
 for member in stored_children_projection_query(query,owner,"owned_relationship")? {
  if !metaclass_conforms(&member.kind,policy::MEMBERSHIP) {continue;}
  if stored_children_projection_query(query,member,"owned_related_element")?.iter().any(|e|metaclass_conforms(&e.kind,policy::MULTIPLICITY)) {return Ok(true);}
 }
 Ok(false)

}
pub(super) fn append(graph: &mut Vec<KirElement>,owner_id: &str,member_id: &str,value_id: &str) -> Result<bool,Diagnostic> {
 let owner=graph.iter().find(|e|e.id==owner_id).ok_or_else(||error("multiplicity owner is absent"))?;
 if !admission(graph,owner)? || existing(graph,owner)? {return Ok(false);}
 if member_id.is_empty() || value_id.is_empty() || member_id==value_id || graph.iter().any(|e|e.id==member_id || e.id==value_id) {return Err(error("multiplicity requires fresh distinct nonempty identities"));}
 let mut member=fresh_definition_element(member_id.into(),policy::MEMBERSHIP)?;
 let mut value=fresh_definition_element(value_id.into(),policy::MULTIPLICITY)?;
 ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut member,&mut value)?;
 let owner=graph.iter_mut().find(|e|e.id==owner_id).unwrap();
 ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,owner,&mut member)?;
 graph.extend([member,value]);Ok(true)
}
/// Atomic batch across exact inherited Usage member-provider bindings.
/// Classes with a custom additional-member lifecycle (including TransitionUsage)
/// remain excluded, rather than receiving an incomplete substitute lifecycle.
pub(crate) fn definition_materialize_usage_multiplicities(graph: &mut Vec<KirElement>,prefix: &str) -> Result<usize,Diagnostic> {
 if prefix.is_empty() {return Err(error("multiplicity batch requires an identity prefix"));}
 if let Some(issue)=ecore_model::validate_publication(graph,ecore_model::ReferenceCompleteness::Closed).first() {return Err(error(format!("invalid multiplicity input: {}",issue.message)));}
 for e in graph.iter() {for field in e.properties.keys() {
  if ecore_model::feature(&e.kind,field).is_some_and(|c| c.derived || c.volatile) {return Err(error(format!("multiplicity snapshot {} {}",e.kind,field)));}
 }}
 let mut owners=graph.iter().filter(|e|policy::uses_default_admission(e.kind.rsplit("::").next().unwrap_or(&e.kind)).is_some()).map(|e|e.id.clone()).collect::<Vec<_>>();owners.sort();
 let mut staged=graph.clone();let mut count=0;
 for owner in owners {count+=usize::from(append(&mut staged,&owner,&format!("{prefix}.{owner}.member"),&format!("{prefix}.{owner}.multiplicity"))?);}
 if let Some(issue)=ecore_model::validate_publication(&staged,ecore_model::ReferenceCompleteness::Closed).first() {return Err(error(format!("invalid multiplicity output: {}",issue.message)));}
 *graph=staged;Ok(count)
}

#[cfg(test)]
mod tests {
 use super::*;
 fn own(graph:&mut Vec<KirElement>,owner:&str,child:KirElement,kind:&str) {cross_own(graph,owner,child,kind);}
 fn cross_own(graph:&mut Vec<KirElement>,owner:&str,mut child:KirElement,kind:&str) {
  let mut member=fresh_definition_element(format!("{}.member",child.id),kind).unwrap();
  ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut member,&mut child).unwrap();
  ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id==owner).unwrap(),&mut member).unwrap();graph.extend([member,child]);
 }
 #[test]
 fn definition_multiplicity_construction_matches_all_pilot_controls() {
  let data:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/conformance/2026-08-support/multiplicity-construction-pilot-controls.json"))).unwrap();
  assert_eq!(data["controls"].as_array().unwrap().len(),4050);
  for c in data["controls"].as_array().unwrap() {
   let mut target=fresh_definition_element("target".into(),c["kind"].as_str().unwrap()).unwrap();target.properties.insert("is_end".into(),c["is_end"].clone());
   let mut graph=vec![];let owner=c["owner"].as_str().unwrap();
   if owner=="detached" {graph.push(target);}else {graph.push(fresh_definition_element("owner".into(),owner).unwrap());own(&mut graph,"owner",target,"FeatureMembership");}
   let subset=c["subset"].as_str().unwrap();
   if subset!="none" {
    graph.push(fresh_definition_element("general".into(),"Feature").unwrap());
    let id=if subset.starts_with("chain_") {graph.push(fresh_definition_element("basic".into(),"Feature").unwrap());let mut r=fresh_definition_element("chain".into(),"FeatureChaining").unwrap();r.properties.insert("chaining_feature".into(),json!("basic"));ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id=="general").unwrap(),&mut r).unwrap();graph.push(r);"basic"}else{"general"};
    if subset.ends_with("owned") {graph.push(fresh_definition_element("context".into(),"Class").unwrap());let at=graph.iter().position(|e|e.id==id).unwrap();let feature=graph.remove(at);own(&mut graph,"context",feature,"FeatureMembership");}
    let mut r=fresh_definition_element("subset".into(),"Subsetting").unwrap();r.properties.insert("subsetting_feature".into(),json!("target"));r.properties.insert("subsetted_feature".into(),json!("general"));ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id=="target").unwrap(),&mut r).unwrap();graph.push(r);
   }
   match c["existing"].as_str().unwrap() {
    "owned_range"=>own(&mut graph,"target",fresh_definition_element("old".into(),"MultiplicityRange").unwrap(),"OwningMembership"),
    "alias"=>{graph.push(fresh_definition_element("alias".into(),"Multiplicity").unwrap());let mut r=fresh_definition_element("alias.member".into(),"Membership").unwrap();r.properties.insert("member_element".into(),json!("alias"));ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,graph.iter_mut().find(|e|e.id=="target").unwrap(),&mut r).unwrap();graph.push(r);},_=>{}
   }
   assert_eq!(admission(&graph,graph.iter().find(|e|e.id=="target").unwrap()).unwrap(),c["admitted"].as_bool().unwrap(),"{c}");
   assert_eq!(definition_additional_members_ready(&graph,graph.iter().find(|e|e.id=="target").unwrap()).unwrap(),!c["admitted"].as_bool().unwrap() || c["before"]==1,"{c}");
   let target=graph.iter().find(|e|e.id=="target").unwrap();let ready=!c["admitted"].as_bool().unwrap() || c["before"]==1;
   if ready {assert!(definition_compatibility_owned_features_empty_query(&graph,target).unwrap(),"{c}");}else{assert!(definition_compatibility_owned_features_empty_query(&graph,target).is_err(),"{c}");}
   let changed=definition_materialize_usage_multiplicities(&mut graph,"generated").unwrap();assert_eq!(changed,(c["after"].as_u64().unwrap()-c["before"].as_u64().unwrap()) as usize,"{c}");
   assert!(definition_additional_members_ready(&graph,graph.iter().find(|e|e.id=="target").unwrap()).unwrap());
   assert!(definition_compatibility_owned_features_empty_query(&graph,graph.iter().find(|e|e.id=="target").unwrap()).unwrap(),"{c}");
   assert!(!scope_boolean(graph.iter().find(|e|e.id=="target").unwrap(),"is_implied_included").unwrap());
   graph.reverse();let mut graph:Vec<KirElement>=serde_json::from_value(serde_json::to_value(graph).unwrap()).unwrap();let before=serde_json::to_value(&graph).unwrap();assert_eq!(definition_materialize_usage_multiplicities(&mut graph,"replay").unwrap(),0);assert_eq!(serde_json::to_value(&graph).unwrap(),before);
   assert!(definition_compatibility_owned_features_empty_query(&graph,graph.iter().find(|e|e.id=="target").unwrap()).unwrap(),"{c}");
  }
 }
 #[test]
 fn definition_compatibility_owned_features_preserve_declared_children_and_pending_producers() {
  for kind in ["StateUsage","ActionUsage","Usage"] {
   let mut graph=vec![fresh_definition_element("target".into(),kind).unwrap()];
   assert!(definition_compatibility_owned_features_empty_query(&graph,&graph[0]).unwrap());
   own(&mut graph,"target",fresh_definition_element("child".into(),"Feature").unwrap(),"FeatureMembership");
   let restored:Vec<KirElement>=serde_json::from_value(serde_json::to_value(&graph).unwrap()).unwrap();
   for mut g in [graph,restored] {for _ in 0..2 {
    let target=g.iter().find(|e|e.id=="target").unwrap();let before=serde_json::to_value(&g).unwrap();assert!(!definition_compatibility_owned_features_empty_query(&g,target).unwrap());assert!(!scope_boolean(target,"is_implied_included").unwrap());assert_eq!(before,serde_json::to_value(&g).unwrap());g.reverse();
   }}
  }
  let mut graph=vec![fresh_definition_element("target".into(),"StateUsage").unwrap()];graph[0].properties.insert("is_end".into(),json!(true));let before=serde_json::to_value(&graph).unwrap();assert!(definition_compatibility_owned_features_empty_query(&graph,&graph[0]).is_err());assert_eq!(before,serde_json::to_value(&graph).unwrap());
  assert!(append(&mut graph,"target","multiplicity.member","multiplicity").unwrap());assert!(definition_compatibility_owned_features_empty_query(&graph,&graph[0]).unwrap());
  graph[0].properties.insert("is_end".into(),json!("true"));assert!(definition_compatibility_owned_features_empty_query(&graph,&graph[0]).is_err());
 }
 #[test]
 fn definition_multiplicity_construction_rolls_back_and_excludes_custom_lifecycles() {
  let mut a=fresh_definition_element("a".into(),"Usage").unwrap();a.properties.insert("is_end".into(),json!(true));
  let mut z=a.clone();z.id="z".into();let mut graph=vec![a,z,fresh_definition_element("batch.z.multiplicity".into(),"Multiplicity").unwrap()];
  let before=serde_json::to_value(&graph).unwrap();assert!(definition_materialize_usage_multiplicities(&mut graph,"batch").is_err());assert_eq!(serde_json::to_value(&graph).unwrap(),before);
  let mut graph=vec![fresh_definition_element("transition".into(),"TransitionUsage").unwrap()];assert_eq!(definition_materialize_usage_multiplicities(&mut graph,"batch").unwrap(),0);
  assert!(admission(&graph,&graph[0]).is_err());
 }
}
