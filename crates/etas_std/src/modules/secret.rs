use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdEffectRef, StdGenericParam, StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, StdType,
    TypeDecl, TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "secret"], "Secret substrate declarations.");
    for name in ["SecretKey", "SecretValue", "SecretError"] {
        let params = if matches!(name, "SecretKey" | "SecretValue") {
            &["K"][..]
        } else {
            &[]
        };
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, params, TypeDeclKind::Support)),
            "Secret substrate support type.",
        );
        builder.prelude(name, symbol);
    }
    builder.symbol_with_intrinsic(
        module,
        "read",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            "read",
            &[StdGenericParam::new("K")],
            &["SecretKey[K]"],
            "std.secret.SecretValue[K]",
            &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("SecretError"),
            )],
            &[StdEffectRef::typed(
                &["Secret", "read"],
                StdType::Var("K".to_owned()),
            )],
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
