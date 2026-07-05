use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use mercurio_core::{KirDocument, KirFieldKind, generate_rust_stdlib_consts};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RegistryEntry {
    id: String,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let resources = manifest_dir.join("../../resources");
    let registry_path = resources.join("metamodels/registry.json");
    println!("cargo:rerun-if-changed={}", registry_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        resources.join("kernel").display()
    );
    let registry_text =
        fs::read_to_string(&registry_path).expect("failed to read SysML metamodel registry");
    let metamodels: Vec<RegistryEntry> =
        serde_json::from_str(&registry_text).expect("failed to parse SysML metamodel registry");

    for metamodel in metamodels {
        println!(
            "cargo:rerun-if-changed={}",
            resources.join("metamodels").join(&metamodel.id).display()
        );
        let document = load_baseline_for_build(&resources, &metamodel.id);
        let rust_source = generate_rust_stdlib_consts(&document, &metamodel.id);
        let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
            .join(format!("stdlib_consts_{}.rs", metamodel.id));
        fs::write(out, rust_source).expect("failed to write generated stdlib constants");
    }
}

fn load_baseline_for_build(resources: &Path, metamodel_id: &str) -> KirDocument {
    let kernel = KirDocument::from_path_with_registered_fields(
        &resources.join("kernel/kerml-kernel.kir.json"),
        sysml_field_specs().iter().copied(),
    )
    .expect("failed to load kernel KIR");
    let sysml = KirDocument::from_path_with_registered_fields(
        &resources
            .join("metamodels")
            .join(metamodel_id)
            .join("stdlib/sysml-library.kir.json"),
        sysml_field_specs().iter().copied(),
    )
    .expect("failed to load SysML stdlib KIR");
    KirDocument::merge([kernel, sysml]).expect("failed to merge stdlib KIR")
}

fn sysml_field_specs() -> &'static [(&'static str, KirFieldKind)] {
    &[
        ("type_label", KirFieldKind::Scalar),
        ("pilot_library_group", KirFieldKind::Scalar),
        ("direction", KirFieldKind::Scalar),
        ("multiplicity", KirFieldKind::Scalar),
        ("multiplicity_lower", KirFieldKind::Scalar),
        ("multiplicity_upper", KirFieldKind::Scalar),
        ("declared_multiplicity", KirFieldKind::Scalar),
        ("operator", KirFieldKind::Scalar),
        ("operator_expression", KirFieldKind::Scalar),
        ("trigger", KirFieldKind::Scalar),
        ("trigger_kind", KirFieldKind::Scalar),
        ("is_initial", KirFieldKind::Scalar),
        ("source_is_initial", KirFieldKind::Scalar),
        ("effect", KirFieldKind::Scalar),
        ("text", KirFieldKind::Scalar),
        ("requirement_id", KirFieldKind::Scalar),
        ("body", KirFieldKind::Scalar),
        ("locale", KirFieldKind::Scalar),
        ("language", KirFieldKind::Scalar),
        ("source_file", KirFieldKind::Scalar),
        ("source_language", KirFieldKind::Scalar),
        ("is_abstract", KirFieldKind::Scalar),
        ("is_conjugated", KirFieldKind::Scalar),
        ("is_derived", KirFieldKind::Scalar),
        ("is_end", KirFieldKind::Scalar),
        ("is_variable", KirFieldKind::Scalar),
        ("is_readonly", KirFieldKind::Scalar),
        ("is_ordered", KirFieldKind::Scalar),
        ("is_unique", KirFieldKind::Scalar),
        ("is_library_element", KirFieldKind::Scalar),
        ("is_implied", KirFieldKind::Scalar),
        ("definition", KirFieldKind::Reference),
        ("metatype", KirFieldKind::Reference),
        ("source_feature", KirFieldKind::Reference),
        ("allocated", KirFieldKind::Reference),
        ("allocated_to", KirFieldKind::Reference),
        ("parent_state", KirFieldKind::Reference),
        ("payload", KirFieldKind::Reference),
        ("result", KirFieldKind::Reference),
        ("original_definition", KirFieldKind::Reference),
        ("conjugated", KirFieldKind::Reference),
        ("opposite", KirFieldKind::Reference),
        ("documentedElement", KirFieldKind::Reference),
        ("annotatedElement", KirFieldKind::Reference),
        ("target_ref", KirFieldKind::Reference),
        ("documentation", KirFieldKind::ReferenceList),
        ("feature_typings", KirFieldKind::ReferenceList),
        ("subsets", KirFieldKind::ReferenceList),
        ("subsetted_features", KirFieldKind::ReferenceList),
        ("redefines", KirFieldKind::ReferenceList),
        ("redefined_features", KirFieldKind::ReferenceList),
        ("specialized_features", KirFieldKind::ReferenceList),
        ("featuring_type", KirFieldKind::ReferenceList),
        ("chaining_feature", KirFieldKind::ReferenceList),
        ("imports", KirFieldKind::ReferenceList),
        ("relationships", KirFieldKind::ReferenceList),
        ("sources", KirFieldKind::ReferenceList),
        ("targets", KirFieldKind::ReferenceList),
        ("parts", KirFieldKind::ReferenceList),
        ("items", KirFieldKind::ReferenceList),
        ("owned_feature", KirFieldKind::ReferenceList),
        ("verify", KirFieldKind::ReferenceList),
        ("satisfy", KirFieldKind::ReferenceList),
        ("related", KirFieldKind::ReferenceList),
        ("parameters", KirFieldKind::ReferenceList),
        ("arguments", KirFieldKind::ReferenceList),
        ("successions", KirFieldKind::ReferenceList),
        ("dependencies", KirFieldKind::ReferenceList),
        ("do_behavior", KirFieldKind::Metadata),
    ]
}
