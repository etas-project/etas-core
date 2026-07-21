use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "runtime", "budget"],
        "Budget support declarations.",
    );
    builder.symbol(
        module,
        "Money",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Money", &[], TypeDeclKind::Support)),
        "Money support type for budgets and limits.",
    );
    builder.symbol_with_intrinsic(
        module,
        "usd",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure("usd", &["f64"], "Money")),
        "Construct a USD money descriptor.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::USD),
            qualified_path: vec![
                "std".into(),
                "runtime".into(),
                "budget".into(),
                "usd".into(),
            ],
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
