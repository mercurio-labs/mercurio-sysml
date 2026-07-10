use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use mercurio_tools::{default_pilot_root, load_pilot_lock, sha256_file};
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const DEFAULT_PROFILE_ID: &str = "sysml-2.0-pilot-2026-04";

const VALIDATOR_FILES: &[(&str, &str)] = &[
    (
        "KerMLValidator",
        "org.omg.kerml.xtext/src/org/omg/kerml/xtext/validation/KerMLValidator.xtend",
    ),
    (
        "SysMLValidator",
        "org.omg.sysml.xtext/src/org/omg/sysml/xtext/validation/SysMLValidator.xtend",
    ),
    (
        "KerMLExpressionsValidator",
        "org.omg.kerml.expressions.xtext/src/org/omg/kerml/expressions/xtext/validation/KerMLExpressionsValidator.xtend",
    ),
];

const HELPER_CALLS: &[&str] = &[
    "checkAllTypes",
    "checkOneType",
    "checkReferenceType",
    "checkAtMostOne",
    "checkAtMostOneRelationship",
    "checkAllRedefinitions",
    "checkAllSubsettings",
    "checkTypeSpecialization",
    "checkOccurrenceUsagePortion",
    "checkStateSubactions",
    "checkSubjectMembership",
    "checkFeatureTyping",
    "checkConnectionUsageEnds",
    "checkReferenceUsageType",
];

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
                "validator extract drift: {} does not match fresh extraction from {}",
                args.out.display(),
                args.pilot_root.display()
            )
            .into());
        }
        println!("validator extract is current: {}", args.out.display());
        return Ok(());
    }

    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, format!("{serialized}\n"))?;
    println!("wrote validator extract: {}", args.out.display());
    Ok(())
}

#[derive(Debug)]
struct Args {
    pilot_root: PathBuf,
    profile_id: String,
    out: PathBuf,
    check: bool,
    allow_dirty: bool,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut pilot_root = default_pilot_root();
        let mut profile_id = DEFAULT_PROFILE_ID.to_string();
        let mut out = None;
        let mut check = false;
        let mut allow_dirty = false;

        let raw = env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;
        while index < raw.len() {
            match raw[index].as_str() {
                "--pilot-root" => pilot_root = next_path(&raw, &mut index)?,
                "--profile-id" | "--profile" => profile_id = next_string(&raw, &mut index)?,
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
                .join("validators.extract.json")
        });

