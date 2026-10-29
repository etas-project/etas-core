use etas_std::{
    CodeTypeRole, StdDecl, StdRegistryBuilder, StdRegistryVersion, StdSymbolKind,
    StdTypeConstruction, TypeDecl, TypeDeclKind, standard_registry,
};

#[test]
fn compiler_descriptor_types_are_opaque_and_have_unique_registry_roles() {
    let registry = standard_registry();
    assert_eq!(
        registry
            .modules()
            .filter(|module| module.path == ["std", "compiler"])
            .count(),
        1
    );
    for (name, role, arity) in [
        ("AgentDef", CodeTypeRole::AgentDefinition, 1),
        ("CallableDef", CodeTypeRole::CallableDefinition, 1),
        ("SyntaxBlock", CodeTypeRole::SyntaxBlock, 0),
    ] {
        let symbol = registry
            .lookup_qualified(&["std", "compiler", name])
            .unwrap();
        let StdDecl::Type(ty) = &symbol.decl else {
            panic!("type")
        };
        assert_eq!(ty.code_role, Some(role));
        assert_eq!(ty.params.len(), arity);
        assert_eq!(ty.kind, TypeDeclKind::Struct);
        assert_eq!(ty.construction, StdTypeConstruction::IntrinsicOnly);
        assert!(ty.representation.is_none());
        let expected = match role {
            CodeTypeRole::AgentDefinition => vec![
                "body",
                "signature",
                "effects",
                "determinism",
                "trace",
                "identity",
                "revision",
                "dependencies",
                "annotations",
                "tools",
                "specs",
                "methods",
            ],
            CodeTypeRole::CallableDefinition => {
                vec![
                    "body",
                    "id",
                    "signature",
                    "effects",
                    "determinism",
                    "trace",
                    "revision",
                    "dependencies",
                ]
            }
            CodeTypeRole::SyntaxBlock => vec!["text"],
            _ => unreachable!(),
        };
        assert_eq!(
            ty.projected_fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            expected.into_iter().collect()
        );
        if matches!(
            role,
            CodeTypeRole::AgentDefinition | CodeTypeRole::CallableDefinition
        ) {
            let revision = ty
                .projected_fields
                .iter()
                .find(|field| field.name == "revision")
                .unwrap();
            assert_eq!(
                revision.ty,
                etas_std::StdType::Named("std.compiler.RevisionInfo".into())
            );
        }
        if role == CodeTypeRole::SyntaxBlock {
            assert_eq!(ty.projected_fields[0].name, "text");
            assert_eq!(
                ty.projected_fields[0].ty,
                etas_std::StdType::parse("string")
            );
        } else {
            assert_eq!(ty.projected_fields[0].name, "body");
            assert_eq!(
                ty.projected_fields[0].ty,
                etas_std::StdType::Option(Box::new(etas_std::StdType::Named(
                    "std.compiler.SyntaxBlock".into()
                )))
            );
        }
    }
    for (name, role) in [
        ("AgentId", CodeTypeRole::AgentId),
        ("TypeInfo", CodeTypeRole::TypeInfo),
        ("GenericOwnerInfo", CodeTypeRole::GenericOwnerInfo),
        ("ToolRef", CodeTypeRole::ToolRef),
        ("StaticValueInfo", CodeTypeRole::StaticValueInfo),
        ("TraceConstraintInfo", CodeTypeRole::TraceConstraintInfo),
    ] {
        let symbol = registry
            .lookup_qualified(&["std", "compiler", name])
            .unwrap();
        let StdDecl::Type(ty) = &symbol.decl else {
            panic!("descriptor type");
        };
        assert_eq!(ty.code_role, Some(role));
        assert_eq!(ty.construction, StdTypeConstruction::IntrinsicOnly);
        assert!(ty.representation.is_none());
    }
}

#[test]
fn registry_rejects_wrong_descriptor_arity_public_construction_and_duplicate_roles() {
    for invalid in [
        TypeDecl::generic("Broken", &[], TypeDeclKind::Struct)
            .code_role(CodeTypeRole::AgentDefinition)
            .intrinsic_only(),
        TypeDecl::generic("Broken", &[], TypeDeclKind::Struct)
            .code_role(CodeTypeRole::CallableDefinition)
            .intrinsic_only(),
        TypeDecl::generic("Broken", &[], TypeDeclKind::Support)
            .code_role(CodeTypeRole::SyntaxBlock)
            .intrinsic_only(),
        TypeDecl::generic("Broken", &[], TypeDeclKind::Struct)
            .with_representation(etas_std::StdType::Record(vec![]))
            .code_role(CodeTypeRole::SyntaxBlock)
            .intrinsic_only(),
        TypeDecl::generic("Broken", &[], TypeDeclKind::Struct).code_role(CodeTypeRole::SyntaxBlock),
    ] {
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        let module = builder.module(&["std", "probe"], "probe");
        builder.symbol(
            module,
            "Broken",
            StdSymbolKind::Type,
            StdDecl::Type(invalid),
            "invalid",
        );
        assert!(builder.try_finish().is_err());
    }
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "probe");
    for name in ["One", "Two"] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(
                TypeDecl::generic(name, &[], TypeDeclKind::Struct)
                    .code_role(CodeTypeRole::SyntaxBlock)
                    .intrinsic_only(),
            ),
            "duplicate",
        );
    }
    assert!(builder.try_finish().is_err());
}

#[test]
fn contextual_inference_is_backed_by_a_unique_checked_standard_action_role() {
    let registry = standard_registry();
    let actions = registry
        .symbols()
        .filter_map(|symbol| match &symbol.decl {
            StdDecl::EffectAction(action)
                if action.compiler_role == Some(etas_std::CompilerActionRole::AgentInference) =>
            {
                Some(action)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(actions.len(), 1);
    let action = actions[0];
    assert_eq!(action.params[0], etas_std::StdType::Prompt);
    assert_eq!(
        action.params[1],
        etas_std::StdType::Schema(Box::new(action.output.clone()))
    );
    assert_eq!(action.type_params.len(), 1);
    assert_eq!(action.effect_args.len(), 2);
    assert_eq!(action.selector_param_names, ["C", "O"]);

    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "invalid"], "invalid compiler action");
    builder.symbol(
        module,
        "infer",
        StdSymbolKind::EffectAction,
        StdDecl::EffectAction(
            etas_std::EffectActionDecl::new("Wrong", "infer", &["string"], "string")
                .with_compiler_role(etas_std::CompilerActionRole::AgentInference),
        ),
        "invalid",
    );
    let error = builder.try_finish().unwrap_err();
    assert!(error.reason.contains("Prompt/Schema<O> ABI"), "{error:?}");
}
