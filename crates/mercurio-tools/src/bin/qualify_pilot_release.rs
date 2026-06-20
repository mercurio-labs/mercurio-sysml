use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use mercurio_core::{KirDocument, LanguageProfile, generate_python_wrappers};
use mercurio_tools::{default_pilot_root, sha256_file, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const DEFAULT_RELEASE: &str = "2026-01";
const DEFAULT_PROFILE_ID: &str = "sysml-2.0-metamodel-0.57.0";
const DEFAULT_SPEC_VERSION: &str = "2.0.0";
const DEFAULT_CORPUS: &str = "all";
const DEFAULT_WRAPPER_MODULE: &str = "mercurio_sysml_2_0";
const REQUIRED_STDLIB_ANCHORS: &[&str] = &["Items::Item", "Parts::Part"];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    std::fs::create_dir_all(args.out.join("reports"))?;
    std::fs::create_dir_all(args.out.join("stdlib"))?;
    std::fs::create_dir_all(args.out.join("locks"))?;

    let mercurio_root = sysml_workspace_root();
    let source_lock = SourceLock {
        schema: "dev.mercurio.pilot-release-source-lock.v1",
        generated_at_utc: now_utc_rfc3339()?,
        release: args.release.clone(),
        spec_version: args.spec_version.clone(),
        profile_id: args.profile_id.clone(),
        corpus: args.corpus.clone(),
        mercurio: fingerprint_repo(&mercurio_root)?,
        pilot: fingerprint_repo(&args.pilot_root)?,
        workspace_repositories: fingerprint_workspace_repositories(&mercurio_root)?,
        inputs: SourceInputs {
            source_archive: args
                .source_archive
                .as_deref()
                .map(fingerprint_file)
                .transpose()?,
            asset_dir: args
                .asset_dir
                .as_deref()
                .map(fingerprint_tree)
                .transpose()?,
        },
    };
    write_json(&args.out.join("locks/source.lock.json"), &source_lock)?;

    let mut stages = Vec::new();
    stages.push(stage_candidate_bundle(&args)?);
    let accepted_differences = AcceptedDifferences::load(
        &args
            .out
            .join("candidate/resources/metamodels")
            .join(&args.profile_id)
            .join("conformance/accepted_differences.json"),
    )?;
    let pilot_artifacts = pilot_java_artifacts_stage(&args.pilot_root)?;
    let pilot_artifacts_ready = pilot_artifacts.status == "passed";
    stages.push(pilot_artifacts);
    if !pilot_artifacts_ready {
        if !args.skip_stdlib_build {
            stages.push(skipped_stage(
                "stdlib_build",
                "build SysML stdlib release bundle from Pilot export",
                "Pilot interactive JAR was not found",
            ));
        }
        stages.push(skipped_stage(
            "syntax_parity",
            "compare Mercurio parser syntax snapshots against the Java Pilot parser",
            "Pilot interactive JAR was not found",
        ));
        stages.push(skipped_stage(
            "semantic_parity",
            "compare Mercurio semantic snapshots against the Java Pilot compiler export",
            "Pilot interactive JAR was not found",
        ));
        stages.push(skipped_stage(
            "compile_diagnostics_parity",
            "compare Mercurio and Java Pilot compile diagnostics",
            "Pilot interactive JAR was not found",
        ));
        return write_trace_and_exit(args, source_lock, stages);
    }

    if !args.skip_stdlib_build {
        stages.push(run_stage(
            "stdlib_build",
            "build SysML stdlib release bundle from Pilot export",
            CommandSpec {
                program: cargo_program(),
                args: vec![
                    "run".to_string(),
                    "-p".to_string(),
                    "mercurio-tools".to_string(),
                    "--features".to_string(),
                    "legacy-pilot-tools".to_string(),
                    "--bin".to_string(),
                    "build_stdlib_release".to_string(),
                    "--".to_string(),
                    "--pilot-root".to_string(),
                    args.pilot_root.display().to_string(),
                    "--out".to_string(),
                    args.out.join("stdlib").display().to_string(),
                    "--spec-version".to_string(),
                    args.spec_version.clone(),
                    "--profile-id".to_string(),
                    args.profile_id.clone(),
                    "--source-id".to_string(),
                    args.release.clone(),
                    "--wrapper-module".to_string(),
                    args.wrapper_module.clone(),
                    "--audit-profile".to_string(),
                ],
                env: Vec::new(),
            },
            &mercurio_root,
            None,
        )?);
    }

    stages.push(python_wrappers_stage(
        &args.out.join("stdlib/python"),
        &args
            .out
            .join("candidate/resources/metamodels")
            .join(&args.profile_id),
        &args.wrapper_module,
        &args.profile_id,
        args.skip_stdlib_build,
    )?);

    if args.skip_parity {
        stages.push(skipped_stage(
            "syntax_parity",
            "compare Mercurio parser syntax snapshots against the Java Pilot parser",
            "parity stages skipped by --skip-parity",
        ));
        stages.push(skipped_stage(
            "semantic_parity",
            "compare Mercurio semantic snapshots against the Java Pilot compiler export",
            "parity stages skipped by --skip-parity",
        ));
        stages.push(skipped_stage(
            "compile_diagnostics_parity",
            "compare Mercurio and Java Pilot compile diagnostics",
            "parity stages skipped by --skip-parity",
        ));
        return write_trace_and_exit(args, source_lock, stages);
    }

    let candidate_stdlib_path = args
        .out
        .join("candidate/resources/metamodels")
        .join(&args.profile_id)
        .join("stdlib/stdlib.full.kir.json");
    let syntax_report = args.out.join("reports/syntax-parity.json");
    stages.push(run_parity_stage(
        "syntax_parity",
        "compare Mercurio parser syntax snapshots against the Java Pilot parser",
        CommandSpec {
            program: cargo_program(),
            args: vec![
                "run".to_string(),
                "-p".to_string(),
                "mercurio-tools".to_string(),
                "--bin".to_string(),
                "compare_pilot_ast".to_string(),
                "--".to_string(),
                "--pilot-root".to_string(),
                args.pilot_root.display().to_string(),
                "--corpus".to_string(),
                args.corpus.clone(),
                "--out".to_string(),
                syntax_report.display().to_string(),
            ],
            env: parity_stdlib_env(&candidate_stdlib_path),
        },
        &mercurio_root,
        &syntax_report,
        &accepted_differences,
    )?);

    let semantic_report = args.out.join("reports/semantic-parity.json");
    stages.push(run_parity_stage(
        "semantic_parity",
        "compare Mercurio semantic snapshots against the Java Pilot compiler export",
        CommandSpec {
            program: cargo_program(),
            args: vec![
                "run".to_string(),
                "-p".to_string(),
                "mercurio-tools".to_string(),
                "--features".to_string(),
                "legacy-pilot-tools".to_string(),
                "--bin".to_string(),
                "compare_pilot_semantics".to_string(),
                "--".to_string(),
                "--pilot-root".to_string(),
                args.pilot_root.display().to_string(),
                "--corpus".to_string(),
                args.corpus.clone(),
                "--out".to_string(),
                semantic_report.display().to_string(),
            ],
            env: parity_stdlib_env(&candidate_stdlib_path),
        },
        &mercurio_root,
        &semantic_report,
        &accepted_differences,
    )?);

    let compile_report = args.out.join("reports/compile-errors-parity.json");
    stages.push(run_parity_stage(
        "compile_diagnostics_parity",
        "compare Mercurio and Java Pilot compile diagnostics",
        CommandSpec {
            program: cargo_program(),
            args: vec![
                "run".to_string(),
                "-p".to_string(),
                "mercurio-tools".to_string(),
                "--features".to_string(),
                "legacy-pilot-tools".to_string(),
                "--bin".to_string(),
                "compare_pilot_compile_errors".to_string(),
                "--".to_string(),
                "--pilot-root".to_string(),
                args.pilot_root.display().to_string(),
                "--corpus".to_string(),
                args.corpus.clone(),
                "--out".to_string(),
                compile_report.display().to_string(),
            ],
            env: parity_stdlib_env(&candidate_stdlib_path),
        },
        &mercurio_root,
        &compile_report,
        &accepted_differences,
    )?);

    write_trace_and_exit(args, source_lock, stages)
}

fn write_trace_and_exit(
    args: Args,
    source_lock: SourceLock,
    mut stages: Vec<StageTrace>,
) -> Result<(), Box<dyn std::error::Error>> {
    append_promotion_stage(&args, &mut stages)?;
    let generated_at_utc = now_utc_rfc3339()?;
    let stage_summary = StageSummary::from_stages(&stages);
    let qualification_metrics = QualificationMetrics::from_source_and_stages(&source_lock, &stages);
    let overall_status = overall_status(&stages);
    let trace = ConformanceTrace {
        schema: "dev.mercurio.pilot-release-conformance-trace.v1",
        generated_at_utc: generated_at_utc.clone(),
        release: args.release.clone(),
        spec_version: args.spec_version.clone(),
        profile_id: args.profile_id.clone(),
        corpus: args.corpus.clone(),
        source_lock: "locks/source.lock.json".to_string(),
        overall_status,
        qualification_metrics: qualification_metrics.clone(),
        stages,
    };
    write_json(&args.out.join("reports/conformance-trace.json"), &trace)?;
    write_text(
        &args.out.join("reports/conformance-trace.md"),
        &render_conformance_trace_markdown(&trace),
    )?;
    write_promoted_conformance_trace(&args, &trace)?;

    let qualification = QualificationReport {
        schema: "dev.mercurio.pilot-release-qualification.v1",
        generated_at_utc,
        release: trace.release.clone(),
        spec_version: trace.spec_version.clone(),
        profile_id: trace.profile_id.clone(),
        corpus: trace.corpus.clone(),
        overall_status: trace.overall_status.clone(),
        source_lock: trace.source_lock.clone(),
        conformance_trace: "reports/conformance-trace.json".to_string(),
        stage_summary,
        qualification_metrics,
    };
    write_json(&args.out.join("reports/qualification.json"), &qualification)?;
    write_text(
        &args.out.join("reports/qualification.md"),
        &render_qualification_markdown(&qualification),
    )?;

    println!("pilot release qualification");
    println!("  release: {}", trace.release);
    println!("  status: {}", trace.overall_status);
    println!("  output: {}", args.out.display());

    if trace.overall_status == "passed" {
        Ok(())
    } else {
        Err("one or more qualification stages failed".into())
    }
}

#[derive(Debug)]
struct Args {
    release: String,
    pilot_root: PathBuf,
    out: PathBuf,
    profile_id: String,
    spec_version: String,
    corpus: String,
    wrapper_module: String,
    source_archive: Option<PathBuf>,
    asset_dir: Option<PathBuf>,
    skip_stdlib_build: bool,
    skip_parity: bool,
    promote_candidate: bool,
    mark_latest: bool,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut args = Self {
            release: DEFAULT_RELEASE.to_string(),
            pilot_root: default_pilot_root(),
            out: sysml_workspace_root().join("target/pilot-release-qualification"),
            profile_id: DEFAULT_PROFILE_ID.to_string(),
            spec_version: DEFAULT_SPEC_VERSION.to_string(),
            corpus: DEFAULT_CORPUS.to_string(),
            wrapper_module: DEFAULT_WRAPPER_MODULE.to_string(),
            source_archive: None,
            asset_dir: None,
            skip_stdlib_build: false,
            skip_parity: false,
            promote_candidate: false,
            mark_latest: false,
        };
        let raw = std::env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;
        let mut profile_id_explicit = false;
        while index < raw.len() {
            match raw[index].as_str() {
                "--release" => args.release = next_string(&raw, &mut index)?,
                "--pilot-root" => args.pilot_root = next_path(&raw, &mut index)?,
                "--out" => args.out = next_path(&raw, &mut index)?,
                "--profile-id" => {
                    args.profile_id = next_string(&raw, &mut index)?;
                    profile_id_explicit = true;
                }
                "--spec-version" => args.spec_version = next_string(&raw, &mut index)?,
                "--corpus" => args.corpus = next_string(&raw, &mut index)?,
                "--wrapper-module" => args.wrapper_module = next_string(&raw, &mut index)?,
                "--source-archive" => args.source_archive = Some(next_path(&raw, &mut index)?),
                "--asset-dir" => args.asset_dir = Some(next_path(&raw, &mut index)?),
                "--skip-stdlib-build" => args.skip_stdlib_build = true,
                "--skip-parity" => args.skip_parity = true,
                "--promote-candidate" => args.promote_candidate = true,
                "--mark-latest" => args.mark_latest = true,
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}").into()),
            }
            index += 1;
        }
        if !profile_id_explicit {
            args.profile_id = default_profile_id_for_release(&args.release);
        }
        if args.mark_latest && !args.promote_candidate {
            return Err("--mark-latest requires --promote-candidate".into());
        }
        Ok(args)
    }
}

