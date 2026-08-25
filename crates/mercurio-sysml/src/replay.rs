//! DA-11 Tier-1 authoring-parity replay harness (Track C, C2).
//!
//! Proves that models created through the semantic-mutation action space are
//! oracle-equivalent to their text-authored references:
//!
//! 1. [`derive_gestures`] compiles nothing — it walks the *parsed authoring
//!    model* of a reference project and mechanically emits the ordered
//!    [`SemanticMutation`] operations that recreate it (packages → definitions
//!    → usages → attributes/values → relationships). Constructs the action
//!    space cannot express become `expect: Blocked` steps carrying the
//!    construct name — "blocked is an answer" — and are recorded in the
//!    coverage ledger.
//! 2. [`replay_gesture_script`] replays the script into an empty workspace
//!    through the production proposal pipeline: per step it rebuilds the
//!    project from the current file map (statelessness is mandatory — a stale
//!    [`MutationContext`] trips `StaleWorkspaceRevision`), runs
//!    `sysml_mutation_feasibility_service().check`, applies via
//!    `apply_checked_plan`, and merges `edited_files`.
//! 3. [`run_authoring_parity`] compares the replayed compile against the
//!    reference compile with the C1 equivalence oracle
//!    (`mercurio_core::kir_canonical`).
//!
//! Blocked-construct exclusion is principled: the deriver produces, alongside
//! the script, an *expressible reference* — the reference authoring tree with
//! every blocked declaration (or blocked facet) pruned, re-rendered through
//! the canonical printer. When a model has blocked steps the equivalence
//! comparison runs against that pruned reference, so blocked elements are
//! excluded from both sides symmetrically (the replay never created them; the
//! reference no longer contains them) and counted in the ledger. When nothing
//! is blocked the comparison runs against the original text-authored compile.
//!
//! Deriver design decisions (documented because they are load-bearing):
//!
//! - Usage types and specialization lists are emitted as follow-up
//!   `SetAttribute` operations (`type`, `specializes`, ...) with the names
//!   exactly as written in the source, rather than inline on
//!   `AddUsage`/`AddDefinition`. Inline references are existence-checked
//!   against the project by the feasibility service, and standard-library
//!   targets (`ScalarValues::Real`, `AnalysisCase`, ...) never exist in the
//!   replayed project's own files — they resolve only at compile time. The
//!   `SetAttribute` path is the same first-class mutation the properties
//!   panel uses and reproduces the source text verbatim.
//! - Imports are expressed with `SetAttribute { attribute: "imports" }` on
//!   the owning package (the action space has no dedicated import mutation).
//! - The deriver consults the same legality service the feasibility pipeline
//!   uses: containments and attribute writes the semantic capability profile
//!   refuses (`transition_source` on transitions, `multiplicity` on parts,
//!   `decide` in action definitions, ...) become Blocked steps up front
//!   instead of doomed operations, so the ledger doubles as the DA-7 gesture
//!   backlog.
//! - State transitions are blocked wholesale: the capability profile refuses
//!   the `transition_source` / `transition_target` / `trigger` writes that
//!   would wire them up.
//! - Anonymous usages (`assume constraint = ...`, `assert constraint`,
//!   bodies of `do` actions, ...) are blocked: `AddUsage` requires a declared
//!   name, and inventing one would change the model.
//! - Multi-line doc bodies are blocked: their interior indentation cannot
//!   round-trip the canonical printer byte-exactly, and a doc whose text
//!   drifts is non-equivalent output.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use mercurio_core::authoring::{
    AuthoringModule, AuthoringProject, Declaration, Definition, Package, QualifiedName, Usage,
    textual_model_authoring_render_profile,
};
use mercurio_core::kir_canonical::{KirEquivalenceReport, kir_equivalence_report};
use mercurio_core::{
    ElementRef, FeasibilityStatus, MutationContext, MutationFeasibilityService, MutationProposal,
    SemanticExpression, SemanticLegalityStatus, SemanticMutation,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::authoring::load_authoring_project_from_sysml;
use crate::mutation::{
    SysmlSemanticLegalityService, sysml_mutation_feasibility_service,
    sysml_semantic_legality_service,
};
use crate::parser::{compile_sysml_text_with_context, parse_sysml, shared_sysml_baseline};
use crate::semantic_profile::sysml_field_specs;
use crate::{Diagnostic, KirDocument};

pub const GESTURE_SCRIPT_SCHEMA_VERSION: &str = "mercurio.gesture_script.v1";
pub const AUTHORING_PARITY_LEDGER_SCHEMA_VERSION: &str = "mercurio.authoring_parity_ledger.v1";

const NOT_EXPRESSIBLE: &str = "not expressible as semantic mutations";

// --- gesture script ---------------------------------------------------------

/// An ordered, replayable list of semantic-mutation gestures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GestureScript {
    pub schema_version: String,
    /// Human-readable model label (for corpus runs: the model's relative path).
    pub model: String,
    pub steps: Vec<GestureStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GestureStep {
    pub intent: String,
    /// Dot-qualified name of the element this step concerns, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
    /// Operations as plain serde `SemanticMutation` JSON. Empty for derived
    /// `Blocked` steps (there is nothing the action space could run).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<SemanticMutation>,
    pub expect: GestureExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum GestureExpectation {
    Applied,
    Blocked { construct: String },
}

// --- replay outcome ---------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayReport {
    /// The workspace files after the last applied step.
    pub final_files: BTreeMap<String, String>,
    pub applied: Vec<AppliedGesture>,
    pub blocked: Vec<BlockedConstruct>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliedGesture {
    pub intent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
    pub operations: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedConstruct {
    pub construct: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
    pub intent: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ReplayError {
    /// A step's workspace files no longer parse into an authoring project.
    Load { step: usize, message: String },
    /// A step expected `Applied` but the feasibility service did not allow it
    /// (or expected `Blocked` but the pipeline applied it).
    ExpectationMismatch {
        step: usize,
        intent: String,
        expected: GestureExpectation,
        status: String,
        reasons: Vec<String>,
    },
    /// `apply_checked_plan` failed after a successful check.
    Apply { step: usize, message: String },
    /// The feasibility report allowed the step but carried no plan.
    MissingPlan { step: usize },
    /// A reference or replayed compile failed.
    Compile { context: String, message: String },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load { step, message } => {
                write!(f, "step {step}: workspace files failed to load: {message}")
            }
            Self::ExpectationMismatch {
                step,
                intent,
                expected,
                status,
                reasons,
            } => write!(
                f,
                "step {step} (`{intent}`) expected {expected:?} but feasibility returned {status}: {}",
                reasons.join("; ")
            ),
            Self::Apply { step, message } => {
                write!(f, "step {step}: apply_checked_plan failed: {message}")
            }
            Self::MissingPlan { step } => {
                write!(
                    f,
                    "step {step}: feasibility allowed the step but returned no plan"
                )
            }
            Self::Compile { context, message } => {
                write!(f, "compile failed ({context}): {message}")
            }
        }
    }
}

impl std::error::Error for ReplayError {}

// --- replay engine ----------------------------------------------------------

/// Replay a gesture script into an empty workspace through the production
/// check → apply pipeline. Every step rebuilds the project from the current
/// file map and proposes against the fresh workspace revision.
pub fn replay_gesture_script(script: &GestureScript) -> Result<ReplayReport, ReplayError> {
    let service = sysml_mutation_feasibility_service();
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    let mut applied = Vec::new();
    let mut blocked = Vec::new();

    for (index, step) in script.steps.iter().enumerate() {
        if step.operations.is_empty() {
            match &step.expect {
                GestureExpectation::Blocked { construct } => {
                    blocked.push(BlockedConstruct {
                        construct: construct.clone(),
                        element: step.element.clone(),
                        intent: step.intent.clone(),
                        reasons: vec![NOT_EXPRESSIBLE.to_string()],
                    });
                    continue;
                }
                GestureExpectation::Applied => {
                    return Err(ReplayError::ExpectationMismatch {
                        step: index,
                        intent: step.intent.clone(),
                        expected: step.expect.clone(),
                        status: "EmptyStep".to_string(),
                        reasons: vec![
                            "step expected Applied but carries no operations".to_string(),
                        ],
                    });
                }
            }
        }

        let project =
            load_authoring_project_from_sysml(files.clone()).map_err(|err| ReplayError::Load {
                step: index,
                message: err.to_string(),
            })?;
        let context = MutationContext::from_project(project);
        let proposal = MutationProposal {
            intent: step.intent.clone(),
            operations: step.operations.clone(),
            evidence: Vec::new(),
            rationale: None,
            workspace_revision: context.workspace_revision.clone(),
        };
        let report = service.check(&context, &proposal);

        match report.status {
            FeasibilityStatus::Allowed | FeasibilityStatus::AllowedWithWarnings => {
                if let GestureExpectation::Blocked { construct } = &step.expect {
                    return Err(ReplayError::ExpectationMismatch {
                        step: index,
                        intent: step.intent.clone(),
                        expected: GestureExpectation::Blocked {
                            construct: construct.clone(),
                        },
                        status: format!("{:?}", report.status),
                        reasons: vec![
                            "the pipeline now supports this construct; re-bless the ledger"
                                .to_string(),
                        ],
                    });
                }
                let plan = report
                    .normalized_plan
                    .ok_or(ReplayError::MissingPlan { step: index })?;
                let result = service
                    .apply_checked_plan(&context, &plan)
                    .map_err(|issue| ReplayError::Apply {
                        step: index,
                        message: format!("{:?}: {}", issue.kind, issue.message),
                    })?;
                files.extend(result.edited_files);
                applied.push(AppliedGesture {
                    intent: step.intent.clone(),
                    element: step.element.clone(),
                    operations: step.operations.len(),
                    warnings: report
                        .warnings
                        .iter()
                        .map(|issue| issue.message.clone())
                        .collect(),
                });
            }
            status => {
                let reasons = report
                    .blocking_reasons
                    .iter()
                    .chain(report.warnings.iter())
                    .map(|issue| format!("{:?}: {}", issue.kind, issue.message))
                    .collect::<Vec<_>>();
                match &step.expect {
                    GestureExpectation::Blocked { construct } => {
                        blocked.push(BlockedConstruct {
                            construct: construct.clone(),
                            element: step.element.clone(),
                            intent: step.intent.clone(),
                            reasons,
                        });
                    }
                    GestureExpectation::Applied => {
                        return Err(ReplayError::ExpectationMismatch {
                            step: index,
                            intent: step.intent.clone(),
                            expected: GestureExpectation::Applied,
                            status: format!("{status:?}"),
                            reasons,
                        });
                    }
                }
            }
        }
    }

    Ok(ReplayReport {
        final_files: files,
        applied,
        blocked,
    })
}

// --- deriver ----------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DerivedGestures {
    pub script: GestureScript,
    /// The reference files with every blocked construct pruned, rendered
    /// through the canonical printer. Compile these for the equivalence
    /// comparison when the script contains blocked steps.
    pub expressible_files: BTreeMap<String, String>,
    /// Summary of the blocked steps (same data as the script's Blocked steps).
    pub blocked: Vec<BlockedConstruct>,
}

/// Modifier flags the action space can set back via `SetAttribute`.
const FLAG_MODIFIERS: &[(&str, &str, bool)] = &[
    ("abstract", "is_abstract", true),
    ("derived", "is_derived", true),
    ("end", "is_end", true),
    ("individual", "is_individual", true),
    ("ordered", "is_ordered", true),
    ("nonunique", "is_unique", false),
    ("variable", "is_variable", true),
];

const TRANSITION_MODIFIER_KEYS: &[&str] = &[
    "transition_source",
    "transition_target",
    "trigger",
    "trigger_kind",
];

struct GestureDeriver<'a> {
    project: &'a AuthoringProject,
    /// The same legality service the feasibility pipeline consults; the
    /// deriver asks it up front so profile-blocked containments and attribute
    /// writes become honest `Blocked` steps instead of doomed operations.
    legality: SysmlSemanticLegalityService,
    steps: Vec<GestureStep>,
    relationship_steps: Vec<GestureStep>,
    blocked: Vec<BlockedConstruct>,
    /// Pruned header/member data per populated definition, consumed when the
    /// pruned tree is assembled in source order.
    pruned_definitions: BTreeMap<String, PrunedDefinitionParts>,
}

