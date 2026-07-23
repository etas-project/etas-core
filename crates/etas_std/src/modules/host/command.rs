use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "host", "command"], "Host command support types.");
    for name in ["Command", "CommandResult", "CommandHandle"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Host command support type.",
        );
    }

    builder.symbol_with_intrinsic(
        module,
        "run",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_actions(
            "run",
            &["Command", "SandboxProfile"],
            "CommandResult",
            &[],
            &["Command.run[_]"],
        )),
        "Run a command through the checked command host boundary.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::COMMAND_RUN),
            qualified_path: vec!["std".into(), "host".into(), "command".into(), "run".into()],
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
