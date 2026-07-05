use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use mercurio_core::KirDocument;
use mercurio_sysml::{
    SysmlJsonExportOptions, SysmlJsonImportOptions, compile_sysml_module_with_context,
    export_sysml_abstract_syntax_value, import_sysml_abstract_syntax_json, load_sysml_baseline,
    parse_sysml,
};
use mercurio_tools::default_pilot_root;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let args = Args::parse()?;
    std::fs::create_dir_all(&args.out_dir)?;

    let input_paths = args.input_paths();
    let mercurio_document = compile_mercurio_sources(&input_paths)?;
    let mercurio_export = export_sysml_abstract_syntax_value(
        &mercurio_document,
        SysmlJsonExportOptions {
            source_uri: Some(args.model.display().to_string()),
            schema_profile: Some("sysml-api-json".to_string()),
            include_mercurio_extensions: true,
        },
    )?;
    let mercurio_json_path = args.out_dir.join("mercurio.sysml.json");
    write_json(&mercurio_json_path, &mercurio_export.value)?;

    let mercurio_reimport = import_sysml_abstract_syntax_json(
        &std::fs::read_to_string(&mercurio_json_path)?,
        SysmlJsonImportOptions {
            source_uri: Some(mercurio_json_path.display().to_string()),
            source_kind: Some("mercurio_export".to_string()),
            ..Default::default()
        },
    )?;
    let mercurio_reimport_path = args.out_dir.join("mercurio.reimport.kir.json");
    write_json(&mercurio_reimport_path, &mercurio_reimport.document)?;

    let interactive_jar = find_interactive_jar(&args.pilot_root)?;
    let classes_dir = tool_repo_path("target/pilot-exporter-classes");
    let probe_source = tool_repo_path(
        "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotJsonInteropProbe.java",
    );
    compile_java_probe(
        &interactive_jar,
        &probe_source,
        &classes_dir,
        "dev/mercurio/pilot/PilotJsonInteropProbe.class",
    )?;

    let pilot_read_report_path = args.out_dir.join("pilot.read-mercurio-json.json");
    run_java_probe(
        &interactive_jar,
        &classes_dir,
        &[
            "--read-json".to_string(),
            mercurio_json_path.display().to_string(),
            pilot_read_report_path.display().to_string(),
        ],
    )?;
    let pilot_read_report: PilotReadReport =
        serde_json::from_str(&std::fs::read_to_string(&pilot_read_report_path)?)?;

    let pilot_json_path = args.out_dir.join("pilot.api.json");
    let mut pilot_export_args = vec![
        "--export-api-json".to_string(),
        args.library_root().display().to_string(),
        pilot_json_path.display().to_string(),
    ];
    pilot_export_args.extend(input_paths.iter().map(|path| path.display().to_string()));
    run_java_probe(&interactive_jar, &classes_dir, &pilot_export_args)?;

    let pilot_import = import_sysml_abstract_syntax_json(
        &std::fs::read_to_string(&pilot_json_path)?,
        SysmlJsonImportOptions {
            source_uri: Some(pilot_json_path.display().to_string()),
            source_kind: Some("pilot_api_json".to_string()),
            ..Default::default()
        },
    )?;
    let pilot_import_path = args.out_dir.join("pilot.imported.kir.json");
    write_json(&pilot_import_path, &pilot_import.document)?;

    let report = InteropReport {
        generated_at_utc: now_utc_rfc3339()?,
        status: if mercurio_export.has_errors()
            || mercurio_reimport.has_errors()
            || pilot_read_report.status != "ok"
            || pilot_import.has_errors()
        {
            "error".to_string()
        } else {
            "ok".to_string()
        },
        pilot_root: args.pilot_root.display().to_string(),
        model: args.model.display().to_string(),
        support_files: args
            .support
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        artifacts: BTreeMap::from([
            (
                "mercurio_sysml_json".to_string(),
                mercurio_json_path.display().to_string(),
            ),
            (
                "mercurio_reimport_kir".to_string(),
                mercurio_reimport_path.display().to_string(),
            ),
            (
                "pilot_read_mercurio_json".to_string(),
                pilot_read_report_path.display().to_string(),
            ),
            (
                "pilot_api_json".to_string(),
                pilot_json_path.display().to_string(),
            ),
            (
                "pilot_imported_kir".to_string(),
                pilot_import_path.display().to_string(),
            ),
        ]),
        metrics: InteropMetrics {
            mercurio_compiled_elements: mercurio_document.elements.len(),
            mercurio_exported_elements: mercurio_export
                .value
                .get("elements")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0),
            mercurio_export_diagnostics: mercurio_export.diagnostics.len(),
            mercurio_reimport_elements: mercurio_reimport.document.elements.len(),
            mercurio_reimport_diagnostics: mercurio_reimport.diagnostics.len(),
            pilot_read_elements: pilot_read_report.element_count,
            pilot_read_diagnostics: pilot_read_report.diagnostics.len(),
            pilot_import_elements: pilot_import.document.elements.len(),
            pilot_import_diagnostics: pilot_import.diagnostics.len(),
            total_ms: elapsed_ms(started),
        },
        diagnostics: json!({
            "mercurioExport": mercurio_export.diagnostics,
            "mercurioReimport": mercurio_reimport.diagnostics,
            "pilotReadMercurioJson": pilot_read_report.diagnostics,
            "pilotImport": pilot_import.diagnostics,
        }),
    };

    let report_path = args.out_dir.join("interop-report.json");
    write_json(&report_path, &report)?;

    println!("SysML JSON interop validation");
    println!("  status: {}", report.status);
    println!("  report: {}", report_path.display());
    println!(
        "  Mercurio export/read: {} elements, {} diagnostics",
        report.metrics.mercurio_exported_elements, report.metrics.mercurio_export_diagnostics
    );
    println!(
        "  Pilot read Mercurio JSON: {} elements, {} diagnostics",
        report.metrics.pilot_read_elements, report.metrics.pilot_read_diagnostics
    );
    println!(
        "  Pilot JSON imported by Mercurio: {} elements, {} diagnostics",
        report.metrics.pilot_import_elements, report.metrics.pilot_import_diagnostics
    );

    if report.status == "ok" {
        Ok(())
    } else {
        Err("SysML JSON interop validation failed; see interop-report.json".into())
    }
}

