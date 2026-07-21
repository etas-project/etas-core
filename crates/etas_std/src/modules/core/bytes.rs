use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "bytes"], "Byte sequence helper declarations.");
    builder.symbol_with_intrinsic(
        module,
        "len",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure("len", &["bytes"], "usize")),
        "Return the byte sequence length.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::pure::BYTES_LEN),
            qualified_path: vec!["std".into(), "bytes".into(), "len".into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
