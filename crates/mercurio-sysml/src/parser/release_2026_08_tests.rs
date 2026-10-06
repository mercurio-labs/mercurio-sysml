//! Regressions grounded in the pinned 2026-08 Xtext grammar and sample corpus.
use super::*;

#[test]
fn release_2026_08_arrow_invocation_preserves_receiver_and_precedence() {
    let mut parser = Parser::new(lex("positions->including(position) + 1").unwrap(), false);
    let Expr::Binary {
        left,
        op: BinaryOp::Add,
        ..
    } = parser.parse_expression().unwrap()
    else {
        panic!("expected addition")
    };
    let Expr::Call { function, args, .. } = *left else {
        panic!("expected invocation")
    };
    assert_eq!(function, "including");
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0], Expr::Name(n) if n.as_dot_string() == "positions"));
    assert!(matches!(&args[1], Expr::Name(n) if n.as_dot_string() == "position"));
    assert!(matches!(parser.peek_kind(), TokenKind::Eof));
}

#[test]
fn release_2026_08_transition_retains_typed_trigger_receiver_and_body() {
    let source = "transition first off accept cmd: Command via port if true then on { attribute n: ScalarValues::Integer; }";
    let mut parser = Parser::new(lex(source).unwrap(), false);
    let Declaration::GenericUsage(usage) = parser.parse_declaration().unwrap().unwrap() else {
        panic!("transition")
    };
    assert!(usage.is_implicit_name);
    assert!(
        usage
            .modifiers
            .contains(&"transition_source=off".to_string())
    );
    assert!(
        usage
            .modifiers
            .contains(&"transition_target=on".to_string())
    );
    let accepter = usage
        .body_members
        .iter()
        .find_map(|d| match d {
            Declaration::GenericUsage(u) if u.name == "accepter" => Some(u),
            _ => None,
        })
        .unwrap();
    assert!(accepter.body_members.iter().any(|d| matches!(d, Declaration::GenericUsage(u) if u.name == "cmd" && u.ty.as_ref().is_some_and(|ty| ty.as_dot_string() == "Command"))));
    assert!(accepter.body_members.iter().any(|d| matches!(d, Declaration::GenericUsage(u) if u.name == "receiver" && u.reference_target.as_ref().is_some_and(|r| r.as_dot_string() == "port"))));
    assert!(
        usage
            .body_members
            .iter()
            .any(|d| matches!(d, Declaration::GenericUsage(u) if u.name == "n"))
    );
}

#[test]
fn release_2026_08_flow_retains_named_payload_type() {
    let mut parser = Parser::new(
        lex("flow of fuel: Fuel from tank.outlet to engine.inlet;").unwrap(),
        false,
    );
    let Declaration::GenericUsage(usage) = parser.parse_declaration().unwrap().unwrap() else {
        panic!("flow")
    };
    assert!(usage.body_members.iter().any(|d| matches!(d, Declaration::GenericUsage(u) if u.name == "fuel" && u.ty.as_ref().is_some_and(|ty| ty.as_dot_string() == "Fuel"))));
}

#[test]
fn release_2026_08_lexical_imports_survive_support_file_name_collisions() {
    let source = parse_sysml("package Model { private import Left::*; part def Car :> Vehicle { attribute :>> mass = 1; } }").unwrap();
    let left = parse_sysml("package Left { private import Types::*; part def Vehicle :> Base; } package Types { part def Base { attribute mass: ScalarValues::Integer; } }").unwrap();
    let right = parse_sysml("package Right { part def Vehicle; part def Base; }").unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = compile_sysml_module_with_context(
        &source,
        "scope.sysml",
        &[source.clone(), left, right],
        &library,
    )
    .unwrap();
    assert!(kir.elements.iter().any(|e| {
        e.properties
            .get("redefined_features")
            .is_some_and(|v| v.to_string().contains("Types.Base.mass"))
    }));
}

#[test]
fn release_2026_08_inherited_chain_resolves_and_missing_member_still_fails() {
    let library = load_sysml_baseline().unwrap();
    let valid = "package P { part def Engine { port outlet; } part def Vehicle { part engine: Engine; } part v: Vehicle; part upgraded :> v; connect upgraded.engine.outlet to v.engine.outlet; }";
    let module = parse_sysml(valid).unwrap();
    assert!(compile_sysml_module(&module, "chain.sysml", &library).is_ok());
    let invalid =
        parse_sysml(&valid.replace("upgraded.engine.outlet", "upgraded.engine.missing")).unwrap();
    assert!(compile_sysml_module(&invalid, "invalid.sysml", &library).is_err());
}

#[test]
fn release_2026_08_metadata_retains_kerml_kind_and_type() {
    let module = crate::kerml::parse_kerml(
        "package P { metaclass Security; feature x { @ : Security; metadata tagged: Security; } }",
    )
    .unwrap();
    let library = crate::kerml::load_kernel_baseline().unwrap();
    let kir = crate::kerml::compile_kerml_module(&module, "metadata.kerml", &library).unwrap();
    let metadata = kir
        .elements
        .iter()
        .filter(|e| e.kind == "MetadataFeature")
        .collect::<Vec<_>>();
    assert_eq!(metadata.len(), 2);
    assert!(
        metadata
            .iter()
            .all(|e| e.properties["type"].to_string().contains("P.Security"))
    );
}

#[test]
fn release_2026_08_nested_bodies_fit_default_windows_main_stack() {
    // A deterministic small-stack regression, independent of the large corpus.
    let mut source = "package P {".to_string();
    for i in 0..8 {
        source.push_str(&format!("part p{i} {{"));
    }
    source.push_str("attribute value: ScalarValues::Integer;");
    source.push_str(&"}".repeat(9));
    assert!(
        std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(move || parse_sysml(&source).is_ok())
            .unwrap()
            .join()
            .unwrap()
    );
}

#[test]
fn release_2026_08_attribute_type_validation_matches_pilot_control() {
    let library = load_sysml_baseline().unwrap();
    let bad =
        parse_sysml("package Audit { part def Vehicle; attribute invalid: Vehicle; }").unwrap();
    let error = compile_sysml_module(&bad, "invalid-type.sysml", &library).unwrap_err();
    assert!(error.message.contains("validateAttributeUsageType_"));
    let good = parse_sysml("package Audit { attribute def Mass; attribute mass: Mass; attribute value: ScalarValues::Real; enum def Choice { one; two; } attribute choice: Choice; }").unwrap();
    compile_sysml_module(&good, "valid-type.sysml", &library).unwrap();
    let support = parse_sysml("package Types { part def Vehicle; }").unwrap();
    let bad = parse_sysml("package Audit { attribute invalid: Types::Vehicle; }").unwrap();
    assert!(
        compile_sysml_module_with_context(
            &bad,
            "cross-file.sysml",
            &[bad.clone(), support],
            &library
        )
        .is_err()
    );
}

#[test]
fn release_2026_08_control_actions_keep_metaclasses_and_operands() {
    let library = load_sysml_baseline().unwrap();
    let source = parse_sysml("package P { action a { action worker; then fork; then join; then merge; then decide; terminate worker; terminate worker; } }").unwrap();
    let kir = compile_sysml_module(&source, "controls.sysml", &library).unwrap();
    for kind in [
        "ForkNode",
        "JoinNode",
        "MergeNode",
        "DecisionNode",
        "TerminateActionUsage",
    ] {
        assert!(
            kir.elements.iter().any(|e| e.kind == kind),
            "missing {kind}"
        );
    }
    assert_eq!(
        kir.elements
            .iter()
            .filter(|e| e.kind == "TerminateActionUsage")
            .count(),
        2
    );
    assert_eq!(
        kir.elements
            .iter()
            .map(|e| &e.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        kir.elements.len()
    );
    let invalid = parse_sysml("package P { action a { terminate missing; } }").unwrap();
    assert!(compile_sysml_module(&invalid, "bad-terminate.sysml", &library).is_err());
}

#[test]
fn release_2026_08_named_accept_and_null_preserve_ast() {
    let mut parser = Parser::new(
        lex("action trigger accept scene: Scene via sensor;").unwrap(),
        false,
    );
    let Declaration::GenericUsage(usage) = parser.parse_declaration().unwrap().unwrap() else {
        panic!("accept")
    };
    assert_eq!(usage.keyword, "accept");
    assert_eq!(usage.name, "trigger");
    assert!(usage.body_members.iter().any(|m| matches!(m, Declaration::GenericUsage(u) if u.name == "scene" && u.ty.as_ref().is_some_and(|t| t.as_dot_string() == "Scene"))));
    assert!(usage.body_members.iter().any(|m| matches!(m, Declaration::GenericUsage(u) if u.name == "receiver" && u.reference_target.as_ref().is_some_and(|t| t.as_dot_string() == "sensor"))));
    for source in ["null", "()"] {
        let mut parser = Parser::new(lex(source).unwrap(), false);
        assert!(
            matches!(parser.parse_expression().unwrap(), Expr::Tuple { items, .. } if items.is_empty())
        );
    }
}

#[test]
fn release_2026_08_anonymous_relationships_do_not_replace_named_scopes() {
    let source = parse_sysml("package P { part def A { attribute x: ScalarValues::Integer; attribute def Nested; } part a: A; part b: A; bind a.x = b.x; bind a.x = 1; flow a.x to b.x; part c: A { attribute n: Nested; } }").unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = compile_sysml_module(&source, "anonymous.sysml", &library).unwrap();
    assert!(
        kir.elements
            .iter()
            .any(
                |e| e.properties.get("declared_name") == Some(&serde_json::json!("a"))
                    && e.properties["type"].to_string().contains("P.A")
            )
    );
    assert_eq!(
        kir.elements
            .iter()
            .map(|e| &e.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        kir.elements.len()
    );
    let invalid = parse_sysml("package P { part a; bind a.missing = 1; }").unwrap();
    assert!(compile_sysml_module(&invalid, "invalid-bind.sysml", &library).is_err());
}

#[test]
fn release_2026_08_public_reexports_resolve_with_conflicting_support_names() {
    let target =
        parse_sysml("package User { private import Facade::*; part thing: Thing; }").unwrap();
    let support = parse_sysml("package Types { part def Thing; } package Facade { public import Types::*; } package Unrelated { part def Thing; }").unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = compile_sysml_module_with_context(
        &target,
        "reexport.sysml",
        &[target.clone(), support],
        &library,
    )
    .unwrap();
    assert!(kir.elements.iter().any(|e| {
        e.properties
            .get("type")
            .is_some_and(|t| t.to_string().contains("Types.Thing"))
    }));
}

#[test]
fn release_2026_08_assignment_and_event_preserve_referenced_symbols() {
    let library = load_sysml_baseline().unwrap();
    let source = parse_sysml("package P { part counter { attribute count: ScalarValues::Integer; } action run { assign counter.count := 1; assign counter.count := 2; } event run; event run; }").unwrap();
    let kir = compile_sysml_module(&source, "assignment.sysml", &library).unwrap();
    assert_eq!(
        kir.elements
            .iter()
            .filter(|e| e.kind == "AssignmentActionUsage")
            .count(),
        2
    );
    assert_eq!(
        kir.elements
            .iter()
            .map(|e| &e.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        kir.elements.len()
    );
    let invalid =
        parse_sysml("package P { part counter; action run { assign counter.missing := 1; } }")
            .unwrap();
    assert!(compile_sysml_module(&invalid, "bad-assignment.sysml", &library).is_err());
}

#[test]
fn release_2026_08_message_payload_is_typed_and_addressable() {
    let library = load_sysml_baseline().unwrap();
    let source = parse_sysml("package P { item def Command; occurrence sender; occurrence receiver; message first of payload: Command from sender to receiver; message second of payload: Command = first.payload from receiver to sender; }").unwrap();
    let kir = compile_sysml_module(&source, "message.sysml", &library).unwrap();
    assert_eq!(
        kir.elements
            .iter()
            .filter(|e| e.kind == "FlowUsage")
            .count(),
        2
    );
    assert!(!kir.elements.iter().any(|e| e.kind == "MessageUsage"));
}

#[test]
fn release_2026_08_relative_public_reexport_keeps_declaration_scope() {
    let source = parse_sysml("package API { public import Definitions::*; package Definitions { part def Thing; } } package User { private import API::*; package Definitions { part def Thing; } part a: Thing; }").unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = compile_sysml_module(&source, "relative-import.sysml", &library).unwrap();
    let a = kir
        .elements
        .iter()
        .find(|e| e.id == "feature.User.a")
        .unwrap();
    assert_eq!(
        a.properties["type"],
        serde_json::json!("type.API.Definitions.Thing")
    );
}

#[test]
fn release_2026_08_transition_without_first_keeps_source_and_target() {
    let mut parser = Parser::new(lex("transition initial then off;").unwrap(), false);
    let Declaration::GenericUsage(usage) = parser.parse_declaration().unwrap().unwrap() else {
        panic!("transition");
    };
    assert!(usage.is_implicit_name);
    assert!(
        usage
            .modifiers
            .contains(&"transition_source=initial".into())
    );
    assert!(usage.modifiers.contains(&"transition_target=off".into()));
}

#[test]
fn release_2026_08_implicit_enum_members_validate_metadata_reference_properties() {
    let library = load_sysml_baseline().unwrap();
    let source = parse_sysml("package P { enum def Level { low; high; } metadata def Classified { ref level: Level; } part p { @Classified { level = Level::high; } } }").unwrap();
    let kir = compile_sysml_module(&source, "metadata-reference.sysml", &library).unwrap();
    assert!(
        kir.elements
            .iter()
            .any(|e| e.kind.ends_with("EnumerationUsage")
                && e.properties.get("declared_name") == Some(&serde_json::json!("high")))
    );
    let invalid = parse_sysml("package P { enum def Level { low; } metadata def Classified { ref level: Level; } part p { @Classified { level = Level::missing; } } }").unwrap();
    assert!(compile_sysml_module(&invalid, "bad-metadata-reference.sysml", &library).is_err());
}

#[test]
fn release_2026_08_structured_actions_keep_body_scopes_and_parameters() {
    let library = load_sysml_baseline().unwrap();
    let source = parse_sysml("package P { action a { attribute i: ScalarValues::Integer; if i > 0 { assign i := 0; } else if i == 0 { assign i := 1; } else { assign i := 2; } while i > 0 { assign i := i - 1; } for n: ScalarValues::Integer in (1, 2) { assign i := n; } } }").unwrap();
    let kir = compile_sysml_module(&source, "structured.sysml", &library).unwrap();
    for kind in [
        "IfActionUsage",
        "WhileLoopActionUsage",
        "ForLoopActionUsage",
    ] {
        assert!(kir.elements.iter().any(|e| e.kind == kind), "{kind}");
    }
    assert_eq!(
        kir.elements
            .iter()
            .filter(|e| e.kind == "AssignmentActionUsage")
            .count(),
        5
    );
    assert_eq!(
        kir.elements
            .iter()
            .map(|e| &e.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        kir.elements.len()
    );
    let invalid = parse_sysml("package P { action a { while missing { } } }").unwrap();
    assert!(compile_sysml_module(&invalid, "bad-loop.sysml", &library).is_err());
}

#[test]
fn release_2026_08_kerml_short_names_bind_in_imports_and_specializations() {
    let library = load_sysml_baseline().unwrap();
    let source = crate::kerml::parse_kerml("package Types { class <'2'> Named; } package P { private import Types::'2'; class C specializes Types::'2'; }").unwrap();
    let kir = crate::kerml::compile_kerml_module_strict_with_context(
        &source,
        "short-name.kerml",
        std::slice::from_ref(&source),
        &library,
    )
    .unwrap();
    assert!(kir.elements.iter().any(|e| e.id == "type.P.C"));
}

#[test]
fn release_2026_08_named_public_imports_are_addressable_through_facade() {
    let library = load_sysml_baseline().unwrap();
    let source = parse_sysml("package Types { part def A; } package Facade { public import Types::A; } package User { private import Facade::A; part a: Facade::A; }").unwrap();
    compile_sysml_module(&source, "named-reexport.sysml", &library).unwrap();
}

#[test]
fn release_2026_08_transition_text_retains_quoted_names_and_separates_effect() {
    let mut parser = Parser::new(lex("accept when sensor.temp > vehicle.maxTemperature do send new OverTemp() to controller then degraded;").unwrap(), false);
    let Declaration::GenericUsage(usage) = parser.parse_declaration().unwrap().unwrap() else {
        panic!("transition");
    };
    assert!(
        usage
            .modifiers
            .iter()
            .any(|m| m == "trigger=when sensor . temp > vehicle . maxTemperature")
    );
    assert!(usage.modifiers.iter().any(|m| m.starts_with("effect=send")));
    assert!(
        usage
            .modifiers
            .contains(&"transition_target=degraded".to_string())
    );
    let mut parser = Parser::new(lex("vehicle.'brake pedal depressed'").unwrap(), false);
    let text = parser.collect_behavior_text_until_do_then_or_end();
    assert!(text.contains("'brake pedal depressed'"));
    let mut reparsed = Parser::new(lex(&text).unwrap(), false);
    reparsed.parse_expression().unwrap();
    assert!(matches!(reparsed.peek_kind(), TokenKind::Eof));
}

#[test]
fn release_2026_08_multiplicity_reference_and_return_parameters_keep_names() {
    let library = load_sysml_baseline().unwrap();
    let source = parse_sysml("package P { item def Person; item def Country { ref president[1]: Person; } item country: Country { ref :>> president: Person; } calc def F { return attribute result: ScalarValues::Integer; } }").unwrap();
    let kir = compile_sysml_module(&source, "members.sysml", &library).unwrap();
    assert!(
        kir.elements
            .iter()
            .any(|e| e.properties.get("declared_name") == Some(&serde_json::json!("president")))
    );
    assert!(
        kir.elements
            .iter()
            .any(|e| e.properties.get("declared_name") == Some(&serde_json::json!("result")))
    );
}

#[test]
fn release_2026_08_member_access_preserves_invocation_and_rejects_missing_member() {
    let library = load_sysml_baseline().unwrap();
    let source = parse_sysml("package P { part def Counter { attribute count: ScalarValues::Integer; } part c: Counter; calc def Increment { in value: Counter; return : Counter; } attribute x = Increment(c).count; attribute y = new Counter().count; }").unwrap();
    let kir = compile_sysml_module(&source, "computed-member.sysml", &library).unwrap();
    let expressions = kir
        .elements
        .iter()
        .filter_map(|e| e.properties.get("expression_ir"))
        .collect::<Vec<_>>();
    assert_eq!(
        expressions.iter().filter(|e| e["kind"] == "select").count(),
        2
    );
    let invalid =
        parse_sysml("package P { part def Counter; attribute x = new Counter().missing; }")
            .unwrap();
    assert!(compile_sysml_module(&invalid, "bad-computed-member.sysml", &library).is_err());
}

#[test]
fn release_2026_08_local_specialization_redefinitions_and_perform_references() {
    let library = load_sysml_baseline().unwrap();
    for source in [
        "package P { part def Engine; part def Cylinder; part engine: Engine { part cylinders: Cylinder[4..8]; } part engine4:>engine { part redefines cylinders[4]; } }",
        "package P { action task { in part input; action child; } part p { perform task { in part :>> input; } perform task.child; } }",
        "package P { requirement def R; verification def V { objective { verify requirement : R; } } }",
        "package P { enum def Status { approved; } metadata def StatusHolder { status: Status; } part p { @StatusHolder { status = Status::approved; } } }",
    ] {
        let module = parse_sysml(source).unwrap();
        compile_sysml_module(&module, "references.sysml", &library)
            .unwrap_or_else(|error| panic!("{source}: {error}"));
    }
}

#[test]
fn release_2026_08_send_effect_retains_operands_and_transition_payload_scope() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { item def Signal { item value; } part receiver; state machine { state off; transition t first off accept pub: Signal do send pub.value to receiver then off; } event machine.t.effect; }";
    let module = parse_sysml(source).unwrap();
    let kir = compile_sysml_module(&module, "send.sysml", &library).unwrap();
    assert!(
        kir.elements
            .iter()
            .any(|e| e.kind == "SendActionUsage" && e.id.contains("effect"))
    );
    for invalid in [
        source.replace("pub.value", "pub.missing"),
        source.replace("to receiver", "to missing"),
    ] {
        let module = parse_sysml(&invalid).unwrap();
        assert!(compile_sysml_module(&module, "bad-send.sysml", &library).is_err());
    }
}

#[test]
fn release_2026_08_specialized_usage_members_precede_general_type_members() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { part def Vehicle; part vehicle: Vehicle { part output { attribute velocity: ScalarValues::Real; } } analysis def A { subject v: Vehicle; } analysis base: A { subject :>> v :> vehicle; } analysis def B :> A; analysis refined :> base : B { return speed = v.output.velocity; } }";
    compile_sysml_module(&parse_sysml(source).unwrap(), "precedence.sysml", &library).unwrap();
    assert!(
        compile_sysml_module(
            &parse_sysml(&source.replace("v.output.velocity", "v.output.missing")).unwrap(),
            "bad.sysml",
            &library
        )
        .is_err()
    );
}

#[test]
fn release_2026_08_aliases_survive_public_wildcard_reexports() {
    let library = load_sysml_baseline().unwrap();
    let source = "package Library { package Types { alias Amount for ScalarValues::Real; part def Engine; } public import Types::*; } package Client { private import Library::*; package Facade { public import Engine; } attribute amount: Amount; part engine: Facade::Engine; } package Other { alias Amount for ScalarValues::Integer; part def Engine; }";
    compile_sysml_module(&parse_sysml(source).unwrap(), "aliases.sysml", &library).unwrap();
}

#[test]
fn release_2026_08_kernel_concrete_metaclasses_and_occurrence_defaults() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { class C { feature redefines startShot; feature redefines endShot; } struct S; behavior B; datatype D; }";
    let module = crate::kerml::parse_kerml(source).unwrap();
    let kir = crate::kerml::compile_kerml_module_strict_with_context(
        &module,
        "kernel.kerml",
        std::slice::from_ref(&module),
        &library,
    )
    .unwrap();
    for kind in ["Class", "Structure", "Behavior", "DataType"] {
        assert!(
            kir.elements.iter().any(|e| e.kind == kind),
            "missing {kind}"
        );
    }
}

#[test]
fn release_2026_08_multiple_inheritance_uses_most_specific_member() {
    let library = load_sysml_baseline().unwrap();
    for parents in ["A, B", "B, A"] {
        let source = format!(
            "package P {{ classifier A {{ feature f; }} classifier B specializes A {{ feature redefines f {{ feature g; }} }} classifier C specializes {parents} {{ feature subsets f {{ feature redefines g; }} }} }}"
        );
        let module = crate::kerml::parse_kerml(&source).unwrap_or_else(|error| panic!("{source}: {error}"));
        crate::kerml::compile_kerml_module_strict_with_context(
            &module,
            "redefinition.kerml",
            std::slice::from_ref(&module),
            &library,
        )
        .unwrap();
    }
}

#[test]
fn release_2026_08_feature_types_are_visible_through_owning_usage_type() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { part def Vehicle { attribute def Output { attribute velocity: ScalarValues::Real; } } part vehicle: Vehicle { action behavior { out output: Output; } } attribute result = vehicle.behavior.output.velocity; }";
    compile_sysml_module(&parse_sysml(source).unwrap(), "nested-type.sysml", &library).unwrap();
    assert!(
        compile_sysml_module(
            &parse_sysml(&source.replace("output.velocity", "output.missing")).unwrap(),
            "bad.sysml",
            &library
        )
        .is_err()
    );
}

#[test]
fn release_2026_08_recursive_import_binds_nested_public_members() {
    let library = load_sysml_baseline().unwrap();
    let source = "package Library { package Nested { part def Engine; part engine: Engine; private part hidden; } } package Client { private import Library::**; part p: Engine :> engine; }";
    compile_sysml_module(&parse_sysml(source).unwrap(), "recursive.sysml", &library).unwrap();
    let invalid = source.replace("p: Engine :> engine", "p: Engine :> hidden");
    assert!(
        compile_sysml_module(&parse_sysml(&invalid).unwrap(), "private.sysml", &library).is_err()
    );
}

#[test]
fn release_2026_08_imports_follow_pilot_membership_order() {
    let library = load_sysml_baseline().unwrap();
    let source = "package A { package Types { part def Engine; } public import Types::*; } package B { part def Engine; } package C { private import A::*; private import B::*; part engine: Engine; } package R { part def Engine; package Nested { part def Engine; } } package S { private import R::**; part engine: Engine; }";
    let kir = compile_sysml_module(
        &parse_sysml(source).unwrap(),
        "ordered-imports.sysml",
        &library,
    )
    .unwrap();
    for (id, expected) in [
        ("feature.C.engine", "type.A.Types.Engine"),
        ("feature.S.engine", "type.R.Engine"),
    ] {
        let element = kir.elements.iter().find(|e| e.id == id).unwrap();
        assert!(
            element.properties["type"].to_string().contains(expected),
            "{id}: {:?}",
            element.properties["type"]
        );
    }
}

#[test]
fn release_2026_08_anonymous_enumeration_values_remain_distinct() {
    let library = load_sysml_baseline().unwrap();
    let module = parse_sysml("package P { enum def Choices :> ScalarValues::Integer { enum = 60; enum = 80; enum = 100; } }").unwrap();
    let kir = compile_sysml_module(&module, "anonymous-enum.sysml", &library).unwrap();
    let values = kir
        .elements
        .iter()
        .filter(|e| e.kind == "SysML::EnumerationUsage")
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 3);
    assert_eq!(
        values
            .iter()
            .map(|e| &e.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    for (element, value) in values.iter().zip([60, 80, 100]) {
        assert!(!element.properties.contains_key("declared_name"));
        assert_eq!(
            element.properties["expression_ir"]["value"],
            serde_json::json!(value)
        );
    }
}

#[test]
fn release_2026_08_allocations_keep_distinct_identities_and_endpoints() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { part a { part x; part y; } part b; allocate a.x to b; allocate a.y to b; allocation named allocate a to b; }";
    let kir =
        compile_sysml_module(&parse_sysml(source).unwrap(), "allocations.sysml", &library).unwrap();
    let allocations = kir
        .elements
        .iter()
        .filter(|e| e.kind == "AllocationUsage")
        .collect::<Vec<_>>();
    assert_eq!(allocations.len(), 3);
    assert_eq!(
        allocations
            .iter()
            .map(|e| &e.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    for element in allocations {
        assert!(element.properties.contains_key("allocated"));
        assert!(element.properties.contains_key("allocated_to"));
    }
    assert!(
        compile_sysml_module(
            &parse_sysml(&source.replace("a.y to b", "a.missing to b")).unwrap(),
            "bad-allocation.sysml",
            &library
        )
        .is_err()
    );
}

#[test]
fn release_2026_08_parenthesized_allocations_and_requirement_scope() {
    let library = load_sysml_baseline().unwrap();
    for source in [
        "package P { part a; part b; allocation def A { end logical; end physical; } allocation x: A allocate (logical ::> a, physical ::> b); }",
        "package P { requirement specification { requirement r; part p; allocate r to p; } }",
    ] {
        let kir = compile_sysml_module(
            &parse_sysml(source).unwrap(),
            "allocation-ends.sysml",
            &library,
        )
        .unwrap();
        assert!(kir.elements.iter().any(|e| e.kind == "AllocationUsage"
            && e.properties.contains_key("source")
            && e.properties.contains_key("target")));
    }
}

#[test]
fn release_2026_08_nested_namespaces_keep_owners_and_membership() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { part def Outer { package Nested; part def Inner; } part p { part def Local; package Nested { part def Child; } part local: Local; } }";
    let kir = compile_sysml_module(
        &parse_sysml(source).unwrap(),
        "nested-namespaces.sysml",
        &library,
    )
    .unwrap();
    for (owner, child) in [
        ("type.P.Outer", "pkg.P.Outer.Nested"),
        ("type.P.Outer", "type.P.Outer.Inner"),
        ("feature.P.p", "type.P.p.Local"),
        ("feature.P.p", "pkg.P.p.Nested"),
        ("pkg.P.p.Nested", "type.P.p.Nested.Child"),
    ] {
        let child_element = kir.elements.iter().find(|e| e.id == child).unwrap();
        assert_eq!(child_element.properties["owner"], serde_json::json!(owner));
        let owner_element = kir.elements.iter().find(|e| e.id == owner).unwrap();
        assert!(
            owner_element.properties["members"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(child))
        );
    }
}

#[test]
fn release_2026_08_extracted_usage_type_checks_reject_wrong_families() {
    let library = load_sysml_baseline().unwrap();
    for (keyword, issue) in [
        ("attribute", "validateAttributeUsageType"),
        ("port", "validatePortUsageType"),
        ("flow", "validateFlowUsageType"),
        ("interface", "validateInterfaceUsageType"),
        ("allocation", "validateAllocationUsageType"),
        ("state", "validateStateUsageType"),
    ] {
        let source = format!("package P {{ part def Wrong; {keyword} value: Wrong; }}");
        let error =
            compile_sysml_module(&parse_sysml(&source).unwrap(), "bad-typing.sysml", &library)
                .unwrap_err();
        assert!(error.message.contains(issue), "{keyword}: {error}");
    }
}

#[test]
fn release_2026_08_anonymous_message_payload_is_not_the_flow_type() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { item def Signal; event occurrence a; event occurrence b; message of s: Signal from a to b; message of Signal from a to b; }";
    let kir = compile_sysml_module(
        &parse_sysml(source).unwrap(),
        "anonymous-message.sysml",
        &library,
    )
    .unwrap();
    let messages = kir
        .elements
        .iter()
        .filter(|e| e.kind == "FlowUsage")
        .collect::<Vec<_>>();
    assert_eq!(messages.len(), 2);
    for message in messages {
        assert!(!message.properties.contains_key("declared_name"));
        assert_ne!(
            message.properties.get("type"),
            Some(&serde_json::json!("type.P.Signal"))
        );
    }
    let payloads = kir
        .elements
        .iter()
        .filter(|e| e.kind == "PayloadFeature")
        .collect::<Vec<_>>();
    assert_eq!(payloads.len(), 2);
    assert_eq!(
        payloads
            .iter()
            .filter(|e| e.properties.get("declared_name") == Some(&serde_json::json!("s")))
            .count(),
        1
    );
    assert_eq!(
        payloads
            .iter()
            .filter(|e| !e.properties.contains_key("declared_name"))
            .count(),
        1
    );
    for payload in payloads {
        assert_eq!(
            payload.properties["type"],
            serde_json::json!("type.P.Signal")
        );
    }
}

