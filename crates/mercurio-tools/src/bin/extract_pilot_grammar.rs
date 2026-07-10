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
                "grammar extract drift: {} does not match fresh extraction from {}",
                args.out.display(),
                args.pilot_root.display()
            )
            .into());
        }
        println!("grammar extract is current: {}", args.out.display());
        return Ok(());
    }

    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, format!("{serialized}\n"))?;
    println!("wrote grammar extract: {}", args.out.display());
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
                .join("grammar.extract.json")
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
        "Usage: cargo run -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_grammar -- [--pilot-root PATH] [--profile-id ID] [--out PATH] [--check] [--allow-dirty]"
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
struct GrammarExtract {
    schema: &'static str,
    source: ExtractSource,
    constructs: Vec<ConstructExtract>,
    keyword_registry: KeywordRegistryExtract,
    rule_call_graph: Vec<RuleCallGraphEntry>,
    enum_rules: Vec<EnumRuleExtract>,
    rules: Vec<GrammarRuleSummary>,
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

#[derive(Clone, Serialize)]
struct ConstructExtract {
    construct: String,
    metaclass: String,
    grammar: String,
    source_file: String,
    line: usize,
    fragment: bool,
    keywords: Vec<String>,
}

#[derive(Serialize)]
struct KeywordRegistryExtract {
    definitions: BTreeMap<String, Vec<String>>,
    usages: BTreeMap<String, Vec<String>>,
    all: BTreeMap<String, Vec<String>>,
}

#[derive(Serialize)]
struct RuleCallGraphEntry {
    rule: String,
    metaclass: Option<String>,
    source_file: String,
    line: usize,
    calls: Vec<String>,
}

#[derive(Serialize)]
struct EnumRuleExtract {
    rule: String,
    metaclass: Option<String>,
    source_file: String,
    line: usize,
    literals: Vec<String>,
}

#[derive(Serialize)]
struct GrammarRuleSummary {
    rule: String,
    grammar: String,
    source_file: String,
    line: usize,
    fragment: bool,
    enum_rule: bool,
    metaclass: Option<String>,
    keywords: Vec<String>,
}

#[derive(Clone, Debug)]
struct ParsedGrammarFile {
    grammar: String,
    relative_path: String,
    rules: Vec<ParsedRule>,
}

#[derive(Clone, Debug)]
struct ParsedRule {
    name: String,
    metaclass: Option<String>,
    line: usize,
    fragment: bool,
    enum_rule: bool,
    body: String,
    keywords: Vec<String>,
}

fn build_extract(args: &Args) -> Result<GrammarExtract, Box<dyn std::error::Error>> {
    let grammar_files = read_grammar_files(&args.pilot_root)?;
    let rule_names = grammar_files
        .iter()
        .flat_map(|file| file.rules.iter().map(|rule| rule.name.clone()))
        .collect::<BTreeSet<_>>();
    let rule_index = grammar_files
        .iter()
        .flat_map(|file| {
            file.rules
                .iter()
                .map(|rule| (rule.name.clone(), rule.clone()))
        })
        .collect::<BTreeMap<_, _>>();

    let mut constructs = Vec::new();
    let mut call_graph = Vec::new();
    let mut enum_rules = Vec::new();
    let mut summaries = Vec::new();
    let mut all_keywords = BTreeMap::<String, BTreeSet<String>>::new();
    let mut definition_keywords = BTreeMap::<String, BTreeSet<String>>::new();
    let mut usage_keywords = BTreeMap::<String, BTreeSet<String>>::new();

    for file in &grammar_files {
        for rule in &file.rules {
            let calls = rule_calls(&rule.body, &rule_names, &rule.name);
            let effective_keywords = effective_keywords(&rule.name, &rule_index, 0);
            if let Some(metaclass) = &rule.metaclass {
                constructs.push(ConstructExtract {
                    construct: rule.name.clone(),
                    metaclass: metaclass.clone(),
                    grammar: file.grammar.clone(),
                    source_file: file.relative_path.clone(),
                    line: rule.line,
                    fragment: rule.fragment,
                    keywords: effective_keywords.clone(),
                });
            }
            for keyword in &effective_keywords {
                all_keywords
                    .entry(keyword.clone())
                    .or_default()
                    .insert(rule.name.clone());
                if is_definition_construct(&rule.name, rule.metaclass.as_deref()) {
                    definition_keywords
                        .entry(keyword.clone())
                        .or_default()
                        .insert(rule.name.clone());
                }
                if is_usage_construct(&rule.name, rule.metaclass.as_deref()) {
                    usage_keywords
                        .entry(keyword.clone())
                        .or_default()
                        .insert(rule.name.clone());
                }
            }
            call_graph.push(RuleCallGraphEntry {
                rule: rule.name.clone(),
                metaclass: rule.metaclass.clone(),
                source_file: file.relative_path.clone(),
                line: rule.line,
                calls,
            });
            if rule.enum_rule {
                enum_rules.push(EnumRuleExtract {
                    rule: rule.name.clone(),
                    metaclass: rule.metaclass.clone(),
                    source_file: file.relative_path.clone(),
                    line: rule.line,
                    literals: rule.keywords.clone(),
                });
            }
            summaries.push(GrammarRuleSummary {
                rule: rule.name.clone(),
                grammar: file.grammar.clone(),
                source_file: file.relative_path.clone(),
                line: rule.line,
                fragment: rule.fragment,
                enum_rule: rule.enum_rule,
                metaclass: rule.metaclass.clone(),
                keywords: rule.keywords.clone(),
            });
        }
    }

    constructs.sort_by(|left, right| {
        left.construct
            .cmp(&right.construct)
            .then_with(|| left.metaclass.cmp(&right.metaclass))
            .then_with(|| left.source_file.cmp(&right.source_file))
    });
    call_graph.sort_by(|left, right| left.rule.cmp(&right.rule));
    enum_rules.sort_by(|left, right| left.rule.cmp(&right.rule));
    summaries.sort_by(|left, right| left.rule.cmp(&right.rule));

    Ok(GrammarExtract {
        schema: "dev.mercurio.pilot-grammar-extract.v1",
        source: ExtractSource {
            schema: "dev.mercurio.pilot-artifact-source.v1",
            profile_id: args.profile_id.clone(),
            pilot: pilot_source(&args.pilot_root, args.allow_dirty),
            source_files: grammar_source_files(&args.pilot_root)?,
            extractor: ExtractorSource {
                name: "extract_pilot_grammar",
                version: env!("CARGO_PKG_VERSION"),
            },
            extracted_at_utc: now_utc_rfc3339()?,
        },
        constructs,
        keyword_registry: KeywordRegistryExtract {
            definitions: set_map_to_vec_map(definition_keywords),
            usages: set_map_to_vec_map(usage_keywords),
            all: set_map_to_vec_map(all_keywords),
        },
        rule_call_graph: call_graph,
        enum_rules,
        rules: summaries,
    })
}

fn read_grammar_files(
    pilot_root: &Path,
) -> Result<Vec<ParsedGrammarFile>, Box<dyn std::error::Error>> {
    grammar_source_paths()
        .into_iter()
        .map(|relative| {
            let text = std::fs::read_to_string(pilot_root.join(path_from_slashes(relative)))?;
            let grammar = grammar_name(&text).unwrap_or_else(|| relative.to_string());
            Ok(ParsedGrammarFile {
                grammar,
                relative_path: relative.to_string(),
                rules: parse_xtext_rules(&text),
            })
        })
        .collect()
}

fn grammar_source_files(pilot_root: &Path) -> Result<Vec<SourceFile>, Box<dyn std::error::Error>> {
    grammar_source_paths()
        .into_iter()
        .map(|relative| {
            Ok(SourceFile {
                path: relative.to_string(),
                sha256: sha256_file(&pilot_root.join(path_from_slashes(relative)))?,
            })
        })
        .collect()
}

fn grammar_source_paths() -> Vec<&'static str> {
    vec![
        "org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext",
        "org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerML.xtext",
        "org.omg.kerml.expressions.xtext/src/org/omg/kerml/expressions/xtext/KerMLExpressions.xtext",
    ]
}

