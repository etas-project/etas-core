use crate::{
    IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl, StdEffectRef, StdRegistryBuilder,
    StdSymbolKind, StdType, TypeDecl, TypeDeclKind, intrinsic,
};

use crate::modules::registration::{IntrinsicFlowRegistration, register_intrinsic_flow};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "fs"],
        "Project-scoped filesystem substrate declarations.",
    );
    for name in ["WorkspacePath", "FsEntry", "FsStat", "IOError"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Filesystem substrate support type.",
        );
    }
    register_intrinsic_flow(
        builder,
        module,
        &["std", "fs"],
        IntrinsicFlowRegistration {
            name: "read_bytes",
            type_params: &[],
            params: &["WorkspacePath"],
            output: "bytes",
            public_effects: &[StdEffectRef::typed(&["Error"], StdType::parse("IOError"))],
            requested_actions: &[StdEffectRef::wildcard(&["Fs", "read"], 1)],
            intrinsic_id: intrinsic::runtime::FS_READ_BYTES,
            summary: "Read bytes from a project-scoped workspace path.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "fs"],
        IntrinsicFlowRegistration {
            name: "write_bytes",
            type_params: &[],
            params: &["WorkspacePath", "bytes"],
            output: "unit",
            public_effects: &[StdEffectRef::typed(&["Error"], StdType::parse("IOError"))],
            requested_actions: &[StdEffectRef::wildcard(&["Fs", "write"], 1)],
            intrinsic_id: intrinsic::runtime::FS_WRITE_BYTES,
            summary: "Write bytes to a project-scoped workspace path.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "fs"],
        IntrinsicFlowRegistration {
            name: "list",
            type_params: &[],
            params: &["WorkspacePath"],
            output: "List[WorkspacePath]",
            public_effects: &[StdEffectRef::typed(&["Error"], StdType::parse("IOError"))],
            requested_actions: &[StdEffectRef::wildcard(&["Fs", "list"], 1)],
            intrinsic_id: intrinsic::runtime::FS_LIST,
            summary: "List a project-scoped workspace directory.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "fs"],
        IntrinsicFlowRegistration {
            name: "stat",
            type_params: &[],
            params: &["WorkspacePath"],
            output: "FsStat",
            public_effects: &[StdEffectRef::typed(&["Error"], StdType::parse("IOError"))],
            requested_actions: &[StdEffectRef::wildcard(&["Fs", "stat"], 1)],
            intrinsic_id: intrinsic::runtime::FS_STAT,
            summary: "Read project-scoped filesystem metadata.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "fs"],
        IntrinsicFlowRegistration {
            name: "atomic_replace",
            type_params: &[],
            params: &["WorkspacePath", "bytes"],
            output: "unit",
            public_effects: &[StdEffectRef::typed(&["Error"], StdType::parse("IOError"))],
            requested_actions: &[StdEffectRef::wildcard(&["Fs", "atomic_replace"], 1)],
            intrinsic_id: intrinsic::runtime::FS_ATOMIC_REPLACE,
            summary: "Atomically replace bytes at a project-scoped workspace path.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
}
