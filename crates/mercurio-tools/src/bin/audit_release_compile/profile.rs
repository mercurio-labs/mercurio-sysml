//! Tool-owned diagnostic cost accounting. Kernel events carry no clocks or I/O.
//! Reference attempts and staged field writes are distinct from publication.
use mercurio_sysml::definition_document::{DefinitionConstructionEvent as Event, DefinitionConstructionPhase as Phase};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Default)]
struct Cost {
    completed: usize,
    inclusive_ns: u64,
    exclusive_ns: u64,
}
struct Frame {
    started: Phase,
    finished: Phase,
    owner: String,
    field: Option<String>,
    began_ns: u64,
    children_ns: u64,
}
#[derive(Default)]
pub(super) struct QueryProfile {
    counts: BTreeMap<Phase, usize>,
    costs: BTreeMap<Phase, Cost>,
    frames: Vec<Frame>,
    active_field: Option<(String, Option<String>, Option<String>, u64)>,
    committed_fields: usize,
    unbalanced_pairs: usize,
}

fn finish_for(phase: Phase) -> Option<Phase> {
    use Phase::*;
    Some(match phase {
        ResourceStarted=>ResourceFinished, LinkingStarted=>LinkingFinished,
        ReferenceStarted=>ReferenceFinished, PublicationStarted=>PublicationFinished,
        MembershipNamesStarted=>MembershipNamesFinished,
        GeneralTypesStarted=>GeneralTypesFinished,
        InheritedMembershipsStarted=>InheritedMembershipsFinished,
        LibraryPathStarted=>LibraryPathFinished, ScopeMembershipsStarted=>ScopeMembershipsFinished,
        RedefinitionFilterStarted=>RedefinitionFilterFinished,
        RedefinitionClosureStarted=>RedefinitionClosureFinished,
        RedefinitionClosureEvaluationStarted=>RedefinitionClosureEvaluationFinished,
        ExplicitRedefinitionReadinessStarted=>ExplicitRedefinitionReadinessFinished,
        MetadataSelectionStarted=>MetadataSelectionFinished,
        LiteralProducerStarted=>LiteralProducerFinished, LiteralStageStarted=>LiteralStageFinished,
        ContainmentReadStarted=>ContainmentReadFinished, ContainerReadStarted=>ContainerReadFinished,
        _=>return None,
    })
}
fn is_finish(phase: Phase) -> bool {
    use Phase::*;
    matches!(phase,ResourceFinished|LinkingFinished|ReferenceFinished|PublicationFinished
        |MembershipNamesFinished|GeneralTypesFinished|InheritedMembershipsFinished
        |LibraryPathFinished|ScopeMembershipsFinished|RedefinitionFilterFinished
        |RedefinitionClosureFinished|RedefinitionClosureEvaluationFinished
        |ExplicitRedefinitionReadinessFinished|MetadataSelectionFinished
        |LiteralProducerFinished|LiteralStageFinished|ContainmentReadFinished|ContainerReadFinished)
}

impl QueryProfile {
    /// Time is supplied by the tool so deterministic controls need no sleeps.
    pub(super) fn observe(&mut self, event: Event<'_>, elapsed_ns: u64) {
        *self.counts.entry(event.phase).or_default()+=1;
        if event.phase==Phase::LinkFieldStarted {
            self.active_field=Some((event.owner_id.into(),event.owner_kind.map(str::to_owned),event.field.map(str::to_owned),elapsed_ns));
        }
        if event.phase==Phase::LinkFieldCommitted {
            self.committed_fields+=1;
            if self.active_field.as_ref().is_some_and(|(owner,_,field,_)|owner==event.owner_id && field.as_deref()==event.field) {
                self.active_field=None;
            }
        }
        if let Some(finished)=finish_for(event.phase) {
            self.frames.push(Frame {started:event.phase,finished,owner:event.owner_id.into(),
                field:event.field.map(str::to_owned),began_ns:elapsed_ns,children_ns:0});
        } else if is_finish(event.phase) {
            // Do not silently attribute a mismatched sequence to a different read.
            let matches=self.frames.last().is_some_and(|frame|frame.finished==event.phase
                && frame.owner==event.owner_id && frame.field.as_deref()==event.field);
            if !matches {self.unbalanced_pairs+=1;return;}
            let Some(frame)=self.frames.pop() else {return;};
            let elapsed=elapsed_ns.saturating_sub(frame.began_ns);
            let cost=self.costs.entry(frame.started).or_default();
            cost.completed+=1;cost.inclusive_ns+=elapsed;
            cost.exclusive_ns+=elapsed.saturating_sub(frame.children_ns);
            if let Some(parent)=self.frames.last_mut() {parent.children_ns+=elapsed;}
        }
    }

