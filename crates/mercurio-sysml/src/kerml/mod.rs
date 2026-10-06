//! KerML language facade.
//!
//! This crate is the public KerML-facing boundary while the parser/compiler
//! implementation is still hosted in `mercurio-foundation`. Keep this surface narrow:
//! parsing KerML, compiling KerML to KIR, and loading the KerML/Kernel baseline.

pub mod compiler;
mod flow;
mod multiplicity;
pub mod parser;
pub use flow::{derived_flow_end_ids, derived_flow_payload_feature_id, derived_flow_payload_type_ids, derived_flow_value_expression_id};
pub use multiplicity::{LiteralBounds, derived_bound_value, derived_has_bounds, derived_literal_bounds};

pub use crate::language_frontend::SourceLanguage;
pub use compiler::{
    BaselineLibrary, KermlError, KermlLanguageModule, compile_kerml_module,
    compile_kerml_module_strict_with_context, compile_kerml_module_with_context,
    compile_kerml_module_with_resolver_context, compile_kerml_text,
    compile_kerml_text_with_context, compile_kerml_text_with_empty_context, compile_text,
    compile_text_with_context, default_kernel_library_path, load_kerml_document,
    load_kerml_document_with_stdlib, load_kernel_baseline,
};
pub use mercurio_foundation::kir::{KirDocument, KirError};
pub use mercurio_foundation::language_contracts::Concept;
pub use mercurio_foundation::language_contracts::ast::{ParsedModule, QualifiedName, SourceSpan};
pub use mercurio_foundation::language_contracts::diagnostics::Diagnostic;
pub use mercurio_foundation::language_contracts::service::{CompileContext, LanguageService};
pub use parser::{parse, parse_kerml};

impl LanguageService for KermlLanguageModule {
    fn language_id(&self) -> &str {
        "kerml"
    }

    fn extensions(&self) -> &[&str] {
        &["kerml"]
    }

