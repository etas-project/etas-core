use crate::{FlowDecl, StdDecl, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "json"], "Structured JSON support declarations.");
    builder.symbol(
        module,
        "JsonValue",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("JsonValue", &[], TypeDeclKind::Support)),
        "Structured JSON value support type.",
    );
    builder.symbol(
        module,
        "JsonError",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("JsonError", &[], TypeDeclKind::Support)),
        "Structured JSON parse/stringify error support type.",
    );
    builder.symbol(
        module,
        "InvalidJson",
        StdSymbolKind::Constructor,
        StdDecl::Flow(FlowDecl::pure("InvalidJson", &["string"], "JsonError")),
        "JSON parse/stringify error carrying a human-readable diagnostic message.",
    );
    builder.symbol(
        module,
        "parse",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure(
            "parse",
            &["string"],
            "Result[JsonValue, JsonError]",
        )),
        "Parse JSON into a structured value or return a JSON error.",
    );
    builder.symbol(
        module,
        "stringify",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure(
            "stringify",
            &["JsonValue"],
            "Result[string, JsonError]",
        )),
        "Stringify a structured JSON value or return a JSON error.",
    );
}
