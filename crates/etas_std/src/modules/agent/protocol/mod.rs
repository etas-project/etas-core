mod decision;
mod tool;
mod turn;

use crate::{
    StdDecl, StdModuleId, StdRecordField, StdRegistryBuilder, StdSymbolId, StdSymbolKind, StdType,
    TypeDecl, TypeDeclKind,
};

pub(super) fn register(builder: &mut StdRegistryBuilder) -> (StdModuleId, StdSymbolId) {
    let module = builder.module(
        &["std", "agent"],
        "Checked agent interaction and transparent protocol values.",
    );
    turn::register(builder, module);
    let tool_ref = tool::register(builder, module);
    decision::register(builder, module);
    (module, tool_ref)
}

fn record(
    builder: &mut StdRegistryBuilder,
    module: StdModuleId,
    name: &str,
    params: &[&str],
    fields: Vec<StdRecordField>,
) -> StdSymbolId {
    builder.symbol(
        module,
        name,
        StdSymbolKind::Type,
        StdDecl::Type(
            TypeDecl::generic(name, params, TypeDeclKind::Struct)
                .with_representation(StdType::Record(fields)),
        ),
        "Transparent agent protocol data; not authority or execution evidence.",
    )
}

fn field(name: &str, ty: &str) -> StdRecordField {
    StdRecordField::new(name, StdType::parse(ty))
}
