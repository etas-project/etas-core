use crate::{
    FlowDecl, StdDecl, StdModuleId, StdRecordField, StdRegistryBuilder, StdSymbolKind, StdType,
    TypeDecl, TypeDeclKind,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    let content = builder.symbol(
        module,
        "MessageContent",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("MessageContent", &[], TypeDeclKind::Enum)),
        "Assistant text or shared JSON content.",
    );
    for (name, ty) in [("Text", "string"), ("Value", "std.json.JsonValue")] {
        builder
            .enum_constructor(
                content,
                FlowDecl::pure(name, &[ty], "std.agent.MessageContent"),
                "Assistant content constructor.",
            )
            .expect("MessageContent owner is registered");
    }
    super::record(
        builder,
        module,
        "AssistantTurn",
        &[],
        vec![
            super::field("request_id", "Option[string]"),
            super::field("content", "Array[std.agent.MessageContent]"),
            super::field("provider_data", "Map[string, std.json.JsonValue]"),
        ],
    );
    super::record(
        builder,
        module,
        "Usage",
        &[],
        vec![
            super::field("input_tokens", "Option[u64]"),
            super::field("output_tokens", "Option[u64]"),
            StdRecordField::new(
                "cost",
                StdType::Option(Box::new(StdType::Record(vec![
                    super::field("micros", "u128"),
                    super::field("currency", "string"),
                ]))),
            ),
        ],
    );
}
