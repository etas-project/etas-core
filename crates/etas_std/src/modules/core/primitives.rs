use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdImplFact, StdIntrinsicId, StdModuleId, StdRegistryBuilder, StdSpecRef, StdSymbolKind,
    StdType, TypeDecl, TypeDeclKind, intrinsic,
};

const PRIMITIVES: &[&str] = &[
    "bool", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
    "f32", "f64", "char", "string", "bytes", "unit", "never",
];

pub fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    for primitive in PRIMITIVES {
        let symbol = builder.symbol(
            module,
            primitive,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::primitive(primitive)),
            "Primitive Etas value type.",
        );
        builder.prelude(primitive, symbol);
    }

    let index = builder.symbol(
        module,
        "Index",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Index", &[], TypeDeclKind::Spec)),
        "Compiler-known spec for checked sequence and range indexing.",
    );
    builder.prelude("Index", index);
    for primitive in [
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
    ] {
        builder.spec_impl(StdImplFact::new(
            StdType::parse(primitive),
            StdSpecRef::new(&["std", "core", "Index"]),
        ));
    }

    let assert = builder.symbol_with_intrinsic(
        module,
        "assert",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure("assert", &["bool"], "unit")),
        "Assert a condition or fail deterministically.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::pure::ASSERT),
            qualified_path: vec!["std".into(), "core".into(), "assert".into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
    builder.prelude("assert", assert);

    let abort = builder.symbol_with_intrinsic(
        module,
        "abort",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure("abort", &["string"], "never")),
        "Abort execution with a deterministic message.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::pure::ABORT),
            qualified_path: vec!["std".into(), "core".into(), "abort".into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
    builder.prelude("abort", abort);
}
