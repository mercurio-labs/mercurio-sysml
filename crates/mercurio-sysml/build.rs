use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use mercurio_core::{KirDocument, generate_rust_stdlib_consts};
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
    println!(
        "cargo:rerun-if-changed={}",
        resources
            .join("metamodels/sysml-2.0-metamodel-0.57.0")
            .display()
    );

    let registry_text =
        fs::read_to_string(&registry_path).expect("failed to read SysML metamodel registry");
    let metamodels: Vec<RegistryEntry> =
        serde_json::from_str(&registry_text).expect("failed to parse SysML metamodel registry");

    for metamodel in metamodels {
        let document = load_baseline_for_build(&resources, &metamodel.id);
        let rust_source = generate_rust_stdlib_consts(&document, &metamodel.id);
        let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
            .join(format!("stdlib_consts_{}.rs", metamodel.id));
        fs::write(out, rust_source).expect("failed to write generated stdlib constants");
    }
}

fn load_baseline_for_build(resources: &Path, metamodel_id: &str) -> KirDocument {
    let kernel = KirDocument::from_path(&resources.join("kernel/kerml-kernel.kir.json"))
        .expect("failed to load kernel KIR");
    let sysml = KirDocument::from_path(
        &resources
            .join("metamodels")
            .join(metamodel_id)
            .join("stdlib/sysml-library.kir.json"),
    )
    .expect("failed to load SysML stdlib KIR");
    KirDocument::merge([kernel, sysml]).expect("failed to merge stdlib KIR")
}
