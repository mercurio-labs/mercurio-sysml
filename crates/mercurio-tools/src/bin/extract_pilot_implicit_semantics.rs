use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use mercurio_tools::{default_pilot_root, load_pilot_lock, sha256_file};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const DEFAULT_PROFILE_ID: &str = "sysml-2.0-pilot-2026-04";
const DEFAULT_CORPUS: &str = "small";
const DEFAULT_MAX_RECORDED_ELEMENTS_PER_CASE: usize = 100;
const DEFAULT_MAX_RECORDED_RELATIONSHIPS_PER_CASE: usize = 200;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    validate_pilot_checkout(&args.pilot_root, args.allow_dirty)?;
    let extract = build_extract(&args)?;
    let serialized = serde_json::to_string_pretty(&extract)?;

    if args.check {
        let existing_text = std::fs::read_to_string(&args.out)?;
        let existing = normalize_for_check(serde_json::from_str::<Value>(&existing_text)?);
        let fresh = normalize_for_check(serde_json::from_str::<Value>(&serialized)?);
        if existing != fresh {
            return Err(format!(
                "implicit semantics extract drift: {} does not match fresh extraction from {}",
                args.out.display(),
                args.pilot_root.display()
            )
            .into());
        }
        println!(
            "implicit semantics extract is current: {}",
            args.out.display()
        );
        return Ok(());
    }

    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, format!("{serialized}\n"))?;
    println!("wrote implicit semantics extract: {}", args.out.display());
    Ok(())
}

#[derive(Debug)]
struct Args {
    pilot_root: PathBuf,
    profile_id: String,
    corpus: String,
    max_recorded_elements_per_case: usize,
    max_recorded_relationships_per_case: usize,
    out: PathBuf,
    check: bool,
    allow_dirty: bool,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut pilot_root = default_pilot_root();
        let mut profile_id = DEFAULT_PROFILE_ID.to_string();
        let mut corpus = DEFAULT_CORPUS.to_string();
        let mut max_recorded_elements_per_case = DEFAULT_MAX_RECORDED_ELEMENTS_PER_CASE;
        let mut max_recorded_relationships_per_case = DEFAULT_MAX_RECORDED_RELATIONSHIPS_PER_CASE;
        let mut out = None;
        let mut check = false;
        let mut allow_dirty = false;

