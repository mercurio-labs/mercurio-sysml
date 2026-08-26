//! Resolution policy controls for lowering.

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvePolicy {
    pub(crate) preserve_unresolved_references: bool,
}

pub(crate) const STRICT_RESOLVE_POLICY: ResolvePolicy = ResolvePolicy {
    preserve_unresolved_references: false,
};

pub(crate) const KERML_RESOLVE_POLICY: ResolvePolicy = ResolvePolicy {
    preserve_unresolved_references: true,
};
