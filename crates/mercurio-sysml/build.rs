use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use mercurio_core::{KirDocument, KirFieldKind, generate_rust_stdlib_consts};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RegistryEntry {
    id: String,
}

#[derive(Debug, Deserialize)]
struct GeneratedFieldSpecs {
    fields: Vec<GeneratedFieldSpec>,
}

#[derive(Debug, Deserialize)]
struct GeneratedFieldSpec {
    field: String,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct FieldSpecOverlay {
    entries: Vec<FieldSpecOverlayEntry>,
}

#[derive(Debug, Deserialize)]
struct FieldSpecOverlayEntry {
    field: String,
    kind: String,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let resources = manifest_dir.join("../../resources");
    let registry_path = resources.join("metamodels/registry.json");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    println!("cargo:rerun-if-changed={}", registry_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        resources.join("kernel").display()
    );
    let registry_text =
        fs::read_to_string(&registry_path).expect("failed to read SysML metamodel registry");
    let metamodels: Vec<RegistryEntry> =
        serde_json::from_str(&registry_text).expect("failed to parse SysML metamodel registry");

    let mut runtime_field_specs = None;
    for metamodel in metamodels {
        println!(
            "cargo:rerun-if-changed={}",
            resources.join("metamodels").join(&metamodel.id).display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            resources
                .join("metamodels")
                .join(&metamodel.id)
                .join("mappings/field_specs.generated.json")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            resources
                .join("metamodels")
                .join(&metamodel.id)
                .join("mappings/field_specs.overlay.json")
                .display()
        );
        let field_specs = load_generated_field_specs(&resources, &metamodel.id);
        match &runtime_field_specs {
            Some(existing) if existing != &field_specs => {
                panic!(
                    "generated field specs for `{}` differ from the first metamodel registry entry",
                    metamodel.id
                );
            }
            None => runtime_field_specs = Some(field_specs.clone()),
            _ => {}
        }

        let document = load_baseline_for_build(&resources, &metamodel.id, &field_specs);
        let rust_source = generate_rust_stdlib_consts(&document, &metamodel.id);
        let out = out_dir.join(format!("stdlib_consts_{}.rs", metamodel.id));
        fs::write(out, rust_source).expect("failed to write generated stdlib constants");
    }

    let runtime_field_specs =
        runtime_field_specs.expect("metamodel registry must contain at least one entry");
    fs::write(
        out_dir.join("sysml_field_specs.rs"),
        generate_field_specs_rust(&runtime_field_specs),
    )
    .expect("failed to write generated SysML field specs");
}

fn load_baseline_for_build(
    resources: &Path,
    metamodel_id: &str,
    field_specs: &[(String, KirFieldKind)],
) -> KirDocument {
    let kernel = KirDocument::from_path_with_registered_fields(
        &resources.join("kernel/kerml-kernel.kir.json"),
        field_specs
            .iter()
            .map(|(name, kind)| (name.as_str(), *kind)),
    )
    .expect("failed to load kernel KIR");
    let sysml = KirDocument::from_path_with_registered_fields(
        &resources
            .join("metamodels")
            .join(metamodel_id)
            .join("stdlib/sysml-library.kir.json"),
        field_specs
            .iter()
            .map(|(name, kind)| (name.as_str(), *kind)),
    )
    .expect("failed to load SysML stdlib KIR");
    KirDocument::merge([kernel, sysml]).expect("failed to merge stdlib KIR")
}

fn load_generated_field_specs(resources: &Path, metamodel_id: &str) -> Vec<(String, KirFieldKind)> {
    let generated_path = resources
        .join("metamodels")
        .join(metamodel_id)
        .join("mappings/field_specs.generated.json");
    let text = fs::read_to_string(&generated_path).unwrap_or_else(|err| {
        panic!(
            "failed to read generated SysML field specs `{}`: {err}",
            generated_path.display()
        )
    });
    let generated: GeneratedFieldSpecs = serde_json::from_str(&text).unwrap_or_else(|err| {
        panic!(
            "failed to parse generated SysML field specs `{}`: {err}",
            generated_path.display()
        )
    });
    let overlay_path = resources
        .join("metamodels")
        .join(metamodel_id)
        .join("mappings/field_specs.overlay.json");
    let overlay_text = fs::read_to_string(&overlay_path).unwrap_or_else(|err| {
        panic!(
            "failed to read SysML field spec overlay `{}`: {err}",
            overlay_path.display()
        )
    });
    let overlay: FieldSpecOverlay = serde_json::from_str(&overlay_text).unwrap_or_else(|err| {
        panic!(
            "failed to parse SysML field spec overlay `{}`: {err}",
            overlay_path.display()
        )
    });
    let mut fields = BTreeMap::new();
    for field in generated.fields {
        let kind = parse_field_kind(&field.kind).unwrap_or_else(|| {
            panic!(
                "unknown generated SysML field kind `{}` for `{}` in `{}`",
                field.kind,
                field.field,
                generated_path.display()
            )
        });
        fields.insert(field.field, kind);
    }
    for field in overlay.entries {
        let kind = parse_field_kind(&field.kind).unwrap_or_else(|| {
            panic!(
                "unknown overlay SysML field kind `{}` for `{}` in `{}`",
                field.kind,
                field.field,
                overlay_path.display()
            )
        });
        fields.insert(field.field, kind);
    }
    fields.into_iter().collect()
}

fn parse_field_kind(value: &str) -> Option<KirFieldKind> {
    match value {
        "Scalar" => Some(KirFieldKind::Scalar),
        "Reference" => Some(KirFieldKind::Reference),
        "ReferenceList" => Some(KirFieldKind::ReferenceList),
        "Expression" => Some(KirFieldKind::Expression),
        "Metadata" => Some(KirFieldKind::Metadata),
        _ => None,
    }
}

fn generate_field_specs_rust(field_specs: &[(String, KirFieldKind)]) -> String {
    let mut source = String::from(
        "pub fn sysml_field_specs() -> &'static [(&'static str, KirFieldKind)] {\n    &[\n",
    );
    for (field, kind) in field_specs {
        source.push_str(&format!(
            "        ({:?}, KirFieldKind::{}),\n",
            field,
            field_kind_variant(*kind)
        ));
    }
    source.push_str("    ]\n}\n");
    source
}

fn field_kind_variant(kind: KirFieldKind) -> &'static str {
    match kind {
        KirFieldKind::Scalar => "Scalar",
        KirFieldKind::Reference => "Reference",
        KirFieldKind::ReferenceList => "ReferenceList",
        KirFieldKind::Expression => "Expression",
        KirFieldKind::Metadata => "Metadata",
    }
}
