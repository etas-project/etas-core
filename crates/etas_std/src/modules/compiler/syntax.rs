use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, StdType, intrinsic,
};

pub(super) fn register(builder: &mut StdRegistryBuilder, module: crate::StdModuleId) {
    for (name, params, output, id) in [
        (
            "parse_body",
            vec![StdType::parse("string")],
            StdType::Result {
                ok: Box::new(StdType::Named("std.compiler.SyntaxBlock".into())),
                err: Box::new(StdType::Named("std.compiler.ParseError".into())),
            },
            intrinsic::runtime::CODE_PARSE_BODY,
        ),
        (
            "format",
            vec![StdType::Named("std.compiler.SyntaxBlock".into())],
            StdType::parse("string"),
            intrinsic::runtime::CODE_FORMAT,
        ),
    ] {
        builder.symbol_with_intrinsic(
            module,
            name,
            StdSymbolKind::Flow,
            StdDecl::Flow(FlowDecl {
                name: name.into(),
                type_params: vec![],
                params,
                output,
                public_effects: vec![],
                requested_actions: vec![],
                source_method: None,
            }),
            "Pure syntax processing; never resolves names or executes generated code.",
            Some(IntrinsicDescriptor {
                id: StdIntrinsicId(id),
                qualified_path: vec!["std".into(), "compiler".into(), name.into()],
                purity: IntrinsicPurity::Pure,
                dispatch: IntrinsicDispatch::Runtime,
                lowering: LoweringHint::None,
                latent_effect: crate::IntrinsicLatentEffect::None,
                memory_access: crate::IntrinsicMemoryAccess::None,
                runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
            }),
        );
    }
    let definition = StdType::NamedApplied {
        name: "std.compiler.AgentDef".into(),
        args: vec![StdType::Var("A".into())],
    };
    let diff = builder.symbol_with_intrinsic(
        module,
        "diff",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl {
            name: "diff".into(),
            type_params: vec![crate::StdGenericParam::new("A")],
            params: vec![definition.clone(), definition.clone()],
            output: StdType::Named("std.compiler.ChangeReport".into()),
            public_effects: vec![],
            requested_actions: vec![],
            source_method: None,
        }),
        "Compare available immutable definition data, not behavioral compatibility.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::CODE_DIFF),
            qualified_path: vec!["std".into(), "compiler".into(), "diff".into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::None,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
    let with_body = builder.symbol_with_intrinsic(
        module,
        "with_body",
        StdSymbolKind::Flow,
        StdDecl::Flow(
            FlowDecl {
                name: "with_body".into(),
                type_params: vec![crate::StdGenericParam::new("A")],
                params: vec![
                    definition.clone(),
                    StdType::Named("std.compiler.SyntaxBlock".into()),
                ],
                output: definition.clone(),
                public_effects: vec![],
                requested_actions: vec![],
                source_method: None,
            }
            .with_intrinsic_value_method(definition, "with_body"),
        ),
        "Replace immutable definition body syntax without checking or executing it.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::CODE_WITH_BODY),
            qualified_path: vec!["std".into(), "compiler".into(), "with_body".into()],
            purity: IntrinsicPurity::Pure,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::None,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
    let definition = StdType::NamedApplied {
        name: "std.compiler.CallableDef".into(),
        args: vec![StdType::Var("F".into())],
    };
    builder
        .flow_overload(
            diff,
            FlowDecl {
                name: "diff".into(),
                type_params: vec![crate::StdGenericParam::new("F")],
                params: vec![definition.clone(), definition.clone()],
                output: StdType::Named("std.compiler.ChangeReport".into()),
                public_effects: vec![],
                requested_actions: vec![],
                source_method: None,
            },
        )
        .expect("snapshot diff overload preserves its registered identity");
    builder
        .flow_overload(
            with_body,
            FlowDecl {
                name: "with_body".into(),
                type_params: vec![crate::StdGenericParam::new("F")],
                params: vec![
                    definition.clone(),
                    StdType::Named("std.compiler.SyntaxBlock".into()),
                ],
                output: definition.clone(),
                public_effects: Vec::new(),
                requested_actions: Vec::new(),
                source_method: None,
            }
            .with_intrinsic_value_method(definition, "with_body"),
        )
        .expect("body-edit overload preserves its registered identity");
}
