use crate::{
    FlowDecl, StdDecl, StdModuleId, StdRecordField, StdRegistryBuilder, StdSymbolKind, StdType,
    TypeDecl, TypeDeclKind,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    builder.symbol(
        module,
        "ByteStream",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("ByteStream", &[], TypeDeclKind::Spec)),
        "Standard byte-stream marker spec.",
    );
    for (name, field) in [("ByteLimit", "bytes"), ("Timeout", "ms")] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(
                TypeDecl::generic(name, &[], TypeDeclKind::Support).with_representation(
                    StdType::Record(vec![StdRecordField::new(field, StdType::parse("i32"))]),
                ),
            ),
            "Bounded stream operation configuration.",
        );
    }
    let read = builder.symbol(
        module,
        "StreamRead",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("StreamRead", &[], TypeDeclKind::Enum)),
        "A byte chunk or ordinary end of stream.",
    );
    for (name, fields) in [("Data", &["bytes"][..]), ("Eof", &[][..])] {
        builder
            .enum_constructor(
                read,
                FlowDecl::pure(name, fields, "std.stream.StreamRead"),
                "Standard stream read result.",
            )
            .expect("StreamRead owner is registered");
    }
    let error = builder.symbol(
        module,
        "StreamError",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("StreamError", &[], TypeDeclKind::Enum)),
        "Typed stream operation failure.",
    );
    for (name, fields) in [
        ("TimedOut", &[][..]),
        ("Cancelled", &[][..]),
        ("Closed", &[][..]),
        ("Interrupted", &[][..]),
        ("LimitExceeded", &[][..]),
        ("Host", &["string"][..]),
    ] {
        let constructor = builder
            .enum_constructor(
                error,
                FlowDecl::pure(name, fields, "std.stream.StreamError"),
                "Standard stream error variant.",
            )
            .expect("StreamError owner is registered");
        builder
            .re_export(module, name, constructor)
            .expect("stream error export is unique");
    }
}