#[test]
fn release_2026_08_complete_expression_families_are_consumed() {
    for source in [
        "if x > y ? x - y else y - x",
        "a ?? b ?? c",
        "x & true xor y | false implies z",
        "x === y",
        "x !== y",
        "x hastype T",
        "x istype T",
        "x as T",
        "x @ T",
        "@@T",
        "meta T",
        "all T",
        "c#(1).count",
        "10[SI::m]",
        "x.?{in item; item != null}",
        "x.{in item; item + 1}",
        "x->collect {in item; item + 1}->reduce '+'",
        "f(input = x, P::other = 2)",
        "10 % 3",
        "~false",
        "+3",
        "*",
    ] {
        let mut parser = Parser::new(lex(source).unwrap(), false);
        let result = parser
            .parse_expression()
            .unwrap_or_else(|error| panic!("{source}: {error}"));
        assert!(
            matches!(parser.peek_kind(), TokenKind::Eof),
            "{source}: remaining {:?}; {result:?}",
            parser.peek_kind()
        );
    }
}

#[test]
fn release_2026_08_named_arguments_and_filters_survive_ast() {
    let mut parser = Parser::new(
        lex("f(input = data.?{in item: T; item != null})").unwrap(),
        false,
    );
    let Expr::Call { args, .. } = parser.parse_expression().unwrap() else {
        panic!("call")
    };
    let Expr::NamedArgument {
        parameter, value, ..
    } = &args[0]
    else {
        panic!("named argument")
    };
    assert_eq!(parameter.as_colon_string(), "input");
    let Expr::Operation {
        operator, operands, ..
    } = value.as_ref()
    else {
        panic!("selection")
    };
    assert_eq!(operator, ".?");
    let Expr::Lambda {
        parameters, body, ..
    } = &operands[1]
    else {
        panic!("body")
    };
    assert_eq!(parameters[0].name, "item");
    assert_eq!(parameters[0].ty.as_ref().unwrap().as_colon_string(), "T");
    assert!(matches!(
        body.as_ref(),
        Expr::Binary {
            op: BinaryOp::NotEqual,
            ..
        }
    ));
}

#[test]
fn release_2026_08_kerml_initializer_and_function_result_survive() {
    let source =
        "package P { feature x = if true ? 3 else 4; var count := 0; function f { in n; n + 1 } }";
    let module = crate::kerml::parse_kerml(source).unwrap();
    let Declaration::Package(package) = &module.members[0] else {
        panic!("package")
    };
    let Declaration::GenericUsage(x) = &package.members[0] else {
        panic!("x")
    };
    assert!(matches!(&x.expression, Some(Expr::Operation { operator, .. }) if operator == "if"));
    let Declaration::GenericUsage(count) = &package.members[1] else {
        panic!("count")
    };
    assert_eq!(
        count.expression,
        Some(Expr::Literal(LiteralExpr::Integer(0)))
    );
    let Declaration::GenericDefinition(function) = &package.members[2] else {
        panic!("function")
    };
    assert!(function.members.iter().any(|member| matches!(member, Declaration::GenericUsage(result) if result.modifiers.contains(&"return".to_string()) && matches!(result.expression, Some(Expr::Binary { op: BinaryOp::Add, .. })))));
}

#[test]
fn release_2026_08_lambda_binding_is_lexical_and_preserved_in_kir() {
    let source = parse_sysml(
        "package P { attribute data = (1, 2); attribute result = data.?{in item; item > 1}; }",
    )
    .unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = compile_sysml_module(&source, "lambda.sysml", &library).unwrap();
    let result = kir
        .elements
        .iter()
        .find(|e| {
            e.properties
                .get("declared_name")
                .is_some_and(|v| v == "result")
        })
        .unwrap();
    let ir = &result.properties["expression_ir"];
    assert_eq!(ir["operator"], ".?");
    assert_eq!(ir["operands"][1]["parameters"][0]["name"], "item");
    assert_eq!(ir["operands"][1]["body"]["left"]["kind"], "variable");
    let outside = parse_sysml("package P { attribute result = item; }").unwrap();
    assert!(compile_sysml_module(&outside, "unbound.sysml", &library).is_err());
}

#[test]
fn release_2026_08_kerml_values_preserve_flags_multiplicity_and_bodies_in_kir() {
    let module = crate::kerml::parse_kerml(
        "package P { feature data[2..4] default := (1, 2); expr doubled { in x; x + x } }",
    )
    .unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = crate::kerml::compile_kerml_module_strict_with_context(
        &module,
        "values.kerml",
        std::slice::from_ref(&module),
        &library,
    )
    .unwrap();
    let data = kir
        .elements
        .iter()
        .find(|e| {
            e.properties
                .get("declared_name")
                .is_some_and(|v| v == "data")
        })
        .unwrap();
    assert_eq!(data.properties["multiplicity_lower"], "2");
    assert_eq!(data.properties["multiplicity_upper"], "4");
    assert_eq!(data.properties["expression_is_initial"], true);
    assert_eq!(data.properties["expression_is_default"], true);
    assert_eq!(
        data.properties["expression_ir"]["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let doubled = kir
        .elements
        .iter()
        .find(|e| {
            e.properties
                .get("declared_name")
                .is_some_and(|v| v == "doubled")
        })
        .unwrap();
    assert!(doubled.kind.ends_with("Expression"));
    assert_eq!(doubled.properties["expression_ir"]["op"], "add");
}

#[test]
fn release_2026_08_unit_short_name_and_ref_lambda_parameter_are_preserved() {
    let source = parse_sysml("package P { attribute <'u/x'> unit = 1; attribute data = 3['u/x']; attribute filtered = data.?{in ref item; item == data}; }").unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = compile_sysml_module(&source, "units.sysml", &library).unwrap();
    let unit = kir
        .elements
        .iter()
        .find(|e| {
            e.properties
                .get("declared_name")
                .is_some_and(|v| v == "unit")
        })
        .unwrap();
    assert_eq!(unit.properties["declared_short_name"], "u/x");
    let filtered = kir
        .elements
        .iter()
        .find(|e| {
            e.properties
                .get("declared_name")
                .is_some_and(|v| v == "filtered")
        })
        .unwrap();
    let lambda = &filtered.properties["expression_ir"]["operands"][1];
    assert_eq!(lambda["parameters"][0]["name"], "item");
    assert_eq!(lambda["body"]["left"]["kind"], "variable");
    assert!(
        lambda["parameters"][0]["properties"]["modifiers"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("ref"))
    );
}

#[test]
fn release_2026_08_assertions_and_default_interface_ends_use_pilot_metaclasses() {
    let source = parse_sysml("package P { port def Port; interface def I { end source: Port; end sink: Port; } assert constraint valid { true } }").unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = compile_sysml_module(&source, "metaclasses.sysml", &library).unwrap();
    for name in ["source", "sink", "valid"] {
        let element = kir
            .elements
            .iter()
            .find(|e| e.properties.get("declared_name").is_some_and(|v| v == name))
            .unwrap();
        assert!(
            element.kind.ends_with(if name == "valid" {
                "AssertConstraintUsage"
            } else {
                "PortUsage"
            }),
            "{name}: {}",
            element.kind
        );
    }
}

#[test]
fn release_2026_08_conditional_type_predicates_keep_pilot_exclusions() {
    let library = load_sysml_baseline().unwrap();
    for (keyword, valid_kind, invalid_kind, issue) in [
        ("part", "part", "attribute", "validateOccurrenceUsageType"),
        ("action", "action", "attribute", "validateActionUsageType"),
        (
            "connection",
            "connection",
            "attribute",
            "validateConnectionUsageType",
        ),
    ] {
        let positive = format!("package P {{ {valid_kind} def T; {keyword} valid: T; }}");
        compile_sysml_module(&parse_sysml(&positive).unwrap(), "valid.sysml", &library)
            .unwrap_or_else(|error| panic!("{positive}: {error}"));
        let negative = format!("package P {{ {invalid_kind} def T; {keyword} invalid: T; }}");
        let error =
            compile_sysml_module(&parse_sysml(&negative).unwrap(), "invalid.sysml", &library)
                .unwrap_err();
        assert!(error.message.contains(issue), "{negative}: {error}");
    }
    // PortUsage is excluded from the OccurrenceUsage Class predicate.
    let source = parse_sysml("package P { port def T; port valid: T; }").unwrap();
    compile_sysml_module(&source, "port.sysml", &library).unwrap();
}

#[test]
fn release_2026_08_sysml_default_and_initial_values_remain_distinct() {
    let source = parse_sysml("package P { attribute ordinary = 1; attribute initial := 2; attribute fallback default 3; attribute both default := 4; }").unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = compile_sysml_module(&source, "value-flags.sysml", &library).unwrap();
    for (name, initial, default) in [
        ("ordinary", false, false),
        ("initial", true, false),
        ("fallback", false, true),
        ("both", true, true),
    ] {
        let element = kir
            .elements
            .iter()
            .find(|e| e.properties.get("declared_name").is_some_and(|v| v == name))
            .unwrap();
        assert_eq!(element.properties["expression_is_initial"], initial);
        assert_eq!(element.properties["expression_is_default"], default);
    }
}

#[test]
fn release_2026_08_individuals_and_actions_use_declared_semantics() {
    let source = parse_sysml("package P { part def Vehicle; individual def TestVehicle :> Vehicle; individual test: TestVehicle; attribute def Incr; action incr; action accept payload: Incr; }").unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = compile_sysml_module(&source, "individual-actions.sysml", &library).unwrap();
    let find = |name: &str| {
        kir.elements
            .iter()
            .find(|e| e.properties.get("declared_name").is_some_and(|v| v == name))
            .unwrap_or_else(|| {
                panic!(
                    "missing {name}: {:?}",
                    kir.elements
                        .iter()
                        .map(|e| (
                            &e.id,
                            &e.kind,
                            e.properties.get("declared_name"),
                            e.properties.get("type")
                        ))
                        .collect::<Vec<_>>()
                )
            })
    };
    assert!(find("TestVehicle").kind.ends_with("OccurrenceDefinition"));
    assert!(
        find("test").kind.ends_with("OccurrenceUsage"),
        "{} {source:?}",
        find("test").kind
    );
    assert_eq!(find("test").properties["is_individual"], true);
    assert_eq!(find("TestVehicle").properties["is_individual"], true);
    assert!(
        !find("incr")
            .properties
            .get("type")
            .is_some_and(|v| v.to_string().contains("Incr"))
    );
    assert!(
        kir.elements
            .iter()
            .any(|e| e.kind.ends_with("AcceptActionUsage"))
    );
    assert!(
        find("payload").properties["type"]
            .to_string()
            .contains("Incr")
    );
}

#[test]
fn release_2026_08_import_all_reaches_private_import_members() {
    let source = parse_sysml("package P { package Types { part def A; } package Hidden { private import Types::*; } package Client { public import all Hidden::*; part a: A; } }").unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = compile_sysml_module(&source, "import-all.sysml", &library).unwrap();
    let a = kir
        .elements
        .iter()
        .find(|e| e.properties.get("declared_name").is_some_and(|v| v == "a"))
        .unwrap();
    assert!(a.properties["type"].to_string().contains("P.Types.A"));
}

#[test]
fn release_2026_08_inherited_typing_resolves_in_target_lexical_scope() {
    let source = parse_sysml("package Q { part def F; part def B { part f: F; } part def C { part b: B; part c subsets b.f; } }").unwrap();
    let library = load_sysml_baseline().unwrap();
    let kir = compile_sysml_module(&source, "inherited-type.sysml", &library).unwrap();
    let c = kir
        .elements
        .iter()
        .find(|e| e.properties.get("declared_name").is_some_and(|v| v == "c"))
        .unwrap();
    assert_eq!(c.properties["type"], "type.Q.F");
}

#[test]
fn release_2026_08_scientific_constants_preserve_values_and_units() {
    let library = load_sysml_baseline().unwrap();
    for (keyword, kerml) in [("attribute", false), ("feature", true)] {
        let text = format!(
            "package P {{ {keyword} one = 1; {keyword} fine = 7.2973525693E-3[one]; {keyword} mass = 5.44617021487E-4[one]; {keyword} integerExponent = 1e+4; {keyword} fractionalExponent = .5E2; }}"
        );
        let module = if kerml {
            crate::kerml::parse_kerml(&text).unwrap()
        } else {
            parse_sysml(&text).unwrap()
        };
        let kir = if kerml {
            crate::kerml::compile_kerml_module_strict_with_context(
                &module,
                "constants.kerml",
                std::slice::from_ref(&module),
                &library,
            )
            .unwrap()
        } else {
            compile_sysml_module(&module, "constants.sysml", &library).unwrap()
        };
        for (name, expected, unit) in [
            ("fine", 0.0072973525693, true),
            ("mass", 0.000544617021487, true),
            ("integerExponent", 10000.0, false),
            ("fractionalExponent", 50.0, false),
        ] {
            let element = kir
                .elements
                .iter()
                .find(|e| e.properties.get("declared_name").is_some_and(|v| v == name))
                .unwrap();
            let ir = &element.properties["expression_ir"];
            let value = if unit {
                assert_eq!(ir["operator"], "[");
                assert_eq!(ir["operands"].as_array().unwrap().len(), 2);
                &ir["operands"][0]["value"]
            } else {
                &ir["value"]
            };
            assert_eq!(value.as_f64().unwrap(), expected, "{keyword} {name}");
        }
    }
}

#[test]
fn release_2026_08_initializer_self_reference_keeps_lexical_identity() {
    let library = load_sysml_baseline().unwrap();
    for (keyword, kerml) in [("attribute", false), ("feature", true)] {
        let text = format!(
            "package Other {{ {keyword} z = true; }} package P {{ {keyword} z = true implies z; {keyword} qualified = P::qualified; {keyword} data = (1, 2); {keyword} shadow = data.?{{in shadow; shadow > 1}}; }}"
        );
        let module = if kerml {
            crate::kerml::parse_kerml(&text).unwrap()
        } else {
            parse_sysml(&text).unwrap()
        };
        let kir = if kerml {
            crate::kerml::compile_kerml_module_strict_with_context(
                &module,
                "self.kerml",
                std::slice::from_ref(&module),
                &library,
            )
            .unwrap()
        } else {
            compile_sysml_module(&module, "self.sysml", &library).unwrap()
        };
        let z = kir.elements.iter().find(|e| e.id == "feature.P.z").unwrap();
        assert_eq!(
            z.properties["expression_ir"]["operands"][1]["segments"][0]["feature"],
            z.id
        );
        let qualified = kir
            .elements
            .iter()
            .find(|e| e.id == "feature.P.qualified")
            .unwrap();
        assert_eq!(
            qualified.properties["expression_ir"]["segments"][0]["feature"],
            qualified.id
        );
        let shadow = kir
            .elements
            .iter()
            .find(|e| e.id == "feature.P.shadow")
            .unwrap();
        assert_eq!(
            shadow.properties["expression_ir"]["operands"][1]["body"]["left"]["kind"],
            "variable"
        );
    }
}


#[test]
fn release_2026_08_typed_lambda_members_use_inherited_scope() {
    let library = load_sysml_baseline().unwrap();
    let declarations = "part def Inner { attribute value; } part def Base { part inner: Inner; } part def Child :> Base; part data: Child;";
    for (expression, valid) in [
        ("data.{in x: Child; x.inner.value}", true),
        ("data.{in x: Child; x.missing}", false),
        ("data.{in x: Child; x.inner.missing}", false),
        ("data.{in x: Child; data.{in x: Inner; x.value}}", true),
        ("data.{in x: Child; data.{in x: Inner; x.inner}}", false),
        ("data.{in x; x.dynamic}", true),
    ] {
        let source = format!("package P {{ {declarations} attribute result = {expression}; }}");
        let result = crate::compile_sysml_text(&source, "typed-lambda.sysml", &library);
        assert_eq!(result.is_ok(), valid, "{expression}: {result:?}");
        if let Err(error) = result { assert!(error.to_string().contains("unresolved member"), "{error}"); }
    }
}

#[test]
fn release_2026_08_single_type_controls_match_pilot() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/cases.json")).unwrap();
    let sources = [
        ("enum-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/enum-valid.sysml")),
        ("enum-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/enum-multiple.sysml")),
        ("enum-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/enum-redundant.sysml")),
        ("calc-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/calc-valid.sysml")),
        ("calc-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/calc-multiple.sysml")),
        ("calc-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/calc-redundant.sysml")),
        ("constraint-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/constraint-valid.sysml")),
        ("constraint-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/constraint-multiple.sysml")),
        ("constraint-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/constraint-redundant.sysml")),
        ("requirement-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/requirement-valid.sysml")),
        ("requirement-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/requirement-multiple.sysml")),
        ("requirement-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/requirement-redundant.sysml")),
        ("analysis-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/analysis-valid.sysml")),
        ("analysis-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/analysis-multiple.sysml")),
        ("analysis-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/analysis-redundant.sysml")),
        ("verification-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/verification-valid.sysml")),
        ("verification-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/verification-multiple.sysml")),
        ("verification-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/verification-redundant.sysml")),
        ("use-case-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/use-case-valid.sysml")),
        ("use-case-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/use-case-multiple.sysml")),
        ("use-case-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/use-case-redundant.sysml")),
        ("rendering-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/rendering-valid.sysml")),
        ("rendering-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/rendering-multiple.sysml")),
        ("rendering-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/rendering-redundant.sysml")),
        ("viewpoint-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/viewpoint-valid.sysml")),
        ("viewpoint-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/viewpoint-multiple.sysml")),
        ("viewpoint-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/viewpoint-redundant.sysml")),
        ("view-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/view-valid.sysml")),
        ("view-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/view-multiple.sysml")),
        ("view-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/view-redundant.sysml")),
        ("metadata-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/metadata-valid.sysml")),
        ("metadata-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/metadata-multiple.sysml")),
        ("metadata-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/metadata-redundant.sysml")),
        ("inherited-valid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/inherited-valid.sysml")),
        ("inherited-invalid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/inherited-invalid.sysml")),
        ("inherited-second-invalid.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/inherited-second-invalid.sysml")),
        ("constraint-inherited-multiple.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/constraint-inherited-multiple.sysml")),
        ("constraint-inherited-redundant.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/single-type-controls/constraint-inherited-redundant.sysml")),
    ];
    let library = load_sysml_baseline().unwrap();
    let context = sources.iter().map(|(_, text)| parse_sysml(text).unwrap()).collect::<Vec<_>>();
    for (name, source) in sources {
        let case = cases["cases"].as_array().unwrap().iter().find(|c| c["relative_path"] == name).unwrap();
        let result = crate::compile_sysml_text_with_context(source, name, &context, &library);
        assert_eq!(result.is_ok(), case["expected_status"] == "ok", "{name}: {result:?}");
        if let Err(error) = result {
            assert!(case["pilot_error_codes"].as_array().unwrap().iter().any(|code| error.to_string().contains(code.as_str().unwrap())), "{name}: {error}");
        }
    }
}


#[test]
fn release_2026_08_named_assignment_retains_action_kind_and_operands() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { part def A; part left: A; part right: A; action behavior { action replace assign left := right; } }";
    let document = crate::compile_sysml_text(source, "assignment.sysml", &library).unwrap();
    let action = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "replace")).unwrap();
    assert!(action.kind.ends_with("AssignmentActionUsage"), "{action:?}");
    assert!(!action.properties.contains_key("expression_ir"));
    for operand in ["target", "replacementValues"] {
        assert!(document.elements.iter().any(|e| e.properties.get("owner").is_some_and(|v| v == &action.id)
            && e.id.contains(operand) && e.properties.contains_key("expression_ir")), "{operand}: {document:?}");
    }
}

#[test]
fn release_2026_08_type_defaults_remain_local_to_the_compiled_source() {
    let library = load_sysml_baseline().unwrap();
    let action = parse_sysml("package P { action screen; }").unwrap();
    let part = parse_sysml("package P { part screen; }").unwrap();
    for context in [vec![action.clone(), part.clone()], vec![part.clone(), action.clone()]] {
        for (name, module, expected) in [
            ("action.sysml", &action, "Actions::Action"),
            ("part.sysml", &part, "Parts::Part"),
        ] {
            let document = crate::compile_sysml_module_with_context(module, name, &context, &library)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            let screen = document.elements.iter().find(|element|
                element.properties.get("declared_name").is_some_and(|value| value == "screen")
            ).unwrap();
            let types = screen.properties.get("type").unwrap();
            assert!(types == expected || types.as_array().is_some_and(|values|
                values.len() == 1 && values[0] == expected), "{name}: {types}");
        }
    }
}

#[test]
fn release_2026_08_preserves_parameter_direction_across_usage_families() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { part def T; action def A { in x; out y; inout z; in part input: T; out attribute output: ScalarValues::Integer; inout port bus; } calc def C { return result: ScalarValues::Integer; } }";
    let document = crate::compile_sysml_text(source, "directions.sysml", &library).unwrap();
    for (name, direction) in [("x", "in"), ("y", "out"), ("z", "inout"), ("input", "in"), ("output", "out"), ("bus", "inout"), ("result", "out")] {
        let element = document.elements.iter().find(|element|
            element.properties.get("declared_name").is_some_and(|value| value == name)
        ).unwrap();
        assert_eq!(element.properties.get("direction"), Some(&serde_json::json!(direction)), "{name}: {element:?}");
    }
}

#[test]
fn release_2026_08_bare_sysml_declarations_are_reference_usages() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { item def I; part def A { value; typed: I; } }";
    let document = crate::compile_sysml_text(source, "references.sysml", &library).unwrap();
    for name in ["value", "typed"] {
        let element = document.elements.iter().find(|element|
            element.properties.get("declared_name").is_some_and(|value| value == name)
        ).unwrap();
        assert!(element.kind.ends_with("ReferenceUsage"), "{element:?}");
    }
    let module = crate::kerml::parse_kerml("package P { feature value; }").unwrap();
    let kernel = crate::kerml::compile_kerml_module_strict_with_context(
        &module, "feature.kerml", std::slice::from_ref(&module), &library
    ).unwrap();
    assert!(kernel.elements.iter().any(|element| element.kind.ends_with("Feature")
        && element.properties.get("declared_name").is_some_and(|value| value == "value")));
}

#[test]
fn release_2026_08_legacy_lowering_aliases_emit_real_metaclasses() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { part a; part b; dependency dep from a to b; comment note /* text */ requirement def R; satisfy requirement satisfied: R by a; }";
    let document = crate::compile_sysml_text(source, "metaclasses.sysml", &library).unwrap();
    for (name, kind) in [("dep", "Dependency"), ("note", "Comment"), ("satisfied", "SatisfyRequirementUsage")] {
        let element = document.elements.iter().find(|element|
            element.properties.get("declared_name").is_some_and(|value| value == name)
        ).unwrap();
        assert_eq!(element.kind.rsplit("::").next(), Some(kind), "{name}: {element:?}");
        assert_eq!(element.properties.get("metatype").and_then(|value| value.as_str()).map(|name| name.rsplit("::").next().unwrap()), Some(kind), "{name}: {element:?}");
    }
}

#[test]
fn release_2026_08_library_packages_keep_identity_in_both_languages() {
    let library = load_sysml_baseline().unwrap();
    for source in ["library package L { package N; }", "standard library package L { package N; }"] {
        let sysml = crate::compile_sysml_text(source, "library.sysml", &library).unwrap();
        let module = crate::kerml::parse_kerml(source).unwrap();
        let kerml = crate::kerml::compile_kerml_module_strict_with_context(&module, "library.kerml", std::slice::from_ref(&module), &library).unwrap();
        for document in [&sysml, &kerml] {
            let outer = document.elements.iter().find(|element| element.id == "pkg.L").unwrap();
            assert!(outer.kind.ends_with("LibraryPackage"), "{outer:?}");
            assert_eq!(outer.properties.get("is_standard"), Some(&serde_json::json!(source.starts_with("standard"))));
            let inner = document.elements.iter().find(|element| element.id == "pkg.L.N").unwrap();
            assert!(!inner.kind.ends_with("LibraryPackage"), "{inner:?}");
        }
    }
}

#[test]
fn release_2026_08_expression_references_use_emitted_usage_ids() {
    let library = load_sysml_baseline().unwrap();
    let document = crate::compile_sysml_text(
        "package P { value = 2; attribute copy = value; attribute literal = \"feature.P.value\"; }",
        "reference-identity.sysml", &library,
    ).unwrap();
    let value = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "value")).unwrap();
    let copy = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "copy")).unwrap();
    assert_eq!(copy.properties["expression_ir"]["segments"][0]["feature"], value.id, "{document:?}");
    let literal = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "literal")).unwrap();
    assert_eq!(literal.properties["expression_ir"]["value"], "feature.P.value");
}

#[test]
fn release_2026_08_reference_identity_crosses_source_boundaries() {
    let library = load_sysml_baseline().unwrap();
    let support = parse_sysml("package A { value = 2; action work; }").unwrap();
    let source = parse_sysml("package B { attribute copy = A::value; allocation task allocate A::work to A::value; }").unwrap();
    let context = vec![support.clone(), source.clone()];
    let owner = crate::compile_sysml_module_with_context(&support, "a.sysml", &context, &library).unwrap();
    let consumer = crate::compile_sysml_module_with_context(&source, "b.sysml", &context, &library).unwrap();
    let value = owner.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "value")).unwrap();
    let work = owner.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "work")).unwrap();
    let copy = consumer.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "copy")).unwrap();
    assert_eq!(copy.properties["expression_ir"]["segments"][0]["feature"], value.id);
    let allocation = consumer.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "task")).unwrap();
    assert_eq!(allocation.properties["source"], work.id);
    assert_eq!(allocation.properties["target"], value.id);
}

#[test]
fn release_2026_08_include_preserves_metaclass_and_reference() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { use case def U; use case target: U; use case def Main { include use case typed: U; include P::target; } }";
    let document = crate::compile_sysml_text(source, "include.sysml", &library).unwrap();
    let includes = document.elements.iter().filter(|e| e.kind.ends_with("IncludeUseCaseUsage")).collect::<Vec<_>>();
    assert_eq!(includes.len(), 2, "{document:?}");
    let target = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "target")).unwrap();
    let inclusion = includes.iter().find(|e| e.properties.contains_key("owned_reference_subsetting")).unwrap();
    let relation = document.elements.iter().find(|e| e.id == inclusion.properties["owned_reference_subsetting"]).unwrap();
    assert!(relation.kind.ends_with("ReferenceSubsetting"));
    assert_eq!(relation.properties["referenced_feature"], target.id);
    assert_eq!(relation.properties["subsetting_feature"], inclusion.id);
    mercurio_foundation::KirDocument::merge_with_registered_fields(
        vec![document], crate::sysml_field_specs().iter().copied()
    ).unwrap();

}

#[test]
fn release_2026_08_include_reference_controls_match_pilot() {
    let library = load_sysml_baseline().unwrap();
    let sources = [
        (include_str!("../../../mercurio-tools/corpus/release-2026-08/include-controls/typed-valid.sysml"), true),
        (include_str!("../../../mercurio-tools/corpus/release-2026-08/include-controls/default-valid.sysml"), true),
        (include_str!("../../../mercurio-tools/corpus/release-2026-08/include-controls/reference-valid.sysml"), true),
        (include_str!("../../../mercurio-tools/corpus/release-2026-08/include-controls/reference-invalid.sysml"), false),
    ];
    let context = sources.iter().map(|(source, _)| parse_sysml(source).unwrap()).collect::<Vec<_>>();
    for (source, valid) in sources {
        let result = crate::compile_sysml_text_with_context(source, "include-control.sysml", &context, &library);
        assert_eq!(result.is_ok(), valid, "{source}: {result:?}");
        if let Err(error) = result { assert!(error.to_string().contains("validateIncludeUseCaseUsageReference"), "{error}"); }
    }
}

