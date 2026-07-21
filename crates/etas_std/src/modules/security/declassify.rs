use crate::{FlowDecl, StdDecl, StdRegistryBuilder, StdSymbolKind};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "security", "declassify"],
        "Explicit sanitization and declassification declarations.",
    );
    for name in ["sanitize", "declassify"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Flow,
            StdDecl::Flow(FlowDecl::effectful(name, &["T"], "T", &["Secret"])),
            "Security declassification support descriptor.",
        );
    }
}
