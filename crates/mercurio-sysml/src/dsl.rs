use mercurio_foundation::DslExtensionSpec;

pub fn sysml_dsl_extension() -> DslExtensionSpec {
    DslExtensionSpec::new("sysml")
        .with_model_set_contains_any("requirements", "metatype", ["Requirement"])
        .with_model_set_contains_any("parts", "kind", ["PartDefinition", "PartUsage"])
        .with_model_set_contains_any("ports", "kind", ["PortDefinition", "PortUsage"])
        .with_model_set_contains_any(
            "interfaces",
            "kind",
            ["InterfaceDefinition", "InterfaceUsage"],
        )
        .with_model_set_contains_any("actions", "kind", ["ActionDefinition", "ActionUsage"])
        .with_model_set_contains_any("states", "kind", ["StateDefinition", "StateUsage"])
        .with_model_set_contains_any("allocations", "metatype", ["Allocation"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysml_dsl_extension_advertises_model_sets() {
        let extension = sysml_dsl_extension();

        assert_eq!(extension.id, "sysml");
        assert!(
            extension
                .schema_functions
                .contains(&"ModelContext.requirements".to_string())
        );
        assert!(
            extension
                .model_sets
                .iter()
                .any(|model_set| model_set.name == "parts")
        );
    }
}