#[test]
fn release_2026_08_usage_flags_match_pilot_context_and_typing() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/usage-flags.sysml");
    let observations: serde_json::Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/usage-flags.pilot.json")).unwrap();
    let document = crate::compile_sysml_text(source, "usage-flags.sysml", &library).unwrap();
    for expected in observations["observations"].as_array().unwrap() {
        let name = expected["qualified_name"].as_str().unwrap().rsplit("::").next().unwrap();
        let element = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
        assert_eq!(element.kind.rsplit("::").next().unwrap(), expected["kind"].as_str().unwrap());
        for (key, value) in expected["properties"].as_object().unwrap() {
            assert_eq!(element.properties.get(key), Some(value), "{name}.{key}: {element:?}");
        }
    }
}

#[test]
fn release_2026_08_preserves_ordered_nonunique_collections_in_both_languages() {
    let library = load_sysml_baseline().unwrap();
    for (source, kernel) in [("package P { attribute values[*] ordered nonunique; }", false), ("package P { feature values[*] ordered nonunique; }", true)] {
        let document = if kernel {
            let module = crate::kerml::parse_kerml(source).unwrap();
            crate::kerml::compile_kerml_module_strict_with_context(&module, "flags.kerml", std::slice::from_ref(&module), &library).unwrap()
        } else { crate::compile_sysml_text(source, "flags.sysml", &library).unwrap() };
        let values = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "values")).unwrap();
        assert_eq!(values.properties.get("is_ordered"), Some(&serde_json::json!(true)));
        assert_eq!(values.properties.get("is_unique"), Some(&serde_json::json!(false)));
    }
}

#[test]
fn release_2026_08_compiled_expression_execution_matches_pilot() {
    use mercurio_foundation::{ExpressionIr, ExpressionEvaluationContext, ExpressionEvaluationError, ExpressionPathSegment};
    use serde_json::Value;
    struct ClosedContext;
    impl ExpressionEvaluationContext for ClosedContext {
        fn owner_id(&self) -> &str { "execution-controls" }
        fn resolve_path(&mut self, segments: &[ExpressionPathSegment]) -> Result<Vec<Value>, ExpressionEvaluationError> {
            Err(ExpressionEvaluationError::MissingBinding(format!("{segments:?}")))
        }
    }
    let library = load_sysml_baseline().unwrap();
    let observations: Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/execution-controls.pilot.json")).unwrap();
    for case in observations["observations"].as_array().unwrap() {
        assert_eq!(case["status"], "ok");
        for kernel in [false, true] {
            let source = format!("package Execution {{ {} result = {}; }}", if kernel {"feature"} else {"attribute"}, case["expression"].as_str().unwrap());
            let document = if kernel {
                let module = crate::kerml::parse_kerml(&source).unwrap_or_else(|error| panic!("{source}: {error}"));
                crate::kerml::compile_kerml_module_strict_with_context(&module, "execution.kerml", std::slice::from_ref(&module), &library).unwrap()
            } else { crate::compile_sysml_text(&source, "execution.sysml", &library).unwrap_or_else(|error| panic!("{source}: {error}")) };
            let result = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v=="result")).unwrap();
            let ir = ExpressionIr::from_value(&result.properties["expression_ir"]).unwrap();
            let actual = ir.evaluate(&mut ClosedContext).unwrap_or_else(|error| panic!("{source}: {error}"));
            let values = match actual { Value::Null => vec![], Value::Array(values)=>values, value=>vec![value] };
            assert_eq!(serde_json::json!(values), case["values"], "{source}");
        }
    }
}

#[test]
fn release_2026_08_audit_derived_flags_match_pilot() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/audit-derived-flags.sysml");
    let observations: serde_json::Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/audit-derived-flags.pilot.json")).unwrap();
    let document = crate::compile_sysml_text(source, "audit-derived-flags.sysml", &library).unwrap();
    for (id, expected) in [("type.FlagDefaults.Interface", "Interfaces::Interface"), ("type.AuditFlags.Bus", "Interfaces::BinaryInterface")] {
        let definition = document.elements.iter().find(|e| e.id == id).unwrap();
        assert!(definition.properties["specializes"].as_array().unwrap().iter().any(|v| v == expected), "{id}: {definition:?}");
    }
    for expected in observations["observations"].as_array().unwrap() {
        let qualified = expected["qualified_name"].as_str().unwrap();
        let name = qualified.rsplit("::").next().unwrap();
        let matches = document.elements.iter().filter(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).collect::<Vec<_>>();
        assert_eq!(matches.len(), 1, "expected a unique {qualified}: {document:?}");
        let element = matches[0];
        assert_eq!(element.kind.rsplit("::").next().unwrap(), expected["kind"].as_str().unwrap());
        for (key, value) in expected["properties"].as_object().unwrap() {
            assert_eq!(element.properties.get(key), Some(value), "{qualified}.{key}: {element:?}");
        }
    }
}

#[test]
fn release_2026_08_anonymous_actions_keep_their_kind_and_owned_variability() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { action { attribute reading; } 'action'; }";
    let document = crate::compile_sysml_text(source, "anonymous-action.sysml", &library).unwrap();
    let reading = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "reading")).unwrap();
    assert_eq!(reading.properties.get("is_variable"), Some(&serde_json::json!(true)));
    let owner = document.elements.iter().find(|e| Some(&serde_json::json!(e.id)) == reading.properties.get("owner")).unwrap();
    assert!(owner.kind.ends_with("ActionUsage"), "{owner:?}");
    assert!(document.elements.iter().any(|e| e.kind.ends_with("ReferenceUsage") && e.properties.get("declared_name").is_some_and(|v| v == "action")));
}

#[test]
fn release_2026_08_structured_action_defaults_follow_pilot() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { action { if true { action a; } if true { action b; } else { action c; } while true { action d; } for n in (1, 2) { action e; } } }";
    let document = crate::compile_sysml_text(source, "structured-defaults.sysml", &library).unwrap();
    for (kind, expected) in [("IfActionUsage", "Actions::IfThenAction"), ("IfActionUsage", "Actions::IfThenElseAction"), ("WhileLoopActionUsage", "Actions::WhileLoopAction"), ("ForLoopActionUsage", "Actions::ForLoopAction")] {
        assert!(document.elements.iter().any(|e| e.kind.ends_with(kind) && e.properties.get("type").is_some_and(|v| v == expected)), "missing {kind}: {expected}");
    }
    let variable = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "n")).unwrap();
    assert_eq!(variable.properties.get("is_variable"), Some(&serde_json::json!(true)));
}

#[test]
fn release_2026_08_conditional_default_types_follow_pilot() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/conditional-defaults.sysml");
    let document = crate::compile_sysml_text(source, "conditional-defaults.sysml", &library).unwrap();
    for (name, line, kind, expected, variable) in [
        ("untyped", 2, "OccurrenceUsage", vec!["Occurrences::Occurrence"], false),
        ("child", 2, "OccurrenceUsage", vec!["Occurrences::Occurrence"], true),
        ("binary", 6, "ConnectionUsage", vec!["Connections::Connection", "Objects::BinaryLinkObject"], false),
        ("nary", 7, "ConnectionUsage", vec!["Connections::Connection"], false),
        ("move", 11, "TransitionUsage", vec!["Actions::DecisionTransitionAction"], false),
        ("move", 16, "TransitionUsage", vec!["States::StateTransitionAction"], false),
    ] {
        let matches = document.elements.iter().filter(|e| e.properties.get("declared_name").is_some_and(|v| v == name)
            && e.properties["metadata"]["source_span"]["start_line"] == line).collect::<Vec<_>>();
        assert_eq!(matches.len(), 1, "expected one {name} on line {line}");
        let element = matches[0];
        assert_eq!(element.kind.rsplit("::").next().unwrap(), kind, "{}", element.id);
        let actual = &element.properties["type"];
        let actual = actual.as_array().map(|a| a.iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>()).unwrap_or_else(|| vec![actual.as_str().unwrap()]);
        assert_eq!(actual, expected, "{}: {element:?}", element.id);
        assert_eq!(element.properties["is_variable"], serde_json::json!(variable), "{}", element.id);
    }
}

#[test]
fn release_2026_08_connector_ends_retain_reference_subsettings() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/conditional-defaults.sysml");
    let document = crate::compile_sysml_text(source, "conditional-defaults.sysml", &library).unwrap();
    for (name, expected) in [("binary", vec!["firstPart", "secondPart"]), ("nary", vec!["firstPart", "secondPart", "thirdPart"])] {
        let connection = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
        let ends = document.elements.iter().filter(|e| e.properties.get("owner").is_some_and(|v| v == &connection.id) && e.properties.get("is_end") == Some(&serde_json::json!(true))).collect::<Vec<_>>();
        assert_eq!(ends.len(), expected.len(), "{name}");
        let mut actual = Vec::new();
        for end in ends {
            assert!(!end.properties.contains_key("declared_name"), "anonymous end acquired a declared name: {end:?}");
            let relation_id = end.properties["owned_reference_subsetting"].as_str().unwrap();
            let relationship = document.elements.iter().find(|e| e.id == relation_id).unwrap();
            assert!(relationship.kind.ends_with("ReferenceSubsetting"));
            let target = relationship.properties["referenced_feature"].as_str().unwrap();
            let feature = document.elements.iter().find(|e| e.id == target).unwrap();
            actual.push(feature.properties["declared_name"].as_str().unwrap());
        }
        assert_eq!(actual, expected);
    }
}

#[test]
fn release_2026_08_sequence_execution_records_pilot_matches_and_divergences() {
    use mercurio_foundation::{ExpressionIr, ExpressionEvaluationContext, ExpressionEvaluationError, ExpressionPathSegment};
    use serde_json::Value;
    struct ClosedContext;
    impl ExpressionEvaluationContext for ClosedContext {
        fn owner_id(&self) -> &str { "execution-controls" }
        fn resolve_path(&mut self, segments: &[ExpressionPathSegment]) -> Result<Vec<Value>, ExpressionEvaluationError> {
            Err(ExpressionEvaluationError::MissingBinding(format!("{segments:?}")))
        }
    }
    let library = load_sysml_baseline().unwrap();
    let observations: Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/sequence-execution-controls.pilot.json")).unwrap();
    let mut matches = 0;
    let mut differences = 0;
    let mut oracle_errors = 0;
    for case in observations["observations"].as_array().unwrap() {
        if case["status"] != "ok" { oracle_errors += 1; continue; }
        for kernel in [false, true] {
            let source = format!("package Execution {{ {} result = {}; }}", if kernel {"feature"} else {"attribute"}, case["expression"].as_str().unwrap());
            let document = if kernel {
                let module = crate::kerml::parse_kerml(&source).unwrap_or_else(|error| panic!("{source}: {error}"));
                crate::kerml::compile_kerml_module_strict_with_context(&module, "execution.kerml", std::slice::from_ref(&module), &library).unwrap()
            } else { crate::compile_sysml_text(&source, "execution.sysml", &library).unwrap_or_else(|error| panic!("{source}: {error}")) };
            let result = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v=="result")).unwrap();
            let ir = ExpressionIr::from_value(&result.properties["expression_ir"]).unwrap();
            let actual = ir.evaluate(&mut ClosedContext).unwrap_or_else(|error| panic!("{source}: {error}"));
            let values = match actual { Value::Null => vec![], Value::Array(values)=>values, value=>vec![value] };
            let values = serde_json::json!(values);
            if let Some(divergence) = observations["known_divergences"].get(case["id"].as_str().unwrap()) {
                assert_ne!(values, case["values"], "recorded divergence changed: {source}");
                assert_eq!(values, divergence["native_values"], "{source}");
                differences += 1;
            } else {
                assert_eq!(values, case["values"], "{source}");
                matches += 1;
            }
        }
    }
    assert_eq!((matches, differences, oracle_errors), (38, 4, 5));
}


#[test]
fn release_2026_08_transition_defaults_resolve_inherited_source_kinds() {
    let library = load_sysml_baseline().unwrap();
    let base = parse_sysml("package SourceKinds { state def Base { state ready; state completed; } }").unwrap();
    let derived = parse_sysml("package UsageKinds { state def Derived :> SourceKinds::Base { transition inherited first ready then completed; action localAction; transition fromAction first localAction then completed; } }").unwrap();
    let document = crate::compile_sysml_module_with_context(&derived, "inherited-transitions.sysml", &[base, derived.clone()], &library).unwrap();
    for (name, expected) in [("inherited", "States::StateTransitionAction"), ("fromAction", "Actions::DecisionTransitionAction")] {
        let element = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
        assert_eq!(element.properties["type"], expected, "{name}: {element:?}");
    }
}

#[test]
fn release_2026_08_flow_end_controls_match_pilot() {
    let library = load_sysml_baseline().unwrap();
    for (source, parent) in [
        (include_str!("../../../mercurio-tools/corpus/release-2026-08/flow-end-controls/abstract-zero.sysml"), "Flows::MessageAction"),
        (include_str!("../../../mercurio-tools/corpus/release-2026-08/flow-end-controls/binary.sysml"), "Flows::Message"),
    ] {
        let document = crate::compile_sysml_text(source, "flow-ends.sysml", &library).unwrap();
        let definition = document.elements.iter().find(|e| e.kind.ends_with("FlowDefinition")).unwrap();
        assert!(definition.properties["specializes"].as_array().unwrap().iter().any(|v| v == parent), "{definition:?}");
    }
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/flow-end-controls/three-ends.sysml");
    let error = crate::compile_sysml_text(source, "flow-ends.sysml", &library).unwrap_err();
    assert!(error.message.contains("validateFlowDefinitionConnectionEnds"), "{error}");
}

#[test]
fn release_2026_08_interface_ends_keep_port_kind_and_reference_subsettings() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/interface-ends.sysml");
    let document = crate::compile_sysml_text(source, "interface-ends.sysml", &library).unwrap();
    for (name, kind, names, targets) in [
        ("namedBinary", "PortUsage", vec![Some("left"), Some("right")], vec!["portA", "portB"]),
        ("anonymousBinary", "PortUsage", vec![None, None], vec!["portA", "portB"]),
        ("namedNary", "PortUsage", vec![Some("alpha"), Some("beta"), Some("gamma")], vec!["portA", "portB", "portC"]),
        ("anonymousNary", "PortUsage", vec![None, None, None], vec!["portA", "portB", "portC"]),
        ("genericConnection", "ReferenceUsage", vec![None, None], vec!["portA", "portB"]),
    ] {
        let owner = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
        let ends = document.elements.iter().filter(|e| e.properties.get("owner").is_some_and(|v| v == &owner.id)
            && e.properties.get("is_end") == Some(&serde_json::json!(true))).collect::<Vec<_>>();
        assert_eq!(ends.len(), targets.len(), "{name}");
        for ((end, declared_name), target) in ends.into_iter().zip(names).zip(targets) {
            assert_eq!(end.kind.rsplit("::").next(), Some(kind), "{name}: {end:?}");
            assert_eq!(end.properties.get("declared_name").and_then(|v| v.as_str()), declared_name);
            let relation_id = end.properties["owned_reference_subsetting"].as_str().unwrap();
            let relation = document.elements.iter().find(|e| e.id == relation_id).unwrap();
            assert!(relation.kind.ends_with("ReferenceSubsetting"));
            let target_id = relation.properties["referenced_feature"].as_str().unwrap();
            let referenced = document.elements.iter().find(|e| e.id == target_id).unwrap();
            assert_eq!(referenced.properties.get("declared_name"), Some(&serde_json::json!(target)));
        }
    }
}

#[test]
fn release_2026_08_referenced_connector_ends_inherit_minimal_port_types() {
    use std::collections::BTreeSet;
    let library = load_sysml_baseline().unwrap();
    let sources = [
        include_str!("../../../mercurio-tools/corpus/release-2026-08/referenced-port-types/definitions.sysml"),
        include_str!("../../../mercurio-tools/corpus/release-2026-08/referenced-port-types/sources.sysml"),
        include_str!("../../../mercurio-tools/corpus/release-2026-08/referenced-port-types/connections.sysml"),
    ];
    let modules = sources.iter().map(|source| parse_sysml(source).unwrap()).collect::<Vec<_>>();
    let combined = sources.join("\n");
    let single = parse_sysml(&combined).unwrap();
    for (module, context) in [(&modules[2], modules.as_slice()), (&single, std::slice::from_ref(&single))] {
        let document = crate::compile_sysml_module_with_context(module, "connections.sysml", context, &library).unwrap();
        for (owner_name, names, type_names) in [
            ("typed", vec![Some("left"), Some("right")], vec![vec!["Derived"], vec!["Derived", "Other"]]),
            ("path", vec![None, None], vec![vec!["Derived"], vec!["Derived"]]),
            ("mixedNamed", vec![Some("left"), None], vec![vec!["Derived"], vec!["Derived", "Other"]]),
            ("mixedAnonymous", vec![None, Some("right")], vec![vec!["Derived"], vec!["Derived", "Other"]]),
            ("minimal", vec![None, None], vec![vec!["Derived"], vec!["Derived"]]),
            ("genericNamed", vec![Some("left"), None], vec![vec!["Derived"], vec!["Derived", "Other"]]),
            ("genericAnonymous", vec![None, Some("right")], vec![vec!["Derived"], vec!["Derived", "Other"]]),
        ] {
            let owner = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == owner_name)).unwrap();
            let ends = document.elements.iter().filter(|e| e.properties.get("owner").is_some_and(|v| v == &owner.id)
                && e.properties.get("is_end") == Some(&serde_json::json!(true))).collect::<Vec<_>>();
            assert_eq!(ends.len(), 2, "{owner_name}");
            for ((end, name), expected) in ends.into_iter().zip(names).zip(type_names) {
                assert_eq!(end.properties.get("declared_name").and_then(|v| v.as_str()), name, "{owner_name}");
                for field in ["type", "definition"] {
                    let value = &end.properties[field];
                    let actual = value.as_array().map(|values| values.iter().collect::<Vec<_>>()).unwrap_or_else(|| vec![value]);
                    let actual = actual.into_iter().map(|v| v.as_str().unwrap().rsplit(['.', ':']).next().unwrap()).collect::<BTreeSet<_>>();
                    assert_eq!(actual, expected.iter().copied().collect(), "{owner_name}.{field}: {end:?}");
                }
                assert!(end.properties.contains_key("owned_reference_subsetting"));
            }
        }
    }
}

#[test]
fn release_2026_08_reference_kind_controls_match_pilot() {
    let library = load_sysml_baseline().unwrap();
    for (name, source, issue) in [
        ("perform-positive.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/reference-kind-controls/perform-positive.sysml"), None),
        ("perform-negative.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/reference-kind-controls/perform-negative.sysml"), Some("validatePerformActionUsageReference")),
        ("exhibit-positive.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/reference-kind-controls/exhibit-positive.sysml"), None),
        ("exhibit-negative.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/reference-kind-controls/exhibit-negative.sysml"), Some("validateExhibitStateUsageReference")),
        ("assert-positive.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/reference-kind-controls/assert-positive.sysml"), None),
        ("assert-negative.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/reference-kind-controls/assert-negative.sysml"), Some("validateAssertConstraintUsageReference")),
        ("satisfy-positive.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/reference-kind-controls/satisfy-positive.sysml"), None),
        ("satisfy-negative.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/reference-kind-controls/satisfy-negative.sysml"), Some("validateSatisfyRequirementUsageReference")),
    ] {
        let result = crate::compile_sysml_text(source, name, &library);
        if let Some(issue) = issue {
            let error = result.unwrap_err();
            assert!(error.message.contains(issue), "{name}: {error}");
        } else {
            let document = result.unwrap_or_else(|error| panic!("{name}: {error}"));
            let references = document.elements.iter().filter(|e| e.properties.contains_key("owned_reference_subsetting")).collect::<Vec<_>>();
            assert_eq!(references.len(), 1, "{name}: {document:?}");
            let usage = references[0];
            assert!(!usage.properties.contains_key("declared_name"), "implicit reference acquired a declared name: {usage:?}");
            let relation_id = usage.properties["owned_reference_subsetting"].as_str().unwrap();
            let relation = document.elements.iter().find(|e| e.id == relation_id).unwrap();
            let target = relation.properties["referenced_feature"].as_str().unwrap();
            assert_ne!(target, usage.id, "reference resolves to itself: {name}");
            assert!(document.elements.iter().any(|e| e.id == target), "missing referenced feature: {name}");
        }
    }
}

#[test]
fn release_2026_08_reference_prefixes_and_succession_flow_keep_semantics() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/reference-kind-controls/prefixes-and-flow.sysml");
    let document = crate::compile_sysml_text(source, "prefixes-and-flow.sysml", &library).unwrap();
    let flow = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "handoff")).unwrap();
    assert!(flow.kind.ends_with("SuccessionFlowUsage"), "{flow:?}");
    assert_eq!(flow.properties["type"], "Flows::SuccessionFlow");
    let assertions = document.elements.iter().filter(|e| e.kind.ends_with("AssertConstraintUsage")).collect::<Vec<_>>();
    assert_eq!(assertions.len(), 2);
    assert!(assertions.iter().all(|e| e.properties.get("is_negated") == Some(&serde_json::json!(true))));
    let satisfactions = document.elements.iter().filter(|e| e.kind.ends_with("SatisfyRequirementUsage")).collect::<Vec<_>>();
    assert_eq!(satisfactions.len(), 3);
    assert_eq!(satisfactions.iter().filter(|e| e.properties.get("is_negated") == Some(&serde_json::json!(true))).count(), 2);
    let performed = document.elements.iter().find(|e| e.kind.ends_with("PerformActionUsage")).unwrap();
    let relation = document.elements.iter().find(|e| e.id == performed.properties["owned_reference_subsetting"].as_str().unwrap()).unwrap();
    assert_eq!(relation.properties["referenced_feature"], flow.id);
}

#[test]
fn release_2026_08_authoring_preserves_qualified_and_negated_references() {
    let source = "package Targets { action work; state ready; constraint condition { true } requirement need; } package Uses { perform Targets::work; exhibit Targets::ready; assert not Targets::condition; not satisfy Targets::need; }";
    let project = crate::authoring::load_authoring_project_from_sysml(std::collections::BTreeMap::from([("references.sysml".into(), source.into())])).unwrap();
    let rendered = project.render_new_file("references.sysml").unwrap();
    for phrase in ["perform Targets::work", "exhibit Targets::ready", "assert not Targets::condition", "not satisfy Targets::need"] {
        assert!(rendered.contains(phrase), "missing {phrase}: {rendered}");
    }
    let library = load_sysml_baseline().unwrap();
    let document = crate::compile_sysml_text(&rendered, "references.sysml", &library).unwrap();
    assert_eq!(document.elements.iter().filter(|e| e.properties.contains_key("owned_reference_subsetting")).count(), 4);
    assert_eq!(document.elements.iter().filter(|e| e.properties.get("is_negated") == Some(&serde_json::json!(true))).count(), 2);
}

#[test]
fn release_2026_08_extended_declarations_keep_base_metaclasses_and_members() {
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/metaclass-preservation/extensions.sysml");
    let library = load_sysml_baseline().unwrap();
    let document = crate::compile_sysml_text(source, "extensions.sysml", &library).unwrap();
    for (name, kind) in [("Extended", "Definition"), ("single", "Usage"), ("stacked", "Usage"), ("ordinary", "PartUsage")] {
        let element = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
        assert_eq!(element.kind.rsplit("::").next(), Some(kind), "{element:?}");
    }
    assert_eq!(document.elements.iter().filter(|e| e.kind.ends_with("ConnectionUsage")).count(), 2);
    for (name, owner) in [("nested", "Extended"), ("child", "single")] {
        let element = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
        let parent = document.elements.iter().find(|e| e.id == element.properties["owner"].as_str().unwrap()).unwrap();
        assert_eq!(parent.properties["declared_name"], owner);
    }
    let project = crate::authoring::load_authoring_project_from_sysml(std::collections::BTreeMap::from([("extensions.sysml".into(), source.into())])).unwrap();
    let rendered = project.render_new_file("extensions.sysml").unwrap();
    assert!(rendered.contains("#Tag #Other"), "{rendered}");
    assert_eq!(rendered.matches("#Tag connect").count(), 2, "{rendered}");
    assert!(!rendered.contains("extended"), "{rendered}");
    let reparsed = crate::compile_sysml_text(&rendered, "extensions.sysml", &library).unwrap();
    assert_eq!(document.elements.len(), reparsed.elements.len());
}

#[test]
fn release_2026_08_kerml_type_and_association_structure_retain_structure() {
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/metaclass-preservation/classifiers.kerml");
    let module = crate::kerml::parse_kerml(source).unwrap();
    let library = crate::kerml::load_kernel_baseline().unwrap();
    let document = crate::kerml::compile_kerml_module(&module, "classifiers.kerml", &library).unwrap();
    for (name, kind) in [("Specialized", "Type"), ("MoreSpecialized", "Type"), ("Conjugate", "Type"), ("Unioned", "Type"), ("Pair", "AssociationStructure")] {
        let element = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
        assert_eq!(element.kind.rsplit("::").next(), Some(kind), "{element:?}");
        if name == "MoreSpecialized" {
            assert!(element.properties["specializes"].to_string().contains("Classifiers.Specialized"), "{element:?}");
        }
    }
    for name in ["contained", "left", "right"] {
        let element = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
        assert!(element.properties["type"].to_string().contains("Classifiers.Base"), "{element:?}");
    }
}

#[test]
fn release_2026_08_semantic_metadata_applies_base_specializations() {
    let library = load_sysml_baseline().unwrap();
    let definitions = include_str!("../../../mercurio-tools/corpus/release-2026-08/semantic-metadata/definitions.sysml");
    let uses = include_str!("../../../mercurio-tools/corpus/release-2026-08/semantic-metadata/uses.sysml");
    let modules = [parse_sysml(definitions).unwrap(), parse_sysml(uses).unwrap()];
    let combined = parse_sysml(&format!("{definitions}\n{uses}")).unwrap();
    for (module, context) in [(&modules[1], modules.as_slice()), (&combined, std::slice::from_ref(&combined))] {
        let document = crate::compile_sysml_module_with_context(module, "metadata.sysml", context, &library).unwrap();
        let named = |name: &str| document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|value| value == name)).unwrap();
        for (name, parent) in [("Extended", "Device"), ("Overridden", "Other"), ("Direct", "Device")] {
            let element = named(name);
            assert!(element.properties["specializes"].to_string().contains(&format!("MetadataBases.{parent}")), "{name}: {element:?}");
        }
        for (name, base, ty) in [("usage", "devices", "Device"), ("overridden", "others", "Other"), ("stacked", "devices", "Device")] {
            let element = named(name);
            assert!(element.properties["subsetted_features"].to_string().contains(&format!("MetadataBases.{base}")), "{name}: {element:?}");
            assert!(element.properties["type"].to_string().contains(&format!("MetadataBases.{ty}")), "{name}: {element:?}");
        }
        assert!(!named("Overridden").properties["specializes"].to_string().contains("MetadataBases.Device"));
        for name in ["untyped", "classifierOnUsage"] {
            assert!(!named(name).properties.get("subsetted_features").is_some_and(|v| v.to_string().contains("MetadataBases")), "{name}: {:?}", named(name));
        }
        for element in document.elements.iter().filter(|e| e.properties.get("declared_name").is_some_and(|v| v == "nested")) {
            assert_eq!(element.properties["is_variable"], true, "{element:?}");
            assert_eq!(element.properties["may_time_vary"], true, "{element:?}");
        }
        assert!(!named("UntypedDefinition").properties.get("specializes").is_some_and(|v| v.to_string().contains("MetadataBases")));
    }
}

