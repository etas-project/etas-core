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
        (
            "HttpDecodeFailure",
            Some(record(&[
                ("kind", "HttpCodecFailureKind"),
                ("offset", "usize"),
            ])),
        ),
        ("HttpCodecFailureKind", None),
        ("HttpCodecError", None),
    ] {
        let kind = if matches!(name, "HttpCodecError" | "HttpCodecFailureKind") {
            TypeDeclKind::Enum
        } else {
            TypeDeclKind::Support
        };
        let mut decl = TypeDecl::generic(name, &[], kind);
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
        "HttpDecodeStep",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic(
            "HttpDecodeStep",
            &["T"],
            TypeDeclKind::Enum,
        )),
        "Closed incremental HTTP decode result.",
    );
    builder.symbol(
        module,
        "MalformedMessage",
        StdSymbolKind::Value,
        StdDecl::Value(ValueDecl::new("MalformedMessage", "HttpCodecError")),
        "HTTP wire codec parse/validation error variant.",
    );
    decode_step_constructor(builder, module, "NeedMore", &[]);
    decode_step_constructor(builder, module, "Complete", &["T", "usize"]);
    decode_step_constructor(builder, module, "Malformed", &["HttpDecodeFailure"]);
    for name in [
        "UnexpectedEof",
        "InvalidLineEnding",
        "InvalidStatusLine",
        "UnsupportedHttpVersion",
        "InvalidStatusCode",
        "InvalidHeader",
        "InvalidContentLength",
        "ConflictingContentLength",
        "ConflictingMessageFraming",
        "UnsupportedTransferEncoding",
        "ForbiddenResponseBody",
        "InvalidChunkSize",
        "InvalidChunkTerminator",
        "InvalidTrailer",
    ] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Value,
            StdDecl::Value(ValueDecl::new(name, "HttpCodecFailureKind")),
            "Incremental HTTP codec failure kind.",
        );
    }
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
        "decode_response_head_incremental",
        &["bytes", "bool"],
        "HttpDecodeStep[HttpWireResponseHead]",
        intrinsic::pure::HTTP_DECODE_RESPONSE_HEAD_INCREMENTAL,
        "Incrementally decode an HTTP response head without conflating incomplete input and malformed input.",
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
    pure_flow(
        builder,
        module,
        "decode_response_incremental",
        &["bytes", "bool"],
        "HttpDecodeStep[HttpWireResponse]",
        intrinsic::pure::HTTP_DECODE_RESPONSE_INCREMENTAL,
        "Incrementally decode a framed HTTP response and report consumed input bytes.",
    );
}

fn decode_step_constructor(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    params: &[&str],
) {
    builder.symbol(
        module,
        name,
        StdSymbolKind::Constructor,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            name,
            &[crate::StdGenericParam::new("T")],
            params,
            "HttpDecodeStep[T]",
            &[],
            &[],
        )),
        "Incremental HTTP decode result constructor.",
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