#[derive(Serialize)]
struct SourceLock {
    schema: &'static str,
    generated_at_utc: String,
    release: String,
    spec_version: String,
    profile_id: String,
    corpus: String,
    mercurio: RepoFingerprint,
    pilot: RepoFingerprint,
    workspace_repositories: BTreeMap<String, RepoFingerprint>,
    inputs: SourceInputs,
}

#[derive(Serialize)]
struct SourceInputs {
    #[serde(skip_serializing_if = "Option::is_none")]
    source_archive: Option<PathFingerprint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    asset_dir: Option<TreeFingerprint>,
}

#[derive(Serialize)]
struct RepoFingerprint {
    root: String,
    commit: Option<String>,
    branch: Option<String>,
    dirty: Option<bool>,
    tracked_file_count: usize,
    tracked_tree_sha256: Option<String>,
    tracked_files: BTreeMap<String, FileFingerprint>,
}

#[derive(Serialize)]
struct FileFingerprint {
    sha256: String,
}

#[derive(Serialize)]
struct PathFingerprint {
    path: String,
    sha256: String,
    byte_len: u64,
}

#[derive(Serialize)]
struct TreeFingerprint {
    root: String,
    file_count: usize,
    tree_sha256: String,
    files: BTreeMap<String, FileFingerprint>,
}

#[derive(Serialize)]
struct ConformanceTrace {
    schema: &'static str,
    generated_at_utc: String,
    release: String,
    spec_version: String,
    profile_id: String,
    corpus: String,
    source_lock: String,
    overall_status: String,
    qualification_metrics: QualificationMetrics,
    stages: Vec<StageTrace>,
}

#[derive(Serialize)]
struct QualificationReport {
    schema: &'static str,
    generated_at_utc: String,
    release: String,
    spec_version: String,
    profile_id: String,
    corpus: String,
    overall_status: String,
    source_lock: String,
    conformance_trace: String,
    stage_summary: StageSummary,
    qualification_metrics: QualificationMetrics,
}

#[derive(Serialize)]
struct StageSummary {
    passed: usize,
    failed: usize,
    skipped: usize,
    total: usize,
    total_duration_ms: u128,
}

impl StageSummary {
    fn from_stages(stages: &[StageTrace]) -> Self {
        Self {
            passed: stages
                .iter()
                .filter(|stage| stage.status == "passed")
                .count(),
            failed: stages
                .iter()
                .filter(|stage| stage.status == "failed")
                .count(),
            skipped: stages
                .iter()
                .filter(|stage| stage.status == "skipped")
                .count(),
            total: stages.len(),
            total_duration_ms: stages.iter().map(|stage| stage.duration_ms).sum(),
        }
    }
}

#[derive(Clone, Serialize)]
struct QualificationMetrics {
    source: SourceMetricSummary,
    stages: BTreeMap<String, StageMetricSummary>,
    standard_conformance: StandardConformanceMetrics,
    candidate: Option<Value>,
    stdlib: Option<Value>,
    python: Option<Value>,
    syntax_parity: Option<Value>,
    semantic_parity: Option<Value>,
    compile_diagnostics_parity: Option<Value>,
}

impl QualificationMetrics {
    fn from_source_and_stages(source_lock: &SourceLock, stages: &[StageTrace]) -> Self {
        let mut stage_metrics = BTreeMap::new();
        let mut candidate = None;
        let mut stdlib = None;
        let mut python = None;
        let mut syntax_parity = None;
        let mut semantic_parity = None;
        let mut compile_diagnostics_parity = None;

        for stage in stages {
            let report_metrics = stage.report.as_ref().map(|report| report.metrics.clone());
            stage_metrics.insert(
                stage.name.clone(),
                StageMetricSummary {
                    status: stage.status.clone(),
                    duration_ms: stage.duration_ms,
                    exit_code: stage.exit_code,
                    report_sha256: stage.report.as_ref().map(|report| report.sha256.clone()),
                    has_error: stage.status == "failed" && stage.error.is_some(),
                },
            );

            match stage.name.as_str() {
                "candidate_staging" => candidate = report_metrics,
                "stdlib_build" => stdlib = report_metrics,
                "python_wrappers" => python = report_metrics,
                "syntax_parity" => syntax_parity = report_metrics,
                "semantic_parity" => semantic_parity = report_metrics,
                "compile_diagnostics_parity" => compile_diagnostics_parity = report_metrics,
                _ => {}
            }
        }

        Self {
            source: SourceMetricSummary::from_source_lock(source_lock),
            stages: stage_metrics,
            standard_conformance: StandardConformanceMetrics::from_stage_metrics(
                &python,
                &syntax_parity,
                &semantic_parity,
                &compile_diagnostics_parity,
            ),
            candidate,
            stdlib,
            python,
            syntax_parity,
            semantic_parity,
            compile_diagnostics_parity,
        }
    }
}

#[derive(Clone, Serialize, Default)]
struct StandardConformanceMetrics {
    python_wrappers: Option<Value>,
    syntax: Option<Value>,
    semantic: Option<Value>,
    compile_diagnostics: Option<Value>,
}

impl StandardConformanceMetrics {
    fn from_stage_metrics(
        python: &Option<Value>,
        syntax: &Option<Value>,
        semantic: &Option<Value>,
        compile_diagnostics: &Option<Value>,
    ) -> Self {
        Self {
            python_wrappers: python.as_ref().map(python_wrapper_conformance_summary),
            syntax: syntax.as_ref().map(syntax_conformance_summary),
            semantic: semantic.as_ref().map(semantic_conformance_summary),
            compile_diagnostics: compile_diagnostics
                .as_ref()
                .map(compile_diagnostics_conformance_summary),
        }
    }
}

fn overall_status(stages: &[StageTrace]) -> String {
    if stages.iter().any(|stage| stage.status == "failed") {
        "failed".to_string()
    } else {
        "passed".to_string()
    }
}

#[derive(Clone, Serialize)]
struct SourceMetricSummary {
    mercurio_commit: Option<String>,
    mercurio_branch: Option<String>,
    mercurio_dirty: Option<bool>,
    mercurio_tracked_file_count: usize,
    mercurio_tracked_tree_sha256: Option<String>,
    pilot_commit: Option<String>,
    pilot_branch: Option<String>,
    pilot_dirty: Option<bool>,
    pilot_tracked_file_count: usize,
    pilot_tracked_tree_sha256: Option<String>,
    workspace_repositories: BTreeMap<String, RepoMetricSummary>,
    source_archive_sha256: Option<String>,
    asset_tree_sha256: Option<String>,
}

impl SourceMetricSummary {
    fn from_source_lock(source_lock: &SourceLock) -> Self {
        Self {
            mercurio_commit: source_lock.mercurio.commit.clone(),
            mercurio_branch: source_lock.mercurio.branch.clone(),
            mercurio_dirty: source_lock.mercurio.dirty,
            mercurio_tracked_file_count: source_lock.mercurio.tracked_file_count,
            mercurio_tracked_tree_sha256: source_lock.mercurio.tracked_tree_sha256.clone(),
            pilot_commit: source_lock.pilot.commit.clone(),
            pilot_branch: source_lock.pilot.branch.clone(),
            pilot_dirty: source_lock.pilot.dirty,
            pilot_tracked_file_count: source_lock.pilot.tracked_file_count,
            pilot_tracked_tree_sha256: source_lock.pilot.tracked_tree_sha256.clone(),
            workspace_repositories: source_lock
                .workspace_repositories
                .iter()
                .map(|(name, fingerprint)| (name.clone(), RepoMetricSummary::from(fingerprint)))
                .collect(),
            source_archive_sha256: source_lock
                .inputs
                .source_archive
                .as_ref()
                .map(|fingerprint| fingerprint.sha256.clone()),
            asset_tree_sha256: source_lock
                .inputs
                .asset_dir
                .as_ref()
                .map(|fingerprint| fingerprint.tree_sha256.clone()),
        }
    }
}

#[derive(Clone, Serialize)]
struct RepoMetricSummary {
    root: String,
    commit: Option<String>,
    branch: Option<String>,
    dirty: Option<bool>,
    tracked_file_count: usize,
    tracked_tree_sha256: Option<String>,
}

impl From<&RepoFingerprint> for RepoMetricSummary {
    fn from(value: &RepoFingerprint) -> Self {
        Self {
            root: value.root.clone(),
            commit: value.commit.clone(),
            branch: value.branch.clone(),
            dirty: value.dirty,
            tracked_file_count: value.tracked_file_count,
            tracked_tree_sha256: value.tracked_tree_sha256.clone(),
        }
    }
}

#[derive(Clone, Serialize)]
struct StageMetricSummary {
    status: String,
    duration_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    report_sha256: Option<String>,
    has_error: bool,
}

#[derive(Serialize)]
struct StageTrace {
    name: String,
    description: &'static str,
    status: String,
    duration_ms: u128,
    command: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<ReportTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct ReportTrace {
    path: String,
    sha256: String,
    metrics: Value,
}

struct CommandSpec {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

fn run_stage(
    name: &str,
    description: &'static str,
    spec: CommandSpec,
    current_dir: &Path,
    report_path: Option<&Path>,
) -> Result<StageTrace, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let mut command = Command::new(&spec.program);
    command.args(&spec.args).current_dir(current_dir);
    for (key, value) in &spec.env {
        command.env(key, value);
    }
    let output = command.output()?;
    let duration_ms = started.elapsed().as_millis();
    let exit_code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let report = if let Some(path) = report_path {
        if path.exists() {
            Some(ReportTrace {
                path: path.display().to_string(),
                sha256: sha256_file(path)?,
                metrics: extract_report_metrics(path)?,
            })
        } else {
            None
        }
    } else {
        None
    };

