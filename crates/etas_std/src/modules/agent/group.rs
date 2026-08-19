use crate::{FlowDecl, StdDecl, StdEffectRef, StdRegistryBuilder, StdSymbolKind};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "agent", "group"],
        "Higher-level multi-agent group combinator declarations.",
    );
    builder.symbol(
        module,
        "round_robin",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::effectful(
            "round_robin",
            &["List[T]"],
            "T",
            &[StdEffectRef::new(&["Agentic"])],
        )),
        "Round-robin agent group combinator descriptor.",
    );
}
