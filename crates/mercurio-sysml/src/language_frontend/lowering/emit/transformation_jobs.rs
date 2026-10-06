//! Handwritten job scheduling, distinct from imported semantic definitions.
//! One typed active path covers reads and the registered construction and specialization stages.
//! Queued roots are inactive. Completion belongs to this private transaction;
//! it never writes provider lifecycle flags or changes constructor admission.
use super::{error, Diagnostic, Prerequisite};
use std::collections::{BTreeSet, VecDeque};

#[derive(Default)]
pub(super) struct Jobs {
    active: Vec<Prerequisite>,
    completed: BTreeSet<Prerequisite>,
    roots: VecDeque<Prerequisite>,
}
fn label(job: &Prerequisite) -> &'static str {
    match job {
        Prerequisite::ReadField { .. } => "generated link",
        Prerequisite::AdditionalMembers { .. } => "additional-member construction",
        Prerequisite::BinaryCrossing { .. } => "crossing construction",
        Prerequisite::CrossSpecialization { .. } => "cross specialization construction",
        Prerequisite::ReferenceResultSubsetting { .. } => "reference-result specialization",
        Prerequisite::ChainSpecialization { .. } => "chain specialization",
        Prerequisite::ExpressionContributions { .. } => "expression contributions",
        Prerequisite::ExpressionContributionBatch { .. } => "expression contribution batch",
        Prerequisite::ReferenceBindingBatch { .. } => "reference binding batch",
        Prerequisite::GeneralValueBindingBatch { .. } => "general value binding batch",
        Prerequisite::ArgumentResultSpecializationBatch { .. } => "argument-result specialization batch",
        Prerequisite::ChainLifecycleBatch { .. } => "canonical chain lifecycle batch",
        Prerequisite::OwningTypeFeaturingBatch { .. } => "fixed owning-Type featuring batch",
        Prerequisite::ExpressionFeaturingBatch { .. } => "expression featuring batch",
        Prerequisite::MultiplicityFeaturing { .. } => "multiplicity featuring",
    }
}
impl Jobs {
    pub(super) fn has_work(&self) -> bool { !self.active.is_empty() || !self.roots.is_empty() }
    pub(super) fn current(&self) -> Option<&Prerequisite> { self.active.last() }
    fn assess(&self, job: &Prerequisite,
        validate: &impl Fn(&Prerequisite) -> Result<(), Diagnostic>) -> Result<(), Diagnostic> {
        validate(job)?;
        if self.active.contains(job) {
            return Err(error(format!("unresolved {} dependency cycle: {:?} -> {job:?}", label(job), self.active)));
        }
        if self.completed.contains(job) {
            return Err(error(format!("repeated {} dependency: {job:?}", label(job))));
        }
        Ok(())
    }
    pub(super) fn require(&mut self, job: Prerequisite,
        validate: &impl Fn(&Prerequisite) -> Result<(), Diagnostic>) -> Result<(), Diagnostic> {
        self.assess(&job, validate)?;
        self.active.push(job);
        Ok(())
    }
    pub(super) fn queue_wave(&mut self, jobs: Vec<Prerequisite>,
        validate: &impl Fn(&Prerequisite) -> Result<(), Diagnostic>) -> Result<(), Diagnostic> {
        if jobs.is_empty() { return Err(error("empty pending transformation plan")); }
        // No work is queued until every typed dependency has passed admission.
        for job in &jobs { self.assess(job, validate)?; }
        for job in jobs {
            if !self.roots.contains(&job) { self.roots.push_back(job); }
        }
        Ok(())
    }
    pub(super) fn begin_root(&mut self,
        still_pending: &impl Fn(&Prerequisite) -> bool) -> bool {
        if !self.active.is_empty() { return true; }
        while let Some(job) = self.roots.pop_front() {
            // A queued root may already have completed as a nested dependency.
            if self.completed.contains(&job) || !still_pending(&job) { continue; }
            self.active.push(job);
            return true;
        }
        false
    }
    pub(super) fn complete(&mut self) -> Result<(), Diagnostic> {
        let job = self.active.pop().ok_or_else(|| error("missing active transformation job"))?;
        if !self.completed.insert(job) { return Err(error("duplicate transformation job completion")); }
        Ok(())
    }
    pub(super) fn read_path(&self) -> Vec<(String, String)> {
        self.active.iter().filter_map(|job| match job {
            Prerequisite::ReadField {owner_id, field} => Some((owner_id.clone(), field.clone())),
            _ => None,
        }).collect()
    }
}
