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

    use crate::compile_sysml_text_with_context_report;

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

    /// Regression: the authoring renderer used to lower `connect a to b;` into
    /// the verbose `connect { end reference <end-source> source references a; …
    /// }` body form. Any container re-render then rewrote connectors the user
    /// never touched, because a typed `AddUsage` into a `part def` re-renders
    /// the whole container body through the `ReplaceContainer` localized patch.
    #[test]
    fn connect_usage_round_trips_through_a_pure_render() {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "m.sysml".to_string(),
            "package P {\n    part def R {\n        part a : X;\n        connect a to a;\n    }\n    part def X;\n}\n".to_string(),
        )]))
        .unwrap();

        let rendered = project.render_new_file("m.sysml").unwrap();
        assert!(
            rendered.contains("connect a to a;"),
            "connector lost its compact form:\n{rendered}"
        );
        assert!(
            !rendered.contains("end reference"),
            "connector was expanded into end members:\n{rendered}"
        );
    }

    /// A typed `AddUsage` into a `part def` takes the `ReplaceContainer`
    /// localized patch mode, so the whole `R` body — the untouched connector
    /// included — is re-rendered.
    #[test]
    fn connect_usage_survives_a_container_level_re_render() {
        let mut project = load_authoring_project_from_sysml(BTreeMap::from([(
            "m.sysml".to_string(),
            "package P {\n    part def R {\n        part a : X;\n        connect a to a;\n    }\n    part def X;\n}\n".to_string(),
        )]))
        .unwrap();

        let result = project
            .apply_mutation(Mutation::AddUsage {
                container: ContainerSelector::Declaration {
                    qualified_name: QualifiedName(vec!["P".to_string(), "R".to_string()]),
                },
                keyword: "part".to_string(),
                name: "b".to_string(),
                ty: Some(QualifiedName(vec!["X".to_string()])),
                specializes: Vec::new(),
            })
            .unwrap();
        let write_back = project.write_back_mutation(&result).unwrap();

        let text = write_back.edited_files.get("m.sysml").unwrap();
        assert!(
            text.contains("part b: X;"),
            "mutation did not land:\n{text}"
        );
        assert!(
            text.contains("connect a to a;"),
            "container re-render rewrote the untouched connector:\n{text}"
        );
        assert!(
            !text.contains("end reference"),
            "container re-render expanded the connector into end members:\n{text}"
        );
    }

    /// The unnamed transition shorthand. `first` opens the source clause; it is
    /// not the transition's declared name.
    const STATE_MACHINE_SOURCE: &str = "package P {\n    state def S {\n        state idle;\n        state driving;\n        transition first idle then driving;\n    }\n}\n";

    /// Word-boundary occurrences of `needle` in `text`.
    fn keyword_count(text: &str, needle: &str) -> usize {
        text.split(|character: char| !character.is_alphanumeric() && character != '_')
            .filter(|token| *token == needle)
            .count()
    }

    /// A render that produces unparseable text must fail by construction, not
    /// by eye: recompile the rendered file and assert it is diagnostic-free.
    fn assert_recompiles_cleanly(label: &str, text: &str) {
        let stdlib = shared_sysml_baseline().expect("baseline loads");
        let module = parse_sysml(text)
            .unwrap_or_else(|diagnostic| panic!("{label}: rendered text does not parse: {diagnostic:?}\n{text}"));
        let report = compile_sysml_text_with_context_report(
            text,
            "rendered.sysml",
            std::slice::from_ref(&module),
            &stdlib,
        );
        assert!(
            report.diagnostics.is_empty(),
            "{label}: rendered text compiled with diagnostics: {:?}\n{text}",
            report.diagnostics
        );
    }

    /// Regression: `transition first idle then driving;` came back from the
    /// canonical printer as `transition first first idle then driving;` — the
    /// parser swallowed the `first` source marker as the transition's declared
    /// name, and the shorthand renderer then re-emitted the marker itself.
    #[test]
    fn unnamed_transition_shorthand_round_trips_through_a_pure_render() {
        let project = load_authoring_project_from_sysml(BTreeMap::from([(
            "m.sysml".to_string(),
            STATE_MACHINE_SOURCE.to_string(),
        )]))
        .unwrap();

        let rendered = project.render_new_file("m.sysml").unwrap();
        assert_eq!(
            keyword_count(&rendered, "first"),
            1,
            "transition shorthand duplicated the `first` marker:\n{rendered}"
        );
        assert!(
            rendered.contains("transition first idle then driving;"),
            "transition shorthand lost its compact form:\n{rendered}"
        );
        assert_recompiles_cleanly("pure render", &rendered);
    }

    /// A typed `AddUsage` into the `state def` takes the `ReplaceContainer`
    /// localized patch mode, so the untouched transition is re-rendered through
    /// the canonical printer — the path the Inspector create gesture hits.
    #[test]
    fn unnamed_transition_survives_a_container_level_re_render() {
        let mut project = load_authoring_project_from_sysml(BTreeMap::from([(
            "m.sysml".to_string(),
            STATE_MACHINE_SOURCE.to_string(),
        )]))
        .unwrap();

        let result = project
            .apply_mutation(Mutation::AddUsage {
                container: ContainerSelector::Declaration {
                    qualified_name: QualifiedName(vec!["P".to_string(), "S".to_string()]),
                },
                keyword: "state".to_string(),
                name: "parked".to_string(),
                ty: None,
                specializes: Vec::new(),
            })
            .unwrap();
        let write_back = project.write_back_mutation(&result).unwrap();

        let text = write_back.edited_files.get("m.sysml").unwrap();
        assert!(text.contains("parked"), "mutation did not land:\n{text}");
        assert_eq!(
            keyword_count(text, "first"),
            1,
            "container re-render duplicated the `first` marker:\n{text}"
        );
        assert!(
            text.contains("transition first idle then driving;"),
            "container re-render rewrote the untouched transition:\n{text}"
        );
        assert_recompiles_cleanly("container re-render", text);
    }

    /// The named form (`transition t first A then B;`) must keep its name and
    /// still emit exactly one `first`.
    #[test]
    fn named_transition_survives_a_container_level_re_render() {
        let source = "package P {\n    state def S {\n        state idle;\n        state driving;\n        transition go first idle then driving;\n    }\n}\n";
        let mut project = load_authoring_project_from_sysml(BTreeMap::from([(
            "m.sysml".to_string(),
            source.to_string(),
        )]))
        .unwrap();

        let result = project
            .apply_mutation(Mutation::AddUsage {
                container: ContainerSelector::Declaration {
                    qualified_name: QualifiedName(vec!["P".to_string(), "S".to_string()]),
                },
                keyword: "state".to_string(),
                name: "parked".to_string(),
                ty: None,
                specializes: Vec::new(),
            })
            .unwrap();
        let write_back = project.write_back_mutation(&result).unwrap();

        let text = write_back.edited_files.get("m.sysml").unwrap();
        assert_eq!(
            keyword_count(text, "first"),
            1,
            "container re-render duplicated the `first` marker:\n{text}"
        );
        assert!(
            text.contains("transition go first idle then driving;"),
            "container re-render rewrote the named transition:\n{text}"
        );
        assert_recompiles_cleanly("named transition re-render", text);
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
