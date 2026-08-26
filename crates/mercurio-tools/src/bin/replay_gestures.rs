//! DA-11 Tier-1 replay CLI (diagram-authoring C2.4).
//!
//! Derives a gesture script from a reference model (or every model under a
//! directory), replays it into an empty workspace through the production
//! check → apply pipeline, and prints the KIR equivalence report JSON.
//!
//! ```text
//! replay_gestures --model <file|dir> [--ledger <path> [--bless]]
//! replay_gestures --corpus <tier> [--pilot-root <dir>] [--ledger <path> [--bless]]
//! replay_gestures --compare <dirA> <dirB> [--prune-ledger <path>] [--prune-gap <path>]
//! ```
//!
//! Exit codes: 0 = every model oracle-equivalent (and ledger matches when
//! given), 1 = an equivalence diff or ledger drift, 2 = error.
//!
//! With `--ledger <path>` the blocked-construct set of the run is compared
//! against the committed ledger; `--bless` regenerates the file instead
//! (the ledger is machine-maintained, never hand-edited).
//!
//! `--corpus <tier>` replays a pilot-corpus tier from
//! `crates/mercurio-tools/corpus/pilot_corpus.seed.json` against an external
//! `SysML-v2-Pilot-Implementation` checkout (resolved like the other pilot
//! tools: `MERCURIO_PILOT_ROOT`, the pilot lock, or `--pilot-root`). This
//! mode depends on that external checkout and is meant for scheduled CI
//! sweeps — it is deliberately not part of any `cargo test` run.
//!
//! `--compare <dirA> <dirB>` (DA-11 Tier-2 oracle mode) compiles the `.sysml`
//! sources of two directories through the same compile entry the replay
//! engine uses and prints the C1 KIR-equivalence report. `dirA` is the
//! reference side, `dirB` the candidate (e.g. a workspace rebuilt through
//! real UI gestures).
//!
//! - `--prune-ledger <path>` applies the Tier-1 expressible-reference pruning
//!   to `dirA` (via `derive_gestures`, the exact code the replay harness
//!   uses) so action-space-blocked constructs are excluded from the
//!   comparison symmetrically; the pruned construct set is validated against
//!   the committed coverage ledger at `<path>` (warnings on stderr for
//!   constructs the ledger does not know).
//! - `--prune-gap <path>` applies an explicit, committed Tier-2 gesture-gap
//!   prune list (JSON: `{ "elements": [qname...], "facets": [{"element":
//!   qname, "facet": name}...] }`) to **both** sides, for expressible
//!   elements/facets today's UI gestures cannot produce. The pruned sets are
//!   echoed in the report JSON — the honest gap statement, never silent.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mercurio_core::authoring::{
    AuthoringModule, Declaration, Package, textual_model_authoring_render_profile,
};
use mercurio_core::kir_canonical::{KirEquivalenceReport, kir_equivalence_report};
use mercurio_sysml::load_authoring_project_from_sysml;
use mercurio_sysml::replay::{
    AuthoringParityOutcome, BlockedConstruct, CoverageLedger, compile_replay_files,
    derive_gestures, run_authoring_parity,
};
use mercurio_tools::default_pilot_root;
use serde::{Deserialize, Serialize};

const SKIP_DIR_COMPONENTS: &[&str] = &[
    "target",
    "output",
    "tmp",
    ".mercurio",
    "node_modules",
    ".git",
];

#[derive(Debug)]
struct Args {
    model: Option<PathBuf>,
    corpus: Option<String>,
    compare: Option<(PathBuf, PathBuf)>,
    pilot_root: PathBuf,
    ledger: Option<PathBuf>,
    bless: bool,
    prune_ledger: Option<PathBuf>,
    prune_gap: Option<PathBuf>,
}

/// The relevant slice of `pilot_corpus.seed.json`.
#[derive(Debug, Deserialize)]
struct PilotCorpusSeed {
    corpora: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Serialize)]
struct ModelResult {
    model: String,
    equivalent: bool,
    compared_against_pruned: bool,
    blocked: Vec<mercurio_sysml::replay::BlockedConstruct>,
    report: mercurio_core::kir_canonical::KirEquivalenceReport,
}

