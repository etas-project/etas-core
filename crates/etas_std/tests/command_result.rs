use etas_std::{
    RuntimeFieldRole, RuntimeTypeRole, StdDecl, StdPrimitiveType, StdRegistryBuilder,
    StdRegistryVersion, StdSymbolKind, StdType, StdTypeConstruction, TypeDecl, TypeDeclKind,
    standard_registry,
};

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
    for defect in 0..5 {
        let mut declaration = valid();
        match defect {
            0 => declaration.construction = StdTypeConstruction::Public,
            1 => declaration.representation = Some(StdType::parse("string")),
            2 => declaration.params.push(etas_std::StdGenericParam::new("T")),
            3 => declaration.projected_fields[1].ty = StdType::parse("string"),
            4 => declaration.kind = TypeDeclKind::Struct,
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
