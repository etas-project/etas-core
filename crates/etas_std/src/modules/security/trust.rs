use crate::{StdDecl, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "security", "trust"],
        "Trust, provenance, and secrecy wrappers.",
    );
    for name in ["Trusted", "Untrusted", "Secret", "Public", "Sanitized"] {
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &["T"], TypeDeclKind::Wrapper)),
            "Trust wrapper declaration.",
        );
        builder.prelude(name, symbol);
    }
}