        Ok(Self {
            pilot_root,
            profile_id,
            out,
            check,
            allow_dirty,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_validators -- [--pilot-root PATH] [--profile-id ID] [--out PATH] [--check] [--allow-dirty]"
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
struct ValidatorsExtract {
    schema: &'static str,
    source: ExtractSource,
    summary: ValidatorSummary,
    validator_files: Vec<ValidatorFileExtract>,
    issue_constants: Vec<IssueConstantExtract>,
    checks: Vec<ValidatorCheckExtract>,
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
struct ValidatorSummary {
    validator_file_count: usize,
    issue_constant_count: usize,
    check_count: usize,
    diagnostic_use_count: usize,
    checks_by_validator: BTreeMap<String, usize>,
    checks_by_classification: BTreeMap<String, usize>,
    diagnostics_by_severity: BTreeMap<String, usize>,
    ledger_status_counts: BTreeMap<String, usize>,
}

#[derive(Serialize)]
struct ValidatorFileExtract {
    validator: String,
    source_file: String,
    line_count: usize,
    issue_constant_count: usize,
    check_count: usize,
    diagnostic_use_count: usize,
}

#[derive(Clone, Serialize)]
struct IssueConstantExtract {
    name: String,
    value: Option<String>,
    value_expression: String,
    message_constant: bool,
    source_file: String,
    line: usize,
}

#[derive(Serialize)]
struct ValidatorCheckExtract {
    id: String,
    validator: String,
    method: String,
    parameter_type: Option<String>,
    parameter_name: Option<String>,
    annotations: Vec<String>,
    validation_comments: Vec<String>,
    helper_calls: Vec<String>,
    diagnostics: Vec<DiagnosticUseExtract>,
    classification: String,
    ledger_status: &'static str,
    source_file: String,
    line: usize,
}

#[derive(Serialize)]
struct DiagnosticUseExtract {
    severity: String,
    message_expr: String,
    message_constant: Option<String>,
    message: Option<String>,
    issue_code_expr: Option<String>,
    issue_code: Option<String>,
    issue_code_value: Option<String>,
    source_file: String,
    line: usize,
}

#[derive(Clone)]
struct ParsedCheck {
    method: String,
    parameter_type: Option<String>,
    parameter_name: Option<String>,
    annotations: Vec<String>,
    validation_comments: Vec<String>,
    helper_calls: Vec<String>,
    diagnostics: Vec<ParsedDiagnostic>,
    line: usize,
}

#[derive(Clone)]
struct ParsedDiagnostic {
    severity: String,
    message_expr: String,
    issue_code_expr: Option<String>,
    line: usize,
}

fn build_extract(args: &Args) -> Result<ValidatorsExtract, Box<dyn std::error::Error>> {
    let mut source_files = Vec::new();
    let mut validator_files = Vec::new();
    let mut issue_constants = Vec::new();
    let mut parsed_checks = Vec::new();

    for (validator, relative_path) in VALIDATOR_FILES {
        let path = args
            .pilot_root
            .join(relative_path.replace('/', std::path::MAIN_SEPARATOR_STR));
        let text = std::fs::read_to_string(&path)?;
        let source_file = source_path(&args.pilot_root, &path);
        source_files.push(SourceFile {
            path: source_file.clone(),
            sha256: sha256_file(&path)?,
        });

        let constants = parse_issue_constants(&text, &source_file);
        let checks = parse_checks(&text);
        let diagnostic_use_count = checks
            .iter()
            .map(|check| check.diagnostics.len())
            .sum::<usize>();
        validator_files.push(ValidatorFileExtract {
            validator: (*validator).to_string(),
            source_file: source_file.clone(),
            line_count: text.lines().count(),
            issue_constant_count: constants.len(),
            check_count: checks.len(),
            diagnostic_use_count,
        });
        issue_constants.extend(constants);
        parsed_checks.extend(
            checks
                .into_iter()
                .map(|check| ((*validator).to_string(), source_file.clone(), check)),
        );
    }

    let constants_by_name = issue_constants
        .iter()
        .map(|constant| (constant.name.clone(), constant.clone()))
        .collect::<BTreeMap<_, _>>();

    let checks = parsed_checks
        .into_iter()
        .map(|(validator, source_file, check)| {
            let diagnostics = check
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    let message_constant = constant_name_from_expr(&diagnostic.message_expr)
                        .filter(|name| name.contains("_MSG"));
                    let message = message_constant
                        .as_ref()
                        .and_then(|name| constants_by_name.get(name))
                        .and_then(|constant| constant.value.clone());
                    let issue_code = diagnostic
                        .issue_code_expr
                        .as_deref()
                        .and_then(constant_name_from_expr);
                    let issue_code_value = issue_code
                        .as_ref()
                        .and_then(|name| constants_by_name.get(name))
                        .and_then(|constant| constant.value.clone());
                    DiagnosticUseExtract {
                        severity: diagnostic.severity.clone(),
                        message_expr: diagnostic.message_expr.clone(),
                        message_constant,
                        message,
                        issue_code_expr: diagnostic.issue_code_expr.clone(),
                        issue_code,
                        issue_code_value,
                        source_file: source_file.clone(),
                        line: diagnostic.line,
                    }
                })
                .collect::<Vec<_>>();
            let classification = classify_check(&check, &diagnostics);
            ValidatorCheckExtract {
                id: format!("{validator}::{}", check.method),
                validator,
                method: check.method,
                parameter_type: check.parameter_type,
                parameter_name: check.parameter_name,
                annotations: check.annotations,
                validation_comments: check.validation_comments,
                helper_calls: check.helper_calls,
                diagnostics,
                classification,
                ledger_status: "pending",
                source_file,
                line: check.line,
            }
        })
        .collect::<Vec<_>>();

    Ok(ValidatorsExtract {
        schema: "https://mercurio.dev/schemas/pilot-validators-extract/v1",
        source: ExtractSource {
            schema: "https://mercurio.dev/schemas/source-provenance/v1",
            profile_id: args.profile_id.clone(),
            pilot: pilot_source(&args.pilot_root, args.allow_dirty),
            source_files,
            extractor: ExtractorSource {
                name: "extract_pilot_validators",
                version: env!("CARGO_PKG_VERSION"),
            },
            extracted_at_utc: OffsetDateTime::now_utc().format(&Rfc3339)?,
        },
        summary: summarize(&validator_files, &issue_constants, &checks),
        validator_files,
        issue_constants,
        checks,
    })
}

fn parse_issue_constants(text: &str, source_file: &str) -> Vec<IssueConstantExtract> {
    let mut constants = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if is_commented_line(line) {
            continue;
        }
        let trimmed = line.trim();
        if !trimmed.contains("static val ") {
            continue;
        }
        let Some(eq_index) = trimmed.find('=') else {
            continue;
        };
        let name = trimmed[..eq_index]
            .split_whitespace()
            .last()
            .unwrap_or_default()
            .trim()
            .to_string();
        if !name.starts_with("INVALID_") {
            continue;
        }
        let value_expression = trimmed[eq_index + 1..]
            .trim()
            .trim_end_matches(';')
            .trim()
            .to_string();
        constants.push(IssueConstantExtract {
            message_constant: name.contains("_MSG"),
            value: parse_string_literal(&value_expression),
            value_expression,
            name,
            source_file: source_file.to_string(),
            line: index + 1,
        });
    }
    constants
}

fn parse_checks(text: &str) -> Vec<ParsedCheck> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut checks = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let trimmed = lines[index].trim();
        if is_commented_line(lines[index]) || !trimmed.starts_with("@Check") {
            index += 1;
            continue;
        }

