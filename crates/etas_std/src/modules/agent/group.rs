use crate::{FlowDecl, StdDecl, StdEffectRef, StdGenericParam, StdRegistryBuilder, StdSymbolKind};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "agent", "group"],
        "Higher-level multi-agent group combinator declarations.",
    );
    builder.symbol(
        module,
        "round_robin",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            "round_robin",
            &[StdGenericParam::new("T")],
            &["List[T]"],
            "T",
            &[StdEffectRef::new(&["Agentic"])],
            &[],
        )),
        "Round-robin agent group combinator descriptor.",
    );
}
