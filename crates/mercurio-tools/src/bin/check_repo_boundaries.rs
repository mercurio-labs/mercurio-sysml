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
    #[serde(default)]
    allowed_support_crates: BTreeSet<String>,
    known_migration_crates: BTreeSet<String>,
    #[serde(default)]
    forbidden_dependencies: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    temporary_dependency_exceptions: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    reasoning_ai_boundary: Option<ReasoningAiBoundary>,
    forbidden_root_dirs: BTreeSet<String>,
}

impl BoundaryManifest {
    fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
}

#[derive(Debug, Deserialize)]
struct ReasoningAiBoundary {
    workspace_root: PathBuf,
    ai_crate: String,
    deterministic_crates: BTreeSet<String>,
}

#[derive(Debug)]
struct BoundaryReport {
    allowed_core_crates: Vec<String>,
    allowed_support_crates: Vec<String>,
    known_migration_crates: Vec<String>,
    unexpected_crates: Vec<String>,
    forbidden_dirs_present: Vec<String>,
    forbidden_dependencies: Vec<String>,
    temporary_dependency_exceptions: Vec<String>,
    reasoning_ai_boundary: Vec<String>,
    errors: Vec<String>,
}

impl BoundaryReport {
    fn build(manifest: &BoundaryManifest) -> Result<Self, Box<dyn std::error::Error>> {
        let crates = discover_child_dirs(&repo_path("crates"))?;
        let root_dirs = discover_child_dirs(&repo_path("."))?;
        let mut allowed_core_crates = Vec::new();
        let mut allowed_support_crates = Vec::new();
        let mut known_migration_crates = Vec::new();
        let mut unexpected_crates = Vec::new();
        let mut forbidden_dirs_present = Vec::new();
        let mut forbidden_dependencies = Vec::new();
        let mut temporary_dependency_exceptions = Vec::new();
        let mut reasoning_ai_boundary = Vec::new();
        let mut errors = Vec::new();

        if manifest.forbidden_dependencies.is_empty() {
            errors.push(
                "repo-boundaries.json must declare forbidden_dependencies tier rules".to_string(),
            );
        }

        for crate_name in &crates {
            match classify_crate(manifest, crate_name) {
                CrateClassification::AllowedCore => allowed_core_crates.push(crate_name.clone()),
                CrateClassification::AllowedSupport => {
                    allowed_support_crates.push(crate_name.clone())
                }
                CrateClassification::KnownMigration => {
                    known_migration_crates.push(crate_name.clone())
                }
                CrateClassification::Unclassified(message) => {
                    errors.push(message);
                    unexpected_crates.push(crate_name.clone());
                    continue;
                }
            }

            if let Some(message) = missing_forbidden_dependency_policy(manifest, crate_name) {
                errors.push(message);
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
            for violation in forbidden_dependency_violations(
                crate_name,
                &dependencies,
                forbidden,
                &manifest.temporary_dependency_exceptions,
            ) {
                if violation.temporary {
                    temporary_dependency_exceptions.push(violation.message);
                } else {
                    errors.push(violation.message.clone());
                    forbidden_dependencies.push(violation.message);
                }
            }
        }

        if let Some(boundary) = &manifest.reasoning_ai_boundary {
            let root = repo_path(".").join(&boundary.workspace_root);
            if root.is_dir() {
                for crate_name in &boundary.deterministic_crates {
                    let manifest_path = root.join("crates").join(crate_name).join("Cargo.toml");
                    let dependencies = crate_dependencies_from_manifest(&manifest_path)?;
                    let message = format!(
                        "crate `{crate_name}` is independent from `{}`",
                        boundary.ai_crate
                    );
                    if dependencies.contains(&boundary.ai_crate) {
                        let error =
                            reasoning_ai_dependency_violation(crate_name, &boundary.ai_crate);
                        errors.push(error.clone());
                        reasoning_ai_boundary.push(error);
                    } else {
                        reasoning_ai_boundary.push(message);
                    }
                }
            } else {
                reasoning_ai_boundary.push(format!(
                    "skipped reasoning/AI boundary; workspace root `{}` is not present",
                    root.display()
                ));
                if manifest.known_migration_crates.contains(&boundary.ai_crate) {
                    reasoning_ai_boundary.push(format!(
                        "crate `{}` is tracked as a migration crate",
                        boundary.ai_crate
                    ));
                } else {
                    reasoning_ai_boundary.push(format!(
                        "crate `{}` is external to this checkout",
                        boundary.ai_crate
                    ));
                }
            }
        }

        Ok(Self {
            allowed_core_crates,
            allowed_support_crates,
            known_migration_crates,
            unexpected_crates,
            forbidden_dirs_present,
            forbidden_dependencies,
            temporary_dependency_exceptions,
            reasoning_ai_boundary,
            errors,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CrateClassification {
    AllowedCore,
    AllowedSupport,
    KnownMigration,
    Unclassified(String),
}

#[derive(Debug, PartialEq, Eq)]
struct DependencyViolation {
    message: String,
    temporary: bool,
}

fn classify_crate(manifest: &BoundaryManifest, crate_name: &str) -> CrateClassification {
    if manifest.allowed_core_crates.contains(crate_name) {
        CrateClassification::AllowedCore
    } else if manifest.allowed_support_crates.contains(crate_name) {
        CrateClassification::AllowedSupport
    } else if manifest.known_migration_crates.contains(crate_name) {
        CrateClassification::KnownMigration
    } else {
        CrateClassification::Unclassified(format!(
            "crate `{crate_name}` is not classified in repo-boundaries.json"
        ))
    }
}

fn missing_forbidden_dependency_policy(
    manifest: &BoundaryManifest,
    crate_name: &str,
) -> Option<String> {
    (!manifest.forbidden_dependencies.contains_key(crate_name))
        .then(|| format!("crate `{crate_name}` has no forbidden_dependencies policy"))
}

fn forbidden_dependency_violations(
    crate_name: &str,
    dependencies: &BTreeSet<String>,
    forbidden: &BTreeSet<String>,
    temporary_exceptions: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<DependencyViolation> {
    dependencies
        .intersection(forbidden)
        .map(|dependency| DependencyViolation {
            message: format!("crate `{crate_name}` must not depend on `{dependency}`"),
            temporary: temporary_exceptions
                .get(crate_name)
                .is_some_and(|exceptions| exceptions.contains(dependency)),
        })
        .collect()
}

fn reasoning_ai_dependency_violation(crate_name: &str, ai_crate: &str) -> String {
    format!("deterministic reasoning crate `{crate_name}` must not depend on `{ai_crate}`")
}

fn crate_dependencies(crate_name: &str) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let manifest_path = repo_path(&format!("crates/{crate_name}/Cargo.toml"));
    crate_dependencies_from_manifest(&manifest_path)
}

fn crate_dependencies_from_manifest(
    manifest_path: &Path,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(manifest_path)?;
    Ok(crate_dependencies_from_toml(&contents))
}

fn crate_dependencies_from_toml(contents: &str) -> BTreeSet<String> {
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

    dependencies
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
        "  allowed support crates: {}",
        join_or_dash(&report.allowed_support_crates)
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
    println!(
        "  reasoning ai boundary: {}",
        join_or_dash(&report.reasoning_ai_boundary)
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
    println!(
        "From mercurio-foundation, use: cargo run --manifest-path ../mercurio-sysml/Cargo.toml -p mercurio-tools --bin check_repo_boundaries -- --manifest repo-boundaries.json"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_dependency_parser_reads_declared_dependencies() {
        let dependencies = crate_dependencies_from_toml(
            r#"
[package]
name = "demo"

[dependencies]
serde = "1"
mercurio-ai = { path = "../mercurio-ai" }

[dev-dependencies]
mercurio-plugin-api = { path = "../mercurio-plugin-api" }

[features]
default = []
"#,
        );

        assert!(dependencies.contains("serde"));
        assert!(dependencies.contains("mercurio-ai"));
        assert!(dependencies.contains("mercurio-plugin-api"));
        assert!(!dependencies.contains("default"));
    }

    #[test]
    fn unclassified_crate_is_reported() {
        let manifest = BoundaryManifest {
            allowed_core_crates: ["mercurio-foundation".to_string()].into_iter().collect(),
            allowed_support_crates: BTreeSet::new(),
            known_migration_crates: BTreeSet::new(),
            forbidden_dependencies: BTreeMap::new(),
            temporary_dependency_exceptions: BTreeMap::new(),
            reasoning_ai_boundary: None,
            forbidden_root_dirs: BTreeSet::new(),
        };

        assert_eq!(
            classify_crate(&manifest, "mercurio-surprise"),
            CrateClassification::Unclassified(
                "crate `mercurio-surprise` is not classified in repo-boundaries.json".to_string()
            )
        );
    }

    #[test]
    fn forbidden_dependency_violation_is_reported() {
        let dependencies = ["mercurio-ai".to_string()].into_iter().collect();
        let forbidden = ["mercurio-ai".to_string(), "mercurio-product".to_string()]
            .into_iter()
            .collect();

        let violations = forbidden_dependency_violations(
            "mercurio-runtime",
            &dependencies,
            &forbidden,
            &BTreeMap::new(),
        );

        assert_eq!(
            violations,
            vec![DependencyViolation {
                message: "crate `mercurio-runtime` must not depend on `mercurio-ai`".to_string(),
                temporary: false,
            }]
        );
    }

    #[test]
    fn reasoning_ai_dependency_violation_is_reported() {
        assert_eq!(
            reasoning_ai_dependency_violation("mercurio-plugin-api", "mercurio-ai"),
            "deterministic reasoning crate `mercurio-plugin-api` must not depend on `mercurio-ai`"
        );
    }
}
