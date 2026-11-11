use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicPurity, StdDecl, StdGenericParam, StdRegistryBuilder,
    StdSymbolKind, intrinsic,
};

pub(super) fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "agent", "conversation"],
        "Pure in-memory conversation protocol assembly.",
    );
    for (name, generics, params, output, id) in [
        (
            "from_prompt",
            &[][..],
            &["Prompt"][..],
            "std.agent.session.Conversation",
            intrinsic::runtime::CONVERSATION_FROM_PROMPT,
        ),
        (
            "append_step",
            &["C", "O"][..],
            &[
                "std.agent.session.Conversation",
                "std.agent.StepResult[C, O]",
            ][..],
            "Result[std.agent.session.Conversation, std.agent.StepDiagnostic]",
            intrinsic::runtime::CONVERSATION_APPEND_STEP,
        ),
        (
            "append_results",
            &["C"][..],
            &[
                "std.agent.session.Conversation",
                "Array[std.agent.ToolResult[C]]",
            ][..],
            "Result[std.agent.session.Conversation, std.agent.StepDiagnostic]",
            intrinsic::runtime::CONVERSATION_APPEND_RESULTS,
        ),
    ] {
        let generics = generics
            .iter()
            .map(|name| StdGenericParam::new(name))
            .collect::<Vec<_>>();
        builder.symbol_with_intrinsic(module, name, StdSymbolKind::Flow,
            StdDecl::Flow(FlowDecl::with_type_params_actions(name, &generics, params, output, &[], &[])),
            "Validate and assemble protocol data without executing tools, persisting a session, or granting authority.",
            Some(IntrinsicDescriptor::runtime(id, &["std", "agent", "conversation", name], IntrinsicPurity::Pure)));
    }
}
