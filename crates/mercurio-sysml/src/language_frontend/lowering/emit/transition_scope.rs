//! Handwritten canonical navigation for a bounded resolved Xtend scope rule.
//! No target/chain/global/validation support is inferred from importing getScope.
use super::*;
#[path="transition_scope_generated.rs"] mod policy;
pub(super) fn source_origin<'g>(graph:&'g [KirElement],index:&BTreeMap<&str,&'g KirElement>,relationship:&KirElement)->Result<Option<Option<&'g KirElement>>,Diagnostic> {
 if !metaclass_conforms(&relationship.kind,policy::MEMBERSHIP) {return Ok(None);}
 let Some(owner)=generated_parent_namespace_indexed(index,&relationship.id)? else {return Ok(None);};
 if !metaclass_conforms(&owner.kind,policy::TRANSITION) {return Ok(None);}
 let first=stored_children(graph,owner,"owned_relationship")?.into_iter().find(|m|metaclass_conforms(&m.kind,policy::MEMBERSHIP) && !metaclass_conforms(&m.kind,policy::EXCLUDED_MEMBERSHIP));
 if !first.is_some_and(|m|m.id==relationship.id) {return Ok(None);}
 Ok(Some(generated_parent_namespace_indexed(index,&owner.id)?))
}
#[cfg(test)] mod tests {
 use super::*;
 fn own(g:&mut Vec<KirElement>,owner:&str,mut m:KirElement,mut child:KirElement) {
  ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut m,&mut child).unwrap();ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,g.iter_mut().find(|e|e.id==owner).unwrap(),&mut m).unwrap();g.extend([m,child]);
 }
 #[test] fn definition_transition_source_scope_origin_matches_pilot() {
  let d:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/conformance/2026-08-support/transition-scope-pilot-controls.json"))).unwrap();let cases=d["cases"].as_array().unwrap();assert_eq!(cases.len(),54);
  for c in cases {
   let mut g=vec![fresh_definition_element("parent".into(),c["parent_kind"].as_str().unwrap()).unwrap()];
   own(&mut g,"parent",fresh_definition_element("owner.member".into(),"FeatureMembership").unwrap(),fresh_definition_element("owner".into(),c["owner_kind"].as_str().unwrap()).unwrap());
   for (id,kind) in [("before",c["preceding"].as_str().unwrap()),("source",c["membership"].as_str().unwrap())] {
    if kind=="none" {continue;}let m=fresh_definition_element(id.into(),kind).unwrap();
    if metaclass_conforms(kind,"OwningMembership") {own(&mut g,"owner",m,fresh_definition_element(format!("{id}.child"),"Feature").unwrap());} else {let mut m=m;ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,g.iter_mut().find(|e|e.id=="owner").unwrap(),&mut m).unwrap();g.push(m);}
   }
   let read=|g:&[KirElement]| {let index=g.iter().map(|e|(e.id.as_str(),e)).collect::<BTreeMap<_,_>>();let source=index["source"];source_origin(g,&index,source).unwrap().unwrap_or(Some(index["owner"])).unwrap().id.clone()};
   let before=serde_json::to_value(&g).unwrap();assert_eq!(read(&g),c["scope_origin"].as_str().unwrap(),"{c}");assert_eq!(before,serde_json::to_value(&g).unwrap());g.reverse();let restored:Vec<KirElement>=serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();assert_eq!(read(&restored),c["scope_origin"].as_str().unwrap());
  }
 }
}
