use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, intrinsic,
};

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
    io_flow(
        builder,
        module,
        "read_all",
        &[],
        "string",
        "Console.stdin_read_all",
        intrinsic::runtime::IO_READ_ALL,
        "Read all standard input through the future runtime boundary.",
    );
    io_flow(
        builder,
        module,
        "read_line",
        &[],
        "string",
        "Console.stdin_read_line",
        intrinsic::runtime::IO_READ_LINE,
        "Read one input line through the future runtime boundary.",
    );
    io_flow(
        builder,
        module,
        "print",
        &["string"],
        "unit",
        "Console.stdout_write",
        intrinsic::runtime::IO_PRINT,
        "Write text without a trailing newline through the future runtime boundary.",
    );
    io_flow(
        builder,
        module,
        "println",
        &["string"],
        "unit",
        "Console.stdout_write",
        intrinsic::runtime::IO_PRINTLN,
        "Write text with a trailing newline through the future runtime boundary.",
    );
    io_flow(
        builder,
        module,
        "eprintln",
        &["string"],
        "unit",
        "Console.stderr_write",
        intrinsic::runtime::IO_EPRINTLN,
        "Write error text with a trailing newline through the future runtime boundary.",
    );
}

fn io_flow(
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
        StdDecl::Flow(FlowDecl::with_actions(
            name,
            params,
            output,
            &["Error[IOError]"],
            &[action],
        )),
        summary,
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(id),
            qualified_path: vec!["std".into(), "io".into(), name.into()],
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
