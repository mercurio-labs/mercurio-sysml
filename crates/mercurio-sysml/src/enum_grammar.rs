//! Pinned Xtext enum rules resolved to Ecore enum literals.
//! Construction consumers select a rule; token spelling alone is ambiguous.

#[path = "enum_grammar_generated.rs"]
mod generated;

pub(crate) use generated::EnumKeyword;

pub(crate) fn lookup(kerml: bool, rule: &str, token: &str) -> Option<&'static EnumKeyword> {
    if !crate::xtext_terminal::is_keyword(kerml, token) { return None; }
    let language = if kerml { "org.omg.kerml.xtext.KerML" } else { "org.omg.sysml.xtext.SysML" };
    generated::ENUM_KEYWORDS.iter().find(|row|
        row.language == language && row.rule == rule && row.token == token)
}

pub(crate) fn declaration_modifier(kerml: bool, token: &str) -> bool {
    ["VisibilityIndicator", "FeatureDirection"]
        .into_iter().any(|rule| lookup(kerml, rule, token).is_some())
        || !kerml && lookup(false, "PortionKind", token).is_some()
}

pub(crate) fn shared_literal(rule: &str, token: &str) -> Option<&'static str> {
    lookup(false, rule, token).map(|row| row.literal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_keywords_resolve_by_rule_and_language() {
        assert_eq!(generated::ENUM_KEYWORDS.len(), 24);
        for kerml in [true, false] {
            let visibility = lookup(kerml, "VisibilityIndicator", "protected").unwrap();
            assert_eq!((visibility.literal, visibility.value), ("protected", 1));
            let direction = lookup(kerml, "FeatureDirection", "inout").unwrap();
            assert_eq!((direction.literal, direction.value), ("inout", 1));
            assert!(lookup(kerml, "FeatureDirection", "protected").is_none());
        }
        assert_eq!(lookup(false, "PortionKind", "snapshot").unwrap().value, 1);
        assert!(lookup(true, "PortionKind", "snapshot").is_none());
        assert_eq!(lookup(false, "ExposeVisibilityKind", "expose").unwrap().literal, "protected");
        assert_eq!(lookup(true, "FilterPackageMemberVisibility", "[").unwrap().literal, "private");
    }

    #[test]
    fn portion_enum_rules_construct_ecore_values() {
        let source = "package P { snapshot part atStart; timeslice part inUse; }";
        let module = crate::parse_sysml(source).unwrap();
        let library = crate::load_sysml_baseline().unwrap();
        let document = crate::compile_sysml_module(&module, "portion-enums.sysml", &library).unwrap();
        for (name, expected) in [("atStart", "snapshot"), ("inUse", "timeslice")] {
            let element = document.elements.iter().find(|element|
                element.properties.get("declared_name") == Some(&serde_json::json!(name))).unwrap();
            assert_eq!(element.properties["portion_kind"], expected);
        }
        mercurio_foundation::KirDocument::merge_with_registered_fields(
            vec![document], crate::sysml_field_specs().iter().copied(),
        ).unwrap();
    }
}