fn parse_xtext_rules(text: &str) -> Vec<ParsedRule> {
    let stripped = strip_comments_preserving_lines(text);
    let lines = stripped.lines().collect::<Vec<_>>();
    let mut rules = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let Some(header) = parse_rule_header(line) else {
            index += 1;
            continue;
        };

        let start_line = index + 1;
        let mut body = String::new();
        if let Some(after_colon) = rule_colon_index(line).map(|colon| &line[colon + 1..]) {
            body.push_str(after_colon);
            body.push('\n');
        }
        while !has_unquoted_semicolon(&body) && index + 1 < lines.len() {
            index += 1;
            body.push_str(lines[index]);
            body.push('\n');
        }
        body = trim_after_rule_semicolon(&body);
        let keywords = keyword_literals(&body);
        rules.push(ParsedRule {
            name: header.name,
            metaclass: header.metaclass,
            line: start_line,
            fragment: header.fragment,
            enum_rule: header.enum_rule,
            body,
            keywords,
        });
        index += 1;
    }
    rules
}

#[derive(Debug)]
struct RuleHeader {
    name: String,
    metaclass: Option<String>,
    fragment: bool,
    enum_rule: bool,
}

fn parse_rule_header(line: &str) -> Option<RuleHeader> {
    let before_colon = line.get(..rule_colon_index(line)?)?.trim();
    if before_colon.is_empty()
        || before_colon.starts_with("grammar ")
        || before_colon.starts_with("import ")
    {
        return None;
    }
    let mut rest = before_colon;
    let mut fragment = false;
    let mut enum_rule = false;
    if let Some(value) = rest.strip_prefix("fragment ") {
        fragment = true;
        rest = value.trim_start();
    }
    if let Some(value) = rest.strip_prefix("enum ") {
        enum_rule = true;
        rest = value.trim_start();
    }
    let name = read_identifier(rest)?;
    let metaclass = rest
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|window| (window[0] == "returns").then(|| clean_type(window[1])));
    Some(RuleHeader {
        name,
        metaclass,
        fragment,
        enum_rule,
    })
}

