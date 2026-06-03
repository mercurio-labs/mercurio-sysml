use std::collections::BTreeMap;

use mercurio_core::{AuthoringError, AuthoringProject, KirDocument};

use crate::{compile_sysml_text, default_sysml_library_path, parse_sysml};

pub fn load_authoring_project_from_sysml(
    files: BTreeMap<String, String>,
) -> Result<AuthoringProject, AuthoringError> {
    let mut modules = BTreeMap::new();
    let mut original_texts = BTreeMap::new();
    for (path, source) in files {
        let module = parse_sysml(&source).map_err(AuthoringError::from)?;
        original_texts.insert(path.clone(), source);
        modules.insert(path, module);
    }
    AuthoringProject::from_parsed_modules(modules, original_texts)
        .map(|project| project.with_source_compiler(compile_sysml_authoring_sources))
}

fn compile_sysml_authoring_sources(
    files: &BTreeMap<String, String>,
) -> Result<KirDocument, AuthoringError> {
    let stdlib =
        KirDocument::from_path(&default_sysml_library_path()).map_err(AuthoringError::Kir)?;
    let mut documents = Vec::new();
    for (path, source) in files {
        documents.push(compile_sysml_text(source, path, &stdlib).map_err(AuthoringError::Parse)?);
    }
    KirDocument::merge(documents).map_err(AuthoringError::Kir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mercurio_core::{ContainerSelector, Mutation, QualifiedName};

    #[test]
    fn loads_sysml_authoring_project_from_source_files() {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "demo.sysml".to_string(),
            "package Demo { part def Vehicle; part vehicle : Vehicle; }".to_string(),
        )]))
        .unwrap();

        assert_eq!(project.files().count(), 1);
        assert!(
            project
                .render_new_file("demo.sysml")
                .unwrap()
                .contains("Vehicle")
        );
    }

    #[test]
    fn validates_mutated_part_definition_with_sysml_compiler() {
        let mut project = load_authoring_project_from_sysml(BTreeMap::new()).unwrap();
        let package = project
            .apply_mutation(Mutation::AddPackage {
                target_file: "demo.sysml".to_string(),
                package_name: QualifiedName(vec!["Demo".to_string()]),
            })
            .unwrap();
        project.write_back_mutation(&package).unwrap();

        let definition = project
            .apply_mutation(Mutation::AddDefinition {
                container: ContainerSelector::Package {
                    qualified_name: QualifiedName(vec!["Demo".to_string()]),
                },
                keyword: "part".to_string(),
                name: "Vehicle".to_string(),
                specializes: Vec::new(),
            })
            .unwrap();
        project.write_back_mutation(&definition).unwrap();
    }
}
