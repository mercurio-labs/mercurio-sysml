use std::collections::BTreeMap;
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
                "metamodel extract drift: {} does not match fresh extraction from {}",
                args.out.display(),
                args.pilot_root.display()
            )
            .into());
        }
        println!("metamodel extract is current: {}", args.out.display());
        return Ok(());
    }

    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, format!("{serialized}\n"))?;
    println!("wrote metamodel extract: {}", args.out.display());
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
                .join("metamodel.extract.json")
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
        "Usage: cargo run -p mercurio-tools --features legacy-pilot-tools --bin extract_pilot_metamodel -- [--pilot-root PATH] [--profile-id ID] [--out PATH] [--check] [--allow-dirty]"
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
struct MetamodelExtract {
    schema: &'static str,
    source: ExtractSource,
    packages: Vec<PackageExtract>,
    metaclasses: Vec<MetaclassExtract>,
    generalizations: Vec<GeneralizationExtract>,
    structural_features: Vec<StructuralFeatureExtract>,
    containment_features: Vec<ContainmentFeatureExtract>,
    schema_cross_check: SchemaCrossCheck,
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
struct PackageExtract {
    name: String,
    display_name: String,
    ns_uri: Option<String>,
    ns_prefix: Option<String>,
    source_file: String,
    line: usize,
}

#[derive(Clone, Serialize)]
struct MetaclassExtract {
    qualified_name: String,
    package: String,
    name: String,
    abstract_class: bool,
    interface: bool,
    source_file: String,
    line: usize,
}

#[derive(Clone, Serialize)]
struct GeneralizationExtract {
    specific: String,
    general: String,
    source_file: String,
    line: usize,
}

#[derive(Clone, Serialize)]
struct StructuralFeatureExtract {
    owner: String,
    name: String,
    qualified_name: String,
    kind: String,
    target: Option<String>,
    lower_bound: i32,
    upper_bound: i32,
    containment: bool,
    derived: bool,
    transient: bool,
    volatile: bool,
    ordered: bool,
    unique: bool,
    id: bool,
    default_value: Option<String>,
    opposite: Option<String>,
    subsets: Vec<String>,
    redefines: Vec<String>,
    source_file: String,
    line: usize,
}

#[derive(Clone, Serialize)]
struct ContainmentFeatureExtract {
    owner: String,
    feature: String,
    target: Option<String>,
    upper_bound: i32,
    source_file: String,
    line: usize,
}

#[derive(Serialize)]
struct SchemaCrossCheck {
    source_files: Vec<String>,
}

#[derive(Clone, Debug)]
struct ParsedPackage {
    name: String,
    display_name: String,
    ns_uri: Option<String>,
    ns_prefix: Option<String>,
    source_file: String,
    line: usize,
    classes: Vec<ParsedClass>,
}

#[derive(Clone, Debug)]
struct ParsedClass {
    package: String,
    name: String,
    abstract_class: bool,
    interface: bool,
    supertypes: Vec<String>,
    source_file: String,
    line: usize,
    features: Vec<ParsedFeature>,
}

#[derive(Clone, Debug)]
struct ParsedFeature {
    name: String,
    kind: String,
    target: Option<String>,
    lower_bound: i32,
    upper_bound: i32,
    containment: bool,
    derived: bool,
    transient: bool,
    volatile: bool,
    ordered: bool,
    unique: bool,
    id: bool,
    default_value: Option<String>,
    opposite: Option<String>,
    subsets: Vec<String>,
    redefines: Vec<String>,
    source_file: String,
    line: usize,
}

#[derive(Clone, Debug)]
struct XmlTag {
    name: String,
    attrs: BTreeMap<String, String>,
    closing: bool,
    self_closing: bool,
    line: usize,
}

fn build_extract(args: &Args) -> Result<MetamodelExtract, Box<dyn std::error::Error>> {
    let source_paths = metamodel_source_paths(&args.pilot_root);
    let schema_paths = schema_source_paths(&args.pilot_root);
    let mut packages = Vec::new();

    for (relative_path, path) in &source_paths {
        let text = std::fs::read_to_string(path)?;
        packages.push(parse_ecore_package(&text, relative_path)?);
    }

    let mut source_files = Vec::new();
    for (relative_path, path) in source_paths.iter().chain(schema_paths.iter()) {
        source_files.push(SourceFile {
            path: relative_path.clone(),
            sha256: sha256_file(path)?,
        });
    }
    source_files.sort_by(|left, right| left.path.cmp(&right.path));

    let package_extracts = packages
        .iter()
        .map(|package| PackageExtract {
            name: package.name.clone(),
            display_name: package.display_name.clone(),
            ns_uri: package.ns_uri.clone(),
            ns_prefix: package.ns_prefix.clone(),
            source_file: package.source_file.clone(),
            line: package.line,
        })
        .collect::<Vec<_>>();

    let class_package_index = packages
        .iter()
        .flat_map(|package| {
            package
                .classes
                .iter()
                .map(|class| (class.name.clone(), package.display_name.clone()))
        })
        .collect::<BTreeMap<_, _>>();

    let mut metaclasses = Vec::new();
    let mut generalizations = Vec::new();
    let mut structural_features = Vec::new();
    let mut containment_features = Vec::new();

    for package in &packages {
        for class in &package.classes {
            let owner = qualified_name(&class.package, &class.name);
            metaclasses.push(MetaclassExtract {
                qualified_name: owner.clone(),
                package: class.package.clone(),
                name: class.name.clone(),
                abstract_class: class.abstract_class,
                interface: class.interface,
                source_file: class.source_file.clone(),
                line: class.line,
            });

            for supertype in &class.supertypes {
                generalizations.push(GeneralizationExtract {
                    specific: owner.clone(),
                    general: normalize_type_name(supertype, &class.package, &class_package_index),
                    source_file: class.source_file.clone(),
                    line: class.line,
                });
            }

            for feature in &class.features {
                let target = feature
                    .target
                    .as_deref()
                    .map(|value| normalize_type_name(value, &class.package, &class_package_index));
                let opposite = feature.opposite.as_deref().map(|value| {
                    normalize_feature_reference(value, &class.package, &class_package_index)
                });
                let subsets = feature
                    .subsets
                    .iter()
                    .map(|value| {
                        normalize_feature_reference(value, &class.package, &class_package_index)
                    })
                    .collect::<Vec<_>>();
                let redefines = feature
                    .redefines
                    .iter()
                    .map(|value| {
                        normalize_feature_reference(value, &class.package, &class_package_index)
                    })
                    .collect::<Vec<_>>();
                let feature_qualified_name = format!("{owner}::{}", feature.name);

                if feature.containment {
                    containment_features.push(ContainmentFeatureExtract {
                        owner: owner.clone(),
                        feature: feature.name.clone(),
                        target: target.clone(),
                        upper_bound: feature.upper_bound,
                        source_file: feature.source_file.clone(),
                        line: feature.line,
                    });
                }

                structural_features.push(StructuralFeatureExtract {
                    owner: owner.clone(),
                    name: feature.name.clone(),
                    qualified_name: feature_qualified_name,
                    kind: feature.kind.clone(),
                    target,
                    lower_bound: feature.lower_bound,
                    upper_bound: feature.upper_bound,
                    containment: feature.containment,
                    derived: feature.derived,
                    transient: feature.transient,
                    volatile: feature.volatile,
                    ordered: feature.ordered,
                    unique: feature.unique,
                    id: feature.id,
                    default_value: feature.default_value.clone(),
                    opposite,
                    subsets,
                    redefines,
                    source_file: feature.source_file.clone(),
                    line: feature.line,
                });
            }
        }
    }

    metaclasses.sort_by(|left, right| left.qualified_name.cmp(&right.qualified_name));
    generalizations.sort_by(|left, right| {
        left.specific
            .cmp(&right.specific)
            .then_with(|| left.general.cmp(&right.general))
    });
    structural_features.sort_by(|left, right| left.qualified_name.cmp(&right.qualified_name));
    containment_features.sort_by(|left, right| {
        left.owner
            .cmp(&right.owner)
            .then_with(|| left.feature.cmp(&right.feature))
    });

    Ok(MetamodelExtract {
        schema: "dev.mercurio.pilot-metamodel-extract.v1",
        source: ExtractSource {
            schema: "dev.mercurio.extract-source.v1",
            profile_id: args.profile_id.clone(),
            pilot: pilot_source(&args.pilot_root, args.allow_dirty),
            source_files,
            extractor: ExtractorSource {
                name: "extract_pilot_metamodel",
                version: env!("CARGO_PKG_VERSION"),
            },
            extracted_at_utc: now_utc_rfc3339()?,
        },
        packages: package_extracts,
        metaclasses,
        generalizations,
        structural_features,
        containment_features,
        schema_cross_check: SchemaCrossCheck {
            source_files: schema_paths
                .into_iter()
                .map(|(relative_path, _)| relative_path)
                .collect(),
        },
    })
}

fn metamodel_source_paths(pilot_root: &Path) -> Vec<(String, PathBuf)> {
    [
        "org.omg.sysml/model/kerml.ecore",
        "org.omg.sysml/model/SysML.ecore",
    ]
    .into_iter()
    .map(|relative| {
        (
            relative.to_string(),
            pilot_root.join(path_from_slashes(relative)),
        )
    })
    .collect()
}

fn schema_source_paths(pilot_root: &Path) -> Vec<(String, PathBuf)> {
    [
        "org.omg.sysml/json-schema/KerML.json",
        "org.omg.sysml/json-schema/SysML.json",
    ]
    .into_iter()
    .map(|relative| {
        (
            relative.to_string(),
            pilot_root.join(path_from_slashes(relative)),
        )
    })
    .collect()
}

fn parse_ecore_package(
    text: &str,
    source_file: &str,
) -> Result<ParsedPackage, Box<dyn std::error::Error>> {
    let mut package: Option<ParsedPackage> = None;
    let mut current_class: Option<ParsedClass> = None;
    let mut current_feature: Option<ParsedFeature> = None;
    let mut current_annotation: Option<String> = None;

    for tag in scan_xml_tags(text)? {
        if tag.closing {
            match tag.name.as_str() {
                "eClassifiers" => {
                    if let Some(class) = current_class.take() {
                        package
                            .as_mut()
                            .ok_or("encountered class before EPackage")?
                            .classes
                            .push(class);
                    }
                }
                "eStructuralFeatures" => {
                    if let Some(feature) = current_feature.take()
                        && let Some(class) = current_class.as_mut()
                    {
                        class.features.push(feature);
                    }
                }
                "eAnnotations" => current_annotation = None,
                _ => {}
            }
            continue;
        }

        match tag.name.as_str() {
            "ecore:EPackage" => {
                let name = required_attr(&tag, "name")?;
                package = Some(ParsedPackage {
                    display_name: display_package_name(&name),
                    name,
                    ns_uri: tag.attrs.get("nsURI").cloned(),
                    ns_prefix: tag.attrs.get("nsPrefix").cloned(),
                    source_file: source_file.to_string(),
                    line: tag.line,
                    classes: Vec::new(),
                });
            }
            "eClassifiers" => {
                if tag.attrs.get("xsi:type").map(String::as_str) != Some("ecore:EClass") {
                    continue;
                }
                if let Some(class) = current_class.take() {
                    package
                        .as_mut()
                        .ok_or("encountered class before EPackage")?
                        .classes
                        .push(class);
                }
                let package_name = package
                    .as_ref()
                    .ok_or("encountered class before EPackage")?
                    .display_name
                    .clone();
                let supertypes = tag
                    .attrs
                    .get("eSuperTypes")
                    .map(|value| value.split_whitespace().map(str::to_string).collect())
                    .unwrap_or_default();
                current_class = Some(ParsedClass {
                    package: package_name,
                    name: required_attr(&tag, "name")?,
                    abstract_class: bool_attr(&tag, "abstract", false),
                    interface: bool_attr(&tag, "interface", false),
                    supertypes,
                    source_file: source_file.to_string(),
                    line: tag.line,
                    features: Vec::new(),
                });
            }
            "eStructuralFeatures" => {
                if current_class.is_none() {
                    continue;
                }
                current_feature = Some(ParsedFeature {
                    name: required_attr(&tag, "name")?,
                    kind: feature_kind(&tag),
                    target: tag.attrs.get("eType").cloned(),
                    lower_bound: int_attr(&tag, "lowerBound", 0)?,
                    upper_bound: int_attr(&tag, "upperBound", 1)?,
                    containment: bool_attr(&tag, "containment", false),
                    derived: bool_attr(&tag, "derived", false),
                    transient: bool_attr(&tag, "transient", false),
                    volatile: bool_attr(&tag, "volatile", false),
                    ordered: bool_attr(&tag, "ordered", true),
                    unique: bool_attr(&tag, "unique", true),
                    id: bool_attr(&tag, "iD", false),
                    default_value: tag.attrs.get("defaultValueLiteral").cloned(),
                    opposite: tag.attrs.get("eOpposite").cloned(),
                    subsets: Vec::new(),
                    redefines: Vec::new(),
                    source_file: source_file.to_string(),
                    line: tag.line,
                });
                if tag.self_closing
                    && let Some(feature) = current_feature.take()
                    && let Some(class) = current_class.as_mut()
                {
                    class.features.push(feature);
                }
            }
            "eAnnotations" => {
                current_annotation = tag.attrs.get("source").cloned();
                if tag.self_closing {
                    if let (Some(source), Some(feature)) =
                        (current_annotation.as_deref(), current_feature.as_mut())
                    {
                        apply_annotation_references(source, &tag, feature);
                    }
                    current_annotation = None;
                }
            }
            "details" => {
                if let (Some(source), Some(feature)) =
                    (current_annotation.as_deref(), current_feature.as_mut())
                {
                    if tag.attrs.get("key").map(String::as_str) == Some("body") {
                        apply_annotation_body(source, &tag, feature);
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(class) = current_class.take() {
        package
            .as_mut()
            .ok_or("encountered class before EPackage")?
            .classes
            .push(class);
    }

    package.ok_or_else(|| format!("no EPackage found in {source_file}").into())
}

fn apply_annotation_references(source: &str, tag: &XmlTag, feature: &mut ParsedFeature) {
    let Some(references) = tag.attrs.get("references") else {
        return;
    };
    let values = references
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    match source {
        "subsets" => feature.subsets.extend(values),
        "redefines" => feature.redefines.extend(values),
        _ => {}
    }
}

fn apply_annotation_body(source: &str, tag: &XmlTag, feature: &mut ParsedFeature) {
    if source != "subsets" && source != "redefines" {
        return;
    }
    let Some(body) = tag.attrs.get("value") else {
        return;
    };
    let values = body
        .split_whitespace()
        .filter(|value| value.contains("#//"))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if source == "subsets" {
        feature.subsets.extend(values);
    } else {
        feature.redefines.extend(values);
    }
}

fn scan_xml_tags(text: &str) -> Result<Vec<XmlTag>, Box<dyn std::error::Error>> {
    let mut tags = Vec::new();
    let mut index = 0;
    while let Some(open_offset) = text[index..].find('<') {
        let open = index + open_offset;
        if text[open..].starts_with("<!--") {
            let end = text[open + 4..]
                .find("-->")
                .ok_or("unterminated XML comment")?
                + open
                + 7;
            index = end;
            continue;
        }
        if text[open..].starts_with("<?") {
            let end = text[open + 2..]
                .find("?>")
                .ok_or("unterminated XML processing instruction")?
                + open
                + 4;
            index = end;
            continue;
        }
        let close = find_tag_close(text, open + 1)?;
        let raw = &text[open + 1..close];
        let line = 1 + text[..open].bytes().filter(|byte| *byte == b'\n').count();
        if let Some(tag) = parse_tag(raw, line) {
            tags.push(tag);
        }
        index = close + 1;
    }
    Ok(tags)
}

fn find_tag_close(text: &str, mut index: usize) -> Result<usize, Box<dyn std::error::Error>> {
    let bytes = text.as_bytes();
    let mut quote = None;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        match quote {
            Some(current) if ch == current => quote = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == '>' => return Ok(index),
            None => {}
        }
        index += 1;
    }
    Err("unterminated XML tag".into())
}

fn parse_tag(raw: &str, line: usize) -> Option<XmlTag> {
    let mut body = raw.trim();
    if body.is_empty() || body.starts_with('!') {
        return None;
    }
    let closing = body.starts_with('/');
    if closing {
        body = body[1..].trim_start();
    }
    let self_closing = body.ends_with('/');
    if self_closing {
        body = body[..body.len() - 1].trim_end();
    }
    let (name, rest) = split_name(body)?;
    Some(XmlTag {
        name: name.to_string(),
        attrs: parse_attrs(rest),
        closing,
        self_closing,
        line,
    })
}

fn split_name(text: &str) -> Option<(&str, &str)> {
    let end = text
        .char_indices()
        .find_map(|(index, ch)| ch.is_whitespace().then_some(index))
        .unwrap_or(text.len());
    (end > 0).then_some((&text[..end], &text[end..]))
}

fn parse_attrs(text: &str) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        let key_start = index;
        while index < bytes.len() && !bytes[index].is_ascii_whitespace() && bytes[index] != b'=' {
            index += 1;
        }
        let key = text[key_start..index].trim();
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'=' {
            continue;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || (bytes[index] != b'"' && bytes[index] != b'\'') {
            continue;
        }
        let quote = bytes[index];
        index += 1;
        let value_start = index;
        while index < bytes.len() && bytes[index] != quote {
            index += 1;
        }
        if index <= bytes.len() {
            attrs.insert(key.to_string(), xml_unescape(&text[value_start..index]));
        }
        index += 1;
    }
    attrs
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&#xA;", "\n")
        .replace("&#x9;", "\t")
}

fn required_attr(tag: &XmlTag, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    tag.attrs
        .get(name)
        .cloned()
        .ok_or_else(|| format!("missing `{name}` on <{}> at line {}", tag.name, tag.line).into())
}

fn bool_attr(tag: &XmlTag, name: &str, default: bool) -> bool {
    tag.attrs
        .get(name)
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(default)
}

fn int_attr(tag: &XmlTag, name: &str, default: i32) -> Result<i32, Box<dyn std::error::Error>> {
    tag.attrs
        .get(name)
        .map(|value| value.parse::<i32>())
        .transpose()?
        .map_or(Ok(default), Ok)
}

fn feature_kind(tag: &XmlTag) -> String {
    match tag.attrs.get("xsi:type").map(String::as_str) {
        Some("ecore:EReference") => "reference".to_string(),
        Some("ecore:EAttribute") => "attribute".to_string(),
        Some(other) => other.to_string(),
        None => "feature".to_string(),
    }
}

fn display_package_name(name: &str) -> String {
    if name.eq_ignore_ascii_case("sysml") {
        "SysML".to_string()
    } else if name.eq_ignore_ascii_case("kerml") {
        "KerML".to_string()
    } else {
        name.to_string()
    }
}

fn qualified_name(package: &str, name: &str) -> String {
    format!("{package}::{name}")
}

fn normalize_type_name(
    value: &str,
    current_package: &str,
    class_package_index: &BTreeMap<String, String>,
) -> String {
    if let Some(name) = value.rsplit("#//").next().filter(|name| !name.is_empty()) {
        let package = class_package_index
            .get(name)
            .map(String::as_str)
            .unwrap_or(current_package);
        return qualified_name(package, name);
    }
    if let Some(name) = value.rsplit('/').next().filter(|name| !name.is_empty()) {
        return name.to_string();
    }
    value.to_string()
}

fn normalize_feature_reference(
    value: &str,
    current_package: &str,
    class_package_index: &BTreeMap<String, String>,
) -> String {
    let Some(fragment) = value.rsplit("#//").next().filter(|value| !value.is_empty()) else {
        return value.to_string();
    };
    let mut parts = fragment.split('/');
    let Some(class_name) = parts.next().filter(|value| !value.is_empty()) else {
        return value.to_string();
    };
    let Some(feature_name) = parts.next().filter(|value| !value.is_empty()) else {
        return normalize_type_name(value, current_package, class_package_index);
    };
    let package = class_package_index
        .get(class_name)
        .map(String::as_str)
        .unwrap_or(current_package);
    format!("{}::{feature_name}", qualified_name(package, class_name))
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
    fn parses_classes_generalizations_features_and_annotations() {
        let text = r##"
            <?xml version="1.0" encoding="UTF-8"?>
            <ecore:EPackage name="sysml" nsURI="uri" nsPrefix="sysml">
              <eClassifiers xsi:type="ecore:EClass" name="RequirementUsage" eSuperTypes="#//ConstraintUsage">
                <eStructuralFeatures xsi:type="ecore:EReference" name="subjectParameter"
                    lowerBound="1" upperBound="-1" eType="#//PartUsage" containment="true"
                    volatile="true" transient="true" derived="true" eOpposite="#//PartUsage/requirement">
                  <eAnnotations source="subsets" references="#//Usage/nestedPart"/>
                  <eAnnotations source="redefines">
                    <details key="body" value="#//Usage/nestedUsage"/>
                  </eAnnotations>
                </eStructuralFeatures>
              </eClassifiers>
            </ecore:EPackage>
        "##;

        let package = parse_ecore_package(text, "SysML.ecore").unwrap();

        assert_eq!(package.display_name, "SysML");
        assert_eq!(package.classes.len(), 1);
        let class = &package.classes[0];
        assert_eq!(class.name, "RequirementUsage");
        assert_eq!(class.supertypes, vec!["#//ConstraintUsage"]);
        assert_eq!(class.features.len(), 1);
        let feature = &class.features[0];
        assert_eq!(feature.name, "subjectParameter");
        assert_eq!(feature.kind, "reference");
        assert_eq!(feature.lower_bound, 1);
        assert_eq!(feature.upper_bound, -1);
        assert!(feature.containment);
        assert!(feature.derived);
        assert_eq!(feature.subsets, vec!["#//Usage/nestedPart"]);
        assert_eq!(feature.redefines, vec!["#//Usage/nestedUsage"]);
    }

    #[test]
    fn normalizes_local_types_and_feature_references() {
        let index = BTreeMap::from([
            ("RequirementUsage".to_string(), "SysML".to_string()),
            ("ConstraintUsage".to_string(), "SysML".to_string()),
            ("Element".to_string(), "KerML".to_string()),
        ]);

        assert_eq!(
            normalize_type_name("#//ConstraintUsage", "SysML", &index),
            "SysML::ConstraintUsage"
        );
        assert_eq!(
            normalize_type_name(
                "../../org.omg.sysml/model/kerml.ecore#//Element",
                "SysML",
                &index
            ),
            "KerML::Element"
        );
        assert_eq!(
            normalize_feature_reference("#//RequirementUsage/subjectParameter", "SysML", &index),
            "SysML::RequirementUsage::subjectParameter"
        );
    }
}
