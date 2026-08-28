use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdEffectRef, StdGenericParam, StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl,
    TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "runtime", "approval"],
        "Approval effect and support declarations.",
    );
    for name in ["ApprovalRequest", "ApprovalDecision", "Risk"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Approval support type.",
        );
    }
    let approve = builder.symbol_with_intrinsic(
        module,
        "approve",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            "approve",
            &[StdGenericParam::new("T")],
            &["string", "T", "Risk"],
            "bool",
            &[StdEffectRef::new(&["Approval"])],
            &[],
        )),
        "Request human approval through the future runtime boundary.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::APPROVE),
            qualified_path: vec![
                "std".into(),
                "runtime".into(),
                "approval".into(),
                "approve".into(),
            ],
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::ApprovalBoundary,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
    builder.prelude("approve", approve);
}
