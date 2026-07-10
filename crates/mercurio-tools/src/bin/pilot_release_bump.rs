use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use mercurio_tools::default_pilot_root;
use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const DEFAULT_PROFILE_ID: &str = "sysml-2.0-pilot-2026-04";
const DEFAULT_CORPUS: &str = "small";
const DEFAULT_JAVA_TIMEOUT_SECONDS: u64 = 300;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    let root = sysml_workspace_root();
    if !args.allow_dirty {
        require_clean_pilot(&args.pilot_root)?;
    }

    let mut stages = Vec::new();
    stages.push(run_tool(
        &root,
        "grammar_extract",
        "extract_pilot_grammar",
        extraction_args(&args, true, !args.write),
    ));
    stages.push(run_tool(
        &root,
        "construct_seed",
        "generate_pilot_constructs_seed",
        profile_check_args(&args.profile_id, !args.write),
    ));
    stages.push(run_tool(
        &root,
        "metamodel_extract",
        "extract_pilot_metamodel",
        extraction_args(&args, true, !args.write),
    ));
    stages.push(run_tool(
        &root,
        "metamodel_audit",
        "audit_metamodel_extract",
        vec![
            "--profile-id".to_string(),
            args.profile_id.clone(),
            "--deny-warnings".to_string(),
        ],
    ));
    stages.push(run_tool(
        &root,
        "field_specs",
        "generate_sysml_field_specs",
        profile_check_args(&args.profile_id, !args.write),
    ));
    stages.push(run_tool(
        &root,
        "legality_rulepack",
        "generate_sysml_legality_rulepack",
        selector_check_args(&args.profile_id, !args.write),
    ));
    stages.push(run_tool(
        &root,
        "validator_extract",
        "extract_pilot_validators",
        extraction_args(&args, true, !args.write),
    ));

    if args.skip_jar_stages {
        stages.push(StageReport::skipped(
            "implicit_semantics",
            "skipped by --skip-jar-stages",
        ));
        stages.push(StageReport::skipped(
            "pilot_conformance",
            "skipped by --skip-jar-stages",
        ));
    } else {
        let mut implicit_args = extraction_args(&args, true, !args.write);
        implicit_args.extend(["--corpus".to_string(), args.corpus.clone()]);
        stages.push(run_tool(
            &root,
            "implicit_semantics",
            "extract_pilot_implicit_semantics",
            implicit_args,
        ));

        let conformance_path = args.out.with_file_name(format!(
            "{}.conformance.{}.json",
            args.out
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("pilot-release-bump"),
            args.corpus
        ));
        stages.push(run_tool(
            &root,
            "pilot_conformance",
            "pilot_conformance_harness",
            vec![
                "--pilot-root".to_string(),
                args.pilot_root.display().to_string(),
                "--corpus".to_string(),
                args.corpus.clone(),
                "--out".to_string(),
                conformance_path.display().to_string(),
                "--java-timeout-seconds".to_string(),
                args.java_timeout_seconds.to_string(),
            ],
        ));
    }

    let report = DriftReport {
        schema: "dev.mercurio.pilot-release-bump.drift-report.v1",
        generated_at_utc: now_utc_rfc3339()?,
        mode: if args.write { "write" } else { "check" },
        profile_id: args.profile_id,
        corpus: args.corpus,
        pilot_root: args.pilot_root.display().to_string(),
        skip_jar_stages: args.skip_jar_stages,
        status: if stages.iter().all(|stage| stage.status != "failed") {
            "passed"
        } else {
            "failed"
        },
        git_status_short: git_status_short(&root).unwrap_or_else(|err| err.to_string()),
        stages,
    };

    write_report(&args.out, &report)?;
    if report.status == "failed" {
        return Err(format!(
            "pilot release bump drift report failed: {}",
            args.out.display()
        )
        .into());
    }
    println!(
        "wrote pilot release bump drift report: {}",
        args.out.display()
    );
    Ok(())
}

#[derive(Debug)]
struct Args {
    pilot_root: PathBuf,
    profile_id: String,
    corpus: String,
    out: PathBuf,
    allow_dirty: bool,
    skip_jar_stages: bool,
    write: bool,
    java_timeout_seconds: u64,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut pilot_root = default_pilot_root();
        let mut profile_id = DEFAULT_PROFILE_ID.to_string();
        let mut corpus = DEFAULT_CORPUS.to_string();
        let mut out = PathBuf::from("target/pilot-release-bump-drift.json");
        let mut allow_dirty = false;
        let mut skip_jar_stages = false;
        let mut write = false;
        let mut java_timeout_seconds = DEFAULT_JAVA_TIMEOUT_SECONDS;

        let raw = env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;
        while index < raw.len() {
            match raw[index].as_str() {
                "--pilot-root" => pilot_root = next_path(&raw, &mut index)?,
                "--profile-id" | "--profile" => profile_id = next_string(&raw, &mut index)?,
                "--corpus" => corpus = next_string(&raw, &mut index)?,
                "--out" => out = next_path(&raw, &mut index)?,
                "--allow-dirty" => allow_dirty = true,
                "--skip-jar-stages" => skip_jar_stages = true,
                "--write" => write = true,
                "--java-timeout-seconds" => {
                    java_timeout_seconds = next_string(&raw, &mut index)?.parse()?
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}").into()),
            }
            index += 1;
        }

