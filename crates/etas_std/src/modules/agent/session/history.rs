use super::*;
use crate::StdPrimitiveType;

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    for (name, fields) in [
        ("SessionCursor", vec![("opaque", string())]),
        ("SessionHistoryFence", vec![("opaque", string())]),
        (
            "SessionContextContent",
            vec![
                ("text", string()),
                (
                    "provenance",
                    StdType::Map {
                        key: Box::new(string()),
                        value: Box::new(string()),
                    },
                ),
            ],
        ),
        (
            "SessionPublishedContext",
            vec![
                ("content", named("SessionContextContent")),
                ("fence", named("SessionHistoryFence")),
                ("version", StdType::Primitive(StdPrimitiveType::U64)),
            ],
        ),
        (
            "SessionHistoryPage",
            vec![
                ("fence", named("SessionHistoryFence")),
                (
                    "messages",
                    StdType::Array(Box::new(StdType::Message(Box::new(named(
                        "std.json.JsonValue",
                    ))))),
                ),
                ("published_context", optional("SessionPublishedContext")),
                ("summary", optional("SessionSummary")),
                ("cursor", optional("SessionCursor")),
            ],
        ),
    ] {
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(
                TypeDecl::generic(name, &[], TypeDeclKind::Support).with_representation(
                    StdType::Record(
                        fields
                            .into_iter()
                            .map(|(name, ty)| StdRecordField::new(name, ty))
                            .collect(),
                    ),
                ),
            ),
            "Bounded session history and backend-issued selection evidence.",
        );
        builder.prelude(name, symbol);
    }
    builder.symbol_with_intrinsic(
        module,
        "history_page",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl {
            name: "history_page".into(),
            type_params: vec![],
            params: vec![
                named("SessionConfig"),
                optional("SessionCursor"),
                StdType::Primitive(StdPrimitiveType::U32),
            ],
            output: named("SessionHistoryPage"),
            public_effects: vec![StdEffectRef::with_args(
                &["Error"],
                vec![StdStaticArg::Type(named("std.memory.StorageError"))],
            )],
            requested_actions: vec![session_memory_action("read")],
            source_method: None,
        }),
        "Read one bounded page from an existing session using a stable backend cursor.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::SESSION_HISTORY_PAGE),
            qualified_path: ["std", "agent", "session", "history_page"]
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

fn string() -> StdType {
    StdType::Primitive(StdPrimitiveType::String)
}
fn named(name: &str) -> StdType {
    StdType::Named(name.into())
}
fn optional(name: &str) -> StdType {
    StdType::Option(Box::new(named(name)))
}
