#[cfg(feature = "embed-stdlib")]
pub(crate) static EMBEDDED_KERNEL: &[u8] = crate::resources::KERML_KERNEL.as_bytes();

pub(crate) static METAMODEL_REGISTRY: &str = crate::resources::METAMODEL_REGISTRY;

pub(crate) static SYSML_2_0_METAMODEL_057_RULEPACK: &str = crate::resources::SYSML_057_RULEPACK;

pub(crate) static SYSML_2_0_PILOT_2026_04_RULEPACK: &str = crate::resources::SYSML_2026_04_RULEPACK;