#[test]
fn release_2026_08_kerml_connectors_preserve_ends_and_types() {
    let library = crate::kerml::load_kernel_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/kernel-connectors.kerml");
    let module = crate::kerml::parse_kerml(source).unwrap();
    let document = crate::kerml::compile_kerml_module(&module, "kernel-connectors.kerml", &library).unwrap();
    let named = |name: &str| document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
    for (name, kind, count, ty) in [
        ("binary", "Connector", 2, "Links::BinaryLink"),
        ("ternary", "Connector", 3, "Links::Link"),
        ("typedLink", "Connector", 2, "KernelConnectors.Pair"),
        ("bound", "Connector", 0, "Links::BinaryLink"),
        ("withBody", "Connector", 2, "Links::BinaryLink"),
        ("equal", "BindingConnector", 2, "Links::SelfLink"),
        ("typedBinding", "BindingConnector", 2, "Links::SelfLink"),
        ("priorNext", "Succession", 2, "Occurrences::HappensBefore"),
        ("typedOrder", "Succession", 2, "Occurrences::HappensBefore"),
        ("decorated", "Connector", 2, "Links::BinaryLink"),
    ] {
        let connector = named(name);
        assert_eq!(connector.kind.rsplit("::").next(), Some(kind), "{connector:?}");
        let ends = document.elements.iter().filter(|e| e.properties.get("owner").is_some_and(|v| v == &connector.id) && e.properties.contains_key("owned_reference_subsetting")).collect::<Vec<_>>();
        assert_eq!(ends.len(), count, "{name}: {connector:?}");
        if count > 0 {
            assert_eq!(connector.properties["source"], named("a").id);
            let target_count = connector.properties["target"].as_array().map_or(1, Vec::len);
            assert_eq!(target_count, count - 1);
        }
        for (index, end) in ends.iter().enumerate() {
            assert_eq!(end.properties["is_end"], true, "{end:?}");
            let relation = document.elements.iter().find(|e| Some(e.id.as_str()) == end.properties["owned_reference_subsetting"].as_str()).unwrap();
            assert!(relation.properties["referenced_feature"].as_str().unwrap().ends_with([".a", ".b", ".c"][index]), "{relation:?}");
            assert!(end.properties["type"].to_string().contains("KernelConnectors.Node"), "{end:?}");
        }
        // The release candidate supplies these implicit library types.
        if library.elements.iter().any(|e| e.id == "Links::binaryLinks") {
            assert!(connector.properties["type"].to_string().contains(ty), "{name}: {connector:?}");
        }
    }
    assert_eq!(named("binary").properties["declared_short_name"], "pair");
    assert_eq!(named("bound").properties["is_abstract"], true);
    assert_eq!(named("nested").properties["owner"], named("decorated").id);
}

#[test]
fn release_2026_08_kerml_connector_syntax_requires_complete_ends() {
    for source in [
        "package P { connector c (a); }",
        "package P { connector c from a to; }",
        "package P { binding b of a =; }",
        "package P { succession s first a then; }",
        "package P { feature references a references b; }",
    ] {
        assert!(crate::kerml::parse_kerml(source).is_err(), "{source}");
    }
}

#[test]
fn release_2026_08_named_sysml_binding_and_succession_ends() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/sysml-connectors.sysml");
    let document = crate::compile_sysml_text(source, "sysml-connectors.sysml", &library).unwrap();
    for (kind, count) in [("BindingConnectorAsUsage", 3), ("SuccessionAsUsage", 4)] {
        let connectors = document.elements.iter().filter(|e| e.kind.rsplit("::").next() == Some(kind)).collect::<Vec<_>>();
        assert_eq!(connectors.len(), count);
        for connector in connectors {
            let ends = document.elements.iter().filter(|e| e.properties.get("owner").is_some_and(|v| v == &connector.id) && e.properties.contains_key("owned_reference_subsetting")).collect::<Vec<_>>();
            assert_eq!(ends.len(), 2, "{connector:?}");
        }
    }
    assert!(document.elements.iter().any(|e| e.properties.get("declared_name").is_some_and(|v| v == "namedOrder")));
}

#[test]
fn release_2026_08_connector_projection_matches_pilot() {
    let library = load_sysml_baseline().unwrap();
    if !library.elements.iter().any(|e| e.id == "Links::binaryLinks") { return; }
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/connectors-pilot-results.json")).unwrap();
    let normalize = |value: &serde_json::Value| -> Vec<String> {
        let values = value.as_array().cloned().unwrap_or_else(|| if value.is_null() { vec![] } else { vec![value.clone()] });
        values.iter().filter_map(serde_json::Value::as_str).map(|id| {
            if let Some((prefix, rest)) = id.split_once('.') {
                if matches!(prefix, "feature" | "type" | "part" | "binding" | "succession" | "connection") { return rest.replace('.', "::"); }
            }
            id.to_string()
        }).collect()
    };
    for (file, source) in [
        ("kernel-connectors.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/kernel-connectors.kerml")),
        ("sysml-connectors.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/sysml-connectors.sysml")),
    ] {
        let document = if file.ends_with(".kerml") {
            crate::kerml::compile_kerml_module(&crate::kerml::parse_kerml(source).unwrap(), file, &library).unwrap()
        } else { crate::compile_sysml_text(source, file, &library).unwrap() };
        for row in oracle["named_declarations"].as_array().unwrap().iter().filter(|r| r["file"] == file) {
            let element = document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&row["name"])).unwrap();
            assert_eq!(element.kind.rsplit("::").next().unwrap(), row["kind"].as_str().unwrap(), "{row:?}");
            for field in ["type", "source", "target"] {
                let mut actual = normalize(element.properties.get(field).unwrap_or(&serde_json::Value::Null));
                let mut expected = normalize(&row[field]);
                if field == "type" { actual.sort(); actual.dedup(); expected.sort(); expected.dedup(); }
                assert_eq!(actual, expected, "{} {}: {element:?}", row["name"], field);
            }
        }
    }
}

#[test]
fn release_2026_08_kerml_return_and_redefinition_headers_keep_types() {
    let module = crate::kerml::parse_kerml("package P { class Value; class Container { :>> self : Container; expr expression { :>> that : Container; return : Value; } } }").unwrap();
    let Declaration::Package(package) = &module.members[0] else { panic!("package"); };
    let Declaration::GenericDefinition(container) = &package.members[1] else { panic!("class"); };
    let Declaration::GenericUsage(own_self) = &container.members[0] else { panic!("self"); };
    assert_eq!(own_self.ty.as_ref().unwrap().as_colon_string(), "Container");
    assert_eq!(own_self.redefines[0].as_colon_string(), "self");
    assert!(own_self.is_implicit_name);
    let Declaration::GenericUsage(expression) = &container.members[1] else { panic!("expression"); };
    let Declaration::GenericUsage(result) = &expression.body_members[1] else { panic!("result"); };
    assert_eq!(result.ty.as_ref().unwrap().as_colon_string(), "Value");
    assert!(result.modifiers.iter().any(|m| m == "return"));
    assert!(result.is_implicit_name);
}

#[test]
fn release_2026_08_expression_inherits_feature_subsettings() {
    let library = load_sysml_baseline().unwrap();
    if !library.elements.iter().any(|e| e.id == "Base::things::that") { return; }
    let source = "package P { class Container; class Scope { expr expression { :>> that : Container; return result : Container; binding result.portionOf = that; } } }";
    let module = crate::kerml::parse_kerml(source).unwrap();
    let document = crate::kerml::compile_kerml_module_strict_with_context(
        &module, "inherited-expression.kerml", std::slice::from_ref(&module), &library,
    ).unwrap();
    let that = document.elements.iter().find(|e| e.id == "feature.P.Scope.expression.that").unwrap();
    assert!(that.properties["redefined_features"].to_string().contains("Base::things::that"), "{that:?}");
    let expression = document.elements.iter().find(|e| e.id == "feature.P.Scope.expression").unwrap();
    assert!(expression.properties["type"].to_string().contains("Performances::Evaluation"), "{expression:?}");
    let invalid = source.replace(":>> that :", ":>> nonexistent :");
    let module = crate::kerml::parse_kerml(&invalid).unwrap();
    let error = crate::kerml::compile_kerml_module_strict_with_context(
        &module, "invalid-inherited-expression.kerml", std::slice::from_ref(&module), &library,
    ).unwrap_err();
    assert!(error.to_string().contains("unresolved redefinition target `nonexistent`"), "{error:?}");
}

#[test]
fn release_2026_08_crossing_feature_is_owned_by_end() {
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/kernel-crossing-features.kerml");
    let module = crate::kerml::parse_kerml(source).unwrap();
    let Declaration::Package(package) = &module.members[0] else { panic!("package"); };
    let Declaration::GenericDefinition(association) = &package.members[1] else { panic!("association"); };
    assert_eq!(association.members.len(), 3);
    let Declaration::GenericUsage(first) = &association.members[0] else { panic!("end"); };
    assert_eq!(first.name, "left");
    assert!(first.modifiers.iter().any(|m| m == "end"));
    let Declaration::GenericUsage(crossing) = &first.body_members[0] else { panic!("crossing"); };
    assert_eq!(crossing.name, "crossing");
    assert!(!crossing.modifiers.iter().any(|m| m == "end" || m == "const"));
    assert_ne!(first.multiplicity, crossing.multiplicity);
    let Declaration::GenericUsage(second) = &association.members[1] else { panic!("end"); };
    let Declaration::GenericUsage(anonymous) = &second.body_members[0] else { panic!("crossing"); };
    assert!(anonymous.is_implicit_name);
    let library = load_sysml_baseline().unwrap();
    let document = crate::kerml::compile_kerml_module_strict_with_context(
        &module, "kernel-crossing-features.kerml", std::slice::from_ref(&module), &library,
    ).unwrap();
    for name in ["left", "right"] {
        let end = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
        assert_eq!(end.properties["is_end"], true);
        let cross = document.elements.iter().find(|e| Some(e.id.as_str()) == end.properties["cross_feature"].as_str()).unwrap();
        assert_eq!(cross.properties["is_end"], false);
        assert_eq!(cross.properties["owner"], end.id);
        assert!(cross.properties["type"].to_string().contains("CrossingFeatures.Value"), "{cross:?}");
    }
}

#[test]
fn release_2026_08_typed_binding_is_never_variable() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/typed-binding-variability.sysml");
    let document = crate::compile_sysml_text(source, "typed-binding-variability.sysml", &library).unwrap();
    let binding = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "bound")).unwrap();
    assert_eq!(binding.properties["is_variable"], false);
    assert_eq!(binding.properties["may_time_vary"], false);
}

#[test]
fn release_2026_08_crossing_projection_matches_pilot() {
    let library = load_sysml_baseline().unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/crossing-pilot-results.json")).unwrap();
    let normalize = |value: &serde_json::Value| -> Vec<String> {
        let values = value.as_array().cloned().unwrap_or_else(|| if value.is_null() { vec![] } else { vec![value.clone()] });
        let mut values = values.iter().filter_map(serde_json::Value::as_str).map(|id| {
            if let Some((prefix, rest)) = id.split_once('.') {
                if matches!(prefix, "feature" | "type" | "part" | "binding") { return rest.replace('.', "::"); }
            }
            id.to_string()
        }).collect::<Vec<_>>();
        values.sort(); values.dedup(); values
    };
    for (file, source) in [
        ("kernel-crossing-features.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/kernel-crossing-features.kerml")),
        ("typed-binding-variability.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/typed-binding-variability.sysml")),
    ] {
        let document = if file.ends_with(".kerml") {
            let module = crate::kerml::parse_kerml(source).unwrap();
            crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library).unwrap()
        } else { crate::compile_sysml_text(source, file, &library).unwrap() };
        for row in oracle["named_declarations"].as_array().unwrap().iter().filter(|r| r["file"] == file) {
            let element = document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&row["name"])).unwrap();
            assert_eq!(element.kind.rsplit("::").next().unwrap(), row["kind"].as_str().unwrap(), "{row:?}");
            for field in ["type", "owner"] {
                assert_eq!(normalize(element.properties.get(field).unwrap_or(&serde_json::Value::Null)), normalize(&row[field]), "{} {field}: {element:?}", row["name"]);
            }
            for (field, expected) in row["flags"].as_object().unwrap() {
                assert_eq!(element.properties.get(field), Some(expected), "{} {field}: {element:?}", row["name"]);
            }
        }
    }
}

#[test]
fn release_2026_08_named_terminate_preserves_argument_and_body() {
    for (source, name) in [
        ("action stop terminate worker { attribute marker : ScalarValues::Boolean = true; }", Some("stop")),
        ("action terminate worker;", None),
        ("terminate worker;", None),
    ] {
        let mut parser = Parser::new(lex(source).unwrap(), false);
        let Declaration::GenericUsage(usage) = parser.parse_declaration().unwrap().unwrap() else { panic!("terminate"); };
        assert_eq!(usage.keyword, "terminate");
        assert_eq!(!usage.is_implicit_name, name.is_some());
        if let Some(name) = name { assert_eq!(usage.name, name); }
        assert!(usage.body_members.iter().any(|member| matches!(member, Declaration::GenericUsage(u)
            if u.name == "terminatedOccurrence" && matches!(&u.expression, Some(Expr::Name(n)) if n.as_dot_string() == "worker"))));
        if name.is_some() {
            assert!(usage.body_members.iter().any(|member| matches!(member, Declaration::GenericUsage(u) if u.name == "marker")));
        }
        assert!(matches!(parser.peek_kind(), TokenKind::Eof));
    }
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/terminate-flow-declarations.sysml");
    let kir = crate::compile_sysml_text(source, "declarations.sysml", &library).unwrap();
    for name in ["stop", "finish"] {
        let element = kir.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|n| n == name)).unwrap();
        assert_eq!(element.kind, "TerminateActionUsage");
    }
    let invalid = source.replace("terminate worker", "terminate unknownWorker");
    assert!(crate::compile_sysml_text(&invalid, "invalid-terminate.sysml", &library).is_err());
}

#[test]
fn release_2026_08_flow_payload_preserves_metaclass_and_declared_name() {
    for (source, implicit) in [
        ("flow of fuel : Fuel from source to target;", false),
        ("flow of Fuel from source to target;", true),
    ] {
        let mut parser = Parser::new(lex(source).unwrap(), false);
        let Declaration::GenericUsage(flow) = parser.parse_declaration().unwrap().unwrap() else { panic!("flow"); };
        let Declaration::GenericUsage(payload) = &flow.body_members[0] else { panic!("payload"); };
        assert_eq!(payload.keyword, "payload");
        assert_eq!(payload.is_implicit_name, implicit);
        assert_eq!(payload.ty.as_ref().unwrap().as_dot_string(), "Fuel");
        assert!(payload.span.start_col > flow.span.start_col);
    }
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/terminate-flow-declarations.sysml");
    let kir = crate::compile_sysml_text(source, "payloads.sysml", &library).unwrap();
    let payloads = kir.elements.iter().filter(|e| e.kind == "PayloadFeature").collect::<Vec<_>>();
    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads.iter().filter(|e| e.properties.get("declared_name").is_some_and(|n| n == "fuel")).count(), 1);
    assert!(!payloads.iter().any(|e| e.properties.get("declared_name").is_some_and(|n| n == "payload")));
}

#[test]
fn release_2026_08_crossing_prefix_metadata_does_not_declare_a_feature() {
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/crossing-metadata-prefix.kerml");
    let module = crate::kerml::parse_kerml(source).unwrap();
    let Declaration::Package(package) = &module.members[0] else { panic!("package"); };
    let Declaration::GenericDefinition(association) = &package.members[2] else { panic!("association"); };
    assert_eq!(association.members.len(), 3);
    for (index, name) in ["plain", "crossed", "anonymous"].into_iter().enumerate() {
        let Declaration::GenericUsage(end) = &association.members[index] else { panic!("end"); };
        assert_eq!(end.name, name);
        assert_eq!(end.body_members.len(), usize::from(index > 0));
        if index > 0 {
            let Declaration::GenericUsage(cross) = &end.body_members[0] else { panic!("crossing"); };
            assert_eq!(cross.is_implicit_name, index == 2);
            if index == 1 { assert_eq!(cross.name, "cross"); }
            assert!(!cross.modifiers.iter().any(|m| m == "end"));
        }
    }
    let library = load_sysml_baseline().unwrap();
    let kir = crate::kerml::compile_kerml_module_strict_with_context(
        &module, "metadata-prefix.kerml", std::slice::from_ref(&module), &library,
    ).unwrap();
    let plain = kir.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|n| n == "plain")).unwrap();
    assert!(!plain.properties.contains_key("cross_feature"));
}

#[test]
fn release_2026_08_followup_declarations_match_pilot() {
    let library = load_sysml_baseline().unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/declaration-followup-pilot-results.json")).unwrap();
    let normalize = |value: &serde_json::Value| -> Vec<String> {
        let values = value.as_array().cloned().unwrap_or_else(|| if value.is_null() { vec![] } else { vec![value.clone()] });
        let mut values = values.iter().filter_map(serde_json::Value::as_str).map(|id| {
            if let Some((prefix, rest)) = id.split_once('.') {
                if matches!(prefix, "feature" | "type" | "part" | "action" | "terminate" | "item") { return rest.replace('.', "::"); }
            }
            id.to_string()
        }).collect::<Vec<_>>();
        values.sort(); values.dedup(); values
    };
    for (file, source) in [
        ("crossing-metadata-prefix.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/crossing-metadata-prefix.kerml")),
        ("terminate-flow-declarations.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/terminate-flow-declarations.sysml")),
    ] {
        let document = if file.ends_with(".kerml") {
            let module = crate::kerml::parse_kerml(source).unwrap();
            crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library).unwrap()
        } else { crate::compile_sysml_text(source, file, &library).unwrap() };
        for row in oracle["named_declarations"].as_array().unwrap().iter().filter(|r| r["file"] == file) {
            let matches = document.elements.iter().filter(|e| e.properties.get("declared_name") == Some(&row["name"]) && e.kind.rsplit("::").next().unwrap() == row["kind"].as_str().unwrap()).collect::<Vec<_>>();
            assert_eq!(matches.len(), 1, "{row:?}");
            for field in ["type", "owner"] {
                if row.get(field).is_some() {
                    assert_eq!(normalize(matches[0].properties.get(field).unwrap_or(&serde_json::Value::Null)), normalize(&row[field]), "{} {field}: {:?}", row["name"], matches[0]);
                }
            }
        }
    }
}

#[test]
fn release_2026_08_variable_feature_legality_matches_pilot() {
    let library = load_sysml_baseline().unwrap();
    if !library.elements.iter().any(|e| e.id == "Occurrences::Occurrence") { return; }
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/feature-validation-pilot-results.json")).unwrap();
    for (file, source) in [
        ("variable-class-positive.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/variable-class-positive.kerml")),
        ("variable-datatype-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/variable-datatype-negative.kerml")),
        ("constant-association-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/constant-association-negative.kerml")),
        ("constant-association-structure-positive.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/constant-association-structure-positive.kerml")),
        ("variable-portion-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/variable-portion-negative.kerml")),
        ("variable-package-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/variable-package-negative.kerml")),
    ] {
        let row = oracle["cases"].as_array().unwrap().iter().find(|r| r["relative_path"] == file).unwrap();
        let module = crate::kerml::parse_kerml(source).unwrap();
        let result = crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library);
        if row["status"] == "error" {
            let error = result.unwrap_err().to_string();
            for issue in row["diagnostics"].as_array().unwrap() {
                assert!(error.contains(issue["code"].as_str().unwrap()), "{file}: {error}");
                assert!(error.contains(issue["message"].as_str().unwrap()), "{file}: {error}");
            }
        } else {
            let document = result.unwrap();
            for expected in oracle["positive_flags"].as_array().unwrap().iter().filter(|r| r["file"] == file) {
                let element = document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&expected["name"])).unwrap();
                assert_eq!(element.kind.rsplit("::").next().unwrap(), expected["kind"].as_str().unwrap());
                for (field, value) in expected["flags"].as_object().unwrap() { assert_eq!(element.properties.get(field), Some(value), "{file} {field}"); }
            }
        }
    }
}

#[test]
fn release_2026_08_inherited_portions_keep_library_feature_identity() {
    let library = load_sysml_baseline().unwrap();
    if !library.elements.iter().any(|e| e.id == "Occurrences::Occurrence::timeSlices") { return; }
    for source in [
        "package P { class Owner { portion slice :> timeSlices { var feature changing; portion inner :> timeSlices { var feature another; } } } }",
        "package P { struct Person { var feature licensed : ScalarValues::Boolean; } struct Car { var feature driver : Person; portion operated :> timeSlices { var feature :>> driver { var feature :>> licensed = true; } } } }",
    ] {
        let module = crate::kerml::parse_kerml(source).unwrap();
        let kir = crate::kerml::compile_kerml_module_strict_with_context(&module, "inherited-portions.kerml", std::slice::from_ref(&module), &library).unwrap();
        assert!(!kir.elements.iter().any(|e| e.properties.values().any(|v| v.to_string().contains("feature.Occurrences::"))));
        assert!(kir.elements.iter().any(|e| e.properties.get("specializes").is_some_and(|v| v.to_string().contains("Occurrences::Occurrence::timeSlices"))));
    }
    let module = crate::kerml::parse_kerml("package P { datatype Value; class Owner { feature value : Value { var feature invalid; } } }").unwrap();
    let error = crate::kerml::compile_kerml_module_strict_with_context(&module, "non-occurrence-feature-owner.kerml", std::slice::from_ref(&module), &library).unwrap_err();
    assert!(error.to_string().contains("validateFeatureIsVariable"), "{error:?}");
}

#[test]
fn release_2026_08_requirement_constraints_have_real_memberships() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/requirement-constraint-controls.sysml");
    let module = parse_sysml(source).unwrap();
    let Declaration::Package(package) = &module.members[0] else { panic!("package"); };
    let Declaration::GenericDefinition(need) = &package.members[2] else { panic!("requirement"); };
    for member in &need.members[2..] {
        let Declaration::GenericUsage(usage) = member else { panic!("reference"); };
        assert!(usage.is_implicit_name);
        assert_eq!(usage.reference_target.as_ref().unwrap().as_colon_string(), "available");
    }
    let document = crate::compile_sysml_text(source, "requirement-controls.sysml", &library).unwrap();
    assert!(!document.elements.iter().any(|e| matches!(e.kind.rsplit("::").next(), Some("AssumeUsage" | "RequireUsage"))));
    mercurio_foundation::KirDocument::merge_with_registered_fields(vec![document.clone()], crate::sysml_field_specs().iter().copied()).unwrap();
    let memberships = document.elements.iter().filter(|e| e.kind.rsplit("::").next() == Some("RequirementConstraintMembership")).collect::<Vec<_>>();
    assert_eq!(memberships.len(), 4);
    let need = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|n| n == "Need")).unwrap();
    let mut anonymous = 0;
    for membership in memberships {
        let member_id = membership.properties["member_element"].as_str().unwrap();
        let constraint = document.elements.iter().find(|e| e.id == member_id).unwrap();
        assert_eq!(constraint.kind.rsplit("::").next().unwrap(), "ConstraintUsage");
        assert_eq!(constraint.properties["owner"], need.id);
        assert_eq!(constraint.properties["owning_relationship"], membership.id);
        assert_eq!(membership.properties["owning_related_element"], need.id);
        assert_eq!(membership.properties["owned_related_element"], serde_json::json!([constraint.id]));
        assert_eq!(constraint.properties["owning_membership"], membership.id);
        assert_eq!(constraint.properties["owning_feature_membership"], membership.id);
        assert_eq!(membership.properties["membership_owning_namespace"], need.id);
        assert!(need.properties["owned_membership"].to_string().contains(&membership.id));
        assert!(need.properties["owned_relationship"].to_string().contains(&membership.id));
        if constraint.properties.get("declared_name").is_none_or(|n| n.is_null()) {
            anonymous += 1;
            let subset_id = constraint.properties["owned_reference_subsetting"].as_str().unwrap();
            let subset = document.elements.iter().find(|e| e.id == subset_id).unwrap();
            assert!(subset.kind.ends_with("ReferenceSubsetting"));
            assert!(subset.properties["subsetted_feature"].to_string().contains("available"));
        } else if constraint.properties["declared_name"] == "assumed" {
            assert_eq!(membership.properties["kind"], "assumption");
            assert_eq!(membership.properties["visibility"], "private");
        } else {
            assert_eq!(membership.properties["kind"], "requirement");
            assert_eq!(membership.properties["visibility"], "public");
        }
    }
    assert_eq!(anonymous, 2);
}

#[test]
fn release_2026_08_requirement_constraint_rejects_wrong_type() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/requirement-constraint-negative.sysml");
    let error = crate::compile_sysml_text(source, "requirement-negative.sysml", &library).unwrap_err();
    assert!(error.to_string().contains("Invalid Constraint Usage - invalid type"), "{error:?}");
    assert!(error.to_string().contains("A constraint must be typed by one constraint definition."), "{error:?}");
}

#[test]
fn release_2026_08_requirement_constraint_references_keep_qualified_targets() {
    for source in ["assume Other::available;", "require Other::available;"] {
        let mut parser = Parser::new(lex(source).unwrap(), false);
        let Declaration::GenericUsage(usage) = parser.parse_declaration().unwrap().unwrap() else { panic!("reference"); };
        assert!(usage.is_implicit_name);
        assert_eq!(usage.reference_target.as_ref().unwrap().as_colon_string(), "Other::available");
        assert!(matches!(parser.peek_kind(), TokenKind::Eof));
    }
}

#[test]
fn release_2026_08_requirement_membership_projection_matches_pilot() {
    let library = load_sysml_baseline().unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/requirement-constraint-pilot-results.json")).unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/requirement-constraint-controls.sysml");
    let document = crate::compile_sysml_text(source, "requirement-controls.sysml", &library).unwrap();
    let normalize = |value: &serde_json::Value| -> Vec<String> {
        let values = value.as_array().cloned().unwrap_or_else(|| if value.is_null() { vec![] } else { vec![value.clone()] });
        let mut values = values.iter().filter_map(serde_json::Value::as_str).map(|id| {
            if let Some((prefix, rest)) = id.split_once('.') {
                if matches!(prefix, "type" | "feature" | "constraint") { return rest.replace('.', "::"); }
            }
            id.to_string()
        }).collect::<Vec<_>>();
        values.sort(); values.dedup(); values
    };
    for row in oracle["memberships"].as_array().unwrap() {
        let membership = document.elements.iter().find(|e| e.kind.ends_with("RequirementConstraintMembership") && e.properties["metadata"]["source_span"]["start_line"] == row["line"]).unwrap();
        for field in ["kind", "visibility"] { assert_eq!(membership.properties.get(field), Some(&row[field]), "line {} {field}", row["line"]); }
        assert_eq!(normalize(&membership.properties["membership_owning_namespace"]), normalize(&row["owner"]));
        let member = document.elements.iter().find(|e| Some(e.id.as_str()) == membership.properties["member_element"].as_str()).unwrap();
        assert_eq!(member.kind.rsplit("::").next().unwrap(), row["member_kind"].as_str().unwrap());
        assert_eq!(member.properties.get("declared_name").unwrap_or(&serde_json::Value::Null), &row["member_declared_name"]);
        assert_eq!(normalize(member.properties.get("type").unwrap_or(&serde_json::Value::Null)), normalize(&row["type"]), "line {}", row["line"]);
        if !row["reference_target_line"].is_null() {
            let subset = document.elements.iter().find(|e| Some(e.id.as_str()) == member.properties["owned_reference_subsetting"].as_str()).unwrap();
            let target = document.elements.iter().find(|e| Some(e.id.as_str()) == subset.properties["subsetted_feature"].as_str()).unwrap();
            assert_eq!(target.properties["metadata"]["source_span"]["start_line"], row["reference_target_line"], "line {}", row["line"]);
            if !row["reference_target"].is_null() { assert_eq!(normalize(&serde_json::json!(target.id)), normalize(&row["reference_target"])); }
        }
    }
}

#[test]
fn release_2026_08_relationship_projection_matches_pilot() {
    let library = crate::kerml::load_kernel_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-declarations.kerml");
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-declarations-pilot-results.json")).unwrap();
    let module = crate::kerml::parse_kerml(source).unwrap();
    let document = crate::kerml::compile_kerml_module_strict_with_context(&module, "relationship-declarations.kerml", std::slice::from_ref(&module), &library).unwrap();
    mercurio_foundation::KirDocument::merge_with_registered_fields(vec![document.clone()], crate::sysml_field_specs().iter().copied()).unwrap();
    let normalize = |value: &serde_json::Value| -> Vec<String> {
        value.as_array().cloned().unwrap_or_else(|| if value.is_null() { vec![] } else { vec![value.clone()] }).iter().map(|v| {
            let id = v.as_str().unwrap();
            if let Some((prefix, rest)) = id.split_once('.') {
                if matches!(prefix, "type" | "feature" | "pkg" | "relationship") { return rest.replace('.', "::"); }
            }
            id.to_string()
        }).collect()
    };
    let mut seen = std::collections::BTreeSet::new();
    for row in oracle["relationships"].as_array().unwrap() {
        let matches: Vec<_> = document.elements.iter().filter(|e| e.kind.rsplit("::").next() == row["kind"].as_str()
            && e.properties["metadata"]["source_span"]["start_line"] == row["line"]).collect();
        assert_eq!(matches.len(), 1, "line {} {}",row["line"],row["kind"]);
        let element = matches[0];
        assert!(seen.insert(&element.id), "relationship identity collapsed");
        for field in ["declared_name", "declared_short_name"] {
            assert_eq!(element.properties.get(field).unwrap_or(&serde_json::Value::Null), &row[field], "line {} {field}: {element:?}",row["line"]);
        }
        assert_eq!(normalize(&element.properties["owner"]), normalize(&row["owner"]));
        for (field, expected) in row["references"].as_object().unwrap() {
            assert_eq!(normalize(element.properties.get(field).unwrap_or(&serde_json::Value::Null)), normalize(expected), "line {} {field}: {element:?}",row["line"]);
        }
        assert_eq!(element.properties["is_implied"], false);
        assert!(!element.properties.contains_key("__relationship_endpoints"));
    }
    assert_eq!(seen.len(),18);
    for name in ["f", "g", "A", "B"] {
        assert_eq!(document.elements.iter().filter(|e| e.properties.get("declared_name").is_some_and(|v| v==name)).count(),1,"operand {name} was redeclared");
    }
}

