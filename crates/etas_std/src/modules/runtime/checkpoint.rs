use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdGenericParam, StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let runtime = builder.module(&["std", "runtime"], "Runtime support declarations.");
    register_checkpoint_symbol(builder, runtime, &["std", "runtime", "checkpoint"]);

    let module = builder.module(
        &["std", "runtime", "checkpoint"],
        "Checkpoint support declarations.",
    );
    register_checkpoint_symbol(
        builder,
        module,
        &["std", "runtime", "checkpoint", "checkpoint"],
    );
}

fn register_checkpoint_symbol(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    qualified_path: &[&str],
) {
    builder.symbol_with_intrinsic(
        module,
        "checkpoint",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            "checkpoint",
            &[StdGenericParam::new("T")],
            &["T"],
            "unit",
            &[],
            &[],
        )),
        "Checkpoint runtime state through the future runtime boundary.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::CHECKPOINT),
            qualified_path: qualified_path
                .iter()
                .map(|segment| (*segment).into())
                .collect(),
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::Checkpoint,
        }),
    );
}
