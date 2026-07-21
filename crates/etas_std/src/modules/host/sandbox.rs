use crate::{StdDecl, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, ValueDecl};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "host", "sandbox"], "Command sandbox support.");
    let profile = builder.symbol(
        module,
        "SandboxProfile",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic(
            "SandboxProfile",
            &[],
            TypeDeclKind::Support,
        )),
        "Compiler-known support type for command sandbox profiles.",
    );
    builder.prelude("SandboxProfile", profile);

    let default = builder.symbol(
        module,
        "DefaultCommandSandbox",
        StdSymbolKind::Value,
        StdDecl::Value(ValueDecl::new(
            "DefaultCommandSandbox",
            "std.host.sandbox.SandboxProfile",
        )),
        "Default command sandbox descriptor.",
    );
    builder.prelude("DefaultCommandSandbox", default);
}
