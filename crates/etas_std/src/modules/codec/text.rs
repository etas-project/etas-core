use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, ValueDecl,
    intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "codec", "text"],
        "Deterministic text codec helpers.",
    );
    for name in ["MalformedInput", "TextCodecError"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Text codec support type.",
        );
    }
    for name in ["Strict", "Replace"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Value,
            StdDecl::Value(ValueDecl::new(name, "MalformedInput")),
            "Malformed-input decoding mode.",
        );
    }
    builder.symbol(
        module,
        "InvalidUtf8",
        StdSymbolKind::Value,
        StdDecl::Value(ValueDecl::new("InvalidUtf8", "TextCodecError")),
        "UTF-8 decoder error variant.",
    );
    pure_flow(
        builder,
        module,
        "utf8_decode",
        &["bytes", "MalformedInput"],
        "Result[string, TextCodecError]",
        intrinsic::pure::TEXT_UTF8_DECODE,
        "Decode UTF-8 bytes with explicit malformed-input behavior.",
    );
    pure_flow(
        builder,
        module,
        "utf8_encode",
        &["string"],
        "bytes",
        intrinsic::pure::TEXT_UTF8_ENCODE,
        "Encode text as UTF-8 bytes.",
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
            qualified_path: vec!["std".into(), "codec".into(), "text".into(), name.into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
