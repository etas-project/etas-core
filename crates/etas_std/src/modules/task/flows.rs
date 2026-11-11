use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicLatentEffect, IntrinsicMemoryAccess,
    IntrinsicPurity, IntrinsicRuntimeRequirement, LoweringHint, StdDecl, StdEffectRef,
    StdEffectRowExpr, StdGenericParam, StdIntrinsicId, StdRegistryBuilder, StdSymbolKind,
    StdTaskIntrinsic, StdType, intrinsic,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    register_flow(
        builder,
        module,
        FlowDecl {
            name: "run".into(),
            type_params: task_generics(),
            params: vec![StdType::Function {
                input: vec![scheduler_type()],
                output: Box::new(StdType::Var("T".into())),
                effects: StdEffectRowExpr::param("E"),
            }],
            output: StdType::Var("T".into()),
            public_effects: vec![],
            public_effect_tail: Some("E".into()),
            requested_actions: vec![],
            requested_action_tail: None,
            source_method: None,
        },
        intrinsic::task::RUN,
        StdTaskIntrinsic::Run,
        IntrinsicDispatch::Runtime,
        &["std", "task", "run"],
        "Run a structured scheduler scope and wait for all owned tasks to settle.",
    );

    register_flow(
        builder,
        module,
        FlowDecl {
            name: "start".into(),
            type_params: task_generics(),
            params: vec![StdType::Function {
                input: vec![],
                output: Box::new(StdType::Var("T".into())),
                effects: StdEffectRowExpr::param("E"),
            }],
            output: task_type(),
            public_effects: vec![],
            public_effect_tail: Some("E".into()),
            requested_actions: vec![],
            requested_action_tail: None,
            source_method: None,
        },
        intrinsic::task::START,
        StdTaskIntrinsic::Start,
        IntrinsicDispatch::Runtime,
        &["std", "task", "start"],
        "Start a task immediately in the scheduler scope and expose its full behavior.",
    );

    register_flow(
        builder,
        module,
        FlowDecl {
            name: "await".into(),
            type_params: task_generics(),
            params: vec![task_type()],
            output: StdType::Var("T".into()),
            public_effects: vec![StdEffectRef::new(&["Async"])],
            public_effect_tail: None,
            requested_actions: vec![StdEffectRef::new(&[
                "std", "task", "actions", "Async", "await",
            ])],
            requested_action_tail: None,
            source_method: None,
        },
        intrinsic::task::AWAIT,
        StdTaskIntrinsic::Await,
        IntrinsicDispatch::LoweringOnly,
        &["std", "task", "await"],
        "Perform Async.await so the active scheduler handler can intercept suspension.",
    );

    let scheduler = builder.module(
        &["std", "task", "scheduler"],
        "Compiler-facing scheduler continuation transfers.",
    );
    register_flow(
        builder,
        scheduler,
        FlowDecl {
            name: "requeue".into(),
            type_params: vec![StdGenericParam::new("T")],
            params: vec![
                scheduler_type(),
                continuation_type(StdType::Var("T".into())),
                StdType::Var("T".into()),
            ],
            output: StdType::parse("never"),
            public_effects: vec![],
            public_effect_tail: None,
            requested_actions: vec![],
            requested_action_tail: None,
            source_method: None,
        }
        .with_value_method("std.task.Scheduler", "requeue"),
        intrinsic::task::REQUEUE,
        StdTaskIntrinsic::Requeue,
        IntrinsicDispatch::LoweringOnly,
        &["std", "task", "scheduler", "requeue"],
        "Consume a yielded continuation and append it to the scheduler ready queue.",
    );
    register_flow(
        builder,
        scheduler,
        FlowDecl {
            name: "park".into(),
            type_params: task_generics(),
            params: vec![
                scheduler_type(),
                task_type(),
                continuation_type(StdType::Var("T".into())),
            ],
            output: StdType::parse("never"),
            public_effects: vec![],
            public_effect_tail: None,
            requested_actions: vec![],
            requested_action_tail: None,
            source_method: None,
        }
        .with_value_method("std.task.Scheduler", "park"),
        intrinsic::task::PARK,
        StdTaskIntrinsic::Park,
        IntrinsicDispatch::LoweringOnly,
        &["std", "task", "scheduler", "park"],
        "Atomically register a continuation as a task waiter or resume it from terminal state.",
    );

    register_combinators(builder, module);
}

