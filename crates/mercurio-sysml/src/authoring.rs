use std::collections::BTreeMap;

use mercurio_core::{
    AuthoringError, AuthoringProject, KirDocument, textual_model_authoring_render_profile,
};

use crate::{
    compile_sysml_text_with_context, parse_sysml, shared_sysml_baseline, sysml_field_specs,
};

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
    AuthoringProject::from_parsed_modules(modules, original_texts).map(|project| {
        project
            .with_render_profile(textual_model_authoring_render_profile())
            .with_source_compiler(compile_sysml_authoring_sources)
    })
}

fn compile_sysml_authoring_sources(
    files: &BTreeMap<String, String>,
) -> Result<KirDocument, AuthoringError> {
    let stdlib = shared_sysml_baseline().map_err(AuthoringError::Kir)?;

    // Every file must be compiled with the whole project in scope, exactly the
    // way the workspace source compiler does it (see
    // `SourceCompileContext::from_source_documents` in mercurio-console-api).
    // Compiling each file in isolation makes a cross-file `import Other::*;`
    // unresolvable, so a perfectly valid `part chassis : Chassis;` in one file
    // fails to compile when `Chassis` is declared in another — which in turn
    // made every semantic mutation over a multi-file workspace report a
    // spurious `unresolved type` validation failure.
    let mut context_modules = Vec::with_capacity(files.len());
    for source in files.values() {
        context_modules.push(parse_sysml(source).map_err(AuthoringError::Parse)?);
    }

    let mut documents = Vec::new();
    for (path, source) in files {
        documents.push(
            compile_sysml_text_with_context(source, path, &context_modules, &stdlib)
                .map_err(AuthoringError::Parse)?,
        );
    }
    KirDocument::merge_with_registered_fields(documents, sysml_field_specs().iter().copied())
        .map_err(AuthoringError::Kir)
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

    #[test]
    fn compiles_authoring_project_with_package_imported_scalar_type() {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "decision.sysml".to_string(),
            "package Demo { import ScalarValues::*; part def Thing { attribute score : Real = 1.0; } }"
                .to_string(),
        )]))
        .unwrap();

        project.compile_kir_document().unwrap();
    }

    #[test]
    fn compiles_authoring_project_across_files_via_wildcard_import() {
        // Regression: each file used to be compiled in isolation, so
        // `import Parts::*;` in `system.sysml` could not see `Chassis`
        // declared in `parts.sysml` and compilation failed with
        // "unresolved type `Chassis`".
        let project = load_authoring_project_from_sysml(BTreeMap::from([
            (
                "parts.sysml".to_string(),
                "package Parts { part def Chassis; }".to_string(),
            ),
            (
                "system.sysml".to_string(),
                "package System { import Parts::*; part def Rover { part chassis : Chassis; } }"
                    .to_string(),
            ),
        ]))
        .unwrap();

        let document = project
            .compile_kir_document()
            .expect("cross-file wildcard import must compile");
        assert!(
            document
                .elements
                .iter()
                .any(|element| element.id.contains("Parts") && element.id.contains("Chassis")),
            "compiled document should contain the cross-file definition"
        );
    }
}
