//! Direct relationship children follow the generated Xtext body arm and Ecore opposites.
use super::*;

pub(super) fn attach(
    scopes: &BTreeMap<String, String>, ids: &BTreeMap<String, String>, kerml: bool,
    elements: &mut [KirElement],
) -> Result<(), Diagnostic> {
    let positions = elements.iter().enumerate().map(|(i,e)| (e.id.clone(),i)).collect::<BTreeMap<_,_>>();
    for (name,id) in ids {
        let Some((prefix, local)) = name.rsplit_once('.') else {continue};
        // Anonymous usage keys add a source-position segment after their
        // synthetic local name; that segment does not introduce a namespace.
        let scope = if local.starts_with('@') {
            let Some((scope, _)) = prefix.rsplit_once('.') else {continue};
            scope
        } else { prefix };
        let Some(owner_id) = scopes.get(scope) else {continue};
        let (&owner_index,&child_index) = positions.get(owner_id).zip(positions.get(id))
            .ok_or_else(|| Diagnostic::new(format!("missing relationship-body endpoint {owner_id} -> {id}"), None))?;
        let child = &elements[child_index];
        if !crate::namespace_grammar::relationship_owns_element(kerml, &child.kind) {
            // The annotation arm is attached by the annotating-element emitter.
            if child.properties.contains_key("owning_relationship") {continue;}
            return Err(Diagnostic::new(format!("unimplemented relationship-body construction for {}",child.kind),None));
        }
        if owner_index==child_index { return Err(Diagnostic::new("self-owned relationship body",None)); }
        let (owner,child) = if owner_index<child_index {
            let (left,right)=elements.split_at_mut(child_index); (&mut left[owner_index],&mut right[0])
        } else {
            let (left,right)=elements.split_at_mut(owner_index); (&mut right[0],&mut left[child_index])
        };
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,owner,child)?;
        // Element.owner is the owningRelatedElement of its owningRelationship,
        // not the containing Relationship itself (KerML 8.3.2.1.2).
        if let Some(element_owner) = owner.properties.get("owning_related_element") {
            child.properties.insert("owner".into(), element_owner.clone());
        } else {
            child.properties.remove("owner");
        }
        child.properties.remove("owning_namespace");
        child.properties.remove("owning_type");
    }
    // Internal resolver keys do not imply a namespace-qualified model name.
    for (name,id) in ids {
        if scopes.keys().any(|scope| name.starts_with(&format!("{scope}."))) {
            if let Some(&index)=positions.get(id) {elements[index].properties.remove("qualified_name");}
        }
    }
    let order=elements.iter().map(|e| {
        let span=&e.properties["metadata"]["source_span"];
        (e.id.clone(),(span["start_line"].as_u64().unwrap_or(u64::MAX),span["start_col"].as_u64().unwrap_or(u64::MAX)))
    }).collect::<BTreeMap<_,_>>();
    for owner_id in scopes.values() {
        if let Some(&index)=positions.get(owner_id) {
            if let Some(Value::Array(children))=elements[index].properties.get_mut("owned_related_element") {
                children.sort_by_key(|v|v.as_str().and_then(|id|order.get(id)).copied().unwrap_or((u64::MAX,u64::MAX)));
            }
        }
    }
    Ok(())
}
