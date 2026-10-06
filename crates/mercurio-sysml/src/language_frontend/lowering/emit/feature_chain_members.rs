//! Handwritten transactional graph execution of the resolved bounded producer.
//! This creates members only. Target/result typing, redefinitions, connectors,
//! multiplicities and transformation completion remain distinct dependencies.
use super::*;
pub(super) fn supports(kind:&str)->bool {feature_chain_members_generated::supports(kind)}
// The resolved Type.ownedFeature delegate filters a genuinely absent member
// value. Construction preserves such unfinished inputs; required-field/model
// validation remains separate and is never waived by this projection.
fn owned_features_for_construction<'g>(query:&DefinitionNameQuery<'g,'_>,owner:&KirElement)->Result<Vec<&'g KirElement>,QueryFailure> {
 let contract=ecore_model::feature(&owner.kind,"owned_feature").ok_or_else(||query_failure("missing owned Feature contract"))?;
 if !contract.setting_delegate.as_ref().is_some_and(|d|d.uri=="http://www.omg.org/spec/SysML"&&d.status=="custom_setting_delegate_source"&&d.candidates==["org.omg.sysml.delegate.setting.Type_ownedFeature_SettingDelegate"]) {return Err(query_failure("unsupported owned Feature delegate"));}
 let mut result=Vec::new();
 for member in stored_children_projection_query(query,owner,"owned_relationship")? {
  if !metaclass_conforms(&member.kind,"FeatureMembership") {continue;}
  let children=stored_children_projection_query(query,member,"owned_related_element")?;
  if children.len()>1 {return Err(query_failure("member construction requires at most one owned Feature"));}
  if let Some(feature)=children.first() {ecore_model::validate_reference_endpoint(&member.kind,"owned_member_feature",&feature.kind).map_err(error)?;result.push(*feature);}
 }
 Ok(result)
}
fn first_parameter<'g>(query:&DefinitionNameQuery<'g,'_>,owner:&'g KirElement)->Result<Option<&'g KirElement>,QueryFailure> {
 let basic=definition_chain_reference_targets_typed_query(query,owner,"feature_target")?.into_iter().next().ok_or_else(||query_failure("missing basic Feature"))?;
 let mut parameters=Vec::new();
 for feature in owned_features_for_construction(query,basic)? {
  let Some((_,owning))=definition_feature_owner_indexed(query.graph.len(),query.index,feature)? else {continue;};
  if parameter_collections_generated::is_parameter(metaclass_conforms(&owning.kind,parameter_collections_generated::OWNER_0),metaclass_conforms(&owning.kind,parameter_collections_generated::OWNER_1),definition_has_direction(feature)?) {parameters.push(feature);}
 }
 Ok(parameters.get(feature_chain_members_generated::SOURCE_POSITION).copied())
}
pub(super) fn ready_query(query:&DefinitionNameQuery<'_, '_>,owner:&KirElement)->Result<bool,QueryFailure> {
 if !supports(&owner.kind)||query.index.len()!=query.graph.len()||!query.index.get(owner.id.as_str()).is_some_and(|e|std::ptr::eq(*e,owner)) {return Err(query_failure("member strategy requires a canonical FeatureChainExpression"));}
 if !stored_children_projection_query(query,owner,"owned_relationship")?.iter().any(|r|metaclass_conforms(&r.kind,result_construction_generated::MEMBERSHIP)) {return Ok(false);}
 let owner=*query.index.get(owner.id.as_str()).unwrap();
 match first_parameter(query,owner)? {
  None=>Ok(true),
  Some(parameter)=>Ok(!owned_features_for_construction(query,parameter)?.is_empty()),
 }
}
pub(super) fn ready(graph:&[KirElement],owner:&KirElement)->Result<Option<bool>,Diagnostic> {
 if !supports(&owner.kind) {return Ok(None);}
 let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
 ready_query(&DefinitionNameQuery::new(graph,&index),owner).map(Some).map_err(|e|e.into_diagnostic_at("feature_chain_members_ready"))
}
pub(super) fn append(graph:&mut Vec<KirElement>,owner_id:&str,prefix:&str)->Result<bool,QueryFailure> {
 if prefix.is_empty() {return Err(query_failure("member construction requires a nonempty identity prefix"));}
 let owner=graph.iter().find(|e|e.id==owner_id).ok_or_else(||query_failure("member owner is absent"))?;
 if !supports(&owner.kind) {return Err(query_failure("unassessed additional-member producer"));}
 if let Some(issue)=ecore_model::validate_publication(graph,ecore_model::ReferenceCompleteness::Closed).first() {return Err(query_failure(format!("invalid member input: {}",issue.message)));}
 let _definition=feature_chain_members_generated::DEFINITION_SHA;
 let mut staged=graph.clone();
 let mut changed=append_owned_expression_result_storage(&mut staged,owner_id,&format!("{prefix}.return"),&format!("{prefix}.result"))?;
 let source={let index=staged.iter().map(|e|(e.id.as_str(),e)).collect::<BTreeMap<_,_>>();let query=DefinitionNameQuery::new(&staged,&index);
  match first_parameter(&query,index[owner_id])? {Some(p) if owned_features_for_construction(&query,p)?.is_empty()=>Some(p.id.clone()),_=>None}};
 if let Some(parameter)=source {
  let member_id=format!("{prefix}.source-target.member");let target_id=format!("{prefix}.source-target");
  if staged.iter().any(|e|e.id==member_id||e.id==target_id) {return Err(query_failure("source-target construction requires fresh identities"));}
  let mut membership=fresh_definition_element(member_id,"FeatureMembership")?;let mut target=fresh_definition_element(target_id,"Feature")?;
  target.properties.insert("declared_name".into(),json!(feature_chain_members_generated::SOURCE_NAME));
  ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut membership,&mut target)?;
  let parameter=staged.iter_mut().find(|e|e.id==parameter).ok_or_else(||query_failure("source parameter disappeared"))?;
  ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,parameter,&mut membership)?;
  staged.extend([membership,target]);changed=true;
 }
 if let Some(issue)=ecore_model::validate_publication(&staged,ecore_model::ReferenceCompleteness::Closed).first() {return Err(query_failure(format!("invalid member output: {}",issue.message)));}
 *graph=staged;Ok(changed)
}
