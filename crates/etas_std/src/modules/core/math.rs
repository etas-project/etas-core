use crate::{FlowDecl, StdDecl, StdGenericParam, StdRegistryBuilder, StdSymbolKind};

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
            StdDecl::Flow(FlowDecl::with_type_params_actions(
                name,
                &[StdGenericParam::new("T")],
                &["T"],
                "T",
                &[],
                &[],
            )),
            "Deterministic numeric helper.",
        );
    }
}
