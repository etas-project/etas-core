use crate::{
    RequirementDecl, StdDecl, StdLimitKind, StdRegistryBuilder, StdSymbolKind, TypeDecl,
    TypeDeclKind,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "runtime", "limits"],
        "Loop, retry, budget, and runtime limit support.",
    );
    let limit = builder.symbol(
        module,
        "Limit",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Limit", &[], TypeDeclKind::Support)),
        "Compiler-known support type for runtime limit constructors.",
    );
    builder.prelude("Limit", limit);

    for (name, params, kind, summary) in [
        (
            "Iterations",
            &["usize"][..],
            StdLimitKind::Iterations,
            "Limit the number of loop iterations.",
        ),
        (
            "Tokens",
            &["usize"][..],
            StdLimitKind::Tokens,
            "Limit model input and output token usage.",
        ),
        (
            "ContextTokens",
            &["usize"][..],
            StdLimitKind::ContextTokens,
            "Limit model context window usage.",
        ),
        (
            "Cost",
            &["Money"][..],
            StdLimitKind::Cost,
            "Limit monetary budget usage.",
        ),
        (
            "WallTime",
            &["Duration"][..],
            StdLimitKind::WallTime,
            "Limit wall-clock duration.",
        ),
        (
            "Attempts",
            &["usize"][..],
            StdLimitKind::Attempts,
            "Limit retry attempts.",
        ),
    ] {
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Requirement,
            StdDecl::Requirement(RequirementDecl::limit(name, params, kind)),
            summary,
        );
        builder.prelude(name, symbol);
    }
}