    pub(super) fn snapshot(&self, elapsed_ns: u64) -> Value {
        let mut costs=self.costs.iter().map(|(phase,cost)|(*phase,(cost.completed,cost.inclusive_ns,cost.exclusive_ns,0usize))).collect::<BTreeMap<_,_>>();
        let mut unfinished=Vec::new();
        for (i,frame) in self.frames.iter().enumerate() {
            let elapsed=elapsed_ns.saturating_sub(frame.began_ns);
            let open_child=self.frames.get(i+1).map_or(0,|child|elapsed_ns.saturating_sub(child.began_ns));
            let exclusive=elapsed.saturating_sub(frame.children_ns).saturating_sub(open_child);
            let cost=costs.entry(frame.started).or_default();
            cost.1+=elapsed;cost.2+=exclusive;cost.3+=1;
            unfinished.push(json!({"phase":format!("{:?}",frame.started),"owner_id":frame.owner,
                "field":frame.field,"elapsed_ms":elapsed as f64/1e6,"exclusive_ms":exclusive as f64/1e6}));
        }
        let mut readers=costs.into_iter().map(|(phase,(completed,inclusive,exclusive,open))|
            json!({"phase":format!("{phase:?}"),"completed_calls":completed,"open_calls":open,
                "inclusive_ms":inclusive as f64/1e6,"exclusive_ms":exclusive as f64/1e6})).collect::<Vec<_>>();
        readers.sort_by(|a,b|b["exclusive_ms"].as_f64().unwrap_or(0.0).total_cmp(&a["exclusive_ms"].as_f64().unwrap_or(0.0)));
        json!({"measurement":"instrumented_native_diagnostic; not a Pilot timing comparison",
            "elapsed_ms":elapsed_ns as f64/1e6,"staged_reference_fields_committed":self.committed_fields,
            "active_field":self.active_field.as_ref().map(|(owner,kind,field,began)|
                json!({"owner_id":owner,"kind":kind,"field":field,"attempt_elapsed_ms":elapsed_ns.saturating_sub(*began) as f64/1e6})),
            "event_counts":self.counts.iter().map(|(phase,count)|(format!("{phase:?}"),count)).collect::<BTreeMap<_,_>>(),"readers":readers,"unfinished_calls":unfinished,
            "unbalanced_pairs":self.unbalanced_pairs,"publication_or_semantic_qualification":"not inferred"})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event<'a>(phase:Phase,owner:&'a str,field:Option<&'a str>)->Event<'a> {
        Event{phase,owner_id:owner,owner_kind:None,field,element_count:1,succeeded:None}
    }
    #[test]
    fn nested_costs_separate_child_work_and_unfinished_calls() {
        let mut profile=QueryProfile::default();
        profile.observe(event(Phase::GeneralTypesStarted,"T",None),0);
        profile.observe(event(Phase::ContainmentReadStarted,"T",Some("owned_relationship")),10);
        profile.observe(event(Phase::ContainmentReadFinished,"T",Some("owned_relationship")),30);
        let open=profile.snapshot(50);
        let readers=open["readers"].as_array().unwrap();
        let general=readers.iter().find(|r|r["phase"]=="GeneralTypesStarted").unwrap();
        assert_eq!(general["open_calls"],1);assert_eq!(general["exclusive_ms"],json!(30.0/1e6));
        profile.observe(event(Phase::GeneralTypesFinished,"T",None),60);
        let closed=profile.snapshot(60);
        assert!(closed["unfinished_calls"].as_array().unwrap().is_empty());
        let general=closed["readers"].as_array().unwrap().iter().find(|r|r["phase"]=="GeneralTypesStarted").unwrap();
        assert_eq!(general["completed_calls"],1);assert_eq!(general["inclusive_ms"],json!(60.0/1e6));
        assert_eq!(general["exclusive_ms"],json!(40.0/1e6));assert_eq!(closed["unbalanced_pairs"],0);
    }
    #[test]
    fn attempts_and_failed_queries_do_not_count_as_field_writes() {
        let mut profile=QueryProfile::default();
        for phase in [Phase::LinkFieldStarted,Phase::ReferenceStarted,Phase::ReferenceFinished,Phase::LinkFieldStarted] {
            profile.observe(event(phase,"typing",Some("type")),10);
        }
        assert_eq!(profile.snapshot(20)["staged_reference_fields_committed"],0);
        assert_eq!(profile.snapshot(20)["active_field"]["owner_id"],"typing");
        profile.observe(event(Phase::LinkFieldCommitted,"typing",Some("type")),30);
        assert_eq!(profile.snapshot(30)["staged_reference_fields_committed"],1);
        assert!(profile.snapshot(30)["active_field"].is_null());
        assert_eq!(profile.snapshot(30)["publication_or_semantic_qualification"],"not inferred");
    }
    #[test]
    fn mismatched_trace_pairs_are_reported_without_silent_cost_attribution() {
        let mut profile=QueryProfile::default();
        profile.observe(event(Phase::GeneralTypesStarted,"T",None),0);
        profile.observe(event(Phase::GeneralTypesFinished,"different",None),10);
        let row=profile.snapshot(20);assert_eq!(row["unbalanced_pairs"],1);
        assert_eq!(row["unfinished_calls"].as_array().unwrap().len(),1);
        assert_eq!(row["readers"][0]["completed_calls"],0);
    }
}
