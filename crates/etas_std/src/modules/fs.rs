use crate::{
    FlowDecl, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl, StdEffectRef,
    StdGenericParam, StdRegistryBuilder, StdSpecRef, StdSymbolKind, StdType, TypeDecl,
    TypeDeclKind, intrinsic,
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
            canonical_type: None,
            derivable: false,
            iterable_item: None,
            code_role: None,
            runtime_role: Some(crate::RuntimeTypeRole::WorkspacePath),
            construction: crate::StdTypeConstruction::Public,
            projected_fields: Vec::new(),
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
    let read_limit_error = builder.symbol(
        module,
        "ReadLimitError",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("ReadLimitError", &[], TypeDeclKind::Enum)),
        "A bounded workspace read exceeded its requested byte limit.",
    );
    builder
        .enum_constructor(
            read_limit_error,
            FlowDecl::pure("LimitExceeded", &["usize"], "std.fs.ReadLimitError"),
            "The file contains more than the requested maximum number of bytes.",
        )
        .expect("ReadLimitError owner is registered");
    let list_limit_error = builder.symbol(
        module,
        "ListLimitError",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("ListLimitError", &[], TypeDeclKind::Enum)),
        "A complete directory listing exceeded its entry or aggregate relative-name byte limit.",
    );
    builder
        .enum_constructor(
            list_limit_error,
            FlowDecl::pure(
                "LimitExceeded",
                &["usize", "usize"],
                "std.fs.ListLimitError",
            ),
            "The maximum entry count or aggregate UTF-8 relative-name bytes was exceeded.",
        )
        .expect("ListLimitError owner is registered");
    for (name, params, output, id) in [
        (
            "root",
            vec![],
            "WorkspacePath[R]",
            intrinsic::runtime::FS_ROOT,
        ),
        (
            "relative_name",
            vec!["WorkspacePath[R]"],
            "string",
            intrinsic::runtime::FS_RELATIVE_NAME,
        ),
    ] {
        register_intrinsic_flow(
            builder,
            module,
            &["std", "fs"],
            IntrinsicFlowRegistration {
                name,
                type_params: &[region_param()],
                params: &params,
                output,
                public_effects: &[],
                requested_actions: &[],
                intrinsic_id: id,
                summary: "Inspect or construct a relative path identity without granting filesystem authority.",
                purity: IntrinsicPurity::Runtime,
                dispatch: IntrinsicDispatch::Runtime,
                lowering: LoweringHint::RuntimeCall,
            },
        );
    }
    register_intrinsic_flow(
        builder,
        module,
        &["std", "fs"],
        IntrinsicFlowRegistration {
            name: "list_limited",
            type_params: &[region_param()],
            params: &["WorkspacePath[R]", "usize", "usize"],
            output: "List[WorkspacePath[R]]",
            public_effects: &[
                StdEffectRef::typed(&["Error"], StdType::parse("std.fs.IOError")),
                StdEffectRef::typed(&["Error"], StdType::parse("std.fs.ListLimitError")),
            ],
            requested_actions: &[StdEffectRef::typed(
                &["Fs", "list"],
                StdType::Var("R".into()),
            )],
            intrinsic_id: intrinsic::runtime::FS_LIST_LIMITED,
            summary: "Return a complete sorted directory listing within entry and aggregate UTF-8 relative-name byte limits, or raise ListLimitError without a partial result.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::ActionReducer {
                stable_id: 138,
                payload: vec![0],
                reducer: crate::StdActionReducer::BoundedDirectoryList {
                    entry_limit_parameter: 1,
                    byte_limit_parameter: 2,
                },
            },
        },
    );
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
            lowering: LoweringHint::default_action(136, &[0]),
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "fs"],
        IntrinsicFlowRegistration {
            name: "read_bytes_limited",
            type_params: &[region_param()],
            params: &["WorkspacePath[R]", "usize"],
            output: "bytes",
            public_effects: &[
                StdEffectRef::typed(&["Error"], StdType::parse("std.fs.IOError")),
                StdEffectRef::typed(&["Error"], StdType::parse("std.fs.ReadLimitError")),
            ],
            requested_actions: &[StdEffectRef::typed(
                &["Fs", "read"],
                StdType::Var("R".to_owned()),
            )],
            intrinsic_id: intrinsic::runtime::FS_READ_BYTES_LIMITED,
            summary: "Read a complete workspace file within a byte limit, or raise ReadLimitError without a partial result.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::ActionReducer {
                stable_id: 136,
                payload: vec![0],
                reducer: crate::StdActionReducer::BoundedFileRead { limit_parameter: 1 },
            },
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
            lowering: LoweringHint::default_action(137, &[0]),
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
            lowering: LoweringHint::default_action(138, &[0]),
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
            lowering: LoweringHint::default_action(139, &[0]),
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
            lowering: LoweringHint::default_action(140, &[0]),
        },
    );
}

fn region_param() -> StdGenericParam {
    StdGenericParam::bounded("R", &[StdSpecRef::new(&["std", "fs", "Region"])])
}
