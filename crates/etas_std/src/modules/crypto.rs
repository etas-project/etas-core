use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "crypto"],
        "Deterministic and secret-backed cryptographic helpers.",
    );
    for name in ["Digest", "SecretValue", "CryptoError"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Deterministic crypto support type.",
        );
    }
    host_flow(
        builder,
        module,
        "hmac_sha256",
        &["SecretValue", "bytes"],
        "Digest",
        &["Error[CryptoError]"],
        &["Secret.use[key]"],
        intrinsic::runtime::SECRET_HMAC_SHA256,
        "Compute HMAC-SHA256 using a host-mediated opaque secret.",
    );
    pure_flow(
        builder,
        module,
        "sha256",
        &["bytes"],
        "Digest",
        intrinsic::pure::CRYPTO_SHA256,
        "Compute SHA-256 over public bytes.",
    );
    pure_flow(
        builder,
        module,
        "constant_time_eq",
        &["bytes", "bytes"],
        "bool",
        intrinsic::pure::CRYPTO_CONSTANT_TIME_EQ,
        "Compare byte arrays without data-dependent early exit.",
    );
}

fn host_flow(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    params: &[&str],
    output: &str,
    public_effects: &[&str],
    requested_actions: &[&str],
    id: u32,
    summary: &str,
) {
    builder.symbol_with_intrinsic(
        module,
        name,
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_actions(
            name,
            params,
            output,
            public_effects,
            requested_actions,
        )),
        summary,
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(id),
            qualified_path: vec!["std".into(), "crypto".into(), name.into()],
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}

fn pure_flow(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    params: &[&str],
    output: &str,
    id: u32,
    summary: &str,
) {
    builder.symbol_with_intrinsic(
        module,
        name,
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure(name, params, output)),
        summary,
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(id),
            qualified_path: vec!["std".into(), "crypto".into(), name.into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
