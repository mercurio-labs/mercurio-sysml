use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use mercurio_tools::{default_pilot_root, sha256_file, sha256_hex};
use serde::Serialize;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const DEFAULT_RELEASE: &str = "2026-01";
const DEFAULT_PROFILE_ID: &str = "sysml-2.0-metamodel-0.57.0";
const DEFAULT_SPEC_VERSION: &str = "2.0.0";
const DEFAULT_CORPUS: &str = "all";

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
    };
    write_json(&args.out.join("locks/source.lock.json"), &source_lock)?;

    let mut stages = Vec::new();
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
        return write_trace_and_exit(args, stages);
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
                    "--audit-profile".to_string(),
                ],
            },
            &mercurio_root,
            None,
        )?);
    }

    let syntax_report = args.out.join("reports/syntax-parity.json");
    stages.push(run_stage(
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
        },
        &mercurio_root,
        Some(&syntax_report),
    )?);

    let semantic_report = args.out.join("reports/semantic-parity.json");
    stages.push(run_stage(
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
        },
        &mercurio_root,
        Some(&semantic_report),
    )?);

    let compile_report = args.out.join("reports/compile-errors-parity.json");
    stages.push(run_stage(
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
        },
        &mercurio_root,
        Some(&compile_report),
    )?);

    write_trace_and_exit(args, stages)
}

fn write_trace_and_exit(
    args: Args,
    stages: Vec<StageTrace>,
) -> Result<(), Box<dyn std::error::Error>> {
    let trace = ConformanceTrace {
        schema: "dev.mercurio.pilot-release-conformance-trace.v1",
        generated_at_utc: now_utc_rfc3339()?,
        release: args.release,
        spec_version: args.spec_version,
        profile_id: args.profile_id,
        corpus: args.corpus,
        source_lock: "locks/source.lock.json".to_string(),
        overall_status: if stages.iter().all(|stage| stage.status == "passed") {
            "passed".to_string()
        } else {
            "failed".to_string()
        },
        stages,
    };
    write_json(&args.out.join("reports/conformance-trace.json"), &trace)?;

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
    skip_stdlib_build: bool,
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
            skip_stdlib_build: false,
        };
        let raw = std::env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;
        while index < raw.len() {
            match raw[index].as_str() {
                "--release" => args.release = next_string(&raw, &mut index)?,
                "--pilot-root" => args.pilot_root = next_path(&raw, &mut index)?,
                "--out" => args.out = next_path(&raw, &mut index)?,
                "--profile-id" => args.profile_id = next_string(&raw, &mut index)?,
                "--spec-version" => args.spec_version = next_string(&raw, &mut index)?,
                "--corpus" => args.corpus = next_string(&raw, &mut index)?,
                "--skip-stdlib-build" => args.skip_stdlib_build = true,
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}").into()),
            }
            index += 1;
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
struct ConformanceTrace {
    schema: &'static str,
    generated_at_utc: String,
    release: String,
    spec_version: String,
    profile_id: String,
    corpus: String,
    source_lock: String,
    overall_status: String,
    stages: Vec<StageTrace>,
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
}

fn run_stage(
    name: &str,
    description: &'static str,
    spec: CommandSpec,
    current_dir: &Path,
    report_path: Option<&Path>,
) -> Result<StageTrace, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let output = Command::new(&spec.program)
        .args(&spec.args)
        .current_dir(current_dir)
        .output()?;
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
        "Usage: qualify_pilot_release [--release 2026-01] [--pilot-root PATH] [--out PATH] [--profile-id ID] [--spec-version VERSION] [--corpus NAME] [--skip-stdlib-build]"
    );
}