    fn compile(
        &self,
        source: &str,
        context: CompileContext<'_>,
    ) -> mercurio_foundation::language_contracts::SemanticCompileReport<KirDocument> {
        match compile_text(source, context.source_name, context.library_context) {
            Ok(document) => mercurio_foundation::language_contracts::SemanticCompileReport {
                status: mercurio_foundation::language_contracts::SemanticCompileStatus::Ok,
                diagnostics: Vec::new(),
                document: Some(document),
            },
            Err(diagnostic) => mercurio_foundation::language_contracts::SemanticCompileReport {
                status: mercurio_foundation::language_contracts::SemanticCompileStatus::Failed,
                diagnostics: vec![diagnostic],
                document: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mercurio_foundation::language_contracts::LanguageRegistry;
    use std::path::Path;

    #[test]
    fn direct_disjoining_preserves_endpoints_and_relationship_body_ownership() {
        let source = "package P { class A; class B; dependency d from A to B { disjoining D disjoint A from B; } disjoint A from B; }";
        let module = parse_kerml(source).unwrap();
        let library = load_kernel_baseline().unwrap();
        let document = compile_kerml_module_strict_with_context(
            &module, "disjoining.kerml", std::slice::from_ref(&module), &library,
        ).unwrap();
        let disjoinings = document.elements.iter().filter(|element|
            element.kind.rsplit("::").next() == Some("Disjoining")
        ).collect::<Vec<_>>();
        assert_eq!(disjoinings.len(), 2);
        for relation in &disjoinings {
            assert_eq!(relation.properties["type_disjoined"], "type.P.A");
            assert_eq!(relation.properties["disjoining_type"], "type.P.B");
            assert_eq!(relation.properties["source"], serde_json::json!(["type.P.A"]));
            assert_eq!(relation.properties["target"], serde_json::json!(["type.P.B"]));
        }
        let nested = disjoinings.iter().find(|relation| relation.properties["declared_name"] == "D").unwrap();
        let owner = document.elements.iter().find(|element|
            element.kind.rsplit("::").next() == Some("Dependency")
        ).unwrap();
        assert!(owner.properties["owned_related_element"].as_array().unwrap().contains(&serde_json::json!(nested.id)));
        assert_eq!(nested.properties["owning_relationship"], owner.id);
        assert!(!nested.properties.contains_key("owner"));
    }

    #[test]
    fn relationship_body_namespace_preserves_named_and_anonymous_objects() {
        let source = "package P { class A; dependency d from A to A { namespace N { class B; alias BAlias for B; import P::A; } namespace { class C; } } }";
        let module = parse_kerml(source).unwrap();
        let library = load_kernel_baseline().unwrap();
        let document = compile_kerml_module_strict_with_context(
            &module, "namespace-body.kerml", std::slice::from_ref(&module), &library,
        ).unwrap();
        let owner = document.elements.iter().find(|element|
            element.kind.rsplit("::").next() == Some("Dependency")
        ).unwrap();
        let namespaces = document.elements.iter().filter(|element|
            element.kind.rsplit("::").next() == Some("Namespace")
                && element.properties.get("owning_relationship") == Some(&serde_json::json!(owner.id))
        ).collect::<Vec<_>>();
        assert_eq!(namespaces.len(), 2);
        assert_eq!(namespaces.iter().filter(|element| element.properties.get("declared_name").is_some()).count(), 1);
        for namespace in namespaces {
            assert!(owner.properties["owned_related_element"].as_array().unwrap().contains(&serde_json::json!(namespace.id)));
        }
        let named = document.elements.iter().find(|element| element.properties.get("declared_name") == Some(&serde_json::json!("N"))).unwrap();
        let member = document.elements.iter().find(|element| element.properties.get("declared_name") == Some(&serde_json::json!("B"))).unwrap();
        let alias = document.elements.iter().find(|element| element.properties.get("member_name") == Some(&serde_json::json!("BAlias"))).unwrap();
        assert_eq!(alias.properties["member_element"], member.id);
        assert_eq!(alias.properties["membership_owning_namespace"], named.id);
        let imported = document.elements.iter().find(|element| element.properties.get("import_owning_namespace") == Some(&serde_json::json!(named.id))).unwrap();
        assert_eq!(imported.properties["imports"], serde_json::json!(["type.P.A"]));
        for name in ["B", "C"] {
            assert!(document.elements.iter().any(|element| element.properties.get("declared_name") == Some(&serde_json::json!(name))));
        }
        mercurio_foundation::KirDocument::merge_with_registered_fields(
            vec![document], crate::sysml_field_specs().iter().copied(),
        ).unwrap();
    }

    #[test]
    fn direct_multiplicity_subset_preserves_feature_subsetting() {
        let source = "package P { feature base { feature child; } alias Link for base { multiplicity m subsets P::base; multiplicity :> P::base; multiplicity chain subsets P::base.child; } }";
        let module = parse_kerml(source).unwrap();
        let library = load_kernel_baseline().unwrap();
        let document = compile_kerml_module_strict_with_context(
            &module, "multiplicity-subset.kerml", std::slice::from_ref(&module), &library,
        ).unwrap();
        let multiplicity = document.elements.iter().find(|element|
            element.kind.rsplit("::").next() == Some("Multiplicity")
                && element.properties.get("declared_name") == Some(&serde_json::json!("m"))
        ).unwrap();
        let base = document.elements.iter().find(|element|
            element.properties.get("declared_name") == Some(&serde_json::json!("base"))
        ).unwrap();
        assert_eq!(multiplicity.properties["subsetted_features"], serde_json::json!([base.id]));
        let owned = multiplicity.properties["owned_relationship"].as_array().unwrap();
        let subsetting = document.elements.iter().find(|element|
            element.kind.rsplit("::").next() == Some("Subsetting")
                && owned.contains(&serde_json::json!(element.id))
        ).unwrap();
        assert_eq!(subsetting.properties["subsetting_feature"], multiplicity.id);
        assert_eq!(subsetting.properties["subsetted_feature"], base.id);
        assert_eq!(subsetting.properties["owning_related_element"], multiplicity.id);
        assert_eq!(document.elements.iter().filter(|element|
            element.kind.rsplit("::").next() == Some("Multiplicity")
                && element.properties.get("subsetted_features") == Some(&serde_json::json!([base.id]))
        ).count(), 2);
        let owner_id = multiplicity.properties["owning_relationship"].as_str().unwrap();
        let owner = document.elements.iter().find(|element| element.id == owner_id).unwrap();
        assert!(owner.properties["owned_related_element"].as_array().unwrap().contains(&serde_json::json!(multiplicity.id)));
        let chain_multiplicity = document.elements.iter().find(|element|
            element.kind.rsplit("::").next() == Some("Multiplicity")
                && element.properties.get("declared_name") == Some(&serde_json::json!("chain"))
        ).unwrap();
        let chain_subsetting = document.elements.iter().find(|element|
            element.kind == "SysML::Subsetting"
                && element.properties.get("subsetting_feature") == Some(&serde_json::json!(chain_multiplicity.id))
        ).unwrap();
        let chain = document.elements.iter().find(|element| element.id == chain_subsetting.properties["subsetted_feature"]).unwrap();
        assert_eq!(chain.kind, "SysML::Feature");
        assert_eq!(chain.properties["owning_relationship"], chain_subsetting.id);
        let child = document.elements.iter().find(|element| element.properties.get("declared_name") == Some(&serde_json::json!("child"))).unwrap();
        assert_eq!(chain.properties["chaining_feature"], serde_json::json!([base.id, child.id]));
        for (index, id) in chain.properties["owned_relationship"].as_array().unwrap().iter().enumerate() {
            let link = document.elements.iter().find(|element| element.id == *id).unwrap();
            assert_eq!(link.kind, "SysML::FeatureChaining");
            assert_eq!(link.properties["chaining_feature"], chain.properties["chaining_feature"][index]);
            assert_eq!(link.properties["owning_related_element"], chain.id);
        }
        assert_eq!(chain_multiplicity.properties["subsetted_features"], serde_json::json!([chain.id]));
        assert!(chain_subsetting.properties["owned_related_element"].as_array().unwrap().contains(&serde_json::json!(chain.id)));
        mercurio_foundation::KirDocument::merge_with_registered_fields(
            vec![document], crate::sysml_field_specs().iter().copied(),
        ).unwrap();
        assert!(parse_kerml("package P { multiplicity m [1+2]; }").is_err());
        let overflow = parse_kerml("package P { multiplicity tooLarge [2147483648]; }").unwrap();
        assert!(compile_kerml_module_strict_with_context(
            &overflow, "overflow-bound.kerml", std::slice::from_ref(&overflow), &library,
        ).is_err());
        let invalid = source.replace("P::base.child", "P::base.missing");
        let invalid = parse_kerml(&invalid).unwrap();
        assert!(compile_kerml_module_strict_with_context(
            &invalid, "invalid-subset-chain.kerml", std::slice::from_ref(&invalid), &library,
        ).is_err());
    }

    #[test]
    fn direct_multiplicity_range_owns_ordered_literal_bounds() {
        let source = "package P { feature base; alias Link for base { multiplicity m [1..2] { feature part; } multiplicity n [2]; multiplicity [*]; } }";
        let module = parse_kerml(source).unwrap();
        let library = load_kernel_baseline().unwrap();
        let document = compile_kerml_module_strict_with_context(
            &module, "multiplicity-range.kerml", std::slice::from_ref(&module), &library,
        ).unwrap();
        let ranges = document.elements.iter().filter(|element|
            element.kind.rsplit("::").next() == Some("MultiplicityRange")
        ).collect::<Vec<_>>();
        assert_eq!(ranges.len(), 3);
        let named = ranges.iter().find(|range| range.properties.get("declared_name") == Some(&serde_json::json!("m"))).unwrap();
        assert_eq!(named.properties["multiplicity_lower"], "1");
        assert_eq!(named.properties["multiplicity_upper"], "2");
        let derived = derived_literal_bounds(named, &document.elements).unwrap();
        let bounds = &derived.bound;
        assert_eq!(bounds.len(), 2);
        assert_eq!(named.properties["members"].as_array().unwrap()[..2], [serde_json::json!(bounds[0]), serde_json::json!(bounds[1])]);
        assert!(named.properties["members"].as_array().unwrap().len() > 2);
        assert_eq!(derived.lower_bound.as_deref(), Some(bounds[0].as_str()));
        assert_eq!(derived.upper_bound, bounds[1]);
        assert_eq!(derived_bound_value(named, Some(&bounds[0]), &document.elements), 1);
        assert_eq!(derived_bound_value(named, Some(&bounds[1]), &document.elements), 2);
        assert!(derived_has_bounds(named, 1, 2, &document.elements));
        assert!(!derived_has_bounds(named, 0, 2, &document.elements));
        for (index, value) in [1, 2].iter().enumerate() {
            let expression = document.elements.iter().find(|element| element.id == bounds[index]).unwrap();
            assert_eq!(expression.kind, "SysML::LiteralInteger");
            assert_eq!(expression.properties["value"], *value);
            let membership = document.elements.iter().find(|element| element.id == expression.properties["owning_membership"]).unwrap();
            assert_eq!(membership.properties["member_element"], expression.id);
            assert_eq!(membership.properties["owned_member_element"], expression.id);
            assert_eq!(membership.properties["membership_owning_namespace"], named.id);
            assert_eq!(membership.properties["owning_related_element"], named.id);
            assert!(named.properties["owned_relationship"].as_array().unwrap().contains(&serde_json::json!(membership.id)));
            assert!(named.properties["owned_membership"].as_array().unwrap().contains(&serde_json::json!(membership.id)));
            assert!(named.properties["membership"].as_array().unwrap().contains(&serde_json::json!(membership.id)));
        }
        let unbounded = ranges.iter().find(|range| range.properties.get("declared_name").is_none()).unwrap();
        let single = ranges.iter().find(|range| range.properties.get("declared_name") == Some(&serde_json::json!("n"))).unwrap();
        assert_eq!(single.properties["multiplicity_lower"], "2");
        assert_eq!(single.properties["multiplicity_upper"], "2");
        let derived_single = derived_literal_bounds(single, &document.elements).unwrap();
        assert_eq!(derived_single.bound.len(), 1);
        assert!(derived_single.lower_bound.is_none());
        assert_eq!(derived_single.upper_bound, derived_single.bound[0]);
        assert!(derived_has_bounds(single, 2, 2, &document.elements));
        assert_eq!(unbounded.properties["multiplicity_lower"], "0");
        assert_eq!(unbounded.properties["multiplicity_upper"], "*");
        let derived = derived_literal_bounds(unbounded, &document.elements).unwrap();
        assert!(derived.lower_bound.is_none());
        let infinity = document.elements.iter().find(|element| element.id == derived.upper_bound).unwrap();
        assert_eq!(infinity.kind, "SysML::LiteralInfinity");
        assert_eq!(derived_bound_value(unbounded, Some(&derived.upper_bound), &document.elements), -1);
        assert!(derived_has_bounds(unbounded, 0, -1, &document.elements));
        assert!(!derived_has_bounds(unbounded, 1, -1, &document.elements));
        mercurio_foundation::KirDocument::merge_with_registered_fields(
            vec![document], crate::sysml_field_specs().iter().copied(),
        ).unwrap();
        assert!(parse_kerml("package P { multiplicity m [1+2]; }").is_err());
    }

    #[test]
    fn multiplicity_range_reference_bound_links_feature_through_membership() {
        let source = "package P { feature count; multiplicity m [count]; multiplicity n [1..count]; multiplicity q [P::count]; }";
        let module = parse_kerml(source).unwrap();
        let library = load_kernel_baseline().unwrap();
        let document = compile_kerml_module_strict_with_context(
            &module, "reference-bounds.kerml", std::slice::from_ref(&module), &library,
        ).unwrap();
        let feature = document.elements.iter().find(|element|
            element.properties.get("declared_name") == Some(&serde_json::json!("count"))).unwrap();
        for (name, position) in [("m", 0), ("n", 1), ("q", 0)] {
            let range = document.elements.iter().find(|element|
                element.kind.rsplit("::").next() == Some("MultiplicityRange")
                    && element.properties.get("declared_name") == Some(&serde_json::json!(name))).unwrap();
            let bounds = derived_literal_bounds(range, &document.elements).unwrap();
            let expression = document.elements.iter().find(|element| element.id == bounds.bound[position]).unwrap();
            assert_eq!(expression.kind, "SysML::FeatureReferenceExpression");
            assert_eq!(derived_bound_value(range, Some(&expression.id), &document.elements), -2);
            let member_id = expression.properties["owned_relationship"][0].as_str().unwrap();
            let member = document.elements.iter().find(|element| element.id == member_id).unwrap();
            assert_eq!(member.kind, "SysML::Membership");
            assert_eq!(member.properties["member_element"], feature.id);
            assert_eq!(member.properties["membership_owning_namespace"], expression.id);
            assert_eq!(member.properties["owning_related_element"], expression.id);
        }
        mercurio_foundation::KirDocument::merge_with_registered_fields(
            vec![document], crate::sysml_field_specs().iter().copied(),
        ).unwrap();
        assert!(parse_kerml("package P { multiplicity bad [1 + 2]; }").is_err());
        let unresolved = parse_kerml("package P { multiplicity bad [missing]; }").unwrap();
        assert!(compile_kerml_module_strict_with_context(
            &unresolved, "unresolved-reference-bound.kerml", std::slice::from_ref(&unresolved), &library,
        ).is_err());
    }

    #[test]
    fn flow_and_succession_flow_end_graphs_preserve_metaclass_and_owner() {
        let source = "package P { class A; class Fuel; feature a { feature child { feature grand; } } feature b { feature child { feature grand; } } dependency d from A to A { flow f { class Nested; } succession flow sf; flow; flow linked from P::a to P::b; succession flow next from P::a to P::b; flow qualified from P::a.child to P::b.child; flow deep from P::a.child.grand to P::b.child.grand; flow all P::a to P::b; flow P::a to P::b; flow carrying of item : P::Fuel from P::a to P::b; flow cargo of : P::Fuel; flow valued = 12; flow initial := 3; flow defaulted default = true; } }";
        let module = parse_kerml(source).unwrap();
        let library = load_kernel_baseline().unwrap();
        let document = compile_kerml_module_strict_with_context(
            &module, "bare-flows.kerml", std::slice::from_ref(&module), &library,
        ).unwrap();
        let owner = document.elements.iter().find(|element|
            element.kind.rsplit("::").next() == Some("Dependency") && element.properties.get("declared_name") == Some(&serde_json::json!("d"))
        ).unwrap();
        let flows = document.elements.iter().filter(|element|
            matches!(element.kind.as_str(), "Flow" | "SuccessionFlow")
        ).collect::<Vec<_>>();
        assert_eq!(flows.len(), 14);
        assert_eq!(flows.iter().filter(|flow| flow.kind == "Flow" && flow.properties["metatype"] == "SysML::Flow").count(), 12);
        assert_eq!(flows.iter().filter(|flow| flow.kind == "SuccessionFlow" && flow.properties["metatype"] == "SysML::SuccessionFlow").count(), 2);
        for flow in &flows {
            assert_eq!(flow.properties["owning_relationship"], owner.id);
            assert!(owner.properties["owned_related_element"].as_array().unwrap().contains(&serde_json::json!(flow.id)));
        }
        let sufficient = flows.iter().filter(|flow| flow.properties.get("is_sufficient") == Some(&serde_json::json!(true))).collect::<Vec<_>>();
        assert_eq!(sufficient.len(), 1);
        assert!(flows.iter().filter(|flow| flow.properties["is_sufficient"] == false).count() == 13);
        assert_eq!(derived_flow_end_ids(sufficient[0], &document.elements).len(), 2);
        let named = flows.iter().find(|flow| flow.properties.get("declared_name") == Some(&serde_json::json!("f"))).unwrap();
        let nested = document.elements.iter().find(|element| element.properties.get("declared_name") == Some(&serde_json::json!("Nested"))).unwrap();
        assert!(named.properties["members"].as_array().unwrap().contains(&serde_json::json!(nested.id)));
        let a = document.elements.iter().find(|element| element.properties.get("declared_name") == Some(&serde_json::json!("a"))).unwrap();
        let b = document.elements.iter().find(|element| element.properties.get("declared_name") == Some(&serde_json::json!("b"))).unwrap();
        let fuel = document.elements.iter().find(|element| element.properties.get("declared_name") == Some(&serde_json::json!("Fuel"))).unwrap();
        for (name, expected, is_initial, is_default) in [
            ("valued", serde_json::json!(12), false, false),
            ("initial", serde_json::json!(3), true, false),
            ("defaulted", serde_json::json!(true), false, true),
        ] {
            let flow = flows.iter().find(|flow| flow.properties.get("declared_name") == Some(&serde_json::json!(name))).unwrap();
            let expression_id = derived_flow_value_expression_id(flow, &document.elements).unwrap();
            let expression = document.elements.iter().find(|element| element.id == expression_id).unwrap();
            assert_eq!(expression.properties["value"], expected);
            let relation_id = expression.properties["owning_relationship"].as_str().unwrap();
            let relation = document.elements.iter().find(|element| element.id == relation_id).unwrap();
            assert_eq!(relation.kind, "SysML::FeatureValue");
            assert_eq!(relation.properties["membership_owning_namespace"], flow.id);
            assert_eq!(relation.properties["member_element"], expression.id);
            assert_eq!(relation.properties["owned_member_element"], expression.id);
            assert_eq!(expression.properties["owning_membership"], relation.id);
            assert_eq!(expression.properties["owning_namespace"], flow.id);
            assert!(flow.properties["owned_membership"].as_array().unwrap().contains(&serde_json::json!(relation.id)));
            assert!(flow.properties["membership"].as_array().unwrap().contains(&serde_json::json!(relation.id)));
            assert!(flow.properties["members"].as_array().unwrap().contains(&serde_json::json!(expression.id)));
            assert_eq!(relation.properties["feature_with_value"], flow.id);
            assert_eq!(relation.properties["is_initial"], is_initial);
            assert_eq!(relation.properties["is_default"], is_default);
        }
        for (name, payload_name) in [("carrying", Some("item")), ("cargo", None)] {
            let flow = flows.iter().find(|flow| flow.properties.get("declared_name") == Some(&serde_json::json!(name))).unwrap();
            let payload_id = derived_flow_payload_feature_id(flow, &document.elements).unwrap();
            let payload = document.elements.iter().find(|element| element.id == payload_id).unwrap();
            assert_eq!(payload.kind, "SysML::PayloadFeature");
            assert_eq!(payload.properties.get("declared_name").and_then(|v| v.as_str()), payload_name);
            assert_eq!(derived_flow_payload_type_ids(flow, &document.elements), vec![fuel.id.clone()]);
            let typing_id = payload.properties["owned_relationship"][0].as_str().unwrap();
            let typing = document.elements.iter().find(|element| element.id == typing_id).unwrap();
            assert_eq!(typing.kind, "SysML::FeatureTyping");
            assert_eq!(typing.properties["typed_feature"], payload.id);
            assert_eq!(typing.properties["type"], fuel.id);
        }
        for name in ["linked", "next"] {
            let linked = flows.iter().find(|flow| flow.properties.get("declared_name") == Some(&serde_json::json!(name))).unwrap();
            let ends = derived_flow_end_ids(linked, &document.elements);
            assert_eq!(ends.len(), 2);
            for (end_id, target) in ends.iter().zip([a.id.as_str(), b.id.as_str()]) {
                let end = document.elements.iter().find(|element| &element.id == end_id).unwrap();
                let feature_membership_id = end.properties["owned_relationship"][0].as_str().unwrap();
                let feature_membership = document.elements.iter().find(|element| element.id == feature_membership_id).unwrap();
                assert_eq!(feature_membership.kind, "SysML::FeatureMembership");
                let feature_id = feature_membership.properties["member_element"].as_str().unwrap();
                let feature = document.elements.iter().find(|element| element.id == feature_id).unwrap();
                let relation_id = feature.properties["owned_relationship"][0].as_str().unwrap();
                let relation = document.elements.iter().find(|element| element.id == relation_id).unwrap();
                assert_eq!(relation.kind, "SysML::Redefinition");
                assert_eq!(relation.properties["redefined_feature"], target);
            }
        }
        let qualified = flows.iter().find(|flow| flow.properties.get("declared_name") == Some(&serde_json::json!("qualified"))).unwrap();
        let ends = derived_flow_end_ids(qualified, &document.elements);
        assert_eq!(ends.len(), 2);
        for (end_id, prefix, child_suffix) in ends.iter().zip([a.id.as_str(), b.id.as_str()]).zip(["a.child", "b.child"]).map(|((end,prefix),suffix)| (end,prefix,suffix)) {
            let end = document.elements.iter().find(|element| &element.id == end_id).unwrap();
            let reference_id = end.properties["owned_relationship"][0].as_str().unwrap();
            let reference = document.elements.iter().find(|element| element.id == reference_id).unwrap();
            assert_eq!(reference.kind, "SysML::ReferenceSubsetting");
            assert_eq!(reference.properties["referenced_feature"], prefix);
            let feature_membership_id = end.properties["owned_relationship"][1].as_str().unwrap();
            let feature_membership = document.elements.iter().find(|element| element.id == feature_membership_id).unwrap();
            let flow_feature_id = feature_membership.properties["member_element"].as_str().unwrap();
            let flow_feature = document.elements.iter().find(|element| element.id == flow_feature_id).unwrap();
            let redef_id = flow_feature.properties["owned_relationship"][0].as_str().unwrap();
            let redef = document.elements.iter().find(|element| element.id == redef_id).unwrap();
            assert!(redef.properties["redefined_feature"].as_str().unwrap().ends_with(child_suffix));
        }
        let deep = flows.iter().find(|flow| flow.properties.get("declared_name") == Some(&serde_json::json!("deep"))).unwrap();
        let ends = derived_flow_end_ids(deep, &document.elements);
        assert_eq!(ends.len(), 2);
        for (end_id, prefix_suffix) in ends.iter().zip(["a.child", "b.child"]) {
            let end = document.elements.iter().find(|element| &element.id == end_id).unwrap();
            let reference_id = end.properties["owned_reference_subsetting"].as_str().unwrap();
            let reference = document.elements.iter().find(|element| element.id == reference_id).unwrap();
            let prefix_id = reference.properties["referenced_feature"].as_str().unwrap();
            let prefix = document.elements.iter().find(|element| element.id == prefix_id).unwrap();
            assert_eq!(prefix.kind, "SysML::Feature");
            assert_eq!(prefix.properties["owning_relationship"], reference.id);
            assert!(reference.properties["owned_related_element"].as_array().unwrap().contains(&serde_json::json!(prefix.id)));
            assert_eq!(prefix.properties["chaining_feature"].as_array().unwrap().len(), 2);
            assert!(prefix.properties["chaining_feature"][1].as_str().unwrap().ends_with(prefix_suffix));
            assert_eq!(prefix.properties["owned_relationship"].as_array().unwrap().len(), 2);
        }
        mercurio_foundation::KirDocument::merge_with_registered_fields(
            vec![document], crate::sysml_field_specs().iter().copied(),
        ).unwrap();
        assert!(parse_kerml("package P { flow f of Fuel from a to b; }").is_ok());
        assert!(parse_kerml("package P { flow f from a to; }").is_err());
        let unresolved = parse_kerml("package P { flow f of : Missing; }").unwrap();
        assert!(compile_kerml_module_strict_with_context(
            &unresolved, "unresolved-payload.kerml", std::slice::from_ref(&unresolved), &library,
        ).is_err());
        let nonliteral = parse_kerml("package P { flow f = 1 + 2; }").unwrap();
        assert!(compile_kerml_module_strict_with_context(
            &nonliteral, "nonliteral-flow-value.kerml", std::slice::from_ref(&nonliteral), &library,
        ).is_err());
    }

    #[test]
    fn flow_payload_multiplicity_reuses_owned_range_graph_in_grammar_order() {
        let source = "package P { class Fuel; feature count; flow named of item : P::Fuel [1..2]; flow direct of P::Fuel [2]; flow before of [0..1] P::Fuel; flow namedBefore of item [1..2] : P::Fuel; flow anonymousBefore of [0..1] : P::Fuel; flow reference of : P::Fuel [count]; }";
        let module = parse_kerml(source).unwrap();
        let library = load_kernel_baseline().unwrap();
        let document = compile_kerml_module_strict_with_context(
            &module, "payload-multiplicity.kerml", std::slice::from_ref(&module), &library,
        ).unwrap();
        for (name, expected_lower, expected_upper, multiplicity_first) in [
            ("named", "1", "2", false), ("direct", "2", "2", false),
            ("before", "0", "1", true), ("namedBefore", "1", "2", true),
            ("anonymousBefore", "0", "1", true), ("reference", "count", "count", false),
        ] {
            let flow = document.elements.iter().find(|element|
                element.kind.rsplit("::").next() == Some("Flow")
                    && element.properties.get("declared_name") == Some(&serde_json::json!(name))).unwrap();
            let payload_id = derived_flow_payload_feature_id(flow, &document.elements).unwrap();
            let payload = document.elements.iter().find(|element| element.id == payload_id).unwrap();
            if name == "namedBefore" { assert_eq!(payload.properties["declared_name"], "item"); }
            let range = document.elements.iter().find(|element|
                element.kind == "SysML::MultiplicityRange"
                    && element.properties.get("owner") == Some(&serde_json::json!(payload.id))).unwrap();
            assert_eq!(range.properties["multiplicity_lower"], expected_lower);
            assert_eq!(range.properties["multiplicity_upper"], expected_upper);
            let range_membership_id = range.properties["owning_membership"].as_str().unwrap();
            let range_membership = document.elements.iter().find(|element| element.id == range_membership_id).unwrap();
            assert_eq!(range_membership.kind, "SysML::OwningMembership");
            assert_eq!(range_membership.properties["membership_owning_namespace"], payload.id);
            assert_eq!(range_membership.properties["owned_member_element"], range.id);
            let owned = payload.properties["owned_relationship"].as_array().unwrap();
            let range_position = owned.iter().position(|id| id == range_membership_id).unwrap();
            let typing_position = owned.iter().position(|id| id.as_str().is_some_and(|id| id.ends_with(".typing"))).unwrap();
            assert_eq!(range_position < typing_position, multiplicity_first);
            assert!(derived_literal_bounds(range, &document.elements).is_some());
        }
        mercurio_foundation::KirDocument::merge_with_registered_fields(
            vec![document], crate::sysml_field_specs().iter().copied(),
        ).unwrap();
        assert!(parse_kerml("package P { class Fuel; flow bad of [1] P::Fuel [2]; }").is_err());
        let unresolved = parse_kerml("package P { class Fuel; flow bad of : P::Fuel [missing]; }").unwrap();
        assert!(compile_kerml_module_strict_with_context(
            &unresolved, "unresolved-payload-bound.kerml", std::slice::from_ref(&unresolved), &library,
        ).is_err());
    }

    #[test]
    fn multiplicity_literal_syntax_reaches_pilot_natural_value_check() {
        let library = load_kernel_baseline().unwrap();
        for literal in ["true", "false", "1.5", "\"x y\"", "\"x..y\""] {
            let source = format!("package P {{ multiplicity m [{literal}]; }}");
            let module = parse_kerml(&source).unwrap();
            let error = compile_kerml_module_strict_with_context(
                &module, "multiplicity-literal.kerml", std::slice::from_ref(&module), &library,
            ).unwrap_err();
            assert!(error.message.contains("Must have a Natural value"), "{literal}: {error:?}");
        }
        let source = "package P { feature NaN; multiplicity m [NaN]; }";
        let module = parse_kerml(source).unwrap();
        let document = compile_kerml_module_strict_with_context(
            &module, "multiplicity-reference.kerml", std::slice::from_ref(&module), &library,
        ).unwrap();
        let reference = document.elements.iter().find(|element|
            element.kind == "SysML::FeatureReferenceExpression").unwrap();
        let member_id = reference.properties["owned_relationship"][0].as_str().unwrap();
        let member = document.elements.iter().find(|element| element.id == member_id).unwrap();
        assert_eq!(member.properties["member_element"], "feature.P.NaN");
    }

    #[test]
    fn facade_parses_minimal_kerml() {
        let module = parse("package Demo { classifier Vehicle; }").unwrap();

        assert!(module.package.is_some());
    }

    #[test]
    fn language_service_compiles_registered_kerml() {
        let mut registry = LanguageRegistry::new();
        registry.register(KermlLanguageModule);
        let library_context = KirDocument {
            metadata: Default::default(),
            elements: Vec::new(),
        };

        let report = registry.compile_path(
            Path::new("demo.kerml"),
            "package Demo { classifier Vehicle; }",
            &library_context,
        );

        assert_eq!(
            report.status,
            mercurio_foundation::language_contracts::SemanticCompileStatus::Ok
        );
        assert!(report.document.is_some());
    }
}
