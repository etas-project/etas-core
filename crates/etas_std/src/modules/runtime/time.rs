use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdEffectRef, StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind,
    intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "runtime", "time"], "Time support declarations.");
    for name in ["Time", "Duration"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Runtime time support type.",
        );
    }
    builder.symbol_with_intrinsic(
        module,
        "now",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::effectful(
            "now",
            &[],
            "Time",
            &[StdEffectRef::new(&["Time"])],
        )),
        "Read the runtime clock through the future runtime boundary.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::TIME_NOW),
            qualified_path: vec!["std".into(), "runtime".into(), "time".into(), "now".into()],
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
