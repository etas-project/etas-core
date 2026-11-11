use etas_std::{
    CompilerActionRole, IntrinsicDispatch, StdDecl, StdGenericParam, StdSymbolKind, StdType,
    TypeDeclKind, intrinsic, standard_registry,
};

#[test]
fn agent_protocol_exports_exact_record_fields_and_real_variants() {
    let registry = standard_registry();
    for (name, generics, fields) in [
        (
            "StepResult",
            &["C", "O"][..],
            &["turn", "decision", "usage"][..],
        ),
        (
            "AssistantTurn",
            &[][..],
            &["request_id", "content", "provider_data"][..],
        ),
        ("ToolRef", &[][..], &["id", "revision"][..]),
        ("ToolCall", &["C"][..], &["id", "tool", "args"][..]),
        ("ToolResult", &["C"][..], &["call_id", "result"][..]),
        ("StepDiagnostic", &[][..], &["code", "message", "path"][..]),
        (
            "Usage",
            &[][..],
            &["input_tokens", "output_tokens", "cost"][..],
        ),
    ] {
        let symbol = registry.lookup_qualified(&["std", "agent", name]).unwrap();
        let StdDecl::Type(decl) = &symbol.decl else {
            panic!("protocol record");
        };
        assert_eq!(decl.kind, TypeDeclKind::Struct, "{name}");
        assert_eq!(
            decl.params,
            generics
                .iter()
                .map(|name| StdGenericParam::new(name))
                .collect::<Vec<_>>()
        );
        let Some(StdType::Record(actual)) = &decl.representation else {
            panic!("{name} representation");
        };
        assert_eq!(
            actual
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            fields,
            "{name}"
        );
    }
    for (name, variants) in [
        ("MessageContent", &["Text", "Value"][..]),
        ("StepDecision", &["Final", "CallTools", "Stopped"][..]),
        (
            "StopReason",
            &["Truncated", "Refused", "InvalidOutput", "InvalidToolCall"][..],
        ),
    ] {
        let owner = registry.lookup_qualified(&["std", "agent", name]).unwrap();
        assert!(matches!(&owner.decl, StdDecl::Type(decl) if decl.kind == TypeDeclKind::Enum));
        assert_eq!(
            registry
                .enum_constructors(owner.id)
                .map(|value| value.name.as_str())
                .collect::<Vec<_>>(),
            variants
        );
    }
}

#[test]
fn agent_step_and_assembly_have_distinct_checked_api_descriptors() {
    let registry = standard_registry();
    for (path, generics, params, output, id) in [
        (
            &["std", "agent", "step"][..],
            &["C", "O"][..],
            &["std.agent.session.Conversation", "Schema[O]"][..],
            "std.agent.StepResult[C, O]",
            intrinsic::runtime::AGENT_STEP,
        ),
        (
            &["std", "tool", "reference"][..],
            &["T"][..],
            &["T"][..],
            "std.agent.ToolRef",
            intrinsic::runtime::TOOL_REFERENCE,
        ),
        (
            &["std", "tool", "invoke"][..],
            &["C"][..],
            &["std.agent.ToolCall[C]"][..],
            "std.agent.ToolResult[C]",
            intrinsic::runtime::TOOL_INVOKE,
        ),
        (
            &["std", "agent", "conversation", "from_prompt"][..],
            &[][..],
            &["Prompt"][..],
            "std.agent.session.Conversation",
            intrinsic::runtime::CONVERSATION_FROM_PROMPT,
        ),
        (
            &["std", "agent", "conversation", "append_step"][..],
            &["C", "O"][..],
            &[
                "std.agent.session.Conversation",
                "std.agent.StepResult[C, O]",
            ][..],
            "Result[std.agent.session.Conversation, std.agent.StepDiagnostic]",
            intrinsic::runtime::CONVERSATION_APPEND_STEP,
        ),
        (
            &["std", "agent", "conversation", "append_results"][..],
            &["C"][..],
            &[
                "std.agent.session.Conversation",
                "Array[std.agent.ToolResult[C]]",
            ][..],
            "Result[std.agent.session.Conversation, std.agent.StepDiagnostic]",
            intrinsic::runtime::CONVERSATION_APPEND_RESULTS,
        ),
    ] {
        let symbol = registry.lookup_qualified(path).unwrap();
        assert_eq!(symbol.kind, StdSymbolKind::Flow);
        let StdDecl::Flow(flow) = &symbol.decl else {
            panic!("public flow");
        };
        assert_eq!(
            flow.type_params,
            generics
                .iter()
                .map(|name| StdGenericParam::new(name))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            flow.params,
            params
                .iter()
                .map(|ty| StdType::parse(ty))
                .collect::<Vec<_>>()
        );
        assert_eq!(flow.output, StdType::parse(output));
        let descriptor = symbol.intrinsic.as_ref().unwrap();
        assert_eq!(descriptor.id.0, id);
        if id == intrinsic::runtime::AGENT_STEP {
            assert_eq!(descriptor.dispatch, IntrinsicDispatch::Host);
            assert_eq!(
                descriptor.lowering,
                etas_std::LoweringHint::default_action(150, &[0, 1])
            );
            assert_eq!(flow.requested_actions.len(), 1);
            assert_eq!(flow.requested_actions[0].path, ["Agentic", "step"]);
        } else {
            assert_eq!(descriptor.dispatch, IntrinsicDispatch::Runtime);
        }
    }
    let step = registry.symbols().find(|symbol| matches!(&symbol.decl,
        StdDecl::EffectAction(action) if action.compiler_role == Some(CompilerActionRole::AgentStep))).unwrap();
    let StdDecl::EffectAction(action) = &step.decl else {
        panic!("step action");
    };
    assert_eq!(action.owner, "Agentic");
    assert_eq!(action.name, "step");
    assert_eq!(action.selector_param_names, ["C", "O"]);
}

#[test]
fn json_is_one_recursive_public_adt_with_collection_payloads() {
    let registry = standard_registry();
    let json = registry
        .lookup_qualified(&["std", "json", "JsonValue"])
        .unwrap();
    let StdDecl::Type(decl) = &json.decl else {
        panic!("JsonValue type");
    };
    assert_eq!(decl.kind, TypeDeclKind::Enum);
    assert_eq!(
        registry
            .enum_constructors(json.id)
            .map(|value| value.name.as_str())
            .collect::<Vec<_>>(),
        ["Null", "Bool", "Number", "String", "Array", "Object"]
    );
    for (name, payload) in [
        ("Array", "Array[std.json.JsonValue]"),
        ("Object", "Map[string, std.json.JsonValue]"),
    ] {
        let constructor = registry
            .lookup_qualified(&["std", "json", "JsonValue", name])
            .unwrap();
        let StdDecl::Flow(decl) = &constructor.decl else {
            panic!("JSON constructor");
        };
        assert_eq!(decl.params, [StdType::parse(payload)]);
    }
}
