use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicMemoryAccess, IntrinsicPurity,
    LoweringHint, StdDecl, StdEffectRef, StdIntrinsicId, StdRecordField, StdRegistryBuilder,
    StdStaticArg, StdSymbolKind, StdType, TypeDecl, TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "memory"],
        "Typed persistent memory support declarations.",
    );

    for (name, params) in [
        ("MemoryRegion", &["S"][..]),
        ("Store", &["K", "V"][..]),
        ("MemorySelection", &["V"][..]),
        ("MemoryTransaction", &[][..]),
    ] {
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, params, TypeDeclKind::Support)),
            "Typed persistent memory support type.",
        );
        builder.prelude(name, symbol);
    }
    let version_symbol = builder.symbol(
        module,
        "MemoryVersion",
        StdSymbolKind::Type,
        StdDecl::Type(
            TypeDecl::generic("MemoryVersion", &[], TypeDeclKind::Support).with_representation(
                StdType::Record(vec![StdRecordField::new(
                    "opaque",
                    StdType::Primitive(crate::StdPrimitiveType::String),
                )]),
            ),
        ),
        "Typed persistent memory version token.",
    );
    builder.prelude("MemoryVersion", version_symbol);
    let conflict_symbol = builder.symbol(
        module,
        "MemoryConflict",
        StdSymbolKind::Type,
        StdDecl::Type(
            TypeDecl::generic("MemoryConflict", &[], TypeDeclKind::Support).with_representation(
                StdType::Record(vec![
                    StdRecordField::new(
                        "expected",
                        StdType::Option(Box::new(StdType::Named("MemoryVersion".to_owned()))),
                    ),
                    StdRecordField::new(
                        "actual",
                        StdType::Option(Box::new(StdType::Named("MemoryVersion".to_owned()))),
                    ),
                    StdRecordField::new(
                        "current_value",
                        StdType::Option(Box::new(StdType::Named("JsonValue".to_owned()))),
                    ),
                ]),
            ),
        ),
        "Typed persistent memory conflict information.",
    );
    builder.prelude("MemoryConflict", conflict_symbol);

    builder.symbol_with_intrinsic(
        module,
        "region",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl {
            name: "region".to_owned(),
            type_params: vec![crate::StdGenericParam::new("S")],
            params: vec![
                StdType::Primitive(crate::StdPrimitiveType::String),
                StdType::Primitive(crate::StdPrimitiveType::String),
            ],
            output: StdType::ResourceHandleMemoryRegion(Box::new(StdType::Var("S".to_owned()))),
            public_effects: Vec::new(),
            requested_actions: Vec::new(),
            source_method: None,
        }),
        "Bind a typed persistent memory region handle.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::MEMORY_REGION),
            qualified_path: vec!["std".into(), "memory".into(), "region".into()],
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
    builder.symbol_with_intrinsic(
        module,
        "version",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure("version", &["string"], "MemoryVersion")),
        "Construct an opaque memory version token for compare-and-set operations.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::pure::MEMORY_VERSION),
            qualified_path: vec!["std".into(), "memory".into(), "version".into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );

    register_store_flow(
        builder,
        module,
        "get",
        vec![store_type(), StdType::Var("K".to_owned())],
        StdType::Option(Box::new(StdType::Var("V".to_owned()))),
        "Read a typed value from a persistent store.",
        intrinsic::runtime::MEMORY_GET,
    );
    register_store_flow(
        builder,
        module,
        "contains",
        vec![store_type(), StdType::Var("K".to_owned())],
        StdType::Primitive(crate::StdPrimitiveType::Bool),
        "Return whether a persistent store contains the given key.",
        intrinsic::runtime::MEMORY_CONTAINS,
    );
    register_store_flow(
        builder,
        module,
        "keys",
        vec![store_type()],
        StdType::List(Box::new(StdType::Var("K".to_owned()))),
        "List keys from a persistent store.",
        intrinsic::runtime::MEMORY_KEYS,
    );
    register_store_flow(
        builder,
        module,
        "select",
        vec![store_type(), StdType::Var("Q".to_owned())],
        StdType::MemorySelection(Box::new(StdType::Var("V".to_owned()))),
        "Build a typed persistent-memory selection from a store.",
        intrinsic::runtime::MEMORY_SELECT,
    );
    register_store_flow(
        builder,
        module,
        "query",
        vec![store_type(), StdType::Var("Q".to_owned())],
        StdType::MemorySelection(Box::new(StdType::Var("V".to_owned()))),
        "Build a typed persistent-memory query from a store.",
        intrinsic::runtime::MEMORY_QUERY,
    );
    register_store_flow(
        builder,
        module,
        "scan",
        vec![store_type()],
        StdType::MemorySelection(Box::new(StdType::Var("V".to_owned()))),
        "Build a typed persistent-memory scan over a store.",
        intrinsic::runtime::MEMORY_SCAN,
    );
    register_store_flow(
        builder,
        module,
        "related_to",
        vec![store_type(), StdType::Var("Q".to_owned())],
        StdType::MemorySelection(Box::new(StdType::Var("V".to_owned()))),
        "Build a retrieval-oriented memory selection related to the given query value.",
        intrinsic::runtime::MEMORY_RELATED_TO,
    );
    builder.symbol(
        module,
        "limit",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure(
            "limit",
            &["MemorySelection[V]", "std.runtime.limits.Limit"],
            "MemorySelection[V]",
        )),
        "Limit a typed persistent-memory selection without performing additional host actions.",
    );
    register_store_flow(
        builder,
        module,
        "put",
        vec![
            store_type(),
            StdType::Var("K".to_owned()),
            StdType::Var("V".to_owned()),
        ],
        StdType::Primitive(crate::StdPrimitiveType::Unit),
        "Write a typed value into a persistent store.",
        intrinsic::runtime::MEMORY_PUT,
    );
    register_store_flow(
        builder,
        module,
        "put_versioned",
        vec![
            store_type(),
            StdType::Var("K".to_owned()),
            StdType::Var("V".to_owned()),
            StdType::Named("MemoryVersion".to_owned()),
        ],
        StdType::Primitive(crate::StdPrimitiveType::Unit),
        "Write a typed value if the persistent store entry still has the expected version.",
        intrinsic::runtime::MEMORY_PUT_VERSIONED,
    );
    register_store_flow(
        builder,
        module,
        "insert",
        vec![
            store_type(),
            StdType::Var("K".to_owned()),
            StdType::Var("V".to_owned()),
        ],
        StdType::Primitive(crate::StdPrimitiveType::Unit),
        "Insert a typed value into a persistent store.",
        intrinsic::runtime::MEMORY_INSERT,
    );
    register_store_flow(
        builder,
        module,
        "update",
        vec![
            store_type(),
            StdType::Var("K".to_owned()),
            StdType::Var("V".to_owned()),
        ],
        StdType::Primitive(crate::StdPrimitiveType::Unit),
        "Update a typed value in a persistent store.",
        intrinsic::runtime::MEMORY_UPDATE,
    );
    register_store_flow(
        builder,
        module,
        "upsert",
        vec![
            store_type(),
            StdType::Var("K".to_owned()),
            StdType::Var("V".to_owned()),
        ],
        StdType::Primitive(crate::StdPrimitiveType::Unit),
        "Upsert a typed value in a persistent store.",
        intrinsic::runtime::MEMORY_UPSERT,
    );
    register_store_flow(
        builder,
        module,
        "delete",
        vec![store_type(), StdType::Var("K".to_owned())],
        StdType::Primitive(crate::StdPrimitiveType::Unit),
        "Delete a typed value from a persistent store.",
        intrinsic::runtime::MEMORY_DELETE,
    );
    register_store_flow(
        builder,
        module,
        "delete_versioned",
        vec![
            store_type(),
            StdType::Var("K".to_owned()),
            StdType::Named("MemoryVersion".to_owned()),
        ],
        StdType::Primitive(crate::StdPrimitiveType::Unit),
        "Delete a typed value if the persistent store entry still has the expected version.",
        intrinsic::runtime::MEMORY_DELETE_VERSIONED,
    );
    register_store_flow(
        builder,
        module,
        "clear",
        vec![store_type()],
        StdType::Primitive(crate::StdPrimitiveType::Unit),
        "Clear a persistent store.",
        intrinsic::runtime::MEMORY_CLEAR,
    );
}

