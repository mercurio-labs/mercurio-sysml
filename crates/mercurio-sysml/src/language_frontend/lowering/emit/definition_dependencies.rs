//! Typed requirements carried by the bounded Feature query slice.
//! Only the document scheduler can authorize retry against its pending registry.
use super::Diagnostic;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Prerequisite {
    ReadField { owner_id: String, field: String },
    BinaryCrossing { owner_id: String },
    CrossSpecialization { owner_id: String },
    AdditionalMembers { owner_id: String },
    ReferenceResultSubsetting { owner_id: String },
    ChainSpecialization { owner_id: String },
    ExpressionContributions { owner_id: String },
    ExpressionContributionBatch { owner_ids: Vec<String> },
    ReferenceBindingBatch { owner_ids: Vec<String> },
    GeneralValueBindingBatch { owner_ids: Vec<String> },
    ArgumentResultSpecializationBatch { owner_ids: Vec<String> },
    ChainLifecycleBatch { owner_ids: Vec<String> },
    OwningTypeFeaturingBatch { owner_ids: Vec<String> },
    ExpressionFeaturingBatch { owner_ids: Vec<String> },
    MultiplicityFeaturing { owner_id: String },
}
#[derive(Debug)]
pub(super) enum QueryFailure {
    Required(Prerequisite),
    Rejected(Diagnostic),
}
impl QueryFailure {
    pub(super) fn into_diagnostic(self) -> Diagnostic {
        self.into_diagnostic_at("unspecified external boundary")
    }
    // Static call-site provenance makes an accidental internal conversion
    // reviewable. It never recovers or schedules requirements from error text.
    pub(super) fn into_diagnostic_at(self, boundary: &str) -> Diagnostic {
        match self {
            Self::Rejected(diagnostic) => diagnostic,
            Self::Required(prerequisite) => Diagnostic::new(format!("Operand construction: unresolved query prerequisite {prerequisite:?} [native boundary: {boundary}]"), None),
        }
    }
}
impl From<Diagnostic> for QueryFailure { fn from(diagnostic: Diagnostic) -> Self { Self::Rejected(diagnostic) } }

/// Owned typed query failure for diagnostic tools. A requirement is neither a
/// successful semantic answer nor permission to resolve an arbitrary field.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DefinitionReferenceQueryError {
    Required { prerequisite: Prerequisite },
    Rejected { message: String, subjects: Vec<String> },
}
impl std::fmt::Display for DefinitionReferenceQueryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Required { prerequisite } => write!(formatter, "Unresolved native query prerequisite: {prerequisite:?}"),
            Self::Rejected { message, .. } => formatter.write_str(message),
        }
    }
}
impl std::error::Error for DefinitionReferenceQueryError {}
impl From<QueryFailure> for DefinitionReferenceQueryError {
    fn from(failure: QueryFailure) -> Self {
        match failure {
            QueryFailure::Required(prerequisite) => Self::Required { prerequisite },
            QueryFailure::Rejected(diagnostic) => Self::Rejected { message: diagnostic.message, subjects: diagnostic.subjects },
        }
    }
}

// Handwritten read-before-write planning over the existing typed query ports.
// A pending wave is not a completed plan or the whole transitive dependency
// closure. Only Ready may authorize the consumer's checked construction stages.
#[derive(Debug)]
pub(super) enum TransformationPlan<T> {
    Ready(T),
    Required(Vec<Prerequisite>),
}
impl<T> TransformationPlan<T> {
    pub(super) fn into_query_result(self) -> Result<T, QueryFailure> {
        match self {
            Self::Ready(value) => Ok(value),
            Self::Required(requirements) => requirements.into_iter().next()
                .map(|requirement| Err(QueryFailure::Required(requirement)))
                .unwrap_or_else(|| Err(QueryFailure::Rejected(Diagnostic::new(
                    "Operand construction: empty pending transformation plan", None)))),
        }
    }
}

#[derive(Default)]
pub(super) struct TransformationReads {
    requirements: Vec<Prerequisite>,
    rejection: Option<Diagnostic>,
}
impl TransformationReads {
    pub(super) fn capture<T>(&mut self, result: Result<T, QueryFailure>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(QueryFailure::Required(requirement)) => {
                if !self.requirements.contains(&requirement) {
                    self.requirements.push(requirement);
                }
                None
            }
            Err(QueryFailure::Rejected(diagnostic)) => {
                if self.rejection.is_none() { self.rejection = Some(diagnostic); }
                None
            }
        }
    }
    pub(super) fn capture_plan<T>(&mut self,
        result: Result<TransformationPlan<T>, QueryFailure>) -> Option<T> {
        match self.capture(result) {
            Some(TransformationPlan::Ready(value)) => Some(value),
            Some(TransformationPlan::Required(requirements)) => {
                if requirements.is_empty() {
                    self.capture::<()>(Err(QueryFailure::Rejected(Diagnostic::new(
                        "Operand construction: empty pending transformation plan", None))));
                }
                for requirement in requirements {
                    self.capture::<()>(Err(QueryFailure::Required(requirement)));
                }
                None
            }
            None => None,
        }
    }
    pub(super) fn finish<T>(self, ready: impl FnOnce() -> Result<T, QueryFailure>)
        -> Result<TransformationPlan<T>, QueryFailure> {
        // A known malformed/unsupported dependency rejects the entire plan,
        // even when another port is pending. It cannot schedule partial writes.
        if let Some(diagnostic) = self.rejection {
            return Err(QueryFailure::Rejected(diagnostic));
        }
        if !self.requirements.is_empty() {
            return Ok(TransformationPlan::Required(self.requirements));
        }
        ready().map(TransformationPlan::Ready)
    }
}
