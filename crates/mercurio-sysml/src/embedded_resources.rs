#[cfg(feature = "embed-stdlib")]
pub(crate) static EMBEDDED_KERNEL: &[u8] =
    include_bytes!("../../../resources/kernel/kerml-kernel.kir.json");

#[cfg(feature = "embed-stdlib")]
pub(crate) static EMBEDDED_SYSML_LIBRARY: &[u8] = include_bytes!(
    "../../../resources/metamodels/sysml-2.0-metamodel-0.57.0/stdlib/sysml-library.kir.json"
);

pub(crate) static METAMODEL_REGISTRY: &str =
    include_str!("../../../resources/metamodels/registry.json");

pub(crate) static SYSML_2_0_METAMODEL_057_RULEPACK: &str = include_str!(
    "../../../resources/metamodels/sysml-2.0-metamodel-0.57.0/stdlib/stdlib.rulepack.json"
);

pub(crate) static SYSML_2_0_PILOT_2026_04_RULEPACK: &str = include_str!(
    "../../../resources/metamodels/sysml-2.0-pilot-2026-04/stdlib/stdlib.rulepack.json"
);
