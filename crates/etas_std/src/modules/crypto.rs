use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, intrinsic,
};

use crate::modules::registration::{IntrinsicFlowRegistration, register_intrinsic_flow};

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
    register_intrinsic_flow(
        builder,
        module,
        &["std", "crypto"],
        IntrinsicFlowRegistration {
            name: "hmac_sha256",
            type_params: &[],
            params: &["SecretValue", "bytes"],
            output: "Digest",
            public_effects: &["Error[CryptoError]"],
            requested_actions: &["Secret.use[key]"],
            intrinsic_id: intrinsic::runtime::SECRET_HMAC_SHA256,
            summary: "Compute HMAC-SHA256 using a host-mediated opaque secret.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
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