    Ok(StageTrace {
        name: name.to_string(),
        description,
        status: if output.status.success() {
            "passed".to_string()
        } else {
            "failed".to_string()
        },
        duration_ms,
        command: std::iter::once(spec.program).chain(spec.args).collect(),
        exit_code,
        report,
        error: if output.status.success() || stderr.is_empty() {
            None
        } else {
            Some(stderr)
        },
    })
}

fn parity_stdlib_env(stdlib_path: &Path) -> Vec<(String, String)> {
    let path = stdlib_path.display().to_string();
    vec![
        ("MERCURIO_STDLIB_PATH".to_string(), path.clone()),
        ("MERCURIO_MODEL_LIBRARY_PATH".to_string(), path),
    ]
}

fn run_parity_stage(
    name: &str,
    description: &'static str,
    spec: CommandSpec,
    current_dir: &Path,
    report_path: &Path,
    accepted_differences: &AcceptedDifferences,
) -> Result<StageTrace, Box<dyn std::error::Error>> {
    let mut stage = run_stage(name, description, spec, current_dir, Some(report_path))?;
    if stage.status != "passed" {
        return Ok(stage);
    }

    let report_value: Value = serde_json::from_str(&std::fs::read_to_string(report_path)?)?;
    let differences = parity_differences(name, &report_value);
    let mut accepted = Vec::new();
    let mut unaccepted = Vec::new();
    for difference in differences {
        if accepted_differences.matches(&difference) {
            accepted.push(difference);
        } else {
            unaccepted.push(difference);
        }
    }

    let gate_metrics = json!({
        "total_differences": accepted.len() + unaccepted.len(),
        "accepted_differences": accepted.len(),
        "unaccepted_differences": unaccepted.len(),
        "accepted": accepted,
        "unaccepted": unaccepted,
    });
    if let Some(report) = &mut stage.report {
        if let Some(object) = report.metrics.as_object_mut() {
            object.insert("accepted_difference_gate".to_string(), gate_metrics.clone());
        }
    }
    if gate_metrics
        .get("unaccepted_differences")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        stage.status = "failed".to_string();
        stage.error = Some(format!(
            "{} unaccepted parity differences in `{name}`",
            gate_metrics["unaccepted_differences"]
        ));
    }

    Ok(stage)
}

#[derive(Debug, Clone, Serialize)]
struct ParityDifference {
    stage: String,
    case: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mercurio_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pilot_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    primary_problem: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AcceptedDifferences {
    #[serde(default)]
    differences: Vec<AcceptedDifference>,
}

impl AcceptedDifferences {
    fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if !path.exists() {
            return Ok(Self {
                differences: Vec::new(),
            });
        }
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    fn matches(&self, difference: &ParityDifference) -> bool {
        self.differences
            .iter()
            .any(|accepted| accepted.matches(difference))
    }
}

#[derive(Debug, Deserialize)]
struct AcceptedDifference {
    #[serde(default)]
    stage: Option<String>,
    #[serde(default, rename = "case")]
    case_path: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    mercurio_status: Option<String>,
    #[serde(default)]
    pilot_status: Option<String>,
    #[serde(default)]
    primary_problem: Option<String>,
    #[serde(default)]
    message_contains: Option<String>,
}

impl AcceptedDifference {
    fn matches(&self, difference: &ParityDifference) -> bool {
        optional_eq(self.stage.as_deref(), &difference.stage)
            && optional_eq(self.case_path.as_deref(), &difference.case)
            && optional_eq(self.kind.as_deref(), &difference.kind)
            && optional_eq_option(self.mercurio_status.as_deref(), &difference.mercurio_status)
            && optional_eq_option(self.pilot_status.as_deref(), &difference.pilot_status)
            && optional_eq_option(self.primary_problem.as_deref(), &difference.primary_problem)
            && self.message_contains.as_ref().is_none_or(|needle| {
                difference
                    .message
                    .as_ref()
                    .is_some_and(|message| message.contains(needle))
            })
    }
}

fn optional_eq(expected: Option<&str>, actual: &str) -> bool {
    expected.is_none_or(|expected| expected == actual)
}

fn optional_eq_option(expected: Option<&str>, actual: &Option<String>) -> bool {
    expected.is_none_or(|expected| actual.as_deref() == Some(expected))
}

fn parity_differences(stage: &str, report: &Value) -> Vec<ParityDifference> {
    let mut differences = Vec::new();
    let Some(cases) = report.get("cases").and_then(Value::as_array) else {
        return differences;
    };

    for case in cases {
        let case_path = case
            .get("relative_path")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>")
            .to_string();
        match stage {
            "syntax_parity" => append_syntax_differences(&mut differences, stage, &case_path, case),
            "semantic_parity" => {
                append_semantic_differences(&mut differences, stage, &case_path, case)
            }
            "compile_diagnostics_parity" => {
                append_compile_differences(&mut differences, stage, &case_path, case)
            }
            _ => {}
        }
    }

    differences
}

fn append_syntax_differences(
    differences: &mut Vec<ParityDifference>,
    stage: &str,
    case_path: &str,
    case: &Value,
) {
    if case.get("status").and_then(Value::as_str) == Some("error") {
        differences.push(ParityDifference {
            stage: stage.to_string(),
            case: case_path.to_string(),
            kind: "syntax_error".to_string(),
            message: case
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_string),
            mercurio_status: None,
            pilot_status: None,
            primary_problem: None,
        });
        return;
    }
    if case.get("exact").and_then(Value::as_bool) == Some(false) {
        for (field, kind) in [
            ("mismatches", "syntax_mismatch"),
            ("rust_only", "syntax_rust_only"),
            ("pilot_only", "syntax_pilot_only"),
        ] {
            let count = case.get(field).and_then(Value::as_u64).unwrap_or(0);
            if count > 0 {
                differences.push(ParityDifference {
                    stage: stage.to_string(),
                    case: case_path.to_string(),
                    kind: kind.to_string(),
                    message: Some(format!("{field}={count}")),
                    mercurio_status: None,
                    pilot_status: None,
                    primary_problem: None,
                });
            }
        }
    }
}

fn append_semantic_differences(
    differences: &mut Vec<ParityDifference>,
    stage: &str,
    case_path: &str,
    case: &Value,
) {
    if case.get("status").and_then(Value::as_str) == Some("error") {
        differences.push(ParityDifference {
            stage: stage.to_string(),
            case: case_path.to_string(),
            kind: "semantic_error".to_string(),
            message: case
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_string),
            mercurio_status: Some("error".to_string()),
            pilot_status: None,
            primary_problem: None,
        });
        return;
    }
    if case.get("exact").and_then(Value::as_bool) == Some(false) {
        for (field, kind) in [
            ("mismatches", "semantic_mismatch"),
            ("mercurio_only", "semantic_mercurio_only"),
            ("pilot_only", "semantic_pilot_only"),
        ] {
            let count = case.get(field).and_then(Value::as_u64).unwrap_or(0);
            if count > 0 {
                differences.push(ParityDifference {
                    stage: stage.to_string(),
                    case: case_path.to_string(),
                    kind: kind.to_string(),
                    message: Some(format!("{field}={count}")),
                    mercurio_status: None,
                    pilot_status: None,
                    primary_problem: None,
                });
            }
        }
    }
}

fn append_compile_differences(
    differences: &mut Vec<ParityDifference>,
    stage: &str,
    case_path: &str,
    case: &Value,
) {
    if case.get("status").and_then(Value::as_str) == Some("error") {
        differences.push(ParityDifference {
            stage: stage.to_string(),
            case: case_path.to_string(),
            kind: "compile_report_error".to_string(),
            message: case
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_string),
            mercurio_status: None,
            pilot_status: None,
            primary_problem: None,
        });
        return;
    }

    let comparison = case.get("comparison").unwrap_or(&Value::Null);
    if comparison
        .get("status_match")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        return;
    }
    differences.push(ParityDifference {
        stage: stage.to_string(),
        case: case_path.to_string(),
        kind: "compile_status_mismatch".to_string(),
        message: None,
        mercurio_status: case
            .get("mercurio_status")
            .and_then(Value::as_str)
            .map(str::to_string),
        pilot_status: case
            .get("pilot_status")
            .and_then(Value::as_str)
            .map(str::to_string),
        primary_problem: comparison
            .get("mercurio_primary_problem")
            .and_then(Value::as_str)
            .map(str::to_string),
    });
}

fn pilot_java_artifacts_stage(pilot_root: &Path) -> Result<StageTrace, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let target_dir = pilot_root.join("org.omg.sysml.interactive/target");
    let jar = std::fs::read_dir(&target_dir).ok().and_then(|entries| {
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| {
                        name.starts_with("org.omg.sysml.interactive-") && name.ends_with("-all.jar")
                    })
                    .unwrap_or(false)
            })
            .max()
    });
    let report = if let Some(path) = &jar {
        Some(ReportTrace {
            path: path.display().to_string(),
            sha256: sha256_file(path)?,
            metrics: json!({
                "artifact": "org.omg.sysml.interactive-*-all.jar"
            }),
        })
    } else {
        None
    };

    Ok(StageTrace {
        name: "pilot_java_artifacts".to_string(),
        description: "verify the Pilot checkout has built Java artifacts required for parity export",
        status: if jar.is_some() {
            "passed".to_string()
        } else {
            "failed".to_string()
        },
        duration_ms: started.elapsed().as_millis(),
        command: Vec::new(),
        exit_code: None,
        report,
        error: if jar.is_some() {
            None
        } else {
            Some(format!(
                "missing org.omg.sysml.interactive-*-all.jar under {}",
                target_dir.display()
            ))
        },
    })
}

fn stage_candidate_bundle(args: &Args) -> Result<StageTrace, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let requested_source_root = sysml_workspace_root()
        .join("resources/metamodels")
        .join(&args.profile_id);
    let template_root = sysml_workspace_root()
        .join("resources/metamodels")
        .join(DEFAULT_PROFILE_ID);
    let source_root = if requested_source_root.is_dir() {
        requested_source_root.as_path()
    } else {
        template_root.as_path()
    };
    let candidate_root = args
        .out
        .join("candidate/resources/metamodels")
        .join(&args.profile_id);
    let report_path = args.out.join("reports/candidate-staging.json");
    let mut errors = Vec::new();

    if !source_root.is_dir() {
        errors.push(format!(
            "source profile resources not found at {}",
            source_root.display()
        ));
    } else {
        copy_candidate_profile(source_root, &candidate_root, args, &mut errors);
        copy_candidate_metamodel(source_root, &candidate_root, args, &mut errors);
        copy_optional_file(
            &source_root,
            &candidate_root,
            "provenance.json",
            &mut errors,
        );
        copy_required_dir(&source_root, &candidate_root, "mappings", &mut errors);

        let generated_stdlib = args.out.join("stdlib");
        let bundled_stdlib = source_root.join("stdlib");
        let stdlib_source = select_candidate_stdlib_source(&generated_stdlib, &bundled_stdlib);
        copy_required_dir_from(&stdlib_source, &candidate_root.join("stdlib"), &mut errors);
        validate_candidate_stdlib(&candidate_root.join("stdlib"), &mut errors);
        write_candidate_resource_provenance(&candidate_root, args, source_root, &stdlib_source)?;
    }

    let conformance_dir = candidate_root.join("conformance");
    if let Err(err) = std::fs::create_dir_all(&conformance_dir) {
        errors.push(format!(
            "failed to create conformance dir {}: {err}",
            conformance_dir.display()
        ));
    } else {
        write_json(
            &conformance_dir.join("accepted_differences.json"),
            &json!({
                "schema": "dev.mercurio.pilot-accepted-differences.v1",
                "release": args.release,
                "differences": []
            }),
        )?;
        write_json(
            &conformance_dir.join("conformance-trace.json"),
            &json!({
                "schema": "dev.mercurio.pilot-release-conformance-trace-ref.v1",
                "release": args.release,
                "trace": "../../../../reports/conformance-trace.json"
            }),
        )?;
    }

    let registry_entry = json!({
        "id": args.profile_id,
        "release": args.release,
        "selector": args.release,
        "display_name": format!("SysML v2 ({})", args.release),
        "sysml_version": args.spec_version,
        "kerml_version": "1.0",
        "metamodel_version": args.release,
        "status": "supported",
        "legacy_ids": [],
        "aliases": [
            args.release,
            format!("pilot-{}", args.release)
        ],
        "bundle": {
            "profile": { "path": "profile.json" },
            "stdlib": {
                "locator": "file:stdlib/stdlib.full.kir.json",
                "rulepack": "stdlib/stdlib.rulepack.json"
            },
            "mappings": {
                "path": "mappings",
                "metamodel_constructs": "mappings/metamodel_constructs.seed.json",
                "kir_emission": "mappings/kir_emission.seed.json",
                "lowering_rules": "mappings/lowering_rules.seed.json",
                "semantic_defaults": "mappings/semantic_defaults.seed.json"
            },
            "conformance": {
                "accepted_differences": "conformance/accepted_differences.json",
                "trace": "conformance/conformance-trace.json"
            },
            "python": {
                "wrapper_module": args.wrapper_module
            }
        }
    });
    let registry = json!([registry_entry]);
    let candidate_registry = args
        .out
        .join("candidate/resources/metamodels/registry.json");
    write_json(&candidate_registry, &registry)?;

    let candidate_files = if candidate_root.exists() {
        collect_files(&candidate_root)?
    } else {
        Vec::new()
    };
    let candidate_tree_sha256 = if candidate_files.is_empty() {
        None
    } else {
        Some(digest_paths(&candidate_root, &candidate_files)?)
    };
    write_json(
        &report_path,
        &json!({
            "schema": "dev.mercurio.pilot-candidate-staging.v1",
            "release": args.release,
            "profile_id": args.profile_id,
            "candidate_root": candidate_root.display().to_string(),
            "registry": candidate_registry.display().to_string(),
            "file_count": candidate_files.len(),
            "tree_sha256": candidate_tree_sha256,
            "errors": errors
        }),
    )?;

    let report = Some(ReportTrace {
        path: report_path.display().to_string(),
        sha256: sha256_file(&report_path)?,
        metrics: json!({
            "candidate_root": candidate_root.display().to_string(),
            "file_count": candidate_files.len(),
            "tree_sha256": candidate_tree_sha256,
        }),
    });

    Ok(StageTrace {
        name: "candidate_staging".to_string(),
        description: "stage release candidate resources without promoting them",
        status: if errors.is_empty() {
            "passed".to_string()
        } else {
            "failed".to_string()
        },
        duration_ms: started.elapsed().as_millis(),
        command: Vec::new(),
        exit_code: None,
        report,
        error: if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        },
    })
}