fn rule_colon_index(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    for index in 0..bytes.len() {
        if bytes[index] != b':' {
            continue;
        }
        let previous_is_colon = index > 0 && bytes[index - 1] == b':';
        let next_is_colon = index + 1 < bytes.len() && bytes[index + 1] == b':';
        if !previous_is_colon && !next_is_colon {
            return Some(index);
        }
    }
    None
}

fn read_identifier(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let first = chars.next()?;
    if !is_identifier_start(first) {
        return None;
    }
    let mut value = String::from(first);
    for ch in chars {
        if is_identifier_continue(ch) {
            value.push(ch);
        } else {
            break;
        }
    }
    Some(value)
}

fn clean_type(value: &str) -> String {
    value
        .trim_matches(|ch: char| ch == ':' || ch == ';' || ch == ',')
        .to_string()
}

fn strip_comments_preserving_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(ch) = chars.next() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }
        if in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                out.push(' ');
                out.push(' ');
                in_block_comment = false;
            } else if ch == '\n' {
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }
        if in_string {
            out.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            } else if ch == '\'' {
                in_string = false;
            }
            continue;
        }

        if ch == '\'' {
            in_string = true;
            out.push(ch);
        } else if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            out.push(' ');
            out.push(' ');
            in_line_comment = true;
        } else if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            out.push(' ');
            out.push(' ');
            in_block_comment = true;
        } else {
            out.push(ch);
        }
    }

    out
}

