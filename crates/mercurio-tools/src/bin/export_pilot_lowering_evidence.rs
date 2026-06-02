use std::path::{Path, PathBuf};
use std::process::Command;

use mercurio_core::repo_path;
use mercurio_tools::default_pilot_root;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    let output_path = export_from_pilot(&args.pilot_root, &args.output_path)?;

    println!("Exported Pilot lowering evidence:");
    println!("  pilot root: {}", args.pilot_root.display());
    println!("  output: {}", output_path.display());
    Ok(())
}

struct Args {
    pilot_root: PathBuf,
    output_path: PathBuf,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut pilot_root = default_pilot_root();
        let mut output_path = repo_path("target/pilot_lowering_evidence.json");
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--pilot-root" => {
                    index += 1;
                    pilot_root =
                        PathBuf::from(args.get(index).ok_or("missing --pilot-root value")?);
                }
                "--out" => {
                    index += 1;
                    output_path = PathBuf::from(args.get(index).ok_or("missing --out value")?);
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
            output_path,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --bin export_pilot_lowering_evidence -- [--pilot-root PATH] [--out PATH]"
    );
}

fn export_from_pilot(
    pilot_root: &Path,
    output_path: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let pilot_root = pilot_root.canonicalize()?;
    let interactive_jar = find_interactive_jar(&pilot_root)?;
    let classes_dir = repo_path("target/pilot-exporter-classes");
    let java_source = repo_path(
        "../mercurio-sysml/tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotLoweringEvidenceExporter.java",
    );

    compile_java_exporter(&interactive_jar, &java_source, &classes_dir)?;
    run_java_exporter(&interactive_jar, &classes_dir, output_path)?;
    Ok(output_path.to_path_buf())
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

fn compile_java_exporter(
    interactive_jar: &Path,
    java_source: &Path,
    classes_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let class_file = classes_dir.join("dev/mercurio/pilot/PilotLoweringEvidenceExporter.class");
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

    if !status.success() {
        return Err("failed to compile Java Pilot lowering evidence exporter".into());
    }
    Ok(())
}

fn run_java_exporter(
    interactive_jar: &Path,
    classes_dir: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let classes_dir = absolute_path(classes_dir)?;
    let interactive_jar = absolute_path(interactive_jar)?;
    let lib_dir = interactive_jar
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("lib");
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
        .arg("dev.mercurio.pilot.PilotLoweringEvidenceExporter")
        .arg(output_path)
        .status()?;

    if !status.success() {
        return Err("failed to run Java Pilot lowering evidence exporter".into());
    }
    Ok(())
}

fn absolute_path(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn java_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
