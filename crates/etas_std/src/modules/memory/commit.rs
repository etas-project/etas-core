use super::*;

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    for (name, id, params, result, access) in [
        (
            "commit",
            intrinsic::runtime::MEMORY_COMMIT,
            vec![applied("MemoryWriteIntent", vec![var("K"), var("V")])],
            "WriteOutcome",
            IntrinsicMemoryAccess::WriteFirstArgIntent,
        ),
        (
            "reconcile",
            intrinsic::runtime::MEMORY_RECONCILE,
            vec![store_type(), StdType::Named("StorageOperationRef".into())],
            "ReconcileResult",
            IntrinsicMemoryAccess::ReadFirstArgStore,
        ),
    ] {
        builder.symbol_with_intrinsic(
            module,
            name,
            StdSymbolKind::Flow,
            StdDecl::Flow(FlowDecl {
                name: name.into(),
                type_params: vec![
                    crate::StdGenericParam::new("K"),
                    crate::StdGenericParam::new("V"),
                ],
                params,
                output: applied(
                    result,
                    vec![
                        applied("MemoryWriteReceipt", vec![var("K")]),
                        StdType::Named("MemoryWriteRejection".into()),
                    ],
                ),
                public_effects: vec![StdEffectRef::with_args(
                    &["Error"],
                    vec![StdStaticArg::Type(StdType::Named("StorageError".into()))],
                )],
                requested_actions: Vec::new(),
                source_method: None,
            }),
            "Commit an immutable storage intent or query its retained completion evidence.",
            Some(IntrinsicDescriptor {
                id: StdIntrinsicId(id),
                qualified_path: vec!["std".into(), "memory".into(), name.into()],
                purity: IntrinsicPurity::Runtime,
                dispatch: IntrinsicDispatch::Runtime,
                lowering: LoweringHint::RuntimeCall,
                latent_effect: crate::IntrinsicLatentEffect::None,
                memory_access: access,
                runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
            }),
        );
    }
}
fn var(name: &str) -> StdType {
    StdType::Var(name.into())
}
fn applied(name: &str, args: Vec<StdType>) -> StdType {
    StdType::NamedApplied {
        name: name.into(),
        args,
    }
}