#[cfg(test)]
fn copy_required_file(
    source_root: &Path,
    dest_root: &Path,
    relative: &str,
    errors: &mut Vec<String>,
) {
    let source = source_root.join(relative);
    let dest = dest_root.join(relative);
    if !source.is_file() {
        errors.push(format!("missing required file {}", source.display()));
        return;
    }
    if let Err(err) = copy_file(&source, &dest) {
        errors.push(format!(
            "failed to copy {} to {}: {err}",
            source.display(),
            dest.display()
        ));
    }
}

fn copy_candidate_profile(
    source_root: &Path,
    dest_root: &Path,
    args: &Args,
    errors: &mut Vec<String>,
) {
    let source = source_root.join("profile.json");
    let dest = dest_root.join("profile.json");
    if !source.is_file() {
        errors.push(format!("missing required file {}", source.display()));
        return;
    }
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut profile: Value = serde_json::from_str(&std::fs::read_to_string(&source)?)?;
        if let Some(object) = profile.as_object_mut() {
            object.insert("id".to_string(), Value::String(args.profile_id.clone()));
            object.insert(
                "stdlib_path".to_string(),
                Value::String(format!(
                    "resources/metamodels/{}/stdlib/stdlib.full.kir.json",
                    args.profile_id
                )),
            );
        }
        write_json(&dest, &profile)?;
        Ok(())
    })();
    if let Err(err) = result {
        errors.push(format!(
            "failed to copy {} to {}: {err}",
            source.display(),
            dest.display()
        ));
    }
}

fn copy_candidate_metamodel(
    source_root: &Path,
    dest_root: &Path,
    args: &Args,
    errors: &mut Vec<String>,
) {
    let source = source_root.join("metamodel.json");
    if !source.exists() {
        return;
    }
    let dest = dest_root.join("metamodel.json");
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut descriptor: Value = serde_json::from_str(&std::fs::read_to_string(&source)?)?;
        if let Some(object) = descriptor.as_object_mut() {
            object.insert("id".to_string(), Value::String(args.profile_id.clone()));
            object.insert(
                "display_name".to_string(),
                Value::String(format!("SysML v2 ({})", args.release)),
            );
            object.insert(
                "metamodel_version".to_string(),
                Value::String(args.release.clone()),
            );
            object.insert("status".to_string(), Value::String("supported".to_string()));
            object.insert("release".to_string(), Value::String(args.release.clone()));
            object.insert("selector".to_string(), Value::String(args.release.clone()));
            object.insert("legacy_ids".to_string(), Value::Array(Vec::new()));
            object.insert(
                "stdlib_path".to_string(),
                Value::String("stdlib/stdlib.full.kir.json".to_string()),
            );
            object.insert(
                "sysml_delta_path".to_string(),
                Value::String("stdlib/sysml-library.kir.json".to_string()),
            );
        }
        write_json(&dest, &descriptor)?;
        Ok(())
    })();
    if let Err(err) = result {
        errors.push(format!(
            "failed to copy {} to {}: {err}",
            source.display(),
            dest.display()
        ));
    }
}

fn select_candidate_stdlib_source(generated_stdlib: &Path, bundled_stdlib: &Path) -> PathBuf {
    if candidate_stdlib_has_required_anchors(generated_stdlib) {
        generated_stdlib.to_path_buf()
    } else {
        bundled_stdlib.to_path_buf()
    }
}

fn validate_candidate_stdlib(stdlib_root: &Path, errors: &mut Vec<String>) {
    let stdlib_path = stdlib_root.join("stdlib.full.kir.json");
    if !stdlib_path.is_file() {
        errors.push(format!(
            "candidate stdlib is missing {}",
            stdlib_path.display()
        ));
        return;
    }
    if !candidate_stdlib_has_required_anchors(stdlib_root) {
        errors.push(format!(
            "candidate stdlib {} is missing required anchors: {}",
            stdlib_path.display(),
            REQUIRED_STDLIB_ANCHORS.join(", ")
        ));
    }
}

fn write_candidate_resource_provenance(
    candidate_root: &Path,
    args: &Args,
    source_root: &Path,
    stdlib_source: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let source_profile_id = source_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(DEFAULT_PROFILE_ID);
    let stdlib_files = collect_files(stdlib_source)?;
    let stdlib_tree_sha256 = digest_paths(stdlib_source, &stdlib_files)?;
    write_json(
        &candidate_root.join("provenance.json"),
        &json!({
            "schema": "dev.mercurio.pilot-release-resource-provenance.v1",
            "release": args.release,
            "selector": args.release,
            "profile_id": args.profile_id,
            "spec_version": args.spec_version,
            "source_profile_id": source_profile_id,
            "source_profile_root": path_to_slash_path(source_root),
            "stdlib_source_root": path_to_slash_path(stdlib_source),
            "stdlib_source_file_count": stdlib_files.len(),
            "stdlib_source_tree_sha256": stdlib_tree_sha256,
            "stdlib_path": format!(
                "resources/metamodels/{}/stdlib/stdlib.full.kir.json",
                args.profile_id
            ),
            "wrapper_module": args.wrapper_module
        }),
    )?;
    write_json(
        &candidate_root.join("stdlib/source.lock.json"),
        &json!({
            "schema": "dev.mercurio.pilot-release-candidate-stdlib-source-lock.v1",
            "release": args.release,
            "profile_id": args.profile_id,
            "source_profile_id": source_profile_id,
            "source_root": path_to_slash_path(stdlib_source),
            "file_count": stdlib_files.len(),
            "tree_sha256": stdlib_tree_sha256
        }),
    )?;
    write_json(
        &candidate_root.join("stdlib/release.lock.json"),
        &json!({
            "schema": "dev.mercurio.pilot-release-candidate-stdlib-lock.v1",
            "release": args.release,
            "profile_id": args.profile_id,
            "spec_version": args.spec_version,
            "wrapper_module": args.wrapper_module,
            "source_profile_id": source_profile_id,
            "artifacts": {
                "pilot_stdlib_export": fingerprint_optional_candidate_file(candidate_root, "stdlib/pilot-stdlib-export.json")?,
                "stdlib_full_kir": fingerprint_optional_candidate_file(candidate_root, "stdlib/stdlib.full.kir.json")?,
                "stdlib_delta_kir": fingerprint_optional_candidate_file(candidate_root, "stdlib/stdlib.kir.json")?,
                "sysml_library_kir": fingerprint_optional_candidate_file(candidate_root, "stdlib/sysml-library.kir.json")?,
                "rulepack": fingerprint_optional_candidate_file(candidate_root, "stdlib/stdlib.rulepack.json")?
            }
        }),
    )?;
    Ok(())
}

fn fingerprint_optional_candidate_file(
    candidate_root: &Path,
    relative: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let path = candidate_root.join(path_from_slashes(relative));
    if !path.exists() {
        return Ok(Value::Null);
    }
    Ok(json!({
        "path": relative,
        "sha256": sha256_file(&path)?,
        "byte_len": std::fs::metadata(&path)?.len()
    }))
}

fn candidate_stdlib_has_required_anchors(stdlib_root: &Path) -> bool {
    let stdlib_path = stdlib_root.join("stdlib.full.kir.json");
    let Ok(raw) = std::fs::read_to_string(&stdlib_path) else {
        return false;
    };
    let Ok(document) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    let Some(elements) = document.get("elements").and_then(Value::as_array) else {
        return false;
    };
    REQUIRED_STDLIB_ANCHORS.iter().all(|required| {
        elements.iter().any(|element| {
            element
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| id == *required)
        })
    })
}

fn copy_optional_file(
    source_root: &Path,
    dest_root: &Path,
    relative: &str,
    errors: &mut Vec<String>,
) {
    let source = source_root.join(relative);
    if !source.exists() {
        return;
    }
    let dest = dest_root.join(relative);
    if let Err(err) = copy_file(&source, &dest) {
        errors.push(format!(
            "failed to copy {} to {}: {err}",
            source.display(),
            dest.display()
        ));
    }
}

fn copy_required_dir(
    source_root: &Path,
    dest_root: &Path,
    relative: &str,
    errors: &mut Vec<String>,
) {
    copy_required_dir_from(
        &source_root.join(relative),
        &dest_root.join(relative),
        errors,
    );
}

fn copy_required_dir_from(source: &Path, dest: &Path, errors: &mut Vec<String>) {
    if !source.is_dir() {
        errors.push(format!("missing required directory {}", source.display()));
        return;
    }
    if let Err(err) = copy_dir_recursive(source, dest) {
        errors.push(format!(
            "failed to copy directory {} to {}: {err}",
            source.display(),
            dest.display()
        ));
    }
}

fn copy_file(source: &Path, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, dest)?;
    Ok(())
}

fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &dest_path)?;
        } else if source_path.is_file() {
            copy_file(&source_path, &dest_path)?;
        }
    }
    Ok(())
}

fn append_promotion_stage(
    args: &Args,
    stages: &mut Vec<StageTrace>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !args.promote_candidate {
        return Ok(());
    }

    if !stages.iter().all(|stage| stage.status == "passed") {
        stages.push(skipped_stage(
            "candidate_promotion",
            "promote staged release candidate resources into the repository",
            "promotion skipped because qualification gates did not all pass",
        ));
        return Ok(());
    }

    stages.push(promote_candidate_stage(args)?);
    Ok(())
}

