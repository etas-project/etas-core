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
    builder.symbol(
        module,
        "WorkspacePath",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl {
            name: "WorkspacePath".to_owned(),
            params: vec![region_param()],
            kind: TypeDeclKind::Support,
            representation: None,
            derivable: false,
        }),
        "Opaque project-scoped path indexed by its filesystem authority region.",
    );
    for name in ["FsEntry", "FsStat", "IOError"] {
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
            name: "path",
            type_params: &[region_param()],
            params: &["string"],
            output: "Result[std.fs.WorkspacePath[R], std.fs.IOError]",
            public_effects: &[],
            requested_actions: &[],
            intrinsic_id: intrinsic::runtime::FS_PATH,
            summary: "Construct an opaque region-indexed workspace path from a relative path.",
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
        },
    );
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
