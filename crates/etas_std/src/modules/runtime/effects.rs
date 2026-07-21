use crate::{EffectDecl, StdDecl, StdRegistryBuilder, StdSymbolKind};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "runtime", "effects"], "Core effect declarations.");
    for name in [
        "Agentic", "Network", "FileIO", "Command", "Memory", "Secret", "Time", "Human",
    ] {
        let decl = EffectDecl::core(name);
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Effect,
            StdDecl::Effect(decl),
            "Core Etas effect declaration.",
        );
        builder.prelude(name, symbol);
    }
    let error = builder.symbol(
        module,
        "Error",
        StdSymbolKind::Effect,
        StdDecl::Effect(EffectDecl::generic_core("Error", &["E"])),
        "Generic error effect declaration.",
    );
    builder.prelude("Error", error);
}
