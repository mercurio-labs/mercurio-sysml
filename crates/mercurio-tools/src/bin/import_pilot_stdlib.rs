use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use mercurio_core::{
    Graph, KirDocument, PilotExportDocument, RulePack, default_stdlib_path, load_pilot_export,
    normalize_pilot_export, repo_root,
};
use mercurio_sysml::{
    promote_observed_library_defaults, reconcile_library_metafeatures_with_ecore,
    sysml_metamodel_adapter_from_graph,
};
use mercurio_tools::{load_pilot_lock, sha256_file, sysml_workspace_root};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let input_path = if let Some(pilot_root) = args.pilot_root.as_deref() {
        validate_pilot_checkout(pilot_root, args.allow_dirty)?;
        if args.from_export {
            args.input_path.clone()
        } else {
            export_from_pilot(pilot_root, &args.input_path, args.pilot_jar.as_deref())?
        }
    } else {
        args.input_path.clone()
    };

    let export = load_pilot_export(&input_path)?;
    let enriched_export = with_ecore_derived_opposites(&export)?;
    validate_ecore_export_relationships(&enriched_export)?;
    let mut kir = normalize_pilot_export(enriched_export)?;
    if export.metadata.as_ref().and_then(|metadata| metadata.get("observed_ecore_defaults_v1"))
        == Some(&Value::Bool(true)) {
        let promoted = promote_observed_library_defaults(&mut kir.elements)?;
        println!("  observed Ecore attributes: {promoted}");
    }
    let reconciled = reconcile_library_metafeatures_with_ecore(&mut kir.elements)?;
    println!("  Ecore attribute feature descriptors reconciled: {reconciled}");
    apply_ecore_membership_cardinality(&mut kir)?;
    kir.metadata = build_kir_metadata(&args, &input_path, &export)?;
    let rulepack = sysml_metamodel_adapter_from_graph(&Graph::from_document(kir.clone())?);
    kir.write_pretty_to_path(&args.output_path)?;
    write_rulepack(&rulepack, &args.rulepack_output_path)?;

    println!("Imported pilot stdlib export:");
    println!("  input: {}", input_path.display());
    println!("  output: {}", args.output_path.display());
    println!("  rulepack: {}", args.rulepack_output_path.display());
    println!("  elements: {}", kir.elements.len());
    println!("  adapter facts: {}", rulepack.facts.len());
    Ok(())
}

fn membership_ecore_path() -> PathBuf {
    sysml_workspace_root().join(
        "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-effective.extract.json",
    )
}

fn with_ecore_derived_opposites(export: &PilotExportDocument) -> Result<PilotExportDocument, Box<dyn std::error::Error>> {
    let extracted: Value = serde_json::from_str(&std::fs::read_to_string(membership_ecore_path())?)?;
    let features = extracted.get("features").and_then(Value::as_array)
        .ok_or("effective Ecore extract has no features")?;
    let mappings = [
        ("Element", "owner", "owner", "Element", "ownedElement", "owned_element"),
        ("Element", "owningMembership", "owning_membership", "OwningMembership", "ownedMemberElement", "owned_member_element"),
    ];
    let mut enriched = export.clone();
    let mut seen = export.relationships.iter().map(|edge|
        (edge.source.clone(), edge.relation.clone(), edge.target.clone())).collect::<BTreeSet<_>>();
    for (forward_owner, forward_name, forward_relation, reverse_owner, reverse_name, reverse_relation) in mappings {
        let find = |owner: &str, name: &str| features.iter().filter(|feature| {
            feature["owner"].as_str().is_some_and(|id| id.rsplit("//").next() == Some(owner))
                && feature["name"].as_str() == Some(name)
                && feature["kind"].as_str() == Some("reference")
        }).collect::<Vec<_>>();
        let forward = find(forward_owner, forward_name);
        let reverse = find(reverse_owner, reverse_name);
        if forward.len() != 1 || reverse.len() != 1
            || forward[0]["opposite"] != reverse[0]["id"]
            || reverse[0]["opposite"] != forward[0]["id"]
            || reverse[0]["derived"].as_bool() != Some(true) {
            return Err(format!("missing derived Ecore opposite: {forward_owner}.{forward_name} / {reverse_owner}.{reverse_name}").into());
        }
        for edge in export.relationships.iter().filter(|edge| edge.relation == forward_relation) {
            let key = (edge.target.clone(), reverse_relation.to_string(), edge.source.clone());
            if seen.insert(key) {
                let mut opposite = edge.clone();
                opposite.source = edge.target.clone();
                opposite.relation = reverse_relation.to_string();
                opposite.target = edge.source.clone();
                enriched.relationships.push(opposite);
            }
        }
    }
    Ok(enriched)
}

