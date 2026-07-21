use crate::{StdDecl, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "agent", "schema"],
        "Agent schema and model response decoding support.",
    );

    for (name, params) in [
        ("Schema", &["T"][..]),
        ("ResponseDecode", &["T"][..]),
        ("ModelResponse", &[][..]),
    ] {
        let mut decl = TypeDecl::generic(name, params, TypeDeclKind::Support);
        if matches!(name, "Schema" | "ResponseDecode") {
            decl = decl.derivable();
        }
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(decl),
            "Agent schema support declaration.",
        );
        builder.prelude(name, symbol);
    }
}
