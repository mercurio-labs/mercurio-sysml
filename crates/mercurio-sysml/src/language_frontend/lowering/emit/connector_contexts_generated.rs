// Generated concrete utility inventory; lifecycle producers are separately implemented.
pub(super) const UTILITY_SHA: &str = "476c67057bce6e28f6a39f4faf1c5096fe066bbc047328fe042f89127a60338b";
pub(super) fn supports(kind: &str) -> bool { matches!(kind, "AllocationUsage" | "BindingConnector" | "BindingConnectorAsUsage" | "ConnectionUsage" | "Connector" | "Flow" | "FlowUsage" | "InterfaceUsage" | "Succession" | "SuccessionAsUsage" | "SuccessionFlow" | "SuccessionFlowUsage") }