        let raw = env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;
        while index < raw.len() {
            match raw[index].as_str() {
                "--pilot-root" => pilot_root = next_path(&raw, &mut index)?,
                "--profile-id" | "--profile" => profile_id = next_string(&raw, &mut index)?,
                "--corpus" => corpus = next_string(&raw, &mut index)?,
                "--max-recorded-elements-per-case" => {
                    max_recorded_elements_per_case = next_string(&raw, &mut index)?.parse()?
                }
                "--max-recorded-relationships-per-case" => {
                    max_recorded_relationships_per_case = next_string(&raw, &mut index)?.parse()?
                }
                "--out" => out = Some(next_path(&raw, &mut index)?),
                "--check" => check = true,
                "--allow-dirty" => allow_dirty = true,
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}").into()),
            }
            index += 1;
        }

        let out = out.unwrap_or_else(|| {
            PathBuf::from("resources")
                .join("metamodels")
                .join(&profile_id)
                .join("implicit_semantics.extract.json")
        });

        Ok(Self {
            pilot_root,
            profile_id,
            corpus,
            max_recorded_elements_per_case,
            max_recorded_relationships_per_case,
            out,
            check,
            allow_dirty,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_implicit_semantics -- [--pilot-root PATH] [--profile-id ID] [--corpus NAME] [--out PATH] [--check] [--allow-dirty]"
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
struct ImplicitSemanticsExtract {
    schema: &'static str,
    source: ExtractSource,
    corpus: CorpusSummary,
    summary: ExtractSummary,
    cases: Vec<ImplicitCaseExtract>,
}

#[derive(Serialize)]
struct ExtractSource {
    schema: &'static str,
    profile_id: String,
    pilot: PilotSource,
    source_files: Vec<SourceFile>,
    extractor: ExtractorSource,
    extracted_at_utc: String,
}

#[derive(Serialize)]
struct PilotSource {
    repository: String,
    commit: Option<String>,
    git_describe: Option<String>,
    dirty: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dirty_waiver: Option<String>,
}

#[derive(Serialize)]
struct SourceFile {
    path: String,
    sha256: String,
}

#[derive(Serialize)]
struct ExtractorSource {
    name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct CorpusSummary {
    name: String,
    case_count: usize,
    support_policy: &'static str,
}

#[derive(Serialize)]
struct ExtractSummary {
    case_count: usize,
    cases_with_implicit_deltas: usize,
    added_element_count: usize,
    added_relationship_count: usize,
    added_relationships_by_relation: BTreeMap<String, usize>,
    added_relationships_by_source_kind: BTreeMap<String, usize>,
}

#[derive(Serialize)]
struct ImplicitCaseExtract {
    relative_path: String,
    explicit_element_count: usize,
    explicit_relationship_count: usize,
    implicit_element_count: usize,
    implicit_relationship_count: usize,
    added_element_count: usize,
    added_relationship_count: usize,
    recorded_added_elements: Vec<ImpliedElement>,
    recorded_added_relationships: Vec<ImpliedRelationship>,
}

#[derive(Clone, Deserialize, Serialize)]
struct ImpliedElement {
    qualified_name: String,
    kind: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct ImpliedRelationship {
    source: String,
    relation: String,
    target: String,
    source_kind: Option<String>,
    target_kind: Option<String>,
}

#[derive(Deserialize)]
struct PilotCorpusSeed {
    #[serde(default)]
    corpora: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    support_dependencies: BTreeMap<String, Vec<String>>,
}

#[derive(Serialize)]
struct PilotCorpusSpec<'a> {
    cases: Vec<PilotCorpusSpecCase<'a>>,
}

#[derive(Serialize)]
struct PilotCorpusSpecCase<'a> {
    relative_path: &'a str,
    input_files: Vec<String>,
}

#[derive(Deserialize)]
struct JavaImplicitDiffDocument {
    cases: Vec<JavaImplicitDiffCase>,
}

#[derive(Deserialize)]
struct JavaImplicitDiffCase {
    relative_path: String,
    explicit_element_count: usize,
    explicit_relationship_count: usize,
    implicit_element_count: usize,
    implicit_relationship_count: usize,
    #[serde(default)]
    added_elements: Vec<ImpliedElement>,
    #[serde(default)]
    added_relationships: Vec<ImpliedRelationship>,
}

fn build_extract(args: &Args) -> Result<ImplicitSemanticsExtract, Box<dyn std::error::Error>> {
    let pilot_root = args.pilot_root.canonicalize()?;
    let corpus_seed = load_corpus_seed()?;
    let relative_paths = corpus_seed
        .corpora
        .get(&args.corpus)
        .cloned()
        .ok_or_else(|| format!("unknown corpus `{}`", args.corpus))?;

    let raw_diff =
        run_pilot_implicit_diff(&pilot_root, &args.corpus, &relative_paths, &corpus_seed)?;
    let summary = summarize_java_cases(&raw_diff.cases);
    let cases = raw_diff
        .cases
        .into_iter()
        .map(|case| ImplicitCaseExtract {
            relative_path: case.relative_path,
            explicit_element_count: case.explicit_element_count,
            explicit_relationship_count: case.explicit_relationship_count,
            implicit_element_count: case.implicit_element_count,
            implicit_relationship_count: case.implicit_relationship_count,
            added_element_count: case.added_elements.len(),
            added_relationship_count: case.added_relationships.len(),
            recorded_added_elements: case
                .added_elements
                .into_iter()
                .take(args.max_recorded_elements_per_case)
                .collect(),
            recorded_added_relationships: case
                .added_relationships
                .into_iter()
                .take(args.max_recorded_relationships_per_case)
                .collect(),
        })
        .collect::<Vec<_>>();

    Ok(ImplicitSemanticsExtract {
        schema: "https://mercurio.dev/schemas/pilot-implicit-semantics-extract/v1",
        source: ExtractSource {
            schema: "https://mercurio.dev/schemas/source-provenance/v1",
            profile_id: args.profile_id.clone(),
            pilot: pilot_source(&pilot_root, args.allow_dirty),
            source_files: source_files(&pilot_root, &relative_paths, &corpus_seed)?,
            extractor: ExtractorSource {
                name: "extract_pilot_implicit_semantics",
                version: env!("CARGO_PKG_VERSION"),
            },
            extracted_at_utc: OffsetDateTime::now_utc().format(&Rfc3339)?,
        },
        corpus: CorpusSummary {
            name: args.corpus.clone(),
            case_count: relative_paths.len(),
            support_policy: "explicit support_dependencies plus same-folder SysML files",
        },
        summary,
        cases,
    })
}

fn load_corpus_seed() -> Result<PilotCorpusSeed, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(
        tool_repo_path("crates/mercurio-tools/corpus/pilot_corpus.seed.json"),
    )?)?)
}

