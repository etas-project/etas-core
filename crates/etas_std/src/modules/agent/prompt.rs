use crate::{
    FlowDecl, StdDecl, StdGenericParam, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "agent", "prompt"], "Prompt construction support.");
    for (name, params) in [
        ("Prompt", &[][..]),
        ("PromptPart", &[][..]),
        ("PromptEncode", &[][..]),
    ] {
        let kind = if name == "PromptEncode" {
            TypeDeclKind::Spec
        } else {
            TypeDeclKind::Support
        };
        let mut decl = TypeDecl::generic(name, params, kind);
        if name == "PromptEncode" {
            decl = decl.derivable();
        }
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(decl),
            "Agent prompt support declaration.",
        );
        if matches!(name, "Prompt" | "PromptPart" | "PromptEncode") {
            builder.prelude(name, symbol);
        }
    }

    for (name, params, output, docs) in [
        ("new", &[][..], "Prompt", "Construct an empty prompt value."),
        (
            "system",
            &["Prompt", "Trusted[string]"][..],
            "Prompt",
            "Append trusted system-channel content to a prompt value.",
        ),
        (
            "user",
            &["Prompt", "Public[string]"][..],
            "Prompt",
            "Append user-channel content to a prompt value.",
        ),
        (
            "assistant",
            &["Prompt", "string"][..],
            "Prompt",
            "Append assistant-channel content to a prompt value.",
        ),
        (
            "data",
            &["Prompt", "T"][..],
            "Prompt",
            "Append data-channel content to a prompt value.",
        ),
    ] {
        let declaration = if name == "data" {
            FlowDecl::with_type_params_actions(
                name,
                &[StdGenericParam::new("T")],
                params,
                output,
                &[],
                &[],
            )
        } else {
            FlowDecl::pure(name, params, output)
        };
        builder.symbol(
            module,
            name,
            StdSymbolKind::Flow,
            StdDecl::Flow(declaration),
            docs,
        );
    }
}
