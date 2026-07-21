use crate::{FlowDecl, StdDecl, StdRegistryBuilder, StdSymbolKind};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "math"],
        "Deterministic numeric helper declarations.",
    );
    for name in ["min", "max", "abs"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Flow,
            StdDecl::Flow(FlowDecl::pure(name, &["T"], "T")),
            "Deterministic numeric helper.",
        );
    }
}
