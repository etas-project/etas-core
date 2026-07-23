use crate::{
    IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, RequirementDecl,
    StdDecl, StdIntrinsicId, StdLimitKind, StdRegistryBuilder, StdSymbolKind, TypeDecl,
    TypeDeclKind, intrinsic,
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
        let symbol = builder.symbol_with_intrinsic(
            module,
            name,
            StdSymbolKind::Requirement,
            StdDecl::Requirement(RequirementDecl::limit(name, params, kind)),
            summary,
            Some(IntrinsicDescriptor {
                id: StdIntrinsicId(limit_intrinsic_id(kind)),
                qualified_path: vec![
                    "std".into(),
                    "runtime".into(),
                    "limits".into(),
                    name.to_owned(),
                ],
                purity: IntrinsicPurity::Pure,
                dispatch: IntrinsicDispatch::Runtime,
                lowering: LoweringHint::RuntimeCall,
                latent_effect: crate::IntrinsicLatentEffect::None,
                memory_access: crate::IntrinsicMemoryAccess::None,
                runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
            }),
        );
        builder.prelude(name, symbol);
    }
}

fn limit_intrinsic_id(kind: StdLimitKind) -> u32 {
    match kind {
        StdLimitKind::Iterations => intrinsic::pure::LIMIT_ITERATIONS,
        StdLimitKind::Tokens => intrinsic::pure::LIMIT_TOKENS,
        StdLimitKind::ContextTokens => intrinsic::pure::LIMIT_CONTEXT_TOKENS,
        StdLimitKind::Cost => intrinsic::pure::LIMIT_COST,
        StdLimitKind::WallTime => intrinsic::pure::LIMIT_WALL_TIME,
        StdLimitKind::Attempts => intrinsic::pure::LIMIT_ATTEMPTS,
    }
}
