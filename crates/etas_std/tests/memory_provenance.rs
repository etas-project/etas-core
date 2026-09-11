use etas_std::*;

#[test]
fn preparation_declares_provenance_without_memory_authority() {
    let registry = standard_registry();
    for name in ["prepare_put", "prepare_delete"] {
        let symbol = registry.lookup_qualified(&["std", "memory", name]).unwrap();
        assert_eq!(registry.memory_place_result_argument(symbol.id), Some(0));
        assert_eq!(
            symbol.intrinsic.as_ref().unwrap().memory_access,
            IntrinsicMemoryAccess::None
        );
        let StdDecl::Flow(flow) = &symbol.decl else {
            panic!("flow")
        };
        assert!(flow.requested_actions.is_empty());
    }
    let inspect = registry
        .lookup_qualified(&["std", "memory", "operation_ref"])
        .unwrap();
    assert_eq!(registry.memory_place_result_argument(inspect.id), None);
}

fn declare(
    builder: &mut StdRegistryBuilder,
    module: StdModuleId,
    name: &str,
    parameter: &str,
) -> StdSymbolId {
    let mut declaration = FlowDecl::pure(name, &[], "unit");
    declaration.params = vec![if parameter == "Store<string, string>" {
        StdType::Store {
            key: Box::new(StdType::parse("string")),
            value: Box::new(StdType::parse("string")),
        }
    } else {
        StdType::parse(parameter)
    }];
    builder.symbol_with_intrinsic(
        module,
        name,
        StdSymbolKind::Flow,
        StdDecl::Flow(declaration),
        "test",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(900_001),
            qualified_path: vec!["std".into(), "test".into(), name.into()],
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: IntrinsicLatentEffect::None,
            memory_access: IntrinsicMemoryAccess::None,
            runtime_requirement: IntrinsicRuntimeRequirement::None,
        }),
    )
}

#[test]
fn provenance_is_shared_by_intrinsic_identity_and_aliases_are_validated() {
    for parameter in ["Store<string, string>", "string"] {
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        let module = builder.module(&["std", "test"], "test");
        let original = declare(&mut builder, module, "original", "Store<string, string>");
        let alias = declare(&mut builder, module, "alias", parameter);
        builder.memory_place_result(original, 0).unwrap();
        if parameter == "string" {
            let error = builder.try_finish().unwrap_err();
            assert!(error.reason.contains("same Store argument"));
        } else {
            let registry = builder.try_finish().unwrap();
            assert_eq!(registry.memory_place_result_argument(alias), Some(0));
        }
    }
}

#[test]
fn provenance_registration_rejects_missing_or_non_store_arguments() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "test"], "test");
    let symbol = declare(&mut builder, module, "producer", "string");
    assert!(builder.memory_place_result(symbol, 0).is_err());
    assert!(builder.memory_place_result(symbol, 1).is_err());
    assert!(builder.memory_place_result(StdSymbolId(99), 0).is_err());
}