fn has_unquoted_semicolon(text: &str) -> bool {
    let mut in_string = false;
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if in_string {
            if ch == '\\' {
                chars.next();
            } else if ch == '\'' {
                in_string = false;
            }
            continue;
        }
        if ch == '\'' {
            in_string = true;
        } else if ch == ';' {
            return true;
        }
    }
    false
}

fn trim_after_rule_semicolon(text: &str) -> String {
    let mut in_string = false;
    let mut result = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if in_string {
            result.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            } else if ch == '\'' {
                in_string = false;
            }
            continue;
        }
        if ch == '\'' {
            in_string = true;
            result.push(ch);
        } else if ch == ';' {
            break;
        } else {
            result.push(ch);
        }
    }
    result
}

fn keyword_literals(body: &str) -> Vec<String> {
    let mut values = BTreeSet::new();
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch != '\'' {
            continue;
        }
        let mut value = String::new();
        while let Some(inner) = chars.next() {
            if inner == '\\' {
                if let Some(escaped) = chars.next() {
                    value.push(escaped);
                }
            } else if inner == '\'' {
                break;
            } else {
                value.push(inner);
            }
        }
        if !value.is_empty() {
            values.insert(value);
        }
    }
    values.into_iter().collect()
}

fn rule_calls(body: &str, rule_names: &BTreeSet<String>, current_rule: &str) -> Vec<String> {
    identifier_tokens(body)
        .into_iter()
        .filter(|token| token != current_rule)
        .filter(|token| rule_names.contains(token))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn identifier_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\'' {
            while let Some(inner) = chars.next() {
                if inner == '\\' {
                    chars.next();
                } else if inner == '\'' {
                    break;
                }
            }
            continue;
        }
        if !is_identifier_start(ch) {
            continue;
        }
        let mut token = String::from(ch);
        while let Some(next) = chars.peek() {
            if is_identifier_continue(*next) {
                token.push(*next);
                chars.next();
            } else {
                break;
            }
        }
        tokens.push(token);
    }
    tokens
}

fn effective_keywords(
    rule_name: &str,
    rule_index: &BTreeMap<String, ParsedRule>,
    depth: usize,
) -> Vec<String> {
    if depth > 6 {
        return Vec::new();
    }
    let Some(rule) = rule_index.get(rule_name) else {
        return Vec::new();
    };
    let mut keywords = rule
        .keywords
        .iter()
        .filter(|keyword| is_named_keyword(keyword))
        .cloned()
        .collect::<BTreeSet<_>>();
    let rule_names = rule_index.keys().cloned().collect::<BTreeSet<_>>();
    for called in rule_calls(&rule.body, &rule_names, rule_name) {
        if should_follow_keyword_rule(&called) || depth == 0 {
            keywords.extend(effective_keywords(&called, rule_index, depth + 1));
        }
    }
    keywords.into_iter().collect()
}

fn should_follow_keyword_rule(rule: &str) -> bool {
    rule.contains("Keyword")
        || rule.ends_with("Prefix")
        || rule.ends_with("Declaration")
        || rule.ends_with("Usage")
        || rule.ends_with("Definition")
}