#[test]
fn release_2026_08_relationship_missing_endpoints_match_pilot_rejections() {
    let library = crate::kerml::load_kernel_baseline().unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-declarations-pilot-results.json")).unwrap();
    let cases = [
        ("relationship-subclassification-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-subclassification-negative.kerml")),
        ("relationship-specialization-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-specialization-negative.kerml")),
        ("relationship-feature-typing-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-feature-typing-negative.kerml")),
        ("relationship-subsetting-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-subsetting-negative.kerml")),
        ("relationship-redefinition-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-redefinition-negative.kerml")),
        ("relationship-conjugation-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-conjugation-negative.kerml")),
        ("relationship-inverting-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-inverting-negative.kerml")),
        ("relationship-featuring-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-featuring-negative.kerml")),
        ("relationship-dependency-negative.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-dependency-negative.kerml")),
    ];
    for (name, source) in cases {
        let row = oracle["cases"].as_array().unwrap().iter().find(|c| c["relative_path"]==name).unwrap();
        assert_eq!(row["status"], "error");
        assert!(row["diagnostics"].as_array().unwrap().iter().any(|d| d["code"]=="org.eclipse.xtext.diagnostics.Diagnostic.Linking"));
        let module = crate::kerml::parse_kerml(source).unwrap();
        let error = crate::kerml::compile_kerml_module_strict_with_context(&module,name,std::slice::from_ref(&module),&library).unwrap_err();
        assert!(error.to_string().contains("unresolved relationship endpoint `Missing`"),"{name}: {error:?}");
    }
}

#[test]
fn release_2026_08_relationship_syntax_requires_complete_operands() {
    for source in ["specialization X subclassifier A :>;", "subtype A specializes;", "typing f typed by;",
        "subset f subsets;", "redefinition f redefines;", "conjugation X conjugate A ~;",
        "inverting X inverse f of;", "featuring X of f by;", "dependency X from A, to B;",
        "dependency A to;", "specialization X;", "conjugation X inverse f of g;"] {
        assert!(crate::kerml::parse_kerml(source).is_err(),"accepted incomplete relationship: {source}");
    }
}

#[test]
fn release_2026_08_relationship_package_and_relationship_endpoints_keep_identity() {
    let library = crate::kerml::load_kernel_baseline().unwrap();
    for source in [include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-package-endpoints.kerml"),
        "package P { classifier A; feature f; specialization T typing f : A; dependency D from T, T to T; dependency E from P to D; }"] {
        let module = crate::kerml::parse_kerml(source).unwrap();
        let document = crate::kerml::compile_kerml_module_strict_with_context(&module,"relationship-endpoints.kerml",std::slice::from_ref(&module),&library).unwrap();
        for relation in document.elements.iter().filter(|e| e.kind.ends_with("Dependency")) {
            if relation.properties.get("declared_name").is_some_and(|v| v=="D") {
                assert_eq!(relation.properties["source"].as_array().unwrap().len(),1);
                assert_eq!(relation.properties["related_element"].as_array().unwrap().len(),1);
            }
            for field in ["source", "target"] {
                for id in relation.properties[field].as_array().unwrap() {
                    assert!(document.elements.iter().any(|e| Some(e.id.as_str())==id.as_str()), "dangling {field}: {id}, {relation:?}");
                }
            }
        }
    }
}

#[test]
fn release_2026_08_relationship_feature_chain_keeps_each_step() {
    let library = crate::kerml::load_kernel_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-feature-chain.kerml");
    let module = crate::kerml::parse_kerml(source).unwrap();
    let document = crate::kerml::compile_kerml_module_strict_with_context(&module,"Inverses.kerml",std::slice::from_ref(&module),&library).unwrap();
    mercurio_foundation::KirDocument::merge_with_registered_fields(vec![document.clone()], crate::sysml_field_specs().iter().copied()).unwrap();
    let inverse = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v|v=="Invert")).unwrap();
    let chain_id = inverse.properties["source"][0].as_str().unwrap();
    let chain = document.elements.iter().find(|e|e.id==chain_id).unwrap();
    assert!(chain.kind.ends_with("Feature"));
    assert_eq!(chain.properties["metadata"]["lowering"]["metaclass"],"SysML::Feature");
    assert!(chain.properties.get("declared_name").is_none());
    assert_eq!(chain.properties["chaining_feature"],serde_json::json!(["feature.Inverses.B.g","feature.Inverses.A.f"]));
    assert_eq!(chain.properties["type"],"type.Inverses.B");
    assert_eq!(chain.properties["owning_relationship"],inverse.id);
    assert_eq!(inverse.properties["feature_inverted"],chain.id);
    let links=chain.properties["owned_relationship"].as_array().unwrap();
    assert_eq!(links.len(),2);
    for (i,id) in links.iter().enumerate() {
        let link=document.elements.iter().find(|e|Some(e.id.as_str())==id.as_str()).unwrap();
        assert!(link.kind.ends_with("FeatureChaining"));
        assert_eq!(link.properties["source"],serde_json::json!([chain.id]));
        assert_eq!(link.properties["chaining_feature"],chain.properties["chaining_feature"][i]);
    }
    let bad=source.replace("B::g.f of", "B::g.missing of");
    let module=crate::kerml::parse_kerml(&bad).unwrap();
    assert!(crate::kerml::compile_kerml_module_strict_with_context(&module,"invalid-chain.kerml",std::slice::from_ref(&module),&library).is_err());
}


#[test]
fn release_2026_08_textual_projection_matches_pilot() {
    let library = load_sysml_baseline().unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../mercurio-tools/corpus/release-2026-08/textual-pilot-results.json")).unwrap();
    for (file, source, kernel) in [
        ("textual-invariants.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/textual-invariants.kerml"), true),
        ("textual-representations.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/textual-representations.sysml"), false),
    ] {
        let module = if kernel { crate::kerml::parse_kerml(source) } else { parse_sysml(source) }.unwrap();
        let document = if kernel {
            crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library)
        } else { compile_sysml_text(source, file, &library) }.unwrap();
        mercurio_foundation::KirDocument::merge_with_registered_fields(vec![document.clone()], crate::sysml_field_specs().iter().copied()).unwrap();
        let reps = document.elements.iter().filter(|e| e.kind.ends_with("TextualRepresentation")).collect::<Vec<_>>();
        assert_eq!(reps.len(), 3);
        for row in oracle["projections"].as_array().unwrap().iter().filter(|r| r["file"] == file) {
            let props = row["properties"].as_object().unwrap();
            let element = document.elements.iter().find(|e| {
                e.kind.rsplit("::").next() == row["kind"].as_str()
                    && if row["kind"] == "TextualRepresentation" { e.properties.get("language") == props.get("language") }
                        else { e.properties.get("declared_name") == props.get("declared_name") }
            }).unwrap();
            for (key, value) in props { assert_eq!(element.properties.get(key), Some(value), "{file} {key}"); }
            let owner = document.elements.iter().find(|e| element.properties.get("owner").and_then(|v| v.as_str()) == Some(e.id.as_str())).unwrap();
            let pilot_owner = row["references"]["owner"][0].as_str().unwrap().rsplit("::").next().unwrap();
            assert_eq!(owner.properties["declared_name"], pilot_owner);
            let membership = document.elements.iter().find(|e| element.properties.get("owning_membership").and_then(|v| v.as_str()) == Some(e.id.as_str())).unwrap();
            assert_eq!(membership.properties["member_element"], element.id);
            assert_eq!(membership.properties["membership_owning_namespace"], owner.id);
            if row["kind"] == "TextualRepresentation" {
                assert!(membership.kind.ends_with("OwningMembership"));
                assert_eq!(element.properties["represented_element"], owner.id);
                assert_eq!(element.properties["annotated_element"], serde_json::json!([owner.id]));
                assert_ne!(element.id, owner.id);
                assert!(!owner.properties.get("features").is_some_and(|v| v.to_string().contains(&element.id)));
                assert!(owner.properties["textual_representation"].to_string().contains(&element.id));
            } else {
                for field in ["type", "specializes", "function", "result", "parameter"] {
                    for target in row["references"][field].as_array().unwrap() {
                        assert!(element.properties[field].to_string().contains(target.as_str().unwrap()), "{} {field}", element.id);
                    }
                }
                if props["declared_name"] != "described" {
                    let result = document.elements.iter().find(|e| element.properties["result_expression"] == e.id).unwrap();
                    assert!(result.kind.ends_with("LiteralBoolean"));
                    assert_eq!(result.properties["value"], props["declared_name"] != "negative");
                    let result_membership = document.elements.iter().find(|e| result.properties["owning_membership"] == e.id).unwrap();
                    assert!(result_membership.kind.ends_with("ResultExpressionMembership"));
                    assert_eq!(result_membership.properties["owned_result_expression"], result.id);
                }
            }
        }
    }
}

#[test]
fn release_2026_08_textual_syntax_requires_language_and_body() {
    for source in [
        "package P { rep bad /* body */ }",
        "package P { rep bad language \"ocl\"; }",
        "package P { rep bad language ocl /* body */ }",
        "package P { language \"ocl\"; }",
        "package P { rep <short language \"ocl\" /* body */ }",
    ] {
        assert!(parse_sysml(source).is_err(), "SysML accepted {source}");
        assert!(crate::kerml::parse_kerml(source).is_err(), "KerML accepted {source}");
    }
}

#[test]
fn release_2026_08_textual_authoring_round_trip_preserves_verbatim_body() {
    use std::collections::BTreeMap;
    for kernel in [false, true] {
        let source = "package P { rep <s> named language \"ocl\" /*  x > 0 \n * untouched \n */ language \"alf\" /* a = b; */ }";
        let parse = |text: &str| if kernel { crate::kerml::parse_kerml(text) } else { parse_sysml(text) };
        let original = parse(source).unwrap();
        let project = mercurio_foundation::AuthoringProject::from_parsed_modules(
            BTreeMap::from([("p.sysml".into(), original.clone())]), BTreeMap::from([("p.sysml".into(), source.into())]),
        ).unwrap().with_render_profile(mercurio_foundation::textual_model_authoring_render_profile());
        let rendered = project.render_new_file("p.sysml").unwrap();
        let reparsed = parse(&rendered).unwrap();
        let before = &original.package.as_ref().unwrap().members;
        let after = &reparsed.package.as_ref().unwrap().members;
        assert_eq!(before.len(), after.len());
        for (before, after) in before.iter().zip(after) {
            let (Declaration::GenericUsage(before), Declaration::GenericUsage(after)) = (before, after) else { panic!("representations") };
            assert_eq!(before.metadata_properties, after.metadata_properties);
            assert_eq!(before.modifiers, after.modifiers);
            assert_eq!(before.is_implicit_name, after.is_implicit_name);
            assert!(after.comments.is_empty(), "body leaked into comment trivia");
        }
    }
}

#[test]
fn release_2026_08_textual_invariant_result_round_trip_and_initializer_distinction() {
    use std::collections::BTreeMap;
    let source = "package P { inv false negative { false } inv valued = true; }";
    let original = crate::kerml::parse_kerml(source).unwrap();
    let project = mercurio_foundation::AuthoringProject::from_parsed_modules(
        BTreeMap::from([("p.kerml".into(), original)]), BTreeMap::from([("p.kerml".into(), source.into())]),
    ).unwrap().with_render_profile(mercurio_foundation::textual_model_authoring_render_profile());
    let rendered = project.render_new_file("p.kerml").unwrap();
    let parsed = crate::kerml::parse_kerml(&rendered).unwrap();
    let members = &parsed.package.as_ref().unwrap().members;
    let Declaration::GenericUsage(negative) = &members[0] else { panic!("invariant") };
    assert!(negative.modifiers.iter().any(|m| m == "is_negated"));
    assert!(negative.modifiers.iter().any(|m| m == "expression_is_result"));
    let Declaration::GenericUsage(valued) = &members[1] else { panic!("invariant") };
    assert!(!valued.modifiers.iter().any(|m| m == "expression_is_result"));
    assert!(valued.expression.is_some());
}

#[test]
fn release_2026_08_textual_escaped_language_and_body_round_trip() {
    use std::collections::BTreeMap;
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/textual-escaped.sysml");
    let project = crate::authoring::load_authoring_project_from_sysml(BTreeMap::from([("escaped.sysml".into(), source.into())])).unwrap();
    let rendered = project.render_new_file("escaped.sysml").unwrap();
    let original = parse_sysml(source).unwrap();
    let reparsed = parse_sysml(&rendered).unwrap();
    let Declaration::GenericUsage(before) = &original.package.unwrap().members[0] else { panic!("rep") };
    let Declaration::GenericUsage(after) = &reparsed.package.unwrap().members[0] else { panic!("rep") };
    assert_eq!(before.metadata_properties, after.metadata_properties);
    let library = load_sysml_baseline().unwrap();
    let document = compile_sysml_text(source, "escaped.sysml", &library).unwrap();
    let rep = document.elements.iter().find(|e| e.kind.ends_with("TextualRepresentation")).unwrap();
    assert_eq!(rep.properties["language"], "a\"b\\c\n");
    assert_eq!(rep.properties["body"], r#"keep "quoted" and \\ literal "#);
}


#[test]
fn release_2026_08_flag_names_preserve_context_and_identity() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/flag-name-controls.sysml");
    let document = compile_sysml_text(source, "flag-name-controls.sysml", &library).unwrap();
    let find = |name: &str| document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
    assert_eq!(find("Vehicle").properties["declared_short_name"], "veh");
    assert_eq!(find("Need").properties["declared_short_name"], "req");
    for name in ["system", "reviewer", "car", "driver", "cycle"] {
        assert_eq!(find(name).properties["direction"], "in", "{name}");
    }
    for name in ["inCart", "products"] {
        let cross = find(name);
        assert_eq!(cross.properties["is_end"], false);
        assert_eq!(cross.properties["is_variable"], false);
        assert_eq!(cross.properties["is_constant"], false);
        let end = if name == "inCart" { find("cart") } else { find("product") };
        assert_eq!(cross.properties["owner"], end.id);
        assert_eq!(end.properties["cross_feature"], cross.id);
        assert_eq!(end.properties["is_end"], true);
    }
    assert!(find("shaft").properties["type"].to_string().contains("Interfaces::BinaryInterface"));
    assert_eq!(find("change").properties["is_variable"], false);
    for name in ["cycle", "temp", "source", "target"] {
        assert_eq!(find(name).properties["is_variable"], true, "{name}");
    }
    assert_eq!(find("cycle").properties["is_composite"], false);
    for name in ["source", "target"] { assert_eq!(find(name).properties["is_constant"], true); }
}

#[test]
fn release_2026_08_flag_names_kernel_directions_and_negative_context() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/flag-name-controls.kerml");
    let module = crate::kerml::parse_kerml(source).unwrap();
    let document = crate::kerml::compile_kerml_module_strict_with_context(&module, "flag-name-controls.kerml", std::slice::from_ref(&module), &library).unwrap();
    let find = |name: &str| document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
    assert_eq!(find("C").properties["declared_short_name"], "c");
    for (name, direction) in [("input", "in"), ("output", "out"), ("shared", "inout")] {
        assert_eq!(find(name).properties["direction"], direction);
    }
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/flag-name-variable-negative.kerml");
    let module = crate::kerml::parse_kerml(source).unwrap();
    let error = crate::kerml::compile_kerml_module_strict_with_context(&module, "flag-name-variable-negative.kerml", std::slice::from_ref(&module), &library).unwrap_err();
    assert!(error.to_string().contains("validateFeatureIsVariable"), "{error:?}");
}

#[test]
fn release_2026_08_flag_names_kernel_authoring_keeps_short_names_and_direction() {
    use std::collections::BTreeMap;
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/flag-name-controls.kerml");
    let module = crate::kerml::parse_kerml(source).unwrap();
    let project = mercurio_foundation::AuthoringProject::from_parsed_modules(
        BTreeMap::from([("p.kerml".into(), module)]), BTreeMap::from([("p.kerml".into(), source.into())]),
    ).unwrap().with_render_profile(mercurio_foundation::textual_model_authoring_render_profile());
    let rendered = project.render_new_file("p.kerml").unwrap();
    let module = crate::kerml::parse_kerml(&rendered).unwrap();
    let members = &module.package.as_ref().unwrap().members;
    let Declaration::GenericDefinition(class) = &members[0] else { panic!("class") };
    assert!(class.modifiers.iter().any(|m| m == "short_name=c"));
    let Declaration::GenericDefinition(behavior) = &members[1] else { panic!("behavior") };
    for (member, direction) in behavior.members.iter().zip(["in", "out", "inout"]) {
        let Declaration::GenericUsage(usage) = member else { panic!("feature") };
        assert!(usage.modifiers.iter().any(|m| m == direction));
    }
}

#[test]
fn release_2026_08_declarations_preserve_runtime_constraint_projection() {
    let library = load_sysml_baseline().unwrap();
    let source = include_str!("../simulation/thermal-deadline.sysml");
    let document = crate::compile_sysml_text(source, "thermal-deadline.sysml", &library).unwrap();
    let runtime = mercurio_foundation::runtime::Runtime::from_document(document).unwrap();
    let case = crate::simulation::list_analysis_cases(&runtime).into_iter().find(|c| c.label == "HeatProfile").unwrap();
    let scenario = crate::simulation::scenario_from_analysis_case(&runtime, &case.id).unwrap();
    assert_eq!(scenario.subjects.len(), 1);
    assert_eq!(scenario.requirements.len(), 1);
    assert!(scenario.requirements[0].expression.is_some(), "{scenario:?}");
    let model = crate::simulation::canonical_simulation_model(&runtime).unwrap();
    for element in runtime.graph().elements() {
        if crate::requirements::requirement_constraint_kind(runtime.graph(), element).is_some() {
            assert!(!model.derived_rules.iter().any(|r| r.id.starts_with(&element.element_id)));
        }
    }
    let report = crate::simulation::run_analysis_case(&runtime, &case.id, "declaration-projection").unwrap();
    assert_eq!(report.artifacts[0].payload["requirement_outcomes"][0]["status"], "violated");
}
#[test]
fn release_2026_08_namespace_locale_comments_preserve_following_members() {
    for head in ["locale", "comment locale"] {
        let source = format!("package P {{ {head} \"en_US\" /* Localized */ comment named /* Named */ part def C; }}");
        let module = parse_sysml(&source).unwrap();
        let members = &module.package.as_ref().unwrap().members;
        assert_eq!(members.len(), 3, "{head}: {members:?}");
        let Declaration::GenericUsage(comment) = &members[0] else { panic!("comment") };
        assert_eq!(comment.keyword, "comment");
        assert!(comment.is_implicit_name);
        assert_eq!(comment.metadata_properties["locale"], "en_US");
        assert_eq!(comment.metadata_properties["body"], " Localized ");
        let Declaration::GenericUsage(named) = &members[1] else { panic!("named comment") };
        assert_eq!(named.name, "named");
        assert!(members.iter().any(|m| matches!(m, Declaration::GenericDefinition(d) if d.name == "C")));
    }
    let source = "package P { locale \"en_US\" /* Localized */ comment c /* Named */ class C; }";
    let module = crate::kerml::parse_kerml(source).unwrap();
    let members = &module.package.as_ref().unwrap().members;
    assert_eq!(members.len(), 3, "{members:?}");
    assert!(members.iter().any(|m| matches!(m, Declaration::GenericDefinition(d) if d.name == "C")));
    assert!(parse_sysml("package P { comment locale /* Missing string */ part def C; }").is_err());
}

#[test]
fn release_2026_08_namespace_metadata_prefix_is_in_declaration_span() {
    let module = parse_sysml("package P {\n #Meta\n part def C;\n #Meta\n part c: C;\n}").unwrap();
    let members = &module.package.as_ref().unwrap().members;
    let Declaration::GenericDefinition(definition) = &members[0] else { panic!("definition") };
    assert_eq!((definition.span.start_line, definition.span.start_col), (2, 2));
    assert_eq!(definition.span.end_line, 3);
    assert!(definition.modifiers.iter().any(|m| m == "language_extension=Meta"));
    let Declaration::GenericUsage(usage) = &members[1] else { panic!("usage") };
    assert_eq!((usage.span.start_line, usage.span.start_col), (4, 2));
    assert_eq!(usage.span.end_line, 5);
    let module = crate::kerml::parse_kerml("package P {\n #atom\n class C;\n}").unwrap();
    let definition = module.package.unwrap().members.iter().find_map(Declaration::as_definition_like).unwrap();
    assert_eq!(definition.keyword, "class");
    assert!(definition.modifiers.iter().any(|m| m == "language_extension=atom"));
    assert_eq!((definition.span.start_line, definition.span.start_col), (2, 2));
    assert_eq!(definition.span.end_line, 3);
}

#[test]
fn release_2026_08_namespace_quoted_annotation_keywords_remain_names() {
    let tokens = lex("attribute 'locale' = \"en_US\" /* ordinary trivia */; attribute 'comment' = \"c\" /* more trivia */;").unwrap();
    assert_eq!(tokens.iter().filter(|t| matches!(t.kind, TokenKind::BlockDoc(_))).count(), 2);
    assert!(tokens.iter().any(|t| matches!(&t.kind, TokenKind::Identifier(name) if name == "locale")));
    assert!(tokens.iter().any(|t| matches!(&t.kind, TokenKind::Identifier(name) if name == "comment")));
}

#[test]
fn release_2026_08_namespace_metadata_prefix_authoring_round_trip() {
    use std::collections::BTreeMap;
    for (file, source) in [
        ("p.kerml", "package P {\n abstract #Meta::atom #Other\n class C;\n}"),
        ("p.sysml", "package P {\n abstract #Meta::atom #Other\n part def C;\n}"),
    ] {
        let parse = |text: &str| if file.ends_with(".kerml") {
            crate::kerml::parse_kerml(text)
        } else { parse_sysml(text) };
        let module = parse(source).unwrap();
        let project = mercurio_foundation::AuthoringProject::from_parsed_modules(
            BTreeMap::from([(file.into(), module)]),
            BTreeMap::from([(file.into(), source.into())]),
        ).unwrap().with_render_profile(mercurio_foundation::textual_model_authoring_render_profile());
        let rendered = project.render_new_file(file).unwrap();
        assert_eq!(rendered.matches("#Meta::atom").count(), 1, "{rendered}");
        assert_eq!(rendered.matches("#Other").count(), 1, "{rendered}");
        let module = parse(&rendered).unwrap();
        let definition = module.package.unwrap().members.iter().find_map(Declaration::as_definition_like).unwrap_or_else(|| panic!("{file}: {rendered}"));
        for modifier in ["language_extension=Meta::atom", "language_extension=Other", "abstract"] {
            assert!(definition.modifiers.iter().any(|m| m == modifier), "{file}: {definition:?}");
        }
    }
    assert!(crate::kerml::parse_kerml("package P { # ; }").is_err());
    assert!(crate::kerml::parse_kerml("package P { #Meta:: ; }").is_err());
}

#[test]
fn release_2026_08_namespace_import_and_alias_modifiers_round_trip() {
    use std::collections::BTreeMap;
    let source = "package P { private import all Types::**; protected import Types::C; public import Types::*; private alias Local for Types::C; }";
    for file in ["p.kerml", "p.sysml"] {
        let parse = |s: &str| if file.ends_with(".kerml") { crate::kerml::parse_kerml(s) } else { parse_sysml(s) };
        let module = parse(source).unwrap();
        let check = |module: &SysmlModule| {
            let members = &module.package.as_ref().unwrap().members;
            for (member, visibility) in members.iter().zip(["private", "protected", "public", "private"]) {
                let modifiers = match member { Declaration::Import(d) => &d.modifiers, Declaration::Alias(d) => &d.modifiers, _ => panic!("{member:?}") };
                assert!(modifiers.iter().any(|m| m == visibility), "{file}: {member:?}");
            }
            let Declaration::Import(import) = &members[0] else { panic!("import") };
            assert_eq!(import.path.as_colon_string(), "Types::**");
            assert!(import.modifiers.iter().any(|m| m == "import_all"));
        };
        check(&module);
        let project = mercurio_foundation::AuthoringProject::from_parsed_modules(
            BTreeMap::from([(file.into(), module)]), BTreeMap::from([(file.into(), source.into())]),
        ).unwrap().with_render_profile(mercurio_foundation::textual_model_authoring_render_profile());
        check(&parse(&project.render_new_file(file).unwrap()).unwrap());
    }
}

#[test]
fn release_2026_08_namespace_import_visibility_validation_and_properties() {
    let library = load_sysml_baseline().unwrap();
    for (ext, definition) in [("kerml", "class C;"), ("sysml", "part def C;")] {
        let compile = |source: &str| {
            let module = if ext == "kerml" { crate::kerml::parse_kerml(source) } else { parse_sysml(source) }.unwrap();
            if ext == "kerml" {
                crate::kerml::compile_kerml_module_strict_with_context(&module, "imports.kerml", std::slice::from_ref(&module), &library)
            } else { compile_sysml_module(&module, "imports.sysml", &library) }
        };
        for visibility in ["private", "public", "protected"] {
            let source = format!("{visibility} import P::*; package P {{ {definition} }}");
            let result = compile(&source);
            if visibility == "private" {
                let document = result.unwrap();
                let import = document.elements.iter().find(|e| e.kind.ends_with("Import")).unwrap();
                assert_eq!(import.properties["visibility"], "private");
                assert_eq!(import.properties["is_import_all"], false);
            } else {
                let error = result.unwrap_err();
                assert!(error.message.contains("validateImportTopLevelVisibility"), "{ext}: {error}");
            }
        }
        let source = format!("package P {{ {definition} }} package User {{ public import P::*; protected import P::C; private import all P::**; }}");
        let document = compile(&source).unwrap();
        let imports = document.elements.iter().filter(|e| e.kind.ends_with("Import")).collect::<Vec<_>>();
        assert_eq!(imports.len(), 3);
        for (import, visibility) in imports.iter().zip(["public", "protected", "private"]) {
            assert_eq!(import.properties["visibility"], visibility);
        }
        assert_eq!(imports[2].properties["is_import_all"], true);
        assert_eq!(imports[2].properties["is_recursive"], true);
    }
}

#[test]
fn release_2026_08_namespace_comment_authoring_keeps_body_locale_and_target() {
    use std::collections::BTreeMap;
    for (file, definition) in [("p.kerml", "class C;"), ("p.sysml", "part def C;")] {
        let source = format!("package P {{ locale \"en_US\" /* Localized */ comment named about C locale \"en\" /* Named body */ {definition} }}");
        let parse = |s: &str| if file.ends_with(".kerml") { crate::kerml::parse_kerml(s) } else { parse_sysml(s) };
        let before = parse(&source).unwrap();
        let project = mercurio_foundation::AuthoringProject::from_parsed_modules(
            BTreeMap::from([(file.into(), before.clone())]), BTreeMap::from([(file.into(), source)]),
        ).unwrap().with_render_profile(mercurio_foundation::textual_model_authoring_render_profile());
        let rendered = project.render_new_file(file).unwrap();
        let after = parse(&rendered).unwrap();
        let left = &before.package.as_ref().unwrap().members;
        let right = &after.package.as_ref().unwrap().members;
        assert_eq!(left.len(), right.len(), "{rendered}");
        for (a, b) in left.iter().zip(right).take(2) {
            let (Declaration::GenericUsage(a), Declaration::GenericUsage(b)) = (a,b) else { panic!("comment") };
            assert_eq!(a.metadata_properties, b.metadata_properties, "{rendered}");
            assert_eq!(a.is_implicit_name, b.is_implicit_name);
            if !a.is_implicit_name { assert_eq!(a.name, b.name); }
            assert_eq!(a.reference_target.as_ref().map(|n| n.as_colon_string()), b.reference_target.as_ref().map(|n| n.as_colon_string()));
        }
    }
}

#[test]
fn release_2026_08_namespace_comments_resolve_packages_and_other_comments() {
    let library = load_sysml_baseline().unwrap();
    for (file, definition) in [("p.kerml", "class C"), ("p.sysml", "part def C")] {
        let source = format!("package P {{ comment cmt locale \"en\" /* Named */ comment other about cmt /* Target */ {definition} {{ comment about P /* Package */ }} }}");
        let module = if file.ends_with(".kerml") { crate::kerml::parse_kerml(&source) } else { parse_sysml(&source) }.unwrap();
        let document = if file.ends_with(".kerml") {
            crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library)
        } else { compile_sysml_module(&module, file, &library) }.unwrap();
        let comments = document.elements.iter().filter(|e| e.kind.ends_with("::Comment")).collect::<Vec<_>>();
        assert_eq!(comments.len(), 3, "{file}");
        let cmt = comments.iter().find(|e| e.properties.get("declared_name").is_some_and(|n| n == "cmt")).unwrap();
        assert_eq!(cmt.properties["body"], "Named ");
        assert_eq!(cmt.properties["locale"], "en");
        let other = comments.iter().find(|e| e.properties.get("declared_name").is_some_and(|n| n == "other")).unwrap();
        assert_eq!(other.properties["annotated_element"], serde_json::json!([cmt.id]));
        let package_note = comments.iter().find(|e| e.properties.get("body").is_some_and(|n| n == "Package ")).unwrap();
        assert_eq!(package_note.properties["annotated_element"], serde_json::json!(["pkg.P"]));
    }
}

