use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRecordField, StdRegistryBuilder, StdSymbolKind, StdType, TypeDecl,
    TypeDeclKind, TypeParam, ValueDecl, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "stream"],
        "Bounded byte stream substrate declarations.",
    );
    for (name, representation) in [
        ("ByteStream", None),
        ("StreamRead", None),
        ("StreamError", None),
        ("ByteLimit", Some(record(&[("bytes", "i32")]))),
        ("Timeout", Some(record(&[("ms", "i32")]))),
    ] {
        let kind = if name == "ByteStream" {
            TypeDeclKind::Spec
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
            "Stream substrate support type.",
        );
    }
    for name in [
        "TimedOut",
        "Cancelled",
        "Closed",
        "Interrupted",
        "LimitExceeded",
    ] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Value,
            StdDecl::Value(ValueDecl::new(name, "StreamError")),
            "Standard stream error variant.",
        );
    }
    builder.symbol(
        module,
        "Host",
        StdSymbolKind::Constructor,
        StdDecl::Flow(FlowDecl::pure("Host", &["string"], "StreamError")),
        "Standard stream host-failure error variant.",
    );
    stream_flow(
        builder,
        module,
        "read",
        &["S", "usize", "Option[std.stream.Timeout]"],
        "std.stream.StreamRead",
        "Stream.read[stream]",
        intrinsic::runtime::STREAM_READ,
        "Read at most the requested byte count from a host-mediated stream.",
    );
    stream_flow(
        builder,
        module,
        "read_until_limit",
        &["S", "std.stream.ByteLimit", "Option[std.stream.Timeout]"],
        "bytes",
        "Stream.read[stream]",
        intrinsic::runtime::STREAM_READ,
        "Read from a host-mediated stream until EOF or the requested byte limit.",
    );
    stream_flow(
        builder,
        module,
        "write_all",
        &["S", "bytes"],
        "unit",
        "Stream.write[stream]",
        intrinsic::runtime::STREAM_WRITE_ALL,
        "Write all bytes to a host-mediated stream.",
    );
    stream_flow(
        builder,
        module,
        "flush",
        &["S"],
        "unit",
        "Stream.flush[stream]",
        intrinsic::runtime::STREAM_FLUSH,
        "Flush a host-mediated stream.",
    );
    stream_flow(
        builder,
        module,
        "close",
        &["S"],
        "unit",
        "Stream.close[stream]",
        intrinsic::runtime::STREAM_CLOSE,
        "Close a host-mediated stream.",
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

fn stream_flow(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    params: &[&str],
    output: &str,
    action: &str,
    id: u32,
    summary: &str,
) {
    builder.symbol_with_intrinsic(
        module,
        name,
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            name,
            &[TypeParam::bounded("S", &["std.stream.ByteStream"])],
            params,
            output,
            &["Error[std.stream.StreamError]"],
            &[action],
        )),
        summary,
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(id),
            qualified_path: vec!["std".into(), "stream".into(), name.into()],
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
