use crate::{
    IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdEffectRef, StdGenericParam,
    StdRegistryBuilder, StdSpecRef, StdType, intrinsic,
};

use crate::modules::registration::{IntrinsicFlowRegistration, register_intrinsic_flow};
mod types;

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "stream"],
        "Bounded byte stream substrate declarations.",
    );
    types::register(builder, module);
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
            requested_actions: &[StdEffectRef::typed(
                &["Stream", "read"],
                StdType::Var("S".to_owned()),
            )],
            intrinsic_id: intrinsic::runtime::STREAM_READ,
            summary: "Read at most the requested byte count from a host-mediated stream.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::default_action(131, &[0]),
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
            requested_actions: &[StdEffectRef::typed(
                &["Stream", "read"],
                StdType::Var("S".to_owned()),
            )],
            intrinsic_id: intrinsic::runtime::STREAM_READ_UNTIL_LIMIT,
            summary: "Read from a host-mediated stream until EOF or the requested byte limit.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::ActionReducer {
                stable_id: 131,
                payload: vec![0],
                reducer: crate::StdActionReducer::ReadUntilLimit {
                    limit_parameter: 1,
                    timeout_parameter: 2,
                },
            },
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
            requested_actions: &[StdEffectRef::typed(
                &["Stream", "write"],
                StdType::Var("S".to_owned()),
            )],
            intrinsic_id: intrinsic::runtime::STREAM_WRITE_ALL,
            summary: "Write all bytes to a host-mediated stream.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::default_action(132, &[0]),
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
            requested_actions: &[StdEffectRef::typed(
                &["Stream", "flush"],
                StdType::Var("S".to_owned()),
            )],
            intrinsic_id: intrinsic::runtime::STREAM_FLUSH,
            summary: "Flush a host-mediated stream.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::default_action(133, &[0]),
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
            requested_actions: &[StdEffectRef::typed(
                &["Stream", "close"],
                StdType::Var("S".to_owned()),
            )],
            intrinsic_id: intrinsic::runtime::STREAM_CLOSE,
            summary: "Close a host-mediated stream.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::default_action(134, &[0]),
        },
    );
}

fn byte_stream_param() -> StdGenericParam {
    StdGenericParam::bounded("S", &[StdSpecRef::new(&["std", "stream", "ByteStream"])])
}

fn stream_error_effect() -> StdEffectRef {
    StdEffectRef::typed(&["Error"], StdType::parse("std.stream.StreamError"))
}
