use etas_std::{StdDecl, StdRuntimeRequirement, StdType, StdTypeConstruction, standard_registry};

#[test]
fn diff_accepts_agent_and_flow_snapshots_with_a_neutral_definition_identity() {
    let registry = standard_registry();
    let diff = registry
        .lookup_qualified(&["std", "compiler", "diff"])
        .unwrap();
    let signatures = registry.flow_declarations(diff.id).collect::<Vec<_>>();
    assert_eq!(signatures.len(), 2);
    assert!(
        matches!(&signatures[1].params[..], [StdType::NamedApplied { name: left, args: a }, StdType::NamedApplied { name: right, args: b }] if left == "std.compiler.CallableDef" && right == left && a == b && a == &[StdType::Var("F".into())])
    );
    assert!(
        signatures
            .iter()
            .all(|signature| signature.public_effects.is_empty()
                && signature.requested_actions.is_empty())
    );
    let StdDecl::Type(report) = &registry
        .lookup_qualified(&["std", "compiler", "ChangeReport"])
        .unwrap()
        .decl
    else {
        panic!("change report declaration");
    };
    let identity = report
        .projected_fields
        .iter()
        .find(|field| field.name == "identity")
        .unwrap();
    assert_eq!(
        identity.ty,
        StdType::Named("std.compiler.DefinitionId".into())
    );
}

#[test]
fn flow_reflection_overloads_share_compiler_intrinsics_and_action_authority() {
    let registry = standard_registry();
    for name in ["check", "compile"] {
        let wrapper = registry
            .lookup_qualified(&["std", "compiler", name])
            .unwrap();
        let action = registry
            .lookup_qualified(&["std", "effects", "actions", "Code", name])
            .unwrap();
        let flows = registry.flow_declarations(wrapper.id).collect::<Vec<_>>();
        let actions = registry.action_declarations(action.id).collect::<Vec<_>>();
        assert_eq!(flows.len(), 2);
        assert_eq!(actions.len(), 2);
        for (flow, action) in flows.iter().zip(&actions) {
            assert_eq!(flow.type_params, action.type_params);
            assert_eq!(flow.params, action.params);
            assert_eq!(flow.output, action.output);
            assert_eq!(
                action.default_handler_effects.as_ref(),
                Some(&flow.public_effects)
            );
            assert!(action.effect_args.is_empty());
            assert_eq!(action.stable_id, actions[0].stable_id);
        }
        assert!(
            matches!(&flows[1].params[..], [StdType::NamedApplied { name, args }] if name == "std.compiler.CallableDef" && args == &[StdType::Var("F".into())])
        );
    }
    let edit = registry
        .lookup_qualified(&["std", "compiler", "with_body"])
        .unwrap();
    let signatures = registry.flow_declarations(edit.id).collect::<Vec<_>>();
    assert_eq!(signatures.len(), 2);
    assert_eq!(
        signatures[1].source_method.as_ref().unwrap().receiver,
        signatures[1].params[0]
    );
}

#[test]
fn action_overload_cannot_change_authority_or_hide_an_invalid_type() {
    use etas_std::{
        EffectActionDecl, EffectDecl, StdGenericParam, StdRegistryBuilder, StdRegistryVersion,
        StdSymbolKind,
    };
    for change in 0..3 {
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        let module = builder.module(&["std", "test"], "test");
        builder.symbol(
            module,
            "Boundary",
            StdSymbolKind::Effect,
            StdDecl::Effect(EffectDecl::standard("Boundary", &[])),
            "test",
        );
        let module = builder.module(&["std", "test", "Boundary"], "test");
        let primary =
            EffectActionDecl::new("Boundary", "call", &["string"], "unit").with_stable_id(901);
        let symbol = builder.symbol(
            module,
            "call",
            StdSymbolKind::EffectAction,
            StdDecl::EffectAction(primary.clone()),
            "test",
        );
        let mut alternate =
            EffectActionDecl::new("Boundary", "call", &["i32"], "unit").with_stable_id(901);
        match change {
            0 => alternate.stable_id = Some(902),
            1 => alternate.output = StdType::Var("Undeclared".into()),
            2 => {
                alternate.type_params = vec![StdGenericParam::new("T")];
                alternate.high_impact_ack = true;
            }
            _ => unreachable!(),
        }
        builder.action_overload(symbol, alternate).unwrap();
        assert!(builder.try_finish().is_err());
    }
}

