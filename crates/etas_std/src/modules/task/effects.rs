use crate::{
    CompilerActionRole, EffectActionDecl, EffectDecl, StdDecl, StdEffectRowExpr, StdGenericParam,
    StdRegistryBuilder, StdRuntimeRequirement, StdSymbolKind, StdType,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, task_module: crate::StdModuleId) {
    let effect = builder.symbol(
        task_module,
        "Async",
        StdSymbolKind::Effect,
        StdDecl::Effect(
            EffectDecl::core("Async")
                .with_stable_id(28)
                .with_runtime_requirement(StdRuntimeRequirement::TaskScheduler),
        ),
        "Structured task suspension handled by a task scheduler.",
    );
    builder.prelude("Async", effect);

    let module = builder.module(
        &["std", "task", "actions", "Async"],
        "Scheduler-intercepted task suspension actions.",
    );

    builder.symbol(
        module,
        "yield",
        StdSymbolKind::EffectAction,
        StdDecl::EffectAction(
            EffectActionDecl::local("Async", "yield", &[], "unit")
                .with_stable_id(148)
                .with_runtime_requirement(StdRuntimeRequirement::TaskScheduler)
                .with_compiler_role(CompilerActionRole::TaskYield)
                .with_default_handler_effects(vec![]),
        ),
        "Yield the current task and enqueue its continuation at the back of the ready queue.",
    );

    builder.symbol(
        module,
        "await",
        StdSymbolKind::EffectAction,
        StdDecl::EffectAction(
            EffectActionDecl::typed(
                "Async",
                "await",
                vec![StdType::Task {
                    output: Box::new(StdType::Var("T".into())),
                    effects: StdEffectRowExpr::param("E"),
                }],
                StdType::Var("T".into()),
            )
            .with_type_params(&[StdGenericParam::new("T"), StdGenericParam::effect_row("E")])
            .with_stable_id(149)
            .with_runtime_requirement(StdRuntimeRequirement::TaskScheduler)
            .with_compiler_role(CompilerActionRole::TaskAwait)
            .with_default_handler_effect_tail("E"),
        ),
        "Suspend until the task reaches a terminal state, then resume with its result.",
    );
}
