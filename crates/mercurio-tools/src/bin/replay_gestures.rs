//! DA-11 Tier-1 replay CLI (diagram-authoring C2.4).
//!
//! Derives a gesture script from a reference model (or every model under a
//! directory), replays it into an empty workspace through the production
//! check → apply pipeline, and prints the KIR equivalence report JSON.
//!
//! ```text
//! replay_gestures --model <file|dir> [--ledger <path> [--bless]]
//! replay_gestures --corpus <tier> [--pilot-root <dir>] [--ledger <path> [--bless]]
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

use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mercurio_sysml::replay::{AuthoringParityOutcome, CoverageLedger, run_authoring_parity};
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
    pilot_root: PathBuf,
    ledger: Option<PathBuf>,
    bless: bool,
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
    let models = match (&args.model, &args.corpus) {
        (Some(model), None) => {
            let models = discover_models(model)?;
            if models.is_empty() {
                return Err(format!("no .sysml models found under {}", model.display()).into());
            }
            models
        }
        (None, Some(tier)) => pilot_corpus_models(tier, &args.pilot_root)?,
        _ => return Err("exactly one of --model or --corpus is required".into()),
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
    let mut pilot_root = None;
    let mut ledger = None;
    let mut bless = false;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--model" => {
                model = Some(PathBuf::from(iter.next().ok_or("--model requires a path")?));
            }
            "--corpus" => {
                corpus = Some(iter.next().ok_or("--corpus requires a tier name")?);
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
            "--help" | "-h" => {
                return Err(
                    "usage: replay_gestures --model <file|dir> [--ledger <path> [--bless]]\n\
                     \x20      replay_gestures --corpus <tier> [--pilot-root <dir>] [--ledger <path> [--bless]]"
                        .into(),
                );
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    if bless && ledger.is_none() {
        return Err("--bless requires --ledger".into());
    }
    if model.is_none() && corpus.is_none() {
        return Err("one of --model or --corpus is required".into());
    }
    Ok(Args {
        model,
        corpus,
        pilot_root: pilot_root.unwrap_or_else(default_pilot_root),
        ledger,
        bless,
    })
}
