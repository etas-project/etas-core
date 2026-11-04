use crate::{
    FlowDecl, StdDecl, StdModuleId, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: StdModuleId) {
    let owner = builder.symbol(
        module,
        "TextError",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("TextError", &[], TypeDeclKind::Enum)),
        "A checked text position, parsing, formatting, or output-length failure.",
    );
    for name in [
        "InvalidIndex",
        "InvalidRange",
        "NotCharBoundary",
        "ParseFailure",
        "NumericOverflow",
        "LengthOverflow",
        "InvalidFormat",
    ] {
        let constructor = builder
            .enum_constructor(
                owner,
                FlowDecl::pure(name, &[], "std.text.TextError"),
                "A standard text failure.",
            )
            .expect("TextError owner is registered");
        builder
            .re_export(module, name, constructor)
            .expect("TextError constructor export is unique");
    }
}
