//! Versioned resource bundle for the Mercurio KerML and SysML crates.
//!
//! This is an implementation package of the `mercurio-sysml` release unit.
//! Applications should normally depend on `mercurio-sysml`, not this crate.

use std::path::{Path, PathBuf};

pub const SYSML_2_0_METAMODEL_057_ID: &str = "sysml-2.0-metamodel-0.57.0";
pub const SYSML_2_0_PILOT_2026_04_ID: &str = "sysml-2.0-pilot-2026-04";

pub static KERML_KERNEL: &str = include_str!("../kernel/kerml-kernel.kir.json");
pub static METAMODEL_REGISTRY: &str = include_str!("../metamodels/registry.json");
pub static SYSML_057_METAMODEL: &str =
    include_str!("../metamodels/sysml-2.0-metamodel-0.57.0/metamodel.json");
pub static SYSML_2026_04_METAMODEL: &str =
    include_str!("../metamodels/sysml-2.0-pilot-2026-04/metamodel.json");
pub static SYSML_057_LIBRARY: &[u8] =
    include_bytes!("../metamodels/sysml-2.0-metamodel-0.57.0/stdlib/sysml-library.kir.json");
// The 2026-04 release currently shares these generated library bytes. The
// archive retains both release directories for provenance, while compiled
// consumers carry only one copy.
pub static SYSML_2026_04_LIBRARY: &[u8] = SYSML_057_LIBRARY;
pub static SYSML_057_FULL_STDLIB: &[u8] =
    include_bytes!("../metamodels/sysml-2.0-metamodel-0.57.0/stdlib/stdlib.full.kir.json");
pub static SYSML_2026_04_FULL_STDLIB: &[u8] = SYSML_057_FULL_STDLIB;
pub static SYSML_057_RULEPACK: &str =
    include_str!("../metamodels/sysml-2.0-metamodel-0.57.0/stdlib/stdlib.rulepack.json");
pub static SYSML_2026_04_RULEPACK: &str =
    include_str!("../metamodels/sysml-2.0-pilot-2026-04/stdlib/stdlib.rulepack.json");
pub static SYSML_057_METAMODEL_CONSTRUCTS: &str = include_str!(
    "../metamodels/sysml-2.0-metamodel-0.57.0/mappings/metamodel_constructs.seed.json"
);
pub static SYSML_057_KIR_EMISSION: &str =
    include_str!("../metamodels/sysml-2.0-metamodel-0.57.0/mappings/kir_emission.seed.json");
pub static SYSML_057_LOWERING_RULES: &str =
    include_str!("../metamodels/sysml-2.0-metamodel-0.57.0/mappings/lowering_rules.seed.json");
pub static SYSML_057_SEMANTIC_DEFAULTS: &str =
    include_str!("../metamodels/sysml-2.0-metamodel-0.57.0/mappings/semantic_defaults.seed.json");

pub fn resource_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

pub fn metamodel_descriptor(id: &str) -> Option<&'static str> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(SYSML_057_METAMODEL),
        SYSML_2_0_PILOT_2026_04_ID => Some(SYSML_2026_04_METAMODEL),
        _ => None,
    }
}

pub fn sysml_library(id: &str) -> Option<&'static [u8]> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(SYSML_057_LIBRARY),
        SYSML_2_0_PILOT_2026_04_ID => Some(SYSML_2026_04_LIBRARY),
        _ => None,
    }
}

pub fn full_stdlib(id: &str) -> Option<&'static [u8]> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(SYSML_057_FULL_STDLIB),
        SYSML_2_0_PILOT_2026_04_ID => Some(SYSML_2026_04_FULL_STDLIB),
        _ => None,
    }
}

pub fn rulepack(id: &str) -> Option<&'static str> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(SYSML_057_RULEPACK),
        SYSML_2_0_PILOT_2026_04_ID => Some(SYSML_2026_04_RULEPACK),
        _ => None,
    }
}

