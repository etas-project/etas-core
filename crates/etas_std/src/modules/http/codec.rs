use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRecordField, StdRegistryBuilder, StdSymbolKind, StdType, TypeDecl,
    TypeDeclKind, ValueDecl, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "http", "codec"],
        "Deterministic HTTP message codec helpers.",
    );
    for (name, representation) in [
        (
            "HttpHeader",
            Some(record(&[("name", "string"), ("value", "string")])),
        ),
        (
            "HttpWireRequest",
            Some(record(&[
                ("method", "string"),
                ("target", "string"),
                ("version", "string"),
                ("headers", "List[HttpHeader]"),
                ("body", "bytes"),
            ])),
        ),
        (
            "HttpWireResponseHead",
            Some(record(&[
                ("version", "string"),
                ("status", "i32"),
                ("reason", "string"),
                ("headers", "List[HttpHeader]"),
            ])),
        ),
        (
            "HttpWireResponse",
            Some(record(&[
                ("head", "HttpWireResponseHead"),
                ("body", "bytes"),
            ])),
        ),
        ("HttpCodecError", None),
    ] {
        let mut decl = TypeDecl::generic(name, &[], TypeDeclKind::Support);
        if let Some(representation) = representation {
            decl = decl.with_representation(representation);
        }
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(decl),
            "HTTP codec support type.",
        );
    }
    builder.symbol(
        module,
        "MalformedMessage",
        StdSymbolKind::Value,
        StdDecl::Value(ValueDecl::new("MalformedMessage", "HttpCodecError")),
        "HTTP wire codec parse/validation error variant.",
    );
    pure_flow(
        builder,
        module,
        "encode_request",
        &["HttpWireRequest"],
        "Result[bytes, HttpCodecError]",
        intrinsic::pure::HTTP_ENCODE_REQUEST,
        "Encode a structured HTTP request head/body into bytes.",
    );
    pure_flow(
        builder,
        module,
        "decode_response_head",
        &["bytes"],
        "Result[HttpWireResponseHead, HttpCodecError]",
        intrinsic::pure::HTTP_DECODE_RESPONSE_HEAD,
        "Decode an HTTP response head from bytes.",
    );
    pure_flow(
        builder,
        module,
        "decode_response",
        &["bytes"],
        "Result[HttpWireResponse, HttpCodecError]",
        intrinsic::pure::HTTP_DECODE_RESPONSE,
        "Decode an HTTP response head and body from bytes.",
    );
}

fn record(fields: &[(&str, &str)]) -> StdType {
    StdType::Record(
        fields
            .iter()
            .map(|(name, ty)| StdRecordField::new(name, StdType::parse(ty)))
            .collect(),
    )
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
            qualified_path: vec!["std".into(), "http".into(), "codec".into(), name.into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