fn promote_candidate_stage(args: &Args) -> Result<StageTrace, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let candidate_resources_root = args.out.join("candidate/resources/metamodels");
    let repo_metamodels_root = sysml_workspace_root().join("resources/metamodels");
    let report_path = args.out.join("reports/candidate-promotion.json");
    let result = promote_candidate_resources(
        &candidate_resources_root,
        &repo_metamodels_root,
        &args.profile_id,
        args.mark_latest,
    );
    let duration_ms = started.elapsed().as_millis();

    match result {
        Ok(report) => {
            write_json(&report_path, &report)?;
            Ok(StageTrace {
                name: "candidate_promotion".to_string(),
                description: "promote staged release candidate resources into the repository",
                status: "passed".to_string(),
                duration_ms,
                command: Vec::new(),
                exit_code: None,
                report: Some(ReportTrace {
                    path: report_path.display().to_string(),
                    sha256: sha256_file(&report_path)?,
                    metrics: json!({
                        "promoted_root": report.promoted_root,
                        "registry": report.registry,
                        "marked_latest": report.marked_latest,
                        "file_count": report.file_count,
                    }),
                }),
                error: None,
            })
        }
        Err(err) => Ok(StageTrace {
            name: "candidate_promotion".to_string(),
            description: "promote staged release candidate resources into the repository",
            status: "failed".to_string(),
            duration_ms,
            command: Vec::new(),
            exit_code: None,
            report: None,
            error: Some(err.to_string()),
        }),
    }
}

#[derive(Debug, Serialize)]
struct CandidatePromotionReport {
    schema: &'static str,
    profile_id: String,
    promoted_root: String,
    registry: String,
    marked_latest: bool,
    file_count: usize,
    tree_sha256: String,
}

fn promote_candidate_resources(
    candidate_resources_root: &Path,
    repo_metamodels_root: &Path,
    profile_id: &str,
    mark_latest: bool,
) -> Result<CandidatePromotionReport, Box<dyn std::error::Error>> {
    let candidate_profile_root = candidate_resources_root.join(profile_id);
    if !candidate_profile_root.is_dir() {
        return Err(format!(
            "candidate profile root does not exist: {}",
            candidate_profile_root.display()
        )
        .into());
    }

    let repo_profile_root = repo_metamodels_root.join(profile_id);
    if repo_profile_root.exists() {
        return Err(format!(
            "refusing to overwrite existing promoted profile directory: {}",
            repo_profile_root.display()
        )
        .into());
    }

    validate_registry_promotion(
        &candidate_resources_root.join("registry.json"),
        &repo_metamodels_root.join("registry.json"),
        profile_id,
    )?;
    copy_dir_recursive(&candidate_profile_root, &repo_profile_root)?;
    promote_registry_entry(
        &candidate_resources_root.join("registry.json"),
        &repo_metamodels_root.join("registry.json"),
        profile_id,
        mark_latest,
    )?;

    let files = collect_files(&repo_profile_root)?;
    let tree_sha256 = digest_paths(&repo_profile_root, &files)?;
    Ok(CandidatePromotionReport {
        schema: "dev.mercurio.pilot-candidate-promotion.v1",
        profile_id: profile_id.to_string(),
        promoted_root: repo_profile_root.display().to_string(),
        registry: repo_metamodels_root
            .join("registry.json")
            .display()
            .to_string(),
        marked_latest: mark_latest,
        file_count: files.len(),
        tree_sha256,
    })
}

fn validate_registry_promotion(
    candidate_registry_path: &Path,
    repo_registry_path: &Path,
    profile_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let candidate_registry: Value =
        serde_json::from_str(&std::fs::read_to_string(candidate_registry_path)?)?;
    let candidate_exists = candidate_registry.as_array().is_some_and(|entries| {
        entries
            .iter()
            .any(|entry| entry.get("id").and_then(Value::as_str) == Some(profile_id))
    });
    if !candidate_exists {
        return Err(format!("candidate registry does not contain `{profile_id}`").into());
    }

    if repo_registry_path.exists() {
        let repo_registry: Value =
            serde_json::from_str(&std::fs::read_to_string(repo_registry_path)?)?;
        let entries = repo_registry
            .as_array()
            .ok_or("repo metamodel registry must be a JSON array")?;
        if entries
            .iter()
            .any(|entry| entry.get("id").and_then(Value::as_str) == Some(profile_id))
        {
            return Err(format!("registry already contains `{profile_id}`").into());
        }
    }
    Ok(())
}

fn promote_registry_entry(
    candidate_registry_path: &Path,
    repo_registry_path: &Path,
    profile_id: &str,
    mark_latest: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let candidate_registry: Value =
        serde_json::from_str(&std::fs::read_to_string(candidate_registry_path)?)?;
    let mut candidate_entry = candidate_registry
        .as_array()
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.get("id").and_then(Value::as_str) == Some(profile_id))
        })
        .cloned()
        .ok_or_else(|| format!("candidate registry does not contain `{profile_id}`"))?;

    if let Some(object) = candidate_entry.as_object_mut() {
        object.insert(
            "status".to_string(),
            Value::String(if mark_latest { "latest" } else { "supported" }.to_string()),
        );
    }

    let mut repo_registry: Value = if repo_registry_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(repo_registry_path)?)?
    } else {
        Value::Array(Vec::new())
    };
    let entries = repo_registry
        .as_array_mut()
        .ok_or("repo metamodel registry must be a JSON array")?;
    if entries
        .iter()
        .any(|entry| entry.get("id").and_then(Value::as_str) == Some(profile_id))
    {
        return Err(format!("registry already contains `{profile_id}`").into());
    }
    if mark_latest {
        for entry in entries.iter_mut() {
            if entry.get("status").and_then(Value::as_str) == Some("latest")
                && let Some(object) = entry.as_object_mut()
            {
                object.insert("status".to_string(), Value::String("supported".to_string()));
            }
        }
    }
    entries.push(candidate_entry);
    write_json(repo_registry_path, &repo_registry)?;
    Ok(())
}

fn write_promoted_conformance_trace(
    args: &Args,
    trace: &ConformanceTrace,
) -> Result<(), Box<dyn std::error::Error>> {
    if !args.promote_candidate
        || !trace
            .stages
            .iter()
            .any(|stage| stage.name == "candidate_promotion" && stage.status == "passed")
    {
        return Ok(());
    }

    let conformance_root = sysml_workspace_root()
        .join("resources/metamodels")
        .join(&args.profile_id)
        .join("conformance");
    write_json(&conformance_root.join("conformance-trace.json"), trace)?;
    write_text(
        &conformance_root.join("conformance-trace.md"),
        &render_conformance_trace_markdown(trace),
    )?;
    Ok(())
}

fn python_wrappers_stage(
    python_root: &Path,
    candidate_profile_root: &Path,
    wrapper_module: &str,
    expected_profile_id: &str,
    skipped_stdlib_build: bool,
) -> Result<StageTrace, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let module_root = python_root.join(wrapper_module);
    if !module_root.exists() {
        if skipped_stdlib_build {
            generate_trace_python_wrappers(python_root, candidate_profile_root, wrapper_module)?;
        }
        if !module_root.exists() {
            return Ok(StageTrace {
                name: "python_wrappers".to_string(),
                description: "verify generated Python stdlib wrapper package",
                status: "failed".to_string(),
                duration_ms: started.elapsed().as_millis(),
                command: Vec::new(),
                exit_code: None,
                report: None,
                error: Some(format!(
                    "missing generated Python wrapper module at {}",
                    module_root.display()
                )),
            });
        }
    }

    let required = [
        "__init__.py",
        "base.py",
        "concepts.py",
        "generation_info.py",
        "metamodel.py",
        "py.typed",
        "stdlib/__init__.py",
        "stdlib/isq.py",
        "stdlib/si.py",
    ];
    let mut errors = Vec::new();
    for relative in required {
        if !module_root.join(relative).exists() {
            errors.push(format!("missing {wrapper_module}/{relative}"));
        }
    }

    let generation_info = module_root.join("generation_info.py");
    if generation_info.exists() {
        let content = std::fs::read_to_string(&generation_info)?;
        if !content.contains(&format!("PROFILE_ID = {expected_profile_id:?}")) {
            errors.push(format!(
                "generation_info.py does not declare PROFILE_ID = {expected_profile_id:?}"
            ));
        }
    }

    let files = collect_files(python_root)?;
    let py_files = files
        .iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "py"))
        .cloned()
        .collect::<Vec<_>>();
    let mut py_compile_exit_code = None;
    let mut command = Vec::new();
    if !py_files.is_empty() {
        let python = std::env::var("PYTHON").unwrap_or_else(|_| "python".to_string());
        let mut spec_args = vec!["-m".to_string(), "py_compile".to_string()];
        spec_args.extend(py_files.iter().map(|path| path.display().to_string()));
        command = std::iter::once(python.clone())
            .chain(spec_args.clone())
            .collect();
        match Command::new(&python).args(&spec_args).output() {
            Ok(output) => {
                py_compile_exit_code = output.status.code();
                if !output.status.success() {
                    errors.push(format!(
                        "python py_compile failed: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ));
                }
            }
            Err(err) => {
                errors.push(format!("failed to launch Python for py_compile: {err}"));
            }
        }
    }

    let tree_sha256 = digest_paths(python_root, &files)?;
    let report = ReportTrace {
        path: python_root.display().to_string(),
        sha256: tree_sha256,
        metrics: json!({
            "module": wrapper_module,
            "file_count": files.len(),
            "python_file_count": py_files.len(),
            "metamodel_class_count": count_metamodel_classes(&module_root.join("metamodel.py"))?,
            "stdlib_catalog_entries": {
                "isq": count_catalog_entries(&module_root.join("stdlib/isq.py"))?,
                "si": count_catalog_entries(&module_root.join("stdlib/si.py"))?
            },
            "py_compile_exit_code": py_compile_exit_code
        }),
    };

    Ok(StageTrace {
        name: "python_wrappers".to_string(),
        description: "verify generated Python stdlib wrapper package",
        status: if errors.is_empty() {
            "passed".to_string()
        } else {
            "failed".to_string()
        },
        duration_ms: started.elapsed().as_millis(),
        command,
        exit_code: py_compile_exit_code,
        report: Some(report),
        error: if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        },
    })
}

fn generate_trace_python_wrappers(
    python_root: &Path,
    candidate_profile_root: &Path,
    wrapper_module: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let profile = LanguageProfile::from_path(&candidate_profile_root.join("profile.json"))?;
    let stdlib = KirDocument::from_path(
        &candidate_profile_root
            .join("stdlib")
            .join("stdlib.full.kir.json"),
    )?;
    let generated = generate_python_wrappers(&stdlib, &profile, wrapper_module);
    for (relative, content) in generated.files {
        write_text(&python_root.join(path_from_slashes(&relative)), &content)?;
    }
    Ok(())
}

fn skipped_stage(name: &str, description: &'static str, reason: &str) -> StageTrace {
    StageTrace {
        name: name.to_string(),
        description,
        status: "skipped".to_string(),
        duration_ms: 0,
        command: Vec::new(),
        exit_code: None,
        report: None,
        error: Some(reason.to_string()),
    }
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            for entry in std::fs::read_dir(&path)? {
                stack.push(entry?.path());
            }
        } else if path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn digest_paths(root: &Path, paths: &[PathBuf]) -> Result<String, Box<dyn std::error::Error>> {
    let mut material = String::new();
    for path in paths {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        material.push_str(&relative);
        material.push('\0');
        material.push_str(&sha256_file(path)?);
        material.push('\n');
    }
    Ok(sha256_hex(material.as_bytes()))
}

fn count_metamodel_classes(path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(0);
    }
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|line| line.starts_with("class ") && line.contains("(ElementView):"))
        .count())
}

fn count_catalog_entries(path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(0);
    }
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|line| line.starts_with("    ") && line.contains("StdlibRef("))
        .count())
}