#[test]
fn release_2026_08_namespace_documentation_preserves_headers_ownership_and_authoring() {
    use std::collections::BTreeMap;
    let library = load_sysml_baseline().unwrap();
    for (file, definition) in [("p.kerml", "class C"), ("p.sysml", "part def C")] {
        for header in ["doc", "doc Named", "doc <short> Named", "doc <short>", "doc locale \"en_US\"", "doc <short> 'Named Doc' locale \"en_US\""] {
            let source = format!("package P {{ {definition} {{\n {header} /* Body */\n }} }}");
            let parse = |s: &str| if file.ends_with(".kerml") { crate::kerml::parse_kerml(s) } else { parse_sysml(s) };
            let module = parse(&source).unwrap();
            let doc = module.package.as_ref().unwrap().members[0].as_definition_like().unwrap().members[0].as_usage_like().unwrap();
            assert_eq!(doc.keyword, "doc");
            assert_eq!(doc.span.start_line, 2);
            assert_eq!(doc.metadata_properties["body"], " Body ");
            let project = mercurio_foundation::AuthoringProject::from_parsed_modules(
                BTreeMap::from([(file.into(), module.clone())]), BTreeMap::from([(file.into(), source)]),
            ).unwrap().with_render_profile(mercurio_foundation::textual_model_authoring_render_profile());
            let rendered = project.render_new_file(file).unwrap();
            let after = parse(&rendered).unwrap();
            let after_doc = after.package.as_ref().unwrap().members[0].as_definition_like().unwrap().members[0].as_usage_like().unwrap();
            assert_eq!(doc.metadata_properties, after_doc.metadata_properties, "{rendered}");
            assert_eq!(doc.name, after_doc.name, "{rendered}");
            assert_eq!(doc.modifiers, after_doc.modifiers, "{rendered}");
            let document = if file.ends_with(".kerml") {
                crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library)
            } else { compile_sysml_module(&module, file, &library) }.unwrap();
            let emitted = document.elements.iter().find(|e| e.kind.ends_with("::Documentation")).unwrap();
            assert_eq!(emitted.properties["body"], "Body ");
            let owner = &emitted.properties["owner"];
            assert_eq!(emitted.properties["documented_element"], *owner);
            assert_eq!(emitted.properties["annotated_element"], serde_json::json!([owner]));
            let membership = document.elements.iter().find(|e| Some(&serde_json::json!(e.id)) == emitted.properties.get("owning_membership")).unwrap();
            assert!(membership.kind.ends_with("::OwningMembership"));
            assert_eq!(membership.properties["member_element"], emitted.id);
            let owner = document.elements.iter().find(|e| serde_json::json!(e.id) == *owner).unwrap();
            assert!(owner.properties["documentation"].as_array().unwrap().contains(&serde_json::json!(emitted.id)));
            mercurio_foundation::runtime::Runtime::from_document(document).unwrap();
        }
        for invalid in ["doc Named;", "doc Named about C /* Body */", "doc Named locale /* Body */"] {
            assert!(if file.ends_with(".kerml") { crate::kerml::parse_kerml(invalid) } else { parse_sysml(invalid) }.is_err(), "{invalid}");
        }
    }
}

#[test]
fn release_2026_08_namespace_anonymous_documentation_keeps_distinct_source_identity() {
    let library = load_sysml_baseline().unwrap();
    for (file, definition) in [("p.kerml", "class C"), ("p.sysml", "part def C")] {
        let source = format!("package P {{\n doc /* Package */\n {definition} {{\n doc /* First */ doc /**/\n }}\n}}");
        let module = if file.ends_with(".kerml") { crate::kerml::parse_kerml(&source) } else { parse_sysml(&source) }.unwrap();
        let document = if file.ends_with(".kerml") {
            crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library)
        } else { compile_sysml_module(&module, file, &library) }.unwrap();
        let docs = document.elements.iter().filter(|e| e.kind == "SysML::Documentation").collect::<Vec<_>>();
        assert_eq!(docs.len(), 3);
        assert_eq!(docs.iter().map(|e| &e.id).collect::<std::collections::BTreeSet<_>>().len(), 3);
        assert!(docs.iter().all(|e| !e.properties.contains_key("declared_name")));
        let package_doc = docs.iter().find(|e| e.properties["body"] == "Package ").unwrap();
        assert_eq!(package_doc.properties["owner"], "pkg.P");
        let nested = docs.iter().filter(|e| e.properties["owner"] != "pkg.P").collect::<Vec<_>>();
        assert_eq!(nested.len(), 2);
        assert_ne!(nested[0].properties["metadata"]["source_span"]["start_col"], nested[1].properties["metadata"]["source_span"]["start_col"]);
        for doc in docs {
            let owner = document.elements.iter().find(|e| doc.properties["owner"] == e.id).unwrap();
            assert_eq!(doc.properties["documented_element"], owner.id);
            assert_eq!(doc.properties["annotated_element"], serde_json::json!([owner.id]));
            assert!(owner.properties["documentation"].as_array().unwrap().contains(&serde_json::json!(doc.id)));
            let membership = document.elements.iter().find(|e| doc.properties["owning_membership"] == e.id).unwrap();
            assert_eq!(membership.kind, "SysML::OwningMembership");
            assert_eq!(membership.properties["member_element"], doc.id);
            assert_eq!(membership.properties["membership_owning_namespace"], owner.id);
            for field in ["features", "owned_feature"] {
                assert!(!owner.properties.get(field).and_then(serde_json::Value::as_array).is_some_and(|values| values.contains(&serde_json::json!(doc.id))));
            }
        }
        mercurio_foundation::runtime::Runtime::from_document(document).unwrap();
    }
}

#[test]
fn release_2026_08_namespace_documentation_before_constraint_result_keeps_both() {
    let library = load_sysml_baseline().unwrap();
    for (text, keyword, kind) in [
        ("doc /* Explanation */", "doc", "Documentation"),
        ("/* Explanation */", "comment", "Comment"),
        ("comment /* Explanation */", "comment", "Comment"),
        ("locale \"en\" /* Explanation */", "comment", "Comment"),
        ("rep language \"text\" /* Explanation */", "rep", "TextualRepresentation"),
    ] {
        let source = format!("package P {{ requirement r {{ assume constraint c {{ {text} true }} }} }}");
        let module = parse_sysml(&source).unwrap();
        let package = module.package.as_ref().unwrap();
        let requirement = package.members[0].as_usage_like().unwrap();
        let constraint = requirement.body_members[0].as_usage_like().unwrap();
        assert!(constraint.expression.is_some(), "{source}");
        assert_eq!(constraint.body_members[0].as_usage_like().unwrap().keyword, keyword);
        let document = compile_sysml_module(&module, "p.sysml", &library).unwrap();
        let constraint = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "c")).unwrap();
        assert!(constraint.properties.contains_key("expression_ir"));
        let annotation = document.elements.iter().find(|e| e.kind == format!("SysML::{kind}")).unwrap();
        assert_eq!(annotation.properties["owner"], constraint.id);
    }
}

#[test]
fn release_2026_08_namespace_ordered_annotations_keep_targets_and_ownership() {
    use std::collections::BTreeMap;
    let library = load_sysml_baseline().unwrap();
    for (file, definition) in [("p.kerml", "class"), ("p.sysml", "part def")] {
        let source = format!("package P {{\n {definition} 'A.b'; {definition} B;\n doc Help /* Help */\n comment note about B,\n 'A.b' /* Ordered */\n comment about note, Help /* Linked */\n comment /* Implicit */\n}}");
        let parse = |s: &str| if file.ends_with(".kerml") { crate::kerml::parse_kerml(s) } else { parse_sysml(s) };
        let module = parse(&source).unwrap();
        let note = module.package.as_ref().unwrap().members[3].as_usage_like().unwrap();
        assert_eq!(note.annotation_targets.iter().map(|n| n.segments.clone()).collect::<Vec<_>>(), vec![vec!["B"], vec!["A.b"]]);
        let project = mercurio_foundation::AuthoringProject::from_parsed_modules(BTreeMap::from([(file.into(), module.clone())]), BTreeMap::from([(file.into(), source)])).unwrap();
        let rendered = project.render_new_file(file).unwrap();
        let after = parse(&rendered).unwrap();
        let after_note = after.package.as_ref().unwrap().members[3].as_usage_like().unwrap();
        assert_eq!(note.annotation_targets.iter().map(|n| &n.segments).collect::<Vec<_>>(), after_note.annotation_targets.iter().map(|n| &n.segments).collect::<Vec<_>>());
        let document = if file.ends_with(".kerml") {
            crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library)
        } else { compile_sysml_module(&module, file, &library) }.unwrap();
        let note = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "note")).unwrap();
        assert_eq!(note.properties["body"], "Ordered ");
        assert_eq!(note.properties["annotated_element"], serde_json::json!(["type.P.B", "type.P.A.b"]));
        let annotations = note.properties["annotation"].as_array().unwrap();
        assert_eq!(annotations.len(), 2);
        for (index, id) in annotations.iter().enumerate() {
            let annotation = document.elements.iter().find(|e| id == &serde_json::json!(e.id)).unwrap();
            assert_eq!(annotation.kind, "SysML::Annotation");
            assert!(!annotation.properties.contains_key("owner"));
            assert_eq!(annotation.properties["owning_annotating_element"], note.id);
            assert_eq!(annotation.properties["annotating_element"], note.id);
            assert_eq!(annotation.properties["source"], serde_json::json!([note.id]));
            assert_eq!(annotation.properties["target"], serde_json::json!([note.properties["annotated_element"][index]]));
            assert_eq!(annotation.properties["metadata"]["source_span"]["start_line"], 4 + index);
            assert!(note.properties["owned_relationship"].as_array().unwrap().contains(id));
        }
        let linked = document.elements.iter().find(|e| e.properties.get("body").is_some_and(|v| v == "Linked ")).unwrap();
        let help = document.elements.iter().find(|e| e.kind == "SysML::Documentation").unwrap();
        assert_eq!(linked.properties["annotated_element"], serde_json::json!([note.id, help.id]));
        let implicit = document.elements.iter().find(|e| e.properties.get("body").is_some_and(|v| v == "Implicit ")).unwrap();
        assert_eq!(implicit.properties["annotated_element"], serde_json::json!(["pkg.P"]));
        assert_eq!(implicit.properties["annotation"], serde_json::json!([]));
        assert!(implicit.properties.contains_key("owning_membership"));
        mercurio_foundation::runtime::Runtime::from_document(document).unwrap();
        let bad = parse(&format!("package P {{ {definition} A; comment about A, Missing /* bad */ }}")).unwrap();
        let result = if file.ends_with(".kerml") { crate::kerml::compile_kerml_module_strict_with_context(&bad, file, std::slice::from_ref(&bad), &library) } else { compile_sysml_module(&bad, file, &library) };
        assert!(result.is_err());
    }
}

#[test]
fn release_2026_08_namespace_relationship_owned_annotations() {
    let library = load_sysml_baseline().unwrap();
    for (file, definition) in [("owned.kerml", "class"), ("owned.sysml", "part def")] {
        let source = format!("package P {{\n {definition} A; {definition} B;\n dependency d from A to B {{\n doc Help /* Documentation */\n comment note about B, A /* Comment */\n rep Text language \"text\" /* Representation */\n }}\n}}");
        let module = if file.ends_with("kerml") { crate::kerml::parse_kerml(&source) } else { parse_sysml(&source) }.unwrap();
        let document = if file.ends_with("kerml") {
            crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library)
        } else { compile_sysml_module(&module, file, &library) }.unwrap();
        let dependency = document.elements.iter().find(|e| e.kind.rsplit("::").next() == Some("Dependency")).unwrap();
        assert_eq!(dependency.properties["owned_annotation"].as_array().unwrap().len(), 3);
        for name in ["Help", "note", "Text"] {
            let element = document.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
            assert_eq!(element.properties["owner"], dependency.id);
            assert!(!element.properties.contains_key("owning_membership"));
            assert!(!element.properties.contains_key("owning_namespace"));
            let id = element.properties["owning_annotating_relationship"].as_str().unwrap();
            let annotation = document.elements.iter().find(|e| e.id == id).unwrap();
            assert_eq!(annotation.properties["owned_annotating_element"], element.id);
            assert_eq!(annotation.properties["owning_annotated_element"], dependency.id);
            assert_eq!(annotation.properties["owned_related_element"], serde_json::json!([element.id]));
            assert_eq!(annotation.properties["source"], serde_json::json!([element.id]));
            assert_eq!(annotation.properties["target"], serde_json::json!([dependency.id]));
            assert!(!annotation.properties.contains_key("owner"));
            assert_eq!(element.properties["annotation"][0], id);
            assert_eq!(element.properties["annotated_element"][0], dependency.id);
            assert!(dependency.properties["owned_relationship"].as_array().unwrap().contains(&serde_json::json!(id)));
            if name == "note" {
                assert_eq!(element.properties["annotation"].as_array().unwrap().len(), 3);
                assert_eq!(element.properties["annotated_element"].as_array().unwrap().len(), 3);
            }
        }
        mercurio_foundation::runtime::Runtime::from_document(document).unwrap();
    }
}

#[test]
fn release_2026_08_namespace_legacy_docs_use_canonical_ownership() {
    let library = load_sysml_baseline().unwrap();
    for (file, definition) in [("legacy.kerml", "class"), ("legacy.sysml", "part def")] {
        let source = format!("package P {{ {definition} A; {definition} B; dependency d from A to B; }}");
        let mut module = if file.ends_with("kerml") { crate::kerml::parse_kerml(&source) } else { parse_sysml(&source) }.unwrap();
        let Declaration::Package(package) = &mut module.members[0] else { panic!("expected package"); };
        package.docs = vec!["Package docs".into()];
        if let Declaration::GenericUsage(usage) = package.members.last_mut().unwrap() {
            usage.docs = vec!["Relationship docs".into(), "Second docs".into()];
        } else { panic!("expected relationship declaration"); }
        let document = if file.ends_with("kerml") {
            crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library)
        } else { compile_sysml_module(&module, file, &library) }.unwrap();
        let docs = document.elements.iter().filter(|e| e.kind == "SysML::Documentation").collect::<Vec<_>>();
        assert_eq!(docs.len(), 3);
        for doc in docs {
            let owner = document.elements.iter().find(|e| Some(&serde_json::json!(e.id)) == doc.properties.get("owner")).unwrap();
            assert!(owner.properties["documentation"].as_array().unwrap().contains(&serde_json::json!(doc.id)));
            assert_eq!(doc.properties["documented_element"], owner.id);
            assert_eq!(doc.properties["metadata"]["generated"], true);
            if owner.kind.ends_with("Dependency") { assert!(doc.properties.contains_key("owning_annotating_relationship")); }
            else { assert!(doc.properties.contains_key("owning_membership")); }
        }
        mercurio_foundation::runtime::Runtime::from_document(document).unwrap();
    }
}

#[test]
fn release_2026_08_namespace_bare_comments_keep_identity_and_round_trip() {
    use std::collections::BTreeMap;
    let library = load_sysml_baseline().unwrap();
    for file in ["bare.kerml", "bare.sysml"] {
        let source = "package P {\n /* First */ /* Second */\n comment note /* Named */\n /* Last */\n}";
        let parse = |s: &str| if file.ends_with("kerml") { crate::kerml::parse_kerml(s) } else { parse_sysml(s) };
        let module = parse(source).unwrap();
        let members = &module.package.as_ref().unwrap().members;
        assert_eq!(members.len(), 4);
        let bodies = members.iter().map(|m| m.as_usage_like().unwrap().metadata_properties["body"].clone()).collect::<Vec<_>>();
        assert_eq!(bodies, [" First ", " Second ", " Named ", " Last "]);
        let project = mercurio_foundation::AuthoringProject::from_parsed_modules(BTreeMap::from([(file.into(), module.clone())]), BTreeMap::from([(file.into(), source.into())])).unwrap();
        let rendered = project.render_new_file(file).unwrap();
        assert_eq!(rendered.matches("comment").count(), 1, "{rendered}");
        let reparsed = parse(&rendered).unwrap();
        assert_eq!(reparsed.package.as_ref().unwrap().members.iter().map(|m| m.as_usage_like().unwrap().metadata_properties["body"].clone()).collect::<Vec<_>>(), bodies);
        let document = if file.ends_with("kerml") { crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library) } else { compile_sysml_module(&module, file, &library) }.unwrap();
        let comments = document.elements.iter().filter(|e| e.kind == "SysML::Comment").collect::<Vec<_>>();
        assert_eq!(comments.len(), 4);
        assert_ne!(comments[0].id, comments[1].id);
        assert_ne!(comments[0].properties["metadata"]["source_span"], comments[1].properties["metadata"]["source_span"]);
        for element in comments { assert_eq!(element.properties["annotated_element"], serde_json::json!(["pkg.P"])); }
        mercurio_foundation::runtime::Runtime::from_document(document).unwrap();
    }
}

#[test]
fn release_2026_08_namespace_self_annotation_and_missing_bodies_are_rejected() {
    let library = load_sysml_baseline().unwrap();
    for file in ["invalid.kerml", "invalid.sysml"] {
        let parse = |s: &str| if file.ends_with("kerml") { crate::kerml::parse_kerml(s) } else { parse_sysml(s) };
        for source in ["package P { comment note; }", "package P { comment note {} }", "package P { doc note; }"] {
            assert!(parse(source).is_err(), "{file}: {source}");
        }
        let module = parse("package P { comment note about note /* Invalid ownership */ }").unwrap();
        let result = if file.ends_with("kerml") { crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library) } else { compile_sysml_module(&module, file, &library) };
        let error = result.unwrap_err();
        assert!(error.message.contains("validateAnnotationAnnotatedElementOwnership"), "{error:?}");
    }
}

#[test]
fn release_2026_08_namespace_alias_and_import_bodies_survive_compilation_and_authoring() {
    use std::collections::BTreeMap;
    let library = load_sysml_baseline().unwrap();
    for (file, definition) in [("members.kerml", "class"), ("members.sysml", "part def")] {
        let source = format!("package P {{\n {definition} A;\n alias <short> Long for A {{ doc /* Alias docs */ comment /* Alias note */ }}\n alias <Only> for A;\n alias for A;\n package Q {{ private import P::A {{ doc /* Import docs */ /* Import note */ }} }}\n}}");
        let parse = |s: &str| if file.ends_with("kerml") { crate::kerml::parse_kerml(s) } else { parse_sysml(s) };
        let module = parse(&source).unwrap();
        let project = mercurio_foundation::AuthoringProject::from_parsed_modules(BTreeMap::from([(file.into(), module.clone())]), BTreeMap::from([(file.into(), source)])).unwrap();
        let rendered = project.render_new_file(file).unwrap();
        assert!(rendered.contains("alias <Only> for A"), "{rendered}");
        assert!(rendered.contains("alias for A"), "{rendered}");
        let reparsed = parse(&rendered).unwrap();
        for model in [&module, &reparsed] {
            let document = if file.ends_with("kerml") { crate::kerml::compile_kerml_module_strict_with_context(model, file, std::slice::from_ref(model), &library) } else { compile_sysml_module(model, file, &library) }.unwrap();
            let aliases = document.elements.iter().filter(|e| e.kind == "SysML::Membership").collect::<Vec<_>>();
            assert_eq!(aliases.len(), 3);
            let long = aliases.iter().find(|e| e.properties.get("member_name").is_some_and(|v| v == "Long")).unwrap();
            assert_eq!(long.properties["member_short_name"], "short");
            assert_eq!(long.properties["member_element"], "type.P.A");
            assert_eq!(long.properties["membership_owning_namespace"], "pkg.P");
            assert_eq!(long.properties["owned_annotation"].as_array().unwrap().len(), 2);
            let docs = document.elements.iter().filter(|e| e.kind == "SysML::Documentation").collect::<Vec<_>>();
            assert_eq!(docs.len(), 2);
            assert!(docs.iter().all(|e| e.properties.contains_key("owning_annotating_relationship")));
            assert!(document.elements.iter().any(|e| e.kind == "SysML::Comment" && e.properties.get("body").is_some_and(|v| v == "Import note ")));
            mercurio_foundation::runtime::Runtime::from_document(document).unwrap();
        }
        let bad = parse("package P { alias X for Missing; }").unwrap();
        let result = if file.ends_with("kerml") { crate::kerml::compile_kerml_module_strict_with_context(&bad, file, std::slice::from_ref(&bad), &library) } else { compile_sysml_module(&bad, file, &library) };
        assert!(result.is_err());
    }
}

#[test]
fn release_2026_08_namespace_memberships_are_ordered_and_imports_target_memberships() {
    let library = load_sysml_baseline().unwrap();
    for (file, definition) in [("ordered.kerml", "class"), ("ordered.sysml", "part def")] {
        let source = format!("package P {{\n private {definition} Z;\n alias First for Z;\n {definition} A;\n alias Second for A;\n package Q {{ private import P::First; private import P::A; }}\n}}");
        let module = if file.ends_with("kerml") { crate::kerml::parse_kerml(&source) } else { parse_sysml(&source) }.unwrap();
        let document = if file.ends_with("kerml") { crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library) } else { compile_sysml_module(&module, file, &library) }.unwrap();
        let lookup = |id: &str| document.elements.iter().find(|e| e.id == id).unwrap();
        let package = lookup("pkg.P");
        let memberships = package.properties["owned_membership"].as_array().unwrap().iter().map(|id| lookup(id.as_str().unwrap())).collect::<Vec<_>>();
        assert_eq!(memberships.iter().map(|e| e.properties["member_name"].as_str().unwrap()).collect::<Vec<_>>(), ["Z", "First", "A", "Second", "Q"]);
        assert_eq!(memberships[0].properties["visibility"], "private");
        assert_eq!(memberships[1].kind, "SysML::Membership");
        let imports = document.elements.iter().filter(|e| e.kind == "SysML::MembershipImport").collect::<Vec<_>>();
        assert_eq!(imports.len(), 2);
        for (import, member) in imports.iter().zip([memberships[1], memberships[2]]) {
            assert_eq!(import.properties["imported_membership"], member.id);
            assert_eq!(import.properties["target"], serde_json::json!([member.id]));
            assert_eq!(import.properties["source"], serde_json::json!(["pkg.P.Q"]));
        }
    }
}

#[test]
fn release_2026_08_namespace_resource_ids_are_distinct_without_rewriting_text() {
    let library = load_sysml_baseline().unwrap();
    let documents = [("one.sysml", "One"), ("two.sysml", "Two")].map(|(file, name)| {
        let source = format!("package {name} {{ comment note /*pkg.root*/ }}");
        compile_sysml_module(&parse_sysml(&source).unwrap(), file, &library).unwrap()
    });
    let roots = documents.iter().map(|document| document.elements.iter().find(|e| e.kind == "SysML::Namespace").unwrap().id.clone()).collect::<Vec<_>>();
    assert_ne!(roots[0], roots[1]);
    for document in &documents {
        let comment = document.elements.iter().find(|e| e.kind == "SysML::Comment").unwrap();
        assert_eq!(comment.properties["body"], "pkg.root");
    }
    mercurio_foundation::KirDocument::merge_with_registered_fields(documents, crate::sysml_field_specs().iter().copied()).unwrap();
}

#[test]
fn release_2026_08_namespace_short_only_alias_in_definition_is_not_discarded() {
    let source = include_str!("../../../mercurio-tools/corpus/release-2026-08/membership-bodies/positive-nested.sysml");
    let module = parse_sysml(source).unwrap();
    let document = compile_sysml_module(&module, "nested.sysml", &load_sysml_baseline().unwrap()).unwrap();
    let alias = document.elements.iter().find(|e| e.kind == "SysML::Membership").unwrap();
    assert_eq!(alias.properties["member_short_name"], "Local");
    assert_eq!(alias.properties["membership_owning_namespace"], "type.P.Holder");
    let doc = document.elements.iter().find(|e| e.kind == "SysML::Documentation").unwrap();
    assert_eq!(doc.properties["owner"], alias.id);
}

#[test]
fn release_2026_08_namespace_visibility_matches_isolated_pilot_controls() {
    let library = load_sysml_baseline().unwrap();
    for (file, source, accepted) in [
        ("positive-all.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/namespace-visibility/positive-all.kerml"), true),
        ("positive-all.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/namespace-visibility/positive-all.sysml"), true),
        ("positive-public-alias.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/namespace-visibility/positive-public-alias.kerml"), true),
        ("positive-public-alias.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/namespace-visibility/positive-public-alias.sysml"), true),
        ("invalid-private-alias.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/namespace-visibility/invalid-private-alias.kerml"), false),
        ("invalid-private-alias.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/namespace-visibility/invalid-private-alias.sysml"), false),
        ("invalid-hidden-parent.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/namespace-visibility/invalid-hidden-parent.kerml"), false),
        ("invalid-hidden-parent.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/namespace-visibility/invalid-hidden-parent.sysml"), false),
    ] {
        let module = if file.ends_with("kerml") { crate::kerml::parse_kerml(source) } else { parse_sysml(source) }.unwrap();
        let result = if file.ends_with("kerml") { crate::kerml::compile_kerml_module_strict_with_context(&module, file, std::slice::from_ref(&module), &library) } else { compile_sysml_module(&module, file, &library) };
        assert_eq!(result.is_ok(), accepted, "{file}: {:?}", result.err());
    }
}

#[test]
fn release_2026_08_namespace_aliases_resolve_inherited_and_exposed_members() {
    let library = load_sysml_baseline().unwrap();
    let source = "package P { class A { feature f; } feature y: A { feature g redefines f; } feature w subsets y; alias us for w::g; }";
    let module = crate::kerml::parse_kerml(source).unwrap();
    let document = crate::kerml::compile_kerml_module_strict_with_context(&module, "inherited.kerml", std::slice::from_ref(&module), &library).unwrap();
    let alias = document.elements.iter().find(|e| e.kind == "SysML::Membership").unwrap();
    assert_eq!(alias.properties["member_element"], "feature.P.y.g");
    let source = "package P { package Inner { private part hidden; } view v { expose Inner::*; alias exposed for hidden; } }";
    let document = compile_sysml_module(&parse_sysml(source).unwrap(), "exposed.sysml", &library).unwrap();
    let alias = document.elements.iter().find(|e| e.kind == "SysML::Membership").unwrap();
    assert_eq!(alias.properties["member_element"], "feature.P.Inner.hidden");
    let expose = document.elements.iter().find(|e| e.kind == "SysML::NamespaceExpose").unwrap();
    assert_eq!(expose.properties["visibility"], "protected");
    assert_eq!(expose.properties["is_import_all"], true);
}

#[test]
fn release_2026_08_namespace_grammar_distinguishes_relationship_bodies() {
    for header in ["alias Link for A", "private import A", "expose A"] {
        for member in ["part def Owned;", "part child;", "alias Other for A;", "private import A;"] {
            let source = format!("package P {{ part def A; {header} {{ {member} }} }}");
            let error = parse_sysml(&source).unwrap_err();
            assert!(error.message.contains("only annotating elements"), "{error:?}");
        }
        let source = format!("package P {{ part def A; {header} {{ doc /* Body */ comment /* Note */ rep language \"text\" /* Representation */ }} }}");
        parse_sysml(&source).unwrap();
    }
    for header in ["alias Link for A", "private import A"] {
        let source = format!("package P {{ class A; {header} {{ class Owned; feature child; }} }}");
        let module = crate::kerml::parse_kerml(&source).unwrap();
        let package = module.package.as_ref().unwrap();
        let body = match &package.members[1] {
            Declaration::Alias(alias) => &alias.body_members,
            Declaration::Import(import) => &import.body_members,
            other => panic!("expected alias/import, got {other:?}"),
        };
        assert_eq!(body.len(), 2);
        assert!(matches!(&body[0], Declaration::GenericDefinition(d) if d.name == "Owned"));
        assert!(matches!(&body[1], Declaration::GenericUsage(u) if u.name == "child"));
        let invalid = format!("package P {{ class A; {header} {{ alias Other for A; }} }}");
        assert!(crate::kerml::parse_kerml(&invalid).is_err());
    }
}

