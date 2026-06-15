use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use mercurio_core::repo_path;
use mercurio_tools::default_pilot_root;
use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let source_root = args.source_root();
    let mut files = Vec::new();
    collect_sysml_files(&source_root, &mut files)?;
    files.sort();

    let mut cases = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path)?;
        let relative_path = path
            .strip_prefix(&source_root)?
            .to_string_lossy()
            .replace('\\', "/");
        cases.push(audit_case(relative_path, &text));
    }

    let report = CoverageReport {
        generated_at_utc: OffsetDateTime::now_utc().format(&Rfc3339)?,
        source_root: source_root.display().to_string(),
        file_count: cases.len(),
        summary: CoverageSummary::from_cases(&cases),
        cases,
    };

    if let Some(parent) = args.output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.output_path, serde_json::to_string_pretty(&report)?)?;

    println!("pilot authoring coverage");
    println!("  source root: {}", report.source_root);
    println!("  files: {}", report.file_count);
    println!("  output: {}", args.output_path.display());
    for (status, count) in &report.summary.status_counts {
        println!("  {status}: {count}");
    }
    println!("  top gaps:");
    for gap in report.summary.top_gap_tags.iter().take(12) {
        println!("    {}: {} files", gap.tag, gap.file_count);
    }
    println!(
        "  current structural candidates: {}",
        report.summary.current_structural_candidates.len()
    );
    println!(
        "  near structural candidates: {}",
        report.summary.near_structural_candidates.len()
    );
    Ok(())
}

struct Args {
    pilot_root: PathBuf,
    src_root: Option<PathBuf>,
    output_path: PathBuf,
}

impl Args {
    fn source_root(&self) -> PathBuf {
        if let Some(src_root) = &self.src_root {
            return src_root.clone();
        }
        let candidate = self.pilot_root.join("sysml").join("src");
        if candidate.is_dir() {
            candidate
        } else {
            self.pilot_root.clone()
        }
    }
}

fn parse_args() -> Result<Args, Box<dyn std::error::Error>> {
    let mut pilot_root = default_pilot_root();
    let mut src_root = None;
    let mut output_path = repo_path("target/pilot_authoring_coverage.json");
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--pilot-root" => {
                index += 1;
                pilot_root = PathBuf::from(args.get(index).ok_or("missing --pilot-root value")?);
            }
            "--src-root" => {
                index += 1;
                src_root = Some(PathBuf::from(
                    args.get(index).ok_or("missing --src-root value")?,
                ));
            }
            "--out" => {
                index += 1;
                output_path = PathBuf::from(args.get(index).ok_or("missing --out value")?);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument `{other}`").into()),
        }
        index += 1;
    }
    Ok(Args {
        pilot_root,
        src_root,
        output_path,
    })
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --bin pilot_authoring_coverage -- [--pilot-root PATH | --src-root PATH] [--out PATH]"
    );
}

#[derive(Debug, Serialize)]
struct CoverageReport {
    generated_at_utc: String,
    source_root: String,
    file_count: usize,
    summary: CoverageSummary,
    cases: Vec<CoverageCase>,
}

#[derive(Debug, Serialize)]
struct CoverageSummary {
    status_counts: BTreeMap<String, usize>,
    tag_file_counts: BTreeMap<String, usize>,
    top_gap_tags: Vec<GapCount>,
    current_structural_candidates: Vec<String>,
    near_structural_candidates: Vec<NearCandidate>,
}