fn is_named_keyword(keyword: &str) -> bool {
    keyword
        .chars()
        .any(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn is_definition_construct(rule: &str, metaclass: Option<&str>) -> bool {
    rule.ends_with("Definition") || metaclass.is_some_and(|value| value.ends_with("Definition"))
}

fn is_usage_construct(rule: &str, metaclass: Option<&str>) -> bool {
    rule.ends_with("Usage") || metaclass.is_some_and(|value| value.ends_with("Usage"))
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn grammar_name(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("grammar "))
        .and_then(|rest| rest.split_whitespace().next())
        .map(str::to_string)
}

fn set_map_to_vec_map(input: BTreeMap<String, BTreeSet<String>>) -> BTreeMap<String, Vec<String>> {
    input
        .into_iter()
        .map(|(key, values)| (key, values.into_iter().collect()))
        .collect()
}

fn pilot_source(pilot_root: &Path, allow_dirty: bool) -> PilotSource {
    let dirty = git_dirty(pilot_root);
    PilotSource {
        repository: "SysML-v2-Pilot-Implementation".to_string(),
        commit: git_stdout(pilot_root, ["rev-parse", "HEAD"]),
        git_describe: git_stdout(pilot_root, ["describe", "--tags", "--always", "--dirty"]),
        dirty,
        dirty_waiver: (dirty == Some(true) && allow_dirty).then(|| {
            "Generated from an explicitly allowed dirty Pilot checkout; do not use for release without review."
                .to_string()
        }),
    }
}

fn validate_pilot_checkout(
    pilot_root: &Path,
    allow_dirty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(lock) = load_pilot_lock() {
        let Some(actual_commit) = git_stdout(pilot_root, ["rev-parse", "HEAD"]) else {
            return Err(format!(
                "could not read Pilot git commit from {}",
                pilot_root.display()
            )
            .into());
        };
        if actual_commit != lock.commit {
            return Err(format!(
                "Pilot checkout commit `{actual_commit}` does not match pinned commit `{}` from resources/pilot.lock.json",
                lock.commit
            )
            .into());
        }
    }
    if git_dirty(pilot_root) == Some(true) && !allow_dirty {
        return Err(format!(
            "Pilot checkout `{}` is dirty; pass --allow-dirty only for non-release/debug extraction",
            pilot_root.display()
        )
        .into());
    }
    Ok(())
}

fn git_stdout<const N: usize>(repo: &Path, args: [&str; N]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn git_dirty(repo: &Path) -> Option<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(!output.stdout.is_empty())
}

fn now_utc_rfc3339() -> Result<String, Box<dyn std::error::Error>> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

fn normalize_for_check(mut value: Value) -> Value {
    if let Some(source) = value.get_mut("source").and_then(Value::as_object_mut) {
        source.remove("extracted_at_utc");
    }
    value
}

fn path_from_slashes(path: &str) -> PathBuf {
    path.split('/').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_returning_rules_and_enum_rules() {
        let text = r#"
            grammar demo.Demo

            fragment Package returns SysML::Package :
                'package' Identification? ';'
            ;

            enum VisibilityIndicator returns SysML::VisibilityKind:
                PUBLIC = 'public' | PRIVATE = 'private'
            ;
        "#;

        let rules = parse_xtext_rules(text);

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name, "Package");
        assert_eq!(rules[0].metaclass.as_deref(), Some("SysML::Package"));
        assert!(rules[0].fragment);
        assert_eq!(rules[0].keywords, vec![";", "package"]);
        assert_eq!(rules[1].name, "VisibilityIndicator");
        assert!(rules[1].enum_rule);
        assert_eq!(rules[1].keywords, vec!["private", "public"]);
    }

    #[test]
    fn ignores_comments_and_quoted_semicolons_when_splitting_rules() {
        let text = r#"
            /* Ignored returns SysML::Ignored : */
            RuleA returns SysML::Element :
                // OtherRule returns SysML::Other :
                'literal;still-body' RuleB
            ;
            RuleB : 'b' ;
        "#;

        let rules = parse_xtext_rules(text);

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name, "RuleA");
        assert_eq!(rules[0].keywords, vec!["literal;still-body"]);
        assert_eq!(rules[1].name, "RuleB");
    }

    #[test]
    fn resolves_rule_calls_against_known_rules() {
        let names = BTreeSet::from(["RuleA".to_string(), "RuleB".to_string()]);

        let calls = rule_calls("owned += RuleB [SysML::Element|RuleA]", &names, "RuleA");

        assert_eq!(calls, vec!["RuleB"]);
    }
}
