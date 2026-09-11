use super::*;

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    for (name, params, kind, representation) in [
        (
            "MemoryWriteIntent",
            &["K", "V"][..],
            TypeDeclKind::Support,
            None,
        ),
        ("WriteCondition", &[][..], TypeDeclKind::Enum, None),
        (
            "StorageOperationRef",
            &[][..],
            TypeDeclKind::Support,
            Some(record(&[("key", "string"), ("fingerprint", "string")])),
        ),
        (
            "StorageError",
            &[][..],
            TypeDeclKind::Support,
            Some(record(&[("code", "string"), ("message", "string")])),
        ),
    ] {
        let mut decl = TypeDecl::generic(name, params, kind);
        decl.representation = representation;
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(decl),
            "Storage preparation contract.",
        );
        builder.prelude(name, symbol);
    }
    for name in ["Any", "Missing", "Exists"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Value,
            StdDecl::Value(crate::ValueDecl::new(name, "WriteCondition")),
            "Conditional mutation requirement.",
        );
    }
    builder.symbol(
        module,
        "Match",
        StdSymbolKind::Constructor,
        StdDecl::Flow(FlowDecl::pure(
            "Match",
            &["MemoryVersion"],
            "WriteCondition",
        )),
        "Require the exact backend-issued version.",
    );
    for (name, params, output, id, fallible) in [
        (
            "prepare_put",
            vec![
                store_type(),
                var("K"),
                var("V"),
                StdType::Named("WriteCondition".into()),
            ],
            applied("MemoryWriteIntent"),
            intrinsic::runtime::MEMORY_PREPARE_PUT,
            true,
        ),
        (
            "prepare_delete",
            vec![
                store_type(),
                var("K"),
                StdType::Named("WriteCondition".into()),
            ],
            applied("MemoryWriteIntent"),
            intrinsic::runtime::MEMORY_PREPARE_DELETE,
            true,
        ),
        (
            "operation_ref",
            vec![applied("MemoryWriteIntent")],
            StdType::Named("StorageOperationRef".into()),
            intrinsic::runtime::MEMORY_OPERATION_REF,
            false,
        ),
    ] {
        let symbol = builder.symbol_with_intrinsic(
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
                output,
                public_effects: if fallible {
                    vec![StdEffectRef::with_args(
                        &["Error"],
                        vec![StdStaticArg::Type(StdType::Named("StorageError".into()))],
                    )]
                } else {
                    Vec::new()
                },
                requested_actions: Vec::new(),
                source_method: None,
            }),
            "Prepare immutable write data or inspect its identity without accessing storage.",
            Some(IntrinsicDescriptor {
                id: StdIntrinsicId(id),
                qualified_path: vec!["std".into(), "memory".into(), name.into()],
                purity: IntrinsicPurity::Runtime,
                dispatch: IntrinsicDispatch::Runtime,
                lowering: LoweringHint::RuntimeCall,
                latent_effect: crate::IntrinsicLatentEffect::None,
                memory_access: IntrinsicMemoryAccess::None,
                runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
            }),
        );
        if fallible {
            builder
                .memory_place_result(symbol, 0)
                .expect("memory preparation preserves its Store argument");
        }
    }
}

fn var(name: &str) -> StdType {
    StdType::Var(name.into())
}
fn applied(name: &str) -> StdType {
    StdType::NamedApplied {
        name: name.into(),
        args: vec![var("K"), var("V")],
    }
}
fn record(fields: &[(&str, &str)]) -> StdType {
    StdType::Record(
        fields
            .iter()
            .map(|(name, ty)| StdRecordField::new(name, StdType::parse(ty)))
            .collect(),
    )
}