impl CoverageSummary {
    fn from_cases(cases: &[CoverageCase]) -> Self {
        let mut status_counts = BTreeMap::new();
        let mut tag_file_counts = BTreeMap::new();
        for case in cases {
            *status_counts.entry(case.status.clone()).or_insert(0) += 1;
            for tag in &case.tags {
                *tag_file_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        let mut top_gap_tags = tag_file_counts
            .iter()
            .filter(|(tag, _)| !CURRENTLY_SUPPORTED_TAGS.contains(&tag.as_str()))
            .map(|(tag, file_count)| GapCount {
                tag: tag.clone(),
                file_count: *file_count,
            })
            .collect::<Vec<_>>();
        top_gap_tags.sort_by(|left, right| {
            right
                .file_count
                .cmp(&left.file_count)
                .then_with(|| left.tag.cmp(&right.tag))
        });
        let current_structural_candidates = cases
            .iter()
            .filter(|case| case.status == "current_structural_candidate")
            .map(|case| case.relative_path.clone())
            .collect::<Vec<_>>();
        let near_structural_candidates = cases
            .iter()
            .filter(|case| {
                case.status == "needs_structural_primitives"
                    && case.unsupported_tags.iter().all(|tag| tag == "short_name")
            })
            .map(|case| NearCandidate {
                relative_path: case.relative_path.clone(),
                unsupported_tags: case.unsupported_tags.clone(),
            })
            .collect::<Vec<_>>();
        Self {
            status_counts,
            tag_file_counts,
            top_gap_tags,
            current_structural_candidates,
            near_structural_candidates,
        }
    }
}

#[derive(Debug, Serialize)]
struct GapCount {
    tag: String,
    file_count: usize,
}

#[derive(Debug, Serialize)]
struct NearCandidate {
    relative_path: String,
    unsupported_tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CoverageCase {
    relative_path: String,
    status: String,
    max_tier: u8,
    tags: Vec<String>,
    unsupported_tags: Vec<String>,
    construct_counts: BTreeMap<String, usize>,
}

fn audit_case(relative_path: String, text: &str) -> CoverageCase {
    let construct_counts = construct_counts(text);
    let tags = tags_for_counts(&construct_counts, text);
    let max_tier = tags.iter().map(|tag| tier_for_tag(tag)).max().unwrap_or(1);
    let unsupported_tags = tags
        .iter()
        .filter(|tag| !CURRENTLY_SUPPORTED_TAGS.contains(&tag.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let status = status_for(max_tier, &unsupported_tags).to_string();
    CoverageCase {
        relative_path,
        status,
        max_tier,
        tags,
        unsupported_tags,
        construct_counts,
    }
}

fn status_for(max_tier: u8, unsupported_tags: &[String]) -> &'static str {
    if unsupported_tags.is_empty() {
        "current_structural_candidate"
    } else {
        match max_tier {
            0 | 1 => "needs_structural_primitives",
            2 => "needs_relationship_primitives",
            3 => "needs_behavior_primitives",
            _ => "needs_analysis_view_or_occurrence_primitives",
        }
    }
}

fn construct_counts(text: &str) -> BTreeMap<String, usize> {
    CONSTRUCT_PATTERNS
        .iter()
        .filter_map(|pattern| {
            let count = pattern
                .needles
                .iter()
                .map(|needle| text.matches(needle).count())
                .sum::<usize>();
            (count > 0).then(|| (pattern.tag.to_string(), count))
        })
        .collect()
}

fn tags_for_counts(counts: &BTreeMap<String, usize>, text: &str) -> Vec<String> {
    let mut tags = counts.keys().cloned().collect::<BTreeSet<_>>();
    if text.contains('\'') {
        tags.insert("quoted_name".to_string());
    }
    if contains_short_name(text) {
        tags.insert("short_name".to_string());
    }
    if text.contains(" ordered") || text.contains(" nonunique") || text.contains(" derived") {
        tags.insert("advanced_modifier".to_string());
    }
    if text.contains(" action ") && text.contains('{') {
        tags.insert("action_body".to_string());
    }
    if text.contains(" constraint ") && text.contains('{') {
        tags.insert("constraint_body".to_string());
    }
    if text.contains('@') {
        tags.insert("metadata_annotation".to_string());
    }
    if text.contains('#') {
        tags.insert("language_extension_keyword".to_string());
    }
    if text.contains("metadata ") && text.contains(" about ") {
        tags.insert("metadata_usage_about".to_string());
    }
    tags.into_iter().collect()
}

fn tier_for_tag(tag: &str) -> u8 {
    match tag {
        "package"
        | "import"
        | "part_def"
        | "part_usage"
        | "attribute_def"
        | "attribute_usage"
        | "item_def"
        | "item_usage"
        | "port_def"
        | "port_usage"
        | "connection_def"
        | "connection_usage"
        | "interface_def"
        | "interface_usage"
        | "action_def"
        | "action_usage"
        | "state_def"
        | "state_usage"
        | "requirement_def"
        | "requirement_usage"
        | "specializes"
        | "subsets"
        | "redefines"
        | "multiplicity"
        | "default_value"
        | "quoted_name"
        | "short_name"
        | "advanced_modifier"
        | "metadata_annotation"
        | "metadata_def"
        | "metadata_usage"
        | "metadata_usage_about"
        | "language_extension_keyword"
        | "analysis_def"
        | "analysis_usage"
        | "verification"
        | "use_case"
        | "view_def"
        | "view_usage"
        | "concern_def"
        | "stakeholder"
        | "transition" => 1,
        "flow" | "succession" | "allocation" => 2,
        "perform" | "action_body" => 3,
        _ => 4,
    }
}

fn contains_short_name(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        token.starts_with('<')
            && token.ends_with('>')
            && token.len() > 2
            && !token.contains('=')
            && !token.contains("::")
    })
}

fn collect_sysml_files(
    root: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_sysml_files(&path, files)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("sysml") {
            files.push(path);
        }
    }
    Ok(())
}

struct ConstructPattern {
    tag: &'static str,
    needles: &'static [&'static str],
}