#[test]
fn release_2026_08_namespace_generated_queries_preserve_target_kinds() {
    let source = "package P { part p; package Q { private import P::p; private import P::*; } view v { expose p; expose p::*; } }";
    let library = load_sysml_baseline().unwrap();
    let doc = compile_sysml_module(&parse_sysml(source).unwrap(), "generated-contract.sysml", &library).unwrap();
    for (kind, field, expected_target_kind) in [
        ("SysML::MembershipImport", "imported_membership", "SysML::OwningMembership"),
        ("SysML::NamespaceImport", "imported_namespace", "SysML::Package"),
        ("SysML::MembershipExpose", "imported_membership", "SysML::OwningMembership"),
        ("SysML::NamespaceExpose", "imported_namespace", "SysML::PartUsage"),
    ] {
        let element = doc.elements.iter().find(|e| e.kind == kind).unwrap_or_else(|| panic!("missing {kind}; actual: {:?}", doc.elements.iter().filter(|e| e.kind.contains("Import") || e.kind.contains("Expose")).map(|e| (&e.kind, &e.properties)).collect::<Vec<_>>()));
        let target = element.properties[field].as_str().unwrap();
        assert_eq!(doc.elements.iter().find(|e| e.id == target).unwrap_or_else(|| panic!("missing target {target} for {kind}: {element:?}")).kind, expected_target_kind);
        if kind.ends_with("Expose") { assert_eq!(element.properties["visibility"], "protected"); }
    }
}

#[test]
fn release_2026_08_namespace_relationship_bodies_do_not_discard_unknown_syntax() {
    for body in ["class Owned;", "unknown nonsense;", "1;", ";", "private comment /* Hidden */"] {
        let source = format!("package P {{ part def A; alias a for A {{ {body} }} }}");
        assert!(parse_sysml(&source).is_err(), "discarded SysML body: {body}");
    }
    for body in ["unknown nonsense;", "private class Owned;", "public comment /* Hidden */"] {
        let source = format!("package P {{ class A; alias a for A {{ {body} }} }}");
        assert!(crate::kerml::parse_kerml(&source).is_err());
    }
}

#[test]
fn release_2026_08_translated_scalar_enumeration_default_and_reference_flags() {
    let library = load_sysml_baseline().unwrap();
    let source = parse_sysml("package ScalarChecks { enum def Choice { one; two; } part def Container { ref value: Choice; } }").unwrap();
    let kir = compile_sysml_module(&source, "scalar-checks.sysml", &library).unwrap();
    let enumeration = kir.elements.iter().find(|e| e.kind.ends_with("::EnumerationDefinition")
        && e.properties.get("declared_name").and_then(|v| v.as_str()) == Some("Choice")).unwrap();
    assert_eq!(enumeration.properties.get("is_variation"), Some(&serde_json::json!(true)));
    let reference = kir.elements.iter().find(|e| e.kind.ends_with("::ReferenceUsage")
        && e.properties.get("declared_name").and_then(|v| v.as_str()) == Some("value")).unwrap();
    assert_eq!(reference.properties.get("is_reference"), Some(&serde_json::json!(true)));
    assert_eq!(reference.properties.get("is_composite"), Some(&serde_json::json!(false)));
}

#[test]
fn release_2026_08_textual_membership_ecore_opposites_and_resource_root() {
    let library = load_sysml_baseline().unwrap();
    for (source, kernel) in [("comment note /* Root note */", false), ("package P { comment note /* Nested note */ inv valid { true } }", true)] {
        let module = if kernel { crate::kerml::parse_kerml(source) } else { parse_sysml(source) }.unwrap();
        let document = if kernel {
            crate::kerml::compile_kerml_module_strict_with_context(&module, "textual-ownership.kerml", std::slice::from_ref(&module), &library)
        } else { compile_sysml_module(&module, "textual-ownership.sysml", &library) }.unwrap();
        let memberships: Vec<_> = document.elements.iter().filter(|e| {
            e.id.ends_with(".membership") && matches!(e.kind.rsplit("::").next(),
                Some("OwningMembership" | "FeatureMembership" | "ResultExpressionMembership"))
        }).collect();
        assert!(!memberships.is_empty(), "{source}");
        for member in memberships {
            let owner_id = member.properties["owning_related_element"].as_str().unwrap();
            let owner = document.elements.iter().find(|e| e.id == owner_id).unwrap();
            assert!(owner.properties["owned_relationship"].as_array().unwrap().contains(&serde_json::json!(member.id)));
            let children = member.properties["owned_related_element"].as_array().unwrap();
            assert_eq!(children.len(), 1);
            let child = document.elements.iter().find(|e| Some(e.id.as_str()) == children[0].as_str()).unwrap();
            assert_eq!(child.properties["owning_relationship"], member.id);
        }
        if source.starts_with("comment") {
            let root = document.elements.iter().find(|e| e.kind == "SysML::Namespace").unwrap();
            assert!(root.id.starts_with("namespace.resource."));
            assert_eq!(root.properties["metadata"]["generated"], true);
            assert!(!document.elements.iter().any(|e| e.id == "pkg.root"));
        } else {
            assert!(document.elements.iter().any(|e| e.kind.ends_with("ResultExpressionMembership")));
        }
    }
}

#[test]
fn release_2026_08_relationship_owned_definitions_and_features_compile() {
    let library=load_sysml_baseline().unwrap();
    for source in [
        include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-owned-elements/positive-alias.kerml"),
        include_str!("../../../mercurio-tools/corpus/release-2026-08/relationship-owned-elements/positive-import.kerml"),
    ] {
        let module=crate::kerml::parse_kerml(source).unwrap();
        let doc=crate::kerml::compile_kerml_module_strict_with_context(&module,"relationship-owned.kerml",std::slice::from_ref(&module),&library).unwrap();
        let owned=doc.elements.iter().find(|e|e.properties.get("declared_name").is_some_and(|v|v=="Owned")).unwrap();
        let child=doc.elements.iter().find(|e|e.properties.get("declared_name").is_some_and(|v|v=="child")).unwrap();
        let owner=doc.elements.iter().find(|e|Some(e.id.as_str())==owned.properties["owning_relationship"].as_str()).unwrap();
        assert!(matches!(owner.kind.rsplit("::").next(),Some("Membership"|"MembershipImport")));
        assert_eq!(owner.properties["owned_related_element"],serde_json::json!([owned.id,child.id]));
        for element in [owned,child] {
            assert_eq!(element.properties["owning_relationship"],owner.id);
            assert_eq!(element.properties["owner"],owner.properties["owning_related_element"]);
            assert!(element.properties.get("owning_membership").is_none());
            assert!(element.properties.get("owning_namespace").is_none());
            assert!(element.properties.get("qualified_name").is_none());
        }
        assert!(!owner.properties["target"].as_array().unwrap().is_empty());
    }
}

#[test]
fn release_2026_08_relationship_body_nonfeature_and_feature_families() {
    let library = load_sysml_baseline().unwrap();
    for (body, names) in [
        ("package Owned { class Nested; }", &["Owned"][..]),
        ("function Owned;", &["Owned"][..]),
        ("connector child;", &["child"][..]),
    ] {
        let source = format!("package P {{ class A; alias Link for A {{ {body} }} }}");
        let module = crate::kerml::parse_kerml(&source).unwrap_or_else(|error| panic!("{body}: {error}"));
        let document = crate::kerml::compile_kerml_module_strict_with_context(
            &module, "relationship-families.kerml", std::slice::from_ref(&module), &library,
        ).unwrap_or_else(|error| panic!("{body}: {error}"));
        let relation = document.elements.iter().find(|element| element.kind == "SysML::Membership"
            && element.id.starts_with("membership.alias.P.Link."))
            .unwrap_or_else(|| panic!("{body}: missing alias relationship; {:?}", document.elements.iter().filter(|element| element.kind.contains("Membership")).map(|element| (&element.id, &element.kind, element.properties.get("declared_name"))).collect::<Vec<_>>()));
        for name in names {
            let child = document.elements.iter().find(|element| element.properties.get("declared_name") == Some(&serde_json::json!(name)))
                .unwrap_or_else(|| panic!("{body}: missing {name}"));
            assert_eq!(child.properties.get("owning_relationship"), Some(&serde_json::json!(relation.id)), "{body}");
            assert!(relation.properties["owned_related_element"].as_array().unwrap().contains(&serde_json::json!(child.id)), "{body}");
        }
    }
}

#[test]
fn release_2026_08_alias_nested_in_definition_retains_owner_links() {
    let library = load_sysml_baseline().unwrap();
    let module = crate::kerml::parse_kerml("package P { class A; class Container { alias Link for P::A { class Owned; feature child; } } }").unwrap();
    let doc = crate::kerml::compile_kerml_module_strict_with_context(&module, "nested-alias.kerml", std::slice::from_ref(&module), &library).unwrap();
    let alias = doc.elements.iter().find(|e| e.properties.get("member_name").is_some_and(|v| v == "Link")).unwrap();
    let owner = doc.elements.iter().find(|e| Some(e.id.as_str()) == alias.properties["owning_related_element"].as_str()).unwrap();
    assert_eq!(owner.properties["declared_name"], "Container");
    for key in ["owned_relationship", "owned_membership", "membership"] {
        assert!(owner.properties[key].as_array().unwrap().contains(&serde_json::json!(alias.id)), "{key}");
    }
    assert_eq!(alias.properties["owned_related_element"].as_array().unwrap().len(), 2);
}

#[test]
fn release_2026_08_relationship_body_nested_aliases_collect_recursively() {
    let library = load_sysml_baseline().unwrap();
    let module = crate::kerml::parse_kerml("package P { class A; alias Outer for A { class Container { alias Inner for P::A { class Deep; } } } }").unwrap();
    let doc = crate::kerml::compile_kerml_module_strict_with_context(&module, "recursive-alias.kerml", std::slice::from_ref(&module), &library).unwrap();
    let inner = doc.elements.iter().find(|e| e.properties.get("member_name").is_some_and(|v| v == "Inner")).unwrap();
    let deep = doc.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "Deep")).unwrap();
    assert_eq!(deep.properties["owning_relationship"], inner.id);
    assert_eq!(inner.properties["owned_related_element"], serde_json::json!([deep.id]));
}

#[test]
fn release_2026_08_member_expose_uses_emitted_usage_membership() {
    let library = load_sysml_baseline().unwrap();
    let module = parse_sysml(include_str!("../../../mercurio-tools/corpus/release-2026-08/expose-bodies/positive-membership.sysml")).unwrap();
    let doc = compile_sysml_module(&module, "member-expose.sysml", &library).unwrap();
    let vehicle = doc.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "vehicle")).unwrap();
    let expose = doc.elements.iter().find(|e| e.kind.ends_with("MembershipExpose")).unwrap();
    assert_eq!(expose.properties["imported_membership"], vehicle.properties["owning_membership"]);
    assert_eq!(expose.properties["target"], serde_json::json!([vehicle.properties["owning_membership"]]));
    assert_eq!(expose.properties["owned_relationship"].as_array().unwrap().len(), 3);
}

#[test]
fn release_2026_08_alias_uses_emitted_usage_identity() {
    let library = load_sysml_baseline().unwrap();
    let module = parse_sysml("package P { part vehicle; alias Link for vehicle; }").unwrap();
    let doc = compile_sysml_module(&module, "usage-alias.sysml", &library).unwrap();
    let vehicle = doc.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "vehicle")).unwrap();
    let alias = doc.elements.iter().find(|e| e.properties.get("member_name").is_some_and(|v| v == "Link")).unwrap();
    assert_eq!(alias.properties["member_element"], vehicle.id);
    assert_eq!(alias.properties["target"], serde_json::json!([vehicle.id]));
}

#[test]
fn release_2026_08_qualified_membership_visibility_controls() {
    let library = load_sysml_baseline().unwrap();
    let cases = [
        ("invalid-private-alias.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/qualified-visibility/invalid-private-alias.sysml"), false),
        ("invalid-private-prefix.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/qualified-visibility/invalid-private-prefix.sysml"), false),
        ("invalid-private-type.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/qualified-visibility/invalid-private-type.sysml"), false),
        ("invalid-protected-type.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/qualified-visibility/invalid-protected-type.sysml"), false),
        ("positive-lexical-private.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/qualified-visibility/positive-lexical-private.sysml"), true),
        ("positive-public-alias.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/qualified-visibility/positive-public-alias.sysml"), true),
        ("positive-public-prefix.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/qualified-visibility/positive-public-prefix.sysml"), true),
        ("positive-public-type.sysml", include_str!("../../../mercurio-tools/corpus/release-2026-08/qualified-visibility/positive-public-type.sysml"), true),
    ];
    for (name, source, valid) in cases {
        let module = parse_sysml(source).unwrap();
        let result = compile_sysml_module(&module, name, &library);
        assert_eq!(result.is_ok(), valid, "{name}: {result:?}");
        if let Err(error) = result { assert!(error.message.contains("non-public membership"), "{name}: {error:?}"); }
    }
}

#[test]
fn release_2026_08_qualified_visibility_checks_reference_roles() {
    let library = load_sysml_baseline().unwrap();
    for source in [
        "package P { private part def Hidden; } package Q { private import P::Hidden; }",
        "package P { private part def Hidden; } package Q { alias Visible for P::Hidden; }",
        "package P { private part def Hidden; } package Q { part def Visible :> P::Hidden; }",
        "package P { private attribute Hidden = 1; } package Q { attribute x = P::Hidden; }",
    ] {
        let module = parse_sysml(source).unwrap();
        let error = compile_sysml_module(&module, "qualified-roles.sysml", &library).unwrap_err();
        assert!(error.message.contains("non-public membership"), "{source}: {error:?}");
        assert!(error.span.is_some());
    }
}

#[test]
fn release_2026_08_inherited_membership_visibility_controls() {
    let library = load_sysml_baseline().unwrap();
    let cases = [
        ("invalid-private-alias.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-visibility/invalid-private-alias.kerml"), false),
        ("invalid-private-feature.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-visibility/invalid-private-feature.kerml"), false),
        ("invalid-qualified-base.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-visibility/invalid-qualified-base.kerml"), false),
        ("invalid-qualified-inherited.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-visibility/invalid-qualified-inherited.kerml"), false),
        ("positive-nested-protected.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-visibility/positive-nested-protected.kerml"), true),
        ("positive-protected-alias.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-visibility/positive-protected-alias.kerml"), true),
        ("positive-protected-feature.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-visibility/positive-protected-feature.kerml"), true),
        ("positive-public-alias.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-visibility/positive-public-alias.kerml"), true),
    ];
    for (name, source, valid) in cases {
        let module = crate::kerml::parse_kerml(source).unwrap();
        let result = crate::kerml::compile_kerml_module_strict_with_context(&module, name, std::slice::from_ref(&module), &library);
        assert_eq!(result.is_ok(), valid, "{name}: {result:?}");
        match result {
            Err(error) => assert!(error.message.contains("Hidden"), "{name}: {error:?}"),
            Ok(doc) => {
                let target_name = if name == "positive-protected-alias.kerml" { "original" } else { "Hidden" };
                let source_name = if name == "positive-nested-protected.kerml" { "c" } else { "b" };
                let target = doc.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == target_name)).unwrap();
                let source = doc.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == source_name)).unwrap();
                assert!(source.properties["specializes"].as_array().unwrap().contains(&serde_json::json!(target.id)), "{name}: {source:?}");
            }
        }
    }
}

#[test]
fn release_2026_08_inherited_type_visibility_controls() {
    let library = load_sysml_baseline().unwrap();
    let cases = [
        ("invalid-private-specialization.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-types/invalid-private-specialization.kerml"), false),
        ("invalid-private-type.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-types/invalid-private-type.kerml"), false),
        ("invalid-qualified-inherited.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-types/invalid-qualified-inherited.kerml"), false),
        ("invalid-qualified-protected.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-types/invalid-qualified-protected.kerml"), false),
        ("positive-protected-type.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-types/positive-protected-type.kerml"), true),
        ("positive-public-alias.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-types/positive-public-alias.kerml"), true),
        ("positive-qualified-public.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-types/positive-qualified-public.kerml"), true),
        ("positive-specialization.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/inherited-types/positive-specialization.kerml"), true),
    ];
    for (name, source, valid) in cases {
        let module = crate::kerml::parse_kerml(source).unwrap();
        let result = crate::kerml::compile_kerml_module_strict_with_context(&module, name, std::slice::from_ref(&module), &library);
        assert_eq!(result.is_ok(), valid, "{name}: {result:?}");
        match result {
            Err(error) => assert!(error.message.contains("Hidden"), "{name}: {error:?}"),
            Ok(doc) => {
                let named = |name: &str| doc.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == name)).unwrap();
                let target = named(if name == "positive-qualified-public.kerml" { "Visible" } else { "Hidden" });
                if name == "positive-specialization.kerml" || name == "positive-public-alias.kerml" {
                    assert!(named("Derived").properties["specializes"].as_array().unwrap().contains(&serde_json::json!(target.id)), "{name}");
                }
                if name == "positive-specialization.kerml" {
                    assert!(named("b").properties["specializes"].as_array().unwrap().contains(&serde_json::json!(named("original").id)), "{name}");
                } else {
                    assert_eq!(named("b").properties["type"], target.id, "{name}");
                }
            }
        }
    }
}

#[test]
fn release_2026_08_imported_membership_visibility_controls() {
    let library = load_sysml_baseline().unwrap();
    let cases = [
        ("invalid-explicit-private.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-visibility/invalid-explicit-private.kerml"), false),
        ("invalid-explicit-protected.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-visibility/invalid-explicit-protected.kerml"), false),
        ("invalid-qualified-private-import.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-visibility/invalid-qualified-private-import.kerml"), false),
        ("invalid-reexport-private.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-visibility/invalid-reexport-private.kerml"), false),
        ("positive-import-all-private.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-visibility/positive-import-all-private.kerml"), true),
        ("positive-public-alias-private.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-visibility/positive-public-alias-private.kerml"), true),
        ("positive-qualified-public-import.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-visibility/positive-qualified-public-import.kerml"), true),
        ("positive-reexport-public.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-visibility/positive-reexport-public.kerml"), true),
    ];
    for (name, source, valid) in cases {
        let module = crate::kerml::parse_kerml(source).unwrap();
        let result = crate::kerml::compile_kerml_module_strict_with_context(&module, name, std::slice::from_ref(&module), &library);
        assert_eq!(result.is_ok(), valid, "{name}: {result:?}");
        if let Ok(doc) = result {
            let derived = doc.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "B")).unwrap();
            let target_name = if matches!(name, "positive-import-all-private.kerml" | "positive-public-alias-private.kerml") { "Hidden" } else { "Visible" };
            let target = doc.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == target_name)).unwrap();
            assert!(derived.properties["specializes"].as_array().unwrap().contains(&serde_json::json!(target.id)), "{name}: {derived:?}");
        }
    }
}

#[test]
fn release_2026_08_imported_feature_visibility_controls() {
    let library = load_sysml_baseline().unwrap();
    let cases = [
        ("invalid-private-reexport.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-feature-visibility/invalid-private-reexport.kerml"), false),
        ("invalid-private-sibling.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-feature-visibility/invalid-private-sibling.kerml"), false),
        ("invalid-qualified-private.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-feature-visibility/invalid-qualified-private.kerml"), false),
        ("positive-import-all-private.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-feature-visibility/positive-import-all-private.kerml"), true),
        ("positive-local-private.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-feature-visibility/positive-local-private.kerml"), true),
        ("positive-public-alias-private.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-feature-visibility/positive-public-alias-private.kerml"), true),
        ("positive-public-reexport.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-feature-visibility/positive-public-reexport.kerml"), true),
        ("positive-qualified-public.kerml", include_str!("../../../mercurio-tools/corpus/release-2026-08/import-feature-visibility/positive-qualified-public.kerml"), true),
    ];
    for (name, source, valid) in cases {
        let module = crate::kerml::parse_kerml(source).unwrap();
        let result = crate::kerml::compile_kerml_module_strict_with_context(&module, name, std::slice::from_ref(&module), &library);
        assert_eq!(result.is_ok(), valid, "{name}: {result:?}");
        if let Ok(doc) = result {
            let source = doc.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == "x")).unwrap();
            let target_name = if matches!(name, "positive-import-all-private.kerml" | "positive-public-alias-private.kerml") { "Hidden" } else { "Visible" };
            let target = doc.elements.iter().find(|e| e.properties.get("declared_name").is_some_and(|v| v == target_name)).unwrap();
            assert!(source.properties["specializes"].as_array().unwrap().contains(&serde_json::json!(target.id)), "{name}: {source:?}");
        }
    }
}


#[test]
fn release_2026_08_documentation_generated_context_and_value_boundaries() {
    for kerml in [true, false] {
        let parse = |text: &str| if kerml { crate::kerml::parse_kerml(text) } else { parse_sysml(text) };
        let source = r#"package P { doc <'short\tname'> 'Named\tDoc' locale "en\tUS" /* raw\nbody */ doc /**/ }"#;
        let module = parse(source).unwrap();
        let members = &module.package.as_ref().unwrap().members;
        assert_eq!(members.len(), 2);
        let doc = members[0].as_usage_like().unwrap();
        assert_eq!(doc.name, "Named\tDoc");
        assert!(doc.modifiers.contains(&"short_name=short\tname".to_string()));
        assert_eq!(doc.metadata_properties["locale"], r"en\tUS");
        assert_eq!(doc.metadata_properties["body"], r" raw\nbody ");
        assert!(members[1].as_usage_like().unwrap().is_implicit_name);
        for reserved in ["package", "doc", "locale"] {
            assert!(parse(&format!("package P {{ doc {reserved} /* body */ }}")).is_err());
        }
        let language_specific = "package P { doc part /* body */ }";
        assert_eq!(parse(language_specific).is_ok(), kerml);
    }
}


#[test]
fn release_2026_08_textual_generated_context_and_value_boundaries() {
    for kerml in [true, false] {
        let parse = |text: &str| if kerml { crate::kerml::parse_kerml(text) } else { parse_sysml(text) };
        let source = r#"package P { rep <'short\tname'> 'Named\tRep' language "text\tplain" /* raw\nbody */ language "text" /**/ }"#;
        let module = parse(source).unwrap();
        let members = &module.package.as_ref().unwrap().members;
        assert_eq!(members.len(), 2);
        let rep = members[0].as_usage_like().unwrap();
        assert_eq!(rep.name, "Named\tRep");
        assert!(rep.modifiers.contains(&"short_name=short\tname".to_string()));
        assert_eq!(rep.metadata_properties["language"], r"text\tplain");
        assert_eq!(rep.metadata_properties["body"], r" raw\nbody ");
        assert!(members[1].as_usage_like().unwrap().is_implicit_name);
        for invalid in [r#"rep package language "text" /**/"#, r#"rep <s> <t> language "text" /**/"#,
            r#"language "bad\q" /**/"#, r#"rep R /**/"#, r#"language "text""#] {
            assert!(parse(&format!("package P {{ {invalid} }}")).is_err(), "{invalid}");
        }
        assert_eq!(parse(r#"package P { rep part language "text" /**/ }"#).is_ok(), kerml);
    }
}


#[test]
fn release_2026_08_comment_generated_targets_preserve_order_spans_and_context() {
    for kerml in [true, false] {
        let parse = |text: &str| if kerml { crate::kerml::parse_kerml(text) } else { parse_sysml(text) };
        let source = "package P {\n comment <s> C about P::'A.B',\n 'B\\tC', P::'A.B' locale \"en\\tUS\" /* raw */\n /**/ }";
        let module = parse(source).unwrap();
        let members = &module.package.as_ref().unwrap().members;
        assert_eq!(members.len(), 2);
        let comment = members[0].as_usage_like().unwrap();
        assert_eq!(comment.annotation_targets.len(), 3);
        assert_eq!(comment.annotation_targets[0].segments, ["P", "A.B"]);
        assert_eq!(comment.annotation_targets[1].segments, ["B\tC"]);
        assert_eq!(comment.annotation_targets[2].segments, ["P", "A.B"]);
        assert_eq!(comment.reference_target.as_ref(), comment.annotation_targets.first());
        assert_eq!(comment.metadata_properties["locale"], r"en\tUS");
        for (target, spelling) in comment.annotation_targets.iter().zip(["P::'A.B'", "'B\\tC'", "P::'A.B'"]) {
            let span = &target.span;
            assert_eq!(span.start_line, span.end_line);
            let line = source.lines().nth(span.start_line-1).unwrap();
            assert_eq!(&line[span.start_col-1..span.end_col], spelling);
        }
        assert_ne!(comment.annotation_targets[0].span, comment.annotation_targets[2].span);
        assert_eq!(members[1].as_usage_like().unwrap().metadata_properties["__bare_comment"], "true");
        for invalid in ["comment C about /* body */", "comment about A, /* body */",
            "comment about A.B /* body */", "comment package /* body */", "locale about A /* body */",
            "comment C locale /* body */", "comment <s> <t> /* body */"] {
            assert!(parse(&format!("package P {{ {invalid} }}")).is_err(), "{invalid}");
        }
        assert_eq!(parse("package P { comment about part /* body */ }").is_ok(), kerml);
    }
}


#[test]
fn release_2026_08_qualified_alias_targets_follow_imported_name_grammar() {
    for kerml in [true, false] {
        let parse = |source: &str| if kerml { crate::kerml::parse_kerml(source) } else { parse_sysml(source) };
        let source = "package P { alias X for P::'A.B'; }";
        let module = parse(source).unwrap();
        let Declaration::Alias(alias) = &module.package.as_ref().unwrap().members[0] else { panic!("expected alias"); };
        assert_eq!(alias.target.segments, ["P", "A.B"]);
        for target in ["P::", "P.member", "package"] {
            assert!(parse(&format!("package P {{ alias X for {target}; }}")).is_err(), "{target}");
        }
        assert_eq!(parse("package P { alias X for part; }").is_ok(), kerml);
        assert!(parse("package P { alias X for 'part'; }").is_ok());
    }
}


#[test]
fn release_2026_08_generated_alias_references_fit_small_stack() {
    let mut source = "package P {".to_string();
    for i in 0..8 { source.push_str(&format!("part p{i} {{")); }
    source.push_str("alias X for P::Target;");
    source.push_str(&"}".repeat(9));
    assert!(std::thread::Builder::new().stack_size(1024 * 1024)
        .spawn(move || parse_sysml(&source).is_ok()).unwrap().join().unwrap());
}


#[test]
fn release_2026_08_generated_relationship_operands_fit_small_stack() {
    let mut source = "package P {".to_string();
    for i in 0..8 { source.push_str(&format!("class C{i} {{")); }
    source.push_str("feature x; feature y; subset x :> y;");
    source.push_str(&"}".repeat(9));
    assert!(std::thread::Builder::new().stack_size(1024 * 1024)
        .spawn(move || crate::kerml::parse_kerml(&source).is_ok()).unwrap().join().unwrap());
}

#[test]
fn release_2026_08_relationship_chain_uses_previous_feature_scope() {
    let library = crate::kerml::load_kernel_baseline().unwrap();
    let source = "package ChainScope {
        class A { feature x : B; }
        class B { feature y : C; }
        class C { feature z : A; }
        feature y : A;
        feature missing : A;
        inverting Invert inverse A::x.y.z of C::z;
    }";
    let compile = |text: &str| {
        let module = crate::kerml::parse_kerml(text).unwrap();
        crate::kerml::compile_kerml_module_strict_with_context(
            &module, "chain-scope.kerml", std::slice::from_ref(&module), &library)
    };
    let document = compile(source).unwrap();
    let inverse = document.elements.iter().find(|element|
        element.properties.get("declared_name").is_some_and(|value| value == "Invert")).unwrap();
    let chain = document.elements.iter().find(|element|
        Some(element.id.as_str()) == inverse.properties["source"][0].as_str()).unwrap();
    assert_eq!(chain.properties["chaining_feature"], serde_json::json!([
        "feature.ChainScope.A.x", "feature.ChainScope.B.y", "feature.ChainScope.C.z"
    ]));
    // A matching lexical/global name cannot rescue a missing scoped step.
    assert!(compile(&source.replace("A::x.y.z", "A::x.missing")).is_err());
    // Qualification traverses members without adding intermediate chain entries.
    let qualified = compile(&source.replace("A::x.y.z", "A::x.y::z")).unwrap();
    let chain = qualified.elements.iter().find(|element|
        element.properties.get("chaining_feature").is_some_and(|value| value.is_array())).unwrap();
    assert_eq!(chain.properties["chaining_feature"], serde_json::json!([
        "feature.ChainScope.A.x", "feature.ChainScope.C.z"
    ]));
}

#[test]
fn release_2026_08_relationship_chain_links_match_pilot_resources() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/conformance/2026-08-support/chain-link-pilot-controls.json")).unwrap();
    let library = crate::kerml::load_kernel_baseline().unwrap();
    let cases = oracle["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 29);
    for case in cases {
        let source = case["source"].as_str().unwrap();
        let module = crate::kerml::parse_kerml(source).unwrap();
        let result = crate::kerml::compile_kerml_module_strict_with_context(
            &module, "chain-link.kerml", std::slice::from_ref(&module), &library);
        let links = case["links"].as_array().unwrap();
        if links.iter().any(|link| link["resolved"] == false) {
            let error = result.err().unwrap_or_else(|| panic!("native resolved a Pilot-unresolved chain: {source}"));
            let message = format!("{error:?}");
            assert!(message.contains("unresolved qualified member") || message.contains("chain step requires a Feature"), "{source}: {error:?}");
            continue;
        }
        let document = result.unwrap();
        let relationship = document.elements.iter().find(|element|
            element.properties.get("declared_name").is_some_and(|value| value == "Invert")).unwrap();
        let chain = document.elements.iter().find(|element|
            Some(element.id.as_str()) == relationship.properties["source"][0].as_str()).unwrap();
        let expected: Vec<_> = links.iter().map(|link| {
            let id = format!("feature.{}", link["target_path"].as_array().unwrap().iter()
                .map(|part| part.as_str().unwrap()).collect::<Vec<_>>().join("."));
            let target = document.elements.iter().find(|element| element.id == id).unwrap();
            assert_eq!(target.kind.rsplit("::").next().unwrap(), link["target_kind"].as_str().unwrap());
            id
        }).collect();
        assert_eq!(chain.properties["chaining_feature"], serde_json::json!(expected), "{source}");
        assert_eq!(chain.properties["owned_relationship"].as_array().unwrap().len(), expected.len());
    }
}

#[test]
fn release_2026_08_relationship_identification_matches_pilot_names() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/conformance/2026-08-support/chain-link-pilot-controls.json")).unwrap();
    let cases = oracle["identification_controls"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for case in cases {
        let source = case["source"].as_str().unwrap();
        assert_eq!(crate::kerml::parse_kerml(source).is_ok(), case["accepted"].as_bool().unwrap(), "{source}");
    }
}

#[test]
fn release_2026_08_relationship_chain_checks_external_ecore_types() {
    use mercurio_foundation::kir::KirElement;
    use std::collections::BTreeMap;
    let source = "package ExternalChain { class A { feature x : External::B; }
        inverting Invert inverse A::x.b of A::x; }";
    let module = crate::kerml::parse_kerml(source).unwrap();
    let mut library = crate::kerml::load_kernel_baseline().unwrap();
    library.elements.push(KirElement { id: "External::B".into(), kind: "Class".into(),
        layer: 0, properties: BTreeMap::new() });
    library.elements.push(KirElement { id: "External::B::b".into(), kind: "Feature".into(),
        layer: 0, properties: BTreeMap::new() });
    // Reuse identities while changing kinds: cached metadata must track content.
    for (kind, accepted) in [("Feature", true), ("MultiplicityRange", true),
        ("Class", false), ("Comment", false), ("Disjoining", false),
        ("UnknownMetaclass", false), ("Feature", true)] {
        library.elements.last_mut().unwrap().kind = kind.into();
        let result = crate::kerml::compile_kerml_module_strict_with_context(
            &module, "external-chain.kerml", std::slice::from_ref(&module), &library);
        if accepted {
            let document = result.unwrap_or_else(|error| panic!("{kind}: {error:?}"));
            assert!(document.elements.iter().any(|element|
                element.properties.get("chaining_feature") == Some(&serde_json::json!([
                    "feature.ExternalChain.A.x", "External::B::b"
                ]))));
        } else {
            let error = result.err().unwrap_or_else(|| panic!("accepted {kind} chain endpoint"));
            assert!(format!("{error:?}").contains("chain step requires a Feature"), "{kind}: {error:?}");
        }
    }
}

