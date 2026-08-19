use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdEffectRef, StdGenericParam, StdIntrinsicId, StdRecordField, StdRegistryBuilder, StdSpecRef,
    StdSymbolKind, StdType, TypeDecl, TypeDeclKind, ValueDecl, intrinsic,
};

use crate::modules::registration::{IntrinsicFlowRegistration, register_intrinsic_flow};

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
    builder.symbol_with_intrinsic(
        module,
        "Host",
        StdSymbolKind::Constructor,
        StdDecl::Flow(FlowDecl::pure("Host", &["string"], "StreamError")),
        "Standard stream host-failure error variant.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::pure::STREAM_ERROR_HOST),
            qualified_path: vec!["std".into(), "stream".into(), "Host".into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "stream"],
        IntrinsicFlowRegistration {
            name: "read",
            type_params: &[byte_stream_param()],
            params: &["S", "usize", "Option[std.stream.Timeout]"],
            output: "std.stream.StreamRead",
            public_effects: &[stream_error_effect()],
            requested_actions: &[StdEffectRef::wildcard(&["Stream", "read"], 1)],
            intrinsic_id: intrinsic::runtime::STREAM_READ,
            summary: "Read at most the requested byte count from a host-mediated stream.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "stream"],
        IntrinsicFlowRegistration {
            name: "read_until_limit",
            type_params: &[byte_stream_param()],
            params: &["S", "std.stream.ByteLimit", "Option[std.stream.Timeout]"],
            output: "bytes",
            public_effects: &[stream_error_effect()],
            requested_actions: &[StdEffectRef::wildcard(&["Stream", "read"], 1)],
            intrinsic_id: intrinsic::runtime::STREAM_READ_UNTIL_LIMIT,
            summary: "Read from a host-mediated stream until EOF or the requested byte limit.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "stream"],
        IntrinsicFlowRegistration {
            name: "write_all",
            type_params: &[byte_stream_param()],
            params: &["S", "bytes"],
            output: "unit",
            public_effects: &[stream_error_effect()],
            requested_actions: &[StdEffectRef::wildcard(&["Stream", "write"], 1)],
            intrinsic_id: intrinsic::runtime::STREAM_WRITE_ALL,
            summary: "Write all bytes to a host-mediated stream.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "stream"],
        IntrinsicFlowRegistration {
            name: "flush",
            type_params: &[byte_stream_param()],
            params: &["S"],
            output: "unit",
            public_effects: &[stream_error_effect()],
            requested_actions: &[StdEffectRef::wildcard(&["Stream", "flush"], 1)],
            intrinsic_id: intrinsic::runtime::STREAM_FLUSH,
            summary: "Flush a host-mediated stream.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "stream"],
        IntrinsicFlowRegistration {
            name: "close",
            type_params: &[byte_stream_param()],
            params: &["S"],
            output: "unit",
            public_effects: &[stream_error_effect()],
            requested_actions: &[StdEffectRef::wildcard(&["Stream", "close"], 1)],
            intrinsic_id: intrinsic::runtime::STREAM_CLOSE,
            summary: "Close a host-mediated stream.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
}

fn byte_stream_param() -> StdGenericParam {
    StdGenericParam::bounded("S", &[StdSpecRef::new(&["std", "stream", "ByteStream"])])
}

fn stream_error_effect() -> StdEffectRef {
    StdEffectRef::typed(&["Error"], StdType::parse("std.stream.StreamError"))
}

fn record(fields: &[(&str, &str)]) -> StdType {
    StdType::Record(
        fields
            .iter()
            .map(|(name, ty)| StdRecordField::new(name, StdType::parse(ty)))
            .collect(),
    )
}
