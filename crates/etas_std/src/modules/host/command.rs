use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdEffectRef, StdGenericParam, StdIntrinsicId, StdRegistryBuilder, StdSpecRef, StdSymbolKind,
    TypeDecl, TypeDeclKind, intrinsic,
};

use crate::modules::registration::{IntrinsicFlowRegistration, register_intrinsic_flow};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "host", "command"], "Host command support types.");
    for name in ["Command", "CommandResult", "CommandHandle"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Host command support type.",
        );
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
