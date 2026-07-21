use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "text"], "Deterministic text helper declarations.");
    builder.symbol(
        module,
        "ParseError",
        StdSymbolKind::Type,
        StdDecl::Type(crate::TypeDecl::generic(
            "ParseError",
            &[],
            crate::TypeDeclKind::Support,
        )),
        "Standard text parsing error support type.",
    );
    text_intrinsic(
        builder,
        module,
        "len",
        &["string"],
        "usize",
        intrinsic::pure::TEXT_LEN,
        "Return the number of characters in a string.",
    );
    text_intrinsic(
        builder,
        module,
        "trim",
        &["string"],
        "string",
        intrinsic::pure::TEXT_TRIM,
        "Trim leading and trailing whitespace.",
    );
    text_intrinsic(
        builder,
        module,
        "lowercase",
        &["string"],
        "string",
        intrinsic::pure::TEXT_LOWERCASE,
        "Convert text to lowercase.",
    );
    text_intrinsic(
        builder,
        module,
        "uppercase",
        &["string"],
        "string",
        intrinsic::pure::TEXT_UPPERCASE,
        "Convert text to uppercase.",
    );
    text_intrinsic(
        builder,
        module,
        "contains",
        &["string", "string"],
        "bool",
        intrinsic::pure::TEXT_CONTAINS,
        "Return whether one string contains another.",
    );
    text_intrinsic(
        builder,
        module,
        "starts_with",
        &["string", "string"],
        "bool",
        intrinsic::pure::TEXT_STARTS_WITH,
        "Return whether one string starts with another.",
    );
    text_intrinsic(
        builder,
        module,
        "ends_with",
        &["string", "string"],
        "bool",
        intrinsic::pure::TEXT_ENDS_WITH,
        "Return whether one string ends with another.",
    );
    text_intrinsic(
        builder,
        module,
        "lines",
        &["string"],
        "Array[string]",
        intrinsic::pure::TEXT_LINES,
        "Split a string into lines.",
    );
    text_intrinsic(
        builder,
        module,
        "split",
        &["string", "string"],
        "Array[string]",
        intrinsic::pure::TEXT_SPLIT,
        "Split a string by a separator.",
    );
    text_intrinsic(
        builder,
        module,
        "join",
        &["Array[string]", "string"],
        "string",
        intrinsic::pure::TEXT_JOIN,
        "Join string parts with a separator.",
    );
    text_intrinsic(
        builder,
        module,
        "to_string_i32",
        &["i32"],
        "string",
        intrinsic::pure::TEXT_TO_STRING_I32,
        "Format an i32 as string.",
    );
    text_intrinsic(
        builder,
        module,
        "to_string_usize",
        &["usize"],
        "string",
        intrinsic::pure::TEXT_TO_STRING_USIZE,
        "Format a usize as string.",
    );
    text_intrinsic(
        builder,
        module,
        "parse_i32",
        &["string"],
        "Result[i32, ParseError]",
        intrinsic::pure::TEXT_PARSE_I32,
        "Parse a string into an i32.",
    );
}

fn text_intrinsic(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    params: &[&str],
    output: &str,
    id: u32,
    summary: &str,
) {
    builder.symbol_with_intrinsic(
        module,
        name,
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure(name, params, output)),
        summary,
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(id),
            qualified_path: vec!["std".into(), "text".into(), name.into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::PureKernel,
            lowering: LoweringHint::PureBuiltin,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
