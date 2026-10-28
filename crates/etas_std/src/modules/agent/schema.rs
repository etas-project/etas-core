use crate::{StdDecl, StdRegistryBuilder, StdSymbolKind, StdType, TypeDecl, TypeDeclKind};

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
        if name == "Schema" {
            decl = decl.with_canonical_type(StdType::Schema(Box::new(StdType::Var("T".into()))));
        }
        if name == "ModelResponse" {
            decl = decl.runtime_role(crate::RuntimeTypeRole::ModelResponse);
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
