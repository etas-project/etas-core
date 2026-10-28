use etas_std::{
    RuntimeFieldRole, RuntimeTypeRole, StdDecl, StdPrimitiveType, StdRegistryBuilder,
    StdRegistryVersion, StdSymbolKind, StdType, StdTypeConstruction, TypeDecl, TypeDeclKind,
    standard_registry,
};

#[test]
fn native_runtime_roles_cover_the_declared_value_abis_without_name_inference() {
    let registry = standard_registry();
    let roles = registry
        .symbols()
        .filter_map(|symbol| {
            let StdDecl::Type(declaration) = &symbol.decl else {
                return None;
            };
            declaration.runtime_role.map(|role| (role, declaration))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    for (role, arity) in [
        (RuntimeTypeRole::Command, 0),
        (RuntimeTypeRole::CommandResult, 0),
        (RuntimeTypeRole::JsonValue, 0),
        (RuntimeTypeRole::Conversation, 0),
        (RuntimeTypeRole::Provenance, 0),
        (RuntimeTypeRole::ModelResponse, 0),
        (RuntimeTypeRole::Deque, 1),
        (RuntimeTypeRole::Queue, 1),
        (RuntimeTypeRole::Stack, 1),
        (RuntimeTypeRole::PriorityQueue, 2),
        (RuntimeTypeRole::OrderedMap, 2),
        (RuntimeTypeRole::OrderedSet, 1),
        (RuntimeTypeRole::WorkspacePath, 1),
        (RuntimeTypeRole::MemoryWriteIntent, 2),
    ] {
        let declaration = roles.get(&role).expect("registered native ABI role");
        assert_eq!(role.type_parameter_count(), arity);
        assert_eq!(declaration.params.len(), arity);
    }
    assert_eq!(roles.len(), 14);
    assert!(
        roles[&RuntimeTypeRole::Conversation]
            .representation
            .is_some()
    );
}

#[test]
fn native_runtime_roles_reject_wrong_generic_kind_and_duplicates() {
    for defect in 0..5 {
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        let module = builder.module(&["std", "probe"], "native ABI probe");
        let mut decl = TypeDecl::generic("Native", &["T"], TypeDeclKind::Support)
            .runtime_role(RuntimeTypeRole::Deque);
        match defect {
            0 => decl.params.clear(),
            1 => decl.kind = TypeDeclKind::Struct,
            2 => decl.representation = Some(StdType::parse("T")),
            3 => decl = decl.derivable(),
            4 => {
                decl = decl.with_projected_fields(vec![etas_std::StdRecordField::new(
                    "unexpected",
                    StdType::parse("string"),
                )])
            }
            _ => unreachable!(),
        }
        builder.symbol(
            module,
            "Native",
            StdSymbolKind::Type,
            StdDecl::Type(decl),
            "invalid",
        );
        assert!(
            builder
                .try_finish()
                .unwrap_err()
                .reason
                .contains("native runtime type role")
        );
    }
}

#[test]
fn command_result_exposes_typed_read_only_bytes_without_a_record_constructor() {
    let registry = standard_registry();
    let result = registry
        .lookup_qualified(&["std", "host", "command", "CommandResult"])
        .unwrap();
    let StdDecl::Type(result) = &result.decl else {
        panic!("type")
    };
    assert_eq!(result.runtime_role, Some(RuntimeTypeRole::CommandResult));
    assert_eq!(result.construction, StdTypeConstruction::IntrinsicOnly);
    assert!(result.representation.is_none());
    assert_eq!(
        result.projected_fields,
        RuntimeTypeRole::CommandResult.projected_fields()
    );
    assert_eq!(
        RuntimeTypeRole::CommandResult
            .fields()
            .iter()
            .map(|field| (field.name, field.ty, field.role))
            .collect::<Vec<_>>(),
        [
            (
                "exit_code",
                StdPrimitiveType::I32,
                RuntimeFieldRole::CommandExitCode
            ),
            (
                "stdout",
                StdPrimitiveType::Bytes,
                RuntimeFieldRole::CommandStdout
            ),
            (
                "stderr",
                StdPrimitiveType::Bytes,
                RuntimeFieldRole::CommandStderr
            ),
        ]
    );
}

#[test]
fn native_runtime_roles_reject_ambiguous_identity_or_changed_abi() {
    let valid = || {
        TypeDecl::generic("Result", &[], TypeDeclKind::Support)
            .runtime_role(RuntimeTypeRole::CommandResult)
            .intrinsic_only()
    };
    for defect in 0..6 {
        let mut declaration = valid();
        match defect {
            0 => declaration.construction = StdTypeConstruction::Public,
            1 => declaration.representation = Some(StdType::parse("string")),
            2 => declaration.params.push(etas_std::StdGenericParam::new("T")),
            3 => declaration.projected_fields[1].ty = StdType::parse("string"),
            4 => declaration.kind = TypeDeclKind::Struct,
            5 => declaration.derivable = true,
            _ => unreachable!(),
        }
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        let module = builder.module(&["std", "probe"], "native ABI probe");
        builder.symbol(
            module,
            "Result",
            StdSymbolKind::Type,
            StdDecl::Type(declaration),
            "invalid",
        );
        let error = builder.try_finish().unwrap_err();
        assert!(error.reason.contains("native runtime type role"), "{error}");
    }
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "native ABI probe");
    for name in ["One", "Two"] {
        let mut declaration = valid();
        declaration.name = name.into();
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(declaration),
            "duplicate role",
        );
    }
    assert!(
        builder
            .try_finish()
            .unwrap_err()
            .reason
            .contains("native runtime type role")
    );
}