fn register_combinators(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    let join = register_flow(
        builder,
        module,
        join_tuple_decl(&["A", "B"]),
        intrinsic::task::JOIN_TUPLE,
        StdTaskIntrinsic::JoinTuple,
        IntrinsicDispatch::Runtime,
        &["std", "task", "join"],
        "Run fixed or homogeneous jobs concurrently and preserve source result order.",
    );
    for declaration in [
        join_tuple_decl(&["A", "B", "C"]),
        join_tuple_decl(&["A", "B", "C", "D"]),
        join_array_decl(),
    ] {
        builder
            .flow_overload(join, declaration)
            .expect("task join overload preserves its registered identity");
    }

    register_flow(
        builder,
        module,
        error_array_decl(
            "try_join",
            StdType::Result {
                ok: Box::new(StdType::Array(Box::new(StdType::Var("T".into())))),
                err: Box::new(StdType::Var("X".into())),
            },
        ),
        intrinsic::task::TRY_JOIN,
        StdTaskIntrinsic::TryJoin,
        IntrinsicDispatch::Runtime,
        &["std", "task", "try_join"],
        "Run homogeneous jobs fail-fast and return the primary typed error as Result.",
    );
    register_flow(
        builder,
        module,
        error_array_decl(
            "collect",
            StdType::Array(Box::new(StdType::Result {
                ok: Box::new(StdType::Var("T".into())),
                err: Box::new(StdType::Var("X".into())),
            })),
        ),
        intrinsic::task::COLLECT,
        StdTaskIntrinsic::Collect,
        IntrinsicDispatch::Runtime,
        &["std", "task", "collect"],
        "Run every homogeneous job and retain an input-ordered Result for each branch.",
    );

    let mut race = error_array_decl("race", StdType::Var("T".into()));
    race.public_effects = vec![error_effect()];
    register_flow(
        builder,
        module,
        race,
        intrinsic::task::RACE,
        StdTaskIntrinsic::Race,
        IntrinsicDispatch::Runtime,
        &["std", "task", "race"],
        "Return the first successful branch and propagate a deterministic error if all fail.",
    );

    register_flow(
        builder,
        module,
        map_decl(false),
        intrinsic::task::MAP_CONCURRENT,
        StdTaskIntrinsic::MapConcurrent,
        IntrinsicDispatch::Runtime,
        &["std", "task", "map_concurrent"],
        "Map an array concurrently with stable input-to-output ordering.",
    );
    register_flow(
        builder,
        module,
        map_decl(true),
        intrinsic::task::MAP_CONCURRENT_COLLECT,
        StdTaskIntrinsic::MapConcurrentCollect,
        IntrinsicDispatch::Runtime,
        &["std", "task", "map_concurrent_collect"],
        "Map an array concurrently and retain an input-ordered Result for every item.",
    );
}

fn join_tuple_decl(outputs: &[&str]) -> FlowDecl {
    let mut type_params = outputs
        .iter()
        .map(|name| StdGenericParam::new(name))
        .collect::<Vec<_>>();
    type_params.push(StdGenericParam::effect_row("E"));
    let branches = outputs
        .iter()
        .map(|name| job_type(StdType::Var((*name).into()), StdEffectRowExpr::param("E")))
        .collect::<Vec<_>>();
    FlowDecl {
        name: "join".into(),
        type_params,
        params: vec![StdType::Tuple(branches)],
        output: StdType::Tuple(
            outputs
                .iter()
                .map(|name| StdType::Var((*name).into()))
                .collect(),
        ),
        public_effects: vec![],
        public_effect_tail: Some("E".into()),
        requested_actions: vec![],
        requested_action_tail: None,
        source_method: None,
    }
}