#[derive(Debug)]
struct Args {
    pilot_root: PathBuf,
    model: PathBuf,
    support: Vec<PathBuf>,
    out_dir: PathBuf,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut pilot_root = None;
        let mut model = None;
        let mut support = Vec::new();
        let mut out_dir = PathBuf::from("target/sysml-json-interop");

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--pilot-root" => pilot_root = Some(next_path(&mut args, "--pilot-root")?),
                "--model" => model = Some(next_path(&mut args, "--model")?),
                "--support" => support.push(next_path(&mut args, "--support")?),
                "--out" => out_dir = next_path(&mut args, "--out")?,
                "-h" | "--help" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => return Err(format!("unknown argument `{other}`").into()),
            }
        }

        Ok(Self {
            pilot_root: pilot_root.unwrap_or_else(default_pilot_root),
            model: model.ok_or("missing required --model")?,
            support,
            out_dir,
        })
    }

    fn input_paths(&self) -> Vec<PathBuf> {
        let mut paths = self.support.clone();
        paths.push(self.model.clone());
        paths
    }

    fn library_root(&self) -> PathBuf {
        self.pilot_root.join("sysml.library")
    }
}

fn next_path(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing value for {flag}").into())
}

fn print_usage() {
    println!(
        "Usage: compare_sysml_json_interop --model <model.sysml> [--support <file.sysml> ...] [--pilot-root <path>] [--out <dir>]"
    );
}

