use crate::{
    FlowDecl, StdDecl, StdGenericParam, StdModuleId, StdRegistryBuilder, StdSymbolKind, TypeDecl,
    TypeDeclKind,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    let stop = builder.symbol(
        module,
        "StopReason",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("StopReason", &[], TypeDeclKind::Enum)),
        "Model turn outcome, not a handler for runtime faults or ordinary errors.",
    );
    for (name, params) in [
        ("Truncated", &[][..]),
        ("Refused", &["string"][..]),
        ("InvalidOutput", &["std.agent.StepDiagnostic"][..]),
        ("InvalidToolCall", &["std.agent.StepDiagnostic"][..]),
    ] {
        builder
            .enum_constructor(
                stop,
                FlowDecl::pure(name, params, "std.agent.StopReason"),
                "Model stop constructor.",
            )
            .expect("StopReason owner is registered");
    }
    let decision = builder.symbol(
        module,
        "StepDecision",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic(
            "StepDecision",
            &["C", "O"],
            TypeDeclKind::Enum,
        )),
        "One generation returns a final answer, tool requests, or a stopped turn.",
    );
    for (name, payload) in [
        ("Final", "O"),
        ("CallTools", "Array[std.agent.ToolCall[C]]"),
        ("Stopped", "std.agent.StopReason"),
    ] {
        builder
            .enum_constructor(
                decision,
                FlowDecl::with_type_params_actions(
                    name,
                    &[StdGenericParam::new("C"), StdGenericParam::new("O")],
                    &[payload],
                    "std.agent.StepDecision[C, O]",
                    &[],
                    &[],
                ),
                "Transparent step decision constructor.",
            )
            .expect("StepDecision owner is registered");
    }
    super::record(
        builder,
        module,
        "StepResult",
        &["C", "O"],
        vec![
            super::field("turn", "std.agent.AssistantTurn"),
            super::field("decision", "std.agent.StepDecision[C, O]"),
            super::field("usage", "Option[std.agent.Usage]"),
        ],
    );
}