        let mut annotations = vec![trimmed.to_string()];
        let mut signature_index = index + 1;
        while signature_index < lines.len() {
            let candidate = lines[signature_index].trim();
            if is_commented_line(lines[signature_index]) || candidate.is_empty() {
                signature_index += 1;
                continue;
            }
            if candidate.starts_with('@') {
                annotations.push(candidate.to_string());
                signature_index += 1;
                continue;
            }
            if candidate.contains("def ") {
                break;
            }
            signature_index += 1;
        }
        if signature_index >= lines.len() {
            index += 1;
            continue;
        }

        let Some(signature) = parse_signature(lines[signature_index]) else {
            index = signature_index + 1;
            continue;
        };
        let (body, end_index) = collect_body(&lines, signature_index);
        checks.push(parse_check_body(signature, annotations, body));
        index = end_index.saturating_add(1);
    }
    checks
}

struct Signature {
    method: String,
    parameter_type: Option<String>,
    parameter_name: Option<String>,
    line: usize,
}

fn parse_signature(line: &str) -> Option<Signature> {
    let def_index = line.find("def ")?;
    let after_def = line[def_index + 4..].trim_start();
    let paren_index = after_def.find('(')?;
    let method = after_def[..paren_index].trim().to_string();
    let params = after_def[paren_index + 1..].split(')').next()?.trim();
    let (parameter_type, parameter_name) = parse_first_parameter(params);
    Some(Signature {
        method,
        parameter_type,
        parameter_name,
        line: 0,
    })
}

fn parse_first_parameter(params: &str) -> (Option<String>, Option<String>) {
    let Some(first) = params
        .split(',')
        .next()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    else {
        return (None, None);
    };
    let parts = first
        .split_whitespace()
        .filter(|part| *part != "final" && *part != "var" && *part != "val")
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return (None, parts.first().map(|part| (*part).to_string()));
    }
    (
        Some(parts[..parts.len() - 1].join(" ")),
        parts.last().map(|part| (*part).to_string()),
    )
}