fn compile_mercurio_sources(paths: &[PathBuf]) -> Result<KirDocument, Box<dyn std::error::Error>> {
    let stdlib = load_sysml_baseline()?;
    let mut parsed = Vec::new();
    for path in paths {
        let source = std::fs::read_to_string(path)?;
        parsed.push((path.clone(), parse_sysml(&source)?));
    }

    let modules = parsed
        .iter()
        .map(|(_, module)| module.clone())
        .collect::<Vec<_>>();
    let mut documents = Vec::new();
    for (path, module) in &parsed {
        let source_name = path.display().to_string();
        let document = compile_sysml_module_with_context(module, &source_name, &modules, &stdlib)?;
        documents.push(document);
    }

    Ok(KirDocument::merge(documents)?)
}

fn find_interactive_jar(pilot_root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let target_dir = pilot_root.join("org.omg.sysml.interactive/target");
    let mut jars = std::fs::read_dir(&target_dir)?
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
        .collect::<Vec<_>>();
    jars.sort();

    jars.into_iter().last().ok_or_else(|| {
        format!(
            "could not find org.omg.sysml.interactive-*-all.jar under {}",
            target_dir.display()
        )
        .into()
    })
}

fn compile_java_probe(
    interactive_jar: &Path,
    java_source: &Path,
    classes_dir: &Path,
    class_file_relative: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let class_file = classes_dir.join(class_file_relative);
    let should_compile = match (
        std::fs::metadata(java_source),
        std::fs::metadata(&class_file),
    ) {
        (Ok(source), Ok(class)) => source.modified()? > class.modified()?,
        _ => true,
    };
    if !should_compile {
        return Ok(());
    }

    std::fs::create_dir_all(classes_dir)?;
    let status = Command::new("javac")
        .arg("-cp")
        .arg(interactive_jar)
        .arg("-d")
        .arg(classes_dir)
        .arg(java_source)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err("failed to compile PilotJsonInteropProbe.java".into())
    }
}

fn run_java_probe(
    interactive_jar: &Path,
    classes_dir: &Path,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let classes_dir = absolute_path(classes_dir)?;
    let interactive_jar = absolute_path(interactive_jar)?;
    let lib_dir = interactive_jar
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("lib")
        .to_path_buf();
    let separator = if cfg!(windows) { ";" } else { ":" };
    let classpath = format!(
        "{}{}{}{}{}",
        java_path_string(&classes_dir),
        separator,
        java_path_string(&interactive_jar),
        separator,
        java_path_string(&lib_dir.join("*"))
    );

    let status = Command::new("java")
        .arg("-cp")
        .arg(classpath)
        .arg("dev.mercurio.pilot.PilotJsonInteropProbe")
        .args(args)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err("failed to run PilotJsonInteropProbe".into())
    }
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

fn tool_repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("mercurio-tools lives under crates")
        .join(relative)
}

fn absolute_path(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(env::current_dir()?.join(path))
}

fn java_path_string(path: &Path) -> String {
    path.display().to_string().replace("\\\\?\\", "")
}

fn now_utc_rfc3339() -> Result<String, Box<dyn std::error::Error>> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

fn elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

#[derive(Debug, Deserialize)]
struct PilotReadReport {
    status: String,
    element_count: usize,
    #[serde(default)]
    diagnostics: Vec<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InteropReport {
    generated_at_utc: String,
    status: String,
    pilot_root: String,
    model: String,
    support_files: Vec<String>,
    artifacts: BTreeMap<String, String>,
    metrics: InteropMetrics,
    diagnostics: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InteropMetrics {
    mercurio_compiled_elements: usize,
    mercurio_exported_elements: usize,
    mercurio_export_diagnostics: usize,
    mercurio_reimport_elements: usize,
    mercurio_reimport_diagnostics: usize,
    pilot_read_elements: usize,
    pilot_read_diagnostics: usize,
    pilot_import_elements: usize,
    pilot_import_diagnostics: usize,
    total_ms: u64,
}
