use crate::{
    IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl, StdIntrinsicId,
    StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "security", "trust"],
        "Trust, provenance, and secrecy wrappers.",
    );
    for name in ["Trusted", "Untrusted", "Secret", "Public", "Sanitized"] {
        let symbol = builder.symbol_with_intrinsic(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &["T"], TypeDeclKind::Wrapper)),
            "Trust wrapper declaration.",
            Some(IntrinsicDescriptor {
                id: StdIntrinsicId(trust_intrinsic_id(name)),
                qualified_path: vec![
                    "std".into(),
                    "security".into(),
                    "trust".into(),
                    name.to_owned(),
                ],
                purity: IntrinsicPurity::Pure,
                dispatch: IntrinsicDispatch::Runtime,
                lowering: LoweringHint::RuntimeCall,
                latent_effect: crate::IntrinsicLatentEffect::TransparentFirstArg,
                memory_access: crate::IntrinsicMemoryAccess::None,
                runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
            }),
        );
        builder.prelude(name, symbol);
    }
}

fn trust_intrinsic_id(name: &str) -> u32 {
    match name {
        "Trusted" => intrinsic::pure::TRUST_TRUSTED,
        "Untrusted" => intrinsic::pure::TRUST_UNTRUSTED,
        "Secret" => intrinsic::pure::TRUST_SECRET,
        "Public" => intrinsic::pure::TRUST_PUBLIC,
        "Sanitized" => intrinsic::pure::TRUST_SANITIZED,
        _ => unreachable!("unknown standard trust wrapper"),
    }
}
