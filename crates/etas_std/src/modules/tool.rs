use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicPurity, StdDecl, StdGenericParam, StdRegistryBuilder,
    StdSymbolKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "tool"],
        "Checked native tool references and invocation.",
    );
    for (name, generics, params, output, id, purity) in [
        (
            "reference",
            &["T"][..],
            &["T"][..],
            "std.agent.ToolRef",
            intrinsic::runtime::TOOL_REFERENCE,
            IntrinsicPurity::Pure,
        ),
        (
            "invoke",
            &["C"][..],
            &["std.agent.ToolCall[C]"][..],
            "std.agent.ToolResult[C]",
            intrinsic::runtime::TOOL_INVOKE,
            IntrinsicPurity::Runtime,
        ),
    ] {
        let generics = generics
            .iter()
            .map(|name| StdGenericParam::new(name))
            .collect::<Vec<_>>();
        builder.symbol_with_intrinsic(module, name, StdSymbolKind::Flow,
            StdDecl::Flow(FlowDecl::with_type_params_actions(name, &generics, params, output, &[], &[])),
            if name == "reference" { "Describe a resolved native tool declaration without registration or authorization." }
            else { "Validate identity, revision, arguments and current authority, then execute one exposed tool; ordinary tool errors remain escaping." },
            Some(IntrinsicDescriptor::runtime(id, &["std", "tool", name], purity)));
    }
}
