use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdEffectRef, StdGenericParam, StdIntrinsicId, StdRegistryBuilder, StdSpecRef, StdSymbolKind,
    TypeDecl, TypeDeclKind, intrinsic,
};

use crate::modules::registration::{IntrinsicFlowRegistration, register_intrinsic_flow};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "host", "command"], "Host command support types.");
    for name in ["Command", "CommandResult", "CommandHandle"] {
        let declaration = TypeDecl::generic(name, &[], TypeDeclKind::Support);
        let declaration = if name == "CommandResult" {
            declaration
                .runtime_role(crate::RuntimeTypeRole::CommandResult)
                .intrinsic_only()
        } else if name == "Command" {
            declaration.runtime_role(crate::RuntimeTypeRole::Command)
        } else {
            declaration
        };
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(declaration),
            "Host command support type.",
        );
    }
    let event = builder.symbol(
        module,
        "CommandEvent",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("CommandEvent", &[], TypeDeclKind::Enum)),
        "An incremental command output or terminal process event.",
    );
    for (name, fields) in [
        ("Output", &["bytes", "bytes"][..]),
        ("Exited", &["i32", "bytes", "bytes"][..]),
        ("Stopped", &["bytes", "bytes"][..]),
    ] {
        builder
            .enum_constructor(
                event,
                FlowDecl::pure(name, fields, "std.host.command.CommandEvent"),
                "Command process event constructor.",
            )
            .expect("CommandEvent owner is registered");
    }

    register_runtime_flow(
        builder,
        module,
        IntrinsicFlowRegistration {
            name: "command",
            type_params: &[],
            params: &["string", "Array[string]"],
            output: "Command",
            public_effects: &[],
            requested_actions: &[],
            intrinsic_id: intrinsic::runtime::COMMAND_NEW,
            summary: "Construct an opaque command value from a program and argument list.",
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
        },
    );

    register_runtime_flow(
        builder,
        module,
        IntrinsicFlowRegistration {
            name: "id",
            type_params: &[],
            params: &["CommandHandle"],
            output: "string",
            public_effects: &[],
            requested_actions: &[],
            intrinsic_id: intrinsic::runtime::COMMAND_HANDLE_ID,
            summary: "Return the stable reconnect identifier for a command handle.",
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
        },
    );

    for (name, params, output, action, intrinsic_id, summary) in [
        (
            "start",
            &["Command", "SandboxProfile"][..],
            "CommandHandle",
            "start",
            intrinsic::runtime::COMMAND_START,
            "Start a command as a host-managed background process.",
        ),
        (
            "reconnect",
            &["string", "SandboxProfile"][..],
            "CommandHandle",
            "reconnect",
            intrinsic::runtime::COMMAND_RECONNECT,
            "Reconnect to a host-managed background command by stable identifier.",
        ),
        (
            "read",
            &["CommandHandle", "usize"][..],
            "CommandEvent",
            "read",
            intrinsic::runtime::COMMAND_READ,
            "Read a bounded incremental output event from a background command.",
        ),
        (
            "write",
            &["CommandHandle", "bytes"][..],
            "unit",
            "write",
            intrinsic::runtime::COMMAND_WRITE,
            "Write bytes to a background command's standard input.",
        ),
        (
            "close_stdin",
            &["CommandHandle"][..],
            "unit",
            "write",
            intrinsic::runtime::COMMAND_CLOSE_STDIN,
            "Close a background command's standard input.",
        ),
        (
            "stop",
            &["CommandHandle"][..],
            "CommandEvent",
            "stop",
            intrinsic::runtime::COMMAND_STOP,
            "Stop a background command and return its terminal output event.",
        ),
    ] {
        register_host_command_flow(
            builder,
            module,
            name,
            params,
            output,
            action,
            intrinsic_id,
            summary,
        );
    }
    register_runtime_flow(
        builder,
        module,
        IntrinsicFlowRegistration {
            name: "with_env",
            type_params: &[],
            params: &["Command", "Map[string, string]"],
            output: "Command",
            public_effects: &[],
            requested_actions: &[],
            intrinsic_id: intrinsic::runtime::COMMAND_WITH_ENV,
            summary: "Return a command value with an explicit environment.",
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_runtime_flow(
        builder,
        module,
        IntrinsicFlowRegistration {
            name: "with_cwd",
            type_params: &[region_param()],
            params: &["Command", "std.fs.WorkspacePath[R]"],
            output: "Command",
            public_effects: &[],
            requested_actions: &[],
            intrinsic_id: intrinsic::runtime::COMMAND_WITH_CWD,
            summary: "Return a command value with a region-indexed working directory.",
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_runtime_flow(
        builder,
        module,
        IntrinsicFlowRegistration {
            name: "with_stdin",
            type_params: &[],
            params: &["Command", "bytes"],
            output: "Command",
            public_effects: &[],
            requested_actions: &[],
            intrinsic_id: intrinsic::runtime::COMMAND_WITH_STDIN,
            summary: "Return a command value with bounded stdin bytes.",
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
        },
    );

    builder.symbol_with_intrinsic(
        module,
        "run",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_actions(
            "run",
            &["Command", "SandboxProfile"],
            "CommandResult",
            &[],
            &[StdEffectRef::wildcard(&["Command", "run"], 1)],
        )),
        "Run a command through the checked command host boundary.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::COMMAND_RUN),
            qualified_path: vec!["std".into(), "host".into(), "command".into(), "run".into()],
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}

fn register_runtime_flow(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    registration: IntrinsicFlowRegistration<'_>,
) {
    register_intrinsic_flow(builder, module, &["std", "host", "command"], registration);
}

fn region_param() -> StdGenericParam {
    StdGenericParam::bounded("R", &[StdSpecRef::new(&["std", "fs", "Region"])])
}

#[allow(clippy::too_many_arguments)]
fn register_host_command_flow(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    params: &[&str],
    output: &str,
    action: &str,
    intrinsic_id: u32,
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
            &[],
            &[StdEffectRef::wildcard(&["Command", action], 1)],
        )),
        summary,
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic_id),
            qualified_path: vec!["std".into(), "host".into(), "command".into(), name.into()],
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