const CONSTRUCT_PATTERNS: &[ConstructPattern] = &[
    ConstructPattern {
        tag: "package",
        needles: &["package "],
    },
    ConstructPattern {
        tag: "import",
        needles: &["import "],
    },
    ConstructPattern {
        tag: "part_def",
        needles: &["part def "],
    },
    ConstructPattern {
        tag: "part_usage",
        needles: &["\n\tpart ", "\n    part ", "\npart "],
    },
    ConstructPattern {
        tag: "attribute_def",
        needles: &["attribute def "],
    },
    ConstructPattern {
        tag: "attribute_usage",
        needles: &["\n\tattribute ", "\n    attribute ", "\nattribute "],
    },
    ConstructPattern {
        tag: "item_def",
        needles: &["item def "],
    },
    ConstructPattern {
        tag: "item_usage",
        needles: &["\n\titem ", "\n    item ", "\nitem "],
    },
    ConstructPattern {
        tag: "port_def",
        needles: &["port def "],
    },
    ConstructPattern {
        tag: "port_usage",
        needles: &["\n\tport ", "\n    port ", "\nport "],
    },
    ConstructPattern {
        tag: "connection_def",
        needles: &["connection def "],
    },
    ConstructPattern {
        tag: "connection_usage",
        needles: &["\n\tconnection ", "\n    connection ", "\nconnection "],
    },
    ConstructPattern {
        tag: "interface_def",
        needles: &["interface def "],
    },
    ConstructPattern {
        tag: "interface_usage",
        needles: &["\n\tinterface ", "\n    interface ", "\ninterface "],
    },
    ConstructPattern {
        tag: "action_def",
        needles: &["action def "],
    },
    ConstructPattern {
        tag: "action_usage",
        needles: &["\n\taction ", "\n    action ", "\naction "],
    },
    ConstructPattern {
        tag: "perform",
        needles: &["perform "],
    },
    ConstructPattern {
        tag: "state_def",
        needles: &["state def "],
    },
    ConstructPattern {
        tag: "state_usage",
        needles: &["\n\tstate ", "\n    state ", "\nstate "],
    },
    ConstructPattern {
        tag: "transition",
        needles: &["transition "],
    },
    ConstructPattern {
        tag: "requirement_def",
        needles: &["requirement def "],
    },
    ConstructPattern {
        tag: "requirement_usage",
        needles: &["\n\trequirement", "\n    requirement", "\nrequirement"],
    },
    ConstructPattern {
        tag: "constraint_def",
        needles: &["constraint def "],
    },
    ConstructPattern {
        tag: "constraint_usage",
        needles: &["constraint "],
    },
    ConstructPattern {
        tag: "analysis_def",
        needles: &["analysis def "],
    },
    ConstructPattern {
        tag: "analysis_usage",
        needles: &["\n\tanalysis ", "\n    analysis ", "\nanalysis "],
    },
    ConstructPattern {
        tag: "use_case",
        needles: &["use case "],
    },
    ConstructPattern {
        tag: "verification",
        needles: &["verification "],
    },
    ConstructPattern {
        tag: "view_def",
        needles: &["view def "],
    },
    ConstructPattern {
        tag: "view_usage",
        needles: &["\n\tview ", "\n    view ", "\nview "],
    },
    ConstructPattern {
        tag: "concern_def",
        needles: &["concern def "],
    },
    ConstructPattern {
        tag: "stakeholder",
        needles: &["stakeholder "],
    },
    ConstructPattern {
        tag: "individual",
        needles: &["individual "],
    },
    ConstructPattern {
        tag: "occurrence",
        needles: &["occurrence "],
    },
    ConstructPattern {
        tag: "flow",
        needles: &["flow "],
    },
    ConstructPattern {
        tag: "succession",
        needles: &["succession "],
    },
    ConstructPattern {
        tag: "allocation",
        needles: &["allocation "],
    },
    ConstructPattern {
        tag: "metadata_def",
        needles: &["metadata def "],
    },
    ConstructPattern {
        tag: "metadata_usage",
        needles: &["\n\tmetadata ", "\n    metadata ", "\nmetadata "],
    },
    ConstructPattern {
        tag: "specializes",
        needles: &[":>", "specializes "],
    },
    ConstructPattern {
        tag: "subsets",
        needles: &["subsets "],
    },
    ConstructPattern {
        tag: "redefines",
        needles: &[":>>", "redefines "],
    },
    ConstructPattern {
        tag: "multiplicity",
        needles: &["[0", "[1", "[2", "[3", "[4", "[5", "[6", "[*", "[n"],
    },
    ConstructPattern {
        tag: "default_value",
        needles: &[" = ", " := ", " default = "],
    },
];

const CURRENTLY_SUPPORTED_TAGS: &[&str] = &[
    "package",
    "import",
    "part_def",
    "part_usage",
    "attribute_def",
    "attribute_usage",
    "item_def",
    "item_usage",
    "port_def",
    "port_usage",
    "connection_def",
    "connection_usage",
    "interface_def",
    "interface_usage",
    "action_def",
    "action_usage",
    "state_def",
    "state_usage",
    "requirement_def",
    "requirement_usage",
    "constraint_def",
    "constraint_usage",
    "occurrence",
    "specializes",
    "subsets",
    "redefines",
    "multiplicity",
    "default_value",
    "quoted_name",
    "advanced_modifier",
    "short_name",
    "flow",
    "succession",
    "allocation",
    "perform",
    "action_body",
    "constraint_body",
    "individual",
    "metadata_annotation",
    "metadata_def",
    "metadata_usage",
    "metadata_usage_about",
    "language_extension_keyword",
    "analysis_def",
    "analysis_usage",
    "verification",
    "use_case",
    "view_def",
    "view_usage",
    "concern_def",
    "stakeholder",
    "transition",
];
