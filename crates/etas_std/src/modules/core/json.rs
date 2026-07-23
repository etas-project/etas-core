use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, intrinsic,
};

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
    builder.symbol_with_intrinsic(
        module,
        "InvalidJson",
        StdSymbolKind::Constructor,
        StdDecl::Flow(FlowDecl::pure("InvalidJson", &["string"], "JsonError")),
        "JSON parse/stringify error carrying a human-readable diagnostic message.",
        Some(runtime_pure_descriptor(
            intrinsic::pure::JSON_INVALID_JSON,
            &["std", "json", "InvalidJson"],
        )),
    );
    builder.symbol_with_intrinsic(
        module,
        "parse",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure(
            "parse",
            &["string"],
            "Result[JsonValue, JsonError]",
        )),
        "Parse JSON into a structured value or return a JSON error.",
        Some(runtime_pure_descriptor(
            intrinsic::pure::JSON_PARSE,
            &["std", "json", "parse"],
        )),
    );
    builder.symbol_with_intrinsic(
        module,
        "stringify",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure(
            "stringify",
            &["JsonValue"],
            "Result[string, JsonError]",
        )),
        "Stringify a structured JSON value or return a JSON error.",
        Some(runtime_pure_descriptor(
            intrinsic::pure::JSON_STRINGIFY,
            &["std", "json", "stringify"],
        )),
    );
}

fn runtime_pure_descriptor(id: u32, path: &[&str]) -> IntrinsicDescriptor {
    IntrinsicDescriptor {
        id: StdIntrinsicId(id),
        qualified_path: path.iter().map(|segment| (*segment).to_owned()).collect(),
        purity: IntrinsicPurity::Pure,
        dispatch: IntrinsicDispatch::Runtime,
        lowering: LoweringHint::RuntimeCall,
        latent_effect: crate::IntrinsicLatentEffect::None,
        memory_access: crate::IntrinsicMemoryAccess::None,
        runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
    }
}