fn collect_body<'a>(lines: &[&'a str], signature_index: usize) -> (Vec<(usize, &'a str)>, usize) {
    let mut body = Vec::new();
    let mut depth = 0i32;
    let mut started = false;
    let mut index = signature_index;
    while index < lines.len() {
        let line = lines[index];
        body.push((index + 1, line));
        let active = if is_commented_line(line) { "" } else { line };
        for ch in active.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    started = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if started && depth <= 0 {
            return (body, index);
        }
        index += 1;
    }
    (body, lines.len().saturating_sub(1))
}

fn parse_check_body(
    mut signature: Signature,
    annotations: Vec<String>,
    body: Vec<(usize, &str)>,
) -> ParsedCheck {
    if let Some((line, _)) = body.first() {
        signature.line = *line;
    }
    let mut validation_comments = Vec::new();
    let mut helper_calls = BTreeSet::new();
    let mut diagnostics = Vec::new();
    let mut index = 0;
    while index < body.len() {
        let (line_number, line) = body[index];
        let trimmed = line.trim();
        if let Some(comment) = validation_comment(trimmed) {
            validation_comments.push(comment);
        }
        if !is_commented_line(line) {
            for helper in HELPER_CALLS {
                if trimmed.contains(&format!("{helper}(")) {
                    helper_calls.insert((*helper).to_string());
                }
            }
            if let Some((diagnostic, next_index)) = parse_diagnostic_call(&body, index) {
                diagnostics.push(diagnostic);
                index = next_index + 1;
                continue;
            }
        }
        let _ = line_number;
        index += 1;
    }

    ParsedCheck {
        method: signature.method,
        parameter_type: signature.parameter_type,
        parameter_name: signature.parameter_name,
        annotations,
        validation_comments,
        helper_calls: helper_calls.into_iter().collect(),
        diagnostics,
        line: signature.line,
    }
}

fn validation_comment(trimmed: &str) -> Option<String> {
    let comment = trimmed.strip_prefix("//")?.trim();
    if comment.starts_with("validate") || comment.starts_with("Check validate") {
        Some(comment.to_string())
    } else {
        None
    }
}

fn parse_diagnostic_call(
    body: &[(usize, &str)],
    index: usize,
) -> Option<(ParsedDiagnostic, usize)> {
    let (line_number, line) = body[index];
    let (severity, start) = diagnostic_start(line)?;
    let mut call = line[start..].trim().to_string();
    let mut depth = paren_delta(&call);
    let mut next_index = index;
    while depth > 0 && next_index + 1 < body.len() {
        next_index += 1;
        let (_, next_line) = body[next_index];
        if is_commented_line(next_line) {
            continue;
        }
        call.push(' ');
        call.push_str(next_line.trim());
        depth += paren_delta(next_line);
    }

    let args_text = call
        .split_once('(')
        .and_then(|(_, rest)| rest.rsplit_once(')').map(|(args, _)| args))?;
    let args = split_args(args_text);
    if args.is_empty() {
        return None;
    }
    let issue_code_expr = args
        .iter()
        .rev()
        .find(|arg| constant_name_from_expr(arg).is_some_and(|name| name.starts_with("INVALID_")))
        .cloned();
    Some((
        ParsedDiagnostic {
            severity,
            message_expr: args[0].clone(),
            issue_code_expr,
            line: line_number,
        },
        next_index,
    ))
}

fn diagnostic_start(line: &str) -> Option<(String, usize)> {
    let error = line.find("error(");
    let warning = line.find("warning(");
    match (error, warning) {
        (Some(e), Some(w)) if e < w => Some(("error".to_string(), e)),
        (Some(_), Some(w)) => Some(("warning".to_string(), w)),
        (Some(e), None) => Some(("error".to_string(), e)),
        (None, Some(w)) => Some(("warning".to_string(), w)),
        (None, None) => None,
    }
}

fn paren_delta(text: &str) -> i32 {
    let mut delta = 0;
    let mut quote = None;
    let mut escaped = false;
    for ch in text.chars() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' => delta += 1,
            ')' => delta -= 1,
            _ => {}
        }
    }
    delta
}

fn split_args(args_text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    for ch in args_text.chars() {
        if let Some(q) = quote {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth -= 1;
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth -= 1;
                current.push(ch);
            }
            ',' if paren_depth == 0 && bracket_depth == 0 => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }
    args
}