fn validate_ecore_export_relationships(export: &PilotExportDocument) -> Result<(), Box<dyn std::error::Error>> {
    let extracted: Value = serde_json::from_str(&std::fs::read_to_string(membership_ecore_path())?)?;
    let features = extracted.get("features").and_then(Value::as_array)
        .ok_or("effective Ecore extract has no features")?;
    let classes = extracted.get("classes").and_then(Value::as_array)
        .ok_or("effective Ecore extract has no classes")?;
    let class_parents = classes.iter().map(|class| {
        let name = class["name"].as_str().ok_or("Ecore class without name")?;
        let parents = class["super_types"].as_array().ok_or("Ecore class without super types")?
            .iter().map(|parent| parent.as_str()
                .and_then(|id| id.rsplit("//").next()).ok_or("invalid Ecore super type")
                .map(str::to_string)).collect::<Result<Vec<_>, _>>()?;
        Ok((name.to_string(), parents))
    }).collect::<Result<BTreeMap<String, Vec<String>>, Box<dyn std::error::Error>>>()?;
    fn ancestors(kind: &str, parents: &BTreeMap<String, Vec<String>>,
        cache: &mut BTreeMap<String, BTreeSet<String>>) -> Result<BTreeSet<String>, String> {
        if let Some(found) = cache.get(kind) { return Ok(found.clone()); }
        let direct = parents.get(kind).ok_or_else(|| format!("unknown Ecore class: {kind}"))?;
        let mut result = BTreeSet::from([kind.to_string()]);
        for parent in direct { result.extend(ancestors(parent, parents, cache)?); }
        cache.insert(kind.to_string(), result.clone());
        Ok(result)
    }
    let mut cache = BTreeMap::new();
    for kind in class_parents.keys() { ancestors(kind, &class_parents, &mut cache)?; }
    let contracts = [
        ("Element", "owner", "owner", 1, false, true),
        ("Element", "ownedElement", "owned_element", -1, true, true),
        ("Element", "owningMembership", "owning_membership", 1, false, true),
        ("OwningMembership", "ownedMemberElement", "owned_member_element", 1, false, true),
        ("Membership", "memberElement", "member_element", 1, false, true),
        ("Membership", "membershipOwningNamespace", "membership_owning_namespace", 1, false, true),
        ("Namespace", "ownedMember", "members", -1, true, true),
        ("Namespace", "ownedMembership", "owned_membership", -1, true, true),
        ("Type", "ownedFeature", "features", -1, true, true),
        ("Feature", "type", "type", -1, true, true),
        ("Feature", "featuringType", "featuring_type", -1, true, true),
        ("Feature", "chainingFeature", "chaining_feature", -1, true, false),
    ];
    let mut rules = BTreeMap::new();
    let mut relations_by_ecore_id = BTreeMap::new();
    let mut opposite_ids = Vec::new();
    for (owner, source_name, relation, upper, ordered, unique) in contracts {
        let matching = features.iter().filter(|feature| {
            feature.get("name").and_then(Value::as_str) == Some(source_name)
                && feature.get("owner").and_then(Value::as_str)
                    .is_some_and(|id| id.rsplit("//").next() == Some(owner))
                && feature.get("kind").and_then(Value::as_str) == Some("reference")
                && feature.get("upper_bound").and_then(Value::as_i64) == Some(upper)
                && feature.get("ordered").and_then(Value::as_bool) == Some(ordered)
                && feature.get("unique").and_then(Value::as_bool) == Some(unique)
        }).collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(format!("missing Ecore reference contract: {owner}.{source_name}").into());
        }
        let lower = matching[0]["lower_bound"].as_i64().ok_or("Ecore reference without lower bound")?;
        let target_kind = matching[0]["type"].as_str().and_then(|id| id.rsplit("//").next())
            .ok_or("Ecore reference without target class")?;
        let ecore_id = matching[0]["id"].as_str().ok_or("Ecore reference without ID")?;
        relations_by_ecore_id.insert(ecore_id, relation);
        if let Some(opposite) = matching[0]["opposite"].as_str() {
            opposite_ids.push((relation, opposite));
        }
        rules.insert(relation, (owner, target_kind, lower, upper, unique));
    }
    let source_kinds = export.elements.iter().map(|element|
        (element.qualified_name.as_str(), element.kind.as_str())).collect::<BTreeMap<_, _>>();
    if source_kinds.len() != export.elements.len() { return Err("duplicate Pilot export element".into()); }
    let mut seen = BTreeSet::new();
    let mut edges = BTreeSet::new();
    let mut counts = BTreeMap::<(&str, &str), i64>::new();
    for relationship in &export.relationships {
        if relationship.relation == "specializes" { continue; } // TypeUtil derivation, not one Ecore reference.
        let (owner, target_class, _, upper, unique) = rules.get(relationship.relation.as_str())
            .ok_or_else(|| format!("unmapped Pilot export relation: {}", relationship.relation))?;
        let kind = source_kinds.get(relationship.source.as_str())
            .ok_or_else(|| format!("missing Pilot export source: {}", relationship.source))?;
        if !cache.get(*kind).is_some_and(|ancestors| ancestors.contains(*owner)) {
            return Err(format!("{} cannot own Ecore reference {}", kind, relationship.relation).into());
        }
        let target_kind = source_kinds.get(relationship.target.as_str())
            .ok_or_else(|| format!("missing Pilot export target: {}", relationship.target))?;
        if !cache.get(*target_kind).is_some_and(|ancestors| ancestors.contains(*target_class)) {
            return Err(format!("{} cannot target Ecore reference {}", target_kind, relationship.relation).into());
        }
        let count = counts.entry((&relationship.source, &relationship.relation)).or_default();
        *count += 1;
        if *upper >= 0 && *count > *upper {
            return Err(format!("Ecore upper bound exceeded: {}.{}", relationship.source, relationship.relation).into());
        }
        if *unique && !seen.insert((&relationship.source, &relationship.relation, &relationship.target)) {
            return Err(format!("duplicate target in unique Ecore collection: {}.{} -> {}",
                relationship.source, relationship.relation, relationship.target).into());
        }
        edges.insert((relationship.source.as_str(), relationship.relation.as_str(), relationship.target.as_str()));
    }
    for (relation, opposite_id) in opposite_ids {
        let Some(opposite_relation) = relations_by_ecore_id.get(opposite_id) else { continue; };
        for &(source, observed_relation, target) in &edges {
            if observed_relation == relation && !edges.contains(&(target, *opposite_relation, source)) {
                return Err(format!("missing Ecore opposite: {source}.{relation} -> {target}.{opposite_relation}").into());
            }
        }
    }
    for (source, kind) in source_kinds {
        let ancestry = cache.get(kind).ok_or_else(|| format!("unknown Pilot export kind: {kind}"))?;
        for (relation, (owner, _, lower, _, _)) in &rules {
            if *lower > 0 && ancestry.contains(*owner)
                && counts.get(&(source, relation)).copied().unwrap_or_default() < *lower {
                return Err(format!("required Ecore reference missing: {source}.{relation}").into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod collection_contract_tests {
    use super::*;

    #[test]
    fn pinned_ecore_distinguishes_unique_and_nonunique_collections() {
        let mut export: PilotExportDocument = serde_json::from_value(json!({
            "elements": [
                {"qualified_name":"Kernel::owner", "kind":"Feature", "library_group":"Kernel"},
                {"qualified_name":"Kernel::member", "kind":"Type", "library_group":"Kernel"}
            ],
            "relationships": [
                {"source": "Kernel::owner", "relation": "type", "target": "Kernel::member"},
                {"source": "Kernel::owner", "relation": "type", "target": "Kernel::member"}
            ]
        })).unwrap();
        assert!(validate_ecore_export_relationships(&export).is_err());
        for relationship in &mut export.relationships {
            relationship.relation = "chaining_feature".to_string();
        }
        export.elements[1].kind = "Feature".to_string();
        validate_ecore_export_relationships(&export).unwrap();
    }

    #[test]
    fn pinned_ecore_rejects_multiple_singular_and_missing_required_references() {
        let mut export: PilotExportDocument = serde_json::from_value(json!({
            "elements": [
                {"qualified_name":"Kernel::member", "kind":"Membership", "library_group":"Kernel"},
                {"qualified_name":"Kernel::a", "kind":"Element", "library_group":"Kernel"},
                {"qualified_name":"Kernel", "kind":"Namespace", "library_group":"Kernel"},
                {"qualified_name":"Kernel::other", "kind":"Element", "library_group":"Kernel"}
            ],
            "relationships": []
        })).unwrap();
        let error = validate_ecore_export_relationships(&export).unwrap_err().to_string();
        assert!(error.contains("required Ecore reference missing"), "{error}");
        export.relationships = serde_json::from_value(json!([
            {"source":"Kernel::member", "relation":"member_element", "target":"Kernel::a"},
            {"source":"Kernel::member", "relation":"membership_owning_namespace", "target":"Kernel"},
            {"source":"Kernel", "relation":"owned_membership", "target":"Kernel::member"},
            {"source":"Kernel::member", "relation":"owner", "target":"Kernel"},
            {"source":"Kernel", "relation":"owned_element", "target":"Kernel::member"},
            {"source":"Kernel::member", "relation":"owner", "target":"Kernel::other"}
        ])).unwrap();
        let error = validate_ecore_export_relationships(&export).unwrap_err().to_string();
        assert!(error.contains("Ecore upper bound exceeded"), "{error}");
        export.relationships.pop();
        validate_ecore_export_relationships(&export).unwrap();
    }

    #[test]
    fn pinned_ecore_checks_reference_source_and_target_classes() {
        let mut export: PilotExportDocument = serde_json::from_value(json!({
            "elements": [
                {"qualified_name":"Kernel::feature", "kind":"Feature", "library_group":"Kernel"},
                {"qualified_name":"Kernel::other", "kind":"Namespace", "library_group":"Kernel"}
            ],
            "relationships": [
                {"source":"Kernel::feature", "relation":"type", "target":"Kernel::other"}
            ]
        })).unwrap();
        let error = validate_ecore_export_relationships(&export).unwrap_err().to_string();
        assert!(error.contains("cannot target Ecore reference"), "{error}");
        export.elements[1].kind = "Type".to_string();
        validate_ecore_export_relationships(&export).unwrap();
        export.relationships[0].relation = "member_element".to_string();
        let error = validate_ecore_export_relationships(&export).unwrap_err().to_string();
        assert!(error.contains("cannot own Ecore reference"), "{error}");
    }

    #[test]
    fn pinned_ecore_requires_both_membership_opposites() {
        let mut export: PilotExportDocument = serde_json::from_value(json!({
            "elements": [
                {"qualified_name":"Kernel::member", "kind":"Membership", "library_group":"Kernel"},
                {"qualified_name":"Kernel::target", "kind":"Element", "library_group":"Kernel"},
                {"qualified_name":"Kernel", "kind":"Namespace", "library_group":"Kernel"},
                {"qualified_name":"Kernel::other", "kind":"Namespace", "library_group":"Kernel"}
            ],
            "relationships": [
                {"source":"Kernel::member", "relation":"member_element", "target":"Kernel::target"},
                {"source":"Kernel::member", "relation":"membership_owning_namespace", "target":"Kernel"}
            ]
        })).unwrap();
        let error = validate_ecore_export_relationships(&export).unwrap_err().to_string();
        assert!(error.contains("missing Ecore opposite"), "{error}");
        export.relationships.push(serde_json::from_value(json!({
            "source":"Kernel", "relation":"owned_membership", "target":"Kernel::member"
        })).unwrap());
        validate_ecore_export_relationships(&export).unwrap();
        export.relationships[1].target = "Kernel::other".to_string();
        let error = validate_ecore_export_relationships(&export).unwrap_err().to_string();
        assert!(error.contains("missing Ecore opposite"), "{error}");
    }

    #[test]
    fn pinned_ecore_derives_missing_opposites_in_encounter_order_once() {
        let export: PilotExportDocument = serde_json::from_value(json!({
            "elements": [],
            "relationships": [
                {"source":"Kernel::second", "relation":"owner", "target":"Kernel"},
                {"source":"Kernel::first", "relation":"owner", "target":"Kernel"},
                {"source":"Kernel::first", "relation":"owning_membership", "target":"Kernel::membership"}
            ]
        })).unwrap();
        let enriched = with_ecore_derived_opposites(&export).unwrap();
        let derived = enriched.relationships.iter().filter(|edge| edge.relation == "owned_element")
            .map(|edge| edge.target.as_str()).collect::<Vec<_>>();
        assert_eq!(derived, ["Kernel::second", "Kernel::first"]);
        assert!(enriched.relationships.iter().any(|edge|
            edge.source == "Kernel::membership" && edge.relation == "owned_member_element"
                && edge.target == "Kernel::first"));
        assert_eq!(with_ecore_derived_opposites(&enriched).unwrap().relationships.len(),
            enriched.relationships.len());
    }
}

fn apply_ecore_membership_cardinality(kir: &mut KirDocument) -> Result<(), Box<dyn std::error::Error>> {
    let extracted: Value = serde_json::from_str(&std::fs::read_to_string(membership_ecore_path())?)?;
    let features = extracted.get("features").and_then(Value::as_array)
        .ok_or("effective Ecore extract has no features")?;
    for (owner, source_name, property) in [
        ("Element", "owningMembership", "owning_membership"),
        ("Membership", "memberElement", "member_element"),
        ("Membership", "membershipOwningNamespace", "membership_owning_namespace"),
        ("OwningMembership", "ownedMemberElement", "owned_member_element"),
    ] {
        let matches = features.iter().filter(|feature| {
            feature.get("name").and_then(Value::as_str) == Some(source_name)
                && feature.get("owner").and_then(Value::as_str)
                    .is_some_and(|id| id.rsplit("//").next() == Some(owner))
                && feature.get("kind").and_then(Value::as_str) == Some("reference")
                && feature.get("upper_bound").and_then(Value::as_i64) == Some(1)
        }).count();
        if matches != 1 { return Err(format!("missing unique singular Ecore reference: {owner}.{source_name}").into()); }
        let suffix = format!("::{owner}.{property}");
        let mut found = 0;
        for element in &mut kir.elements {
            if element.kind == "MetamodelFeature" && element.id.ends_with(&suffix) {
                element.properties.insert("upper".into(), json!(1));
                found += 1;
            }
        }
        if found != 1 { return Err(format!("missing unique KIR metafeature: {owner}.{property}").into()); }
    }
    Ok(())
}

struct Args {
    input_path: PathBuf,
    from_export: bool,
    pilot_jar: Option<PathBuf>,
    output_path: PathBuf,
    rulepack_output_path: PathBuf,
    pilot_root: Option<PathBuf>,
    allow_dirty: bool,
}

fn parse_args() -> Result<Args, Box<dyn std::error::Error>> {
    let mut input_path = None;
    let mut pilot_jar = None;
    let mut output_path = default_stdlib_path();
    let mut rulepack_output_path = default_rulepack_path(&output_path);
    let mut pilot_root = None;
    let mut allow_dirty = false;
    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--from-export" => {
                index += 1;
                let value = args.get(index).ok_or("missing value for --from-export")?;
                input_path = Some(PathBuf::from(value));
            }
            "--out" => {
                index += 1;
                let value = args.get(index).ok_or("missing value for --out")?;
                output_path = PathBuf::from(value);
                rulepack_output_path = default_rulepack_path(&output_path);
            }
            "--rulepack-out" => {
                index += 1;
                let value = args.get(index).ok_or("missing value for --rulepack-out")?;
                rulepack_output_path = PathBuf::from(value);
            }
            "--pilot-root" => {
                index += 1;
                let value = args.get(index).ok_or("missing value for --pilot-root")?;
                pilot_root = Some(PathBuf::from(value));
            }
            "--pilot-jar" => {
                index += 1;
                pilot_jar = Some(PathBuf::from(
                    args.get(index).ok_or("missing value for --pilot-jar")?,
                ));
            }
            "--allow-dirty" => {
                allow_dirty = true;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            unknown => {
                return Err(format!("unknown argument: {unknown}").into());
            }
        }
        index += 1;
    }

    let from_export = input_path.is_some();
    let input_path = match (input_path, pilot_root.as_ref()) {
        (Some(input_path), _) => input_path,
        (None, Some(_)) => sysml_workspace_root().join("target/stdlib-release/pilot-stdlib-export.json"),
        (None, None) => {
            return Err("expected --from-export PATH or --pilot-root PATH".into());
        }
    };

    Ok(Args {
        input_path,
        from_export,
        pilot_jar,
        output_path,
        rulepack_output_path,
        pilot_root,
        allow_dirty,
    })
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --bin import_pilot_stdlib -- [--pilot-root PATH] [--from-export PATH] [--pilot-jar PATH] [--out PATH] [--rulepack-out PATH] [--allow-dirty]"
    );
}

fn default_rulepack_path(output_path: &Path) -> PathBuf {
    let file_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("stdlib.kir.json");
    let rulepack_name = if let Some(prefix) = file_name.strip_suffix(".kir.json") {
        format!("{prefix}.rulepack.json")
    } else {
        format!("{file_name}.rulepack.json")
    };
    output_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(rulepack_name)
}

fn write_rulepack(
    rulepack: &RulePack,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    rulepack.write_pretty_to_path(output_path)?;
    Ok(())
}

fn build_kir_metadata(
    args: &Args,
    input_path: &Path,
    export: &PilotExportDocument,
) -> Result<BTreeMap<String, Value>, Box<dyn std::error::Error>> {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "import_source".to_string(),
        Value::String("pilot".to_string()),
    );
    metadata.insert(
        "imported_at_utc".to_string(),
        Value::String(now_utc_rfc3339()?),
    );
    metadata.insert(
        "importer_version".to_string(),
        Value::String(env!("CARGO_PKG_VERSION").to_string()),
    );
    metadata.insert(
        "input_export_path".to_string(),
        Value::String(metadata_path_string(input_path)),
    );
    metadata.insert(
        "input_export_sha256".to_string(),
        Value::String(sha256_file(input_path)?),
    );
    metadata.insert("ecore_effective_sha256".to_string(), json!(sha256_file(&membership_ecore_path())?));

    if let Some(pilot_root) = &args.pilot_root {
        metadata.insert(
            "pilot_root".to_string(),
            Value::String(metadata_path_string(pilot_root)),
        );
        metadata.insert(
            "library_root".to_string(),
            Value::String(metadata_path_string(&pilot_root.join("sysml.library"))),
        );
        if let Some(commit) = git_stdout(pilot_root, ["rev-parse", "HEAD"]) {
            metadata.insert("pilot_commit".to_string(), Value::String(commit));
        }
        if let Some(describe) =
            git_stdout(pilot_root, ["describe", "--tags", "--always", "--dirty"])
        {
            metadata.insert("pilot_git_describe".to_string(), Value::String(describe));
        }
        if let Some(dirty) = git_dirty(pilot_root) {
            metadata.insert("pilot_dirty".to_string(), Value::Bool(dirty));
        }
    }

    if let Some(jar) = &args.pilot_jar {
        metadata.insert(
            "pilot_jar_path".to_string(),
            json!(metadata_path_string(jar)),
        );
        metadata.insert("pilot_jar_sha256".to_string(), json!(sha256_file(jar)?));
    }
    if let Some(stdlib_version) = infer_stdlib_version(args, export) {
        metadata.insert("stdlib_version".to_string(), Value::String(stdlib_version));
    }

    if let Some(export_metadata) = &export.metadata {
        metadata.insert("source_export".to_string(), export_metadata.clone());
    }

    metadata.insert(
        "element_count".to_string(),
        json!(kir_element_count(export)),
    );
    metadata.insert(
        "relationship_count".to_string(),
        json!(export.relationships.len()),
    );

    Ok(metadata)
}