fn run_pilot_implicit_diff(
    pilot_root: &Path,
    corpus_name: &str,
    relative_paths: &[String],
    corpus_seed: &PilotCorpusSeed,
) -> Result<JavaImplicitDiffDocument, Box<dyn std::error::Error>> {
    let library_root = pilot_root.join("sysml.library");
    let interactive_jar = find_interactive_jar(pilot_root)?;
    let classes_dir = tool_repo_path("target/pilot-exporter-classes");
    let java_source = tool_repo_path(
        "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java",
    );
    compile_java_exporter(
        &interactive_jar,
        &java_source,
        &classes_dir,
        "dev/mercurio/pilot/PilotModelExporter.class",
    )?;

    let group_slug = corpus_name.replace(['\\', '/', ' '], "_");
    let spec_path = tool_repo_path(&format!(
        "target/pilot_implicit_semantics.{group_slug}.spec.json"
    ));
    let output_path = tool_repo_path(&format!(
        "target/pilot_implicit_semantics.{group_slug}.json"
    ));
    let spec = PilotCorpusSpec {
        cases: relative_paths
            .iter()
            .map(|relative_path| PilotCorpusSpecCase {
                relative_path,
                input_files: corpus_seed
                    .support_paths_for_case(pilot_root, relative_path)
                    .iter()
                    .map(|path| pilot_root.join(path).display().to_string())
                    .chain(std::iter::once(
                        pilot_root.join(relative_path).display().to_string(),
                    ))
                    .collect(),
            })
            .collect(),
    };

    if let Some(parent) = spec_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&spec_path, serde_json::to_string_pretty(&spec)?)?;
    run_java_implicit_diff(
        &interactive_jar,
        &classes_dir,
        &library_root,
        &spec_path,
        &output_path,
    )?;
    Ok(serde_json::from_str(&std::fs::read_to_string(
        output_path,
    )?)?)
}