fn classify_check(check: &ParsedCheck, diagnostics: &[DiagnosticUseExtract]) -> String {
    let mut text = String::new();
    if let Some(parameter_type) = &check.parameter_type {
        text.push_str(parameter_type);
        text.push(' ');
    }
    for comment in &check.validation_comments {
        text.push_str(comment);
        text.push(' ');
    }
    for helper in &check.helper_calls {
        text.push_str(helper);
        text.push(' ');
    }
    for diagnostic in diagnostics {
        if let Some(code_value) = &diagnostic.issue_code_value {
            text.push_str(code_value);
            text.push(' ');
        }
        if let Some(code) = &diagnostic.issue_code {
            text.push_str(code);
            text.push(' ');
        }
    }
    let lower = text.to_ascii_lowercase();
    if lower.contains("type") || lower.contains("typing") {
        "typing".to_string()
    } else if lower.contains("specialization") || lower.contains("specialize") {
        "specialization".to_string()
    } else if lower.contains("multiplicity") || lower.contains("only one") {
        "multiplicity".to_string()
    } else if lower.contains("membership")
        || lower.contains("owning")
        || lower.contains("owned")
        || lower.contains("contain")
    {
        "containment".to_string()
    } else if lower.contains("reference") || lower.contains("referent") {
        "reference".to_string()
    } else {
        "bespoke".to_string()
    }
}

fn summarize(
    validator_files: &[ValidatorFileExtract],
    issue_constants: &[IssueConstantExtract],
    checks: &[ValidatorCheckExtract],
) -> ValidatorSummary {
    let mut checks_by_validator = BTreeMap::new();
    let mut checks_by_classification = BTreeMap::new();
    let mut diagnostics_by_severity = BTreeMap::new();
    let mut ledger_status_counts = BTreeMap::new();

    for check in checks {
        *checks_by_validator
            .entry(check.validator.clone())
            .or_insert(0usize) += 1;
        *checks_by_classification
            .entry(check.classification.clone())
            .or_insert(0usize) += 1;
        *ledger_status_counts
            .entry(check.ledger_status.to_string())
            .or_insert(0usize) += 1;
        for diagnostic in &check.diagnostics {
            *diagnostics_by_severity
                .entry(diagnostic.severity.clone())
                .or_insert(0usize) += 1;
        }
    }

    ValidatorSummary {
        validator_file_count: validator_files.len(),
        issue_constant_count: issue_constants.len(),
        check_count: checks.len(),
        diagnostic_use_count: checks
            .iter()
            .map(|check| check.diagnostics.len())
            .sum::<usize>(),
        checks_by_validator,
        checks_by_classification,
        diagnostics_by_severity,
        ledger_status_counts,
    }
}

fn parse_string_literal(value_expression: &str) -> Option<String> {
    let mut chars = value_expression.trim().chars();
    let quote = chars.next().filter(|ch| *ch == '"' || *ch == '\'')?;
    let mut value = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            value.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == quote {
            return Some(value);
        } else {
            value.push(ch);
        }
    }
    None
}

fn constant_name_from_expr(expr: &str) -> Option<String> {
    let mut candidate = expr
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'))
        .filter(|part| !part.is_empty())
        .next_back()?
        .to_string();
    if let Some((_, tail)) = candidate.rsplit_once('.') {
        candidate = tail.to_string();
    }
    candidate.starts_with("INVALID_").then_some(candidate)
}

fn is_commented_line(line: &str) -> bool {
    line.trim_start().starts_with("//")
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

fn source_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_check_signature_comments_and_diagnostics() {
        let text = r#"
            public static val INVALID_THING = "validateThing"
            public static val INVALID_THING_MSG = "Thing is invalid"

            @Check
            def checkThing(Thing t) {
                // validateThing
                if (bad) {
                    error(INVALID_THING_MSG, t, null, INVALID_THING)
                }
            }
        "#;

        let constants = parse_issue_constants(text, "ThingValidator.xtend");
        let checks = parse_checks(text);

        assert_eq!(constants.len(), 2);
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].method, "checkThing");
        assert_eq!(checks[0].parameter_type.as_deref(), Some("Thing"));
        assert_eq!(checks[0].validation_comments, ["validateThing"]);
        assert_eq!(checks[0].diagnostics.len(), 1);
        assert_eq!(checks[0].diagnostics[0].severity, "error");
        assert_eq!(
            checks[0].diagnostics[0].issue_code_expr.as_deref(),
            Some("INVALID_THING")
        );
    }

    #[test]
    fn ignores_commented_out_checks() {
        let text = r#"
            // @Check
            // def checkGreeting(Greeting greeting) {
            //     warning('bad', greeting, null, INVALID_NAME)
            // }
        "#;

        assert!(parse_checks(text).is_empty());
        assert!(parse_issue_constants(text, "Commented.xtend").is_empty());
    }
}
