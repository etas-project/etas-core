use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, StdDecl, StdEffectRef,
    StdGenericParam, StdModuleId, StdRegistryBuilder, StdStaticArg, StdSymbolKind, StdType,
    intrinsic,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    let mut descriptor = IntrinsicDescriptor::runtime(
        intrinsic::runtime::AGENT_STEP,
        &["std", "agent", "step"],
        IntrinsicPurity::Runtime,
    );
    descriptor.dispatch = IntrinsicDispatch::Host;
    descriptor.purity = IntrinsicPurity::Host;
    descriptor.lowering = crate::LoweringHint::default_action(150, &[0, 1]);
    builder.symbol_with_intrinsic(module, "step", StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions("step",
            &[StdGenericParam::new("C"), StdGenericParam::new("O")],
            &["std.agent.session.Conversation", "Schema[O]"], "std.agent.StepResult[C, O]", &[],
            &[StdEffectRef::with_args(&["Agentic", "step"], vec![
                StdStaticArg::Type(StdType::Var("C".into())), StdStaticArg::Type(StdType::Var("O".into())),
            ])])),
        "Perform exactly one model generation in the active checked agent context; never invoke tools or repair output.",
        Some(descriptor));
}
