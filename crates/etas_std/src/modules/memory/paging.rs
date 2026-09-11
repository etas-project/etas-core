use super::*;

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    let cursor = builder.symbol(
        module,
        "MemoryCursor",
        StdSymbolKind::Type,
        StdDecl::Type(
            TypeDecl::generic("MemoryCursor", &[], TypeDeclKind::Support).with_representation(
                StdType::Record(vec![StdRecordField::new(
                    "opaque",
                    StdType::Primitive(crate::StdPrimitiveType::String),
                )]),
            ),
        ),
        "Store-scoped continuation token; writes invalidate the scan cursor.",
    );
    builder.prelude("MemoryCursor", cursor);
    let entry = builder.symbol(
        module,
        "MemoryEntry",
        StdSymbolKind::Type,
        StdDecl::Type(
            TypeDecl::generic("MemoryEntry", &["K", "V"], TypeDeclKind::Support)
                .with_representation(StdType::Record(vec![
                    StdRecordField::new("key", StdType::Var("K".into())),
                    StdRecordField::new("value", StdType::Var("V".into())),
                    StdRecordField::new("version", StdType::Named("MemoryVersion".into())),
                ])),
        ),
        "A typed stored value and its compare-and-set version.",
    );
    builder.prelude("MemoryEntry", entry);
    let page = builder.symbol(
        module,
        "MemoryPage",
        StdSymbolKind::Type,
        StdDecl::Type(
            TypeDecl::generic("MemoryPage", &["K", "V"], TypeDeclKind::Support)
                .with_representation(StdType::Record(vec![
                    StdRecordField::new("entries", StdType::List(Box::new(applied("MemoryEntry")))),
                    StdRecordField::new(
                        "cursor",
                        StdType::Option(Box::new(StdType::Named("MemoryCursor".into()))),
                    ),
                ])),
        ),
        "A bounded page of entries and an optional continuation cursor.",
    );
    builder.prelude("MemoryPage", page);
    register_store_flow(
        builder,
        module,
        StoreFlowRegistration {
            name: "page",
            type_params: &["K", "V"],
            params: vec![
                store_type(),
                StdType::Option(Box::new(StdType::Named("MemoryCursor".into()))),
                StdType::Primitive(crate::StdPrimitiveType::U32),
            ],
            output: applied("MemoryPage"),
            docs: "Read one bounded page; pass None for the first page. Writes invalidate its cursor.",
            intrinsic_id: intrinsic::runtime::MEMORY_PAGE,
        },
    );
    register_store_flow(
        builder,
        module,
        StoreFlowRegistration {
            name: "get_entry",
            type_params: &["K", "V"],
            params: vec![store_type(), StdType::Var("K".into())],
            output: StdType::Option(Box::new(applied("MemoryEntry"))),
            docs: "Read the key, value and version atomically for a conditional write.",
            intrinsic_id: intrinsic::runtime::MEMORY_GET_ENTRY,
        },
    );
}

fn applied(name: &str) -> StdType {
    StdType::NamedApplied {
        name: name.into(),
        args: vec![StdType::Var("K".into()), StdType::Var("V".into())],
    }
}
