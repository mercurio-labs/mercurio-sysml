//! Native retained-registry equivalence and adversarial controls. No Pilot
//! endpoint or prepared transformation state is supplied to either path.
use super::*;
use mercurio_foundation::kir::KirElement;

fn sources() -> [DefinitionSource<'static>; 2] {
    [DefinitionSource {uri:"cached-library.kerml",text:"package L { class A; class B; }",language:SourceLanguage::Kerml},
     DefinitionSource {uri:"cached-source.kerml",text:"package P { feature selected : L::A; feature next : L::B; feature ignored : Missing; }",language:SourceLanguage::Kerml}]
}
fn resources<'a>(sources: &'a [DefinitionSource<'a>]) -> Vec<DefinitionConstructedResource<'a>> {
    sources.iter().map(|s| DefinitionConstructedResource {uri:s.uri,language:s.language}).collect()
}
fn port(inspection: &DefinitionStructureInspection, name: &str) -> DefinitionQueryPrerequisite {
    let owner = inspection.elements().iter().find(|e| e.properties.get("declared_name") == Some(&json!(name))).unwrap();
    let p = inspection.pending_references().iter().find(|p| inspection.elements().iter()
        .find(|e| e.id == p.owner_id).unwrap().properties.get("owning_related_element") == Some(&json!(owner.id))).unwrap();
    DefinitionQueryPrerequisite::ReadField {owner_id:p.owner_id.clone(),field:p.field.clone()}
}
type Event = (String, String, Option<String>, usize, Option<bool>);
fn event(e: DefinitionConstructionEvent<'_>) -> Event {
    (format!("{:?}", e.phase), e.owner_id.to_owned(), e.field.map(str::to_owned), e.element_count, e.succeeded)
}
fn linker_events(events: Vec<Event>) -> Vec<Event> {
    let start = events.iter().position(|e| e.0 == "LinkingStarted").unwrap();
    events.into_iter().skip(start).collect()
}

#[test]
fn definition_cached_dependencies_match_fresh_cross_resource_resolution_and_trace() {
    let sources = sources(); let original = inspect_source_structure(&sources).unwrap();
    let pending: Vec<DefinitionPendingReference> = serde_json::from_value(serde_json::to_value(original.pending_references()).unwrap()).unwrap();
    let wave = [port(&original,"selected"),port(&original,"next"),port(&original,"selected")];
    let before = serde_json::to_value(&original).unwrap();
    let mut fresh_events = Vec::new();
    let fresh = inspect_source_dependencies_traced(&sources,&wave,|_,_|{},|e| fresh_events.push(event(e))).unwrap();
    let mut cached_events = Vec::new();
    let cached = inspect_constructed_dependencies_traced(&resources(&sources),original.elements(),&pending,&wave,|e|cached_events.push(event(e))).unwrap();
    assert_eq!(serde_json::to_value(&cached).unwrap(),serde_json::to_value(&fresh).unwrap());
    assert_eq!(linker_events(fresh_events),cached_events);
    assert_eq!(cached.pending_references().len()+2,original.pending_references().len());
    assert_eq!(before,serde_json::to_value(&original).unwrap());
    assert!(parse_and_link_sources(&sources).is_err());
    let encoded=serde_json::to_value(cached).unwrap();
    assert_eq!(encoded["transformation_completion"],"not_assessed");
    assert_eq!(encoded["semantic_validation"],"not_assessed");
    assert!(encoded.get("model").is_none());
}

#[test]
fn definition_cached_dependencies_match_fresh_import_and_language_scope() {
    for (language,text,uri) in [
        (SourceLanguage::Kerml,"package L { class A; } package P { private import L::*; feature selected : A; }","cached-import.kerml"),
        (SourceLanguage::Sysml,"package L { part def A; } package P { part selected : L::A; }","cached-import.sysml"),
    ] {
        let sources=[DefinitionSource {uri,text,language}];
        let original=inspect_source_structure(&sources).unwrap();
        let wave=original.pending_references().iter().map(|p|DefinitionQueryPrerequisite::ReadField {owner_id:p.owner_id.clone(),field:p.field.clone()}).collect::<Vec<_>>();
        let fresh=inspect_source_dependencies(&sources,&wave).unwrap();
        let cached=inspect_constructed_dependencies(&resources(&sources),original.elements(),original.pending_references(),&wave).unwrap();
        assert_eq!(serde_json::to_value(cached).unwrap(),serde_json::to_value(fresh).unwrap(),"{uri}");
    }
}

#[test]
fn definition_cached_dependencies_share_additional_member_construction() {
    let sources=[DefinitionSource {uri:"cached-members.kerml",text:"package P { feature target; feature selected = target; }",language:SourceLanguage::Kerml}];
    let original=inspect_source_structure(&sources).unwrap();
    let expression=original.elements().iter().find(|e|e.kind.ends_with("FeatureReferenceExpression")).unwrap();
    let wave=[DefinitionQueryPrerequisite::AdditionalMembers {owner_id:expression.id.clone()}];
    let fresh=inspect_source_dependencies(&sources,&wave).unwrap();
    let cached=inspect_constructed_dependencies(&resources(&sources),original.elements(),original.pending_references(),&wave).unwrap();
    assert_eq!(serde_json::to_value(&cached).unwrap(),serde_json::to_value(fresh).unwrap());
    assert_eq!(cached.elements().len(),original.elements().len()+2);
    assert_eq!(serde_json::to_value(cached.pending_references()).unwrap(),serde_json::to_value(original.pending_references()).unwrap());
    assert!(cached.elements().iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
}

#[test]
fn definition_cached_dependencies_resume_only_remaining_native_ports() {
    let sources=sources();let original=inspect_source_structure(&sources).unwrap();
    let first=[port(&original,"selected")];let second=[port(&original,"next")];
    let partial=inspect_constructed_dependencies(&resources(&sources),original.elements(),original.pending_references(),&first).unwrap();
    let resumed=inspect_constructed_dependencies(&resources(&sources),partial.elements(),partial.pending_references(),&second).unwrap();
    let fresh=inspect_source_dependencies(&sources,&[first[0].clone(),second[0].clone()]).unwrap();
    assert_eq!(serde_json::to_value(&resumed).unwrap(),serde_json::to_value(fresh).unwrap());
    let mut committed=false;
    let replayed=inspect_constructed_dependencies_traced(&resources(&sources),partial.elements(),partial.pending_references(),&first,|event| {
        committed|=event.phase==DefinitionConstructionPhase::LinkFieldCommitted;
    }).unwrap();
    assert!(!committed,"completed port replay cannot perform a link write");
    assert_eq!(serde_json::to_value(&replayed).unwrap(),serde_json::to_value(&partial).unwrap(),"completed port replay preserves the retained graph and pending registry");
}

#[test]
fn definition_cached_dependencies_are_atomic_after_a_real_link_commit() {
    let sources=sources();let original=inspect_source_structure(&sources).unwrap();let before=serde_json::to_value(&original).unwrap();
    let wave=[port(&original,"selected"),port(&original,"ignored")];let mut commits=Vec::new();
    let failure=inspect_constructed_dependencies_traced(&resources(&sources),original.elements(),original.pending_references(),&wave,|e| {
        if e.phase==DefinitionConstructionPhase::LinkFieldCommitted {commits.push(e.owner_id.to_owned());}
    }).unwrap_err();
    assert_eq!(commits.len(),1,"failure must follow an actual native write");
    assert_eq!(failure.to_string(),inspect_source_dependencies(&sources,&wave).unwrap_err().to_string());
    assert_eq!(before,serde_json::to_value(&original).unwrap(),"no failed output or partial input mutation");
}

#[test]
fn definition_cached_dependencies_validate_entire_wave_before_work() {
    let sources=sources();let original=inspect_source_structure(&sources).unwrap();let real=port(&original,"selected");
    for wave in [vec![],vec![real.clone(),DefinitionQueryPrerequisite::ReadField {owner_id:"foreign".into(),field:"type".into()}],
        vec![real.clone(),DefinitionQueryPrerequisite::AdditionalMembers {owner_id:"foreign".into()}]] {
        let mut committed=false;
        assert!(inspect_constructed_dependencies_traced(&resources(&sources),original.elements(),original.pending_references(),&wave,|e| {
            committed|=e.phase==DefinitionConstructionPhase::LinkFieldCommitted;
        }).is_err());
        assert!(!committed,"invalid wave cannot execute its valid prefix");
    }
}

#[test]
fn definition_cached_dependencies_reject_tampered_registry_before_work() {
    let sources=sources();let original=inspect_source_structure(&sources).unwrap();let wave=[port(&original,"selected")];
    for case in 0..12 {
        let mut pending=original.pending_references().to_vec();
        match case {
            0=>pending[0].owner_id="absent".into(),
            1=>pending[0].owner_kind="SysML::Class".into(),
            2=>pending[0].field="unknown".into(),
            3=>pending[0].feature_id.push_str("-changed"),
            4=>pending[0].ecore_target="Feature".into(),
            5=>pending[0].grammar_target="foreign://Type".into(),
            6=>pending[0].grammar_target=pending[0].grammar_target.replace("#//Type","#//OwningMembership"),
            7=>pending[0].spelling.clear(),
            8=>pending[0].spelling="Bad\0Name".into(),
            9=>pending[0].reference_ordinal=1,
            10=>pending.push(pending[0].clone()),
            11=>{pending.remove(0);},
            _=>unreachable!(),
        };
        let mut committed=false;
        assert!(inspect_constructed_dependencies_traced(&resources(&sources),original.elements(),&pending,&wave,|e|{
            committed|=e.phase==DefinitionConstructionPhase::LinkFieldCommitted;
        }).is_err(),"case{case}");
        assert!(!committed,"tampered registry case{case}");
    }
}

#[test]
fn definition_cached_dependencies_reject_stale_and_invalid_graphs() {
    let sources=sources();let original=inspect_source_structure(&sources).unwrap();let wave=[port(&original,"selected")];
    let p=&original.pending_references()[0];
    for case in 0..6 {
        let mut graph:Vec<KirElement>=original.elements().to_vec();
        match case {
            0=>graph.push(graph[0].clone()),
            1=>graph[0].id.clear(),
            2=>graph[0].kind="SysML::Unknown".into(),
            3=>graph[0].id="foreign.root".into(),
            4=>{graph.iter_mut().find(|e|e.id==p.owner_id).unwrap().properties.insert(p.field.clone(),json!("absent"));},
            5=>{graph.iter_mut().find(|e|e.id==p.owner_id).unwrap().properties.insert("owning_related_element".into(),json!("absent"));},
            _=>unreachable!(),
        }
        assert!(inspect_constructed_dependencies(&resources(&sources),&graph,original.pending_references(),&wave).is_err(),"case{case}");
    }
    for contexts in [vec![],vec![resources(&sources)[0],resources(&sources)[0]],
        vec![DefinitionConstructedResource {uri:"unknown.kerml",language:SourceLanguage::Kerml}],
        vec![DefinitionConstructedResource {uri:" ",language:SourceLanguage::Kerml}]] {
        assert!(inspect_constructed_dependencies(&contexts,original.elements(),original.pending_references(),&wave).is_err());
    }
}

#[test]
fn definition_cached_dependencies_preserve_rational_storage_on_json_round_trip() {
    for number in [1e-28_f64,1e-24_f64,-1e-28_f64,-0.0,f64::MIN_POSITIVE,f64::from_bits(1),f64::MAX] {
        let original=KirElement {id:"rational".into(),kind:"SysML::LiteralRational".into(),layer:2,
            properties:std::collections::BTreeMap::from([("value".into(),json!(number))])};
        let encoded=serde_json::to_string(&original).unwrap();
        let decoded:KirElement=serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.properties["value"].as_f64().unwrap().to_bits(),number.to_bits(),"{encoded}");
    }
    let sources=[DefinitionSource {uri:"rational-cache.kerml",
        text:"package P { feature x = 0.0000000000000000000000000001; feature y = x; }",language:SourceLanguage::Kerml}];
    let original=inspect_source_structure(&sources).unwrap();
    let restored:Vec<KirElement>=serde_json::from_str(&serde_json::to_string(original.elements()).unwrap()).unwrap();
    let literal=original.elements().iter().find(|e|e.kind.ends_with("LiteralRational")).unwrap();
    assert_eq!(literal.properties["value"].as_f64().unwrap().to_bits(),1e-28_f64.to_bits());
    let persisted=restored.iter().find(|e|e.id==literal.id).unwrap();
    assert_eq!(persisted.properties["value"],literal.properties["value"]);
    let expression=original.elements().iter().find(|e|e.kind.ends_with("FeatureReferenceExpression")).unwrap();
    let wave=[DefinitionQueryPrerequisite::AdditionalMembers {owner_id:expression.id.clone()}];
    let native=inspect_constructed_dependencies(&resources(&sources),&restored,original.pending_references(),&wave).unwrap();
    assert_eq!(native.elements().iter().find(|e|e.id==literal.id).unwrap().properties["value"],literal.properties["value"]);
}