impl PilotCorpusSeed {
    fn support_paths_for(&self, relative_path: &str) -> &[String] {
        self.support_dependencies
            .get(relative_path)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn support_paths_for_case(&self, pilot_root: &Path, relative_path: &str) -> Vec<String> {
        let mut support_paths = Vec::new();
        for path in self.support_paths_for(relative_path) {
            push_unique(&mut support_paths, path.clone());
        }
        for path in same_folder_sysml_paths(pilot_root, relative_path) {
            if path != relative_path {
                push_unique(&mut support_paths, path);
            }
        }
        support_paths
    }
}

fn same_folder_sysml_paths(pilot_root: &Path, relative_path: &str) -> Vec<String> {
    let Some(parent) = Path::new(relative_path).parent() else {
        return Vec::new();
    };
    let folder = pilot_root.join(parent);
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };

    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "sysml")
        })
        .filter_map(|path| {
            path.strip_prefix(pilot_root)
                .ok()
                .map(|path| path.to_string_lossy().replace('\\', "/"))
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn source_files(
    pilot_root: &Path,
    relative_paths: &[String],
    corpus_seed: &PilotCorpusSeed,
) -> Result<Vec<SourceFile>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    push_source_file(
        &mut files,
        &tool_repo_path("crates/mercurio-tools/corpus/pilot_corpus.seed.json"),
    )?;
    push_source_file(
        &mut files,
        &tool_repo_path(
            "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java",
        ),
    )?;

    let mut pilot_files = BTreeSet::new();
    for relative_path in relative_paths {
        pilot_files.insert(relative_path.clone());
        for support_path in corpus_seed.support_paths_for_case(pilot_root, relative_path) {
            pilot_files.insert(support_path);
        }
    }
    for relative_path in pilot_files {
        let path = pilot_root.join(&relative_path);
        files.push(SourceFile {
            path: relative_path,
            sha256: sha256_file(&path)?,
        });
    }
    Ok(files)
}

fn push_source_file(
    files: &mut Vec<SourceFile>,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    files.push(SourceFile {
        path: source_path(&tool_repo_path(""), path),
        sha256: sha256_file(path)?,
    });
    Ok(())
}

fn summarize_java_cases(cases: &[JavaImplicitDiffCase]) -> ExtractSummary {
    let mut by_relation = BTreeMap::new();
    let mut by_source_kind = BTreeMap::new();
    let mut added_element_count = 0;
    let mut added_relationship_count = 0;
    let mut cases_with_implicit_deltas = 0;

    for case in cases {
        added_element_count += case.added_elements.len();
        added_relationship_count += case.added_relationships.len();
        if !case.added_elements.is_empty() || !case.added_relationships.is_empty() {
            cases_with_implicit_deltas += 1;
        }
        for relationship in &case.added_relationships {
            *by_relation
                .entry(relationship.relation.clone())
                .or_insert(0) += 1;
            *by_source_kind
                .entry(
                    relationship
                        .source_kind
                        .clone()
                        .unwrap_or_else(|| "<unknown>".to_string()),
                )
                .or_insert(0) += 1;
        }
    }

    ExtractSummary {
        case_count: cases.len(),
        cases_with_implicit_deltas,
        added_element_count,
        added_relationship_count,
        added_relationships_by_relation: by_relation,
        added_relationships_by_source_kind: by_source_kind,
    }
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
    if !status.success() {
        return Err("failed to compile Java pilot exporter".into());
    }
    Ok(())
}

fn run_java_implicit_diff(
    interactive_jar: &Path,
    classes_dir: &Path,
    library_root: &Path,
    spec_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let classes_dir = absolute_path(classes_dir)?;
    let interactive_jar = absolute_path(interactive_jar)?;
    let spec_path = absolute_path(spec_path)?;
    let output_path = absolute_path(output_path)?;
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

    let status = if cfg!(windows) {
        let script_path = tool_repo_path("target/run_pilot_implicit_semantics.ps1");
        let script = format!(
            "$cp = '{}'\njava -cp $cp dev.mercurio.pilot.PilotModelExporter --implicit-diff '{}' '{}' '{}'\n",
            classpath.replace('\'', "''"),
            java_path_string(library_root).replace('\'', "''"),
            java_path_string(&spec_path).replace('\'', "''"),
            java_path_string(&output_path).replace('\'', "''"),
        );
        std::fs::write(&script_path, script)?;
        Command::new("powershell")
            .arg("-File")
            .arg(script_path)
            .status()?
    } else {
        Command::new("java")
            .arg("-cp")
            .arg(classpath)
            .arg("dev.mercurio.pilot.PilotModelExporter")
            .arg("--implicit-diff")
            .arg(library_root)
            .arg(spec_path)
            .arg(output_path)
            .status()?
    };
    if !status.success() {
        return Err("failed to run Java pilot implicit semantics exporter".into());
    }
    Ok(())
}

fn validate_pilot_checkout(
    pilot_root: &Path,
    allow_dirty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let lock = load_pilot_lock()?;
    let head = git_output(pilot_root, ["rev-parse", "HEAD"])?;
    if head.as_deref() != Some(lock.commit.as_str()) {
        return Err(format!(
            "Pilot checkout `{}` is at `{}`, expected `{}` from resources/pilot.lock.json",
            pilot_root.display(),
            head.unwrap_or_else(|| "<unknown>".to_string()),
            lock.commit
        )
        .into());
    }

    let dirty = pilot_dirty(pilot_root)?;
    if dirty && !allow_dirty {
        return Err(format!(
            "Pilot checkout `{}` has uncommitted changes; rerun with --allow-dirty only for non-release debug extraction",
            pilot_root.display()
        )
        .into());
    }
    Ok(())
}

fn pilot_source(pilot_root: &Path, allow_dirty: bool) -> PilotSource {
    let lock = load_pilot_lock().ok();
    let commit = git_output(pilot_root, ["rev-parse", "HEAD"])
        .ok()
        .flatten()
        .or_else(|| lock.as_ref().map(|lock| lock.commit.clone()));
    let git_describe = git_output(pilot_root, ["describe", "--tags", "--always", "--dirty"])
        .ok()
        .flatten()
        .or_else(|| lock.as_ref().and_then(|lock| lock.git_describe.clone()));
    let dirty = pilot_dirty(pilot_root).ok();
    PilotSource {
        repository: lock
            .as_ref()
            .map(|lock| lock.repository.clone())
            .unwrap_or_else(|| {
                "https://github.com/Systems-Modeling/SysML-v2-Pilot-Implementation".to_string()
            }),
        commit,
        git_describe,
        dirty,
        dirty_waiver: (dirty == Some(true) && allow_dirty)
            .then_some("local debug extraction allowed with --allow-dirty".to_string()),
    }
}

fn pilot_dirty(pilot_root: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let status = git_output(pilot_root, ["status", "--porcelain"])?;
    Ok(status.is_some_and(|status| !status.trim().is_empty()))
}

fn git_output<const N: usize>(
    cwd: &Path,
    args: [&str; N],
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}

fn normalize_for_check(mut value: Value) -> Value {
    if let Some(source) = value.get_mut("source").and_then(Value::as_object_mut) {
        source.insert(
            "extracted_at_utc".to_string(),
            Value::String("<normalized>".to_string()),
        );
        if let Some(pilot) = source.get_mut("pilot").and_then(Value::as_object_mut) {
            pilot.insert("dirty".to_string(), Value::Bool(false));
            pilot.remove("dirty_waiver");
        }
    }
    value
}

fn absolute_path(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(env::current_dir()?.join(path))
}

fn tool_repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("mercurio-tools lives under crates")
        .join(relative)
}

