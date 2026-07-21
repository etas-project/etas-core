use crate::{FlowDecl, StdDecl, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "host", "command"], "Host command support types.");
    for name in ["Command", "CommandResult", "CommandHandle"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Host command support type.",
        );
    }

    builder.symbol(
        module,
        "run",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_actions(
            "run",
            &["Command", "SandboxProfile"],
            "CommandResult",
            &[],
            &["Command.run[_]"],
        )),
        "Run a command through the checked command host boundary.",
    );
}