#[derive(Debug, Clone, Default)]
struct PrunedDefinitionParts {
    docs: Vec<String>,
    members: Vec<Declaration>,
    specializes: Vec<QualifiedName>,
    modifiers: Vec<String>,
}

/// Derive the gesture script that recreates `project`, plus the pruned
/// expressible reference. Purely mechanical — no per-model curation.
pub fn derive_gestures(project: &AuthoringProject, model: &str) -> DerivedGestures {
    let mut deriver = GestureDeriver {
        project,
        legality: sysml_semantic_legality_service(),
        steps: Vec::new(),
        relationship_steps: Vec::new(),
        blocked: Vec::new(),
        pruned_definitions: BTreeMap::new(),
    };
    let render = textual_model_authoring_render_profile();
    let mut expressible_files = BTreeMap::new();

    for (path, module) in project.files() {
        let mut pruned_module = AuthoringModule {
            package: None,
            members: Vec::new(),
        };
        if let Some(package) = &module.package {
            pruned_module.package = deriver.derive_package(path, package);
        }
        for member in &module.members {
            match member {
                Declaration::Package(package) => {
                    if let Some(pruned) = deriver.derive_package(path, package) {
                        pruned_module.members.push(Declaration::Package(pruned));
                    }
                }
                other => {
                    deriver.block(
                        &format!("top-level {} outside a package", declaration_label(other)),
                        None,
                        format!(
                            "Recreate top-level {} in `{path}`",
                            declaration_label(other)
                        ),
                    );
                }
            }
        }
        expressible_files.insert(path.to_string(), (render.render_module)(&pruned_module));
    }

    let mut steps = deriver.steps;
    steps.append(&mut deriver.relationship_steps);
    DerivedGestures {
        script: GestureScript {
            schema_version: GESTURE_SCRIPT_SCHEMA_VERSION.to_string(),
            model: model.to_string(),
            steps,
        },
        expressible_files,
        blocked: deriver.blocked,
    }
}

