use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicLatentEffect, IntrinsicPurity,
    LoweringHint, StdDecl, StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "char"], "Unicode scalar queries and conversion.");
    for (name, params, output, id, summary) in [
        (
            "is_whitespace",
            &["char"][..],
            "bool",
            intrinsic::pure::CHAR_IS_WHITESPACE,
            "Test Unicode White_Space.",
        ),
        (
            "is_alphabetic",
            &["char"][..],
            "bool",
            intrinsic::pure::CHAR_IS_ALPHABETIC,
            "Test Unicode Alphabetic.",
        ),
        (
            "is_alphanumeric",
            &["char"][..],
            "bool",
            intrinsic::pure::CHAR_IS_ALPHANUMERIC,
            "Test Unicode alphabetic or numeric.",
        ),
        (
            "is_control",
            &["char"][..],
            "bool",
            intrinsic::pure::CHAR_IS_CONTROL,
            "Test Unicode control category.",
        ),
        (
            "is_ascii_digit",
            &["char"][..],
            "bool",
            intrinsic::pure::CHAR_IS_ASCII_DIGIT,
            "Test ASCII decimal digit.",
        ),
        (
            "to_u32",
            &["char"][..],
            "u32",
            intrinsic::pure::CHAR_TO_U32,
            "Return the Unicode scalar value.",
        ),
        (
            "from_u32",
            &["u32"][..],
            "Option[char]",
            intrinsic::pure::CHAR_FROM_U32,
            "Construct a valid Unicode scalar.",
        ),
    ] {
        builder.symbol_with_intrinsic(
            module,
            name,
            StdSymbolKind::Flow,
            StdDecl::Flow(FlowDecl::pure(name, params, output)),
            summary,
            Some(IntrinsicDescriptor {
                id: StdIntrinsicId(id),
                qualified_path: vec!["std".into(), "char".into(), name.into()],
                purity: IntrinsicPurity::Pure,
                dispatch: IntrinsicDispatch::PureKernel,
                lowering: LoweringHint::PureBuiltin,
                latent_effect: IntrinsicLatentEffect::None,
                memory_access: crate::IntrinsicMemoryAccess::None,
                runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
            }),
        );
    }
}