fn fingerprint_file(path: &Path) -> Result<PathFingerprint, Box<dyn std::error::Error>> {
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(format!("source archive is not a file: {}", path.display()).into());
    }
    Ok(PathFingerprint {
        path: path.display().to_string(),
        sha256: sha256_file(path)?,
        byte_len: metadata.len(),
    })
}

fn fingerprint_tree(root: &Path) -> Result<TreeFingerprint, Box<dyn std::error::Error>> {
    if !root.is_dir() {
        return Err(format!("asset dir is not a directory: {}", root.display()).into());
    }
    let paths = collect_files(root)?;
    let mut files = BTreeMap::new();
    for path in &paths {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        files.insert(
            relative,
            FileFingerprint {
                sha256: sha256_file(path)?,
            },
        );
    }
    let tree_sha256 = digest_paths(root, &paths)?;
    Ok(TreeFingerprint {
        root: root.display().to_string(),
        file_count: files.len(),
        tree_sha256,
        files,
    })
}

fn fingerprint_repo(path: &Path) -> Result<RepoFingerprint, Box<dyn std::error::Error>> {
    let tracked = git_stdout(path, &["ls-files"])?;
    let mut files = BTreeMap::new();
    let mut tree_chunks = Vec::new();
    for relative in tracked.lines().filter(|line| !line.trim().is_empty()) {
        let file_path = path.join(relative);
        if file_path.is_file() {
            let sha256 = sha256_file(&file_path)?;
            tree_chunks.push(format!("{relative}\0{sha256}\n"));
            files.insert(relative.to_string(), FileFingerprint { sha256 });
        }
    }
    let tree_material = tree_chunks.concat();
    Ok(RepoFingerprint {
        root: path.display().to_string(),
        commit: git_stdout(path, &["rev-parse", "HEAD"]).ok(),
        branch: git_stdout(path, &["branch", "--show-current"]).ok(),
        dirty: git_stdout(path, &["status", "--porcelain"])
            .ok()
            .map(|value| !value.trim().is_empty()),
        tracked_file_count: files.len(),
        tracked_tree_sha256: if tree_material.is_empty() {
            None
        } else {
            Some(sha256_hex(tree_material.as_bytes()))
        },
        tracked_files: files,
    })
}

fn fingerprint_workspace_repositories(
    mercurio_root: &Path,
) -> Result<BTreeMap<String, RepoFingerprint>, Box<dyn std::error::Error>> {
    let workspace_root = mercurio_root
        .parent()
        .ok_or("mercurio workspace root has no parent")?;
    let mut repositories = BTreeMap::new();
    for name in [
        "mercurio-foundation",
        "mercurio-sysml",
        "mercurio-host-adapters",
        "mercurio-plugins",
        "mercurio-ai",
        "mercurio-product",
        "mercurio-examples",
    ] {
        let root = workspace_root.join(name);
        if root.join(".git").exists() {
            repositories.insert(name.to_string(), fingerprint_repo(&root)?);
        }
    }
    Ok(repositories)
}

fn git_stdout(path: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).current_dir(path).output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_string()
            .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn extract_report_metrics(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let value: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let aggregate = value.get("aggregate").cloned().unwrap_or(Value::Null);
    Ok(json!({
        "case_count": value.get("case_count").cloned().unwrap_or(Value::Null),
        "aggregate": aggregate,
    }))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

fn write_text(path: &Path, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, value)?;
    Ok(())
}

fn python_wrapper_conformance_summary(metrics: &Value) -> Value {
    json!({
        "module": value_at(metrics, &["module"]),
        "file_count": value_at(metrics, &["file_count"]),
        "python_file_count": value_at(metrics, &["python_file_count"]),
        "metamodel_class_count": value_at(metrics, &["metamodel_class_count"]),
        "py_compile_exit_code": value_at(metrics, &["py_compile_exit_code"]),
        "stdlib_catalog_entries": value_at(metrics, &["stdlib_catalog_entries"]),
    })
}

fn syntax_conformance_summary(metrics: &Value) -> Value {
    json!({
        "case_count": value_at(metrics, &["case_count"]),
        "exact_match_cases": value_at(metrics, &["aggregate", "exact_match_cases"]),
        "failed_cases": value_at(metrics, &["aggregate", "failed_cases"]),
        "total_mismatches": value_at(metrics, &["aggregate", "total_mismatches"]),
        "total_rust_only": value_at(metrics, &["aggregate", "total_rust_only"]),
        "total_pilot_only": value_at(metrics, &["aggregate", "total_pilot_only"]),
        "accepted_difference_gate": accepted_difference_gate_summary(metrics),
    })
}

fn semantic_conformance_summary(metrics: &Value) -> Value {
    json!({
        "case_count": value_at(metrics, &["case_count"]),
        "exact_match_cases": value_at(metrics, &["aggregate", "exact_match_cases"]),
        "failed_cases": value_at(metrics, &["aggregate", "failed_cases"]),
        "total_mismatches": value_at(metrics, &["aggregate", "total_mismatches"]),
        "total_mercurio_only": value_at(metrics, &["aggregate", "total_mercurio_only"]),
        "total_pilot_only": value_at(metrics, &["aggregate", "total_pilot_only"]),
        "total_metatype_mismatches": value_at(metrics, &["aggregate", "total_metatype_mismatches"]),
        "total_specialization_chain_mismatches": value_at(metrics, &["aggregate", "total_specialization_chain_mismatches"]),
        "total_declared_attribute_mismatches": value_at(metrics, &["aggregate", "total_declared_attribute_mismatches"]),
        "attribute_value_comparison": "declared_attributes.effective_value",
        "accepted_difference_gate": accepted_difference_gate_summary(metrics),
    })
}

fn compile_diagnostics_conformance_summary(metrics: &Value) -> Value {
    json!({
        "case_count": value_at(metrics, &["case_count"]),
        "both_pass_cases": value_at(metrics, &["aggregate", "both_pass_cases"]),
        "both_fail_cases": value_at(metrics, &["aggregate", "both_fail_cases"]),
        "status_match_cases": value_at(metrics, &["aggregate", "status_match_cases"]),
        "rust_only_fail_cases": value_at(metrics, &["aggregate", "rust_only_fail_cases"]),
        "pilot_only_fail_cases": value_at(metrics, &["aggregate", "pilot_only_fail_cases"]),
        "primary_problem_match_cases": value_at(metrics, &["aggregate", "primary_problem_match_cases"]),
        "failed_cases": value_at(metrics, &["aggregate", "failed_cases"]),
        "accepted_difference_gate": accepted_difference_gate_summary(metrics),
    })
}

fn accepted_difference_gate_summary(metrics: &Value) -> Value {
    json!({
        "total_differences": value_at(metrics, &["accepted_difference_gate", "total_differences"]),
        "accepted_differences": value_at(metrics, &["accepted_difference_gate", "accepted_differences"]),
        "unaccepted_differences": value_at(metrics, &["accepted_difference_gate", "unaccepted_differences"]),
    })
}

fn value_at(value: &Value, path: &[&str]) -> Value {
    let mut current = value;
    for segment in path {
        let Some(next) = current.get(*segment) else {
            return Value::Null;
        };
        current = next;
    }
    current.clone()
}

fn path_from_slashes(value: &str) -> PathBuf {
    value.split('/').collect()
}

fn path_to_slash_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn render_conformance_trace_markdown(trace: &ConformanceTrace) -> String {
    let mut output = String::new();
    output.push_str("# Pilot Conformance Trace\n\n");
    output.push_str(&format!("- Release: `{}`\n", trace.release));
    output.push_str(&format!("- Status: `{}`\n", trace.overall_status));
    output.push_str(&format!("- Generated: `{}`\n", trace.generated_at_utc));
    output.push_str(&format!("- Profile: `{}`\n", trace.profile_id));
    output.push_str(&format!("- Corpus: `{}`\n", trace.corpus));
    output.push_str(&format!("- Source lock: `{}`\n\n", trace.source_lock));
    output.push_str("## Source Fingerprints\n\n");
    output.push_str(&render_source_metrics_markdown(
        &trace.qualification_metrics.source,
    ));
    output.push('\n');
    output.push_str("## Standard Conformance Metrics\n\n");
    output.push_str(&render_standard_conformance_metrics_markdown(
        &trace.qualification_metrics.standard_conformance,
    ));
    output.push('\n');
    output.push_str("## Stage Reports\n\n");
    output.push_str("| Stage | Status | Duration ms | Report | Metrics |\n");
    output.push_str("|---|---:|---:|---|---|\n");
    for stage in &trace.stages {
        let report_path = stage
            .report
            .as_ref()
            .map(|report| report.path.as_str())
            .unwrap_or("");
        let metrics = stage
            .report
            .as_ref()
            .map(|report| compact_json(&report.metrics))
            .unwrap_or_default();
        output.push_str(&format!(
            "| `{}` | `{}` | {} | `{}` | `{}` |\n",
            stage.name,
            stage.status,
            stage.duration_ms,
            escape_markdown_table_cell(report_path),
            escape_markdown_table_cell(&metrics)
        ));
    }
    output
}

fn render_qualification_markdown(report: &QualificationReport) -> String {
    let mut output = String::new();
    output.push_str("# Pilot Release Qualification\n\n");
    output.push_str(&format!("- Release: `{}`\n", report.release));
    output.push_str(&format!("- Status: `{}`\n", report.overall_status));
    output.push_str(&format!("- Generated: `{}`\n", report.generated_at_utc));
    output.push_str(&format!("- Spec version: `{}`\n", report.spec_version));
    output.push_str(&format!("- Profile: `{}`\n", report.profile_id));
    output.push_str(&format!("- Corpus: `{}`\n", report.corpus));
    output.push_str(&format!("- Source lock: `{}`\n", report.source_lock));
    output.push_str(&format!(
        "- Conformance trace: `{}`\n\n",
        report.conformance_trace
    ));
    output.push_str("## Stage Summary\n\n");
    output.push_str(&format!("- Passed: {}\n", report.stage_summary.passed));
    output.push_str(&format!("- Failed: {}\n", report.stage_summary.failed));
    output.push_str(&format!("- Skipped: {}\n", report.stage_summary.skipped));
    output.push_str(&format!("- Total: {}\n", report.stage_summary.total));
    output.push_str(&format!(
        "- Total duration ms: {}\n",
        report.stage_summary.total_duration_ms
    ));
    output.push_str("\n## Source Fingerprints\n\n");
    output.push_str(&render_source_metrics_markdown(
        &report.qualification_metrics.source,
    ));
    output.push_str("\n## Standard Conformance Metrics\n\n");
    output.push_str(&render_standard_conformance_metrics_markdown(
        &report.qualification_metrics.standard_conformance,
    ));
    output
}

fn render_standard_conformance_metrics_markdown(metrics: &StandardConformanceMetrics) -> String {
    let rows = [
        ("Python wrappers", metrics.python_wrappers.as_ref()),
        ("Syntax parity", metrics.syntax.as_ref()),
        ("Semantic parity", metrics.semantic.as_ref()),
        ("Compile diagnostics", metrics.compile_diagnostics.as_ref()),
    ];
    let mut output = String::new();
    output.push_str("| Area | Metrics |\n");
    output.push_str("|---|---|\n");
    for (area, value) in rows {
        output.push_str(&format!(
            "| {} | `{}` |\n",
            area,
            value
                .map(compact_json)
                .map(|text| escape_markdown_table_cell(&text))
                .unwrap_or_else(|| "not reported".to_string())
        ));
    }
    output
}