fn infer_stdlib_version(args: &Args, export: &PilotExportDocument) -> Option<String> {
    if let Some(version) = export
        .metadata
        .as_ref()
        .and_then(|value| value.get("pilot_version"))
        .and_then(Value::as_str)
    {
        return Some(version.to_string());
    }

    args.pilot_root
        .as_deref()
        .and_then(|pilot_root| find_interactive_jar(pilot_root).ok())
        .and_then(|jar| {
            jar.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .and_then(|file_name| {
            file_name
                .strip_prefix("org.omg.sysml.interactive-")
                .and_then(|name| name.strip_suffix("-all.jar"))
                .map(str::to_string)
        })
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
            "Pilot checkout `{}` is dirty; pass --allow-dirty only for non-release/debug regeneration",
            pilot_root.display()
        )
        .into());
    }

    Ok(())
}

fn kir_element_count(export: &PilotExportDocument) -> usize {
    export.elements.len()
}

fn now_utc_rfc3339() -> Result<String, Box<dyn std::error::Error>> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

fn metadata_path_string(path: &Path) -> String {
    let repo_root = repo_root();
    let absolute_path = path
        .canonicalize()
        .unwrap_or_else(|_| absolute_path_lossy(path));
    let absolute_repo_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| absolute_path_lossy(&repo_root));

    if let Ok(relative) = absolute_path.strip_prefix(&absolute_repo_root) {
        return path_to_slash_string(relative);
    }

    if let Some(parent) = absolute_repo_root.parent() {
        if let Ok(relative) = absolute_path.strip_prefix(parent) {
            return format!("../{}", path_to_slash_string(relative));
        }
    }

    path_to_slash_string(&absolute_path)
}

