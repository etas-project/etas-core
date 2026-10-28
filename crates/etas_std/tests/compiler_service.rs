use etas_std::{StdDecl, StdRuntimeRequirement, StdType, StdTypeConstruction, standard_registry};

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