        Ok(Self {
            pilot_root,
            profile_id,
            corpus,
            out,
            allow_dirty,
            skip_jar_stages,
            write,
            java_timeout_seconds,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --features legacy-pilot-tools --bin pilot_release_bump -- [--pilot-root PATH] [--profile-id ID] [--corpus NAME] [--out PATH] [--write] [--skip-jar-stages] [--allow-dirty] [--java-timeout-seconds N]"
    );
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

#[derive(Serialize)]
struct DriftReport {
    schema: &'static str,
    generated_at_utc: String,
    mode: &'static str,
    profile_id: String,
    corpus: String,
    pilot_root: String,
    skip_jar_stages: bool,
    status: &'static str,
    git_status_short: String,
    stages: Vec<StageReport>,
}

#[derive(Serialize)]
struct StageReport {
    name: String,
    tool: Option<String>,
    status: String,
    duration_ms: u128,
    command: Vec<String>,
    exit_code: Option<i32>,
    stdout_tail: String,
    stderr_tail: String,
}

impl StageReport {
    fn skipped(name: &str, reason: &str) -> Self {
        Self {
            name: name.to_string(),
            tool: None,
            status: "skipped".to_string(),
            duration_ms: 0,
            command: Vec::new(),
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: reason.to_string(),
        }
    }
}

fn extraction_args(args: &Args, pilot_root: bool, check: bool) -> Vec<String> {
    let mut out = vec!["--profile-id".to_string(), args.profile_id.clone()];
    if pilot_root {
        out.extend([
            "--pilot-root".to_string(),
            args.pilot_root.display().to_string(),
        ]);
    }
    if check {
        out.push("--check".to_string());
    }
    if args.allow_dirty {
        out.push("--allow-dirty".to_string());
    }
    out
}

fn profile_check_args(profile_id: &str, check: bool) -> Vec<String> {
    let mut out = vec!["--profile-id".to_string(), profile_id.to_string()];
    if check {
        out.push("--check".to_string());
    }
    out
}

fn selector_check_args(selector: &str, check: bool) -> Vec<String> {
    let mut out = vec!["--selector".to_string(), selector.to_string()];
    if check {
        out.push("--check".to_string());
    }
    out
}

fn run_tool(root: &Path, name: &str, tool: &str, tool_args: Vec<String>) -> StageReport {
    let mut command = vec![
        cargo_program(),
        "run".to_string(),
        "-p".to_string(),
        "mercurio-tools".to_string(),
        "--features".to_string(),
        "legacy-pilot-tools".to_string(),
        "--bin".to_string(),
        tool.to_string(),
        "--".to_string(),
    ];
    command.extend(tool_args);

    let started = Instant::now();
    let output = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(root)
        .output();
    let duration_ms = started.elapsed().as_millis();

    match output {
        Ok(output) => StageReport {
            name: name.to_string(),
            tool: Some(tool.to_string()),
            status: if output.status.success() {
                "passed".to_string()
            } else {
                "failed".to_string()
            },
            duration_ms,
            command,
            exit_code: output.status.code(),
            stdout_tail: tail(&String::from_utf8_lossy(&output.stdout), 4000),
            stderr_tail: tail(&String::from_utf8_lossy(&output.stderr), 4000),
        },
        Err(err) => StageReport {
            name: name.to_string(),
            tool: Some(tool.to_string()),
            status: "failed".to_string(),
            duration_ms,
            command,
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: err.to_string(),
        },
    }
}

fn write_report(path: &Path, report: &DriftReport) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(report)?))?;
    std::fs::write(path.with_extension("md"), render_markdown(report))?;
    Ok(())
}

fn render_markdown(report: &DriftReport) -> String {
    let mut out = String::new();
    out.push_str("# Pilot Release Bump Drift Report\n\n");
    out.push_str(&format!("- Status: {}\n", report.status));
    out.push_str(&format!("- Mode: {}\n", report.mode));
    out.push_str(&format!("- Profile: {}\n", report.profile_id));
    out.push_str(&format!("- Corpus: {}\n", report.corpus));
    out.push_str(&format!("- Pilot root: {}\n\n", report.pilot_root));
    out.push_str("| Stage | Status | Duration ms | Tool |\n");
    out.push_str("| --- | --- | ---: | --- |\n");
    for stage in &report.stages {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            stage.name,
            stage.status,
            stage.duration_ms,
            stage.tool.as_deref().unwrap_or("")
        ));
    }
    out.push_str("\n## Git Status\n\n```text\n");
    out.push_str(&report.git_status_short);
    if !report.git_status_short.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("```\n");
    out
}

fn require_clean_pilot(pilot_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(pilot_root)
        .arg("status")
        .arg("--porcelain")
        .output()?;
    if !output.status.success() {
        return Err("failed to inspect Pilot checkout cleanliness".into());
    }
    if !output.stdout.is_empty() {
        return Err(format!(
            "Pilot checkout `{}` is dirty; pass --allow-dirty only for non-release drift reports",
            pilot_root.display()
        )
        .into());
    }
    Ok(())
}

fn git_status_short(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("status")
        .arg("--short")
        .output()?;
    if !output.status.success() {
        return Err("failed to inspect Mercurio SysML git status".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn tail(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(max_chars);
    chars[start..].iter().collect()
}

fn cargo_program() -> String {
    env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

fn sysml_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn now_utc_rfc3339() -> Result<String, Box<dyn std::error::Error>> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}
