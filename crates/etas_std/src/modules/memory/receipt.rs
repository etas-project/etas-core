use super::*;

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    for (name, params, fields) in [
        (
            "MemoryTombstone",
            vec![],
            vec![("opaque", StdType::parse("string"))],
        ),
        (
            "MemoryWriteTarget",
            vec!["K"],
            vec![
                ("region", StdType::parse("string")),
                ("store", StdType::Array(Box::new(StdType::parse("string")))),
                (
                    "schema_fingerprint",
                    StdType::Option(Box::new(StdType::parse("string"))),
                ),
                ("key", StdType::Var("K".into())),
            ],
        ),
        (
            "MemoryWriteReceipt",
            vec!["K"],
            vec![
                ("operation", named("StorageOperationRef")),
                (
                    "target",
                    StdType::NamedApplied {
                        name: "std.memory.MemoryWriteTarget".into(),
                        args: vec![StdType::Var("K".into())],
                    },
                ),
                ("change", named("MemoryWriteChange")),
                ("durability", named("StorageDurability")),
            ],
        ),
    ] {
        let decl = TypeDecl::generic(name, &params, TypeDeclKind::Support).with_representation(
            StdType::Record(
                fields
                    .into_iter()
                    .map(|(name, ty)| StdRecordField::new(name, ty))
                    .collect(),
            ),
        );
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(decl),
            "Typed storage receipt evidence.",
        );
        builder.prelude(name, symbol);
    }
    enumeration(
        builder,
        module,
        "MemoryWriteChange",
        vec![
            ("Written", vec![named("MemoryVersion")]),
            ("Deleted", vec![named("MemoryTombstone")]),
        ],
    );
    enumeration(
        builder,
        module,
        "StorageDurability",
        vec![("Volatile", vec![]), ("SqliteWalFull", vec![])],
    );
    enumeration(
        builder,
        module,
        "MemoryWriteRejection",
        vec![
            (
                "ConditionConflict",
                vec![
                    StdType::Option(Box::new(named("MemoryVersion"))),
                    StdType::Option(Box::new(named("MemoryVersion"))),
                ],
            ),
            ("Unchanged", vec![]),
            ("Rejected", vec![named("StorageError")]),
        ],
    );
}

fn named(name: &str) -> StdType {
    StdType::Named(format!("std.memory.{name}"))
}

fn enumeration(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    variants: Vec<(&str, Vec<StdType>)>,
) {
    let owner = builder.symbol(
        module,
        name,
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Enum)),
        "Closed storage evidence.",
    );
    builder.prelude(name, owner);
    for (variant, params) in variants {
        builder
            .enum_constructor(
                owner,
                FlowDecl {
                    name: variant.into(),
                    type_params: vec![],
                    params,
                    output: named(name),
                    public_effects: vec![],
                    requested_actions: vec![],
                    source_method: None,
                },
                "Storage evidence variant.",
            )
            .expect("storage enum owner is registered");
    }
}