#[test]
fn release_2026_08_relationship_chain_uses_library_memberships() {
    use mercurio_foundation::kir::KirElement;
    use std::collections::BTreeMap;
    let mut library = crate::kerml::load_kernel_baseline().unwrap();
    library.elements.extend([
        KirElement { id: "External::B".into(), kind: "Class".into(), layer: 0, properties: BTreeMap::new() },
        KirElement { id: "External::B::old".into(), kind: "Feature".into(), layer: 0, properties: BTreeMap::new() },
        KirElement { id: "membership-opaque".into(), kind: "FeatureMembership".into(), layer: 0,
            properties: BTreeMap::from([
                ("member_element".into(), serde_json::json!("External::B::old")),
                ("membership_owning_namespace".into(), serde_json::json!(["External::B"])),
                ("member_name".into(), serde_json::json!("member")),
                ("member_short_name".into(), serde_json::json!("short")),
                ("visibility".into(), serde_json::json!("public")),
            ]) },
    ]);
    let compile = |library: &mercurio_foundation::KirDocument, name: &str| {
        let source = format!("package MemberGraph {{ class A {{ feature x : External::B; }} inverting Invert inverse A::x.'{name}' of A::x; }}");
        let module = crate::kerml::parse_kerml(&source).unwrap();
        crate::kerml::compile_kerml_module_strict_with_context(
            &module, "membership-chain.kerml", std::slice::from_ref(&module), library)
    };
    for name in ["member", "short"] {
        let document = compile(&library, name).unwrap();
        assert!(document.elements.iter().any(|element|
            element.properties.get("chaining_feature") == Some(&serde_json::json!([
                "feature.MemberGraph.A.x", "External::B::old"
            ]))));
    }
    assert!(compile(&library, "old").is_err());
    library.elements.last_mut().unwrap().properties.insert("member_name".into(), serde_json::json!("renamed"));
    assert!(compile(&library, "member").is_err());
    assert!(compile(&library, "renamed").is_ok());
    library.elements.last_mut().unwrap().properties.insert("visibility".into(), serde_json::json!("private"));
    assert!(compile(&library, "renamed").is_err());
    assert!(compile(&library, "short").is_err());
    library.elements.push(KirElement { id: "alias-opaque".into(), kind: "Membership".into(), layer: 0,
        properties: BTreeMap::from([
            ("member_element".into(), serde_json::json!(["External::B::old"])),
            ("membership_owning_namespace".into(), serde_json::json!("External::B")),
            ("member_name".into(), serde_json::json!("exposed")),
            ("visibility".into(), serde_json::json!("public")),
        ]) });
    assert!(compile(&library, "exposed").is_ok());
    assert!(compile(&library, "old").is_err());
    library.elements.push(KirElement { id: "External::other".into(), kind: "Feature".into(),
        layer: 0, properties: BTreeMap::new() });
    library.elements.push(KirElement { id: "competing-alias".into(), kind: "Membership".into(), layer: 0,
        properties: BTreeMap::from([
            ("member_element".into(), serde_json::json!("External::other")),
            ("membership_owning_namespace".into(), serde_json::json!("External::B")),
            ("member_short_name".into(), serde_json::json!("exposed")),
            ("visibility".into(), serde_json::json!("public")),
        ]) });
    assert!(compile(&library, "exposed").is_err());
    library.elements.reverse();
    assert!(compile(&library, "exposed").is_err());
}

#[test]
fn release_2026_08_relationship_chain_uses_generated_library_names() {
    use mercurio_foundation::kir::KirElement;
    use serde_json::json;
    use std::collections::BTreeMap;
    let element = |id: &str, kind: &str, properties| KirElement {
        id: id.into(), kind: kind.into(), layer: 0, properties,
    };
    let mut library = crate::kerml::load_kernel_baseline().unwrap();
    let first = library.elements.len();
    library.elements.extend([
        element("External::B", "Class", BTreeMap::new()),
        element("External::B::opaque", "Feature", BTreeMap::from([
            ("declared_name".into(), json!("derived")),
            ("declared_short_name".into(), json!("d")),
        ])),
        element("membership-opaque", "FeatureMembership", BTreeMap::from([
            ("member_element".into(), json!("External::B::opaque")),
            ("membership_owning_namespace".into(), json!("External::B")),
        ])),
    ]);
    let compile = |library: &mercurio_foundation::KirDocument, name: &str| {
        let source = format!("package DerivedNames {{ class A {{ feature x : External::B; }} inverting Invert inverse A::x.'{name}' of A::x; }}");
        let module = crate::kerml::parse_kerml(&source).unwrap();
        crate::kerml::compile_kerml_module_strict_with_context(
            &module, "derived-names.kerml", std::slice::from_ref(&module), library)
    };
    for name in ["derived", "d"] {
        let document = compile(&library, name).unwrap();
        assert!(document.elements.iter().any(|element|
            element.properties.get("chaining_feature") == Some(&json!([
                "feature.DerivedNames.A.x", "External::B::opaque"
            ]))));
    }
    // Canonical Ecore subtype fields and inherited projections resolve alike.
    let member = &mut library.elements[first + 2];
    member.properties.remove("member_element");
    member.properties.remove("membership_owning_namespace");
    member.properties.insert("owned_member_feature".into(), json!(["External::B::opaque"]));
    member.properties.insert("owning_type".into(), json!("External::B"));
    assert!(compile(&library, "derived").is_ok());
    library.elements[first + 1].kind = "Class".into();
    let error = compile(&library, "derived").unwrap_err();
    assert!(format!("{error:?}").contains("target mismatch"), "{error:?}");
    library.elements[first + 1].kind = "Feature".into();
    assert!(compile(&library, "opaque").is_err());
    library.elements[first + 1].properties.insert("declared_name".into(), json!("renamed"));
    assert!(compile(&library, "derived").is_err());
    assert!(compile(&library, "renamed").is_ok());
    // Explicit redefinition is a supported dependency of the shared dispatch.
    library.elements[first + 1].properties = BTreeMap::from([
        ("owned_relationship".into(), json!(["naming-redefinition"])),
    ]);
    library.elements.extend([
        element("naming-redefinition", "Redefinition", BTreeMap::from([
            ("owning_related_element".into(), json!("External::B::opaque")),
            ("redefining_feature".into(), json!("External::B::opaque")),
            ("redefined_feature".into(), json!("naming-feature")),
        ])),
        element("naming-feature", "Feature", BTreeMap::from([
            ("declared_name".into(), json!("inherited")),
            ("declared_short_name".into(), json!("i")),
        ])),
    ]);
    assert!(compile(&library, "inherited").is_ok());
    assert!(compile(&library, "i").is_ok());
    assert!(compile(&library, "renamed").is_err());
    library.elements.last_mut().unwrap().properties.insert("declared_name".into(), json!("updated"));
    assert!(compile(&library, "inherited").is_err());
    assert!(compile(&library, "updated").is_ok());
    library.elements[first + 1].properties.clear();
    let error = compile(&library, "updated").unwrap_err();
    assert!(format!("{error:?}").contains("computed redefinitions require complete canonical Feature relationships"), "{error:?}");
    // A failed index build must not cache success or mask a later repair.
    library.elements[first + 1].properties.insert("declared_name".into(), json!("repaired"));
    assert!(compile(&library, "repaired").is_ok());
    // The same public pipeline accepts canonical stored ownership without any
    // materialized membership endpoints or names.
    library.elements[first].properties.insert("owned_relationship".into(), json!(["membership-opaque"]));
    library.elements[first + 1].properties.insert("owning_relationship".into(), json!("membership-opaque"));
    library.elements[first + 2].properties = BTreeMap::from([
        ("owning_related_element".into(), json!("External::B")),
        ("owned_related_element".into(), json!(["External::B::opaque"])),
    ]);
    assert!(compile(&library, "repaired").is_ok());
    library.elements[first + 2].properties.insert("owned_member_name".into(), json!("stale"));
    let error = compile(&library, "stale").unwrap_err();
    assert!(format!("{error:?}").contains("name snapshot disagrees"), "{error:?}");
    library.elements[first + 2].properties.remove("owned_member_name");
    library.elements[first + 1].properties.remove("owning_relationship");
    let error = compile(&library, "repaired").unwrap_err();
    assert!(format!("{error:?}").contains("nonreciprocal"), "{error:?}");

}

#[test]
fn release_2026_08_external_type_names_follow_membership_graph() {
    use mercurio_foundation::kir::KirElement;
    use serde_json::json;
    use std::collections::BTreeMap;
    let element = |id: &str, kind: &str, properties| KirElement {
        id: id.into(), kind: kind.into(), layer: 0, properties,
    };
    let mut library = crate::kerml::load_kernel_baseline().unwrap();
    let start = library.elements.len();
    library.elements.extend([
        element("External", "Package", BTreeMap::from([("owned_relationship".into(), json!(["class-member"]))])),
        element("External::old", "Class", BTreeMap::from([
            ("declared_name".into(), json!("Fresh")),
            ("declared_short_name".into(), json!("F")),
            ("owning_relationship".into(), json!("class-member")),
            ("owned_relationship".into(), json!(["inner-member"])),
        ])),
        element("class-member", "OwningMembership", BTreeMap::from([
            ("owning_related_element".into(), json!("External")),
            ("owned_related_element".into(), json!(["External::old"])),
        ])),
        element("opaque-inner", "Class", BTreeMap::from([
            ("declared_name".into(), json!("Inner")),
            ("owning_relationship".into(), json!("inner-member")),
        ])),
        element("inner-member", "OwningMembership", BTreeMap::from([
            ("owning_related_element".into(), json!("External::old")),
            ("owned_related_element".into(), json!(["opaque-inner"])),
        ])),
    ]);
    let compile = |library: &mercurio_foundation::KirDocument, name: &str| {
        let source = format!("package TypeGraph {{ feature x : {name}; }}");
        let module = crate::kerml::parse_kerml(&source).unwrap();
        crate::kerml::compile_kerml_module_strict_with_context(
            &module, "type-graph.kerml", std::slice::from_ref(&module), library)
    };
    for (name, target) in [("External::Fresh", "External::old"), ("External::F", "External::old"), ("External::Fresh::Inner", "opaque-inner")] {
        let document = compile(&library, name).unwrap();
        let feature = document.elements.iter().find(|element| element.id == "feature.TypeGraph.x").unwrap();
        assert_eq!(feature.properties.get("type"), Some(&json!(target)));
    }
    assert!(compile(&library, "External::old").is_err());
    library.elements[start + 2].properties.insert("visibility".into(), json!("private"));
    assert!(compile(&library, "External::Fresh").is_err());
    library.elements.push(element("alias", "Membership", BTreeMap::from([
        ("membership_owning_namespace".into(), json!("External")),
        ("member_element".into(), json!("External::old")),
        ("member_name".into(), json!("Facade")),
    ])));
    assert!(compile(&library, "External::Facade::Inner").is_ok());
    library.elements[start + 4].properties.insert("visibility".into(), json!("protected"));
    assert!(compile(&library, "External::Facade::Inner").is_err());
    library.elements[start + 4].properties.insert("visibility".into(), json!("public"));
    library.elements.extend([
        element("opaque-comment", "Comment", BTreeMap::new()),
        element("opaque-comment::Nested", "Class", BTreeMap::new()),
        element("non-namespace-alias", "Membership", BTreeMap::from([
            ("membership_owning_namespace".into(), json!("External")),
            ("member_element".into(), json!("opaque-comment")),
            ("member_name".into(), json!("NotNamespace")),
        ])),
    ]);
    assert!(compile(&library, "External::NotNamespace::Nested").is_err());
    library.elements.push(element("competing-alias", "Membership", BTreeMap::from([
        ("membership_owning_namespace".into(), json!("External")),
        ("member_element".into(), json!("opaque-inner")),
        ("member_short_name".into(), json!("Facade")),
    ])));
    assert!(compile(&library, "External::Facade").is_err());
    library.elements.reverse();
    assert!(compile(&library, "External::Facade").is_err());
}

#[test]
fn release_2026_08_external_type_names_follow_canonical_specializations() {
    use mercurio_foundation::kir::KirElement;
    use serde_json::json;
    use std::collections::BTreeMap;
    let element = |id: &str, kind: &str, properties| KirElement { id: id.into(), kind: kind.into(), layer: 0, properties };
    let mut library = crate::kerml::load_kernel_baseline().unwrap();
    let start = library.elements.len();
    library.elements.extend([
        element("InheritanceTestBase", "Class", BTreeMap::from([("owned_relationship".into(), json!(["member"]))])),
        element("widget-id", "Class", BTreeMap::from([
            ("declared_name".into(), json!("Widget")), ("owning_relationship".into(), json!("member")),
        ])),
        element("member", "OwningMembership", BTreeMap::from([
            ("owning_related_element".into(), json!("InheritanceTestBase")), ("owned_related_element".into(), json!(["widget-id"])),
        ])),
        element("InheritanceTestDerived", "Class", BTreeMap::from([("owned_relationship".into(), json!(["inheritance"]))])),
        element("inheritance", "Subclassification", BTreeMap::from([
            ("owning_related_element".into(), json!("InheritanceTestDerived")),
            ("subclassifier".into(), json!("InheritanceTestDerived")), ("superclassifier".into(), json!("InheritanceTestBase")),
        ])),
    ]);
    let compile = |library: &mercurio_foundation::KirDocument| {
        let module = crate::kerml::parse_kerml("package InheritedTypes { feature x : InheritanceTestDerived::Widget; }").unwrap();
        crate::kerml::compile_kerml_module_strict_with_context(
            &module, "inherited-types.kerml", std::slice::from_ref(&module), library)
    };
    let document = compile(&library).unwrap();
    let feature = document.elements.iter().find(|element| element.id == "feature.InheritedTypes.x").unwrap();
    assert_eq!(feature.properties.get("type"), Some(&json!("widget-id")));
    library.elements[start + 2].properties.insert("visibility".into(), json!("private"));
    assert!(compile(&library).is_err());
    library.elements[start + 2].properties.insert("visibility".into(), json!("public"));
    library.elements[start + 3].properties.insert("specializes".into(), json!([]));
    let error = compile(&library).unwrap_err();
    assert!(format!("{error:?}").contains("Specialization snapshot disagrees"), "{error:?}");
    library.elements[start + 3].properties.remove("specializes");
    library.elements[start + 3].properties.insert("owned_relationship".into(), json!(["inheritance", "other-inheritance"]));
    library.elements.extend([
        element("InheritanceTestOther", "Class", BTreeMap::new()),
        element("other-widget", "Class", BTreeMap::new()),
        element("alias", "Membership", BTreeMap::from([
            ("membership_owning_namespace".into(), json!("InheritanceTestOther")),
            ("member_element".into(), json!("other-widget")), ("member_name".into(), json!("Widget")),
        ])),
        element("other-inheritance", "Subclassification", BTreeMap::from([
            ("owning_related_element".into(), json!("InheritanceTestDerived")),
            ("subclassifier".into(), json!("InheritanceTestDerived")), ("superclassifier".into(), json!("InheritanceTestOther")),
        ])),
    ]);
    assert!(compile(&library).is_err());
    library.elements.last_mut().unwrap().properties.insert("superclassifier".into(), json!("InheritanceTestBase"));
    assert!(compile(&library).is_ok());
}

#[test]
fn release_2026_08_external_package_imports_use_shared_graph_scope() {
    use mercurio_foundation::kir::KirElement;
    use serde_json::json;
    use std::collections::BTreeMap;
    let element = |id: &str, kind: &str, properties| KirElement { id: id.into(), kind: kind.into(), layer: 0, properties };
    let mut library = crate::kerml::load_kernel_baseline().unwrap();
    let start = library.elements.len();
    library.elements.extend([
        element("ImportSource", "Package", BTreeMap::from([("owned_relationship".into(), json!(["import-member"]))])),
        element("import-widget", "Class", BTreeMap::from([
            ("declared_name".into(), json!("Widget")), ("declared_short_name".into(), json!("W")),
            ("owning_relationship".into(), json!("import-member")),
        ])),
        element("import-member", "OwningMembership", BTreeMap::from([
            ("owning_related_element".into(), json!("ImportSource")), ("owned_related_element".into(), json!(["import-widget"])),
        ])),
        element("ImportFacade", "Package", BTreeMap::from([("owned_relationship".into(), json!(["namespace-import"]))])),
        element("namespace-import", "NamespaceImport", BTreeMap::from([
            ("owning_related_element".into(), json!("ImportFacade")), ("imported_namespace".into(), json!("ImportSource")),
            ("visibility".into(), json!("public")),
        ])),
    ]);
    let compile = |library: &mercurio_foundation::KirDocument, name: &str| {
        let module = crate::kerml::parse_kerml(&format!("package ImportedTypes {{ feature x : ImportFacade::{name}; }}")).unwrap();
        crate::kerml::compile_kerml_module_strict_with_context(
            &module, "imported-types.kerml", std::slice::from_ref(&module), library)
    };
    for name in ["Widget", "W"] {
        let document = compile(&library, name).unwrap();
        let feature = document.elements.iter().find(|e| e.id == "feature.ImportedTypes.x").unwrap();
        assert_eq!(feature.properties.get("type"), Some(&json!("import-widget")));
    }
    library.elements[start + 4].properties.insert("visibility".into(), json!("private"));
    assert!(compile(&library, "Widget").is_err());
    library.elements[start + 4].properties.insert("visibility".into(), json!("public"));
    library.elements[start + 2].properties.insert("visibility".into(), json!("private"));
    assert!(compile(&library, "Widget").is_err());
    library.elements[start + 4].properties.insert("is_import_all".into(), json!(true));
    assert!(compile(&library, "Widget").is_ok());
    library.elements[start + 4].kind = "MembershipImport".into();
    library.elements[start + 4].properties.remove("imported_namespace");
    library.elements[start + 4].properties.insert("imported_membership".into(), json!("import-member"));
    assert!(compile(&library, "W").is_ok());
    library.elements[start + 4].properties.insert("imported_membership".into(), json!("import-widget"));
    assert!(compile(&library, "Widget").is_err());
    library.elements[start + 4].properties.remove("imported_membership");
    assert!(format!("{:?}", compile(&library, "Widget").unwrap_err()).contains("resolved endpoint"));
    library.elements[start + 4].properties.insert("imported_membership".into(), json!("import-member"));
    // An owned alternate-name collision hides the entire imported membership.
    library.elements[start + 3].properties.insert("owned_relationship".into(), json!(["namespace-import", "local-import-member"]));
    library.elements.extend([
        element("local-import-member", "OwningMembership", BTreeMap::from([
            ("owning_related_element".into(), json!("ImportFacade")), ("owned_related_element".into(), json!(["local-import-widget"])),
        ])),
        element("local-import-widget", "Class", BTreeMap::from([
            ("owning_relationship".into(), json!("local-import-member")), ("declared_name".into(), json!("Local")),
            ("declared_short_name".into(), json!("W")),
        ])),
    ]);
    assert!(compile(&library, "Widget").is_err());
    assert!(compile(&library, "Local").is_ok());
    library.elements.truncate(start + 5);
    library.elements[start + 3].properties.insert("owned_relationship".into(), json!(["namespace-import"]));
    // A second public import re-exports through the same graph service.
    library.elements.push(element("ImportRelay", "Package", BTreeMap::from([
        ("owned_relationship".into(), json!(["relay-import"])),
    ])));
    library.elements.push(element("relay-import", "NamespaceImport", BTreeMap::from([
        ("owning_related_element".into(), json!("ImportRelay")), ("imported_namespace".into(), json!("ImportSource")),
        ("is_import_all".into(), json!(true)), ("visibility".into(), json!("public")),
    ])));
    library.elements[start + 4].kind = "NamespaceImport".into();
    library.elements[start + 4].properties.remove("imported_membership");
    library.elements[start + 4].properties.insert("imported_namespace".into(), json!("ImportRelay"));
    assert!(compile(&library, "Widget").is_ok());
    library.elements.reverse();
    assert!(compile(&library, "Widget").is_ok());
}

#[test]
fn release_2026_08_recursive_type_imports_derive_inherited_library_members() {
    use serde_json::{json, Value};
    let oracle: Value = serde_json::from_str(include_str!(
        "../../resources/metamodels/sysml-2.0-pilot-2026-08/union-pilot-controls.json"
    )).unwrap();
    let report = crate::abstract_syntax_json::import_sysml_api_elements(
        oracle["canonical_controls"][12]["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
    assert!(!report.has_errors());
    let mut library = crate::kerml::load_kernel_baseline().unwrap();
    library.elements.extend(report.document.elements);
    library.elements.iter_mut().find(|e| e.id == "import").unwrap().properties.insert("visibility".into(), json!("public"));
    let compile = |library: &mercurio_foundation::KirDocument, name: &str| {
        let module = crate::kerml::parse_kerml(&format!("package RecursiveImportTypes {{ feature x : destination::{name}; }}")).unwrap();
        crate::kerml::compile_kerml_module_strict_with_context(
            &module, "recursive-import-types.kerml", std::slice::from_ref(&module), library)
    };
    for name in ["Z", "A"] {
        let document = compile(&library, name).unwrap();
        let feature = document.elements.iter().find(|e| e.id == "feature.RecursiveImportTypes.x").unwrap();
        assert_eq!(feature.properties.get("type"), Some(&json!("target")));
    }
    library.elements.iter_mut().find(|e| e.id == "import").unwrap().properties.insert("is_import_all".into(), json!(false));
    assert!(compile(&library, "Z").is_ok());
    assert!(compile(&library, "A").is_err());
    library.elements.iter_mut().find(|e| e.id == "specialization").unwrap().properties.insert("is_implied".into(), json!(true));
    assert!(compile(&library, "Z").is_err());
    library.elements.iter_mut().find(|e| e.id == "specialization").unwrap().properties.insert("is_implied".into(), json!(false));
    library.elements.reverse();
    assert!(compile(&library, "Z").is_ok());
}

#[test]
fn release_2026_08_materialized_inheritance_filters_library_feature_names() {
    use serde_json::{json, Value};
    let oracle: Value = serde_json::from_str(include_str!(
        "../../resources/metamodels/sysml-2.0-pilot-2026-08/union-pilot-controls.json"
    )).unwrap();
    let compile = |library: &mercurio_foundation::KirDocument, name: &str| {
        let module = crate::kerml::parse_kerml(&format!("package MaterializedImport {{ feature x : destination::{name}; }}")).unwrap();
        crate::kerml::compile_kerml_module_strict_with_context(
            &module, "materialized-import.kerml", std::slice::from_ref(&module), library)
    };
    for (case, visible, hidden, expected) in [(19, "Z", "Missing", "target"), (21, "Z", "A", "fz"), (22, "A", "Z", "fa"), (23, "Own", "Z", "own"), (27, "Z", "Own", "fz"), (28, "A", "Own", "own")] {
        let control = oracle["canonical_controls"].as_array().unwrap().iter().find(|c| c["case"] == case).unwrap();
        let report = crate::abstract_syntax_json::import_sysml_api_elements(control["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
        assert!(!report.has_errors());
        let mut library = crate::kerml::load_kernel_baseline().unwrap();
        library.elements.extend(report.document.elements);
        library.elements.iter_mut().find(|e| e.id == "import").unwrap().properties.insert("visibility".into(), json!("public"));
        let document = compile(&library, visible).unwrap();
        let feature = document.elements.iter().find(|e| e.id == "feature.MaterializedImport.x").unwrap();
        assert_eq!(feature.properties.get("type"), Some(&json!(expected)), "case {case}");
        assert!(compile(&library, hidden).is_err(), "case {case}: {hidden}");
        library.elements.reverse();
        assert!(compile(&library, visible).is_ok());
        library.elements.iter_mut().find(|e| e.id == "base").unwrap().properties.insert("is_implied_included".into(), json!(false));
        assert!(compile(&library, visible).is_err());
    }
}

#[test]
fn release_2026_08_feature_chaining_drives_native_library_scope() {
    use serde_json::{json, Value};
    let oracle: Value = serde_json::from_str(include_str!(
        "../../resources/metamodels/sysml-2.0-pilot-2026-08/union-pilot-controls.json"
    )).unwrap();
    let compile = |library: &mercurio_foundation::KirDocument, name: &str| {
        let module = crate::kerml::parse_kerml(&format!("package ChainImport {{ feature x : destination::{name}; }}")).unwrap();
        crate::kerml::compile_kerml_module_strict_with_context(
            &module, "chain-import.kerml", std::slice::from_ref(&module), library)
    };
    for control in oracle["chaining_controls"].as_array().unwrap() {
        let report = crate::abstract_syntax_json::import_sysml_api_elements(control["graph"].as_array().unwrap().clone(), Default::default()).unwrap();
        assert!(!report.has_errors());
        let mut library = crate::kerml::load_kernel_baseline().unwrap();
        library.elements.extend(report.document.elements);
        library.elements.iter_mut().find(|e| e.id == "import").unwrap().properties.insert("visibility".into(), json!("public"));
        for name in ["First", "Last", "Explicit", "Missing"] {
            let expected = control["membership"].as_array().unwrap().iter().any(|v| v == name);
            let result = compile(&library, name);
            assert_eq!(result.is_ok(), expected, "case {}, name {name}: {result:?}", control["case"]);
            if expected {
                let document = result.unwrap();
                let feature = document.elements.iter().find(|e| e.id == "feature.ChainImport.x").unwrap();
                assert_eq!(feature.properties.get("type"), Some(&json!("target")));
            }
        }
        library.elements.reverse();
        assert_eq!(compile(&library, "Last").is_ok(), control["membership"].as_array().unwrap().iter().any(|v| v == "Last"));
    }
}
