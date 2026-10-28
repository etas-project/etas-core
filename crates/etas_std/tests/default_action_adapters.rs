use etas_std::*;

fn registry(
    payload: &[usize],
    output: &str,
    target: u32,
    default: bool,
) -> Result<StdRegistry, StdRegistryValidationError> {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "adapter_test"], "adapter test");
    builder.symbol(
        module,
        "Boundary",
        StdSymbolKind::Effect,
        StdDecl::Effect(EffectDecl::standard("Boundary", &[])),
        "effect",
    );
    let actions = builder.module(&["std", "adapter_test", "Boundary"], "actions");
    let mut action =
        EffectActionDecl::new("Boundary", "run", &["bool"], "bool").with_stable_id(9_001);
    if default {
        action = action.with_default_handler_effects(vec![]);
    }
    builder.symbol(
        actions,
        "run",
        StdSymbolKind::EffectAction,
        StdDecl::EffectAction(action),
        "action",
    );
    builder.symbol_with_intrinsic(
        module,
        "call",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_actions(
            "call",
            &["bool", "i32"],
            output,
            &[],
            &[StdEffectRef::new(&["Boundary", "run"])],
        )),
        "adapter",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(9_002),
            qualified_path: vec!["std".into(), "adapter_test".into(), "call".into()],
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::default_action(target, payload),
            latent_effect: IntrinsicLatentEffect::None,
            memory_access: IntrinsicMemoryAccess::None,
            runtime_requirement: IntrinsicRuntimeRequirement::None,
        }),
    );
    builder.try_finish()
}

#[test]
fn registry_checks_the_adapter_payload_and_result_abi() {
    registry(&[0], "bool", 9_001, true).unwrap();
    for (payload, output, target, default) in [
        (vec![1], "bool", 9_001, true),
        (vec![2], "bool", 9_001, true),
        (vec![], "bool", 9_001, true),
        (vec![0], "i32", 9_001, true),
        (vec![0], "bool", 9_099, true),
        (vec![0], "bool", 9_001, false),
    ] {
        let error = registry(&payload, output, target, default).unwrap_err();
        assert!(error.reason.contains("default action adapter"), "{error}");
    }
}

#[test]
fn standard_native_adapters_retain_static_selector_binders_and_projection() {
    let registry = standard_registry();
    for (path, indices) in [
        (vec!["std", "fs", "write_bytes"], vec![0]),
        (vec!["std", "stream", "read"], vec![0]),
        (vec!["std", "net", "tcp", "connect"], vec![0, 1]),
        (vec!["std", "tls", "connect"], vec![1]),
    ] {
        let symbol = registry.lookup_qualified(&path).unwrap();
        let LoweringHint::DefaultAction { payload, stable_id } =
            &symbol.intrinsic.as_ref().unwrap().lowering
        else {
            panic!("missing adapter")
        };
        assert_eq!(payload, &indices);
        let action = registry
            .symbols()
            .find_map(|symbol| match &symbol.decl {
                StdDecl::EffectAction(action) if action.stable_id == Some(*stable_id) => {
                    Some(action)
                }
                _ => None,
            })
            .unwrap();
        let StdDecl::Flow(flow) = &symbol.decl else {
            panic!("not a flow")
        };
        assert_eq!(flow.type_params, action.type_params);
        assert_eq!(flow.output, action.output);
        assert_eq!(
            flow.public_effects,
            *action.default_handler_effects.as_ref().unwrap()
        );
    }
}