fn source_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn java_path_string(path: &Path) -> String {
    path.display().to_string().replace("\\\\?\\", "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_relationships_by_relation_and_source_kind() {
        let java_cases = vec![JavaImplicitDiffCase {
            relative_path: "example.sysml".to_string(),
            explicit_element_count: 1,
            explicit_relationship_count: 1,
            implicit_element_count: 2,
            implicit_relationship_count: 3,
            added_elements: vec![ImpliedElement {
                qualified_name: "x".to_string(),
                kind: "FeatureTyping".to_string(),
            }],
            added_relationships: vec![
                ImpliedRelationship {
                    source: "a".to_string(),
                    relation: "specializes".to_string(),
                    target: "b".to_string(),
                    source_kind: Some("PartUsage".to_string()),
                    target_kind: Some("PartDefinition".to_string()),
                },
                ImpliedRelationship {
                    source: "a".to_string(),
                    relation: "type".to_string(),
                    target: "c".to_string(),
                    source_kind: Some("PartUsage".to_string()),
                    target_kind: Some("PartDefinition".to_string()),
                },
            ],
        }];

        let summary = summarize_java_cases(&java_cases);

        assert_eq!(summary.case_count, 1);
        assert_eq!(summary.cases_with_implicit_deltas, 1);
        assert_eq!(summary.added_element_count, 1);
        assert_eq!(summary.added_relationship_count, 2);
        assert_eq!(summary.added_relationships_by_relation["type"], 1);
        assert_eq!(summary.added_relationships_by_source_kind["PartUsage"], 2);
    }
}
