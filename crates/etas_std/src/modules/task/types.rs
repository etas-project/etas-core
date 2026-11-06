use crate::{
    RuntimeTypeRole, StdDecl, StdGenericParam, StdRegistryBuilder, StdSymbolKind, TypeDecl,
    TypeDeclKind,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    for (name, params, role, docs) in [
        (
            "Task",
            vec![StdGenericParam::new("T"), StdGenericParam::effect_row("E")],
            RuntimeTypeRole::Task,
            "Opaque handle to a task owned by the surrounding scheduler scope.",
        ),
        (
            "Scheduler",
            vec![],
            RuntimeTypeRole::Scheduler,
            "Opaque scheduler handle valid only inside its run scope.",
        ),
        (
            "Continuation",
            vec![StdGenericParam::new("T")],
            RuntimeTypeRole::Continuation,
            "Affine continuation token consumed by a scheduler transfer.",
        ),
    ] {
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(
                TypeDecl::generic_params(name, params, TypeDeclKind::Support)
                    .runtime_role(role)
                    .intrinsic_only(),
            ),
            docs,
        );
        builder.prelude(name, symbol);
    }
}