fn join_array_decl() -> FlowDecl {
    FlowDecl {
        name: "join".into(),
        type_params: task_generics(),
        params: vec![StdType::Array(Box::new(job_type(
            StdType::Var("T".into()),
            StdEffectRowExpr::param("E"),
        )))],
        output: StdType::Array(Box::new(StdType::Var("T".into()))),
        public_effects: vec![],
        public_effect_tail: Some("E".into()),
        requested_actions: vec![],
        requested_action_tail: None,
        source_method: None,
    }
}

fn error_array_decl(name: &str, output: StdType) -> FlowDecl {
    FlowDecl {
        name: name.into(),
        type_params: vec![
            StdGenericParam::new("T"),
            StdGenericParam::new("X"),
            StdGenericParam::effect_row("E"),
        ],
        params: vec![StdType::Array(Box::new(job_type(
            StdType::Var("T".into()),
            StdEffectRowExpr::with_tail(vec![error_effect()], "E"),
        )))],
        output,
        public_effects: vec![],
        public_effect_tail: Some("E".into()),
        requested_actions: vec![],
        requested_action_tail: None,
        source_method: None,
    }
}

fn map_decl(collect: bool) -> FlowDecl {
    let mut type_params = vec![StdGenericParam::new("A"), StdGenericParam::new("B")];
    if collect {
        type_params.push(StdGenericParam::new("X"));
    }
    type_params.push(StdGenericParam::effect_row("E"));
    let effects = if collect {
        StdEffectRowExpr::with_tail(vec![error_effect()], "E")
    } else {
        StdEffectRowExpr::param("E")
    };
    let output = if collect {
        StdType::Result {
            ok: Box::new(StdType::Var("B".into())),
            err: Box::new(StdType::Var("X".into())),
        }
    } else {
        StdType::Var("B".into())
    };
    FlowDecl {
        name: if collect {
            "map_concurrent_collect".into()
        } else {
            "map_concurrent".into()
        },
        type_params,
        params: vec![
            StdType::Array(Box::new(StdType::Var("A".into()))),
            job_type_with_input(StdType::Var("A".into()), StdType::Var("B".into()), effects),
        ],
        output: StdType::Array(Box::new(output)),
        public_effects: vec![],
        public_effect_tail: Some("E".into()),
        requested_actions: vec![],
        requested_action_tail: None,
        source_method: None,
    }
}

fn job_type(output: StdType, effects: StdEffectRowExpr) -> StdType {
    StdType::Function {
        input: vec![],
        output: Box::new(output),
        effects,
    }
}

fn job_type_with_input(input: StdType, output: StdType, effects: StdEffectRowExpr) -> StdType {
    StdType::Function {
        input: vec![input],
        output: Box::new(output),
        effects,
    }
}

fn error_effect() -> StdEffectRef {
    StdEffectRef::typed(&["Error"], StdType::Var("X".into()))
}

#[allow(clippy::too_many_arguments)]
fn register_flow(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    declaration: FlowDecl,
    id: u32,
    operation: StdTaskIntrinsic,
    dispatch: IntrinsicDispatch,
    path: &[&str],
    docs: &str,
) -> crate::StdSymbolId {
    let name = declaration.name.clone();
    builder.symbol_with_intrinsic(
        module,
        &name,
        StdSymbolKind::Flow,
        StdDecl::Flow(declaration),
        docs,
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(id),
            qualified_path: path.iter().map(|segment| (*segment).to_owned()).collect(),
            purity: IntrinsicPurity::Runtime,
            dispatch,
            lowering: LoweringHint::Task(operation),
            latent_effect: IntrinsicLatentEffect::None,
            memory_access: IntrinsicMemoryAccess::None,
            runtime_requirement: IntrinsicRuntimeRequirement::TaskScheduler,
        }),
    )
}

fn task_generics() -> Vec<StdGenericParam> {
    vec![StdGenericParam::new("T"), StdGenericParam::effect_row("E")]
}

fn task_type() -> StdType {
    StdType::Task {
        output: Box::new(StdType::Var("T".into())),
        effects: StdEffectRowExpr::param("E"),
    }
}

fn scheduler_type() -> StdType {
    StdType::Named("std.task.Scheduler".into())
}

fn continuation_type(output: StdType) -> StdType {
    StdType::NamedApplied {
        name: "std.task.Continuation".into(),
        args: vec![output],
    }
}
