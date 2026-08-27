use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdEffectRef, StdGenericParam, StdIntrinsicId, StdRecordField, StdRegistryBuilder,
    StdStaticArg, StdSymbolKind, StdType, TypeDecl, TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "agent", "session"],
        "Session and conversation support declarations.",
    );
    for (name, representation) in [
        ("SessionId", None),
        (
            "SessionConfig",
            Some(record(&[
                ("id", "SessionId"),
                ("context", "ContextPolicy"),
                ("retention", "RetentionPolicy"),
                ("compaction", "CompactionPolicy"),
            ])),
        ),
        (
            "Conversation",
            Some(record(&[
                ("session", "SessionId"),
                ("messages", "Array[Message[std.json.JsonValue]]"),
                ("summary", "Option[SessionSummary]"),
                ("cursor", "Option[string]"),
            ])),
        ),
        ("SessionSummary", None),
        ("ContextPolicy", None),
        ("RetentionPolicy", None),
        ("CompactionPolicy", None),
    ] {
        let mut decl = TypeDecl::generic(name, &[], TypeDeclKind::Support);
        if let Some(representation) = representation {
            decl = decl.with_representation(representation);
        }
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(decl),
            "Session support type.",
        );
        builder.prelude(name, symbol);
    }
    for (name, params, output, docs) in [
        (
            "continue_or_new",
            &["T"][..],
            "SessionConfig",
            "Continue an existing session identified by a stable key, or create one.",
        ),
        (
            "LastTurns",
            &["usize"][..],
            "ContextPolicy",
            "Select the last N conversation turns as context.",
        ),
        (
            "SummaryPlusRecent",
            &["usize"][..],
            "ContextPolicy",
            "Select summarized history plus recent turns as context.",
        ),
        (
            "Days",
            &["usize"][..],
            "RetentionPolicy",
            "Retain session history for a number of days.",
        ),
        (
            "SummarizeWhen",
            &["std.runtime.limits.Limit"][..],
            "CompactionPolicy",
            "Compact session history when the given limit is reached.",
        ),
        (
            "load",
            &["SessionConfig"][..],
            "Conversation",
            "Load selected conversation history for a session.",
        ),
        (
            "compact",
            &["SessionConfig"][..],
            "Conversation",
            "Compact conversation history for a session.",
        ),
    ] {
        let decl = match name {
            "load" => {
                FlowDecl::with_actions(name, params, output, &[], &[session_memory_action("read")])
            }
            "compact" => {
                FlowDecl::with_actions(name, params, output, &[], &[session_memory_action("write")])
            }
            "continue_or_new" => FlowDecl::with_type_params_actions(
                name,
                &[StdGenericParam::new("T")],
                params,
                output,
                &[],
                &[],
            ),
            _ => FlowDecl::pure(name, params, output),
        };
        let descriptor = session_policy_intrinsic(name);
        let symbol = builder.symbol_with_intrinsic(
            module,
            name,
            StdSymbolKind::Flow,
            StdDecl::Flow(decl),
            docs,
            descriptor,
        );
        builder.prelude(name, symbol);
    }
    let current_session = builder.symbol_with_intrinsic(
        module,
        "current_session",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_actions(
            "current_session",
            &[],
            "SessionId",
            &[],
            &[session_memory_action("read")],
        )),
        "Read the current runtime session identifier.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::CURRENT_SESSION),
            qualified_path: vec![
                "std".into(),
                "agent".into(),
                "session".into(),
                "current_session".into(),
            ],
            purity: IntrinsicPurity::Runtime,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
    builder.prelude("current_session", current_session);
}

fn session_memory_action(action: &str) -> StdEffectRef {
    StdEffectRef::with_args(
        &["Memory", action],
        vec![StdStaticArg::path(&[
            "std",
            "agent",
            "session",
            "SessionId",
        ])],
    )
}

fn session_policy_intrinsic(name: &str) -> Option<IntrinsicDescriptor> {
    let id = match name {
        "LastTurns" => intrinsic::pure::SESSION_LAST_TURNS,
        "SummaryPlusRecent" => intrinsic::pure::SESSION_SUMMARY_PLUS_RECENT,
        "Days" => intrinsic::pure::SESSION_DAYS,
        "SummarizeWhen" => intrinsic::pure::SESSION_SUMMARIZE_WHEN,
        _ => return None,
    };
    Some(IntrinsicDescriptor {
        id: StdIntrinsicId(id),
        qualified_path: vec![
            "std".into(),
            "agent".into(),
            "session".into(),
            name.to_owned(),
        ],
        purity: IntrinsicPurity::Pure,
        dispatch: IntrinsicDispatch::Runtime,
        lowering: LoweringHint::RuntimeCall,
        latent_effect: crate::IntrinsicLatentEffect::None,
        memory_access: crate::IntrinsicMemoryAccess::None,
        runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
    })
}

fn record(fields: &[(&str, &str)]) -> StdType {
    StdType::Record(
        fields
            .iter()
            .map(|(name, ty)| StdRecordField::new(name, StdType::parse(ty)))
            .collect(),
    )
}
