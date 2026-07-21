use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, intrinsic,
};

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
    fs_flow(
        builder,
        module,
        "read_bytes",
        &["WorkspacePath"],
        "bytes",
        "Fs.read[path]",
        intrinsic::runtime::FS_READ_BYTES,
        "Read bytes from a project-scoped workspace path.",
    );
    fs_flow(
        builder,
        module,
        "write_bytes",
        &["WorkspacePath", "bytes"],
        "unit",
        "Fs.write[path]",
        intrinsic::runtime::FS_WRITE_BYTES,
        "Write bytes to a project-scoped workspace path.",
    );
    fs_flow(
        builder,
        module,
        "list",
        &["WorkspacePath"],
        "List[WorkspacePath]",
        "Fs.list[path]",
        intrinsic::runtime::FS_LIST,
        "List a project-scoped workspace directory.",
    );
    fs_flow(
        builder,
        module,
        "stat",
        &["WorkspacePath"],
        "FsStat",
        "Fs.stat[path]",
        intrinsic::runtime::FS_STAT,
        "Read project-scoped filesystem metadata.",
    );
    fs_flow(
        builder,
        module,
        "atomic_replace",
        &["WorkspacePath", "bytes"],
        "unit",
        "Fs.atomic_replace[path]",
        intrinsic::runtime::FS_ATOMIC_REPLACE,
        "Atomically replace bytes at a project-scoped workspace path.",
    );
}

fn fs_flow(
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
            qualified_path: vec!["std".into(), "fs".into(), name.into()],
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
