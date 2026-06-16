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
const DEFAULT_WRAPPER_MODULE: &str = "mercurio_sysml_2_0";

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
                    "--wrapper-module".to_string(),
                    args.wrapper_module.clone(),
                    "--audit-profile".to_string(),
                ],
            },
            &mercurio_root,
            None,
        )?);
    }

    stages.push(python_wrappers_stage(
        &args.out.join("stdlib/python"),
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
        return write_trace_and_exit(args, stages);
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
    let generated_at_utc = now_utc_rfc3339()?;
    let stage_summary = StageSummary::from_stages(&stages);
    let trace = ConformanceTrace {
        schema: "dev.mercurio.pilot-release-conformance-trace.v1",
        generated_at_utc: generated_at_utc.clone(),
        release: args.release.clone(),
        spec_version: args.spec_version.clone(),
        profile_id: args.profile_id.clone(),
        corpus: args.corpus.clone(),
        source_lock: "locks/source.lock.json".to_string(),
        overall_status: if stages.iter().all(|stage| stage.status == "passed") {
            "passed".to_string()
        } else {
            "failed".to_string()
        },
        stages,
    };
    write_json(&args.out.join("reports/conformance-trace.json"), &trace)?;
    write_text(
        &args.out.join("reports/conformance-trace.md"),
        &render_conformance_trace_markdown(&trace),
    )?;

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
                "--wrapper-module" => args.wrapper_module = next_string(&raw, &mut index)?,
                "--source-archive" => args.source_archive = Some(next_path(&raw, &mut index)?),
                "--asset-dir" => args.asset_dir = Some(next_path(&raw, &mut index)?),
                "--skip-stdlib-build" => args.skip_stdlib_build = true,
                "--skip-parity" => args.skip_parity = true,
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

fn python_wrappers_stage(
    python_root: &Path,
    wrapper_module: &str,
    expected_profile_id: &str,
    skipped_stdlib_build: bool,
) -> Result<StageTrace, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let module_root = python_root.join(wrapper_module);
    if !module_root.exists() {
        return Ok(StageTrace {
            name: "python_wrappers".to_string(),
            description: "verify generated Python stdlib wrapper package",
            status: if skipped_stdlib_build {
                "skipped".to_string()
            } else {
                "failed".to_string()
            },
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

fn render_conformance_trace_markdown(trace: &ConformanceTrace) -> String {
    let mut output = String::new();
    output.push_str("# Pilot Conformance Trace\n\n");
    output.push_str(&format!("- Release: `{}`\n", trace.release));
    output.push_str(&format!("- Status: `{}`\n", trace.overall_status));
    output.push_str(&format!("- Generated: `{}`\n", trace.generated_at_utc));
    output.push_str(&format!("- Profile: `{}`\n", trace.profile_id));
    output.push_str(&format!("- Corpus: `{}`\n", trace.corpus));
    output.push_str(&format!("- Source lock: `{}`\n\n", trace.source_lock));
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
        "Usage: qualify_pilot_release [--release 2026-01] [--pilot-root PATH] [--source-archive PATH] [--asset-dir PATH] [--out PATH] [--profile-id ID] [--spec-version VERSION] [--corpus NAME] [--wrapper-module MODULE] [--skip-stdlib-build] [--skip-parity]"
    );
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
        ];
        let summary = StageSummary::from_stages(&stages);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.total_duration_ms, 12);

        let trace = ConformanceTrace {
            schema: "dev.mercurio.pilot-release-conformance-trace.v1",
            generated_at_utc: "2026-06-16T00:00:00Z".to_string(),
            release: "2026-01".to_string(),
            spec_version: "2.0.0".to_string(),
            profile_id: "sysml-2.0-metamodel-0.57.0".to_string(),
            corpus: "small".to_string(),
            source_lock: "locks/source.lock.json".to_string(),
            overall_status: "failed".to_string(),
            stages,
        };
        let trace_markdown = render_conformance_trace_markdown(&trace);
        assert!(trace_markdown.contains("# Pilot Conformance Trace"));
        assert!(trace_markdown.contains("`pilot_java_artifacts`"));
        assert!(trace_markdown.contains("pilot.jar"));

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
        };
        let qualification_markdown = render_qualification_markdown(&report);
        assert!(qualification_markdown.contains("# Pilot Release Qualification"));
        assert!(qualification_markdown.contains("- Passed: 1"));
        assert!(qualification_markdown.contains("- Skipped: 1"));
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
}