fn store_type() -> StdType {
    StdType::Store {
        key: Box::new(StdType::Var("K".to_owned())),
        value: Box::new(StdType::Var("V".to_owned())),
    }
}

fn register_store_flow(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    params: Vec<StdType>,
    output: StdType,
    docs: &str,
    intrinsic_id: u32,
) {
    builder.symbol_with_intrinsic(
        module,
        name,
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl {
            name: name.to_owned(),
            type_params: vec![
                crate::StdGenericParam::new("K"),
                crate::StdGenericParam::new("V"),
            ],
            params,
            output,
            public_effects: Vec::new(),
            requested_actions: memory_effects(name),
            source_method: None,
        }),
        docs,
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic_id),
            qualified_path: vec!["std".into(), "memory".into(), name.to_owned()],
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: memory_access(name),
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}

fn memory_effects(name: &str) -> Vec<StdEffectRef> {
    match name {
        "get" | "contains" | "keys" | "select" | "query" | "scan" | "related_to" => {
            vec![store_action("read")]
        }
        "put" | "put_versioned" | "insert" | "delete" | "delete_versioned" | "update" | "clear" => {
            vec![store_action("write")]
        }
        "upsert" => vec![store_action("read"), store_action("write")],
        _ => unreachable!("unknown std.memory flow `{name}`"),
    }
}

fn store_action(action: &str) -> StdEffectRef {
    StdEffectRef::with_args(
        &["Memory", action],
        vec![StdStaticArg::path(&["std", "memory", "Store"])],
    )
}

fn memory_access(name: &str) -> IntrinsicMemoryAccess {
    match name {
        "get" | "contains" | "keys" | "select" | "query" | "scan" | "related_to" => {
            IntrinsicMemoryAccess::ReadFirstArgStore
        }
        "put" | "put_versioned" | "insert" | "delete" | "delete_versioned" | "update" | "clear" => {
            IntrinsicMemoryAccess::WriteFirstArgStore
        }
        "upsert" => IntrinsicMemoryAccess::ReadWriteFirstArgStore,
        _ => unreachable!("unknown std.memory flow `{name}`"),
    }
}