fn render_source_metrics_markdown(source: &SourceMetricSummary) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "- Mercurio commit: `{}`\n",
        source.mercurio_commit.as_deref().unwrap_or("")
    ));
    output.push_str(&format!(
        "- Mercurio branch: `{}`\n",
        source.mercurio_branch.as_deref().unwrap_or("")
    ));
    output.push_str(&format!(
        "- Mercurio dirty: `{}`\n",
        source
            .mercurio_dirty
            .map(|dirty| dirty.to_string())
            .unwrap_or_default()
    ));
    output.push_str(&format!(
        "- Mercurio tracked tree SHA256: `{}`\n",
        source.mercurio_tracked_tree_sha256.as_deref().unwrap_or("")
    ));
    output.push_str(&format!(
        "- Pilot commit: `{}`\n",
        source.pilot_commit.as_deref().unwrap_or("")
    ));
    output.push_str(&format!(
        "- Pilot branch: `{}`\n",
        source.pilot_branch.as_deref().unwrap_or("")
    ));
    output.push_str(&format!(
        "- Pilot dirty: `{}`\n",
        source
            .pilot_dirty
            .map(|dirty| dirty.to_string())
            .unwrap_or_default()
    ));
    output.push_str(&format!(
        "- Pilot tracked tree SHA256: `{}`\n",
        source.pilot_tracked_tree_sha256.as_deref().unwrap_or("")
    ));
    if !source.workspace_repositories.is_empty() {
        output.push_str("\n### Workspace Repositories\n\n");
        output.push_str("| Repository | Branch | Dirty | Commit | Tree SHA256 |\n");
        output.push_str("|---|---|---:|---|---|\n");
        for (name, repo) in &source.workspace_repositories {
            output.push_str(&format!(
                "| `{}` | `{}` | `{}` | `{}` | `{}` |\n",
                escape_markdown_table_cell(name),
                escape_markdown_table_cell(repo.branch.as_deref().unwrap_or("")),
                repo.dirty
                    .map(|dirty| dirty.to_string())
                    .unwrap_or_default(),
                escape_markdown_table_cell(repo.commit.as_deref().unwrap_or("")),
                escape_markdown_table_cell(repo.tracked_tree_sha256.as_deref().unwrap_or(""))
            ));
        }
        output.push('\n');
    }
    if let Some(sha256) = &source.source_archive_sha256 {
        output.push_str(&format!("- Source archive SHA256: `{sha256}`\n"));
    }
    if let Some(sha256) = &source.asset_tree_sha256 {
        output.push_str(&format!("- Asset tree SHA256: `{sha256}`\n"));
    }
    output
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn escape_markdown_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn now_utc_rfc3339() -> Result<String, Box<dyn std::error::Error>> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

fn cargo_program() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

fn sysml_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("mercurio-tools lives under crates")
        .to_path_buf()
}

fn next_string(args: &[String], index: &mut usize) -> Result<String, Box<dyn std::error::Error>> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| "missing argument value".into())
}

fn next_path(args: &[String], index: &mut usize) -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(next_string(args, index)?))
}

fn print_usage() {
    println!(
        "Usage: qualify_pilot_release [--release 2026-01] [--pilot-root PATH] [--source-archive PATH] [--asset-dir PATH] [--out PATH] [--profile-id ID] [--spec-version VERSION] [--corpus NAME] [--wrapper-module MODULE] [--skip-stdlib-build] [--skip-parity] [--promote-candidate] [--mark-latest]"
    );
}