impl GestureDeriver<'_> {
    /// True when the capability profile refuses this attribute write.
    fn attribute_write_blocked(&self, kind: &str, attribute: &str) -> bool {
        matches!(
            self.legality.check_attribute_write(kind, attribute).status,
            SemanticLegalityStatus::Blocked
        )
    }

    /// True when the capability profile refuses this containment.
    fn containment_blocked(&self, container_kind: &str, child_keyword: &str) -> bool {
        matches!(
            self.legality
                .check_containment(container_kind, child_keyword)
                .status,
            SemanticLegalityStatus::Blocked
        )
    }

    /// Record a profile-blocked facet (an attribute write the action space
    /// refuses on this kind); the caller prunes the facet from the
    /// expressible reference.
    fn block_facet(&mut self, kind: &str, attribute: &str, qname: &str) {
        self.block(
            &format!("`{attribute}` write on `{kind}`"),
            Some(qname.to_string()),
            format!("Recreate `{attribute}` of `{qname}`"),
        );
    }

    fn block(&mut self, construct: &str, element: Option<String>, intent: String) {
        self.steps.push(GestureStep {
            intent: intent.clone(),
            element: element.clone(),
            operations: Vec::new(),
            expect: GestureExpectation::Blocked {
                construct: construct.to_string(),
            },
        });
        self.blocked.push(BlockedConstruct {
            construct: construct.to_string(),
            element,
            intent,
            reasons: vec![NOT_EXPRESSIBLE.to_string()],
        });
    }

    fn push_step(&mut self, intent: String, element: Option<String>, ops: Vec<SemanticMutation>) {
        if ops.is_empty() {
            return;
        }
        self.steps.push(GestureStep {
            intent,
            element,
            operations: ops,
            expect: GestureExpectation::Applied,
        });
    }

    fn derive_package(&mut self, path: &str, package: &Package) -> Option<Package> {
        let qname = package.name.as_dot_string();
        let mut pruned = Package {
            name: package.name.clone(),
            members: Vec::new(),
            comments: Vec::new(),
            docs: Vec::new(),
            modifiers: Vec::new(),
        };
        let mut create_ops = vec![SemanticMutation::AddPackage {
            target_file: path.to_string(),
            name: package.name.as_colon_string(),
        }];
        if !package.modifiers.is_empty() {
            self.block(
                &format!("package modifier `{}`", package.modifiers.join(" ")),
                Some(qname.clone()),
                format!("Recreate modifiers on package `{qname}`"),
            );
        }
        self.derive_docs(
            &qname,
            "package",
            &package.docs,
            &mut create_ops,
            &mut pruned.docs,
        );

        let import_paths = package
            .members
            .iter()
            .filter_map(|member| match member {
                Declaration::Import(import) => Some(import.path.as_colon_string()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut imports_expressible = true;
        if !import_paths.is_empty() {
            if self.attribute_write_blocked("package", "imports") {
                imports_expressible = false;
                self.block_facet("package", "imports", &qname);
            } else {
                create_ops.push(SemanticMutation::SetAttribute {
                    element: ElementRef::new(qname.clone()),
                    attribute: "imports".to_string(),
                    value: Value::Array(import_paths.iter().cloned().map(Value::String).collect()),
                });
            }
        }
        self.push_step(
            format!("Create package `{qname}`"),
            Some(qname.clone()),
            create_ops,
        );

        // Declare every definition of the package (preorder, so nested
        // definitions follow their containing definition) before any usage
        // references them by type.
        let mut definitions = Vec::new();
        collect_definitions(&package.members, &qname, "package", &mut definitions);
        let mut declare_ops = Vec::new();
        let mut kept_definitions = BTreeSet::new();
        for (owner, owner_kind, definition) in &definitions {
            let def_qname = format!("{owner}.{}", definition.name);
            if let Some(unsupported) = unsupported_definition_modifiers(definition) {
                self.block(
                    &format!("`{unsupported}` modifier on `{} def`", definition.keyword),
                    Some(def_qname.clone()),
                    format!("Recreate definition `{def_qname}`"),
                );
                continue;
            }
            if self.containment_blocked(owner_kind, &definition.keyword) {
                self.block(
                    &format!("`{} def` in `{owner_kind}`", definition.keyword),
                    Some(def_qname.clone()),
                    format!("Recreate definition `{def_qname}`"),
                );
                continue;
            }
            kept_definitions.insert(def_qname);
            declare_ops.push(SemanticMutation::AddDefinition {
                container: ElementRef::new(owner.clone()),
                keyword: definition.keyword.clone(),
                name: definition.name.clone(),
                specializes: Vec::new(),
            });
        }
        self.push_step(
            format!("Declare definitions in `{qname}`"),
            Some(qname.clone()),
            declare_ops,
        );

        // Populate each kept definition: specializations, docs, modifier
        // flags, then its member usages (parents before children).
        for (owner, _, definition) in &definitions {
            let def_qname = format!("{owner}.{}", definition.name);
            let def_kind = format!("{} def", definition.keyword);
            if !kept_definitions.contains(&def_qname) {
                continue;
            }
            let mut ops = Vec::new();
            let mut parts = PrunedDefinitionParts {
                specializes: definition.specializes.clone(),
                modifiers: definition.modifiers.clone(),
                ..PrunedDefinitionParts::default()
            };
            if !definition.specializes.is_empty() {
                if self.attribute_write_blocked(&def_kind, "specializes") {
                    parts.specializes.clear();
                    self.block_facet(&def_kind, "specializes", &def_qname);
                } else {
                    ops.push(SemanticMutation::SetAttribute {
                        element: ElementRef::new(def_qname.clone()),
                        attribute: "specializes".to_string(),
                        value: qname_list_value(&definition.specializes),
                    });
                }
            }
            let mut pruned_docs = Vec::new();
            self.derive_docs(
                &def_qname,
                &def_kind,
                &definition.docs,
                &mut ops,
                &mut pruned_docs,
            );
            for (attribute, enabled) in flag_modifier_attributes(&definition.modifiers) {
                if self.attribute_write_blocked(&def_kind, &attribute) {
                    prune_flag_modifier(&mut parts.modifiers, &attribute);
                    self.block_facet(&def_kind, &attribute, &def_qname);
                } else {
                    ops.push(set_attribute(&def_qname, &attribute, Value::Bool(enabled)));
                }
            }
            let mut pruned_members = Vec::new();
            for member in &definition.members {
                match member {
                    Declaration::Usage(usage) => {
                        if let Some(kept) =
                            self.derive_usage(&def_qname, &def_kind, usage, &mut ops)
                        {
                            pruned_members.push(Declaration::Usage(kept));
                        }
                    }
                    Declaration::Definition(nested) => {
                        // Declared in the definitions pass; keep it in the
                        // pruned tree only if it survived that pass.
                        if kept_definitions.contains(&format!("{def_qname}.{}", nested.name)) {
                            pruned_members.push(member.clone());
                        }
                    }
                    other => {
                        self.block(
                            &format!(
                                "{} inside `{} def`",
                                declaration_label(other),
                                definition.keyword
                            ),
                            Some(def_qname.clone()),
                            format!("Recreate {} inside `{def_qname}`", declaration_label(other)),
                        );
                    }
                }
            }
            self.push_step(
                format!("Populate `{def_qname}`"),
                Some(def_qname.clone()),
                ops,
            );
            // Record the pruned definition parts; the pruned tree is
            // assembled in source order below.
            parts.docs = pruned_docs;
            parts.members = pruned_members;
            self.pruned_definitions.insert(def_qname.clone(), parts);
        }

        // Rebuild the pruned member list in original order.
        for member in &package.members {
            match member {
                Declaration::Import(import) => {
                    if imports_expressible {
                        pruned.members.push(Declaration::Import(
                            mercurio_core::authoring::Import {
                                path: import.path.clone(),
                                comments: Vec::new(),
                                docs: Vec::new(),
                                modifiers: Vec::new(),
                            },
                        ));
                    }
                }
                Declaration::Definition(definition) => {
                    if let Some(kept) =
                        self.pruned_definition(&qname, definition, &kept_definitions)
                    {
                        pruned.members.push(Declaration::Definition(kept));
                    }
                }
                Declaration::Usage(usage) => {
                    let mut ops = Vec::new();
                    if let Some(kept) = self.derive_usage(&qname, "package", usage, &mut ops) {
                        let usage_qname = format!("{qname}.{}", kept.name);
                        self.push_step(format!("Create `{usage_qname}`"), Some(usage_qname), ops);
                        pruned.members.push(Declaration::Usage(kept));
                    }
                }
                Declaration::Package(nested) => {
                    self.block(
                        "nested package",
                        Some(format!("{qname}.{}", nested.name.as_dot_string())),
                        format!("Recreate nested package inside `{qname}`"),
                    );
                }
                Declaration::Alias(alias) => {
                    self.block(
                        "alias",
                        Some(format!("{qname}.{}", alias.name)),
                        format!("Recreate alias `{}` inside `{qname}`", alias.name),
                    );
                }
            }
        }

        Some(pruned)
    }

    fn derive_docs(
        &mut self,
        qname: &str,
        kind: &str,
        docs: &[String],
        ops: &mut Vec<SemanticMutation>,
        pruned_docs: &mut Vec<String>,
    ) {
        if let Some(first) = docs.first() {
            if first.contains('\n') {
                // Multi-line doc bodies keep their interior indentation, and
                // the canonical printer re-indents continuation lines, so the
                // text cannot round-trip byte-exactly. Blocked rather than
                // forcing an op that produces non-equivalent output.
                self.block(
                    "multi-line doc block",
                    Some(qname.to_string()),
                    format!("Recreate multi-line doc block on `{qname}`"),
                );
            } else if self.attribute_write_blocked(kind, "doc") {
                self.block_facet(kind, "doc", qname);
            } else {
                ops.push(SemanticMutation::SetAttribute {
                    element: ElementRef::new(qname.to_string()),
                    attribute: "doc".to_string(),
                    value: Value::String(first.clone()),
                });
                pruned_docs.push(first.clone());
            }
        }
        if docs.len() > 1 {
            self.block(
                "multiple doc blocks",
                Some(qname.to_string()),
                format!("Recreate additional doc blocks on `{qname}`"),
            );
        }
    }

    /// Derive one usage subtree into `ops`. Returns the pruned copy to keep
    /// in the expressible reference, or `None` when the whole subtree is
    /// blocked. `owner_kind` is the owner's kind label as the legality
    /// service knows it (`package`, `part def`, `part`, ...).
    fn derive_usage(
        &mut self,
        owner: &str,
        owner_kind: &str,
        usage: &Usage,
        ops: &mut Vec<SemanticMutation>,
    ) -> Option<Usage> {
        let keyword = usage.keyword.as_str();
        if usage.is_implicit_name || usage.name.is_empty() {
            self.block(
                &format!("anonymous `{keyword}` usage"),
                Some(owner.to_string()),
                format!("Recreate anonymous `{keyword}` usage inside `{owner}`"),
            );
            return None;
        }
        let qname = format!("{owner}.{}", usage.name);

        let modifiers = classify_modifiers(&usage.modifiers);
        if !modifiers.unknown.is_empty() {
            self.block(
                &format!(
                    "`{}` modifier on `{keyword}` usage",
                    modifiers.unknown.join(" ")
                ),
                Some(qname.clone()),
                format!("Recreate `{qname}`"),
            );
            return None;
        }
        if !usage.metadata_properties.is_empty() || keyword == "metadata" {
            self.block(
                "metadata usage",
                Some(qname.clone()),
                format!("Recreate metadata usage `{qname}`"),
            );
            return None;
        }
        if usage.raw_body.is_some() {
            self.block(
                &format!("raw body on `{keyword}` usage"),
                Some(qname.clone()),
                format!("Recreate raw body of `{qname}`"),
            );
            return None;
        }

        if keyword == "transition" || !modifiers.transition_values.is_empty() {
            // The semantic capability profile does not (yet) allow writing
            // `transition_source` / `transition_target` / `trigger` on
            // transitions, so state-machine transitions land in the DA-7
            // gesture backlog via the ledger.
            self.block(
                "state transition",
                Some(qname.clone()),
                format!("Recreate transition `{qname}`"),
            );
            return None;
        }

        if let Some(source) = &modifiers.relationship_source {
            return self.derive_relationship_usage(owner, usage, source);
        }

        if self.containment_blocked(owner_kind, keyword) {
            self.block(
                &format!("`{keyword}` usage in `{owner_kind}`"),
                Some(qname.clone()),
                format!("Recreate `{keyword}` usage `{qname}`"),
            );
            return None;
        }

        let mut pruned = Usage {
            keyword: usage.keyword.clone(),
            name: usage.name.clone(),
            is_implicit_name: usage.is_implicit_name,
            ty: usage.ty.clone(),
            reference_target: usage.reference_target.clone(),
            metadata_properties: usage.metadata_properties.clone(),
            multiplicity: usage.multiplicity.clone(),
            expression: usage.expression.clone(),
            additional_types: usage.additional_types.clone(),
            specializes: usage.specializes.clone(),
            subsets: usage.subsets.clone(),
            redefines: usage.redefines.clone(),
            members: Vec::new(),
            raw_body: usage.raw_body.clone(),
            comments: Vec::new(),
            docs: Vec::new(),
            modifiers: usage.modifiers.clone(),
        };

        ops.push(SemanticMutation::AddUsage {
            container: ElementRef::new(owner.to_string()),
            keyword: usage.keyword.clone(),
            name: usage.name.clone(),
            ty: None,
            specializes: Vec::new(),
        });
        if let Some(ty) = &usage.ty {
            if self.attribute_write_blocked(keyword, "type") {
                pruned.ty = None;
                self.block_facet(keyword, "type", &qname);
            } else {
                ops.push(set_attribute(
                    &qname,
                    "type",
                    Value::String(ty.as_colon_string()),
                ));
            }
        }
        for (attribute, names, pruned_names) in [
            ("specializes", &usage.specializes, &mut pruned.specializes),
            (
                "additional_types",
                &usage.additional_types,
                &mut pruned.additional_types,
            ),
            ("subsets", &usage.subsets, &mut pruned.subsets),
            ("redefines", &usage.redefines, &mut pruned.redefines),
        ] {
            if names.is_empty() {
                continue;
            }
            if self.attribute_write_blocked(keyword, attribute) {
                pruned_names.clear();
                self.block_facet(keyword, attribute, &qname);
            } else {
                ops.push(set_attribute(&qname, attribute, qname_list_value(names)));
            }
        }
        if let Some(multiplicity) = &usage.multiplicity {
            if self.attribute_write_blocked(keyword, "multiplicity") {
                pruned.multiplicity = None;
                self.block_facet(keyword, "multiplicity", &qname);
            } else {
                ops.push(set_attribute(
                    &qname,
                    "multiplicity",
                    Value::String(multiplicity.raw.clone()),
                ));
            }
        }
        if let Some(reference_target) = &usage.reference_target {
            if self.attribute_write_blocked(keyword, "reference_target") {
                pruned.reference_target = None;
                self.block_facet(keyword, "reference_target", &qname);
            } else {
                ops.push(set_attribute(
                    &qname,
                    "reference_target",
                    Value::String(reference_target.as_colon_string()),
                ));
            }
        }
        for (attribute, enabled) in &modifiers.flags {
            if self.attribute_write_blocked(keyword, attribute) {
                prune_flag_modifier(&mut pruned.modifiers, attribute);
                self.block_facet(keyword, attribute, &qname);
            } else {
                ops.push(set_attribute(&qname, attribute, Value::Bool(*enabled)));
            }
        }
        if let Some(direction) = &modifiers.direction {
            if self.attribute_write_blocked(keyword, "direction") {
                pruned
                    .modifiers
                    .retain(|modifier| !matches!(modifier.as_str(), "in" | "out" | "inout"));
                self.block_facet(keyword, "direction", &qname);
            } else {
                ops.push(set_attribute(
                    &qname,
                    "direction",
                    Value::String(direction.clone()),
                ));
            }
        }
        if let Some(expression) = &usage.expression {
            if self.attribute_write_blocked(keyword, "expression") {
                pruned.expression = None;
                self.block_facet(keyword, "expression", &qname);
            } else {
                ops.push(SemanticMutation::SetExpression {
                    element: ElementRef::new(qname.clone()),
                    expression: Some(SemanticExpression::Text(expression.clone())),
                });
            }
        }
        self.derive_docs(&qname, keyword, &usage.docs, ops, &mut pruned.docs);

        for member in &usage.members {
            match member {
                Declaration::Usage(child) => {
                    if let Some(kept) = self.derive_usage(&qname, keyword, child, ops) {
                        pruned.members.push(Declaration::Usage(kept));
                    }
                }
                other => {
                    self.block(
                        &format!("{} inside `{keyword}` usage", declaration_label(other)),
                        Some(qname.clone()),
                        format!("Recreate {} inside `{qname}`", declaration_label(other)),
                    );
                }
            }
        }
        Some(pruned)
    }

    fn derive_relationship_usage(
        &mut self,
        owner: &str,
        usage: &Usage,
        source: &str,
    ) -> Option<Usage> {
        let qname = format!("{owner}.{}", usage.name);
        let Some(target) = &usage.reference_target else {
            self.block(
                &format!("`{}` relationship without a target", usage.keyword),
                Some(qname),
                format!("Recreate `{}` relationship inside `{owner}`", usage.keyword),
            );
            return None;
        };
        let target_tail = target.as_dot_string();
        let target_tail = target_tail.rsplit('.').next().unwrap_or_default();
        if usage.name != target_tail {
            // AddRelationship names the reified usage after the target's
            // tail; a differently named relationship usage cannot be
            // recreated faithfully yet.
            self.block(
                &format!("named `{}` relationship", usage.keyword),
                Some(qname),
                format!(
                    "Recreate `{}` relationship `{}` inside `{owner}`",
                    usage.keyword, usage.name
                ),
            );
            return None;
        }
        let source_ref = self.resolve_reference(owner, source);
        let target_ref = self.resolve_reference(owner, &target.as_dot_string());
        self.relationship_steps.push(GestureStep {
            intent: format!(
                "Relate `{source_ref}` -> `{target_ref}` via `{}`",
                usage.keyword
            ),
            element: Some(qname),
            operations: vec![SemanticMutation::AddRelationship {
                kind: usage.keyword.clone(),
                source: ElementRef::new(source_ref),
                target: ElementRef::new(target_ref),
            }],
            expect: GestureExpectation::Applied,
        });
        Some(usage.clone())
    }

    /// Resolve a source-relative dotted reference against the reference
    /// project by walking the owner scope outward. Falls back to the raw
    /// reference when nothing resolves.
    fn resolve_reference(&self, owner: &str, reference: &str) -> String {
        let normalized = QualifiedName::parse(reference).as_dot_string();
        let mut scope = Some(owner.to_string());
        while let Some(current) = scope {
            let candidate = format!("{current}.{normalized}");
            if self
                .project
                .semantic_attributes(&QualifiedName::parse(&candidate))
                .is_ok()
            {
                return candidate;
            }
            scope = current
                .rsplit_once('.')
                .map(|(parent, _)| parent.to_string());
        }
        if self
            .project
            .semantic_attributes(&QualifiedName::parse(&normalized))
            .is_ok()
        {
            return normalized;
        }
        normalized
    }

    fn pruned_definition(
        &mut self,
        owner: &str,
        definition: &Definition,
        kept_definitions: &BTreeSet<String>,
    ) -> Option<Definition> {
        let def_qname = format!("{owner}.{}", definition.name);
        if !kept_definitions.contains(&def_qname) {
            return None;
        }
        let parts = self
            .pruned_definitions
            .remove(&def_qname)
            .unwrap_or_default();
        let members = parts
            .members
            .into_iter()
            .filter_map(|member| match member {
                Declaration::Definition(nested) => self
                    .pruned_definition(&def_qname, &nested, kept_definitions)
                    .map(Declaration::Definition),
                other => Some(other),
            })
            .collect();
        Some(Definition {
            keyword: definition.keyword.clone(),
            name: definition.name.clone(),
            specializes: parts.specializes,
            members,
            raw_body: definition.raw_body.clone(),
            comments: Vec::new(),
            docs: parts.docs,
            modifiers: parts.modifiers,
        })
    }
}

fn collect_definitions<'a>(
    members: &'a [Declaration],
    owner: &str,
    owner_kind: &str,
    out: &mut Vec<(String, String, &'a Definition)>,
) {
    for member in members {
        if let Declaration::Definition(definition) = member {
            out.push((owner.to_string(), owner_kind.to_string(), definition));
            collect_definitions(
                &definition.members,
                &format!("{owner}.{}", definition.name),
                &format!("{} def", definition.keyword),
                out,
            );
        }
    }
}

/// `(attribute, enabled)` pairs for the flag modifiers present on a
/// declaration (e.g. `individual` -> `("is_individual", true)`).
fn flag_modifier_attributes(modifiers: &[String]) -> Vec<(String, bool)> {
    modifiers
        .iter()
        .filter_map(|modifier| {
            FLAG_MODIFIERS
                .iter()
                .find(|(name, _, _)| name == &modifier.as_str())
                .map(|(_, attribute, enabled)| (attribute.to_string(), *enabled))
        })
        .collect()
}

/// Remove the modifier corresponding to a flag attribute from a pruned
/// modifier list (e.g. `is_individual` removes `individual`).
fn prune_flag_modifier(modifiers: &mut Vec<String>, attribute: &str) {
    if let Some((name, _, _)) = FLAG_MODIFIERS
        .iter()
        .find(|(_, flag_attribute, _)| flag_attribute == &attribute)
    {
        modifiers.retain(|modifier| modifier != name);
    }
}

fn unsupported_definition_modifiers(definition: &Definition) -> Option<String> {
    let unsupported = definition
        .modifiers
        .iter()
        .filter(|modifier| {
            !FLAG_MODIFIERS
                .iter()
                .any(|(name, _, _)| name == &modifier.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    if unsupported.is_empty() {
        None
    } else {
        Some(unsupported.join(" "))
    }
}

struct ClassifiedModifiers {
    flags: Vec<(String, bool)>,
    direction: Option<String>,
    transition_values: Vec<(String, String)>,
    relationship_source: Option<String>,
    unknown: Vec<String>,
}

fn classify_modifiers(modifiers: &[String]) -> ClassifiedModifiers {
    let mut classified = ClassifiedModifiers {
        flags: Vec::new(),
        direction: None,
        transition_values: Vec::new(),
        relationship_source: None,
        unknown: Vec::new(),
    };
    for modifier in modifiers {
        if let Some((key, value)) = modifier.split_once('=') {
            if TRANSITION_MODIFIER_KEYS.contains(&key) {
                classified
                    .transition_values
                    .push((key.to_string(), value.to_string()));
                continue;
            }
            if key == "relationship_source" {
                classified.relationship_source = Some(value.to_string());
                continue;
            }
            classified.unknown.push(modifier.clone());
            continue;
        }
        if let Some((_, attribute, enabled)) = FLAG_MODIFIERS
            .iter()
            .find(|(name, _, _)| name == &modifier.as_str())
        {
            classified.flags.push((attribute.to_string(), *enabled));
            continue;
        }
        match modifier.as_str() {
            "in" | "out" | "inout" => classified.direction = Some(modifier.clone()),
            "source_is_initial" => classified
                .transition_values
                .push(("source_is_initial".to_string(), "true".to_string())),
            "constraint" => {
                // `assume constraint` / `assert constraint` marker; those
                // usages are anonymous and blocked before modifiers matter,
                // but a named constraint-marked usage is not recreatable.
                classified.unknown.push(modifier.clone());
            }
            _ => classified.unknown.push(modifier.clone()),
        }
    }
    classified
}

fn set_attribute(qname: &str, attribute: &str, value: Value) -> SemanticMutation {
    SemanticMutation::SetAttribute {
        element: ElementRef::new(qname.to_string()),
        attribute: attribute.to_string(),
        value,
    }
}

fn qname_list_value(names: &[QualifiedName]) -> Value {
    Value::Array(
        names
            .iter()
            .map(|name| Value::String(name.as_colon_string()))
            .collect(),
    )
}

fn declaration_label(declaration: &Declaration) -> &'static str {
    match declaration {
        Declaration::Package(_) => "package",
        Declaration::Import(_) => "import",
        Declaration::Definition(_) => "definition",
        Declaration::Usage(_) => "usage",
        Declaration::Alias(_) => "alias",
    }
}

// --- parity driver ----------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AuthoringParityOutcome {
    pub script: GestureScript,
    pub replay: ReplayReport,
    /// The pruned expressible reference files (canonical render).
    pub expressible_files: BTreeMap<String, String>,
    pub equivalence: KirEquivalenceReport,
    /// True when blocked constructs forced the comparison onto the pruned
    /// reference instead of the original text-authored compile.
    pub compared_against_pruned: bool,
}

/// Compile a file map the same way the replay engine's projects compile:
/// parse every file, compile each with the others as context, merge with the
/// registered SysML field specs.
pub fn compile_replay_files(files: &BTreeMap<String, String>) -> Result<KirDocument, ReplayError> {
    let stdlib = shared_sysml_baseline().map_err(|err| ReplayError::Compile {
        context: "stdlib baseline".to_string(),
        message: err.to_string(),
    })?;
    let mut modules = Vec::new();
    for (path, source) in files {
        modules.push(parse_sysml(source).map_err(|err| compile_error(path, &err))?);
    }
    let mut documents = Vec::new();
    for (path, source) in files {
        documents.push(
            compile_sysml_text_with_context(source, path, &modules, &stdlib)
                .map_err(|err| compile_error(path, &err))?,
        );
    }
    KirDocument::merge_with_registered_fields(documents, sysml_field_specs().iter().copied())
        .map_err(|err| ReplayError::Compile {
            context: "merge".to_string(),
            message: err.to_string(),
        })
}

fn compile_error(path: &str, err: &Diagnostic) -> ReplayError {
    ReplayError::Compile {
        context: path.to_string(),
        message: err.to_string(),
    }
}

/// Full Tier-1 pass for one model: derive, replay into an empty workspace,
/// compile both sides, and compare with the equivalence oracle.
pub fn run_authoring_parity(
    model: &str,
    files: &BTreeMap<String, String>,
) -> Result<AuthoringParityOutcome, ReplayError> {
    let project =
        load_authoring_project_from_sysml(files.clone()).map_err(|err| ReplayError::Compile {
            context: format!("{model}: reference project"),
            message: err.to_string(),
        })?;
    let derived = derive_gestures(&project, model);
    let replay = replay_gesture_script(&derived.script)?;
    let compared_against_pruned = !replay.blocked.is_empty();
    let reference = if compared_against_pruned {
        compile_replay_files(&derived.expressible_files)?
    } else {
        compile_replay_files(files)?
    };
    let replayed = compile_replay_files(&replay.final_files)?;
    let equivalence = kir_equivalence_report(&reference, &replayed);
    Ok(AuthoringParityOutcome {
        script: derived.script,
        replay,
        expressible_files: derived.expressible_files,
        equivalence,
        compared_against_pruned,
    })
}

// --- coverage ledger --------------------------------------------------------

/// Machine-maintained record of every blocked construct across a corpus.
/// Regenerated (never hand-edited) via the bless flow; CI fails on any drift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageLedger {
    pub schema_version: String,
    /// Model label -> sorted, deduplicated blocked constructs. Models that
    /// replay fully are recorded with an empty list so corpus growth is
    /// ledger-visible too.
    pub models: BTreeMap<String, Vec<CoverageLedgerEntry>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CoverageLedgerEntry {
    pub construct: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
}

impl Default for CoverageLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl CoverageLedger {
    pub fn new() -> Self {
        Self {
            schema_version: AUTHORING_PARITY_LEDGER_SCHEMA_VERSION.to_string(),
            models: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, model: &str, blocked: &[BlockedConstruct]) {
        let mut entries = blocked
            .iter()
            .map(|blocked| CoverageLedgerEntry {
                construct: blocked.construct.clone(),
                element: blocked.element.clone(),
            })
            .collect::<Vec<_>>();
        entries.sort();
        entries.dedup();
        self.models.insert(model.to_string(), entries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_file(source: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("model.sysml".to_string(), source.to_string())])
    }

    #[test]
    fn gesture_script_serde_round_trips() {
        let script = GestureScript {
            schema_version: GESTURE_SCRIPT_SCHEMA_VERSION.to_string(),
            model: "demo".to_string(),
            steps: vec![
                GestureStep {
                    intent: "Create package `Demo`".to_string(),
                    element: Some("Demo".to_string()),
                    operations: vec![SemanticMutation::AddPackage {
                        target_file: "model.sysml".to_string(),
                        name: "Demo".to_string(),
                    }],
                    expect: GestureExpectation::Applied,
                },
                GestureStep {
                    intent: "Recreate alias".to_string(),
                    element: Some("Demo.a".to_string()),
                    operations: Vec::new(),
                    expect: GestureExpectation::Blocked {
                        construct: "alias".to_string(),
                    },
                },
            ],
        };
        let serialized = serde_json::to_string_pretty(&script).expect("script serializes");
        let deserialized: GestureScript =
            serde_json::from_str(&serialized).expect("script deserializes");
        assert_eq!(deserialized, script);
    }

    #[test]
    fn replays_a_minimal_model_equivalently() {
        let files =
            single_file("package Demo {\n    part def Vehicle;\n    part vehicle : Vehicle;\n}\n");
        let outcome = run_authoring_parity("demo", &files).expect("parity run succeeds");
        assert!(
            outcome.replay.blocked.is_empty(),
            "expected nothing blocked: {:?}",
            outcome.replay.blocked
        );
        assert!(!outcome.compared_against_pruned);
        assert!(
            outcome.equivalence.equivalent,
            "diff: {}",
            serde_json::to_string_pretty(&outcome.equivalence.diff).unwrap_or_default()
        );
    }

    #[test]
    fn blocked_constructs_are_ledgered_and_pruned() {
        let files =
            single_file("package Demo {\n    part def Vehicle;\n    alias V for Vehicle;\n}\n");
        let outcome = run_authoring_parity("demo", &files).expect("parity run succeeds");
        assert!(outcome.compared_against_pruned);
        assert_eq!(outcome.replay.blocked.len(), 1);
        assert_eq!(outcome.replay.blocked[0].construct, "alias");
        assert!(
            outcome.equivalence.equivalent,
            "diff: {}",
            serde_json::to_string_pretty(&outcome.equivalence.diff).unwrap_or_default()
        );

        let mut ledger = CoverageLedger::new();
        ledger.record("demo", &outcome.replay.blocked);
        let entries = ledger.models.get("demo").cloned().unwrap_or_default();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].construct, "alias");
    }

    #[test]
    fn expected_applied_step_that_blocks_is_an_error() {
        let script = GestureScript {
            schema_version: GESTURE_SCRIPT_SCHEMA_VERSION.to_string(),
            model: "demo".to_string(),
            steps: vec![GestureStep {
                intent: "Add usage to a missing container".to_string(),
                element: None,
                operations: vec![SemanticMutation::AddUsage {
                    container: ElementRef::new("Missing".to_string()),
                    keyword: "part".to_string(),
                    name: "vehicle".to_string(),
                    ty: None,
                    specializes: Vec::new(),
                }],
                expect: GestureExpectation::Applied,
            }],
        };
        let error = replay_gesture_script(&script).expect_err("step must fail");
        assert!(matches!(error, ReplayError::ExpectationMismatch { .. }));
    }
}
