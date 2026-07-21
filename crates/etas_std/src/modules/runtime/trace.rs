use crate::{
    FlowDecl, StdDecl, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, ValueDecl,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "runtime", "trace"], "Trace metadata declarations.");
    for name in ["TraceLabel", "TraceRedaction", "TraceExport"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Trace support type.",
        );
    }
    let logical = builder.symbol(
        module,
        "Logical",
        StdSymbolKind::Value,
        StdDecl::Value(ValueDecl::new("Logical", "TraceLabel")),
        "Logical trace stage label.",
    );
    builder.prelude("Logical", logical);

    let virtual_stages = builder.symbol(
        module,
        "VirtualStages",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::pure(
            "VirtualStages",
            &["Array[TraceLabel]"],
            "TraceLabel",
        )),
        "Trace label for a fused agent with virtual logical stages.",
    );
    builder.prelude("VirtualStages", virtual_stages);
}
