//! Opt-in candidate document construction from the pinned 2026-08 definitions.
//!
//! The shared Xtext interpreter consumes complete source, constructs canonical
//! Ecore storage, and resolves supported closed namespace references atomically.
//! A returned document has checked stored structure and local references. It is
//! not a declaration of full language conformance: implicit relationships,
//! expression evaluation and remaining validation/delegate algorithms are not
//! computed here. Unsupported services return errors; there is no parser fallback.
//! The existing authoring/compiler APIs retain their current behavior.

use crate::{KirDocument, SourceLanguage};
use mercurio_foundation::kir::KIR_SCHEMA_VERSION;
use serde_json::json;
use std::fmt;

/// Failure stages remain distinct: resource or implementation limits must not be
/// reported as evidence that the user's source is grammatically invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionDocumentError {
    Syntax(String),
    Lexical(String),
    Unsupported(String),
    ResourceLimit(String),
    Artifact(String),
    ConstructionOrLinking(String),
}

impl fmt::Display for DefinitionDocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (stage, message) = match self {
            Self::Syntax(m) => ("syntax", m),
            Self::Lexical(m) => ("lexical recognition", m),
            Self::Unsupported(m) => ("unsupported execution", m),
            Self::ResourceLimit(m) => ("resource limit", m),
            Self::Artifact(m) => ("definition artifact", m),
            Self::ConstructionOrLinking(m) => ("construction, linking or publication", m),
        };
        write!(f, "Candidate definition document {stage}: {message}")
    }
}
impl std::error::Error for DefinitionDocumentError {}

/// Parse and link a self-contained document using the pinned definition-driven
/// candidate. Scope currently covers canonical namespace/package membership,
/// aliases, imports, direct owned type members, typing, ordinary specialization
/// and conjugation references. Plain KerML Type/Classifier/Class/DataType/Structure
/// inheritance computes imported defaults through declared standard-library roots.
/// Resolved Definition default programs additionally select individual-life and
/// binary-end defaults. Missing libraries and Feature redefinition dependencies
/// reject. Ordinary Feature redefinitions and unnamed Multiplicity naming use
/// the shared query; positional redefinitions retain explicit dependency errors.
/// Bounded inherited-first redefinitions are supported; external resources remain unsupported.
///
/// Stored containment, defaults, shapes, local endpoint types, uniqueness and
/// reciprocal ownership are checked before publication. Missing required or
/// derived semantics remain unassessed; metadata records this boundary.
/// Production execution uses Rust and embedded artifacts, with no runtime Java.
pub fn parse_and_link(source: &str, language: SourceLanguage) -> Result<KirDocument, DefinitionDocumentError> {
    let kerml = language == SourceLanguage::Kerml;
    let tree = parse_definition_tree(source, language)?;
    let elements = crate::language_frontend::lowering::emit::materialize_definition_document(&tree, kerml)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))?;
    Ok(KirDocument {
        metadata: [
            ("kir_schema_version".into(), json!(KIR_SCHEMA_VERSION)),
            ("source".into(), json!(if kerml { "kerml" } else { "sysml" })),
            ("definition_profile".into(), json!("sysml-2.0-pilot-2026-08")),
            ("definition_pipeline".into(), json!("candidate")),
            ("semantic_validation".into(), json!("not_assessed")),
        ].into_iter().collect(),
        elements,
    })
}

fn parse_definition_tree(source: &str, language: SourceLanguage)
    -> Result<crate::xtext_fragment::Match, DefinitionDocumentError> {
    use crate::xtext_fragment::CandidateDocumentError as ParseError;
    let kerml = language == SourceLanguage::Kerml;
    crate::xtext_fragment::candidate_document(source, kerml).map_err(|error| match error {
        ParseError::Syntax(m) => DefinitionDocumentError::Syntax(m),
        ParseError::Lexical(m) => DefinitionDocumentError::Lexical(format!("{m:?}")),
        ParseError::Unsupported(m) => DefinitionDocumentError::Unsupported(m),
        ParseError::ResourceLimit(m) => DefinitionDocumentError::ResourceLimit(m),
        ParseError::Artifact(m) => DefinitionDocumentError::Artifact(m),
    })
}

/// One separately parsed source in an explicitly supplied resource environment.
/// The URI is an opaque stable identity, not a request to read from disk or network.
pub struct DefinitionSource<'a> {
    pub uri: &'a str,
    pub text: &'a str,
    pub language: SourceLanguage,
}

