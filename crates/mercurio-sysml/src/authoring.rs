use std::collections::BTreeMap;

use mercurio_foundation::{
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
    use mercurio_foundation::{ContainerSelector, Mutation, QualifiedName};

    #[test]
    fn checked_metadata_edit_preserves_multiple_top_level_packages() {
        let profile = include_str!("../resources/profiles/MercurioMissions.sysml");
        let mission = include_str!("simulation/thermal-deadline.sysml");
        let source = format!("{profile}\n{mission}");
        let mut project = load_authoring_project_from_sysml(BTreeMap::from([("mission.sysml".into(), source.clone())])).unwrap();
        let mutation = project.apply_mutation(Mutation::AddMetadataAnnotation {
            element: QualifiedName(vec!["SimulationConstraintsChannels".into(), "HeatProfile".into()]),
            metadata_type: "Mercurio::Missions::Termination".into(),
            properties: BTreeMap::from([("onAnyViolated".into(), "true".into())]),
        }).unwrap();
        let edited = project.write_back_mutation(&mutation).unwrap();
        assert!(edited.validation.ok, "{:?}", edited.validation);
        let text = &edited.edited_files["mission.sysml"];
        assert!(text.starts_with(profile), "Unrelated profile source was changed: {text}");
        assert_eq!(text.matches("package SimulationConstraintsChannels").count(), 1);
        assert!(text.contains("@Mercurio::Missions::Termination"));
        let stdlib = crate::load_sysml_baseline().unwrap();
        let runtime = mercurio_foundation::runtime::Runtime::from_document(crate::compile_sysml_text(text, "mission.sysml", &stdlib).unwrap()).unwrap();
        let case = crate::simulation::list_analysis_cases(&runtime).into_iter().find(|c| c.label == "HeatProfile").unwrap();
        let report = crate::simulation::run_analysis_case(&runtime, &case.id, "checked-metadata").unwrap();
        assert_eq!(report.artifacts[0].payload["termination"], "requirement_violated");
    }

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
    fn source_add_move_remove_recompiles_ecore_ownership_opposites() {
        fn assert_reciprocal_ownership(document: &KirDocument) {
            let by_id = document.elements.iter().map(|element| (element.id.as_str(), element)).collect::<BTreeMap<_, _>>();
            for owner in &document.elements {
                for (forward, inverse) in [
                    ("owned_relationship", "owning_related_element"),
                    ("owned_related_element", "owning_relationship"),
                ] {
                    let Some(children) = owner.properties.get(forward) else { continue };
                    for child_id in children.as_array().unwrap().iter().filter_map(|value| value.as_str()) {
                        let child = by_id.get(child_id).unwrap_or_else(|| panic!("missing {child_id}"));
                        assert_eq!(child.properties.get(inverse), Some(&serde_json::json!(owner.id)), "{forward}: {} -> {child_id}", owner.id);
                    }
                }
            }
            for child in &document.elements {
                for (inverse, forward) in [
                    ("owning_related_element", "owned_relationship"),
                    ("owning_relationship", "owned_related_element"),
                ] {
                    let Some(owner_id) = child.properties.get(inverse).and_then(|value| value.as_str()) else { continue };
                    let owner = by_id.get(owner_id).unwrap_or_else(|| panic!("missing {owner_id}"));
                    assert!(owner.properties.get(forward).and_then(|value| value.as_array()).is_some_and(|children| children.iter().any(|value| value.as_str() == Some(&child.id))), "{inverse}: {} -> {owner_id}", child.id);
                }
            }
        }

        let mut project = load_authoring_project_from_sysml(BTreeMap::from([(
            "ownership.sysml".into(), "package P {} package Q {}".into(),
        )])).unwrap();
        let add = project.apply_mutation(Mutation::AddDefinition {
            container: ContainerSelector::Package { qualified_name: QualifiedName(vec!["P".into()]) },
            keyword: "part".into(), name: "A".into(), specializes: Vec::new(),
        }).unwrap();
        project.write_back_mutation(&add).unwrap();
        let added = project.compile_kir_document().unwrap();
        assert_reciprocal_ownership(&added);
        assert!(added.elements.iter().any(|element| element.properties.get("qualified_name") == Some(&serde_json::json!("P.A"))));

        let moved = project.apply_mutation(Mutation::MoveDeclaration {
            qualified_name: QualifiedName(vec!["P".into(), "A".into()]),
            destination: ContainerSelector::Package { qualified_name: QualifiedName(vec!["Q".into()]) },
        }).unwrap();
        project.write_back_mutation(&moved).unwrap();
        let after_move = project.compile_kir_document().unwrap();
        assert_reciprocal_ownership(&after_move);
        assert!(after_move.elements.iter().any(|element| element.properties.get("qualified_name") == Some(&serde_json::json!("Q.A"))));
        assert!(!after_move.elements.iter().any(|element| element.properties.get("qualified_name") == Some(&serde_json::json!("P.A"))));

        let removed = project.apply_mutation(Mutation::RemoveDeclaration {
            qualified_name: QualifiedName(vec!["Q".into(), "A".into()]),
        }).unwrap();
        project.write_back_mutation(&removed).unwrap();
        let after_remove = project.compile_kir_document().unwrap();
        assert_reciprocal_ownership(&after_remove);
        assert!(!after_remove.elements.iter().any(|element| element.properties.get("declared_name") == Some(&serde_json::json!("A"))));
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

    fn semantic_expression_properties(document: &KirDocument) -> BTreeMap<String, serde_json::Value> {
        fn without_spans(value: &mut serde_json::Value) {
            match value {
                serde_json::Value::Array(items) => items.iter_mut().for_each(without_spans),
                serde_json::Value::Object(properties) => {
                    properties.remove("span");
                    properties.values_mut().for_each(without_spans);
                }
                _ => {}
            }
        }
        document.elements.iter().filter(|element| element.properties.contains_key("expression_ir"))
            .map(|element| {
                let mut properties = serde_json::json!({
                    "expression": element.properties["expression_ir"],
                    "initial": element.properties.get("expression_is_initial"),
                    "default": element.properties.get("expression_is_default"),
                });
                without_spans(&mut properties);
                (element.id.clone(), properties)
            }).collect()
    }

    #[test]
    fn release_authoring_preserves_feature_values_and_precedence() {
        let source = "package P { attribute a = (1 + 2) * 3; attribute b := 4; attribute c default = 5; attribute d default := 6; }";
        let library = shared_sysml_baseline().unwrap();
        let before = crate::compile_sysml_text(source, "values.sysml", &library).unwrap();
        let project = load_authoring_project_from_sysml(BTreeMap::from([("values.sysml".into(), source.into())])).unwrap();
        let rendered = project.render_new_file("values.sysml").unwrap();
        let after = crate::compile_sysml_text(&rendered, "values.sysml", &library).unwrap();
        assert_eq!(semantic_expression_properties(&before), semantic_expression_properties(&after), "{rendered}");
        assert!(rendered.contains("default :="), "{rendered}");
    }

    #[test]
    fn release_authoring_preserves_lambda_parameters() {
        let source = "package P { attribute def Number; attribute data = (1, 2); attribute result = data.?{in ref 'odd item': Number[1] default := 2; 'odd item' > 1}; }";
        let library = shared_sysml_baseline().unwrap();
        let before = crate::compile_sysml_text(source, "lambda.sysml", &library).unwrap();
        let project = load_authoring_project_from_sysml(BTreeMap::from([("lambda.sysml".into(), source.into())])).unwrap();
        let rendered = project.render_new_file("lambda.sysml").unwrap();
        let after = crate::compile_sysml_text(&rendered, "lambda.sysml", &library).unwrap();
        assert_eq!(semantic_expression_properties(&before), semantic_expression_properties(&after), "{rendered}");
    }

    #[test]
    fn release_authoring_from_kir_preserves_structured_values() {
        let source = "package P { attribute a = (1 + 2) * 3; attribute b := 4; attribute c default = 5; attribute d default := 6; attribute def Number; attribute data = (1, 2); attribute result = data.?{in ref 'odd item': Number[1] default := 2; 'odd item' > 1}; }";
        let library = shared_sysml_baseline().unwrap();
        let before = crate::compile_sysml_text(source, "values.sysml", &library).unwrap();
        let project = AuthoringProject::from_kir_document(&before).unwrap();
        let rendered = project.files().map(|(path, _)| project.render_new_file(path).unwrap()).collect::<Vec<_>>().join("\n");
        let after = crate::compile_sysml_text(&rendered, "values.sysml", &library).unwrap();
        assert_eq!(semantic_expression_properties(&before), semantic_expression_properties(&after), "{rendered}");
    }

    #[test]
    fn release_authoring_preserves_multiple_types() {
        let source = "package P { part def A; part def B; part p: A, B; }";
        let project = load_authoring_project_from_sysml(BTreeMap::from([("types.sysml".into(), source.into())])).unwrap();
        let rendered = project.render_new_file("types.sysml").unwrap();
        assert!(rendered.contains("p: A, B"), "{rendered}");
        let library = shared_sysml_baseline().unwrap();
        let before = crate::compile_sysml_text(source, "types.sysml", &library).unwrap();
        let after = crate::compile_sysml_text(&rendered, "types.sysml", &library).unwrap();
        let types = |document: &KirDocument| document.elements.iter().find(|e| e.id == "feature.P.p").unwrap().properties["type"].clone();
        assert_eq!(types(&before), types(&after));
    }


    #[test]
    fn release_authoring_from_kir_preserves_multiple_types_and_quoted_members() {
        let source = "package P { part def A { attribute 'odd value'; } part def B; part data: A, B; attribute result = data.{in item: A; item.'odd value'}; }";
        let library = shared_sysml_baseline().unwrap();
        let before = crate::compile_sysml_text(source, "types.sysml", &library).unwrap();
        let project = AuthoringProject::from_kir_document(&before).unwrap();
        let rendered = project.files().map(|(path, _)| project.render_new_file(path).unwrap()).collect::<Vec<_>>().join("\n");
        let after = crate::compile_sysml_text(&rendered, "types.sysml", &library).unwrap();
        assert_eq!(semantic_expression_properties(&before), semantic_expression_properties(&after), "{rendered}");
        let types = |document: &KirDocument| {
            let value = &document.elements.iter().find(|e| e.id == "feature.P.data").unwrap().properties["type"];
            value.as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect::<std::collections::BTreeSet<_>>()
        };
        let expected = std::collections::BTreeSet::from(["type.P.A".to_string(), "type.P.B".to_string()]);
        assert_eq!(types(&before), expected);
        assert_eq!(types(&after), expected, "{rendered}");
    }

    #[test]
    fn release_authoring_from_kir_preserves_library_package_prefixes() {
        let source = "standard library package L { library package Nested; package Plain; }";
        let library = shared_sysml_baseline().unwrap();
        let before = crate::compile_sysml_text(source, "library.sysml", &library).unwrap();
        let project = AuthoringProject::from_kir_document(&before).unwrap();
        let rendered = project.files().map(|(path, _)| project.render_new_file(path).unwrap()).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains("standard library package L"), "{rendered}");
        let after = crate::compile_sysml_text(&rendered, "library.sysml", &library).unwrap();
        for id in ["pkg.L", "pkg.L.Nested", "pkg.L.Plain"] {
            let left = before.elements.iter().find(|element| element.id == id).unwrap();
            let right = after.elements.iter().find(|element| element.id == id).unwrap();
            assert_eq!(left.kind, right.kind, "{id}: {rendered}");
            assert_eq!(left.properties.get("is_standard"), right.properties.get("is_standard"), "{id}: {rendered}");
        }
    }

    #[test]
    fn release_authoring_preserves_included_use_cases() {
        let source = "package P { use case def U; use case target: U; use case def Main { include use case child: U; include P::target; } }";
        let library = shared_sysml_baseline().unwrap();
        let before = crate::compile_sysml_text(source, "include.sysml", &library).unwrap();
        let from_source = load_authoring_project_from_sysml(BTreeMap::from([("include.sysml".into(), source.into())])).unwrap();
        let from_kir = AuthoringProject::from_kir_document(&before).unwrap();
        for project in [from_source, from_kir] {
            let rendered = project.files().map(|(path, _)| project.render_new_file(path).unwrap()).collect::<Vec<_>>().join("\n");
            let after = crate::compile_sysml_text(&rendered, "include.sysml", &library).unwrap_or_else(|error| panic!("{error}: {rendered}"));
            assert_eq!(after.elements.iter().filter(|e| e.kind.ends_with("IncludeUseCaseUsage")).count(), 2, "{rendered}");
            let relationship = after.elements.iter().find(|e| e.kind.ends_with("ReferenceSubsetting")).unwrap();
            assert_eq!(relationship.properties["referenced_feature"], "use-case.P.target", "{rendered}");
        }
    }

    #[test]
    fn release_authoring_preserves_ordered_nonunique_collections() {
        let source = "package P { attribute values[*] ordered nonunique; }";
        let library = shared_sysml_baseline().unwrap();
        let before = crate::compile_sysml_text(source, "flags.sysml", &library).unwrap();
        let from_source = load_authoring_project_from_sysml(BTreeMap::from([("flags.sysml".into(), source.into())])).unwrap();
        let from_kir = AuthoringProject::from_kir_document(&before).unwrap();
        for project in [from_source, from_kir] {
            let rendered = project.files().map(|(path, _)| project.render_new_file(path).unwrap()).collect::<Vec<_>>().join("\n");
            let after = crate::compile_sysml_text(&rendered, "flags.sysml", &library).unwrap_or_else(|error| panic!("{error}: {rendered}"));
            let values = after.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "values")).unwrap();
            assert_eq!(values.properties["is_ordered"], true, "{rendered}");
            assert_eq!(values.properties["is_unique"], false, "{rendered}");
        }
    }

    #[test]
    fn release_authoring_preserves_variation_and_abstract_declarations() {
        let source = "package P { variation action def Choice; abstract part def Shape; variation use case choices; }";
        let library = shared_sysml_baseline().unwrap();
        let before = crate::compile_sysml_text(source, "variation.sysml", &library).unwrap();
        let project = AuthoringProject::from_kir_document(&before).unwrap();
        let rendered = project.files().map(|(path, _)| project.render_new_file(path).unwrap()).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains("variation action def Choice"), "{rendered}");
        assert!(rendered.contains("abstract part def Shape"), "{rendered}");
        assert!(rendered.contains("variation use case choices"), "{rendered}");
        let after = crate::compile_sysml_text(&rendered, "variation.sysml", &library).unwrap();
        for name in ["Choice", "Shape", "choices"] {
            let left = before.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
            let right = after.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
            for key in ["is_abstract", "is_variation"] {
                assert_eq!(left.properties.get(key), right.properties.get(key), "{name}.{key}: {rendered}");
            }
        }
    }

}