fn default_profile_id_for_release(release: &str) -> String {
    if release == DEFAULT_RELEASE {
        DEFAULT_PROFILE_ID.to_string()
    } else {
        format!("sysml-2.0-pilot-{release}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualification_reports_render_stage_summary_and_trace_markdown() {
        let stages = vec![
            StageTrace {
                name: "pilot_java_artifacts".to_string(),
                description: "verify jar",
                status: "passed".to_string(),
                duration_ms: 12,
                command: Vec::new(),
                exit_code: None,
                report: Some(ReportTrace {
                    path: "pilot.jar".to_string(),
                    sha256: "abc".to_string(),
                    metrics: json!({ "artifact": "jar" }),
                }),
                error: None,
            },
            StageTrace {
                name: "syntax_parity".to_string(),
                description: "compare syntax",
                status: "skipped".to_string(),
                duration_ms: 0,
                command: Vec::new(),
                exit_code: None,
                report: None,
                error: Some("not requested".to_string()),
            },
            StageTrace {
                name: "semantic_parity".to_string(),
                description: "compare semantics",
                status: "passed".to_string(),
                duration_ms: 34,
                command: Vec::new(),
                exit_code: Some(0),
                report: Some(ReportTrace {
                    path: "reports/semantic-parity.json".to_string(),
                    sha256: "def".to_string(),
                    metrics: json!({
                        "case_count": 2,
                        "aggregate": {
                            "exact_match_cases": 1,
                            "failed_cases": 0,
                            "total_mismatches": 1,
                            "total_mercurio_only": 0,
                            "total_pilot_only": 0,
                            "total_metatype_mismatches": 0,
                            "total_specialization_chain_mismatches": 0,
                            "total_declared_attribute_mismatches": 1
                        },
                        "accepted_difference_gate": {
                            "total_differences": 0,
                            "accepted_differences": 0,
                            "unaccepted_differences": 0
                        }
                    }),
                }),
                error: None,
            },
        ];
        let summary = StageSummary::from_stages(&stages);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.total_duration_ms, 46);
        let metrics = QualificationMetrics::from_source_and_stages(&test_source_lock(), &stages);

        let trace = ConformanceTrace {
            schema: "dev.mercurio.pilot-release-conformance-trace.v1",
            generated_at_utc: "2026-06-16T00:00:00Z".to_string(),
            release: "2026-01".to_string(),
            spec_version: "2.0.0".to_string(),
            profile_id: "sysml-2.0-metamodel-0.57.0".to_string(),
            corpus: "small".to_string(),
            source_lock: "locks/source.lock.json".to_string(),
            overall_status: "failed".to_string(),
            qualification_metrics: metrics.clone(),
            stages,
        };
        let trace_markdown = render_conformance_trace_markdown(&trace);
        assert!(trace_markdown.contains("# Pilot Conformance Trace"));
        assert!(trace_markdown.contains("`pilot_java_artifacts`"));
        assert!(trace_markdown.contains("pilot.jar"));
        assert!(trace_markdown.contains("Mercurio tracked tree SHA256"));
        assert!(trace_markdown.contains("## Standard Conformance Metrics"));
        assert!(trace_markdown.contains("Semantic parity"));
        assert!(trace_markdown.contains("total_declared_attribute_mismatches"));
        assert!(trace_markdown.contains("declared_attributes.effective_value"));

        let report = QualificationReport {
            schema: "dev.mercurio.pilot-release-qualification.v1",
            generated_at_utc: trace.generated_at_utc.clone(),
            release: trace.release.clone(),
            spec_version: trace.spec_version.clone(),
            profile_id: trace.profile_id.clone(),
            corpus: trace.corpus.clone(),
            overall_status: trace.overall_status.clone(),
            source_lock: trace.source_lock.clone(),
            conformance_trace: "reports/conformance-trace.json".to_string(),
            stage_summary: summary,
            qualification_metrics: metrics,
        };
        let qualification_markdown = render_qualification_markdown(&report);
        assert!(qualification_markdown.contains("# Pilot Release Qualification"));
        assert!(qualification_markdown.contains("- Passed: 2"));
        assert!(qualification_markdown.contains("- Skipped: 1"));
        assert!(qualification_markdown.contains("## Standard Conformance Metrics"));
    }

    #[test]
    fn fingerprints_source_archive_and_asset_tree() {
        let root = std::env::temp_dir().join(format!(
            "mercurio-qualification-fingerprint-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("assets/nested")).unwrap();
        let archive = root.join("release.zip");
        std::fs::write(&archive, b"archive").unwrap();
        std::fs::write(root.join("assets/profile.json"), b"profile").unwrap();
        std::fs::write(root.join("assets/nested/stdlib.kpar"), b"stdlib").unwrap();

        let archive_fingerprint = fingerprint_file(&archive).unwrap();
        assert_eq!(archive_fingerprint.byte_len, 7);
        assert!(!archive_fingerprint.sha256.is_empty());

        let tree = fingerprint_tree(&root.join("assets")).unwrap();
        assert_eq!(tree.file_count, 2);
        assert!(tree.files.contains_key("profile.json"));
        assert!(tree.files.contains_key("nested/stdlib.kpar"));
        assert!(!tree.tree_sha256.is_empty());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn skipped_stages_do_not_fail_overall_status() {
        let stages = vec![
            StageTrace {
                name: "candidate_staging".to_string(),
                description: "stage candidate",
                status: "passed".to_string(),
                duration_ms: 1,
                command: Vec::new(),
                exit_code: None,
                report: None,
                error: None,
            },
            StageTrace {
                name: "syntax_parity".to_string(),
                description: "compare syntax",
                status: "skipped".to_string(),
                duration_ms: 0,
                command: Vec::new(),
                exit_code: None,
                report: None,
                error: Some("not requested".to_string()),
            },
        ];

        assert_eq!(overall_status(&stages), "passed");
    }

    #[test]
    fn default_profile_id_uses_release_selector_for_new_releases() {
        assert_eq!(
            default_profile_id_for_release("2026-01"),
            "sysml-2.0-metamodel-0.57.0"
        );
        assert_eq!(
            default_profile_id_for_release("2026-04"),
            "sysml-2.0-pilot-2026-04"
        );
    }

    #[test]
    fn python_wrappers_stage_generates_trace_wrappers_when_stdlib_build_is_skipped() {
        let root = std::env::temp_dir().join(format!(
            "mercurio-python-wrapper-stage-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let candidate = root.join("candidate");
        let python_root = root.join("stdlib/python");
        let profile_id = "sysml-2.0-pilot-2099-01";
        std::fs::create_dir_all(candidate.join("stdlib")).unwrap();
        write_json(
            &candidate.join("profile.json"),
            &json!({
                "id": profile_id,
                "language": "sysml",
                "language_version": "2.0",
                "metamodel_version": "2099-01",
                "stdlib_version": "2099-01",
                "stdlib_path": "stdlib/stdlib.full.kir.json",
                "kir_schema_version": "0.4"
            }),
        )
        .unwrap();
        write_json(
            &candidate.join("stdlib/stdlib.full.kir.json"),
            &json!({
                "metadata": {
                    "kir_schema_version": "0.4"
                },
                "elements": [
                    {
                        "id": "SI::metre",
                        "kind": "AttributeUsage",
                        "layer": 1,
                        "properties": {}
                    }
                ]
            }),
        )
        .unwrap();

        let stage = python_wrappers_stage(
            &python_root,
            &candidate,
            "mercurio_sysml_test",
            profile_id,
            true,
        )
        .unwrap();

        assert_eq!(stage.status, "passed");
        assert!(
            python_root
                .join("mercurio_sysml_test/generation_info.py")
                .is_file()
        );
        assert!(
            python_root
                .join("mercurio_sysml_test/stdlib/si.py")
                .is_file()
        );
        assert_eq!(
            stage
                .report
                .as_ref()
                .and_then(|report| report.metrics.get("module"))
                .and_then(Value::as_str),
            Some("mercurio_sysml_test")
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn candidate_profile_rewrites_profile_id_and_stdlib_path() {
        let root = std::env::temp_dir().join(format!(
            "mercurio-candidate-profile-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let source = root.join("source");
        let candidate = root.join("candidate");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            source.join("profile.json"),
            serde_json::to_vec(&json!({
                "id": "sysml-2.0-metamodel-0.57.0",
                "stdlib_path": "resources/metamodels/sysml-2.0-metamodel-0.57.0/stdlib/stdlib.full.kir.json"
            }))
            .unwrap(),
        )
        .unwrap();
        let args = test_args("2026-04", "sysml-2.0-pilot-2026-04");
        let mut errors = Vec::new();

        copy_candidate_profile(&source, &candidate, &args, &mut errors);

        assert!(errors.is_empty(), "{errors:?}");
        let profile: Value =
            serde_json::from_str(&std::fs::read_to_string(candidate.join("profile.json")).unwrap())
                .unwrap();
        assert_eq!(
            profile.get("id").and_then(Value::as_str),
            Some("sysml-2.0-pilot-2026-04")
        );
        assert_eq!(
            profile.get("stdlib_path").and_then(Value::as_str),
            Some("resources/metamodels/sysml-2.0-pilot-2026-04/stdlib/stdlib.full.kir.json")
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn candidate_metamodel_rewrites_release_identity() {
        let root = std::env::temp_dir().join(format!(
            "mercurio-candidate-metamodel-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let source = root.join("source");
        let candidate = root.join("candidate");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            source.join("metamodel.json"),
            serde_json::to_vec(&json!({
                "id": "sysml-2.0-metamodel-0.57.0",
                "display_name": "SysML 2.0 Metamodel 0.57.0",
                "metamodel_version": "0.57.0",
                "status": "latest",
                "stdlib_path": "stdlib/stdlib.full.kir.json",
                "sysml_delta_path": "stdlib/sysml-library.kir.json",
                "legacy_ids": ["sysml-2.0-pilot-0.57.0"]
            }))
            .unwrap(),
        )
        .unwrap();
        let args = test_args("2026-04", "sysml-2.0-pilot-2026-04");
        let mut errors = Vec::new();

        copy_candidate_metamodel(&source, &candidate, &args, &mut errors);

        assert!(errors.is_empty(), "{errors:?}");
        let metamodel: Value = serde_json::from_str(
            &std::fs::read_to_string(candidate.join("metamodel.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            metamodel.get("id").and_then(Value::as_str),
            Some("sysml-2.0-pilot-2026-04")
        );
        assert_eq!(
            metamodel.get("metamodel_version").and_then(Value::as_str),
            Some("2026-04")
        );
        assert_eq!(
            metamodel.get("selector").and_then(Value::as_str),
            Some("2026-04")
        );
        assert_eq!(
            metamodel.get("status").and_then(Value::as_str),
            Some("supported")
        );
        assert!(
            metamodel
                .get("legacy_ids")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn candidate_stdlib_falls_back_when_generated_full_kir_lacks_required_anchors() {
        let root = std::env::temp_dir().join(format!(
            "mercurio-candidate-stdlib-select-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let generated = root.join("generated");
        let bundled = root.join("bundled");
        std::fs::create_dir_all(&generated).unwrap();
        std::fs::create_dir_all(&bundled).unwrap();
        std::fs::write(
            generated.join("stdlib.full.kir.json"),
            serde_json::to_vec(&json!({ "elements": [] })).unwrap(),
        )
        .unwrap();
        std::fs::write(
            bundled.join("stdlib.full.kir.json"),
            serde_json::to_vec(&json!({
                "elements": [
                    { "id": "Items::Item" },
                    { "id": "Parts::Part" }
                ]
            }))
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            select_candidate_stdlib_source(&generated, &bundled),
            bundled
        );
        assert!(candidate_stdlib_has_required_anchors(&bundled));
        assert!(!candidate_stdlib_has_required_anchors(&generated));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parity_differences_extract_compile_status_mismatch() {
        let report = json!({
            "cases": [
                {
                    "relative_path": "model.sysml",
                    "status": "ok",
                    "comparison": {
                        "status_match": false,
                        "mercurio_primary_problem": "unresolved_reference"
                    },
                    "mercurio_status": "error",
                    "pilot_status": "ok"
                }
            ]
        });

        let differences = parity_differences("compile_diagnostics_parity", &report);

        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].kind, "compile_status_mismatch");
        assert_eq!(differences[0].case, "model.sysml");
        assert_eq!(
            differences[0].primary_problem.as_deref(),
            Some("unresolved_reference")
        );
    }

    #[test]
    fn accepted_differences_match_specific_stage_case_and_problem() {
        let accepted = AcceptedDifferences {
            differences: vec![AcceptedDifference {
                stage: Some("compile_diagnostics_parity".to_string()),
                case_path: Some("model.sysml".to_string()),
                kind: Some("compile_status_mismatch".to_string()),
                mercurio_status: Some("error".to_string()),
                pilot_status: Some("ok".to_string()),
                primary_problem: Some("unresolved_reference".to_string()),
                message_contains: None,
            }],
        };
        let difference = ParityDifference {
            stage: "compile_diagnostics_parity".to_string(),
            case: "model.sysml".to_string(),
            kind: "compile_status_mismatch".to_string(),
            message: None,
            mercurio_status: Some("error".to_string()),
            pilot_status: Some("ok".to_string()),
            primary_problem: Some("unresolved_reference".to_string()),
        };

        assert!(accepted.matches(&difference));
    }

    fn test_source_lock() -> SourceLock {
        SourceLock {
            schema: "dev.mercurio.pilot-release-source-lock.v1",
            generated_at_utc: "2026-06-16T00:00:00Z".to_string(),
            release: "2026-01".to_string(),
            spec_version: "2.0.0".to_string(),
            profile_id: "sysml-2.0-metamodel-0.57.0".to_string(),
            corpus: "small".to_string(),
            mercurio: RepoFingerprint {
                root: "mercurio".to_string(),
                commit: Some("mercurio-commit".to_string()),
                branch: Some("main".to_string()),
                dirty: Some(false),
                tracked_file_count: 2,
                tracked_tree_sha256: Some("mercurio-tree".to_string()),
                tracked_files: BTreeMap::new(),
            },
            pilot: RepoFingerprint {
                root: "pilot".to_string(),
                commit: Some("pilot-commit".to_string()),
                branch: Some("master".to_string()),
                dirty: Some(false),
                tracked_file_count: 3,
                tracked_tree_sha256: Some("pilot-tree".to_string()),
                tracked_files: BTreeMap::new(),
            },
            workspace_repositories: BTreeMap::from([(
                "mercurio-foundation".to_string(),
                RepoFingerprint {
                    root: "mercurio-foundation".to_string(),
                    commit: Some("foundation-commit".to_string()),
                    branch: Some("main".to_string()),
                    dirty: Some(false),
                    tracked_file_count: 4,
                    tracked_tree_sha256: Some("foundation-tree".to_string()),
                    tracked_files: BTreeMap::new(),
                },
            )]),
            inputs: SourceInputs {
                source_archive: Some(PathFingerprint {
                    path: "release.zip".to_string(),
                    sha256: "archive-sha".to_string(),
                    byte_len: 7,
                }),
                asset_dir: Some(TreeFingerprint {
                    root: "assets".to_string(),
                    file_count: 1,
                    tree_sha256: "asset-tree".to_string(),
                    files: BTreeMap::new(),
                }),
            },
        }
    }

    fn test_args(release: &str, profile_id: &str) -> Args {
        Args {
            release: release.to_string(),
            pilot_root: PathBuf::new(),
            out: PathBuf::new(),
            profile_id: profile_id.to_string(),
            spec_version: DEFAULT_SPEC_VERSION.to_string(),
            corpus: DEFAULT_CORPUS.to_string(),
            wrapper_module: DEFAULT_WRAPPER_MODULE.to_string(),
            source_archive: None,
            asset_dir: None,
            skip_stdlib_build: false,
            skip_parity: false,
            promote_candidate: false,
            mark_latest: false,
        }
    }

    #[test]
    fn copies_required_candidate_bundle_files() {
        let root = std::env::temp_dir().join(format!(
            "mercurio-candidate-staging-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let source = root.join("source");
        let candidate = root.join("candidate");
        std::fs::create_dir_all(source.join("mappings")).unwrap();
        std::fs::create_dir_all(source.join("stdlib")).unwrap();
        std::fs::write(source.join("profile.json"), b"{}").unwrap();
        std::fs::write(
            source.join("mappings/metamodel_constructs.seed.json"),
            b"{}",
        )
        .unwrap();
        std::fs::write(source.join("stdlib/stdlib.full.kir.json"), b"{}").unwrap();

        let mut errors = Vec::new();
        copy_required_file(&source, &candidate, "profile.json", &mut errors);
        copy_required_dir(&source, &candidate, "mappings", &mut errors);
        copy_required_dir(&source, &candidate, "stdlib", &mut errors);

        assert!(errors.is_empty(), "{errors:?}");
        assert!(candidate.join("profile.json").is_file());
        assert!(
            candidate
                .join("mappings/metamodel_constructs.seed.json")
                .is_file()
        );
        assert!(candidate.join("stdlib/stdlib.full.kir.json").is_file());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn promotion_merges_registry_and_marks_latest_explicitly() {
        let root = std::env::temp_dir().join(format!(
            "mercurio-candidate-promotion-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let candidate_resources = root.join("candidate/resources/metamodels");
        let repo_resources = root.join("repo/resources/metamodels");
        let profile_id = "sysml-2.0-metamodel-2026-04";
        std::fs::create_dir_all(candidate_resources.join(profile_id)).unwrap();
        std::fs::create_dir_all(&repo_resources).unwrap();
        std::fs::write(
            candidate_resources.join(profile_id).join("profile.json"),
            b"{}",
        )
        .unwrap();
        write_json(
            &candidate_resources.join("registry.json"),
            &json!([
                {
                    "id": profile_id,
                    "release": "2026-04",
                    "selector": "2026-04",
                    "display_name": "SysML v2 (2026-04)",
                    "sysml_version": "2.0",
                    "kerml_version": "1.0",
                    "metamodel_version": "2026-04",
                    "status": "supported"
                }
            ]),
        )
        .unwrap();
        write_json(
            &repo_resources.join("registry.json"),
            &json!([
                {
                    "id": "sysml-2.0-metamodel-0.57.0",
                    "status": "latest"
                }
            ]),
        )
        .unwrap();

        let report =
            promote_candidate_resources(&candidate_resources, &repo_resources, profile_id, true)
                .unwrap();
        assert!(report.marked_latest);
        assert!(
            repo_resources
                .join(profile_id)
                .join("profile.json")
                .is_file()
        );

        let registry: Value = serde_json::from_str(
            &std::fs::read_to_string(repo_resources.join("registry.json")).unwrap(),
        )
        .unwrap();
        let entries = registry.as_array().unwrap();
        assert_eq!(
            entries[0].get("status").and_then(Value::as_str),
            Some("supported")
        );
        assert_eq!(
            entries[1].get("status").and_then(Value::as_str),
            Some("latest")
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn promotion_refuses_existing_profile_directory() {
        let root = std::env::temp_dir().join(format!(
            "mercurio-candidate-promotion-existing-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let candidate_resources = root.join("candidate/resources/metamodels");
        let repo_resources = root.join("repo/resources/metamodels");
        let profile_id = "sysml-2.0-metamodel-2026-04";
        std::fs::create_dir_all(candidate_resources.join(profile_id)).unwrap();
        std::fs::create_dir_all(repo_resources.join(profile_id)).unwrap();
        std::fs::write(
            candidate_resources.join(profile_id).join("profile.json"),
            b"{}",
        )
        .unwrap();
        write_json(
            &candidate_resources.join("registry.json"),
            &json!([{ "id": profile_id, "status": "supported" }]),
        )
        .unwrap();

        let err =
            promote_candidate_resources(&candidate_resources, &repo_resources, profile_id, false)
                .unwrap_err()
                .to_string();
        assert!(err.contains("refusing to overwrite"));

        std::fs::remove_dir_all(root).unwrap();
    }
}
