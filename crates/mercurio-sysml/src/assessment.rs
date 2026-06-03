use mercurio_core::Fact;
use mercurio_language_contracts::ast::{Declaration, GenericUsageDecl, ParsedModule};

pub fn sysml_parsed_module_assessment_facts(module: &ParsedModule) -> Vec<Fact> {
    let mut facts = Vec::new();
    collect_declaration_assessment_facts(&module.members, None, &mut facts);
    facts
}

fn collect_declaration_assessment_facts(
    declarations: &[Declaration],
    owner: Option<&str>,
    facts: &mut Vec<Fact>,
) {
    for declaration in declarations {
        if let Some(definition) = declaration.as_definition_like() {
            let id = scoped_id(owner, &definition.name);
            facts.push(Fact::new("definition", [id.clone()]));
            facts.push(Fact::new(
                "definition_keyword",
                [id.clone(), definition.keyword.clone()],
            ));
            facts.push(Fact::new("name", [id.clone(), definition.name.clone()]));
            if let Some(owner) = owner {
                facts.push(Fact::new("owns", [owner.to_string(), id.clone()]));
            }
            collect_declaration_assessment_facts(&definition.members, Some(&id), facts);
            continue;
        }
        if let Some(usage) = declaration.as_usage_like() {
            collect_usage_assessment_facts(&usage, owner, facts);
            continue;
        }

        if let Declaration::Package(package) = declaration {
            let name = package.name.as_colon_string();
            let id = owner
                .map(|owner| format!("{owner}::{name}"))
                .unwrap_or_else(|| name.clone());
            facts.push(Fact::new("package", [id.clone()]));
            facts.push(Fact::new("name", [id.clone(), name]));
            if owner.is_none() {
                facts.push(Fact::new("top_level_package", [id.clone()]));
            }
            if let Some(owner) = owner {
                facts.push(Fact::new("owns", [owner.to_string(), id.clone()]));
            }
            collect_declaration_assessment_facts(&package.members, Some(&id), facts);
        }
    }
}

fn collect_usage_assessment_facts(
    usage: &GenericUsageDecl,
    owner: Option<&str>,
    facts: &mut Vec<Fact>,
) {
    let id = scoped_id(owner, &usage.name);
    facts.push(Fact::new("usage", [id.clone()]));
    facts.push(Fact::new(
        "usage_keyword",
        [id.clone(), usage.keyword.clone()],
    ));
    for modifier in &usage.modifiers {
        facts.push(Fact::new("modifier", [id.clone(), modifier.clone()]));
    }
    if matches!(
        usage.keyword.as_str(),
        "connect" | "connection" | "interface"
    ) {
        facts.push(Fact::new("connection_usage", [id.clone()]));
    }
    if usage.keyword == "interface" {
        facts.push(Fact::new("interface_usage", [id.clone()]));
    }
    facts.push(Fact::new("name", [id.clone(), usage.name.clone()]));
    if let Some(owner) = owner {
        facts.push(Fact::new("owns", [owner.to_string(), id.clone()]));
    }
    if let Some(ty) = &usage.ty {
        facts.push(Fact::new("type", [id.clone(), ty.as_colon_string()]));
    }
    if let Some(reference_target) = &usage.reference_target {
        let target = reference_target.as_colon_string();
        facts.push(Fact::new("reference_target", [id.clone(), target.clone()]));
        if let Some(owner) = owner {
            if usage
                .modifiers
                .iter()
                .any(|modifier| modifier == "end-source")
            {
                facts.push(Fact::new(
                    "connected_source",
                    [owner.to_string(), target.clone()],
                ));
                facts.push(Fact::new(
                    "connected_endpoint",
                    [owner.to_string(), "source".to_string(), target.clone()],
                ));
            }
            if usage
                .modifiers
                .iter()
                .any(|modifier| modifier == "end-target")
            {
                facts.push(Fact::new(
                    "connected_target",
                    [owner.to_string(), target.clone()],
                ));
                facts.push(Fact::new(
                    "connected_endpoint",
                    [owner.to_string(), "target".to_string(), target],
                ));
            }
        }
    }
    if let Some(multiplicity) = &usage.multiplicity {
        facts.push(Fact::new(
            "multiplicity",
            [id.clone(), multiplicity.raw.clone()],
        ));
        facts.push(Fact::new(
            "multiplicity_lower",
            [id.clone(), multiplicity.lower.clone()],
        ));
        facts.push(Fact::new(
            "multiplicity_upper",
            [id.clone(), multiplicity.upper.clone()],
        ));
    }
    collect_declaration_assessment_facts(&usage.body_members, Some(&id), facts);
}

fn scoped_id(owner: Option<&str>, name: &str) -> String {
    owner
        .map(|owner| format!("{owner}::{name}"))
        .unwrap_or_else(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_sysml;

    #[test]
    fn extracts_sysml_package_definition_and_usage_facts() {
        let module =
            parse_sysml("package Demo { part def Vehicle; part vehicle : Vehicle; }").unwrap();

        let facts = sysml_parsed_module_assessment_facts(&module);

        assert!(
            facts
                .iter()
                .any(|fact| fact.predicate == "package" && fact.terms == ["Demo".to_string()])
        );
        assert!(
            facts
                .iter()
                .any(|fact| fact.predicate == "definition_keyword"
                    && fact.terms == ["Demo::Vehicle".to_string(), "part".to_string()])
        );
        assert!(facts.iter().any(|fact| fact.predicate == "usage_keyword"
            && fact.terms == ["Demo::vehicle".to_string(), "part".to_string()]));
        assert!(facts.iter().any(|fact| fact.predicate == "type"
            && fact.terms == ["Demo::vehicle".to_string(), "Vehicle".to_string()]));
    }
}
