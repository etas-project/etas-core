use super::*;

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    for name in ["WriteOutcome", "ConfirmedOutcome", "ReconcileResult"] {
        let declaration = TypeDecl::generic(name, &["R", "N"], TypeDeclKind::Enum);
        let owner = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(declaration.clone()),
            "Closed storage outcome.",
        );
        builder.prelude(name, owner);
        let operation = StdType::Named("std.memory.StorageOperationRef".into());
        let variants = if name == "ReconcileResult" {
            vec![
                ("Found", vec![applied("ConfirmedOutcome")]),
                ("Unresolved", vec![]),
                ("Expired", vec![]),
            ]
        } else {
            let mut variants = vec![
                ("Committed", vec![StdType::Var("R".into())]),
                (
                    "NotCommitted",
                    vec![operation.clone(), StdType::Var("N".into())],
                ),
            ];
            if name == "WriteOutcome" {
                variants.push(("Unknown", vec![operation]));
            }
            variants
        };
        for (variant, params) in variants {
            builder
                .enum_constructor(
                    owner,
                    FlowDecl {
                        name: variant.into(),
                        type_params: declaration.params.clone(),
                        params,
                        output: applied(name),
                        public_effects: vec![],
                        requested_actions: vec![],
                        source_method: None,
                    },
                    "Storage outcome variant.",
                )
                .expect("storage enum owner is registered");
        }
    }
}

fn applied(name: &str) -> StdType {
    StdType::NamedApplied {
        name: format!("std.memory.{name}"),
        args: vec![StdType::Var("R".into()), StdType::Var("N".into())],
    }
}
