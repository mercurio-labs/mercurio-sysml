#[cfg(feature = "embed-stdlib")]
pub(crate) static EMBEDDED_KERNEL: &[u8] = mercurio_sysml_resources::KERML_KERNEL.as_bytes();

pub(crate) static METAMODEL_REGISTRY: &str = mercurio_sysml_resources::METAMODEL_REGISTRY;

pub(crate) static SYSML_2_0_METAMODEL_057_RULEPACK: &str =
    mercurio_sysml_resources::SYSML_057_RULEPACK;

pub(crate) static SYSML_2_0_PILOT_2026_04_RULEPACK: &str =
    mercurio_sysml_resources::SYSML_2026_04_RULEPACK;