fn absolute_path_lossy(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn path_to_slash_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn export_from_pilot(
    pilot_root: &Path,
    export_path: &Path,
    pilot_jar: Option<&Path>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let pilot_root = pilot_root.canonicalize()?;
    let library_root = pilot_root.join("sysml.library");
    let interactive_jar = match pilot_jar {
        Some(path) => path.canonicalize()?,
        None => find_interactive_jar(&pilot_root)?,
    };
    let classes_dir = sysml_workspace_root().join("../target/pilot-exporter-classes");
    let java_source = sysml_workspace_root().join(
        "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotStdlibExporter.java",
    );

    compile_java_exporter(&interactive_jar, &java_source, &classes_dir)?;
    run_java_exporter(&interactive_jar, &classes_dir, &library_root, export_path)?;
    Ok(export_path.to_path_buf())
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
    let class_file = classes_dir.join("dev/mercurio/pilot/PilotStdlibExporter.class");
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

fn run_java_exporter(
    interactive_jar: &Path,
    classes_dir: &Path,
    library_root: &Path,
    export_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = export_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

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

    let status = if cfg!(windows) {
        let script_path = sysml_workspace_root().join("target/run_pilot_exporter.ps1");
        let script = format!(
            "$cp = '{}'\njava -cp $cp dev.mercurio.pilot.PilotStdlibExporter '{}' '{}'\n",
            classpath.replace('\'', "''"),
            java_path_string(library_root).replace('\'', "''"),
            java_path_string(export_path).replace('\'', "''"),
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
            .arg("dev.mercurio.pilot.PilotStdlibExporter")
            .arg(library_root)
            .arg(export_path)
            .status()?
    };

    if !status.success() {
        return Err("failed to run Java pilot exporter".into());
    }

    Ok(())
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