fn main() -> ExitCode {
    match run() {
        Ok(all_equivalent) => {
            if all_equivalent {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("replay_gestures: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let args = parse_args(env::args().skip(1).collect())?;
    if let Some((reference_dir, candidate_dir)) = &args.compare {
        return run_compare(
            reference_dir,
            candidate_dir,
            args.prune_ledger.as_deref(),
            args.prune_gap.as_deref(),
        );
    }
    let models = match (&args.model, &args.corpus) {
        (Some(model), None) => {
            let models = discover_models(model)?;
            if models.is_empty() {
                return Err(format!("no .sysml models found under {}", model.display()).into());
            }
            models
        }
        (None, Some(tier)) => pilot_corpus_models(tier, &args.pilot_root)?,
        _ => return Err("exactly one of --model, --corpus, or --compare is required".into()),
    };

    let mut ledger = CoverageLedger::new();
    let mut results = Vec::new();
    let mut all_equivalent = true;
    for (label, path) in &models {
        let outcome = run_model(label, path)?;
        ledger.record(label, &outcome.replay.blocked);
        all_equivalent &= outcome.equivalence.equivalent;
        results.push(ModelResult {
            model: label.clone(),
            equivalent: outcome.equivalence.equivalent,
            compared_against_pruned: outcome.compared_against_pruned,
            blocked: outcome.replay.blocked,
            report: outcome.equivalence,
        });
    }

    if results.len() == 1 {
        // Single-model mode prints the equivalence report itself.
        println!("{}", serde_json::to_string_pretty(&results[0].report)?);
        for blocked in &results[0].blocked {
            eprintln!(
                "blocked: {} @ {}",
                blocked.construct,
                blocked.element.as_deref().unwrap_or("<unknown>")
            );
        }
    } else {
        println!("{}", serde_json::to_string_pretty(&results)?);
    }

    if let Some(ledger_path) = &args.ledger {
        let rendered = format!("{}\n", serde_json::to_string_pretty(&ledger)?);
        if args.bless {
            std::fs::write(ledger_path, rendered)?;
            eprintln!("blessed ledger written to {}", ledger_path.display());
        } else {
            let committed = std::fs::read_to_string(ledger_path).map_err(|err| {
                format!(
                    "missing ledger {} ({err}); regenerate with --bless",
                    ledger_path.display()
                )
            })?;
            let committed_ledger: CoverageLedger = serde_json::from_str(&committed)?;
            if committed_ledger != ledger {
                eprintln!(
                    "ledger drift against {} — rerun with --bless and commit the result \
                     (the ledger is machine-maintained, never hand-edited)",
                    ledger_path.display()
                );
                all_equivalent = false;
            }
        }
    }

    Ok(all_equivalent)
}

// --- oracle dir-compare mode (DA-11 Tier 2) ---------------------------------

/// A committed Tier-2 gesture-gap prune list: expressible elements/facets
/// that today's UI gestures cannot produce, excluded from both sides
/// symmetrically and echoed in the report.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GapPruneList {
    #[serde(default)]
    elements: Vec<String>,
    #[serde(default)]
    facets: Vec<GapFacet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct GapFacet {
    element: String,
    facet: String,
}

#[derive(Debug, Serialize)]
struct CompareResult {
    reference_dir: String,
    candidate_dir: String,
    /// Constructs the expressible-reference pruning removed from the
    /// reference side (empty when `--prune-ledger` was not given or nothing
    /// was action-space-blocked).
    pruned_ledger_constructs: Vec<BlockedConstruct>,
    /// The Tier-2 gesture-gap prune list applied to both sides, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pruned_gap: Option<GapPruneList>,
    equivalent: bool,
    report: KirEquivalenceReport,
}

fn run_compare(
    reference_dir: &Path,
    candidate_dir: &Path,
    prune_ledger: Option<&Path>,
    prune_gap: Option<&Path>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut reference_files = read_model_files(reference_dir)?;
    let mut candidate_files = read_model_files(candidate_dir)?;

    let mut pruned_ledger_constructs: Vec<BlockedConstruct> = Vec::new();
    if let Some(ledger_path) = prune_ledger {
        let project =
            load_authoring_project_from_sysml(reference_files.clone()).map_err(|err| {
                format!("{}: reference project: {err}", reference_dir.display())
            })?;
        // The exact pruning the Tier-1 harness applies: derive_gestures
        // produces the expressible reference beside the blocked-construct
        // list (reused, not duplicated).
        let derived = derive_gestures(&project, &reference_dir.display().to_string());
        if !derived.blocked.is_empty() {
            pruned_ledger_constructs = derived.blocked.clone();
            reference_files = derived.expressible_files;
        }
        let committed_text = std::fs::read_to_string(ledger_path)
            .map_err(|err| format!("{}: {err}", ledger_path.display()))?;
        let committed: CoverageLedger = serde_json::from_str(&committed_text)?;
        let known_constructs = committed
            .models
            .values()
            .flatten()
            .map(|entry| entry.construct.as_str())
            .collect::<BTreeSet<_>>();
        for blocked in &pruned_ledger_constructs {
            if !known_constructs.contains(blocked.construct.as_str()) {
                eprintln!(
                    "warning: pruned construct `{}` (at {}) is not present in the committed ledger {}",
                    blocked.construct,
                    blocked.element.as_deref().unwrap_or("<unknown>"),
                    ledger_path.display()
                );
            }
        }
    }

    let mut pruned_gap = None;
    if let Some(gap_path) = prune_gap {
        let gap_text = std::fs::read_to_string(gap_path)
            .map_err(|err| format!("{}: {err}", gap_path.display()))?;
        let gap: GapPruneList = serde_json::from_str(&gap_text)?;
        let (reference_pruned, matched) = apply_gap_prune(&reference_files, &gap)
            .map_err(|err| format!("reference gap prune: {err}"))?;
        reference_files = reference_pruned;
        let (candidate_pruned, _) = apply_gap_prune(&candidate_files, &gap)
            .map_err(|err| format!("candidate gap prune: {err}"))?;
        candidate_files = candidate_pruned;
        // Typo guard: every gap entry should match something on the
        // reference side (the candidate genuinely lacks them, so candidate
        // no-ops are expected).
        for element in &gap.elements {
            if !matched.contains(element) {
                eprintln!("warning: gap prune element `{element}` matched nothing in the reference");
            }
        }
        for facet in &gap.facets {
            let key = format!("{}#{}", facet.element, facet.facet);
            if !matched.contains(&key) {
                eprintln!("warning: gap prune facet `{key}` matched nothing in the reference");
            }
        }
        pruned_gap = Some(gap);
    }

    let reference = compile_replay_files(&reference_files)
        .map_err(|err| format!("reference compile ({}): {err}", reference_dir.display()))?;
    let candidate = compile_replay_files(&candidate_files)
        .map_err(|err| format!("candidate compile ({}): {err}", candidate_dir.display()))?;
    let report = kir_equivalence_report(&reference, &candidate);

    let result = CompareResult {
        reference_dir: reference_dir.display().to_string(),
        candidate_dir: candidate_dir.display().to_string(),
        pruned_ledger_constructs,
        pruned_gap,
        equivalent: report.equivalent,
        report,
    };
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(result.equivalent)
}

/// All `.sysml` sources under a file-or-directory path, keyed by their
/// root-relative label (the same discovery the replay corpus modes use).
fn read_model_files(
    root: &Path,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let models = discover_models(root)?;
    if models.is_empty() {
        return Err(format!("no .sysml models found under {}", root.display()).into());
    }
    let mut files = BTreeMap::new();
    for (label, path) in models {
        let source =
            std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
        files.insert(label, source);
    }
    Ok(files)
}

/// Apply a gesture-gap prune list to a file map: drop listed declarations,
/// clear listed facets, re-render through the canonical printer. Returns the
/// pruned files plus the set of matched entries (`qname` for elements,
/// `qname#facet` for facets).
fn apply_gap_prune(
    files: &BTreeMap<String, String>,
    gap: &GapPruneList,
) -> Result<(BTreeMap<String, String>, BTreeSet<String>), Box<dyn std::error::Error>> {
    let project = load_authoring_project_from_sysml(files.clone())?;
    let render = textual_model_authoring_render_profile();
    let mut pruner = GapPruner::new(gap);
    let mut out = BTreeMap::new();
    for (path, module) in project.files() {
        let pruned = pruner.prune_module(module);
        out.insert(path.to_string(), (render.render_module)(&pruned));
    }
    Ok((out, pruner.matched))
}

struct GapPruner {
    elements: BTreeSet<String>,
    facets: BTreeMap<String, BTreeSet<String>>,
    matched: BTreeSet<String>,
}

impl GapPruner {
    fn new(gap: &GapPruneList) -> Self {
        let mut facets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for facet in &gap.facets {
            facets
                .entry(facet.element.clone())
                .or_default()
                .insert(facet.facet.clone());
        }
        Self {
            elements: gap.elements.iter().cloned().collect(),
            facets,
            matched: BTreeSet::new(),
        }
    }

    fn drop_element(&mut self, qname: &str) -> bool {
        if self.elements.contains(qname) {
            self.matched.insert(qname.to_string());
            true
        } else {
            false
        }
    }

    fn facet(&mut self, qname: &str, facet: &str) -> bool {
        if self
            .facets
            .get(qname)
            .is_some_and(|facets| facets.contains(facet))
        {
            self.matched.insert(format!("{qname}#{facet}"));
            true
        } else {
            false
        }
    }

    fn prune_module(&mut self, module: &AuthoringModule) -> AuthoringModule {
        AuthoringModule {
            package: module
                .package
                .as_ref()
                .and_then(|package| self.prune_package(package, "")),
            members: module
                .members
                .iter()
                .filter_map(|member| self.prune_declaration(member, ""))
                .collect(),
        }
    }

    fn prune_package(&mut self, package: &Package, owner: &str) -> Option<Package> {
        let name = package.name.as_dot_string();
        let qname = if owner.is_empty() {
            name
        } else {
            format!("{owner}.{name}")
        };
        if self.drop_element(&qname) {
            return None;
        }
        let drop_imports = self.facet(&qname, "imports");
        let mut pruned = package.clone();
        if self.facet(&qname, "docs") {
            pruned.docs.clear();
        }
        pruned.members = package
            .members
            .iter()
            .filter_map(|member| {
                if drop_imports && matches!(member, Declaration::Import(_)) {
                    return None;
                }
                self.prune_declaration(member, &qname)
            })
            .collect();
        Some(pruned)
    }

    fn prune_declaration(&mut self, member: &Declaration, owner: &str) -> Option<Declaration> {
        match member {
            Declaration::Package(package) => self
                .prune_package(package, owner)
                .map(Declaration::Package),
            Declaration::Import(_) => Some(member.clone()),
            Declaration::Alias(alias) => {
                if self.drop_element(&format!("{owner}.{}", alias.name)) {
                    None
                } else {
                    Some(member.clone())
                }
            }
            Declaration::Definition(definition) => {
                let qname = format!("{owner}.{}", definition.name);
                if self.drop_element(&qname) {
                    return None;
                }
                let mut pruned = definition.clone();
                if self.facet(&qname, "specializes") {
                    pruned.specializes.clear();
                }
                if self.facet(&qname, "docs") {
                    pruned.docs.clear();
                }
                pruned.members = definition
                    .members
                    .iter()
                    .filter_map(|child| self.prune_declaration(child, &qname))
                    .collect();
                Some(Declaration::Definition(pruned))
            }
            Declaration::Usage(usage) => {
                let qname = format!("{owner}.{}", usage.name);
                if self.drop_element(&qname) {
                    return None;
                }
                let mut pruned = usage.clone();
                if self.facet(&qname, "type") {
                    pruned.ty = None;
                }
                if self.facet(&qname, "expression") {
                    pruned.expression = None;
                }
                if self.facet(&qname, "multiplicity") {
                    pruned.multiplicity = None;
                }
                if self.facet(&qname, "reference_target") {
                    pruned.reference_target = None;
                }
                if self.facet(&qname, "specializes") {
                    pruned.specializes.clear();
                }
                if self.facet(&qname, "subsets") {
                    pruned.subsets.clear();
                }
                if self.facet(&qname, "redefines") {
                    pruned.redefines.clear();
                }
                if self.facet(&qname, "additional_types") {
                    pruned.additional_types.clear();
                }
                if self.facet(&qname, "docs") {
                    pruned.docs.clear();
                }
                pruned.members = usage
                    .members
                    .iter()
                    .filter_map(|child| self.prune_declaration(child, &qname))
                    .collect();
                Some(Declaration::Usage(pruned))
            }
        }
    }
}

fn run_model(
    label: &str,
    path: &Path,
) -> Result<AuthoringParityOutcome, Box<dyn std::error::Error>> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "model.sysml".to_string());
    let source =
        std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let files = BTreeMap::from([(name, source)]);
    run_authoring_parity(label, &files).map_err(|err| format!("{label}: {err}").into())
}

fn pilot_corpus_models(
    tier: &str,
    pilot_root: &Path,
) -> Result<Vec<(String, PathBuf)>, Box<dyn std::error::Error>> {
    let seed_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/pilot_corpus.seed.json");
    let seed: PilotCorpusSeed = serde_json::from_str(&std::fs::read_to_string(&seed_path)?)?;
    let files = seed.corpora.get(tier).ok_or_else(|| {
        format!(
            "unknown corpus tier `{tier}`; available: {}",
            seed.corpora.keys().cloned().collect::<Vec<_>>().join(", ")
        )
    })?;
    if !pilot_root.exists() {
        return Err(format!(
            "pilot checkout not found at {} (set MERCURIO_PILOT_ROOT or pass --pilot-root)",
            pilot_root.display()
        )
        .into());
    }
    let mut models = Vec::new();
    for file in files {
        let path = pilot_root.join(file);
        if !path.is_file() {
            return Err(format!("pilot corpus file missing: {}", path.display()).into());
        }
        models.push((file.clone(), path));
    }
    models.sort();
    models.dedup();
    Ok(models)
}

fn discover_models(model: &Path) -> Result<Vec<(String, PathBuf)>, Box<dyn std::error::Error>> {
    if model.is_file() {
        return Ok(vec![(model.display().to_string(), model.to_path_buf())]);
    }
    if !model.is_dir() {
        return Err(format!("--model path does not exist: {}", model.display()).into());
    }
    let mut models = Vec::new();
    walk(model, model, &mut models)?;
    models.sort();
    Ok(models)
}

fn walk(
    root: &Path,
    dir: &Path,
    models: &mut Vec<(String, PathBuf)>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.file_type()?.is_dir() {
            if SKIP_DIR_COMPONENTS.contains(&name.as_str()) {
                continue;
            }
            walk(root, &path, models)?;
        } else if name.ends_with(".sysml") && entry.metadata()?.len() > 0 {
            let label = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            models.push((label, path));
        }
    }
    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<Args, Box<dyn std::error::Error>> {
    let mut model = None;
    let mut corpus = None;
    let mut compare = None;
    let mut pilot_root = None;
    let mut ledger = None;
    let mut bless = false;
    let mut prune_ledger = None;
    let mut prune_gap = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--model" => {
                model = Some(PathBuf::from(iter.next().ok_or("--model requires a path")?));
            }
            "--corpus" => {
                corpus = Some(iter.next().ok_or("--corpus requires a tier name")?);
            }
            "--compare" => {
                let reference =
                    PathBuf::from(iter.next().ok_or("--compare requires two paths")?);
                let candidate =
                    PathBuf::from(iter.next().ok_or("--compare requires two paths")?);
                compare = Some((reference, candidate));
            }
            "--pilot-root" => {
                pilot_root = Some(PathBuf::from(
                    iter.next().ok_or("--pilot-root requires a path")?,
                ));
            }
            "--ledger" => {
                ledger = Some(PathBuf::from(
                    iter.next().ok_or("--ledger requires a path")?,
                ));
            }
            "--bless" => bless = true,
            "--prune-ledger" => {
                prune_ledger = Some(PathBuf::from(
                    iter.next().ok_or("--prune-ledger requires a path")?,
                ));
            }
            "--prune-gap" => {
                prune_gap = Some(PathBuf::from(
                    iter.next().ok_or("--prune-gap requires a path")?,
                ));
            }
            "--help" | "-h" => {
                return Err(
                    "usage: replay_gestures --model <file|dir> [--ledger <path> [--bless]]\n\
                     \x20      replay_gestures --corpus <tier> [--pilot-root <dir>] [--ledger <path> [--bless]]\n\
                     \x20      replay_gestures --compare <dirA> <dirB> [--prune-ledger <path>] [--prune-gap <path>]"
                        .into(),
                );
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    if bless && ledger.is_none() {
        return Err("--bless requires --ledger".into());
    }
    let mode_count =
        usize::from(model.is_some()) + usize::from(corpus.is_some()) + usize::from(compare.is_some());
    if mode_count != 1 {
        return Err("exactly one of --model, --corpus, or --compare is required".into());
    }
    if compare.is_none() && (prune_ledger.is_some() || prune_gap.is_some()) {
        return Err("--prune-ledger/--prune-gap require --compare".into());
    }
    if compare.is_some() && (ledger.is_some() || bless) {
        return Err("--ledger/--bless do not apply to --compare".into());
    }
    Ok(Args {
        model,
        corpus,
        compare,
        pilot_root: pilot_root.unwrap_or_else(default_pilot_root),
        ledger,
        bless,
        prune_ledger,
        prune_gap,
    })
}