pub fn metamodel_constructs(id: &str) -> Option<&'static str> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(include_str!(
            "../metamodels/sysml-2.0-metamodel-0.57.0/mappings/metamodel_constructs.seed.json"
        )),
        SYSML_2_0_PILOT_2026_04_ID => Some(include_str!(
            "../metamodels/sysml-2.0-pilot-2026-04/mappings/metamodel_constructs.seed.json"
        )),
        _ => None,
    }
}

pub fn kir_emission(id: &str) -> Option<&'static str> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(include_str!(
            "../metamodels/sysml-2.0-metamodel-0.57.0/mappings/kir_emission.seed.json"
        )),
        SYSML_2_0_PILOT_2026_04_ID => Some(include_str!(
            "../metamodels/sysml-2.0-pilot-2026-04/mappings/kir_emission.seed.json"
        )),
        _ => None,
    }
}

pub fn lowering_rules(id: &str) -> Option<&'static str> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(include_str!(
            "../metamodels/sysml-2.0-metamodel-0.57.0/mappings/lowering_rules.seed.json"
        )),
        SYSML_2_0_PILOT_2026_04_ID => Some(include_str!(
            "../metamodels/sysml-2.0-pilot-2026-04/mappings/lowering_rules.seed.json"
        )),
        _ => None,
    }
}

pub fn semantic_defaults(id: &str) -> Option<&'static str> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(include_str!(
            "../metamodels/sysml-2.0-metamodel-0.57.0/mappings/semantic_defaults.seed.json"
        )),
        SYSML_2_0_PILOT_2026_04_ID => Some(include_str!(
            "../metamodels/sysml-2.0-pilot-2026-04/mappings/semantic_defaults.seed.json"
        )),
        _ => None,
    }
}

pub fn field_specs_generated(id: &str) -> Option<&'static str> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(include_str!(
            "../metamodels/sysml-2.0-metamodel-0.57.0/mappings/field_specs.generated.json"
        )),
        SYSML_2_0_PILOT_2026_04_ID => Some(include_str!(
            "../metamodels/sysml-2.0-pilot-2026-04/mappings/field_specs.generated.json"
        )),
        _ => None,
    }
}

pub fn field_specs_overlay(id: &str) -> Option<&'static str> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(include_str!(
            "../metamodels/sysml-2.0-metamodel-0.57.0/mappings/field_specs.overlay.json"
        )),
        SYSML_2_0_PILOT_2026_04_ID => Some(include_str!(
            "../metamodels/sysml-2.0-pilot-2026-04/mappings/field_specs.overlay.json"
        )),
        _ => None,
    }
}

pub fn grammar_extract(id: &str) -> Option<&'static str> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(include_str!(
            "../metamodels/sysml-2.0-metamodel-0.57.0/grammar.extract.json"
        )),
        SYSML_2_0_PILOT_2026_04_ID => Some(include_str!(
            "../metamodels/sysml-2.0-pilot-2026-04/grammar.extract.json"
        )),
        _ => None,
    }
}

pub fn metamodel_extract(id: &str) -> Option<&'static str> {
    match id {
        SYSML_2_0_METAMODEL_057_ID => Some(include_str!(
            "../metamodels/sysml-2.0-metamodel-0.57.0/metamodel.extract.json"
        )),
        SYSML_2_0_PILOT_2026_04_ID => Some(include_str!(
            "../metamodels/sysml-2.0-pilot-2026-04/metamodel.extract.json"
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{SYSML_057_FULL_STDLIB, SYSML_057_LIBRARY};

    #[test]
    fn aliased_release_libraries_match_provenance_files() {
        assert_eq!(
            SYSML_057_LIBRARY,
            include_bytes!("../metamodels/sysml-2.0-pilot-2026-04/stdlib/sysml-library.kir.json")
        );
        assert_eq!(
            SYSML_057_FULL_STDLIB,
            include_bytes!("../metamodels/sysml-2.0-pilot-2026-04/stdlib/stdlib.full.kir.json")
        );
    }
}
