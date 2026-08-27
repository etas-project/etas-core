use crate::{
    EffectActionArgKind, EffectActionDecl, StdDecl, StdGenericParam, StdRegistryBuilder,
    StdSymbolKind, TypeDecl, TypeDeclKind,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "runtime", "error"],
        "Runtime error effect support declarations.",
    );
    builder.symbol(
        module,
        "raise",
        StdSymbolKind::EffectAction,
        StdDecl::EffectAction(
            EffectActionDecl::local("Error", "raise", &["E"], "never")
                .with_effect_args(&[EffectActionArgKind::Type])
                .with_type_params(&[StdGenericParam::new("E")])
                .with_selector_param_names(&["E"])
                .with_stable_id(33),
        ),
        "Raise a typed error effect.",
    );
    for name in [
        "IndexError",
        "IOError",
        "SchemaError",
        "ValidationError",
        "ToolError",
        "ToolTimeout",
        "ToolDenied",
        "PolicyViolation",
        "PolicyDenied",
        "EffectBoundaryViolation",
        "SandboxViolation",
        "PromptInjectionRisk",
        "MissingCitation",
        "BudgetExceeded",
        "ProtocolViolation",
        "HumanRejected",
    ] {
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Standard runtime error support type.",
        );
        if name != "IOError" {
            builder.prelude(name, symbol);
        }
    }
}
