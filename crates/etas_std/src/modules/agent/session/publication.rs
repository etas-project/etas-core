use super::*;

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    let receipt = builder.symbol(
        module,
        "SessionContextReceipt",
        StdSymbolKind::Type,
        StdDecl::Type(
            TypeDecl::generic("SessionContextReceipt", &[], TypeDeclKind::Support)
                .with_representation(StdType::Record(vec![
                    StdRecordField::new("operation", named("std.memory.StorageOperationRef")),
                    StdRecordField::new(
                        "session",
                        StdType::Primitive(crate::StdPrimitiveType::String),
                    ),
                    StdRecordField::new(
                        "generation",
                        StdType::Primitive(crate::StdPrimitiveType::String),
                    ),
                    StdRecordField::new(
                        "context_version",
                        StdType::Primitive(crate::StdPrimitiveType::U64),
                    ),
                    StdRecordField::new("durability", named("std.memory.StorageDurability")),
                ])),
        ),
        "Backend-confirmed context publication evidence.",
    );
    builder.prelude("SessionContextReceipt", receipt);
    let rejection = builder.symbol(
        module,
        "SessionContextRejection",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic(
            "SessionContextRejection",
            &[],
            TypeDeclKind::Enum,
        )),
        "Confirmed context publication rejection.",
    );
    builder.prelude("SessionContextRejection", rejection);
    for (name, params) in [
        ("StaleHistory", vec![]),
        ("Rejected", vec![named("std.memory.StorageError")]),
    ] {
        builder
            .enum_constructor(
                rejection,
                FlowDecl {
                    name: name.into(),
                    type_params: vec![],
                    params,
                    output: named("std.agent.session.SessionContextRejection"),
                    public_effects: vec![],
                    requested_actions: vec![],
                    source_method: None,
                },
                "Context publication rejection.",
            )
            .expect("registered SessionContextRejection owner");
    }
    for (name, id, params, output, action) in [
        (
            "prepare_context",
            intrinsic::runtime::SESSION_PREPARE_CONTEXT,
            vec![
                named("SessionConfig"),
                named("SessionHistoryFence"),
                named("SessionContextContent"),
            ],
            named("std.memory.StorageOperationRef"),
            None,
        ),
        (
            "publish_context",
            intrinsic::runtime::SESSION_PUBLISH_CONTEXT,
            vec![
                named("SessionConfig"),
                named("SessionHistoryFence"),
                named("SessionContextContent"),
                named("std.memory.StorageOperationRef"),
            ],
            outcome("WriteOutcome"),
            Some("write"),
        ),
        (
            "reconcile_context",
            intrinsic::runtime::SESSION_RECONCILE_CONTEXT,
            vec![
                named("SessionConfig"),
                named("std.memory.StorageOperationRef"),
            ],
            outcome("ReconcileResult"),
            Some("read"),
        ),
    ] {
        builder.symbol_with_intrinsic(
            module,
            name,
            StdSymbolKind::Flow,
            StdDecl::Flow(FlowDecl {
                name: name.into(),
                type_params: vec![],
                params,
                output,
                public_effects: vec![StdEffectRef::with_args(
                    &["Error"],
                    vec![StdStaticArg::Type(named("std.memory.StorageError"))],
                )],
                requested_actions: action.map(session_memory_action).into_iter().collect(),
                source_method: None,
            }),
            "Prepare, conditionally publish, or reconcile caller-produced session context.",
            Some(IntrinsicDescriptor {
                id: StdIntrinsicId(id),
                qualified_path: ["std", "agent", "session", name]
                    .map(str::to_owned)
                    .to_vec(),
                purity: IntrinsicPurity::Runtime,
                dispatch: IntrinsicDispatch::Runtime,
                lowering: LoweringHint::RuntimeCall,
                latent_effect: crate::IntrinsicLatentEffect::None,
                memory_access: crate::IntrinsicMemoryAccess::None,
                runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
            }),
        );
    }
}
fn named(name: &str) -> StdType {
    StdType::Named(name.into())
}
fn outcome(name: &str) -> StdType {
    StdType::NamedApplied {
        name: format!("std.memory.{name}"),
        args: vec![
            named("SessionContextReceipt"),
            named("SessionContextRejection"),
        ],
    }
}