#[test]
fn compiler_service_has_typed_generic_actions_and_explicit_default_error_contracts() {
    let registry = standard_registry();
    for name in ["check", "compile"] {
        let symbol = registry
            .lookup_qualified(&["std", "effects", "actions", "Code", name])
            .unwrap();
        let StdDecl::EffectAction(action) = &symbol.decl else {
            panic!("action")
        };
        assert_eq!(action.type_params.len(), 1);
        assert_eq!(action.type_params[0].name, "A");
        assert_eq!(
            action.params,
            vec![StdType::NamedApplied {
                name: "std.compiler.AgentDef".into(),
                args: vec![StdType::Var("A".into())],
            }]
        );
        assert!(
            action.effect_args.is_empty(),
            "payload generic A is not an authority selector"
        );
        assert!(action.selector_param_names.is_empty());
        assert_eq!(
            action.runtime_requirement,
            Some(StdRuntimeRequirement::Compiler)
        );
        assert_eq!(
            action.default_handler_effects.as_ref().unwrap(),
            &vec![etas_std::StdEffectRef::typed(
                &["Error"],
                StdType::Named("std.compiler.CompileError".into()),
            )]
        );
        if name == "compile" {
            assert_eq!(action.output, StdType::Var("A".into()));
        } else {
            assert_eq!(
                action.output,
                StdType::Named("std.compiler.CheckReport".into())
            );
        }
    }
}

#[test]
fn compiler_wrappers_preserve_requested_actions_and_only_expose_compile_error() {
    let registry = standard_registry();
    for name in ["check", "compile"] {
        let symbol = registry
            .lookup_qualified(&["std", "compiler", name])
            .unwrap();
        let StdDecl::Flow(flow) = &symbol.decl else {
            panic!("flow")
        };
        assert_eq!(flow.type_params.len(), 1);
        assert_eq!(flow.params.len(), 1);
        assert_eq!(
            flow.requested_actions,
            vec![etas_std::StdEffectRef::new(&[
                "std", "effects", "actions", "Code", name
            ])]
        );
        assert_eq!(
            flow.public_effects,
            vec![etas_std::StdEffectRef::typed(
                &["Error"],
                StdType::Named("std.compiler.CompileError".into()),
            )]
        );
        assert_eq!(
            symbol.intrinsic.as_ref().unwrap().dispatch,
            etas_std::IntrinsicDispatch::Host
        );
    }
}

#[test]
fn reports_preserve_unavailable_analysis_and_are_not_compilation_evidence() {
    let registry = standard_registry();
    let symbol = registry
        .lookup_qualified(&["std", "compiler", "CheckReport"])
        .unwrap();
    let StdDecl::Type(report) = &symbol.decl else {
        panic!("report")
    };
    assert_eq!(report.construction, StdTypeConstruction::Public);
    assert_eq!(
        report.representation,
        Some(StdType::Record(report.projected_fields.clone()))
    );
    assert_eq!(
        report
            .projected_fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        [
            "status",
            "diagnostics",
            "revision",
            "dependencies",
            "snapshot_fingerprint",
            "effects",
            "determinism",
            "trace",
            "runtime_checks"
        ]
    );
    for name in ["determinism", "trace"] {
        assert!(matches!(
            &report
                .projected_fields
                .iter()
                .find(|field| field.name == name)
                .unwrap()
                .ty,
            StdType::Option(_)
        ));
    }
    let StdDecl::Type(summary) = &registry
        .lookup_qualified(&["std", "compiler", "EffectSummary"])
        .unwrap()
        .decl
    else {
        panic!("summary")
    };
    assert!(
        summary
            .projected_fields
            .iter()
            .all(|field| matches!(field.ty, StdType::Option(_)))
    );
    let StdDecl::Type(error) = &registry
        .lookup_qualified(&["std", "compiler", "CompileError"])
        .unwrap()
        .decl
    else {
        panic!("error")
    };
    let Some(StdType::Record(fields)) = &error.representation else {
        panic!("error ABI")
    };
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["kind", "diagnostics"]
    );
}

#[test]
fn registry_rejects_invalid_default_handler_effects_before_type_lowering() {
    use etas_std::{
        EffectActionDecl, EffectDecl, StdRegistryBuilder, StdRegistryVersion, StdSymbolKind,
    };
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "probe");
    builder.symbol(
        module,
        "Boundary",
        StdSymbolKind::Effect,
        StdDecl::Effect(EffectDecl::standard("Boundary", &[])),
        "probe",
    );
    let actions = builder.module(&["std", "probe", "Boundary"], "probe actions");
    builder.symbol(
        actions,
        "run",
        StdSymbolKind::EffectAction,
        StdDecl::EffectAction(
            EffectActionDecl::new("Boundary", "run", &[], "unit").with_default_handler_effects(
                vec![etas_std::StdEffectRef::new(&["MissingEffect"])],
            ),
        ),
        "invalid default handler",
    );
    let error = builder.try_finish().unwrap_err();
    assert!(error.reason.contains("MissingEffect"), "{error}");
}
