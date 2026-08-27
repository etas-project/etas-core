use crate::{
    IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl, StdEffectRef, StdGenericParam,
    StdRegistryBuilder, StdSpecRef, StdSymbolKind, StdType, TypeDecl, TypeDeclKind, intrinsic,
};

use crate::modules::registration::{IntrinsicFlowRegistration, register_intrinsic_flow};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "fs"],
        "Project-scoped filesystem substrate declarations.",
    );
    builder.symbol(
        module,
        "Region",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Region", &[], TypeDeclKind::Spec)),
        "Static filesystem authority-region specification.",
    );
    for name in ["WorkspacePath", "FsEntry", "FsStat", "IOError"] {
        let params = if name == "WorkspacePath" {
            &["R"][..]
        } else {
            &[]
        };
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, params, TypeDeclKind::Support)),
            "Filesystem substrate support type.",
        );
    }
    register_intrinsic_flow(
        builder,
        module,
        &["std", "fs"],
        IntrinsicFlowRegistration {
            name: "read_bytes",
            type_params: &[region_param()],
            params: &["WorkspacePath[R]"],
            output: "bytes",
            public_effects: &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("std.fs.IOError"),
            )],
            requested_actions: &[StdEffectRef::typed(
                &["Fs", "read"],
                StdType::Var("R".to_owned()),
            )],
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
            type_params: &[region_param()],
            params: &["WorkspacePath[R]", "bytes"],
            output: "unit",
            public_effects: &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("std.fs.IOError"),
            )],
            requested_actions: &[StdEffectRef::typed(
                &["Fs", "write"],
                StdType::Var("R".to_owned()),
            )],
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
            type_params: &[region_param()],
            params: &["WorkspacePath[R]"],
            output: "List[WorkspacePath[R]]",
            public_effects: &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("std.fs.IOError"),
            )],
            requested_actions: &[StdEffectRef::typed(
                &["Fs", "list"],
                StdType::Var("R".to_owned()),
            )],
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
            type_params: &[region_param()],
            params: &["WorkspacePath[R]"],
            output: "FsStat",
            public_effects: &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("std.fs.IOError"),
            )],
            requested_actions: &[StdEffectRef::typed(
                &["Fs", "stat"],
                StdType::Var("R".to_owned()),
            )],
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
            type_params: &[region_param()],
            params: &["WorkspacePath[R]", "bytes"],
            output: "unit",
            public_effects: &[StdEffectRef::typed(
                &["Error"],
                StdType::parse("std.fs.IOError"),
            )],
            requested_actions: &[StdEffectRef::typed(
                &["Fs", "atomic_replace"],
                StdType::Var("R".to_owned()),
            )],
            intrinsic_id: intrinsic::runtime::FS_ATOMIC_REPLACE,
            summary: "Atomically replace bytes at a project-scoped workspace path.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
}

fn region_param() -> StdGenericParam {
    StdGenericParam::bounded("R", &[StdSpecRef::new(&["std", "fs", "Region"])])
}
