use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use mercurio_core::repo_path;
use serde::Deserialize;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    let manifest = BoundaryManifest::load(&args.manifest_path)?;
    let report = BoundaryReport::build(&manifest)?;

    print_report(&report);

    if !report.errors.is_empty()
        || (args.strict
            && (!report.known_migration_crates.is_empty()
                || !report.temporary_dependency_exceptions.is_empty()))
    {
        std::process::exit(1);
    }

    Ok(())
}

#[derive(Debug)]
struct Args {
    manifest_path: PathBuf,
    strict: bool,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut manifest_path = repo_path("repo-boundaries.json");
        let mut strict = false;
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--manifest" => {
                    index += 1;
                    manifest_path =
                        PathBuf::from(args.get(index).ok_or("missing value for --manifest")?);
                }
                "--strict" => strict = true,
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}").into()),
            }
            index += 1;
        }

        Ok(Self {
            manifest_path,
            strict,
        })
    }
}

#[derive(Debug, Deserialize)]
struct BoundaryManifest {
    allowed_core_crates: BTreeSet<String>,
    known_migration_crates: BTreeSet<String>,
    #[serde(default)]
    forbidden_dependencies: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    temporary_dependency_exceptions: BTreeMap<String, BTreeSet<String>>,
    forbidden_root_dirs: BTreeSet<String>,
}

impl BoundaryManifest {
    fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
}

#[derive(Debug)]
struct BoundaryReport {
    allowed_core_crates: Vec<String>,
    known_migration_crates: Vec<String>,
    unexpected_crates: Vec<String>,
    forbidden_dirs_present: Vec<String>,
    forbidden_dependencies: Vec<String>,
    temporary_dependency_exceptions: Vec<String>,
    errors: Vec<String>,
}

impl BoundaryReport {
    fn build(manifest: &BoundaryManifest) -> Result<Self, Box<dyn std::error::Error>> {
        let crates = discover_child_dirs(&repo_path("crates"))?;
        let root_dirs = discover_child_dirs(&repo_path("."))?;
        let mut allowed_core_crates = Vec::new();
        let mut known_migration_crates = Vec::new();
        let mut unexpected_crates = Vec::new();
        let mut forbidden_dirs_present = Vec::new();
        let mut forbidden_dependencies = Vec::new();
        let mut temporary_dependency_exceptions = Vec::new();
        let mut errors = Vec::new();

        for crate_name in crates {
            if manifest.allowed_core_crates.contains(&crate_name) {
                allowed_core_crates.push(crate_name);
            } else if manifest.known_migration_crates.contains(&crate_name) {
                known_migration_crates.push(crate_name);
            } else {
                errors.push(format!(
                    "crate `{crate_name}` is not classified in repo-boundaries.json"
                ));
                unexpected_crates.push(crate_name);
            }
        }

        for dir_name in root_dirs {
            if manifest.forbidden_root_dirs.contains(&dir_name) {
                errors.push(format!(
                    "root directory `{dir_name}` belongs in a peer repository"
                ));
                forbidden_dirs_present.push(dir_name);
            }
        }

        for (crate_name, forbidden) in &manifest.forbidden_dependencies {
            let dependencies = crate_dependencies(crate_name)?;
            for dependency in dependencies.intersection(forbidden) {
                let message = format!("crate `{crate_name}` must not depend on `{dependency}`");
                let is_temporary = manifest
                    .temporary_dependency_exceptions
                    .get(crate_name)
                    .is_some_and(|exceptions| exceptions.contains(dependency));
                if is_temporary {
                    temporary_dependency_exceptions.push(message);
                } else {
                    errors.push(message.clone());
                    forbidden_dependencies.push(message);
                }
            }
        }

        Ok(Self {
            allowed_core_crates,
            known_migration_crates,
            unexpected_crates,
            forbidden_dirs_present,
            forbidden_dependencies,
            temporary_dependency_exceptions,
            errors,
        })
    }
}

fn crate_dependencies(crate_name: &str) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let manifest_path = repo_path(&format!("crates/{crate_name}/Cargo.toml"));
    let contents = std::fs::read_to_string(manifest_path)?;
    let mut dependencies = BTreeSet::new();
    let mut in_dependencies = false;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            in_dependencies = matches!(
                line,
                "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]"
            );
            continue;
        }
        if !in_dependencies || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, _)) = line.split_once('=') else {
            continue;
        };
        dependencies.insert(name.trim().trim_matches('"').to_string());
    }

    Ok(dependencies)
}

fn discover_child_dirs(root: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut dirs = std::fs::read_dir(root)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            path.is_dir()
                .then(|| entry.file_name().to_string_lossy().trim().to_string())
        })
        .filter(|name| !name.is_empty() && name != "target" && name != ".git")
        .collect::<Vec<_>>();
    dirs.sort();
    Ok(dirs)
}

fn print_report(report: &BoundaryReport) {
    println!("repo boundary check");
    println!(
        "  allowed core crates: {}",
        join_or_dash(&report.allowed_core_crates)
    );
    println!(
        "  known migration crates: {}",
        join_or_dash(&report.known_migration_crates)
    );
    println!(
        "  unexpected crates: {}",
        join_or_dash(&report.unexpected_crates)
    );
    println!(
        "  forbidden root dirs: {}",
        join_or_dash(&report.forbidden_dirs_present)
    );
    println!(
        "  forbidden dependencies: {}",
        join_or_dash(&report.forbidden_dependencies)
    );
    println!(
        "  temporary dependency exceptions: {}",
        join_or_dash(&report.temporary_dependency_exceptions)
    );

    if report.errors.is_empty() {
        println!("  status: ok");
    } else {
        println!("  status: error");
        for error in &report.errors {
            println!("  error: {error}");
        }
    }
}

fn join_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(", ")
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --bin check_repo_boundaries -- [--manifest PATH] [--strict]"
    );
}
