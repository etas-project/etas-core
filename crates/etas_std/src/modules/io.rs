use crate::{
    IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl, StdEffectRef, StdRegistryBuilder,
    StdSymbolKind, StdType, TypeDecl, TypeDeclKind, intrinsic,
};

use crate::modules::registration::{IntrinsicFlowRegistration, register_intrinsic_flow};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "io"],
        "Console and process-standard-stream declarations.",
    );
    let io_error = builder.symbol(
        module,
        "IOError",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("IOError", &[], TypeDeclKind::Support)),
        "Standard error type for console and standard-stream failures.",
    );
    builder.prelude("IOError", io_error);
    register_intrinsic_flow(
        builder,
        module,
        &["std", "io"],
        IntrinsicFlowRegistration {
            name: "read_all",
            type_params: &[],
            params: &[],
            output: "string",
            public_effects: &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("std.io.IOError"),
            )],
            requested_actions: &[StdEffectRef::new(&["Console", "stdin_read_all"])],
            intrinsic_id: intrinsic::runtime::IO_READ_ALL,
            summary: "Read all standard input through the future runtime boundary.",
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "io"],
        IntrinsicFlowRegistration {
            name: "read_line",
            type_params: &[],
            params: &[],
            output: "string",
            public_effects: &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("std.io.IOError"),
            )],
            requested_actions: &[StdEffectRef::new(&["Console", "stdin_read_line"])],
            intrinsic_id: intrinsic::runtime::IO_READ_LINE,
            summary: "Read one input line through the future runtime boundary.",
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "io"],
        IntrinsicFlowRegistration {
            name: "print",
            type_params: &[],
            params: &["string"],
            output: "unit",
            public_effects: &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("std.io.IOError"),
            )],
            requested_actions: &[StdEffectRef::new(&["Console", "stdout_write"])],
            intrinsic_id: intrinsic::runtime::IO_PRINT,
            summary: "Write text without a trailing newline through the future runtime boundary.",
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "io"],
        IntrinsicFlowRegistration {
            name: "println",
            type_params: &[],
            params: &["string"],
            output: "unit",
            public_effects: &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("std.io.IOError"),
            )],
            requested_actions: &[StdEffectRef::new(&["Console", "stdout_write"])],
            intrinsic_id: intrinsic::runtime::IO_PRINTLN,
            summary: "Write text with a trailing newline through the future runtime boundary.",
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "io"],
        IntrinsicFlowRegistration {
            name: "eprintln",
            type_params: &[],
            params: &["string"],
            output: "unit",
            public_effects: &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("std.io.IOError"),
            )],
            requested_actions: &[StdEffectRef::new(&["Console", "stderr_write"])],
            intrinsic_id: intrinsic::runtime::IO_EPRINTLN,
            summary: "Write error text with a trailing newline through the future runtime boundary.",
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
        },
    );
}
