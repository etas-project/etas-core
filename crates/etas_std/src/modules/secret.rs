use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdEffectRef, StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, StdType, TypeDecl,
    TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "secret"], "Secret substrate declarations.");
    for name in ["SecretKey", "SecretValue", "SecretError"] {
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Secret substrate support type.",
        );
        builder.prelude(name, symbol);
    }
    builder.symbol_with_intrinsic(
        module,
        "read",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_actions(
            "read",
            &["SecretKey"],
            "SecretValue",
            &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("SecretError"),
            )],
            &[StdEffectRef::wildcard(&["Secret", "read"], 1)],
        )),
        "Read a redaction-safe secret value from the host secret provider.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::SECRET_READ),
            qualified_path: vec!["std".into(), "secret".into(), "read".into()],
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
