use crate::{StdDecl, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "host", "url"], "Host URL support types.");
    builder.symbol(
        module,
        "Url",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Url", &[], TypeDeclKind::Support)),
        "Host URL support type.",
    );
}
