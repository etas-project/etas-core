use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdEffectRef, StdGenericParam, StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, StdType,
    intrinsic,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    for (name, output, intrinsic, action_id) in [
        (
            "check",
            compiler_type("CheckReport"),
            intrinsic::runtime::CODE_CHECK,
            146,
        ),
        (
            "compile",
            StdType::Var("A".into()),
            intrinsic::runtime::CODE_COMPILE,
            147,
        ),
    ] {
        builder.symbol_with_intrinsic(
            module,
            name,
            StdSymbolKind::Flow,
            StdDecl::Flow(FlowDecl {
                name: name.into(),
                type_params: vec![StdGenericParam::new("A")],
                params: vec![StdType::NamedApplied {
                    name: "std.compiler.AgentDef".into(),
                    args: vec![StdType::Var("A".into())],
                }],
                output,
                public_effects: vec![StdEffectRef::typed(
                    &["Error"],
                    compiler_type("CompileError"),
                )],
                requested_actions: vec![StdEffectRef::new(&[
                    "std", "effects", "actions", "Code", name,
                ])],
                source_method: None,
            }),
            "Check the immutable definition in its frozen environment without executing it.",
            Some(IntrinsicDescriptor {
                id: StdIntrinsicId(intrinsic),
                qualified_path: vec!["std".into(), "compiler".into(), name.into()],
                purity: IntrinsicPurity::Host,
                dispatch: IntrinsicDispatch::Host,
                lowering: LoweringHint::ForwardAction {
                    stable_id: action_id,
                },
                latent_effect: crate::IntrinsicLatentEffect::None,
                memory_access: crate::IntrinsicMemoryAccess::None,
                runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
            }),
        );
    }
}

fn compiler_type(name: &str) -> StdType {
    StdType::Named(format!("std.compiler.{name}"))
}

pub(super) fn register_actions(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "effects", "actions", "Code"],
        "Checked compiler service actions.",
    );
    for (name, output, stable_id) in [
        ("check", compiler_type("CheckReport"), 146),
        ("compile", StdType::Var("A".into()), 147),
    ] {
        let declaration = crate::EffectActionDecl::typed(
            "Code",
            name,
            vec![StdType::NamedApplied {
                name: "std.compiler.AgentDef".into(),
                args: vec![StdType::Var("A".into())],
            }],
            output,
        )
        .with_type_params(&[StdGenericParam::new("A")])
        .with_stable_id(stable_id)
        .with_runtime_requirement(crate::StdRuntimeRequirement::Compiler)
        .with_default_handler_effects(vec![StdEffectRef::typed(
            &["Error"],
            compiler_type("CompileError"),
        )]);
        builder.symbol(
            module,
            name,
            StdSymbolKind::EffectAction,
            StdDecl::EffectAction(declaration),
            "Compile/check the definition without executing or promoting it.",
        );
    }
}
