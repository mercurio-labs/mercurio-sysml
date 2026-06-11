#[cfg(feature = "embed-stdlib")]
pub(crate) static EMBEDDED_KERNEL: &[u8] =
    include_bytes!("../../../resources/kernel/kerml-kernel.kir.json");

#[cfg(feature = "embed-stdlib")]
pub(crate) static EMBEDDED_SYSML_LIBRARY: &[u8] = include_bytes!(
    "../../../resources/metamodels/sysml-2.0-metamodel-0.57.0/stdlib/sysml-library.kir.json"
);

pub(crate) static METAMODEL_REGISTRY: &str =
    include_str!("../../../resources/metamodels/registry.json");