/// Parse each resource using its pinned grammar, construct all roots, then link
/// against public exports in one transaction. Caller-supplied resource closure
/// replaces loading; no filesystem, network or Java dependency is introduced.
/// Identities are stable under resource reordering, not edits within a resource.
/// Unsupported globals/delegates and full semantic validation remain explicit.
pub fn parse_and_link_sources(sources: &[DefinitionSource<'_>])
    -> Result<KirDocument, DefinitionDocumentError> {
    parse_and_link_sources_observed(sources, |_, _| {})
}

/// Diagnostic progress only; completion of these stages does not qualify
/// contributions or semantic validation. The callback performs no kernel I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionSourcePhase { Parsing, Parsed, ConstructionAndLinking, Constructed }

/// Same atomic candidate pipeline with an optional caller-owned progress sink.
/// A failed stage emits no success event and publishes no partial document.
pub fn parse_and_link_sources_observed(sources: &[DefinitionSource<'_>],
    observer: impl FnMut(Option<&str>, DefinitionSourcePhase))
    -> Result<KirDocument, DefinitionDocumentError> {
    parse_and_link_sources_traced(sources, observer, |_| {})
}

pub use crate::language_frontend::lowering::emit::{DefinitionConstructionEvent, DefinitionConstructionPhase};

/// Atomic source pipeline with caller-owned construction/query observation.
/// Events provide identities and boundaries only; tooling owns timing and I/O.
/// Tracing neither changes admission nor qualifies transformation/validation.
pub fn parse_and_link_sources_traced(sources: &[DefinitionSource<'_>],
    observer: impl FnMut(Option<&str>, DefinitionSourcePhase),
    construction_observer: impl FnMut(DefinitionConstructionEvent<'_>))
    -> Result<KirDocument, DefinitionDocumentError> {
    parse_and_link_sources_traced_with_literal_bindings(sources,observer,construction_observer,&[])
}

/// Opt-in shared literal/null value transformation within the existing atomic
/// definition-driven source pipeline. Every literal in the explicitly selected
/// resource URIs must satisfy the fixed, explicitly typed FeatureValue contract.
/// Other literal contexts, initial/variable values and unsupported dependencies
/// reject; full model lifecycle and semantic validation remain unassessed.
/// Supplied library/prototype/result/owner flags are never marked complete.
/// Final publication still requires Closed Ecore reference integrity.
pub fn parse_and_link_sources_with_literal_value_bindings(sources: &[DefinitionSource<'_>],
    literal_binding_source_uris: &[&str]) -> Result<KirDocument, DefinitionDocumentError> {
    parse_and_link_sources_with_literal_value_bindings_traced(sources,literal_binding_source_uris,|_,_|{},|_|{})
}

/// The same selected native producer stage with caller-owned progress sinks.
/// Observations report work only; they never qualify full semantic validation.
pub fn parse_and_link_sources_with_literal_value_bindings_traced(sources: &[DefinitionSource<'_>],
    literal_binding_source_uris: &[&str],
    observer: impl FnMut(Option<&str>, DefinitionSourcePhase),
    construction_observer: impl FnMut(DefinitionConstructionEvent<'_>)) -> Result<KirDocument, DefinitionDocumentError> {
    let mut document=parse_and_link_sources_traced_with_literal_bindings(sources,observer,construction_observer,literal_binding_source_uris)?;
    document.metadata.insert("literal_value_binding_sources".into(),json!(literal_binding_source_uris));
    document.metadata.insert("literal_value_binding_stage".into(),json!("selected_literal_receivers_and_bindings_completed"));
    Ok(document)
}

fn parse_and_link_sources_traced_with_literal_bindings(sources: &[DefinitionSource<'_>],
    mut observer: impl FnMut(Option<&str>, DefinitionSourcePhase),
    construction_observer: impl FnMut(DefinitionConstructionEvent<'_>),
    literal_binding_source_uris: &[&str])
    -> Result<KirDocument, DefinitionDocumentError> {
    if literal_binding_source_uris.iter().collect::<std::collections::BTreeSet<_>>().len()!=literal_binding_source_uris.len()
        || literal_binding_source_uris.iter().any(|uri|!sources.iter().any(|source|source.uri==*uri)) {
        return Err(DefinitionDocumentError::Unsupported("Literal binding source selection must contain distinct supplied URIs".into()));
    }
    let construction_observer = std::cell::RefCell::new(construction_observer);
    let (roots, trees) = parse_definition_sources(sources, &mut observer)?;
    let resources = roots.iter().zip(&trees).zip(sources).map(|((id,tree),source)|
        (id.as_str(), tree, source.language == SourceLanguage::Kerml)).collect::<Vec<_>>();
    let literal_roots=sources.iter().zip(&roots).filter(|(source,_)|literal_binding_source_uris.contains(&source.uri)).map(|(_,root)|root.as_str()).collect::<Vec<_>>();
    observer(None, DefinitionSourcePhase::ConstructionAndLinking);
    let elements = crate::language_frontend::lowering::emit::materialize_definition_resources_with_literal_bindings_traced(&resources, &|event| construction_observer.borrow_mut()(event), &literal_roots)
        .map_err(|e| DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))?;
    observer(None, DefinitionSourcePhase::Constructed);
    Ok(KirDocument {
        metadata: [
            ("kir_schema_version".into(), json!(KIR_SCHEMA_VERSION)),
            ("source".into(), json!("source_set")),
            ("definition_profile".into(), json!("sysml-2.0-pilot-2026-08")),
            ("definition_pipeline".into(), json!("candidate")),
            ("semantic_validation".into(), json!("not_assessed")),
            ("source_resources".into(), json!(sources.iter().zip(&roots).map(|(source,root)|
                json!({"uri":source.uri,"root_id":root,"language":if source.language==SourceLanguage::Kerml {"kerml"} else {"sysml"}})).collect::<Vec<_>>())),
        ].into_iter().collect(),
        elements,
    })
}

/// Construction and explicit dependency-plan diagnostics. This type is not a
/// KirDocument and has no conversion or publication API. Unexecuted references
/// remain explicit; complete semantics and transformation are not assessed.
#[derive(Debug, serde::Serialize)]
pub struct DefinitionStructureInspection {
    schema: &'static str,
    status: &'static str,
    linking: &'static str,
    semantic_validation: &'static str,
    transformation_completion: &'static str,
    #[serde(rename = "constructed_elements")]
    elements: Vec<mercurio_foundation::kir::KirElement>,
    pending_references: Vec<DefinitionPendingReference>,
}
impl DefinitionStructureInspection {
    /// Typed diagnostic query. Missing dependencies retain their exact owner,
    /// storage field or native producer instead of being parsed from error text.
    /// This method neither resolves pending references nor permits publication.
    pub fn reference_targets_with_dependencies(&self, owner_id: &str, field: &str)
        -> Result<Vec<&mercurio_foundation::kir::KirElement>, DefinitionReferenceQueryError> {
        let owner = self.elements.iter().find(|e| e.id == owner_id)
            .ok_or_else(|| DefinitionReferenceQueryError::Rejected { message: "Inspection query owner is absent".into(), subjects: vec![owner_id.into()] })?;
        crate::language_frontend::lowering::emit::definition_reference_targets_with_dependencies(&self.elements, owner, field)
    }
    pub fn elements(&self) -> &[mercurio_foundation::kir::KirElement] { &self.elements }
    pub fn pending_references(&self) -> &[DefinitionPendingReference] { &self.pending_references }
    /// Execute the same read-only native getter as the candidate. A supported
    /// local result does not resolve other pending fields or qualify the graph.
    /// Missing transitive dependencies remain errors; no snapshot is substituted.
    pub fn reference_targets(&self, owner_id: &str, field: &str)
        -> Result<Vec<&mercurio_foundation::kir::KirElement>, DefinitionDocumentError> {
        let owner = self.elements.iter().find(|e| e.id == owner_id)
            .ok_or_else(|| DefinitionDocumentError::ConstructionOrLinking("Inspection query owner is absent".into()))?;
        crate::language_frontend::lowering::emit::definition_reference_targets(&self.elements, owner, field)
            .map_err(|e| DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
    }
}
pub use crate::language_frontend::lowering::emit::{DefinitionPendingReference, DefinitionQueryPrerequisite, DefinitionReferenceQueryError};

/// Query constructed native records without converting them to a KirDocument.
/// A batch shares the same immutable graph index and semantic query view. Its
/// ordered results preserve each request's exact typed prerequisite or rejection;
/// a failed request does not replace its answer or stop independent requests.
/// This is diagnostic consumption, never publication, transformation completion
/// or proof that serialized records came from a qualified language model.
pub fn query_constructed_references<'g>(
    constructed_elements: &'g [mercurio_foundation::kir::KirElement],
    requests: &[(&str, &str)],
) -> Result<Vec<Result<Vec<&'g mercurio_foundation::kir::KirElement>, DefinitionReferenceQueryError>>, DefinitionReferenceQueryError> {
    crate::language_frontend::lowering::emit::definition_reference_target_batch_with_dependencies(
        constructed_elements, requests)
}



/// Diagnostic execution of the imported Type.directionOf operation. A null
/// direction is a semantic result; an unfinished read remains a typed dependency.
/// No model publication, transformation flag or complete-context credit is added.
pub fn query_constructed_feature_direction(
    constructed_elements: &[mercurio_foundation::kir::KirElement], owner_id: &str, feature_id: &str,
) -> Result<serde_json::Value, DefinitionReferenceQueryError> {
    crate::language_frontend::lowering::emit::definition_direction_of_with_dependencies(
        constructed_elements, owner_id, feature_id)
}

/// Inspect complete declaration bodies through the same native parser, Ecore
/// construction and canonical endpoint postprocessing used by the candidate.
/// This makes missing dependencies inspectable without running or bypassing the
/// linker. A successful inspection is not successful compilation/publication.
pub fn inspect_source_structure(sources: &[DefinitionSource<'_>])
    -> Result<DefinitionStructureInspection, DefinitionDocumentError> {
    inspect_source_structure_traced(sources, |_, _| {}, |_| {})
}

/// Construction-only observability; it never runs a link/publication stage.
/// The caller owns reporting, time measurements and all external I/O.
pub fn inspect_source_structure_traced(sources: &[DefinitionSource<'_>],
    mut source_observer: impl FnMut(Option<&str>, DefinitionSourcePhase),
    construction_observer: impl FnMut(DefinitionConstructionEvent<'_>))
    -> Result<DefinitionStructureInspection, DefinitionDocumentError> {
    let construction_observer = std::cell::RefCell::new(construction_observer);
    let (roots, trees) = parse_definition_sources(sources, &mut source_observer)?;
    let resources = roots.iter().zip(&trees).zip(sources).map(|((id, tree), source)|
        (id.as_str(), tree, source.language == SourceLanguage::Kerml)).collect::<Vec<_>>();
    let (elements, pending_references) = crate::language_frontend::lowering::emit::inspect_definition_resources_traced(&resources,
        &|event| construction_observer.borrow_mut()(event))
        .map_err(|e| DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))?;
    Ok(DefinitionStructureInspection {
        schema: "dev.mercurio.definition-structure-inspection.v1", status: "unlinked_inspection",
        linking: "not_run", semantic_validation: "not_assessed", transformation_completion: "not_assessed",
        elements, pending_references,
    })
}


/// Run an explicit typed requirement wave with the same native source parser,
/// materializer, scope/linking consumers and scheduler as the candidate.
/// Semantic dependencies may add registered reads or supported constructions.
/// The full graph and remaining work are retained; no model is published.
pub fn inspect_source_dependencies(sources: &[DefinitionSource<'_>],
    requirements: &[DefinitionQueryPrerequisite])
    -> Result<DefinitionStructureInspection, DefinitionDocumentError> {
    inspect_source_dependencies_traced(sources, requirements, |_, _| {}, |_| {})
}

/// Caller-owned observation for explicit native dependency execution. Success
/// means the requested wave completed, never semantic or release qualification.
pub fn inspect_source_dependencies_traced(sources: &[DefinitionSource<'_>],
    requirements: &[DefinitionQueryPrerequisite],
    mut source_observer: impl FnMut(Option<&str>, DefinitionSourcePhase),
    construction_observer: impl FnMut(DefinitionConstructionEvent<'_>))
    -> Result<DefinitionStructureInspection, DefinitionDocumentError> {
    let construction_observer = std::cell::RefCell::new(construction_observer);
    let (roots, trees) = parse_definition_sources(sources, &mut source_observer)?;
    let resources = roots.iter().zip(&trees).zip(sources).map(|((id, tree), source)|
        (id.as_str(), tree, source.language == SourceLanguage::Kerml)).collect::<Vec<_>>();
    source_observer(None, DefinitionSourcePhase::ConstructionAndLinking);
    let (elements, pending_references) =
        crate::language_frontend::lowering::emit::inspect_definition_resources_with_plan_traced(
            &resources, requirements, &|event| construction_observer.borrow_mut()(event))
        .map_err(|e| DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))?;
    source_observer(None, DefinitionSourcePhase::Constructed);
    Ok(DefinitionStructureInspection {
        schema: "dev.mercurio.definition-structure-inspection.v1",
        status: "dependency_inspection", linking: "requested_plan_completed",
        semantic_validation: "not_assessed", transformation_completion: "not_assessed",
        elements, pending_references,
    })
}

/// Source identity and language for a retained native constructor resource.
/// The URI is opaque and must be identical to the URI used during construction.
#[derive(Debug, Clone, Copy)]
pub struct DefinitionConstructedResource<'a> { pub uri: &'a str, pub language: SourceLanguage }

/// Execute an explicit dependency wave on retained native constructor records.
/// Uses the same native resolver and scheduler as inspect_source_dependencies.
/// Inputs are borrowed and remain unchanged on failure. Success does not assess
/// semantic validation, complete transformation, publication or qualification.
pub fn inspect_constructed_dependencies(
    resources: &[DefinitionConstructedResource<'_>],
    elements: &[mercurio_foundation::kir::KirElement],
    pending_references: &[DefinitionPendingReference],
    requirements: &[DefinitionQueryPrerequisite],
) -> Result<DefinitionStructureInspection, DefinitionDocumentError> {
    inspect_constructed_dependencies_traced(resources, elements, pending_references, requirements, |_| {})
}

pub fn inspect_constructed_dependencies_traced(
    resources: &[DefinitionConstructedResource<'_>],
    elements: &[mercurio_foundation::kir::KirElement],
    pending_references: &[DefinitionPendingReference],
    requirements: &[DefinitionQueryPrerequisite],
    construction_observer: impl FnMut(DefinitionConstructionEvent<'_>),
) -> Result<DefinitionStructureInspection, DefinitionDocumentError> {
    let mut seen = std::collections::BTreeSet::new();
    let roots = resources.iter().map(|source| {
        if source.uri.trim().is_empty() || source.uri.contains('\0') || !seen.insert(source.uri) {
            return Err(DefinitionDocumentError::Artifact("Source identities must be nonempty and unique".into()));
        }
        let encoded = source.uri.bytes().map(|b| format!("{b:02x}")).collect::<String>();
        Ok((format!("definition.resource.{encoded}"), source.language == SourceLanguage::Kerml))
    }).collect::<Result<Vec<_>, DefinitionDocumentError>>()?;
    let resource_contexts = roots.iter().map(|(id, kerml)| (id.as_str(), *kerml)).collect::<Vec<_>>();
    let observer = std::cell::RefCell::new(construction_observer);
    let (elements, pending_references) = crate::language_frontend::lowering::emit::inspect_constructed_resources_with_plan_traced(
        &resource_contexts, elements.to_vec(), pending_references, requirements, &|event| observer.borrow_mut()(event))
        .map_err(|e| DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))?;
    Ok(DefinitionStructureInspection {
        schema: "dev.mercurio.definition-structure-inspection.v1",
        status: "dependency_inspection", linking: "requested_plan_completed",
        semantic_validation: "not_assessed", transformation_completion: "not_assessed",
        elements, pending_references,
    })
}

fn parse_definition_sources(sources: &[DefinitionSource<'_>],
    observer: &mut impl FnMut(Option<&str>, DefinitionSourcePhase))
    -> Result<(Vec<String>, Vec<crate::xtext_fragment::Match>), DefinitionDocumentError> {
    let mut seen = std::collections::BTreeSet::new();
    if sources.is_empty() { return Err(DefinitionDocumentError::Artifact("Empty source set".into())); }
    let mut roots = Vec::new();
    let mut trees = Vec::new();
    for source in sources {
        if source.uri.trim().is_empty() || source.uri.contains('\0') || !seen.insert(source.uri) {
            return Err(DefinitionDocumentError::Artifact("Source identities must be nonempty and unique".into()));
        }
        let encoded = source.uri.bytes().map(|b| format!("{b:02x}")).collect::<String>();
        roots.push(format!("definition.resource.{encoded}"));
        observer(Some(source.uri), DefinitionSourcePhase::Parsing);
        trees.push(parse_definition_tree(source.text, source.language).map_err(|e| {
            let context = |m: String| format!("{}: {m}", source.uri);
            match e {
                DefinitionDocumentError::Syntax(m) => DefinitionDocumentError::Syntax(context(m)),
                DefinitionDocumentError::Lexical(m) => DefinitionDocumentError::Lexical(context(m)),
                DefinitionDocumentError::Unsupported(m) => DefinitionDocumentError::Unsupported(context(m)),
                DefinitionDocumentError::ResourceLimit(m) => DefinitionDocumentError::ResourceLimit(context(m)),
                DefinitionDocumentError::Artifact(m) => DefinitionDocumentError::Artifact(context(m)),
                DefinitionDocumentError::ConstructionOrLinking(m) => DefinitionDocumentError::ConstructionOrLinking(context(m)),
            }
        })?);
        observer(Some(source.uri), DefinitionSourcePhase::Parsed);
    }
    Ok((roots, trees))
}

/// One required Ecore value that is invalid or still needs a semantic dependency.
/// This assessment covers multiplicity/value/endpoint contracts, not all language
/// validation constraints or transformation completion.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RequiredFeatureIssue {
    pub element_id: String,
    pub field: String,
    pub feature_id: String,
    pub message: String,
    pub unverified: bool,
}

/// Assess required values using imported Ecore contracts and the shared native
/// reference consumers. Derived snapshots are never used as implementation proof.
/// Handwritten delegate algorithms retain their own bounded context checks;
/// unsupported algorithms, remaining attributes and failed prerequisites stay unverified.
/// The graph and its semantic-validation metadata are unchanged.
pub fn assess_required_features(document: &KirDocument)
    -> Result<Vec<RequiredFeatureIssue>, DefinitionDocumentError>
{
    use crate::language_frontend::lowering::{ecore_model, emit};
    if let Some(issue) = ecore_model::validate_publication(&document.elements,
        ecore_model::ReferenceCompleteness::Closed).first() {
        return Err(DefinitionDocumentError::ConstructionOrLinking(format!(
            "Required-value assessment requires a canonical closed graph: {}.{}: {}",
            issue.element_id, issue.field, issue.message)));
    }
    let mut origins = emit::DefinitionLibraryOriginContext::new(&document.elements)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))?;
    Ok(ecore_model::assess_required_features_with(&document.elements, |owner, contract| {
        if contract.kind != ecore_model::FeatureKind::Reference {
            return emit::definition_attribute_value_with_origins(&document.elements, owner, contract.field, Some(&mut origins))
                .map_err(|error| format!("Required Ecore feature {}: {error:?}", contract.id));
        }
        let targets = emit::definition_reference_targets(&document.elements, owner, contract.field)
            .map_err(|error| format!("Required Ecore feature {}: {error:?}", contract.id))?;
        // Convert the consumer result to the requested Ecore shape. Do not
        // truncate singular results or deduplicate ordered/unique values.
        if contract.upper == 1 {
            match targets.as_slice() {
                [] => Ok(serde_json::Value::Null),
                [target] => Ok(json!(target.id)),
                _ => Ok(json!(targets.iter().map(|target| &target.id).collect::<Vec<_>>())),
            }
        } else {
            Ok(json!(targets.iter().map(|target| &target.id).collect::<Vec<_>>()))
        }
    }).into_iter().map(|issue| RequiredFeatureIssue {
        element_id: issue.element_id, field: issue.field, feature_id: issue.feature_id,
        message: issue.message, unverified: issue.unverified,
    }).collect())
}

/// Read an Ecore attribute through its imported redefinition and bounded native
/// consumer. Remaining derived algorithms return explicit dependency errors.
pub fn attribute_value(document: &KirDocument, owner_id: &str, field: &str)
    -> Result<serde_json::Value, DefinitionDocumentError>
{
    let owner = document.elements.iter().find(|e| e.id == owner_id)
        .ok_or_else(|| DefinitionDocumentError::ConstructionOrLinking("Missing attribute owner".into()))?;
    crate::language_frontend::lowering::emit::definition_attribute_value(&document.elements, owner, field)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Return the nearest owning library namespace through the pinned invocation
/// family. This bounded operation uses native reciprocal ownership, not imports
/// or the namespace of an externally referenced target.
pub fn library_namespace<'a>(document: &'a KirDocument, owner_id: &str)
    -> Result<Option<&'a mercurio_foundation::kir::KirElement>, DefinitionDocumentError>
{
    let owner = document.elements.iter().find(|e| e.id == owner_id)
        .ok_or_else(|| DefinitionDocumentError::ConstructionOrLinking("Missing libraryNamespace owner".into()))?;
    crate::language_frontend::lowering::emit::definition_library_namespace(&document.elements, owner)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// A violation of the pinned Element implied-inclusion invariant.
/// This is one assessed constraint, not a full validation/completion certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpliedInclusionViolation {
    pub element_id: String,
    pub implied_relationship_ids: Vec<String>,
}

/// Assess `validateElementIsImpliedIncluded` across the entire canonical graph.
/// Handwritten execution of the written KerML invariant uses imported Ecore
/// ownership and Boolean defaults. Pilot's bounded Xtend predicate is an
/// independent oracle. Structural errors remain errors, not empty issue lists.
/// A passing result does not prove that all necessary relationships were
/// generated: a claimed true completion flag requires separate lifecycle proof.
/// No graph or completion flag is changed by this assessment.
pub fn assess_implied_inclusion(document: &KirDocument)
    -> Result<Vec<ImpliedInclusionViolation>, DefinitionDocumentError>
{
    use crate::language_frontend::lowering::{ecore_defaults, ecore_model};
    if let Some(issue) = ecore_model::validate_publication(&document.elements,
        ecore_model::ReferenceCompleteness::Closed).first() {
        return Err(DefinitionDocumentError::ConstructionOrLinking(format!(
            "Implied inclusion requires a canonical closed graph: {}.{}: {}",
            issue.element_id, issue.field, issue.message)));
    }
    let index = document.elements.iter().map(|e| (e.id.as_str(), e))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut issues = Vec::new();
    for owner in &document.elements {
        let read_bool = |element, field| ecore_defaults::read_attribute(element, field)
            .map_err(DefinitionDocumentError::ConstructionOrLinking)
            .and_then(|value| value.as_bool().ok_or_else(||
                DefinitionDocumentError::Artifact(format!("Expected Boolean Ecore attribute {field}"))));
        if read_bool(owner, "is_implied_included")? { continue; }
        let contract = ecore_model::feature(&owner.kind, "owned_relationship")
            .ok_or_else(|| DefinitionDocumentError::Artifact("Missing Element ownership contract".into()))?;
        if !contract.containment || contract.derived || contract.volatile || contract.transient {
            return Err(DefinitionDocumentError::Artifact("Unsupported Element ownership contract".into()));
        }
        let mut implied = Vec::new();
        if let Some(value) = owner.properties.get(contract.field) {
            ecore_model::validate_value(&owner.kind, contract.field, value)
                .map_err(DefinitionDocumentError::ConstructionOrLinking)?;
            for id in value.as_array().ok_or_else(||
                DefinitionDocumentError::ConstructionOrLinking("Expected ordered ownership list".into()))? {
                let relationship = index[id.as_str().expect("validated reference identity")];
                if read_bool(relationship, "is_implied")? { implied.push(relationship.id.clone()); }
            }
        }
        if !implied.is_empty() { issues.push(ImpliedInclusionViolation {
            element_id: owner.id.clone(), implied_relationship_ids: implied,
        }); }
    }
    Ok(issues)
}

/// Result of an individual written FeatureValue constraint. A dependency
/// failure remains unverified even if other constraints in this assessment pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureValueIssue {
    pub feature_id: String,
    pub valuation_id: String,
    pub constraint: String,
    pub message: String,
    pub unverified: bool,
}

/// Assess required valuation storage, the single-valuation obligation, initial
/// variability and overriding through shared native queries. The algorithms are
/// explicitly handwritten Rust; Ecore provides contracts rather than algorithms.
/// This is a partial semantic assessment, not full validation or transformation.
/// Missing computed redefinitions/variability are returned as unverified issues.
/// Malformed ownership/storage returns an error. No model or flag is changed.
pub fn assess_feature_values(document: &KirDocument)
    -> Result<Vec<FeatureValueIssue>, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_assess_feature_values(&document.elements)
        .map(|issues| issues.into_iter().map(|issue| FeatureValueIssue {
            feature_id: issue.feature_id, valuation_id: issue.valuation_id,
            constraint: issue.constraint.into(), message: issue.message, unverified: issue.unverified,
        }).collect())
        .map_err(|e| DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Result of one of the six written KerML type-set cardinality/self predicates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSetIssue {
    pub type_id: String,
    pub constraint: String,
    pub message: String,
    pub unverified: bool,
}

/// Partial semantic assessment over canonical Closed storage. Ecore/Xtext roles
/// supply endpoints/projections; delegate execution and the six normative
/// predicates are handwritten Rust. This does not assess set interpretations,
/// every Type constraint, or complete resource/lifecycle validity.
pub fn assess_type_sets(document: &KirDocument) -> Result<Vec<TypeSetIssue>, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_assess_type_sets(&document.elements)
        .map(|issues| issues.into_iter().map(|issue| TypeSetIssue {
            type_id: issue.type_id, constraint: issue.constraint.into(),
            message: issue.message, unverified: issue.unverified,
        }).collect())
        .map_err(|e| DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// One unsatisfied written Feature valuation specialization requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuationSpecializationViolation {
    pub feature_id: String,
    pub valuation_id: String,
    pub result_id: String,
}

/// Assess the written `checkFeatureValuationSpecialization` constraint.
/// Checks all directly owned valuations, including default and initial values,
/// when the Feature is undirected and has no explicit owned specialization.
/// Missing expression/result/generalization dependencies return an error;
/// they are not counted as passing. This does not transform or complete models.
pub fn assess_valuation_specialization(document: &KirDocument)
    -> Result<Vec<ValuationSpecializationViolation>, DefinitionDocumentError>
{
    crate::language_frontend::lowering::emit::definition_assess_valuation_specialization(&document.elements)
        .map(|issues| issues.into_iter().map(|(feature_id,valuation_id,result_id)|
            ValuationSpecializationViolation {feature_id,valuation_id,result_id}).collect())
        .map_err(|e| DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Materialize default and positional-redefinition contributions for a fixed,
/// noncomposite out result Feature under an admitted Expression or Function.
/// Uses imported selection and shared reduction/ownership. Unsupported metadata,
/// constructors and other producers remain errors; the result stays incomplete.
/// A canonical directed valuation can be preserved because it cannot affect
/// this selector. Value transformation, binding and validation stay separate.
pub fn materialize_result_feature_defaults(document: &mut KirDocument,
    result_id: &str, prefix: &str) -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_result_feature_defaults(
        &mut document.elements,result_id,prefix)
        .map_err(|e|DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Materialize the imported base-default contribution for admitted Expressions.
/// Detached, package, FeatureMembership and FeatureValue ownership are admitted
/// with Ecore-valid directions and resolved default-role prerequisites. Completed
/// and end receivers, metadata, crossing, receiver-owned valuations and the full
/// inherited lifecycle remain explicit dependencies.
/// The shared reducer and Ecore insertion commit atomically. This does not
/// complete the expression, its result, or its generalization provider.
pub fn materialize_value_expression_defaults(document: &mut KirDocument,
    expression_id: &str, prefix: &str) -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_value_expression_defaults(
        &mut document.elements,expression_id,prefix)
        .map_err(|e|DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Atomic cold receiver and value-binding composition for explicitly typed,
/// fixed literal owners. No complete resource/model qualification is inferred.
pub fn complete_literal_value_bindings(document: &mut KirDocument, receivers: &[&str], prefix: &str)
    -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_complete_literal_value_bindings(&mut document.elements, receivers, prefix)
        .map_err(|e| DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Complete selected literal/null FeatureValue receivers through native ordered
/// defaults, inherited result/typing and featuring stages atomically. Supplied
/// prototypes/owners stay incomplete. This does not validate or complete the
/// enclosing value binding/document; metadata remains semantically unassessed.
pub fn complete_literal_value_expressions(document: &mut KirDocument, owners: &[&str], prefix: &str)
    -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_complete_literal_value_expressions(
        &mut document.elements, owners, prefix)
        .map_err(|e| DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Materialize the shared featuring stage for a FeatureValue-owned expression.
/// The owner needs complete featuring inputs or the assessed fixed ordinary
/// Feature provider. Imported dispatch
/// selects supported expression classes; canonical TypeFeaturing insertion
/// deduplicates and adopts detached targets transactionally. Other expression
/// contexts and full expression/owner completion remain separate dependencies.
pub fn materialize_value_expression_featuring(document: &mut KirDocument,
    expression_id: &str, prefix: &str) -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_value_expression_featuring(
        &mut document.elements,expression_id,prefix)
        .map_err(|e|DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Materialize the bound-value Subsetting contribution for an exact Feature.
/// A single non-default FeatureValue with a resolved expression/result is
/// required. Default/multiple valuations and existing implied specializations
/// reject pending normative assessment; explicit specialization or parameter
/// direction suppresses this contribution. The expression must already be
/// complete; this does not evaluate
/// or transform it. Shared chain construction and Ecore specialization ownership
/// commit atomically. The owner remains incomplete pending its other producers.
pub fn materialize_bound_value_subsetting(document: &mut KirDocument, owner: &str,
    chain: &str, relationship: &str) -> Result<bool, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_bound_value_subsetting(
        &mut document.elements,owner,chain,relationship)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Complete a batch of fixed ordinary Class-owned leaf Features atomically.
/// Each pair is a Feature identity and a unique prefix for generated relations.
/// End/parameter/variable/nested/specialized contexts and unimplemented semantic
/// producers reject. A shared input snapshot supplies default generalizations,
/// reduction and owning featuring before completion flags are committed.
/// This completes only these admitted Features, not the rest of the document.
pub fn complete_ordinary_features(document: &mut KirDocument, owners: &[(&str,&str)])
    -> Result<(), DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_complete_ordinary_features(&mut document.elements,owners)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Construct and complete a fresh binding and its two ends atomically in a
/// bounded ordinary Class context. Endpoints must be already-owned, fixed exact
/// Features without nested Features. Valuation, crossing, expression, metadata,
/// conjugation and specialized-owner dependencies reject. Shared imported
/// defaults, positional selection, reduction, featuring and endpoint services
/// must all succeed before the graph is committed. Existing Types are not marked complete.
/// This does not certify the rest of the document or full language conformance.
pub fn materialize_complete_binding(document: &mut KirDocument, owner: &str,
    identity: &str, source: &str, target: &str) -> Result<(), DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_complete_binding(
        &mut document.elements,owner,identity,source,target)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Transform a fresh binding from explicitly complete, fixed, canonically owned
/// Type/Feature inputs. Existing inputs are never completed by this operation.
/// Variable, valuation, crossing, metadata and unresolved resource dependencies
/// reject. Only the fresh connector and its two constructed ends are completed.
pub fn materialize_resolved_binding(document: &mut KirDocument, owner: &str,
    identity: &str, source: &str, target: &str) -> Result<(), DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_resolved_binding(
        &mut document.elements, owner, identity, source, target)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Materialize the owning-type featuring contribution using pinned getter and
/// adapter dispatch. Usage getters use mayTimeVary; missing semantic inputs
/// reject instead of falling back to raw flags. Locally resolved library roots
/// are required for variable snapshots. Returns whether a relationship was added.
/// Matching reused targets suppress duplicates. Non-Occurrence variable calls
/// construct fresh snapshot identities. This does not complete transformation.
/// Failure leaves the document unchanged.
pub fn materialize_owning_type_featuring(document: &mut KirDocument, feature_id: &str, relation_id: &str)
    -> Result<bool, DefinitionDocumentError>
{
    crate::language_frontend::lowering::emit::definition_materialize_owning_type_featuring(&mut document.elements, feature_id, relation_id)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Construct a detached BindingConnector and two ordered ends atomically.
/// Existing owned endpoints stay in place; detached endpoints are adopted by
/// the first referencing end's ReferenceSubsetting. None produces an unbound end.
/// This is the handwritten construction dependency, not connector transformation
/// or full semantic validation. No implicit-completeness flag is set.
pub fn append_binding_connector(document: &mut KirDocument, id: &str,
    source: Option<&str>, target: Option<&str>) -> Result<(), DefinitionDocumentError>
{
    crate::language_frontend::lowering::emit::definition_append_binding_connector(&mut document.elements, id, source, target)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Append a detached Feature with ordered FeatureChaining relationships.
/// This handwritten Pilot utility dependency expands input chains once and
/// preserves repeated endpoints, using imported Ecore storage contracts.
/// Expression transformation, bound values and crossing remain separate services.
/// Failure leaves the entire document unchanged.
pub fn append_feature_chain(document: &mut KirDocument, id: &str, inputs: &[&str])
    -> Result<(), DefinitionDocumentError>
{
    crate::language_frontend::lowering::emit::definition_append_feature_chain(&mut document.elements, id, inputs)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Materialize a missing owned result for a resolved expression member provider.
/// Existing return membership suppresses creation, including an empty membership.
/// This is a transactional construction step, not complete expression transformation.
pub fn materialize_expression_result(document: &mut KirDocument, owner_id: &str, membership_id: &str, result_id: &str)
    -> Result<bool, DefinitionDocumentError>
{
    crate::language_frontend::lowering::emit::definition_materialize_expression_result(&mut document.elements, owner_id, membership_id, result_id)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Materialize missing owned results for every imported supported provider in a
/// linked document, atomically. Seven resolved expression bindings participate;
/// inherited literal results and other additional-member providers stay separate.
/// Identities derive from the caller's prefix and canonical owner ID; replay is
/// idempotent. No transformation-completion or qualification flag is changed.
pub fn materialize_owned_expression_results(document: &mut KirDocument, identity_prefix: &str)
    -> Result<usize, DefinitionDocumentError>
{
    crate::language_frontend::lowering::emit::definition_materialize_owned_expression_results(&mut document.elements, identity_prefix)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Execute sourceFeature for a Transition whose additional-member producer is a verified no-op.
/// Connector-producing inputs fail explicitly; this query never changes model state.
pub fn transition_source_feature<'a>(document: &'a KirDocument, owner_id: &str)
    -> Result<Option<&'a mercurio_foundation::kir::KirElement>,DefinitionDocumentError> {
    let identities=document.elements.iter().map(|e|e.id.as_str()).collect::<std::collections::BTreeSet<_>>();
    if identities.len()!=document.elements.len() || identities.contains("") {
        return Err(DefinitionDocumentError::ConstructionOrLinking("Source queries require unique nonempty identities".into()));
    }
    let owner=document.elements.iter().find(|e|e.id==owner_id)
        .ok_or_else(||DefinitionDocumentError::ConstructionOrLinking("Transition source owner absent".into()))?;
    crate::language_frontend::lowering::emit::definition_transition_source_feature(&document.elements,owner)
        .map_err(|e|DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Complete a single source membership stage without constructing transition connectors.
pub fn materialize_transition_source(document: &mut KirDocument, owner: &str, identity: &str)
    -> Result<bool, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_transition_source(&mut document.elements,owner,identity)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Materialize only TransitionUsage's previous-feature source membership stage.
/// Connector construction, generalization and semantic validation remain separate.
pub fn materialize_transition_sources(document: &mut KirDocument, identity_prefix: &str)
    -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_transition_sources(&mut document.elements, identity_prefix)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Construct admitted bare owned multiplicities using pinned Usage policies.
/// Bounds, featuring, implicit generalizations and full validation remain separate.
/// The entire batch is atomic and replay does not create duplicate members.
pub fn materialize_usage_multiplicities(document: &mut KirDocument, identity_prefix: &str)
    -> Result<usize, DefinitionDocumentError>
{
    crate::language_frontend::lowering::emit::definition_materialize_usage_multiplicities(&mut document.elements, identity_prefix)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Reduce and atomically insert a fixed snapshot of selected implicit generals.
/// Entries are (relationship metaclass, general ID). Selection, chain
/// deduplication and full transformation remain explicit caller dependencies.
pub fn materialize_selected_generalizations(document: &mut KirDocument, owner_id: &str,
    identity_prefix: &str, contributions: &[(&str,&str)]) -> Result<usize,DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_selected_generals(&mut document.elements,owner_id,identity_prefix,contributions)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Atomically insert selected generalizations for multiple owners. Each tuple is
/// (owner ID, fresh identity prefix, normalized contributions). All reductions
/// read the original graph; no intermediate transformed model is queried.
/// Contribution selection and complete transformation remain caller dependencies.
pub fn materialize_generalization_batch(document: &mut KirDocument,
    batches: &[(&str, &str, &[(&str, &str)])]) -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_general_batch(&mut document.elements,batches)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Select and materialize bounded Feature-family default contributions using
/// imported providers and shared local library resolution. Tuples contain owner
/// IDs and fresh identity prefixes. All owners are planned before mutation.
/// Full transformation, including valuation and featuring, remains separate.
pub fn materialize_feature_defaults(document: &mut KirDocument,
    owners: &[(&str, &str)]) -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_feature_defaults(&mut document.elements,owners)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Materialize generated default generalizations for five plain Type kinds and
/// 26 Definition kinds. All owners are selected and reduced before insertion;
/// exact assessed stored defaults make replay idempotent. Metadata, conjugation
/// and unimplemented adapter contexts reject. Other contribution stages and
/// full semantic validation remain unassessed; no completion flags are set.
pub fn materialize_type_defaults(document: &mut KirDocument,
    owners: &[(&str, &str)]) -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_type_defaults(&mut document.elements, owners)
        .map_err(|e| DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Owners in the exact imported plain-Type/Definition default-selector domain.
/// This reports only applicability of that contribution category, not completion
/// of the owner or absence of other required semantic dependencies.
pub fn type_default_contribution_owners(document: &KirDocument) -> Vec<String> {
    crate::language_frontend::lowering::emit::definition_type_default_contribution_owners(&document.elements)
}

/// Owners in the imported Feature, Connector and ordinary-Usage selector domain.
/// Applicability does not establish that their context dependencies are resolved,
/// nor that the complete adapter transformation is implemented.
pub fn feature_default_contribution_owners(document: &KirDocument) -> Vec<String> {
    crate::language_frontend::lowering::emit::definition_feature_default_contribution_owners(&document.elements)
}



/// Atomically construct/place a binary binding and materialize its owning
/// featuring and bounded default generalizations, including its two owned ends.
/// Requires explicit standard-library inputs. This does not assert completion
/// of connector/end transformation or the pending/post-insertion lifecycle.
pub fn materialize_binding_defaults(document: &mut KirDocument, owner: &str,
    identity: &str, source: &str, target: &str) -> Result<(), DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_binding_defaults(&mut document.elements,owner,identity,source,target)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Construct a reference binding and materialize its bounded defaults and featuring.
/// Connector/endpoint lifecycle completion is not implied by this transaction.
pub fn materialize_reference_binding_defaults(document: &mut KirDocument, expression_id: &str, identity: &str)
    -> Result<(), DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_reference_binding_defaults(&mut document.elements,expression_id,identity)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Select and place an explicit reference/result binding transactionally.
/// This stage does not complete the connector, its ends, or the expression.
pub fn materialize_reference_binding(document: &mut KirDocument, expression_id: &str, identity: &str)
    -> Result<(), DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_reference_binding(&mut document.elements,expression_id,identity)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Construct a binding and place it under its Type owner using computed context.
/// Selected endpoints, membership and featuring are committed atomically.
/// Connector/end transformation and defaults are separate dependencies.
pub fn materialize_binding_structure(document: &mut KirDocument, owner: &str,
    identity: &str, source: &str, target: &str) -> Result<(), DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_binding_structure(&mut document.elements,owner,identity,source,target)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Materialize the selected feature-reference result Subsetting atomically.
/// Requires a constructed result. This inserts one pending contribution; it does
/// not complete transformation or remove redundant stored specializations.
pub fn materialize_reference_result_subsetting(document: &mut KirDocument, expression_id: &str, relation_id: &str)
    -> Result<bool, DefinitionDocumentError>
{
    crate::language_frontend::lowering::emit::definition_materialize_reference_result_subsetting(&mut document.elements, expression_id, relation_id)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Select a prospective binary binding context for two resolved Features.
/// This does not construct or transform a connector or mark its ends complete.
pub fn binding_context_for<'a>(document: &'a KirDocument, source_id: &str, target_id: &str)
    -> Result<Option<&'a mercurio_foundation::kir::KirElement>, DefinitionDocumentError>
{
    let find=|id: &str|document.elements.iter().find(|e|e.id==id)
        .ok_or_else(||DefinitionDocumentError::ConstructionOrLinking("Binding endpoint is absent".into()));
    crate::language_frontend::lowering::emit::definition_binding_context(&document.elements,find(source_id)?,find(target_id)?)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Query native compatibility over assessed generalization and access providers.
/// Global Feature access requires Base::Anything; unresolved contributions return an error.
pub fn is_compatible(document: &KirDocument, subtype_id: &str, supertype_id: &str)
    -> Result<bool, DefinitionDocumentError>
{
    let find = |id: &str| document.elements.iter().find(|e|e.id==id)
        .ok_or_else(||DefinitionDocumentError::ConstructionOrLinking("Compatibility endpoint is absent".into()));
    crate::language_frontend::lowering::emit::definition_is_compatible(&document.elements,find(subtype_id)?,find(supertype_id)?)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Read the ordered transitive featuring closure using the assessed native providers.
/// Unknown implicit contributions reject; the query never marks models complete.
pub fn all_featuring_types<'a>(document: &'a KirDocument, feature_id: &str)
    -> Result<Vec<&'a mercurio_foundation::kir::KirElement>, DefinitionDocumentError>
{
    let feature = document.elements.iter().find(|e| e.id == feature_id)
        .ok_or_else(|| DefinitionDocumentError::ConstructionOrLinking("Featuring owner is absent".into()))?;
    crate::language_frontend::lowering::emit::definition_all_featuring_types(&document.elements, feature)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Read stored reference targets, canonical owned-feature/end queries, and the
/// supported membership/import ownership views, following imported Ecore property redefinitions. Other derived queries
/// return an explicit error. Reading never materializes duplicate alias fields.
pub fn reference_targets<'a>(document: &'a KirDocument, owner_id: &str, field: &str)
    -> Result<Vec<&'a mercurio_foundation::kir::KirElement>, DefinitionDocumentError>
{
    let identities = document.elements.iter().map(|e| e.id.as_str()).collect::<std::collections::BTreeSet<_>>();
    if identities.len() != document.elements.len() || identities.contains("") {
        return Err(DefinitionDocumentError::ConstructionOrLinking("Reference queries require unique nonempty identities".into()));
    }
    let owner = document.elements.iter().find(|e| e.id == owner_id)
        .ok_or_else(|| DefinitionDocumentError::ConstructionOrLinking("Reference owner is absent".into()))?;
    crate::language_frontend::lowering::emit::definition_reference_targets(&document.elements, owner, field)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Compute the bounded Usage variability delegate. Missing standard-library or
/// generalization providers remain explicit errors. No model state is changed.
pub fn may_time_vary(document: &KirDocument, usage_id: &str) -> Result<bool, DefinitionDocumentError> {
    let identities = document.elements.iter().map(|e| e.id.as_str()).collect::<std::collections::BTreeSet<_>>();
    if identities.len() != document.elements.len() || identities.contains("") {
        return Err(DefinitionDocumentError::ConstructionOrLinking("Variability queries require unique nonempty identities".into()));
    }
    let usage = document.elements.iter().find(|e| e.id == usage_id)
        .ok_or_else(|| DefinitionDocumentError::ConstructionOrLinking("Variability owner is absent".into()))?;
    crate::language_frontend::lowering::emit::definition_may_time_vary(&document.elements, usage)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Query reflexive/transitive specialization using supported native general-type
/// providers. Unknown providers are errors, never negative conformance evidence.
/// This reads the model without materializing implied relationships.
pub fn specializes(document: &KirDocument, subtype_id: &str, supertype_id: &str)
    -> Result<bool, DefinitionDocumentError> {
    let identities = document.elements.iter().map(|e| e.id.as_str()).collect::<std::collections::BTreeSet<_>>();
    if identities.len() != document.elements.len() || identities.contains("") {
        return Err(DefinitionDocumentError::ConstructionOrLinking("Specialization queries require unique nonempty identities".into()));
    }
    let endpoint = |id: &str| document.elements.iter().find(|e| e.id == id)
        .ok_or_else(|| DefinitionDocumentError::ConstructionOrLinking("Specialization endpoint is absent".into()));
    crate::language_frontend::lowering::emit::definition_specializes(&document.elements, endpoint(subtype_id)?, endpoint(supertype_id)?)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abstract_syntax_json::{export_sysml_abstract_syntax_value, import_sysml_abstract_syntax_value};

    #[test]
    fn definition_ownership_projection_matches_source_controls_and_persistence() {
        let artifact: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/ownership-projection-pilot-controls.json")).unwrap();
        let mut count = 0;
        for case in &artifact["cases"].as_array().unwrap()[..4] {
            let document = parse_and_link(case["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
            let before = serde_json::to_value(&document).unwrap();
            let mut restored: KirDocument = serde_json::from_value(before.clone()).unwrap();
            restored.elements.reverse();
            for observation in case["observations"].as_array().unwrap() {
                let (owner, field, expected) = if observation["kind"] == "FeatureChaining" {
                    let feature = document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&observation["owner"])).unwrap();
                    let relations = reference_targets(&document, &feature.id, "owned_relationship").unwrap();
                    let owner = relations.into_iter().filter(|e|e.kind.ends_with("FeatureChaining")).nth(observation["ordinal"].as_u64().unwrap() as usize).unwrap();
                    (owner, "feature_chained", vec![observation["feature_chained"].clone()])
                } else {
                    let owner = document.elements.iter().find(|e|e.properties.get("declared_name") == Some(&observation["name"])).unwrap();
                    (owner, "annotated_element", observation["targets"].as_array().unwrap().clone())
                };
                for graph in [&document, &restored] {
                    let targets: Vec<_> = reference_targets(graph, &owner.id, field).unwrap().iter().map(|e|e.properties["declared_name"].clone()).collect();
                    assert_eq!(targets, expected, "{observation}");
                    if let Some(documented) = observation.get("documented") {
                        assert_eq!(reference_targets(graph, &owner.id, "documented_element").unwrap()[0].properties["declared_name"], *documented);
                    }
                    if let Some(expected_count) = observation.get("annotation_count") {
                        assert_eq!(reference_targets(graph, &owner.id, "annotation").unwrap().len(), expected_count.as_u64().unwrap() as usize);
                    }
                }
                count += 1;
            }
            let issues = assess_required_features(&document).unwrap();
            assert!(!issues.iter().any(|issue| matches!(issue.field.as_str(), "annotated_element" | "documented_element" | "feature_chained")), "{issues:?}");
            assert_eq!(serde_json::to_value(&document).unwrap(), before);
            assert_eq!(document.metadata["semantic_validation"], "not_assessed");
        }
        assert_eq!(count, 7);
    }

    #[test]
    fn definition_ownership_projection_follows_normative_annotation_order() {
        use mercurio_foundation::kir::KirElement;
        let mut document = parse_and_link("package P;", SourceLanguage::Kerml).unwrap();
        let element = |id: &str, kind: &str, properties: serde_json::Value| KirElement {
            id: id.into(), kind: format!("SysML::{kind}"), layer: 2,
            properties: properties.as_object().unwrap().iter().map(|(k,v)|(k.clone(),v.clone())).collect(),
        };
        document.elements = vec![
            element("P", "Package", json!({"owned_relationship":["outer","wrong"]})),
            element("outer", "Annotation", json!({"owning_related_element":"P","owned_related_element":["mixed"],"annotated_element":"P","owned_relationship":[]})),
            element("mixed", "Comment", json!({"owning_relationship":"outer","owned_relationship":["owned","self"]})),
            element("owned", "Annotation", json!({"owning_related_element":"mixed","annotated_element":"target","owned_relationship":[]})),
            element("self", "Annotation", json!({"owning_related_element":"mixed","annotated_element":"mixed","owned_relationship":[]})),
            element("target", "Feature", json!({"owned_relationship":[]})),
            element("detached", "Comment", json!({"owned_relationship":[]})),
            element("wrong", "FeatureChaining", json!({"owning_related_element":"P","owned_relationship":[]})),
        ];
        let ids = |doc: &KirDocument, field| reference_targets(doc,"mixed",field).unwrap().iter().map(|e|e.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&document,"owning_annotating_relationship"), ["outer"]);
        assert_eq!(ids(&document,"owned_annotating_relationship"), ["owned"]);
        assert_eq!(ids(&document,"annotation"), ["outer","owned"]);
        assert_eq!(ids(&document,"annotated_element"), ["P","target"]);
        // Independent Pilot factory control returns only target. The written
        // specification requires the owning Annotation first (KerML 8.3.2.3.2).
        let pilot: serde_json::Value = serde_json::from_str(include_str!("../../../docs/conformance/2026-08-support/ownership-projection-pilot-controls.json")).unwrap();
        assert_eq!(pilot["cases"][4]["observation"]["targets"], json!(["target"]));
        assert!(reference_targets(&document,"detached","annotated_element").unwrap().is_empty());
        assert!(reference_targets(&document,"wrong","feature_chained").unwrap().is_empty());
        let before = serde_json::to_value(&document).unwrap();
        let mut restored: KirDocument = serde_json::from_value(before.clone()).unwrap();
        restored.elements.reverse();
        assert_eq!(ids(&restored,"annotated_element"), ["P","target"]);
        assert_eq!(serde_json::to_value(&document).unwrap(), before);
        // A recorded derived cache cannot conceal missing targets or ownership.
        let mixed = restored.elements.iter_mut().find(|e|e.id=="mixed").unwrap();
        mixed.properties.insert("annotated_element".into(),json!(["P","target"]));
        mixed.properties.remove("owned_relationship");
        assert!(reference_targets(&restored,"mixed","annotated_element").is_err());
        let mut open = document.clone();
        open.elements.iter_mut().find(|e|e.id=="owned").unwrap().properties.remove("annotated_element");
        assert!(reference_targets(&open,"mixed","annotated_element").is_err());
        let mut bad_inverse = document.clone();
        bad_inverse.elements.iter_mut().find(|e|e.id=="outer").unwrap().properties.insert("owned_related_element".into(),json!([]));
        assert!(reference_targets(&bad_inverse,"mixed","annotated_element").is_err());
    }

    #[test]
    fn definition_ownership_projection_linking_checks_order_privacy_and_missing_targets() {
        for source in [
            "package P { comment c about B,A /*ordered*/ classifier A; classifier B; }",
            "package P { classifier A; classifier B; comment c about B,A /*ordered*/ }",
        ] {
            let document = parse_and_link(source, SourceLanguage::Kerml).unwrap();
            let owner = document.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!("c"))).unwrap();
            let names: Vec<_> = reference_targets(&document,&owner.id,"annotated_element").unwrap().iter().map(|e|e.properties["declared_name"].clone()).collect();
            assert_eq!(names, vec![json!("B"),json!("A")]);
        }
        assert!(parse_and_link("package P { comment c about Missing /*missing*/ }",SourceLanguage::Kerml).is_err());
        assert!(parse_and_link("package P { private classifier A; } package Q { comment c about P::A /*private*/ }",SourceLanguage::Kerml).is_err());
    }

    #[test]
    fn definition_import_projection_matches_pilot_sources_and_resource_order() {
        let artifact: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/import-projection-pilot-controls.json")).unwrap();
        let mut count = 0;
        for case in &artifact["cases"].as_array().unwrap()[..4] {
            let document = parse_and_link(case["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
            let before = serde_json::to_value(&document).unwrap();
            let mut restored: KirDocument = serde_json::from_value(before.clone()).unwrap();
            restored.elements.reverse();
            let imports: Vec<_> = document.elements.iter().filter(|e|e.kind.ends_with("Import")).collect();
            assert_eq!(imports.len(),2);
            for (owner, observation) in imports.iter().zip(case["observations"].as_array().unwrap()) {
                assert_eq!(owner.kind.rsplit("::").next().unwrap(),observation["kind"].as_str().unwrap());
                for graph in [&document,&restored] {
                    let targets = reference_targets(graph,&owner.id,"imported_element").unwrap();
                    assert_eq!(targets.len(),1);
                    assert_eq!(targets[0].properties["declared_name"],observation["imported_element"]);
                }
                count += 1;
            }
            assert!(!assess_required_features(&document).unwrap().iter().any(|issue|issue.field=="imported_element"));
            assert_eq!(serde_json::to_value(&document).unwrap(),before);
            assert_eq!(document.metadata["semantic_validation"],"not_assessed");
        }
        assert_eq!(count,8);
        let library = DefinitionSource { uri:"memory:/library.kerml",text:"package P { class A; alias a for A; }",language:SourceLanguage::Kerml };
        let client = DefinitionSource { uri:"memory:/client.kerml",text:"package Q { private import P::*; private import P::a; }",language:SourceLanguage::Kerml };
        for inputs in [[&library,&client],[&client,&library]] {
            let sources: Vec<_> = inputs.iter().map(|s|DefinitionSource {uri:s.uri,text:s.text,language:s.language}).collect();
            let document = parse_and_link_sources(&sources).unwrap();
            let mut names:Vec<_>=document.elements.iter().filter(|e|e.kind.ends_with("Import")).map(|owner| reference_targets(&document,&owner.id,"imported_element").unwrap()[0].properties["declared_name"].as_str().unwrap().to_string()).collect();
            names.sort();assert_eq!(names,["A","P"]);
            assert!(!assess_required_features(&document).unwrap().iter().any(|issue|issue.field=="imported_element"));
        }
    }

    #[test]
    fn definition_import_projection_rejects_missing_wrong_and_cached_dependencies() {
        let original = parse_and_link("package P { class A; alias a for A; } package Q { private import P::*; private import P::a; }", SourceLanguage::Kerml).unwrap();
        let member_import = original.elements.iter().find(|e|e.kind.ends_with("MembershipImport")).unwrap();
        let namespace_import = original.elements.iter().find(|e|e.kind.ends_with("NamespaceImport")).unwrap();
        let member_id = member_import.properties["imported_membership"].as_str().unwrap();
        let target = reference_targets(&original,&member_import.id,"imported_element").unwrap()[0].id.clone();
        for (id,field) in [(namespace_import.id.as_str(),"imported_namespace"),(member_import.id.as_str(),"imported_membership"),(member_id,"member_element")] {
            let mut incomplete = original.clone();
            incomplete.elements.iter_mut().find(|e|e.id==member_import.id).unwrap().properties.insert("imported_element".into(),json!(target));
            incomplete.elements.iter_mut().find(|e|e.id==id).unwrap().properties.remove(field);
            let owner_id = if field=="imported_namespace" {&namespace_import.id} else {&member_import.id};
            assert!(reference_targets(&incomplete,owner_id,"imported_element").is_err());
        }
        let mut wrong=original.clone();
        wrong.elements.iter_mut().find(|e|e.id==namespace_import.id).unwrap().properties.insert("imported_namespace".into(),json!(member_id));
        assert!(reference_targets(&wrong,&namespace_import.id,"imported_element").is_err());
        let mut external=original.clone();
        external.elements.iter_mut().find(|e|e.id==member_import.id).unwrap().properties.insert("imported_membership".into(),json!("external"));
        assert!(reference_targets(&external,&member_import.id,"imported_element").is_err());
        assert!(parse_and_link("package Q { private import Missing::*; }",SourceLanguage::Kerml).is_err());
        assert!(parse_and_link("package P { private class A; } package Q { private import P::A; }",SourceLanguage::Kerml).is_err());
        assert!(parse_and_link("package P { alias a for b; alias b for a; } package Q { private import P::a; }",SourceLanguage::Kerml).is_err());
    }

    #[test]
    fn definition_multiplicity_projection_matches_pilot_sources_and_persistence() {
        let artifact: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/multiplicity-projection-pilot-controls.json")).unwrap();
        for case in &artifact["cases"].as_array().unwrap()[..4] {
            let document = parse_and_link(case["source"].as_str().unwrap(),SourceLanguage::Kerml).unwrap();
            let owner = document.elements.iter().find(|e|e.kind.ends_with("MultiplicityRange")).unwrap();
            let before = serde_json::to_value(&document).unwrap();
            let mut restored: KirDocument = serde_json::from_value(before.clone()).unwrap();restored.elements.reverse();
            for graph in [&document,&restored] {
                let value = |e: &&mercurio_foundation::kir::KirElement| if e.kind.ends_with("LiteralInteger") {e.properties["value"].clone()} else {json!("*")};
                let bounds:Vec<_>=reference_targets(graph,&owner.id,"bound").unwrap().iter().map(value).collect();
                assert_eq!(json!(bounds),case["observation"]["bounds"]);
                for (field,key) in [("lower_bound","lower"),("upper_bound","upper")] {
                    let targets = reference_targets(graph,&owner.id,field).unwrap();
                    assert_eq!(targets.first().map(value).unwrap_or(serde_json::Value::Null),case["observation"][key]);
                }
            }
            assert!(!assess_required_features(&document).unwrap().iter().any(|issue|matches!(issue.field.as_str(),"bound"|"upper_bound")));
            assert_eq!(serde_json::to_value(&document).unwrap(),before);
            assert_eq!(document.metadata["semantic_validation"],"not_assessed");
        }
    }

    #[test]
    fn definition_multiplicity_projection_checks_owned_members_and_boundary_inputs() {
        use mercurio_foundation::kir::KirElement;
        let artifact:serde_json::Value=serde_json::from_str(include_str!("../../../docs/conformance/2026-08-support/multiplicity-projection-pilot-controls.json")).unwrap();
        let element=|id:&str,kind:&str,properties:serde_json::Value|KirElement {id:id.into(),kind:format!("SysML::{kind}"),layer:2,properties:properties.as_object().unwrap().iter().map(|(k,v)|(k.clone(),v.clone())).collect()};
        for case in &artifact["cases"].as_array().unwrap()[4..] {
            let size=case["owned_expression_count"].as_u64().unwrap();
            let mut document=parse_and_link("package P;",SourceLanguage::Kerml).unwrap();
            let mut members=vec![json!("otherMember"),json!("alias")];
            document.elements=vec![element("otherMember","OwningMembership",json!({"owning_related_element":"m","owned_related_element":["other"]})),element("other","Feature",json!({"owning_relationship":"otherMember","owned_relationship":[]})),element("alias","Membership",json!({"owning_related_element":"m","member_element":"ignored"})),element("ignored","LiteralInteger",json!({"value":999,"owned_relationship":[]}))];
            for i in 0..size {
                let id=format!("e{i}");let member=format!("member{i}");members.push(json!(member));
                document.elements.push(element(&member,"OwningMembership",json!({"owning_related_element":"m","owned_related_element":[id]})));
                document.elements.push(element(&id,"LiteralInteger",json!({"owning_relationship":member,"owned_relationship":[],"value":i+1})));
            }
            document.elements.push(element("m","MultiplicityRange",json!({"owned_relationship":members})));
            for (field,key) in [("bound","bounds"),("lower_bound","lower"),("upper_bound","upper")] {
                let targets=reference_targets(&document,"m",field).unwrap();
                let actual=if field=="bound" {json!(targets.iter().map(|e|e.properties["value"].clone()).collect::<Vec<_>>())} else {targets.first().map(|e|e.properties["value"].clone()).unwrap_or(serde_json::Value::Null)};
                assert_eq!(actual,case["observation"][key]);
            }
            if size>0 {
                let mut stale=document.clone();let owner=stale.elements.iter_mut().find(|e|e.id=="m").unwrap();
                owner.properties.insert("bound".into(),json!(["ignored"]));owner.properties.insert("upper_bound".into(),json!("ignored"));
                assert_eq!(reference_targets(&stale,"m","upper_bound").unwrap()[0].properties["value"],json!(if size==1 {1} else {2}));
                stale.elements.iter_mut().find(|e|e.id=="m").unwrap().properties.remove("owned_relationship");
                assert!(reference_targets(&stale,"m","bound").is_err());
                let mut open=document.clone();open.elements.iter_mut().find(|e|e.id=="member0").unwrap().properties.insert("owned_related_element".into(),json!(["absent"]));
                assert!(reference_targets(&open,"m","upper_bound").is_err());
                let mut inverse=document.clone();inverse.elements.iter_mut().find(|e|e.id=="e0").unwrap().properties.remove("owning_relationship");
                assert!(reference_targets(&inverse,"m","bound").is_err());
            }
        }
    }

    #[test]
    fn definition_evaluability_matches_pilot_constant_branches_and_persistence() {
        use mercurio_foundation::kir::KirElement;
        let controls:serde_json::Value=serde_json::from_str(include_str!("../../../docs/conformance/2026-08-support/evaluability-pilot-controls.json")).unwrap();
        assert_eq!(controls["cases"].as_array().unwrap().len(),13);
        for case in controls["cases"].as_array().unwrap() {
            let mut document=if let Some(source)=case["source"].as_str() {parse_and_link(source,SourceLanguage::Kerml).unwrap()}
                else {parse_and_link("package P;",SourceLanguage::Kerml).unwrap()};
            let observation=&case["observations"][0];
            let id=if case.get("source").is_some() {
                document.elements.iter().find(|e|e.kind.rsplit("::").next()==observation["kind"].as_str()).unwrap().id.clone()
            } else {
                document.elements.push(KirElement {id:"factory".into(),kind:format!("SysML::{}",observation["kind"].as_str().unwrap()),layer:2,properties:Default::default()});"factory".into()
            };
            let before=serde_json::to_value(&document).unwrap();
            assert_eq!(attribute_value(&document,&id,"is_model_level_evaluable").unwrap(),observation["attribute"]);
            assert_eq!(observation["attribute"],observation["operation"]);
            let mut restored:KirDocument=serde_json::from_value(before.clone()).unwrap();restored.elements.reverse();
            assert_eq!(attribute_value(&restored,&id,"is_model_level_evaluable").unwrap(),observation["attribute"]);
            restored.elements.iter_mut().find(|e|e.id==id).unwrap().properties.insert("is_model_level_evaluable".into(),json!(false));
            assert_eq!(attribute_value(&restored,&id,"is_model_level_evaluable").unwrap(),json!(true));
            assert_eq!(serde_json::to_value(&document).unwrap(),before);
            assert_eq!(document.metadata["semantic_validation"],"not_assessed");
            if case.get("source").is_some() {
                let issues=assess_required_features(&document).unwrap();
                assert!(!issues.iter().any(|issue|issue.element_id==id && issue.field=="is_model_level_evaluable"));
                // Evaluability does not manufacture a result or certify evaluation.
                assert!(issues.iter().any(|issue|issue.element_id==id && issue.field=="result" && issue.unverified));
            }
        }
    }

    #[test]
    fn definition_evaluability_leaves_other_dynamic_algorithms_explicit() {
        let original=parse_and_link("package P { feature f = 2; }",SourceLanguage::Kerml).unwrap();
        let id=original.elements.iter().find(|e|e.kind.ends_with("LiteralInteger")).unwrap().id.clone();
        for kind in ["Expression","FeatureReferenceExpression","InvocationExpression","ConstructorExpression","CalculationUsage","ConstraintUsage"] {
            let mut document=original.clone();let owner=document.elements.iter_mut().find(|e|e.id==id).unwrap();
            owner.kind=format!("SysML::{kind}");owner.properties.remove("value");owner.properties.insert("is_model_level_evaluable".into(),json!(true));
            let error=attribute_value(&document,&id,"is_model_level_evaluable").unwrap_err().to_string();
            assert!(error.contains("unimplemented"),"{kind}: {error}");
            let issues=assess_required_features(&document).unwrap();
            assert!(issues.iter().any(|issue|issue.element_id==id && issue.field=="is_model_level_evaluable" && issue.unverified),"{kind}: {issues:?}");
        }
    }

    #[test]
    fn definition_source_progress_preserves_pipeline_and_failed_stage_boundaries() {
        let sources=[DefinitionSource{uri:"p.kerml",text:"package P { class A; }",language:SourceLanguage::Kerml},DefinitionSource{uri:"q.kerml",text:"package Q { private import P::A; }",language:SourceLanguage::Kerml}];
        let mut events=Vec::new();
        let actual=parse_and_link_sources_observed(&sources,|uri,phase|events.push((uri.map(str::to_string),phase))).unwrap();
        assert_eq!(serde_json::to_value(actual).unwrap(),serde_json::to_value(parse_and_link_sources(&sources).unwrap()).unwrap());
        use DefinitionSourcePhase::*;
        assert_eq!(events,vec![(Some("p.kerml".into()),Parsing),(Some("p.kerml".into()),Parsed),(Some("q.kerml".into()),Parsing),(Some("q.kerml".into()),Parsed),(None,ConstructionAndLinking),(None,Constructed)]);
        let bad=[DefinitionSource{uri:"bad.kerml",text:"package",language:SourceLanguage::Kerml}];
        events.clear();assert!(parse_and_link_sources_observed(&bad,|uri,phase|events.push((uri.map(str::to_string),phase))).is_err());
        assert_eq!(events,[(Some("bad.kerml".into()),Parsing)]);
        let unresolved=[DefinitionSource{uri:"missing.kerml",text:"package Q { private import Missing::*; }",language:SourceLanguage::Kerml}];
        events.clear();assert!(parse_and_link_sources_observed(&unresolved,|uri,phase|events.push((uri.map(str::to_string),phase))).is_err());
        assert_eq!(events.last().unwrap().1,ConstructionAndLinking);
        assert!(!events.iter().any(|(_,phase)|*phase==Constructed));
    }

    #[test]
    fn definition_construction_trace_preserves_models_and_errors() {
        use DefinitionConstructionPhase::*;
        let sources=[DefinitionSource{uri:"trace.kerml",text:"package P { classifier C; alias A for C; }",language:SourceLanguage::Kerml}];
        let plain=parse_and_link_sources(&sources).unwrap();
        let mut events=Vec::new();
        let traced=parse_and_link_sources_traced(&sources,|_,_|{},|e|events.push((e.phase,e.owner_id.to_owned(),e.field.map(str::to_owned),e.succeeded))).unwrap();
        assert_eq!(serde_json::to_value(&plain).unwrap(),serde_json::to_value(&traced).unwrap());
        assert_eq!(events.first().unwrap().0,ResourceStarted);
        assert_eq!(events.last().unwrap().0,PublicationFinished);
        assert_eq!(events.last().unwrap().3,Some(true));
        let started=events.iter().filter(|e|e.0==ReferenceStarted).count();
        assert!(started>0);
        assert_eq!(started,events.iter().filter(|e|e.0==ReferenceFinished).count());
        let sources=[DefinitionSource{uri:"bad.kerml",text:"package P { alias A for Missing; }",language:SourceLanguage::Kerml}];
        let plain=parse_and_link_sources(&sources).unwrap_err();events.clear();
        let traced=parse_and_link_sources_traced(&sources,|_,_|{},|e|events.push((e.phase,e.owner_id.to_owned(),e.field.map(str::to_owned),e.succeeded))).unwrap_err();
        assert_eq!(plain,traced);
        assert!(events.iter().any(|e|e.0==ReferenceFinished && e.3==Some(false)));
        assert_eq!(events.last().unwrap().0,LinkingFinished);
        assert_eq!(events.last().unwrap().3,Some(false));
        assert!(!events.iter().any(|e|e.0==PublicationFinished));
    }

    #[test]
    fn definition_library_origin_matches_independent_pilot_and_persistence() {
        use crate::language_frontend::lowering::emit::definition_attribute_value;
        let artifact: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/library-origin-pilot-controls.json")).unwrap();
        let cases = artifact["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 6);
        let mut count = 0;
        for case in &cases[..5] {
            let document = parse_and_link(case["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
            let before = serde_json::to_value(&document).unwrap();
            for observation in case["observations"].as_array().unwrap() {
                let owner = document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&observation["name"])).unwrap();
                assert_eq!(owner.kind.rsplit("::").next().unwrap(), observation["kind"].as_str().unwrap());
                let library_name = |id: &str| library_namespace(&document, id).unwrap()
                    .map(|e| e.properties["declared_name"].clone()).unwrap_or(serde_json::Value::Null);
                assert_eq!(library_name(&owner.id), observation["library"]);
                assert_eq!(definition_attribute_value(&document.elements, owner, "is_library_element").unwrap(), observation["is_library_element"]);
                assert_eq!(attribute_value(&document, &owner.id, "is_library_element").unwrap(), observation["is_library_element"]);
                assert_eq!(attribute_value(&document, &owner.id, "element_id").unwrap(), json!(owner.id));
                if let Some(expected) = observation.get("is_conjugated") {
                    assert_eq!(definition_attribute_value(&document.elements, owner, "is_conjugated").unwrap(), *expected);
                    let conjugators = reference_targets(&document, &owner.id, "owned_conjugator").unwrap();
                    let original = conjugators.first().map(|relation| reference_targets(&document, &relation.id, "original_type").unwrap()[0].properties["declared_name"].clone()).unwrap_or(serde_json::Value::Null);
                    assert_eq!(original, observation["conjugator_original"]);
                }
                if let Some(id) = owner.properties.get("owning_relationship").and_then(serde_json::Value::as_str) {
                    assert_eq!(library_name(id), observation["owning_relationship_library"]);
                }
                count += 1;
            }
            if let Some(alias) = case.get("alias") {
                let owner = document.elements.iter().find(|e| e.properties.get("member_name") == Some(&alias["name"])).unwrap();
                assert_eq!(library_namespace(&document, &owner.id).unwrap().map(|e|e.properties["declared_name"].clone()).unwrap_or(serde_json::Value::Null), alias["library"]);
                assert_eq!(definition_attribute_value(&document.elements, owner, "is_library_element").unwrap(), alias["is_library_element"]);
            }
            assert!(!assess_required_features(&document).unwrap().iter().any(|issue| matches!(issue.field.as_str(), "is_library_element" | "is_conjugated")));
            assert_eq!(serde_json::to_value(&document).unwrap(), before);
            let mut restored: KirDocument = serde_json::from_value(before).unwrap();
            restored.elements.reverse();
            for owner in &document.elements {
                assert_eq!(library_namespace(&document, &owner.id).unwrap().map(|e| &e.id), library_namespace(&restored, &owner.id).unwrap().map(|e| &e.id));
            }
            assert_eq!(document.metadata["semantic_validation"], "not_assessed");
        }
        assert_eq!(count, 22);
        // Recreate the independent factory control using canonical KIR:
        // LibraryPackage -> OwningMembership -> Specialization.
        let mut document = parse_and_link("library package DetachedLibrary { feature f; }", SourceLanguage::Kerml).unwrap();
        let owner = document.elements.iter_mut().find(|e| e.properties.get("declared_name") == Some(&json!("f"))).unwrap();
        owner.kind = "SysML::Specialization".into();
        owner.properties.retain(|field, _| matches!(field.as_str(), "owning_relationship" | "owned_relationship"));
        let id = owner.id.clone();
        assert_eq!(library_namespace(&document, &id).unwrap().unwrap().properties["declared_name"], cases[5]["library"]);
        let owner = document.elements.iter().find(|e| e.id == id).unwrap();
        assert_eq!(definition_attribute_value(&document.elements, owner, "is_library_element").unwrap(), cases[5]["is_library_element"]);
    }

    #[test]
    fn definition_library_origin_rejects_cycles_open_ownership_and_cached_flags() {
        use crate::language_frontend::lowering::emit::definition_attribute_value;
        let original = parse_and_link("package P;", SourceLanguage::Kerml).unwrap();
        let owner = original.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("P"))).unwrap();
        let id = owner.id.clone();
        let membership_id = owner.properties["owning_relationship"].as_str().unwrap().to_string();
        let root_id = original.elements.iter().find(|e| e.id == membership_id).unwrap().properties["owning_related_element"].as_str().unwrap().to_string();
        let mut cached = original.clone();
        cached.elements.iter_mut().find(|e| e.id == id).unwrap().properties.insert("is_library_element".into(), json!(true));
        let owner = cached.elements.iter().find(|e| e.id == id).unwrap();
        assert_eq!(definition_attribute_value(&cached.elements, owner, "is_library_element").unwrap(), json!(false));
        let mut open = original.clone();
        open.elements.iter_mut().find(|e| e.id == membership_id).unwrap().properties.insert("owning_related_element".into(), json!("missing"));
        assert!(library_namespace(&open, &id).is_err());
        let mut cycle = original.clone();
        cycle.elements.iter_mut().find(|e| e.id == root_id).unwrap().properties.insert("owning_relationship".into(), json!(membership_id));
        cycle.elements.iter_mut().find(|e| e.id == membership_id).unwrap().properties.get_mut("owned_related_element").unwrap().as_array_mut().unwrap().push(json!(root_id));
        let before = serde_json::to_value(&cycle).unwrap();
        assert!(library_namespace(&cycle, &id).unwrap_err().to_string().contains("cyclic"));
        assert_eq!(serde_json::to_value(&cycle).unwrap(), before);
    }

    #[test]
    fn definition_conjugation_projection_rejects_unready_inputs_and_ignores_caches() {
        use crate::language_frontend::lowering::emit::definition_attribute_value;
        let original = parse_and_link("package P { classifier A; classifier B conjugates A; classifier Plain; }", SourceLanguage::Kerml).unwrap();
        let conjugated = original.elements.iter().find(|e|e.properties.get("declared_name") == Some(&json!("B"))).unwrap();
        let plain = original.elements.iter().find(|e|e.properties.get("declared_name") == Some(&json!("Plain"))).unwrap();
        let conjugator = reference_targets(&original, &conjugated.id, "owned_conjugator").unwrap()[0];
        let mut cached = original.clone();
        let owner = cached.elements.iter_mut().find(|e| e.id == plain.id).unwrap();
        owner.properties.insert("owned_conjugator".into(), json!(conjugator.id));
        owner.properties.insert("is_conjugated".into(), json!(true));
        let owner = cached.elements.iter().find(|e|e.id == plain.id).unwrap();
        assert_eq!(definition_attribute_value(&cached.elements, owner, "is_conjugated").unwrap(), json!(false));
        assert!(reference_targets(&cached, &plain.id, "owned_conjugator").unwrap().is_empty());
        let mut incomplete = original.clone();
        incomplete.elements.iter_mut().find(|e|e.id == plain.id).unwrap().properties.remove("owned_relationship");
        assert!(reference_targets(&incomplete, &plain.id, "owned_conjugator").is_err());
        assert!(assess_required_features(&incomplete).unwrap().iter().any(|issue|issue.element_id == plain.id && issue.field == "is_conjugated" && issue.unverified));
        let mut open = original.clone();
        open.elements.iter_mut().find(|e| e.id == conjugated.id).unwrap().properties.insert("owned_relationship".into(), json!(["missing"]));
        assert!(reference_targets(&open, &conjugated.id, "owned_conjugator").is_err());
    }

    #[test]
    fn definition_required_values_use_native_consumers_and_preserve_boundaries() {
        let document = parse_and_link("package P { feature f; }", SourceLanguage::Kerml).unwrap();
        let before = serde_json::to_value(&document).unwrap();
        let issues = assess_required_features(&document).unwrap();
        let feature = document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("f"))).unwrap();
        assert!(!issues.iter().any(|issue| issue.element_id == feature.id && issue.field == "feature_target"), "{issues:?}");
        for membership in document.elements.iter().filter(|e| e.kind.ends_with("OwningMembership")) {
            assert!(!issues.iter().any(|issue| issue.element_id == membership.id && issue.field == "owned_member_element"), "{issues:?}");
        }
        assert!(!issues.iter().any(|issue| matches!(issue.field.as_str(), "member_element" | "member_element_id" | "owned_member_element_id")), "{issues:?}");
        assert!(issues.is_empty(), "{issues:?}");
        let expressions = parse_and_link("package P { feature f [1..2]; }", SourceLanguage::Kerml).unwrap();
        let pending = assess_required_features(&expressions).unwrap();
        assert!(pending.iter().any(|issue| issue.unverified && !issue.feature_id.is_empty()), "{pending:?}");
        assert_eq!(serde_json::to_value(&document).unwrap(), before);
        assert_eq!(document.metadata["semantic_validation"], "not_assessed");
        let restored = serde_json::from_value(before).unwrap();
        assert_eq!(assess_required_features(&restored).unwrap(), issues);

        // A stored, structurally valid derived snapshot cannot supply a missing
        // native consumer prerequisite, even if it looks like the correct value.
        let mut incomplete = document.clone();
        let feature_id = feature.id.clone();
        let feature = incomplete.elements.iter_mut().find(|e| e.id == feature_id).unwrap();
        feature.properties.insert("feature_target".into(), json!(feature.id));
        feature.properties.remove("owned_relationship");
        let issues = assess_required_features(&incomplete).unwrap();
        assert!(issues.iter().any(|issue| issue.element_id == feature_id && issue.field == "feature_target" && issue.unverified));
    }

    #[test]
    fn definition_required_values_resolve_specialization_and_membership_redefinitions() {
        let document = parse_and_link("package P { class A; class B :> A; feature f : A; }", SourceLanguage::Kerml).unwrap();
        let issues = assess_required_features(&document).unwrap();
        assert!(!issues.iter().any(|issue| matches!(issue.field.as_str(), "general" | "specific" | "member_element")), "{issues:?}");
        assert!(!issues.iter().any(|issue| !issue.unverified), "{issues:?}");
        use crate::language_frontend::lowering::emit::definition_attribute_value;
        for membership in document.elements.iter().filter(|e| e.kind.ends_with("OwningMembership") || e.kind.ends_with("FeatureMembership")) {
            let target = reference_targets(&document, &membership.id, "owned_member_element").unwrap()[0];
            assert_eq!(definition_attribute_value(&document.elements, membership, "member_element_id").unwrap(), json!(target.id));
            assert_eq!(definition_attribute_value(&document.elements, membership, "owned_member_element_id").unwrap(), json!(target.id));
        }
        let membership = document.elements.iter().find(|e| e.kind.ends_with("OwningMembership")).unwrap();
        let target_id = reference_targets(&document, &membership.id, "owned_member_element").unwrap()[0].id.clone();
        let mut explicit = document.clone();
        explicit.elements.iter_mut().find(|e| e.id == target_id).unwrap().properties.insert("element_id".into(), json!("explicit-identity"));
        let owner = explicit.elements.iter().find(|e| e.id == membership.id).unwrap();
        assert_eq!(definition_attribute_value(&explicit.elements, owner, "member_element_id").unwrap(), json!("explicit-identity"));
        assert!(assess_required_features(&explicit).unwrap().iter().all(|issue| issue.unverified));
    }

    #[test]
    fn definition_required_values_reject_open_or_malformed_graphs() {
        let mut document = parse_and_link("package P { feature f; }", SourceLanguage::Kerml).unwrap();
        let membership = document.elements.iter_mut().find(|e| e.kind.ends_with("OwningMembership")).unwrap();
        membership.properties.insert("owned_member_element".into(), json!("missing"));
        let before = serde_json::to_value(&document).unwrap();
        assert!(assess_required_features(&document).is_err());
        assert_eq!(serde_json::to_value(&document).unwrap(), before);
    }

    #[test]
    fn definition_typed_generalizations_reach_source_and_persistence() {
        let original=parse_and_link("package P { class A; class B; feature x; }",SourceLanguage::Kerml).unwrap();
        let find=|name|original.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id.clone();
        let a=find("A");let b=find("B");let x=find("x");
        let mut document=original.clone();
        assert_eq!(materialize_generalization_batch(&mut document,&[(&x,"typing",&[("FeatureTyping",&a)]),(&b,"classification",&[("Subclassification",&a)])]).unwrap(),2);
        let exported=export_sysml_abstract_syntax_value(&document,Default::default()).unwrap();assert!(!exported.has_errors());
        let restored=import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!restored.has_errors());
        let restored=restored.persistable_document().unwrap();
        for (id,kind) in [("typing.0","SysML::FeatureTyping"),("classification.0","SysML::Subclassification")] {
            assert_eq!(restored.elements.iter().find(|e|e.id==id).unwrap().kind,kind);
        }
        // Ecore-narrowed specific/general endpoints remain enforced, even when
        // the generic Specialization endpoints would accept these Types.
        for (owner,kind,general) in [(&a,"FeatureTyping",&b),(&b,"Subclassification",&x)] {
            let mut rejected=original.clone();let before=serde_json::to_value(&rejected).unwrap();
            assert!(materialize_selected_generalizations(&mut rejected,owner,"invalid",&[(kind,general)]).is_err());
            assert_eq!(serde_json::to_value(&rejected).unwrap(),before);
        }
    }

    #[test]
    fn definition_default_producers_reach_atomic_source_insertion() {
        let original=parse_and_link("standard library package Base { feature things; } package P { feature a; feature b; }",SourceLanguage::Kerml).unwrap();
        let find=|name|original.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id.clone();
        let a=find("a");let b=find("b");let things=find("things");
        let mut document=original.clone();
        assert_eq!(materialize_feature_defaults(&mut document,&[(&a,"default.a"),(&b,"default.b")]).unwrap(),2);
        for id in ["default.a.0","default.b.0"] {
            let relation=document.elements.iter().find(|e|e.id==id).unwrap();
            assert_eq!(relation.kind,"SysML::Subsetting");assert_eq!(relation.properties["subsetted_feature"],things);
        }
        assert!(document.elements.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
        let exported=export_sysml_abstract_syntax_value(&document,Default::default()).unwrap();assert!(!exported.has_errors());
        let restored=import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!restored.has_errors());
        for batch in [vec![(&a[..],"same"),(&b[..],"same")],vec![(&a[..],"first"),("missing","second")]] {
            let mut rejected=original.clone();let before=serde_json::to_value(&rejected).unwrap();
            assert!(materialize_feature_defaults(&mut rejected,&batch).is_err());
            assert_eq!(serde_json::to_value(&rejected).unwrap(),before);
        }
        // No fabricated library fallback and no silent completion on replay.
        let before=serde_json::to_value(&document).unwrap();assert_eq!(materialize_feature_defaults(&mut document,&[(&a,"again")]).unwrap(),0);
        assert_eq!(serde_json::to_value(&document).unwrap(),before);
        let mut missing=parse_and_link("package P { feature a; }",SourceLanguage::Kerml).unwrap();
        let id=missing.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!("a"))).unwrap().id.clone();
        let before=serde_json::to_value(&missing).unwrap();assert!(materialize_feature_defaults(&mut missing,&[(&id,"absent")]).is_err());
        assert_eq!(serde_json::to_value(&missing).unwrap(),before);
    }

    #[test]
    fn definition_generalization_batch_uses_snapshot_and_rolls_back() {
        let original=parse_and_link("standard library package Base { feature things; } package P { feature a; feature b; feature x; }",SourceLanguage::Kerml).unwrap();
        let find=|name|original.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id.clone();
        let a=find("a");let b=find("b");let x=find("x");
        let first=[("Subsetting",a.as_str())];
        let second=[("Subsetting",b.as_str()),("Subsetting",a.as_str())];
        let mut document=original.clone();
        assert_eq!(materialize_generalization_batch(&mut document,&[(&b,"batch.b",&first),(&x,"batch.x",&second)]).unwrap(),3);
        let exported=export_sysml_abstract_syntax_value(&document,Default::default()).unwrap();assert!(!exported.has_errors());
        let restored=import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!restored.has_errors());
        assert!(document.elements.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
        for batches in [vec![(&b[..],"collision",&first[..]),(&x[..],"collision",&second[..])],
                        vec![(&b[..],"first",&first[..]),(&b[..],"second",&second[..])],
                        vec![(&b[..],"first",&first[..]),("missing","second",&second[..])]] {
            let mut rejected=original.clone();let before=serde_json::to_value(&rejected).unwrap();
            assert!(materialize_generalization_batch(&mut rejected,&batches).is_err());
            assert_eq!(serde_json::to_value(&rejected).unwrap(),before);
        }
    }

    #[test]
    fn definition_implicit_reduction_reaches_source_and_persistence() {
        let mut document=parse_and_link("standard library package Base { feature things; } package P { feature a; feature b subsets a; feature x; }",SourceLanguage::Kerml).unwrap();
        let find=|name|document.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id.clone();
        let a=find("a");let b=find("b");let x=find("x");
        assert_eq!(materialize_selected_generalizations(&mut document,&x,"selected",&[("Subsetting",&a),("Subsetting",&b)]).unwrap(),1);
        assert!(document.elements.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
        let exported=export_sysml_abstract_syntax_value(&document,Default::default()).unwrap();assert!(!exported.has_errors());
        let restored=import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!restored.has_errors());
        let mut restored=restored.persistable_document().unwrap();
        assert_eq!(materialize_selected_generalizations(&mut restored,&x,"replayed",&[("Subsetting",&b)]).unwrap(),0);
        let relation=restored.elements.iter().find(|e|e.id=="selected.0").unwrap();
        assert_eq!(relation.properties["subsetted_feature"],b);
    }

    #[test]
    fn definition_binding_context_reaches_source_and_persistence() {
        let document=parse_and_link("package P { class A { feature x; feature y; } }",SourceLanguage::Kerml).unwrap();
        let find=|name|document.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id.clone();
        let x=find("x");let y=find("y");let a=find("A");
        assert_eq!(binding_context_for(&document,&x,&y).unwrap().unwrap().id,a);
        assert!(binding_context_for(&document,&a,&y).is_err());
        let exported=export_sysml_abstract_syntax_value(&document,Default::default()).unwrap();assert!(!exported.has_errors());
        let restored=import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!restored.has_errors());
        assert_eq!(binding_context_for(&restored.persistable_document().unwrap(),&x,&y).unwrap().unwrap().id,a);
    }

    #[test]
    fn definition_compatibility_reaches_source_and_persistence() {
        let document=parse_and_link("standard library package Occurrences { class Occurrence; } package P { class A; class B :> A; }",SourceLanguage::Kerml).unwrap();
        let find=|name|document.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id.clone();
        let a=find("A");let b=find("B");
        assert!(is_compatible(&document,&b,&a).unwrap());
        assert!(is_compatible(&document,&a,&a).unwrap());
        assert!(is_compatible(&document,"absent",&a).is_err());
        let exported=export_sysml_abstract_syntax_value(&document,Default::default()).unwrap();assert!(!exported.has_errors());
        let restored=import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!restored.has_errors());
        assert!(is_compatible(&restored.persistable_document().unwrap(),&b,&a).unwrap());
    }

    #[test]
    fn definition_featuring_query_reaches_source_and_persistence() {
        use crate::abstract_syntax_json::{export_sysml_abstract_syntax_value, import_sysml_abstract_syntax_value};
        let document=parse_and_link("package P { class A { feature x; } }",SourceLanguage::Kerml).unwrap();
        let feature=document.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!("x"))).unwrap();
        let id=feature.id.clone();
        let actual=all_featuring_types(&document,&id).unwrap();assert_eq!(actual.len(),1);assert_eq!(actual[0].properties["declared_name"],"A");
        let exported=export_sysml_abstract_syntax_value(&document,Default::default()).unwrap();assert!(!exported.has_errors());
        let restored=import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!restored.has_errors());
        let restored=restored.persistable_document().unwrap();
        assert_eq!(all_featuring_types(&restored,&id).unwrap()[0].id,actual[0].id);
    }

    #[test]
    fn definition_binary_binding_defaults_compose_and_roll_back() {
        let library="standard library package Base { feature things; } standard library package Links { assoc Link { feature participant; } feature selfLinks; } ";
        let body="package P { class A { feature x; feature y; } }";
        for with_library in [true,false] {
            let mut document=parse_and_link(&format!("{}{}",if with_library {library}else{""},body),SourceLanguage::Kerml).unwrap();
            let find=|name|document.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id.clone();
            let a=find("A");let x=find("x");let y=find("y");
            let before=serde_json::to_value(&document).unwrap();
            let result=materialize_binding_defaults(&mut document,&a,"binding",&x,&y);
            if !with_library {assert!(result.is_err());assert_eq!(serde_json::to_value(&document).unwrap(),before);continue;}
            result.unwrap();
            for id in ["binding","binding.end.0","binding.end.1"] {
                assert!(document.elements.iter().any(|e|e.id==format!("{id}.featuring")));
                let relations=document.elements.iter().filter(|e|e.id.starts_with(&format!("{id}.defaults."))).collect::<Vec<_>>();
                assert_eq!(relations.len(),1);
                let target=reference_targets(&document,&relations[0].id,"general").unwrap();
                assert_eq!(target[0].properties["declared_name"],if id=="binding" {"selfLinks"}else{"participant"});
            }
            assert!(document.elements.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
            let exported=export_sysml_abstract_syntax_value(&document,Default::default()).unwrap();assert!(!exported.has_errors());
            let restored=import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!restored.has_errors());
            assert!(restored.persistable_document().unwrap().elements.iter().any(|e|e.id=="binding"));
        }
    }

    #[test]
    fn definition_featuring_composes_with_default_producers() {
        let mut document=parse_and_link("standard library package Base { feature things; } package P { class A { feature x; } }",SourceLanguage::Kerml).unwrap();
        let find=|name|document.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id.clone();
        let x=find("x");let things=find("things");
        assert!(specializes(&document,&x,&things).unwrap());
        assert!(materialize_owning_type_featuring(&mut document,&x,"featuring").unwrap());
        assert!(specializes(&document,&x,&things).unwrap());
        let exported=export_sysml_abstract_syntax_value(&document,Default::default()).unwrap();assert!(!exported.has_errors());
        let restored=import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!restored.has_errors());
        let mut restored=restored.persistable_document().unwrap();
        assert!(specializes(&restored,&x,&things).unwrap());
        // A mismatched implied featuring relation is not silently ignored.
        let mut invalid=restored.clone();
        invalid.elements.iter_mut().find(|e|e.id=="featuring").unwrap().properties.insert("feature_of_type".into(),json!(things));
        assert!(specializes(&invalid,&x,&things).is_err());
        assert_eq!(materialize_feature_defaults(&mut restored,&[(&x,"defaults")]).unwrap(),1);
        assert!(restored.elements.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
        // Only the assessed default is admitted; full transformation remains separate.
        assert!(specializes(&restored,&x,&things).unwrap());
        assert_eq!(materialize_feature_defaults(&mut restored,&[(&x,"replay")]).unwrap(),0);
    }

    #[test]
    fn definition_binding_placement_reaches_source_persistence_and_rollback() {
        let original=parse_and_link("package P { class A { feature x; feature y; } class B; }",SourceLanguage::Kerml).unwrap();
        let find=|name|original.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id.clone();
        let a=find("A");let b=find("B");let x=find("x");let y=find("y");
        for (owner,kind) in [(&a,"SysML::FeatureMembership"),(&b,"SysML::OwningMembership")] {
            let mut document=original.clone();
            materialize_binding_structure(&mut document,owner,"binding",&x,&y).unwrap();
            assert_eq!(document.elements.iter().find(|e|e.id=="binding.membership").unwrap().kind,kind);
            assert_eq!(document.elements.iter().filter(|e|e.id=="binding.context").count(),usize::from(owner==&b));
            assert!(document.elements.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
            let exported=export_sysml_abstract_syntax_value(&document,Default::default()).unwrap();assert!(!exported.has_errors());
            let restored=import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!restored.has_errors());
            assert!(restored.persistable_document().unwrap().elements.iter().any(|e|e.id=="binding"));
        }
        let mut collision=original.clone();
        materialize_binding_structure(&mut collision,&a,"binding.membership",&x,&y).unwrap();
        let before=serde_json::to_value(&collision).unwrap();
        assert!(materialize_binding_structure(&mut collision,&a,"binding",&x,&y).is_err());
        assert_eq!(serde_json::to_value(&collision).unwrap(),before);
    }

    #[test]
    fn definition_self_reference_reaches_source_and_persistence() {
        let document=parse_and_link("standard library package Base { classifier Anything { feature self; } } package P { classifier A; feature x = hastype A; }",SourceLanguage::Kerml).unwrap();
        let expression=document.elements.iter().find(|e|e.kind=="SysML::FeatureReferenceExpression").unwrap().id.clone();
        let target=reference_targets(&document,&expression,"referent").unwrap()[0].id.clone();
        assert_eq!(reference_targets(&document,&expression,"referent").unwrap()[0].properties["declared_name"],"self");
        let exported=export_sysml_abstract_syntax_value(&document,Default::default()).unwrap();assert!(!exported.has_errors());
        let restored=import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!restored.has_errors());
        assert_eq!(reference_targets(&restored.persistable_document().unwrap(),&expression,"referent").unwrap()[0].id,target);
        for source in ["package P { classifier A; feature x = hastype A; }", "standard library package Base { classifier Anything { classifier self; } } package P { classifier A; feature x = hastype A; }"] {
            let bad=parse_and_link(source,SourceLanguage::Kerml).unwrap();
            let owner=bad.elements.iter().find(|e|e.kind=="SysML::FeatureReferenceExpression").unwrap();
            assert!(reference_targets(&bad,&owner.id,"referent").is_err());
        }
    }

    #[test]
    fn definition_multiplicity_defaults_link_actual_pinned_libraries_and_replay() {
        let controls:serde_json::Value=serde_json::from_str(include_str!("../../../docs/conformance/2026-08-support/multiplicity-defaults-pilot-controls.json")).unwrap();
        let model="package P { feature a; feature b[a .. a]; feature c = b; }";
        for reverse in [false,true] {
            let mut sources=controls["library_sources"].as_array().unwrap().iter().map(|s|DefinitionSource{uri:s["uri"].as_str().unwrap(),text:s["text"].as_str().unwrap(),language:SourceLanguage::Kerml}).collect::<Vec<_>>();
            sources.push(DefinitionSource{uri:"model.kerml",text:model,language:SourceLanguage::Kerml});if reverse {sources.reverse();}
            let mut document=parse_and_link_sources(&sources).unwrap();
            let ids:Vec<_>=document.elements.iter().filter(|e|e.id.starts_with("definition.resource.") && e.kind=="SysML::MultiplicityRange" && e.id.contains(&"model.kerml".bytes().map(|b|format!("{b:02x}")).collect::<String>())).map(|e|e.id.clone()).collect();
            assert_eq!(ids.len(),1);
            let id=&ids[0];let bounds=reference_targets(&document,id,"bound").unwrap();assert_eq!(bounds.len(),2);
            assert!(bounds.iter().all(|e|e.kind=="SysML::FeatureReferenceExpression"));
            assert_eq!(materialize_owned_expression_results(&mut document,"results").unwrap(),3);
            assert_eq!(materialize_feature_defaults(&mut document,&[(id.as_str(),"multiplicity.default")]).unwrap(),1);
            let owner=document.elements.iter().find(|e|&e.id==id).unwrap();
            let relations=reference_targets(&document,&owner.id,"owned_relationship").unwrap();
            let implied=relations.iter().filter(|e|e.kind=="SysML::Subsetting").collect::<Vec<_>>();assert_eq!(implied.len(),1);
            let general=reference_targets(&document,&implied[0].id,"general").unwrap();assert_eq!(general[0].properties["declared_name"],"naturals");
            let mut restored:KirDocument=serde_json::from_value(serde_json::to_value(&document).unwrap()).unwrap();restored.elements.reverse();
            assert_eq!(materialize_feature_defaults(&mut restored,&[(id.as_str(),"replay")]).unwrap(),0);
            assert_eq!(materialize_owned_expression_results(&mut restored,"replay.results").unwrap(),0);
            assert_eq!(restored.metadata["semantic_validation"],"not_assessed");
            assert!(restored.elements.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
        }
    }

    #[test]
    fn definition_occurrence_defaults_reach_view_expose_source_and_persistence() {
        let libraries="standard library package Base { feature things; } standard library package Views { feature views; class View { feature subviews; } }";
        let model="package P; view v { expose P::*; }";
        let sources=[DefinitionSource {uri:"dependency.kerml",text:libraries,language:SourceLanguage::Kerml},DefinitionSource {uri:"model.sysml",text:model,language:SourceLanguage::Sysml}];
        let mut document=parse_and_link_sources(&sources).unwrap();
        let view=document.elements.iter().find(|e|e.kind=="SysML::ViewUsage").unwrap().id.clone();
        assert_eq!(materialize_feature_defaults(&mut document,&[(view.as_str(),"view.defaults")]).unwrap(),1);
        let expose=document.elements.iter().find(|e|e.kind=="SysML::NamespaceExpose").unwrap();
        assert_eq!(expose.properties["visibility"],json!("protected"));
        let mut restored:KirDocument=serde_json::from_value(serde_json::to_value(&document).unwrap()).unwrap();restored.elements.reverse();let before=serde_json::to_value(&restored).unwrap();
        assert_eq!(materialize_feature_defaults(&mut restored,&[(view.as_str(),"replay")]).unwrap(),0);assert_eq!(serde_json::to_value(restored).unwrap(),before);
        assert_eq!(document.metadata["semantic_validation"],"not_assessed");
        let missing=parse_and_link(model,SourceLanguage::Sysml).unwrap_err().to_string();assert!(missing.contains("Views"),"{missing}");assert!(!missing.contains("no document generalization provider for ViewUsage"));
    }

    #[test]
    fn definition_usage_multiplicity_batch_reaches_source_and_persistence() {
        let mut document=parse_and_link("package P { end item i; end item j[2]; }",SourceLanguage::Sysml).unwrap();
        assert_eq!(materialize_usage_multiplicities(&mut document,"usage.members").unwrap(),1);
        let created=document.elements.iter().filter(|e|e.id.starts_with("usage.members") && e.kind=="SysML::Multiplicity").collect::<Vec<_>>();
        assert_eq!(created.len(),1);
        assert_eq!(document.elements.iter().filter(|e|e.kind=="SysML::MultiplicityRange").count(),1);
        let mut restored:KirDocument=serde_json::from_value(serde_json::to_value(&document).unwrap()).unwrap();restored.elements.reverse();
        let before=serde_json::to_value(&restored).unwrap();assert_eq!(materialize_usage_multiplicities(&mut restored,"replay").unwrap(),0);assert_eq!(serde_json::to_value(restored).unwrap(),before);
        assert_eq!(document.metadata["semantic_validation"],"not_assessed");
        assert!(document.elements.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
    }

    #[test]
    fn definition_owned_result_batch_reaches_multiplicity_and_feature_value_sources() {
        let mut document=parse_and_link("package P { feature x[1 .. 2]; feature y = x; feature z = 1 + 2; }",SourceLanguage::Kerml).unwrap();
        let owners:Vec<_>=document.elements.iter().filter(|e|matches!(e.kind.rsplit("::").next(),Some("OperatorExpression"|"FeatureReferenceExpression"))).map(|e|e.id.clone()).collect();
        assert_eq!(owners.len(),2);
        assert_eq!(materialize_owned_expression_results(&mut document,"whole.results").unwrap(),2);
        for id in &owners {
            let result=reference_targets(&document,id,"result").unwrap();assert_eq!(result.len(),1);
            assert_eq!(result[0].properties["direction"],"out");
        }
        let multiplicity=document.elements.iter().find(|e|e.kind=="SysML::MultiplicityRange").unwrap();
        let bounds=reference_targets(&document,&multiplicity.id,"bound").unwrap();assert_eq!(bounds.len(),2);
        for bound in bounds {assert!(reference_targets(&document,&bound.id,"result").is_err());}
        let unsupported=parse_and_link("package P { feature a; feature x[a]; }",SourceLanguage::Kerml).unwrap_err();
        assert!(!unsupported.to_string().contains("no document generalization provider for MultiplicityRange"));
        let mut restored:KirDocument=serde_json::from_value(serde_json::to_value(&document).unwrap()).unwrap();restored.elements.reverse();
        let before=serde_json::to_value(&restored).unwrap();
        assert_eq!(materialize_owned_expression_results(&mut restored,"replay").unwrap(),0);
        assert_eq!(serde_json::to_value(restored).unwrap(),before);
        assert_eq!(document.metadata["semantic_validation"],"not_assessed");
        assert!(document.elements.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
    }

    #[test]
    fn definition_expression_result_constructor_reaches_candidate_source_and_persistence() {
        use crate::abstract_syntax_json::{export_sysml_abstract_syntax_value, import_sysml_abstract_syntax_value};
        for source in ["package P { feature a; feature b = a; }", "package P { feature b = 1 + 2; }"] {
            let mut document = parse_and_link(source, SourceLanguage::Kerml).unwrap();
            let owners: Vec<_> = document.elements.iter().filter(|e| matches!(e.kind.rsplit("::").next(), Some("FeatureReferenceExpression" | "OperatorExpression"))).map(|e| e.id.clone()).collect();
            assert!(!owners.is_empty());
            for (i, owner) in owners.iter().enumerate() {
                assert!(materialize_expression_result(&mut document, owner, &format!("constructed.{i}.member"), &format!("constructed.{i}.result")).unwrap());
                assert!(!materialize_expression_result(&mut document, owner, "unused.member", "unused.result").unwrap());
                let results = reference_targets(&document, owner, "result").unwrap();
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].id, format!("constructed.{i}.result"));
                let is_reference = document.elements.iter().find(|e| &e.id == owner).unwrap().kind == "SysML::FeatureReferenceExpression";
                if is_reference {
                    assert_eq!(reference_targets(&document,owner,"referent").unwrap()[0].properties["declared_name"],"a");
                    let relation_id = format!("constructed.{i}.subsetting");
                    assert!(materialize_reference_result_subsetting(&mut document, owner, &relation_id).unwrap());
                    let target = reference_targets(&document, &relation_id, "general").unwrap();
                    assert_eq!(target[0].properties["declared_name"], "a");
                }
            }
            assert!(document.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
            let exported = export_sysml_abstract_syntax_value(&document, Default::default()).unwrap();
            assert!(!exported.has_errors(), "{:?}", exported.diagnostics);
            let restored = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
            assert!(!restored.has_errors(), "{:?}", restored.diagnostics);
            let mut restored = restored.persistable_document().unwrap();
            for owner in &owners { assert!(!materialize_expression_result(&mut restored, owner, "unused.member", "unused.result").unwrap()); }
        }
    }

    #[test]
    fn definition_featuring_dispatch_matches_all_admitted_pilot_classes() {
        let artifact: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json")).unwrap();
        let controls = artifact["featuring_dispatch_controls"].as_array().unwrap();
        assert_eq!(controls.len(), 64);
        for control in controls {
            let mut doc = parse_and_link(control["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
            let feature = doc.elements.iter_mut().find(|e| e.properties.get("declared_name") == Some(&json!("f"))).unwrap();
            feature.kind = format!("SysML::{}", control["kind"].as_str().unwrap());
            feature.properties.insert("is_variable".into(), control["variable"].clone());
            let id = feature.id.clone();
            let owner_id = doc.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("C"))).unwrap().id.clone();
            assert!(materialize_owning_type_featuring(&mut doc, &id, "dispatch").unwrap_or_else(|e| panic!("{} {}: {e}", control["kind"], control["variable"])));
            let relation = reference_targets(&doc, &id, "owned_type_featuring").unwrap()[0];
            let target = reference_targets(&doc, &relation.id, "featuring_type").unwrap()[0];
            assert_eq!(target.properties["declared_name"], control["target_name"]);
            assert_eq!(json!(target.id == owner_id), control["target_is_owner"]);
            assert_eq!(json!(target.properties.get("owning_relationship") == Some(&json!(relation.id))), control["target_adopted"]);
            assert_eq!(relation.properties["is_implied"], control["implied"]);
            assert_eq!(json!(relation.properties["feature_of_type"] == id), control["source_matches"]);
            let kinds = target.properties["owned_relationship"].as_array().unwrap().iter().map(|id|
                doc.elements.iter().find(|e| e.id == id.as_str().unwrap()).unwrap().kind.rsplit("::").next().unwrap()).collect::<Vec<_>>();
            assert_eq!(json!(kinds), control["owned_relationships"]);
            assert!(doc.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
        }
    }

    #[test]
    fn definition_usage_featuring_preserves_missing_semantic_dependencies() {
        use crate::language_frontend::lowering::relationship_declarations::metaclass_conforms;
        let artifact: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/feature-redefinitions.extract.json")).unwrap();
        let mut checked = 0;
        for binding in artifact["bindings"].as_array().unwrap() {
            if binding["methods"]["isVariableGetter"] != "org.omg.sysml.lang.sysml.impl.UsageImpl#isVariable" { continue; }
            let mut doc = parse_and_link("package P { class C { feature f; } }", SourceLanguage::Kerml).unwrap();
            let feature = doc.elements.iter_mut().find(|e| e.properties.get("declared_name") == Some(&json!("f"))).unwrap();
            feature.kind = format!("SysML::{}", binding["kind"].as_str().unwrap());
            feature.properties.insert("is_variable".into(), json!(false));
            let excluded = metaclass_conforms(&feature.kind, "SuccessionAsUsage") || metaclass_conforms(&feature.kind, "BindingConnectorAsUsage");
            let id = feature.id.clone();
            let before = serde_json::to_value(&doc).unwrap();
            let result = materialize_owning_type_featuring(&mut doc, &id, "guarded");
            if excluded { assert!(result.unwrap()); }
            else {
                let error = result.unwrap_err().to_string();
                assert!(error.contains("library"), "{}: {error}", binding["kind"]);
                assert_eq!(serde_json::to_value(&doc).unwrap(), before);
            }
            checked += 1;
        }
        assert_eq!(checked, 47);
    }

    #[test]
    fn definition_usage_featuring_matches_all_pilot_bindings_on_complete_inputs() {
        use crate::language_frontend::lowering::relationship_declarations::metaclass_conforms;
        let artifact: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json")).unwrap();
        let controls = artifact["usage_featuring_controls"].as_array().unwrap();
        assert_eq!(controls.len(), 94);
        let mut positives = 0;
        for control in controls {
            let mut doc = parse_and_link(control["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
            // Match Pilot's explicitly supplied complete-generalization inputs.
            // The producer itself must never manufacture this completeness.
            for element in &mut doc.elements {
                if metaclass_conforms(&element.kind, "Type") { element.properties.insert("is_implied_included".into(), json!(true)); }
            }
            let feature = doc.elements.iter_mut().find(|e| e.properties.get("declared_name") == Some(&json!("f"))).unwrap();
            feature.kind = format!("SysML::{}", control["kind"].as_str().unwrap());
            feature.properties.insert("is_portion".into(), control["portion"].clone());
            feature.properties.insert("is_composite".into(), json!(false));
            // A contradictory stored field must not override the resolved getter.
            feature.properties.insert("is_variable".into(), json!(!control["variable"].as_bool().unwrap()));
            let id = feature.id.clone();
            let flags = doc.elements.iter().map(|e| (e.id.clone(), e.properties.get("is_implied_included").cloned())).collect::<Vec<_>>();
            assert_eq!(json!(may_time_vary(&doc, &id).unwrap()), control["variable"]);
            assert!(materialize_owning_type_featuring(&mut doc, &id, "usage-featuring").unwrap_or_else(|e| panic!("{}: {e}", control["kind"])));
            let relation = reference_targets(&doc, &id, "owned_type_featuring").unwrap()[0];
            let target = reference_targets(&doc, &relation.id, "featuring_type").unwrap()[0];
            assert_eq!(target.properties["declared_name"], control["target_name"]);
            assert_eq!(json!(target.properties.get("declared_name") == Some(&json!("Owner"))), control["target_is_owner"]);
            assert_eq!(json!(target.properties.get("owning_relationship") == Some(&json!(relation.id))), control["adopted"]);
            assert_eq!(relation.properties["is_implied"], control["implied"]);
            for (id, flag) in flags { assert_eq!(doc.elements.iter().find(|e| e.id == id).unwrap().properties.get("is_implied_included").cloned(), flag); }
            assert!(doc.elements.iter().filter(|e| e.id.starts_with("usage-featuring")).all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
            positives += usize::from(control["variable"] == true);
        }
        assert!(positives > 0 && positives < controls.len());
    }

    #[test]
    fn definition_variable_featuring_matches_independent_pilot() {
        let artifact: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json")).unwrap();
        let controls = artifact["variable_featuring_controls"].as_array().unwrap();
        assert_eq!(controls.len(), 5);
        for control in controls {
            let mut doc = parse_and_link(control["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
            let feature_id = doc.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("f"))).unwrap().id.clone();
            assert!(materialize_owning_type_featuring(&mut doc, &feature_id, "variable").unwrap(), "{}", control["source"]);
            let outer = reference_targets(&doc, &feature_id, "owned_type_featuring").unwrap()[0];
            assert_eq!(outer.properties["is_implied"], control["outer_implied"]);
            assert_eq!(json!(outer.properties["feature_of_type"] == feature_id), control["outer_source_is_feature"]);
            let result = reference_targets(&doc, &outer.id, "featuring_type").unwrap()[0];
            assert_eq!(result.properties["declared_name"], control["name"]);
            assert_eq!(json!(result.properties.get("owning_relationship") == Some(&json!(outer.id))), control["owned_by_outer"]);
            let rows = result.properties["owned_relationship"].as_array().unwrap().iter().filter_map(|id| {
                let relation = doc.elements.iter().find(|e| e.id == id.as_str().unwrap()).unwrap();
                match relation.kind.as_str() {
                    "SysML::Redefinition" => {
                        let target = reference_targets(&doc, &relation.id, "redefined_feature").unwrap()[0];
                        Some(json!({"kind":"Redefinition", "source_is_result":relation.properties["redefining_feature"] == result.id, "target":target.properties["declared_name"]}))
                    },
                    "SysML::TypeFeaturing" => {
                        let target = reference_targets(&doc, &relation.id, "featuring_type").unwrap()[0];
                        let membership_id = doc.elements.iter().find(|e| e.id == feature_id).unwrap().properties["owning_relationship"].as_str().unwrap();
                        let membership = doc.elements.iter().find(|e| e.id == membership_id).unwrap();
                        Some(json!({"kind":"TypeFeaturing", "source_is_result":relation.properties["feature_of_type"] == result.id, "target_is_owner":membership.properties["owning_related_element"] == target.id}))
                    },
                    _ => None,
                }
            }).collect::<Vec<_>>();
            assert_eq!(json!(rows), control["relations"]);
            let exported = export_sysml_abstract_syntax_value(&doc, Default::default()).unwrap();
            assert!(!exported.has_errors());
            let restored = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap().persistable_document().unwrap();
            for element in doc.elements.iter().filter(|e| e.id.starts_with("variable")) {
                let saved = restored.elements.iter().find(|e| e.id == element.id).unwrap();
                assert_eq!(element.kind, saved.kind);
                for (field, value) in &element.properties {
                    if field != "metadata" { assert_eq!(Some(value), saved.properties.get(field), "{}.{}", element.id, field); }
                }
            }
            assert!(doc.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
        }
    }

    #[test]
    fn definition_variable_featuring_missing_libraries_and_collisions_are_atomic() {
        for source in ["package P { class C { var feature f; } }",
            "standard library package Occurrences { class Occurrence; } package P { class C { var feature f; } }",
            "standard library package Occurrences { class Occurrence { feature snapshots; } } package P { class C { var feature f; } }"] {
            let mut doc = parse_and_link(source, SourceLanguage::Kerml).unwrap();
            let id = doc.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("f"))).unwrap().id.clone();
            let before = serde_json::to_value(&doc).unwrap();
            assert!(materialize_owning_type_featuring(&mut doc, &id, &id).is_err());
            assert_eq!(serde_json::to_value(&doc).unwrap(), before);
        }
    }

    #[test]
    fn definition_owning_type_featuring_matches_independent_pilot() {
        let artifact: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json")).unwrap();
        let controls = artifact["owning_type_featuring_controls"].as_array().unwrap();
        assert_eq!(controls.len(), 8);
        for control in controls {
            let mut doc = parse_and_link(control["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
            let feature_id = doc.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("f"))).unwrap().id.clone();
            if control["existing"] == true {
                if materialize_owning_type_featuring(&mut doc, &feature_id, "existing").unwrap() {
                    doc.elements.iter_mut().find(|e| e.id == "existing").unwrap().properties.insert("is_implied".into(), json!(false));
                }
            }
            let changed = materialize_owning_type_featuring(&mut doc, &feature_id, "generated").unwrap();
            assert_eq!(json!(changed), control["changed"], "{}", control["source"]);
            let relations = reference_targets(&doc, &feature_id, "owned_type_featuring").unwrap();
            let observed = relations.iter().map(|relation| {
                assert_eq!(relation.properties["owning_related_element"], feature_id);
                assert_eq!(relation.properties["feature_of_type"], feature_id);
                let target = reference_targets(&doc, &relation.id, "featuring_type").unwrap()[0];
                json!({"kind":relation.kind.rsplit("::").next().unwrap(), "target":target.properties["declared_name"], "implied":relation.properties["is_implied"]})
            }).collect::<Vec<_>>();
            assert_eq!(json!(observed), control["relations"]);
            let before = serde_json::to_value(&doc).unwrap();
            assert!(!materialize_owning_type_featuring(&mut doc, &feature_id, "repeated").unwrap());
            assert_eq!(serde_json::to_value(&doc).unwrap(), before);
            let exported = export_sysml_abstract_syntax_value(&doc, Default::default()).unwrap();
            assert!(!exported.has_errors());
            let restored = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap().persistable_document().unwrap();
            assert_eq!(reference_targets(&restored, &feature_id, "owned_type_featuring").unwrap().len(), observed.len());
            assert!(doc.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
        }
    }

    #[test]
    fn definition_owning_type_featuring_rejects_unqualified_branches_atomically() {
        for mutation in ["variable", "subclass", "collision", "missing_target", "snapshot"] {
            let mut doc = parse_and_link("package P { class C { feature f; } }", SourceLanguage::Kerml).unwrap();
            let feature = doc.elements.iter_mut().find(|e| e.properties.get("declared_name") == Some(&json!("f"))).unwrap();
            let id = feature.id.clone();
            if mutation == "variable" { feature.properties.insert("is_variable".into(), json!(true)); }
            if mutation == "subclass" { feature.kind = "SysML::Usage".into(); }
            if mutation == "snapshot" { feature.properties.insert("owned_type_featuring".into(), json!([])); }
            let before = serde_json::to_value(&doc).unwrap();
            let target = if mutation == "missing_target" { "absent" } else { &id };
            let relation = if mutation == "collision" { &id } else { "generated" };
            assert!(materialize_owning_type_featuring(&mut doc, target, relation).is_err(), "{mutation}");
            assert_eq!(serde_json::to_value(&doc).unwrap(), before);
        }
    }

    #[test]
    fn definition_binding_construction_matches_independent_pilot() {
        let artifact: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json")).unwrap();
        let controls = artifact["binding_construction_controls"].as_array().unwrap();
        assert_eq!(controls.len(), 15);
        for control in controls {
            let mut doc = KirDocument { metadata: Default::default(), elements: Vec::new() };
            for id in ["a", "b"] { append_feature_chain(&mut doc, id, &[]).unwrap(); }
            if control["ownership"] == "owned" {
                append_binding_connector(&mut doc, "existing", Some("a"), Some("b")).unwrap();
            }
            if control["ownership"] == "orphan" {
                for id in ["a", "b"] {
                    let membership = format!("orphan.{id}");
                    doc.elements.iter_mut().find(|e| e.id == id).unwrap().properties.insert("owning_relationship".into(), json!(membership));
                    doc.elements.push(mercurio_foundation::kir::KirElement { id: membership, kind: "SysML::FeatureMembership".into(), layer: 2,
                        properties: [("owned_related_element".into(), json!([id]))].into_iter().collect() });
                }
            }
            let prior = doc.elements.clone();
            let input = |i: usize| control["inputs"][i].as_str().filter(|id| !id.is_empty());
            append_binding_connector(&mut doc, "binding", input(0), input(1)).unwrap();
            let connector = doc.elements.iter().find(|e| e.id == "binding").unwrap();
            assert_eq!(connector.kind, format!("SysML::{}", control["kind"].as_str().unwrap()));
            let memberships = connector.properties["owned_relationship"].as_array().unwrap();
            assert_eq!(memberships.len(), 2);
            for (index, observed) in control["ends"].as_array().unwrap().iter().enumerate() {
                let get = |id: &str| doc.elements.iter().find(|e| e.id == id).unwrap();
                let membership = get(memberships[index].as_str().unwrap());
                assert_eq!(membership.kind, format!("SysML::{}", observed["membership_kind"].as_str().unwrap()));
                assert_eq!(membership.properties["owning_related_element"], "binding");
                let end = get(membership.properties["owned_related_element"][0].as_str().unwrap());
                assert_eq!(end.kind, format!("SysML::{}", observed["end_kind"].as_str().unwrap()));
                assert_eq!(end.properties["owning_relationship"], membership.id);
                assert_eq!(end.properties["is_end"], observed["is_end"]);
                let refs = end.properties["owned_relationship"].as_array().unwrap();
                assert_eq!(json!(refs.len()), observed["reference_count"]);
                if !refs.is_empty() {
                    let reference = get(refs[0].as_str().unwrap());
                    assert_eq!(reference.kind, format!("SysML::{}", observed["reference_kind"].as_str().unwrap()));
                    assert_eq!(reference.properties["owning_related_element"], end.id);
                    assert_eq!(reference.properties["referenced_feature"], observed["referenced"]);
                    let target = get(reference.properties["referenced_feature"].as_str().unwrap());
                    assert_eq!(json!(target.properties.get("owning_relationship") == Some(&json!(reference.id))), observed["adopted"]);
                }
            }
            if control["ownership"] == "owned" {
                for before in prior { assert_eq!(serde_json::to_value(&before).unwrap(), serde_json::to_value(doc.elements.iter().find(|e| e.id == before.id).unwrap()).unwrap()); }
            }
            assert!(doc.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
            let exported = export_sysml_abstract_syntax_value(&doc, Default::default()).unwrap();
            assert!(!exported.has_errors(), "{:?}", exported.diagnostics);
            let restored = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
            assert!(!restored.has_errors(), "{:?}", restored.diagnostics);
            let restored = restored.persistable_document().unwrap();
            for before in &doc.elements {
                let after = restored.elements.iter().find(|e| e.id == before.id).unwrap();
                for field in ["owned_relationship", "owned_related_element", "owning_relationship", "owning_related_element", "referenced_feature", "is_end"] {
                    assert_eq!(before.properties.get(field), after.properties.get(field), "{}.{}", before.id, field);
                }
            }
        }
    }

    #[test]
    fn definition_binding_construction_rolls_back_failed_adoption() {
        let mut doc = KirDocument { metadata: Default::default(), elements: Vec::new() };
        append_feature_chain(&mut doc, "a", &[]).unwrap();
        append_feature_chain(&mut doc, "collision.end.1", &[]).unwrap();
        for (id, target) in [("", None), ("a", Some("a")), ("collision", Some("a")), ("new", Some("missing"))] {
            let before = serde_json::to_value(&doc).unwrap();
            assert!(append_binding_connector(&mut doc, id, Some("a"), target).is_err());
            assert_eq!(serde_json::to_value(&doc).unwrap(), before);
        }
        append_binding_connector(&mut doc, "good", Some("a"), None).unwrap();
        let before = serde_json::to_value(&doc).unwrap();
        assert!(append_binding_connector(&mut doc, "wrong", Some("good.end.0.membership"), None).is_err());
        assert_eq!(serde_json::to_value(&doc).unwrap(), before);
    }

    #[test]
    fn definition_chain_queries_read_canonical_storage_and_reject_invalid_edges() {
        let mut doc = KirDocument { metadata: Default::default(), elements: Vec::new() };
        append_feature_chain(&mut doc, "a", &[]).unwrap();
        append_feature_chain(&mut doc, "chain", &["a", "a"]).unwrap();
        // Cached derived snapshots are never inputs to this service.
        doc.elements.iter_mut().find(|e| e.id == "chain").unwrap()
            .properties.insert("chaining_feature".into(), json!(["stale"]));
        assert_eq!(reference_targets(&doc, "chain", "chaining_feature").unwrap().len(), 2);
        doc.elements.iter_mut().find(|e| e.id == "chain.chain.1").unwrap()
            .properties.insert("chaining_feature".into(), json!("chain"));
        assert_eq!(reference_targets(&doc, "chain", "feature_target").unwrap()[0].id, "chain");
        for mutation in ["missing", "wrong_kind", "wrong_owner", "missing_relationships"] {
            let mut invalid = doc.clone();
            let edge = invalid.elements.iter_mut().find(|e| e.id == "chain.chain.0").unwrap();
            match mutation {
                "missing" => { edge.properties.insert("chaining_feature".into(), json!("absent")); },
                "wrong_kind" => { edge.properties.insert("chaining_feature".into(), json!("chain.chain.1")); },
                "wrong_owner" => { edge.properties.insert("owning_related_element".into(), json!("a")); },
                _ => { invalid.elements.iter_mut().find(|e| e.id == "chain").unwrap().properties.remove("owned_relationship"); },
            }
            for field in ["owned_feature_chaining", "chaining_feature", "feature_target"] {
                assert!(reference_targets(&invalid, "chain", field).is_err(), "{mutation}: {field}");
            }
        }
    }

    #[test]
    fn definition_chain_construction_matches_independent_pilot() {
        let artifact: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json")).unwrap();
        let controls = artifact["chain_construction_controls"].as_array().unwrap();
        assert_eq!(controls.len(), 5);
        for control in controls {
            let mut doc = KirDocument { metadata: Default::default(), elements: Vec::new() };
            for id in ["a", "b"] { append_feature_chain(&mut doc, id, &[]).unwrap(); }
            append_feature_chain(&mut doc, "pair", &["b", "a", "b"]).unwrap();
            append_feature_chain(&mut doc, "nested", &["a"]).unwrap();
            doc.elements.iter_mut().find(|e| e.id == "nested.chain.0").unwrap()
                .properties.insert("chaining_feature".into(), json!("pair"));
            let inputs = control["inputs"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>();
            append_feature_chain(&mut doc, "output", &inputs).unwrap();
            let owner = doc.elements.iter().find(|e| e.id == "output").unwrap();
            let targets = owner.properties["owned_relationship"].as_array().unwrap().iter().map(|id| {
                let edge = doc.elements.iter().find(|e| e.id == id.as_str().unwrap()).unwrap();
                assert_eq!(edge.properties["owning_related_element"], "output");
                edge.properties["chaining_feature"].clone()
            }).collect::<Vec<_>>();
            assert_eq!(json!(targets), control["targets"]);
            let chain = reference_targets(&doc, "output", "chaining_feature").unwrap();
            assert_eq!(json!(chain.iter().map(|e| &e.id).collect::<Vec<_>>()), control["chaining_feature"]);
            let owned = reference_targets(&doc, "output", "owned_feature_chaining").unwrap();
            assert_eq!(json!(owned.len()), control["owned_feature_chaining_count"]);
            let basic = reference_targets(&doc, "output", "feature_target").unwrap();
            assert_eq!(json!(basic[0].id), control["feature_target"]);
            let exported = export_sysml_abstract_syntax_value(&doc, Default::default()).unwrap();
            assert!(!exported.has_errors());
            let restored = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
            assert!(!restored.has_errors());
            assert!(restored.persistable_document().is_ok());
        }
    }

    #[test]
    fn definition_chain_construction_preserves_order_and_is_atomic() {
        let mut doc = KirDocument { metadata: Default::default(), elements: Vec::new() };
        append_feature_chain(&mut doc, "a", &[]).unwrap();
        append_feature_chain(&mut doc, "b", &[]).unwrap();
        append_feature_chain(&mut doc, "pair", &["b", "a", "b"]).unwrap();
        append_feature_chain(&mut doc, "expanded", &["a", "pair", "a"]).unwrap();
        let targets = |doc: &KirDocument, id: &str| -> Vec<String> {
            let owner = doc.elements.iter().find(|e| e.id == id).unwrap();
            owner.properties["owned_relationship"].as_array().unwrap().iter().map(|rid| {
                doc.elements.iter().find(|e| e.id == rid.as_str().unwrap()).unwrap()
                    .properties["chaining_feature"].as_str().unwrap().to_owned()
            }).collect()
        };
        assert_eq!(targets(&doc, "expanded"), ["a", "b", "a", "b", "a"]);
        // Directly retarget one edge to a chain: expansion must remain one-level.
        doc.elements.iter_mut().find(|e| e.id == "pair.chain.0").unwrap()
            .properties.insert("chaining_feature".into(), json!("expanded"));
        append_feature_chain(&mut doc, "nested", &["pair"]).unwrap();
        assert_eq!(targets(&doc, "nested"), ["expanded", "a", "b"]);
        for (id, inputs) in [("a", vec![]), ("", vec![]), ("bad", vec!["absent"]), ("bad", vec!["pair.chain.0"])] {
            let before = serde_json::to_value(&doc).unwrap();
            assert!(append_feature_chain(&mut doc, id, &inputs).is_err());
            assert_eq!(serde_json::to_value(&doc).unwrap(), before);
        }
        assert!(doc.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
    }

    #[test]
    fn definition_documents_publish_and_roundtrip_canonical_stored_models() {
        for (language, source) in [
            (SourceLanguage::Kerml, "package P { class A; feature x : A; }"),
            (SourceLanguage::Sysml, "package P { part def A; part x : A; }"),
            (SourceLanguage::Sysml, "package P { part x : Alias; alias Alias for A; part def A; }"),
            (SourceLanguage::Sysml, "package A { part def T; } package B { private import A::*; part x : T; }"),
        ] {
            let document = parse_and_link(source, language).unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(document.metadata["semantic_validation"], "not_assessed");
            assert!(document.elements.iter().any(|e| e.kind == "SysML::FeatureTyping"));
            let exported = export_sysml_abstract_syntax_value(&document, Default::default()).unwrap();
            assert!(!exported.has_errors(), "{:?}", exported.diagnostics);
            let report = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
            assert!(!report.has_errors(), "{:?}", report.diagnostics);
            let restored = report.persistable_document().unwrap();
            // Import adds source provenance; compare every canonical Ecore field,
            // containment order, identity and kind without that transport metadata.
            let snapshot = |doc: &KirDocument| doc.elements.iter().map(|e| {
                let mut properties = e.properties.clone();
                properties.remove("metadata");
                (e.id.clone(), (e.kind.clone(), properties))
            }).collect::<std::collections::BTreeMap<_,_>>();
            assert_eq!(snapshot(&document), snapshot(&restored));
            assert_eq!(serde_json::to_value(parse_and_link(source, language).unwrap()).unwrap(), serde_json::to_value(document).unwrap());
        }
    }

    #[test]
    fn definition_type_scopes_preserve_unimplemented_dependency_boundaries() {
        for source in [
            "package P { part def Base { part def Local; } part def Derived :> Base { part x : Local; } }",
            "package P { part def Base { part x; } part def Derived :> Base { part x :>> x; } }",
            "package P { part def Outer { private part def Local; } part x : Outer::Local; }",
            "package P { part def Outer { part x : Missing; } }",
        ] {
            assert!(matches!(parse_and_link(source, SourceLanguage::Sysml), Err(DefinitionDocumentError::ConstructionOrLinking(_))), "{source}");
        }
        let mut document = parse_and_link("package P { class A; feature x : A; }", SourceLanguage::Kerml).unwrap();
        let feature = document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("x"))).unwrap().id.clone();
        assert!(reference_targets(&document, &feature, "type").is_err(), "Derived Feature.type is not implemented by a stored-reference query");
        assert!(reference_targets(&document, &feature, "declared_name").is_err());
        assert!(reference_targets(&document, "missing", "source").is_err());
        document.elements.push(document.elements[0].clone());
        assert!(reference_targets(&document, &feature, "type").is_err());
    }

    #[test]
    fn definition_metadata_absence_drives_inherited_source_linking() {
        let library="standard library package Occurrences { class Occurrence; } ";
        let model="package P { class Base { doc /* ordinary documentation */ class Local; } class Derived :> Base { feature x : Local; } }";
        for source in [format!("{library}{model}"),format!("{model}{library}")] {
            let document=parse_and_link(&source,SourceLanguage::Kerml).unwrap();
            let restored:KirDocument=serde_json::from_str(&serde_json::to_string(&document).unwrap()).unwrap();
            for doc in [&document,&restored] {
                let feature=doc.elements.iter().find(|e|e.properties.get("declared_name")==Some(&json!("x"))).unwrap();
                let relations=reference_targets(doc,&feature.id,"owned_relationship").unwrap();
                let typing=relations.iter().find(|r|r.kind.ends_with("FeatureTyping")).unwrap();
                let types=reference_targets(doc,&typing.id,"type").unwrap();
                assert_eq!(types.len(),1);
                assert_eq!(types[0].properties["declared_name"],"Local");
                assert_eq!(doc.metadata["semantic_validation"],"not_assessed");
            }
        }
    }

    #[test]
    fn definition_plain_inheritance_preserves_semantic_dependency_boundaries() {
        let library = "standard library package Occurrences { class Occurrence; } ";
        for source in [
            "package P { class Base { class Local; } class Derived :> Base { feature x : Local; } }".to_owned(),
            "library package Occurrences { class Occurrence; } package P { class Base { class Local; } class Derived :> Base { feature x : Local; } }".to_owned(),
            "package Wrapper { standard library package Occurrences { class Occurrence; } } package P { class Base { class Local; } class Derived :> Base { feature x : Local; } }".to_owned(),
            "standard library package Occurrences { datatype Occurrence; } package P { class Base { class Local; } class Derived :> Base { feature x : Local; } }".to_owned(),
            format!("{library}{library}package P {{ class Base {{ class Local; }} class Derived :> Base {{ feature x : Local; }} }}"),
            format!("{library}package P {{ class Base {{ private class Local; }} class Derived :> Base {{ feature x : Local; }} }}"),
            format!("{library}package P {{ class Base {{ protected class Local; }} class Derived :> Base; feature x : Derived::Local; }}"),
            format!("{library}package P {{ class First {{ class Local; }} class Second {{ class Local; }} class Derived :> First, Second {{ feature x : Local; }} }}"),
            format!("{library}package P {{ class A :> B; class B :> A; class Derived :> A {{ feature x : Missing; }} }}"),
        ] {
            assert!(matches!(parse_and_link(&source, SourceLanguage::Kerml), Err(DefinitionDocumentError::ConstructionOrLinking(_))), "{source}");
        }
        // Even successful lookup does not claim to materialize all implied
        // relationships or to validate a complete language model.
        let source = format!("{library}package P {{ class Base {{ class Local; }} class Derived :> Base {{ feature x : Local; }} }}");
        let document = parse_and_link(&source, SourceLanguage::Kerml).unwrap();
        assert!(document.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
        assert!(document.elements.iter().all(|e| e.properties.get("is_implied") != Some(&json!(true))));
        assert_eq!(document.metadata["semantic_validation"], "not_assessed");
    }

    #[test]
    fn definition_default_dependencies_do_not_fall_back_or_hide_inherited_ends() {
        let missing_binary = "standard library package Connections { connection def Connection { part def Local; } } package P { connection def C { end part a; end part b; part x : Local; } }";
        let error = parse_and_link(missing_binary, SourceLanguage::Sysml).unwrap_err();
        assert!(error.to_string().contains("Connections::BinaryConnection"), "{error}");
        let inherited_flow_ends = "standard library package Flows { flow def MessageAction { item def Local; } flow def Message; } package P { flow def Base { end item a; end item b; } flow def Derived :> Base { item x : Local; } }";
        let error = parse_and_link(inherited_flow_ends, SourceLanguage::Sysml).unwrap_err();
        assert!(error.to_string().contains("FlowDefinition default selection requires inherited flowEnd semantics"), "{error}");
    }

    #[test]
    fn definition_redefinition_scopes_reject_inaccessible_and_missing_targets() {
        let library = "standard library package Occurrences { class Occurrence; } ";
        for source in [
            "package P { class Base { private feature x; } class Derived :> Base { feature y redefines x; } }",
            "package P { class Base; class Derived :> Base { feature x redefines x; } }",
            "package P { class Base; class Derived :> Base { feature local; feature y redefines local; } }",
            "package P { class Base { feature x; } class Derived :> Base { feature y redefines Missing; } }",
        ] {
            let failure = parse_and_link(&format!("{library}{source}"), SourceLanguage::Kerml).unwrap_err();
            assert!(matches!(failure, DefinitionDocumentError::ConstructionOrLinking(_)), "{source}: {failure}");
        }
    }

    #[test]
    fn definition_documents_reject_missing_ambiguous_and_cyclic_links() {
        for source in [
            "package P { part x : Missing; }",
            "package P { part def A; part def A; part x : A; }",
            "package P { alias A for B; alias B for A; part x : A; }",
            "package P { part def T; part x : External::T; }",
        ] {
            assert!(matches!(parse_and_link(source, SourceLanguage::Sysml), Err(DefinitionDocumentError::ConstructionOrLinking(_))), "{source}");
        }
        assert!(matches!(parse_and_link("package P {", SourceLanguage::Sysml), Err(DefinitionDocumentError::Syntax(_))));
    }
}

#[cfg(test)]
#[path = "definition_document_pilot_tests.rs"]
mod pilot_tests;

/// Select the owned cross-feature from canonical imported membership contracts.
/// This structural query does not compute cross-feature typing or transformation.
pub fn owned_cross_feature(document: &KirDocument, owner_id: &str) -> Result<Option<String>, DefinitionDocumentError> {
    let owner = document.elements.iter().find(|e| e.id == owner_id)
        .ok_or_else(|| DefinitionDocumentError::ConstructionOrLinking("cross-feature owner is absent".into()))?;
    crate::language_frontend::lowering::emit::definition_owned_cross_feature(&document.elements, owner)
        .map(|selected| selected.map(|feature| feature.id.clone()))
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Materialize binary owned-end crossing atomically, with a fresh chain and
/// CrossSubsetting. Existing crossing suppresses insertion. Nonbinary featuring,
/// cross-feature typing, and complete transformation remain separate stages.
pub fn materialize_binary_crossing(document: &mut KirDocument, owner_id: &str, prefix: &str)
    -> Result<bool, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_binary_crossing(&mut document.elements, owner_id, prefix)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Atomically specialize an owned cross-feature by its owning end's resolved
/// types and redefined ends' cross-features. This stage does not assert complete
/// transformation or compute missing owner/generalization dependencies.
pub fn materialize_owned_cross_specialization(document: &mut KirDocument, cross_id: &str, prefix: &str)
    -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_owned_cross_specialization(&mut document.elements, cross_id, prefix)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Materialize binary owned-cross featuring from the other effective end's
/// resolved types. Nonbinary products and unresolved type providers reject.
pub fn materialize_binary_cross_featuring(document: &mut KirDocument, cross_id: &str, prefix: &str)
    -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_binary_cross_featuring(&mut document.elements, cross_id, prefix)
        .map_err(|error| DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

#[cfg(test)]
mod source_set_tests {
    use super::*;
    fn source<'a>(uri: &'a str, text: &'a str, language: SourceLanguage) -> DefinitionSource<'a> {
        DefinitionSource { uri, text, language }
    }
    #[test]
    fn source_sets_link_forward_imports_aliases_and_mixed_languages() {
        let cases = [
            (SourceLanguage::Kerml, SourceLanguage::Kerml, "package A { classifier T; }", "package B { feature x : A::T; }"),
            (SourceLanguage::Sysml, SourceLanguage::Sysml, "package A { part def T; }", "package B { private import A::*; part x : T; }"),
            (SourceLanguage::Sysml, SourceLanguage::Kerml, "package A { classifier T; alias U for T; }", "package B { part x : A::U; }"),
        ];
        for (language, provider_language, provider, consumer) in cases {
            let resources = [source("memory:/consumer.sysml", consumer, language), source("memory:/provider.kerml", provider, provider_language)];
            let doc = parse_and_link_sources(&resources).unwrap();
            let reversed = parse_and_link_sources(&[source(resources[1].uri,provider,provider_language),source(resources[0].uri,consumer,language)]).unwrap();
            let models = [&doc, &reversed];
            for model in models {
                assert_eq!(model.metadata["semantic_validation"], "not_assessed");
                assert_eq!(model.metadata["source_resources"].as_array().unwrap().len(), 2);
                let restored: KirDocument = serde_json::from_str(&serde_json::to_string(model).unwrap()).unwrap();
                assert_eq!(serde_json::to_value(model).unwrap(), serde_json::to_value(restored).unwrap());
            }
            let by_id = |document: &KirDocument| document.elements.iter().map(|e| (e.id.clone(), serde_json::to_value(e).unwrap())).collect::<std::collections::BTreeMap<_,_>>();
            assert_eq!(by_id(&doc),by_id(&reversed));
            assert!(parse_and_link_sources(&resources[..1]).is_err());
        }
    }
    #[test]
    fn source_sets_match_cached_pilot_resource_links() {
        let controls: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/resource-link-pilot-controls.json")).unwrap();
        assert_eq!(controls["cases"].as_array().unwrap().len(),12);
        for case in controls["cases"].as_array().unwrap() {
            let units = case["units"].as_array().unwrap();
            let sources = units.iter().map(|u| DefinitionSource {
                uri:u["uri"].as_str().unwrap(), text:u["source"].as_str().unwrap(),
                language:if u["language"]=="kerml" {SourceLanguage::Kerml} else {SourceLanguage::Sysml},
            }).collect::<Vec<_>>();
            let result = parse_and_link_sources(&sources);
            if case["name"].as_str().unwrap().starts_with("duplicate") {
                // Pilot selects the first exported package in these two controls.
                // Native rejects ambiguity; this disagreement is still unqualified.
                assert!(result.is_err());continue;
            }
            let observations = case["links"].as_array().unwrap();
            if observations.iter().any(|x|x["resolved"]==false) {
                assert!(matches!(result,Err(DefinitionDocumentError::ConstructionOrLinking(_))),"{}",case["name"]);continue;
            }
            let document = result.unwrap();
            let restored: KirDocument = serde_json::from_str(&serde_json::to_string(&document).unwrap()).unwrap();
            for model in [&document,&restored] {
                for observed in observations {
                    let relation = model.elements.iter().find(|e|e.kind=="SysML::FeatureTyping").unwrap();
                    let targets = crate::language_frontend::lowering::emit::definition_reference_targets(&model.elements,relation,"type").unwrap();
                    assert_eq!(targets.len(),1);
                    let target=targets[0];
                    assert_eq!(target.kind,format!("SysML::{}",observed["target_kind"].as_str().unwrap()));
                    let roots=model.metadata["source_resources"].as_array().unwrap();
                    let resource=roots.iter().find(|r|target.id.starts_with(&format!("{}.",r["root_id"].as_str().unwrap()))).unwrap();
                    assert_eq!(resource["uri"],observed["target_uri"]);
                    let mut path=Vec::new();let mut cursor=Some(target);
                    while let Some(element)=cursor {
                        if let Some(name)=element.properties.get("declared_name").and_then(serde_json::Value::as_str) {path.push(name.to_owned());}
                        cursor=element.properties.get("owning_relationship").and_then(serde_json::Value::as_str)
                            .and_then(|id|model.elements.iter().find(|e|e.id==id))
                            .and_then(|r|r.properties.get("owning_related_element").and_then(serde_json::Value::as_str))
                            .and_then(|id|model.elements.iter().find(|e|e.id==id));
                    }
                    path.reverse();assert_eq!(json!(path),observed["target_path"]);
                }
            }
        }
    }
    #[test]
    fn definition_structure_inspection_preserves_unresolved_declaration_bodies() {
        let inputs = [source("prototype.kerml", "standard library package P { function Prototype { in x : Missing::Value; return result : Missing::Value[1]; feature nested { feature child; } } }", SourceLanguage::Kerml)];
        let inspection = inspect_source_structure(&inputs).unwrap();
        assert!(parse_and_link_sources(&inputs).is_err(), "diagnostic inspection cannot satisfy unresolved linking");
        for name in ["Prototype", "x", "result", "nested", "child"] {
            assert!(inspection.elements().iter().any(|e| e.properties.get("declared_name") == Some(&json!(name))), "declaration body lost: {name}");
        }
        assert!(inspection.elements().iter().any(|e| e.kind.ends_with("::MultiplicityRange")));
        assert!(inspection.elements().iter().any(|e| e.kind.ends_with("::LiteralInteger") && e.properties.get("value") == Some(&json!(1))));
        assert_eq!(inspection.pending_references().iter().filter(|r| r.spelling == "Missing::Value").count(), 2);
        for request in inspection.pending_references() {
            let owner = inspection.elements().iter().find(|e| e.id == request.owner_id).unwrap();
            assert_eq!(owner.kind, request.owner_kind);
            assert!(!owner.properties.contains_key(&request.field), "pending reference must not be fabricated");
            assert!(!request.feature_id.is_empty() && !request.ecore_target.is_empty() && !request.grammar_target.is_empty());
            assert!(request.span.is_some());
        }
        assert!(inspection.elements().iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
        let before_query = serde_json::to_value(&inspection).unwrap();
        let prototype = inspection.elements().iter().find(|e| e.properties.get("declared_name") == Some(&json!("Prototype"))).unwrap();
        let result = inspection.reference_targets(&prototype.id, "result").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].properties["declared_name"], json!("result"));
        assert_eq!(result[0].properties["direction"], json!("out"));
        assert!(inspection.reference_targets(&result[0].id, "type").is_err(), "missing typing cannot become an empty successful result");
        assert_eq!(before_query, serde_json::to_value(&inspection).unwrap(), "read-only getters must not resolve fields or complete Types");
        let value = serde_json::to_value(&inspection).unwrap();
        assert_eq!(value["status"], "unlinked_inspection");
        assert_eq!(value["linking"], "not_run");
        assert_eq!(value["semantic_validation"], "not_assessed");
        assert!(value.get("elements").is_none(), "diagnostic schema must not resemble a publishable KirDocument");
        assert!(value.get("constructed_elements").is_some());
    }

    #[test]
    fn definition_structure_inspection_reports_typed_native_prerequisites_without_mutation() {
        let inputs = [source("typed.kerml", r#"
            standard library package Base { feature things; }
            standard library package Performances { function Evaluation { return inherited; } }
            package P { function F specializes Performances::Evaluation { return result : Missing::T; } }
        "#, SourceLanguage::Kerml)];
        let inspection = inspect_source_structure(&inputs).unwrap();
        let before = serde_json::to_value(&inspection).unwrap();
        let result = inspection.elements().iter().find(|e| e.properties.get("declared_name") == Some(&json!("result"))).unwrap();
        let failure = inspection.reference_targets_with_dependencies(&result.id, "type").unwrap_err();
        let DefinitionReferenceQueryError::Required { prerequisite: DefinitionQueryPrerequisite::ReadField { owner_id, field } } = &failure
            else { panic!("pending syntax must stay a typed native dependency: {failure:?}"); };
        assert!(inspection.pending_references().iter().any(|r| &r.owner_id == owner_id && &r.field == field), "requirement must name an actual pending source field");
        assert_eq!(field, "type");
        let encoded = serde_json::to_value(&failure).unwrap();
        assert_eq!(encoded["status"], "required"); assert_eq!(encoded["prerequisite"]["kind"], "read_field");
        assert_eq!(encoded["prerequisite"]["owner_id"], json!(owner_id));
        assert_eq!(inspection.reference_targets_with_dependencies(&result.id, "type").unwrap_err(), failure);
        assert!(matches!(inspection.reference_targets_with_dependencies(&result.id, "unknown"), Err(DefinitionReferenceQueryError::Rejected { .. })));
        assert!(matches!(inspection.reference_targets_with_dependencies("absent", "type"), Err(DefinitionReferenceQueryError::Rejected { .. })));
        assert_eq!(before, serde_json::to_value(&inspection).unwrap());
        assert!(inspection.elements().iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
        assert!(parse_and_link_sources(&inputs).is_err(), "inspection dependencies do not authorize publication");
    }


    #[test]
    fn definition_provider_plan_native_scope_preserves_inspection_and_candidate_boundaries() {
        let sources = [
            DefinitionSource {uri:"library.kerml",text:"package L { class A; }",language:SourceLanguage::Kerml},
            DefinitionSource {uri:"source.kerml",text:"package P { feature selected : L::A; feature ignored : Missing; }",language:SourceLanguage::Kerml},
        ];
        let original=inspect_source_structure(&sources).unwrap();
        let selected=original.elements().iter().find(|e|e.properties.get("declared_name")==Some(&json!("selected"))).unwrap();
        let port=original.pending_references().iter().find(|p|
            original.elements().iter().find(|e|e.id==p.owner_id).unwrap().properties.get("owning_related_element")==Some(&json!(selected.id))).unwrap();
        let roots=[DefinitionQueryPrerequisite::ReadField {owner_id:port.owner_id.clone(),field:port.field.clone()}];
        let observed=std::cell::RefCell::new(Vec::new());
        let linked=inspect_source_dependencies_traced(&sources,&roots,|_,_|{},|event| {
            if event.phase==DefinitionConstructionPhase::LinkFieldCommitted {observed.borrow_mut().push((event.owner_id.to_owned(),event.field.unwrap_or("").to_owned()));}
        }).unwrap();
        assert_eq!(linked.elements().len(),original.elements().len(),"all resources preserved");
        assert_eq!(linked.pending_references().len()+1,original.pending_references().len());
        assert_eq!(*observed.borrow(),vec![(port.owner_id.clone(),port.field.clone())]);
        let target=linked.reference_targets(&port.owner_id,&port.field).unwrap();
        assert_eq!(target.len(),1);assert_eq!(target[0].properties["declared_name"],"A");
        let encoded=serde_json::to_value(&linked).unwrap();
        assert_eq!(encoded["status"],"dependency_inspection");
        assert_eq!(encoded["linking"],"requested_plan_completed");
        assert_eq!(encoded["semantic_validation"],"not_assessed");
        assert_eq!(encoded["transformation_completion"],"not_assessed");
        assert!(linked.elements().iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
        assert!(parse_and_link_sources(&sources).is_err(),"default candidate still rejects unresolved ignored source");
        for requirements in [vec![],vec![roots[0].clone(),DefinitionQueryPrerequisite::ReadField {owner_id:"foreign".into(),field:"type".into()}]] {
            assert!(inspect_source_dependencies(&sources,&requirements).is_err());
        }
    }

    #[test]
    fn definition_structure_inspection_matches_production_construction_and_resource_identity() {
        let inputs = [source("provider.kerml", "package A { classifier T; }", SourceLanguage::Kerml),
            source("consumer.kerml", "package B { feature x : A::T; }", SourceLanguage::Kerml)];
        let inspection = inspect_source_structure(&inputs).unwrap();
        let linked = parse_and_link_sources(&inputs).unwrap();
        let snapshot = inspection.elements().iter().map(|e| (e.id.clone(), serde_json::to_value(e).unwrap())).collect::<std::collections::BTreeMap<_, _>>();
        let mut linked_storage = linked.elements.iter().map(|e| (e.id.clone(), serde_json::to_value(e).unwrap())).collect::<std::collections::BTreeMap<_, _>>();
        for pending in inspection.pending_references() {
            linked_storage.get_mut(&pending.owner_id).unwrap()["properties"].as_object_mut().unwrap().remove(&pending.field);
        }
        assert_eq!(snapshot, linked_storage, "inspection and linker must share source construction and postprocessing");
        let reverse = inspect_source_structure(&[source(inputs[1].uri, inputs[1].text, inputs[1].language), source(inputs[0].uri, inputs[0].text, inputs[0].language)]).unwrap();
        let reverse_storage = reverse.elements().iter().map(|e| (e.id.clone(), serde_json::to_value(e).unwrap())).collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(snapshot, reverse_storage);
        let mut references = inspection.pending_references().iter().map(|r| serde_json::to_string(r).unwrap()).collect::<Vec<_>>();
        let mut reversed = reverse.pending_references().iter().map(|r| serde_json::to_string(r).unwrap()).collect::<Vec<_>>();
        references.sort(); reversed.sort(); assert_eq!(references, reversed);
    }

    #[test]
    fn definition_structure_inspection_rejects_source_errors_without_partial_result() {
        assert!(matches!(inspect_source_structure(&[]), Err(DefinitionDocumentError::Artifact(_))));
        for uri in ["", " ", "bad\0uri"] {
            assert!(matches!(inspect_source_structure(&[source(uri, "package P;", SourceLanguage::Kerml)]), Err(DefinitionDocumentError::Artifact(_))));
        }
        assert!(matches!(inspect_source_structure(&[source("same", "package P;", SourceLanguage::Kerml), source("same", "package Q;", SourceLanguage::Kerml)]), Err(DefinitionDocumentError::Artifact(_))));
        let inputs = [source("valid.kerml", "package P;", SourceLanguage::Kerml), source("invalid.kerml", "package Q { part def X; }", SourceLanguage::Kerml)];
        let error = inspect_source_structure(&inputs).unwrap_err();
        assert!(matches!(error, DefinitionDocumentError::Syntax(_)) && error.to_string().contains("invalid.kerml"));
    }

    #[test]
    fn source_sets_reject_identity_and_grammar_errors_before_publication() {
        assert!(matches!(parse_and_link_sources(&[]), Err(DefinitionDocumentError::Artifact(_))));
        for uri in ["", " ", "bad\0uri"] {
            assert!(matches!(parse_and_link_sources(&[source(uri,"package P;",SourceLanguage::Kerml)]), Err(DefinitionDocumentError::Artifact(_))));
        }
        assert!(matches!(parse_and_link_sources(&[source("same","package P;",SourceLanguage::Kerml),source("same","package Q;",SourceLanguage::Sysml)]),Err(DefinitionDocumentError::Artifact(_))));
        let error = parse_and_link_sources(&[source("invalid.kerml","package P { part def X; }",SourceLanguage::Kerml)]).unwrap_err();
        assert!(matches!(error,DefinitionDocumentError::Syntax(_)));assert!(error.to_string().contains("invalid.kerml"));
    }
    #[test]
    fn source_sets_preserve_privacy_and_reject_ambiguous_exports() {
        for provider in ["private package A { classifier T; }", "package A { private classifier T; }"] {
            assert!(parse_and_link_sources(&[source("provider",provider,SourceLanguage::Kerml),source("consumer","package B { feature x : A::T; }",SourceLanguage::Kerml)]).is_err());
        }
        let duplicates = [source("one","package A { classifier T; }",SourceLanguage::Kerml),source("two","package A { classifier T; }",SourceLanguage::Kerml),source("three","package B { feature x : A::T; }",SourceLanguage::Kerml)];
        assert!(parse_and_link_sources(&duplicates).is_err());
    }
}

/// Insert the definition-selected transition-link redefinition on a canonical
/// ReferenceUsage. This does not complete defaults, variability or transformation.
pub fn materialize_transition_link_redefinition(document: &mut KirDocument, feature: &str, identity_prefix: &str)
    -> Result<usize, DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_transition_link_redefinition(&mut document.elements,feature,identity_prefix)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Read the resolved variation contribution without claiming complete Usage defaults.
pub fn variation_contributions(document: &KirDocument, usage: &str) -> Result<Vec<(String,String)>,DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_variation_contributions(&document.elements,usage)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}
/// Insert only variation typing/subsetting; remaining Usage transformation stays open.
pub fn materialize_variation_typing(document: &mut KirDocument, usage: &str, identity_prefix: &str) -> Result<usize,DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_variation_typing(&mut document.elements,usage,identity_prefix)
        .map_err(|error|DefinitionDocumentError::ConstructionOrLinking(format!("{error:?}")))
}

/// Construct the source/link/member graph and execute the reviewed fresh binding transaction.
/// Full endpoint/Usage transformation and semantic validation remain separate.
pub fn materialize_transition_members(document:&mut KirDocument,owner:&str,identity_prefix:&str)
    -> Result<usize,DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_transition_members(&mut document.elements,owner,identity_prefix)
        .map_err(|e|DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Complete one fresh fixed transition-link ReferenceUsage against supplied
/// complete owner/library inputs. Variable and nonfresh receivers are rejected.
pub fn complete_fresh_transition_link(document:&mut KirDocument,feature:&str,prefix:&str)
    -> Result<(),DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_complete_fresh_transition_link(&mut document.elements,feature,prefix)
        .map_err(|e|DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Atomically construct transition members/bindings and complete newly created
/// fixed links. Native owner lifecycle and actual resources remain separate.
pub fn materialize_transition_members_with_fixed_links(document:&mut KirDocument,owner:&str,prefix:&str)
    -> Result<usize,DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_materialize_transition_members_with_fixed_links(&mut document.elements,owner,prefix)
        .map_err(|e|DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Complete fixed untyped portion Transitions under assessed Class/Action/State owners through
/// explicit featuring, additional-member/binding, default and fresh-link stages.
/// Member/source/role/library inputs must already be complete. State/Action
/// composite contexts, variable links and production scheduler integration remain separate.
pub fn complete_fixed_transition_owner(document:&mut KirDocument,owner:&str,prefix:&str)->Result<(),DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_complete_fixed_transition_owner(&mut document.elements,owner,prefix)
        .map_err(|e|DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

/// Execute assessed receiver and cold child stored stages under admitted
/// Class/Action/State ownership. Canonical role fixtures, supplied inputs and fixed portion bounds
/// remain required; effective-end constraints and actual resources are separate.
pub fn complete_fixed_transition_stored_tree(document:&mut KirDocument,owner:&str,prefix:&str)->Result<(),DefinitionDocumentError> {
    crate::language_frontend::lowering::emit::definition_complete_fixed_transition_stored_tree(&mut document.elements,owner,prefix)
        .map_err(|e|DefinitionDocumentError::ConstructionOrLinking(format!("{e:?}")))
}

#[cfg(test)]
#[path = "definition_document_query_batch_tests.rs"]
mod query_batch_tests;

#[cfg(test)]
#[path = "definition_document_projection_tests.rs"]
mod projection_tests;

#[cfg(test)]
#[path = "definition_document_provider_completion_tests.rs"]
mod provider_completion_tests;

#[cfg(test)]
#[path = "definition_document_shared_services_tests.rs"]
mod shared_services_tests;

#[cfg(test)]
#[path = "definition_document_fresh_structure_tests.rs"]
mod definition_fresh_structure_tests;

#[cfg(test)]
#[path = "definition_document_cached_dependencies_tests.rs"]
mod cached_dependencies_tests;
