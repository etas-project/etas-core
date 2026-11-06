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
            params: vec![
                scheduler_type(),
                StdType::Function {
                    input: vec![],
                    output: Box::new(StdType::Var("T".into())),
                    effects: StdEffectRowExpr::param("E"),
                },
            ],
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
            type_params: vec![],
            params: vec![scheduler_type(), continuation_type(StdType::parse("unit"))],
            output: StdType::parse("never"),
            public_effects: vec![],
            public_effect_tail: None,
            requested_actions: vec![],
            requested_action_tail: None,
            source_method: None,
        },
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
        },
        intrinsic::task::PARK,
        StdTaskIntrinsic::Park,
        IntrinsicDispatch::LoweringOnly,
        &["std", "task", "scheduler", "park"],
        "Atomically register a continuation as a task waiter or resume it from terminal state.",
    );
}

fn register_flow(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    declaration: FlowDecl,
    id: u32,
    operation: StdTaskIntrinsic,
    dispatch: IntrinsicDispatch,
    path: &[&str],
    docs: &str,
) {
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
    );
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
