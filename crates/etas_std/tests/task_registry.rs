use etas_std::{
    CompilerActionRole, IntrinsicDispatch, IntrinsicRuntimeRequirement, LoweringHint,
    RequirementSemantics, RuntimeTypeRole, StdDecl, StdEffectRowExpr, StdGenericParamKind,
    StdLimitKind, StdRegistryBuilder, StdRegistryVersion, StdRuntimeRequirement, StdSymbolKind,
    StdTaskIntrinsic, StdType, StdTypeConstruction, TypeDecl, TypeDeclKind, standard_registry,
};

#[test]
fn task_abi_keeps_effect_rows_actions_and_scheduler_authority_explicit() {
    let registry = standard_registry();

    let task = registry.lookup_qualified(&["std", "task", "Task"]).unwrap();
    let StdDecl::Type(task) = &task.decl else {
        panic!("Task type");
    };
    assert_eq!(task.runtime_role, Some(RuntimeTypeRole::Task));
    assert_eq!(task.construction, StdTypeConstruction::IntrinsicOnly);
    assert_eq!(task.params[0].kind, StdGenericParamKind::Type);
    assert_eq!(task.params[1].kind, StdGenericParamKind::EffectRow);

    let async_effect = registry
        .lookup_qualified(&["std", "task", "Async"])
        .unwrap();
    let StdDecl::Effect(async_effect) = &async_effect.decl else {
        panic!("Async effect");
    };
    assert_eq!(async_effect.stable_id, Some(28));
    assert_eq!(
        async_effect.runtime_requirement,
        Some(StdRuntimeRequirement::TaskScheduler)
    );

    let await_action = registry
        .lookup_qualified(&["std", "task", "actions", "Async", "await"])
        .unwrap();
    let StdDecl::EffectAction(await_action) = &await_action.decl else {
        panic!("Async.await action");
    };
    assert_eq!(
        await_action.compiler_role,
        Some(CompilerActionRole::TaskAwait)
    );
    assert_eq!(
        await_action.default_handler_effect_tail.as_deref(),
        Some("E")
    );
    assert_eq!(
        await_action.params,
        [StdType::Task {
            output: Box::new(StdType::Var("T".into())),
            effects: StdEffectRowExpr::param("E"),
        }]
    );

    let await_flow = registry
        .lookup_qualified(&["std", "task", "await"])
        .unwrap();
    let StdDecl::Flow(await_flow) = &await_flow.decl else {
        panic!("task await flow");
    };
    let intrinsic = await_flow_symbol_intrinsic(&registry);
    assert_eq!(intrinsic.dispatch, IntrinsicDispatch::LoweringOnly);
    assert_eq!(
        intrinsic.lowering,
        LoweringHint::Task(StdTaskIntrinsic::Await)
    );
    assert_eq!(
        intrinsic.runtime_requirement,
        IntrinsicRuntimeRequirement::TaskScheduler
    );
    assert_eq!(await_flow.public_effects[0].path, ["Async"]);
    assert_eq!(
        await_flow.requested_actions[0].path,
        ["std", "task", "actions", "Async", "await"]
    );

    let start = registry
        .lookup_qualified(&["std", "task", "start"])
        .unwrap();
    let StdDecl::Flow(start) = &start.decl else {
        panic!("task start flow");
    };
    assert_eq!(start.params.len(), 1);
    assert!(matches!(
        &start.params[0],
        StdType::Function { input, effects, .. }
            if input.is_empty() && effects.tail.as_deref() == Some("E")
    ));

    let requeue = registry
        .lookup_qualified(&["std", "task", "scheduler", "requeue"])
        .unwrap();
    let StdDecl::Flow(requeue) = &requeue.decl else {
        panic!("scheduler requeue flow");
    };
    assert_eq!(requeue.params.len(), 3);
    assert_eq!(requeue.type_params.len(), 1);
    assert_eq!(
        requeue
            .source_method
            .as_ref()
            .map(|method| method.name.as_str()),
        Some("requeue")
    );

    let park = registry
        .lookup_qualified(&["std", "task", "scheduler", "park"])
        .unwrap();
    let StdDecl::Flow(park) = &park.decl else {
        panic!("scheduler park flow");
    };
    assert_eq!(
        park.source_method
            .as_ref()
            .map(|method| method.name.as_str()),
        Some("park")
    );

    let concurrency = registry
        .lookup_qualified(&["std", "runtime", "limits", "Concurrency"])
        .unwrap();
    let StdDecl::Requirement(concurrency) = &concurrency.decl else {
        panic!("Concurrency requirement");
    };
    assert_eq!(
        concurrency.semantics,
        RequirementSemantics::Limit(StdLimitKind::Concurrency)
    );
}

fn await_flow_symbol_intrinsic(registry: &etas_std::StdRegistry) -> &etas_std::IntrinsicDescriptor {
    registry
        .lookup_qualified(&["std", "task", "await"])
        .and_then(|symbol| symbol.intrinsic.as_ref())
        .unwrap()
}

#[test]
fn task_runtime_role_rejects_a_type_parameter_in_place_of_the_effect_row() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "task"], "task ABI probe");
    builder.symbol(
        module,
        "Task",
        StdSymbolKind::Type,
        StdDecl::Type(
            TypeDecl::generic("Task", &["T", "E"], TypeDeclKind::Support)
                .runtime_role(RuntimeTypeRole::Task)
                .intrinsic_only(),
        ),
        "invalid task ABI",
    );

    assert!(
        builder
            .try_finish()
            .unwrap_err()
            .reason
            .contains("native runtime type role")
    );
}
