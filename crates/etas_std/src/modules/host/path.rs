use crate::{StdDecl, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "host", "path"], "Host path support types.");
    for name in ["Path", "PathPattern"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Host path support type.",
        );
    }
}
