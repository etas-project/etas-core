use etas_std::{
    EffectActionArgKind, EffectActionDecl, EffectDecl, FlowDecl, IntrinsicDispatch,
    IntrinsicLatentEffect, IntrinsicMemoryAccess, IntrinsicPurity, IntrinsicRuntimeRequirement,
    RequirementSemantics, StdDecl, StdEffectRef, StdGenericParam, StdImplFact, StdIntrinsicId,
    StdLimitKind, StdPrimitiveType, StdRegistryBuilder, StdRegistryVersion, StdSpecRef,
    StdStaticArg, StdSupportConstraint, StdSymbolKind, StdType, TypeDecl, TypeDeclKind, intrinsic,
    standard_registry,
};

#[test]
fn registry_exposes_core_qualified_symbols_and_prelude() {
    let registry = standard_registry();

    let bool_symbol = registry
        .lookup_qualified(&["std", "core", "bool"])
        .expect("bool should be a standard primitive");
    assert_eq!(bool_symbol.kind, StdSymbolKind::Type);
    assert!(matches!(bool_symbol.decl, StdDecl::Type(_)));

    let list = registry
        .lookup_prelude("List")
        .expect("List should be in the prelude");
    assert_eq!(list.qualified_path, vec!["std", "collections", "List"]);

    let range = registry
        .lookup_prelude("Range")
        .expect("Range should be in the prelude");
    assert_eq!(range.qualified_path, vec!["std", "collections", "Range"]);

    let slice = registry
        .lookup_prelude("Slice")
        .expect("Slice should be in the prelude");
    assert_eq!(slice.qualified_path, vec!["std", "collections", "Slice"]);

    for name in [
        "Deque",
        "Queue",
        "Stack",
        "PriorityQueue",
        "OrderedMap",
        "OrderedSet",
    ] {
        let symbol = registry
            .lookup_prelude(name)
            .unwrap_or_else(|| panic!("{name} should be in the prelude"));
        assert_eq!(symbol.qualified_path, vec!["std", "collections", name]);
        assert_eq!(symbol.kind, StdSymbolKind::Type);
    }

    let prompt = registry
        .lookup_prelude("Prompt")
        .expect("Prompt should be in the prelude");
    assert_eq!(
        prompt.qualified_path,
        vec!["std", "agent", "prompt", "Prompt"]
    );
}

#[test]
fn registry_exposes_structured_generic_spec_bounds_and_impl_facts() {
    let registry = standard_registry();
    let write_all = registry
        .lookup_qualified(&["std", "stream", "write_all"])
        .expect("write_all should be registered");
    let StdDecl::Flow(write_all) = &write_all.decl else {
        panic!("write_all should be a flow");
    };
    assert_eq!(write_all.type_params.len(), 1);
    assert_eq!(write_all.type_params[0].name, "S");
    assert_eq!(
        write_all.type_params[0].bounds[0].path,
        vec!["std", "stream", "ByteStream"]
    );

    for stream in ["std.net.tcp.TcpStream", "std.tls.TlsStream"] {
        assert!(registry.spec_impls().any(|implementation| {
            implementation.self_type == StdType::parse(stream)
                && implementation.spec.path == vec!["std", "stream", "ByteStream"]
        }));
    }
}

#[test]
fn registry_tracks_intrinsic_descriptors_without_executing_them() {
    let registry = standard_registry();

    let assert = registry
        .intrinsic(StdIntrinsicId(intrinsic::pure::ASSERT))
        .expect("assert should have a pure intrinsic descriptor");
    assert_eq!(assert.purity, IntrinsicPurity::Pure);
    assert_eq!(assert.dispatch, IntrinsicDispatch::PureKernel);

    let approve = registry
        .intrinsic(StdIntrinsicId(intrinsic::runtime::APPROVE))
        .expect("approve should have a runtime descriptor");
    assert_eq!(approve.purity, IntrinsicPurity::Runtime);
    assert_eq!(approve.dispatch, IntrinsicDispatch::Runtime);
}

#[test]
fn registry_marks_latent_transparent_container_intrinsics() {
    let registry = standard_registry();

    for path in [
        ["std", "option", "Some"],
        ["std", "option", "unwrap"],
        ["std", "result", "Ok"],
        ["std", "result", "Err"],
    ] {
        let symbol = registry
            .lookup_qualified(&path)
            .unwrap_or_else(|| panic!("{} should be registered", path.join(".")));
        let intrinsic = symbol
            .intrinsic
            .as_ref()
            .unwrap_or_else(|| panic!("{} should carry intrinsic metadata", path.join(".")));
        assert_eq!(
            intrinsic.latent_effect,
            IntrinsicLatentEffect::TransparentFirstArg
        );
    }
}

#[test]
fn registry_marks_memory_intrinsic_access_footprints() {
    let registry = standard_registry();

    for name in [
        "get",
        "contains",
        "keys",
        "select",
        "query",
        "scan",
        "related_to",
    ] {
        let symbol = registry
            .lookup_qualified(&["std", "memory", name])
            .unwrap_or_else(|| panic!("std.memory.{name} should be registered"));
        let intrinsic = symbol
            .intrinsic
            .as_ref()
            .unwrap_or_else(|| panic!("std.memory.{name} should carry intrinsic metadata"));
        assert_eq!(
            intrinsic.memory_access,
            IntrinsicMemoryAccess::ReadFirstArgStore
        );
    }

    for name in ["put", "insert", "delete", "update", "clear"] {
        let symbol = registry
            .lookup_qualified(&["std", "memory", name])
            .unwrap_or_else(|| panic!("std.memory.{name} should be registered"));
        let intrinsic = symbol
            .intrinsic
            .as_ref()
            .unwrap_or_else(|| panic!("std.memory.{name} should carry intrinsic metadata"));
        assert_eq!(
            intrinsic.memory_access,
            IntrinsicMemoryAccess::WriteFirstArgStore
        );
    }

    let upsert = registry
        .lookup_qualified(&["std", "memory", "upsert"])
        .expect("std.memory.upsert should be registered");
    assert_eq!(
        upsert
            .intrinsic
            .as_ref()
            .expect("std.memory.upsert should carry intrinsic metadata")
            .memory_access,
        IntrinsicMemoryAccess::ReadWriteFirstArgStore
    );
    let StdDecl::Flow(upsert_flow) = &upsert.decl else {
        panic!("std.memory.upsert should be a flow declaration");
    };
    assert_eq!(
        upsert_flow.requested_actions,
        vec![store_action("read"), store_action("write")]
    );
}

#[test]
fn registry_marks_runtime_limit_requirement_semantics() {
    let registry = standard_registry();

    for (name, kind) in [
        ("Iterations", StdLimitKind::Iterations),
        ("Tokens", StdLimitKind::Tokens),
        ("ContextTokens", StdLimitKind::ContextTokens),
        ("Cost", StdLimitKind::Cost),
        ("WallTime", StdLimitKind::WallTime),
        ("Attempts", StdLimitKind::Attempts),
    ] {
        let symbol = registry
            .lookup_qualified(&["std", "runtime", "limits", name])
            .unwrap_or_else(|| panic!("std.runtime.limits.{name} should be registered"));
        let StdDecl::Requirement(requirement) = &symbol.decl else {
            panic!("std.runtime.limits.{name} should be a requirement declaration");
        };
        assert_eq!(requirement.semantics, RequirementSemantics::Limit(kind));
    }
}

#[test]
fn registry_marks_checkpoint_intrinsic_runtime_requirement() {
    let registry = standard_registry();
    let symbol = registry
        .lookup_qualified(&["std", "runtime", "checkpoint"])
        .expect("std.runtime.checkpoint should be registered");
    let intrinsic = symbol
        .intrinsic
        .as_ref()
        .expect("checkpoint should carry intrinsic metadata");
    assert_eq!(
        intrinsic.runtime_requirement,
        IntrinsicRuntimeRequirement::Checkpoint
    );
}

#[test]
fn registry_declares_core_effects_and_requirement_constructors() {
    let registry = standard_registry();

    for name in [
        "Agentic", "Network", "FileIO", "Command", "Memory", "Secret", "Time", "Human", "Error",
    ] {
        let symbol = registry
            .lookup_qualified(&["std", "runtime", "effects", name])
            .unwrap_or_else(|| panic!("{name} effect should be registered"));
        assert_eq!(symbol.kind, StdSymbolKind::Effect);
        let StdDecl::Effect(effect) = &symbol.decl else {
            panic!("{name} should be an effect declaration");
        };
        assert!(effect.core, "{name} should be a core effect");
    }

    assert!(
        registry.lookup_prelude("Memory").is_some(),
        "Memory is the broad core memory effect root in the current SPEC"
    );

    let console = registry
        .lookup_prelude("Console")
        .expect("Console effect should be in prelude");
    assert_eq!(console.kind, StdSymbolKind::Effect);
    let StdDecl::Effect(effect) = &console.decl else {
        panic!("Console should be an effect declaration");
    };
    assert!(
        !effect.core,
        "Console is a std effect and should not be marked core"
    );
    assert_eq!(effect.extends, vec!["FileIO".to_owned()]);

    assert!(
        registry.lookup_prelude("Capability").is_none(),
        "source-level Capability requirements were removed from the language SPEC"
    );
    assert!(
        registry.lookup_prelude("Sandbox").is_none(),
        "source-level Sandbox requirements were removed from the language SPEC"
    );
    let profile = registry
        .lookup_prelude("SandboxProfile")
        .expect("SandboxProfile support type should be in prelude");
    assert_eq!(profile.kind, StdSymbolKind::Type);
    let default_sandbox = registry
        .lookup_prelude("DefaultCommandSandbox")
        .expect("DefaultCommandSandbox support value should be in prelude");
    assert_eq!(default_sandbox.kind, StdSymbolKind::Value);
    let StdDecl::Value(value) = &default_sandbox.decl else {
        panic!("DefaultCommandSandbox should be a value declaration");
    };
    assert_eq!(value.name, "DefaultCommandSandbox");
    assert_eq!(
        value.ty,
        StdType::Named("std.host.sandbox.SandboxProfile".to_owned())
    );
}

#[test]
fn registry_declares_error_raise_as_never_returning_action() {
    let registry = standard_registry();

    let raise = registry
        .lookup_qualified(&["std", "runtime", "error", "raise"])
        .expect("std.runtime.error.raise should exist");
    assert_eq!(raise.kind, StdSymbolKind::EffectAction);
    let StdDecl::EffectAction(action) = &raise.decl else {
        panic!("raise should be an effect action declaration");
    };
    assert_eq!(action.owner, "Error");
    assert_eq!(action.name, "raise");
    assert_eq!(action.params, vec![StdType::Var("E".to_owned())]);
    assert_eq!(action.output, StdType::Primitive(StdPrimitiveType::Never));
}

#[test]
fn registry_declares_approval_request_as_effect_action() {
    let registry = standard_registry();

    let request = registry
        .lookup_qualified(&["std", "runtime", "approval", "request"])
        .expect("std.runtime.approval.request should exist");
    assert_eq!(request.kind, StdSymbolKind::EffectAction);
    let StdDecl::EffectAction(action) = &request.decl else {
        panic!("request should be an effect action declaration");
    };
    assert_eq!(action.owner, "Approval");
    assert_eq!(action.name, "request");
    assert_eq!(
        action.params,
        vec![StdType::Named("ApprovalRequest".to_owned())]
    );
    assert_eq!(action.output, StdType::Named("ApprovalDecision".to_owned()));
}

#[test]
fn registry_declares_minimal_standard_effect_action_vocabulary() {
    let registry = standard_registry();

    for effect in [
        "Agentic", "Network", "FileIO", "Command", "Memory", "Secret", "Time", "Human", "Error",
        "Console", "Approval", "Clock",
    ] {
        let symbol = registry
            .lookup_qualified(&["std", "effects", effect])
            .unwrap_or_else(|| panic!("missing std.effects.{effect}"));
        assert_eq!(symbol.kind, StdSymbolKind::Effect);
    }

    for (owner, actions) in [
        (
            "Console",
            &[
                "stdin_read_line",
                "stdin_read_all",
                "stdout_write",
                "stderr_write",
            ][..],
        ),
        ("Command", &["run"][..]),
        ("Secret", &["read"][..]),
        ("Agentic", &["infer"][..]),
        ("Memory", &["read", "write"][..]),
        ("Approval", &["request"][..]),
        ("Clock", &["now", "sleep"][..]),
    ] {
        for action_name in actions {
            let symbol = registry
                .lookup_qualified(&["std", "effects", "actions", owner, action_name])
                .unwrap_or_else(|| panic!("missing std.effects.actions.{owner}.{action_name}"));
            let StdDecl::EffectAction(action) = &symbol.decl else {
                panic!("std.effects.{action_name} should be an effect action");
            };
            assert_eq!(action.owner, owner);
            assert_eq!(action.name, *action_name);
        }
    }

    let command_run = registry
        .lookup_qualified(&["std", "effects", "actions", "Command", "run"])
        .expect("Command.run action descriptor should exist");
    let StdDecl::EffectAction(command_run) = &command_run.decl else {
        panic!("Command.run should be an effect action");
    };
    assert_eq!(
        command_run.effect_args,
        vec![EffectActionArgKind::StaticResourcePath {
            ty: "std.host.sandbox.SandboxProfile"
        }]
    );

    let memory_read = registry
        .lookup_qualified(&["std", "effects", "actions", "Memory", "read"])
        .expect("Memory.read action descriptor should exist");
    let StdDecl::EffectAction(memory_read) = &memory_read.decl else {
        panic!("Memory.read should be an effect action");
    };
    assert_eq!(
        memory_read.effect_args,
        vec![EffectActionArgKind::MemoryPlace]
    );

    for path in [
        &["std", "effects", "Web"][..],
        &["std", "effects", "Workspace"][..],
        &["std", "effects", "File"][..],
        &["std", "effects", "Db"][..],
        &["std", "effects", "Vector"][..],
        &["std", "effects", "Email"][..],
        &["std", "effects", "Calendar"][..],
        &["std", "effects", "Payment"][..],
        &["std", "effects", "actions", "Web", "search"][..],
        &["std", "effects", "actions", "Command", "spawn"][..],
        &["std", "effects", "actions", "Memory", "migrate"][..],
        &["std", "effects", "actions", "Memory", "compact"][..],
        &["std", "effects", "actions", "Secret", "write"][..],
        &["std", "effects", "actions", "Agentic", "embed"][..],
        &["std", "effects", "actions", "Clock", "schedule"][..],
    ] {
        assert!(
            registry.lookup_qualified(path).is_none(),
            "old std effect vocabulary should not resolve: {path:?}"
        );
    }
}

#[test]
fn registry_covers_phase1_standard_surface_vocabulary() {
    let registry = standard_registry();

    for path in [
        &["std", "json", "JsonValue"][..],
        &["std", "core", "Index"][..],
        &["std", "collections", "LengthInput"][..],
        &["std", "collections", "EmptinessInput"][..],
        &["std", "collections", "len"][..],
        &["std", "collections", "is_empty"][..],
        &["std", "agent", "prompt", "new"][..],
        &["std", "agent", "prompt", "system"][..],
        &["std", "agent", "prompt", "user"][..],
        &["std", "agent", "prompt", "assistant"][..],
        &["std", "agent", "prompt", "data"][..],
        &["std", "agent", "schema", "Schema"][..],
        &["std", "agent", "schema", "ResponseDecode"][..],
        &["std", "agent", "schema", "ModelResponse"][..],
        &["std", "agent", "message", "Message"][..],
        &["std", "agent", "message", "Provenance"][..],
        &["std", "agent", "message", "new"][..],
        &["std", "agent", "message", "cast"][..],
        &["std", "agent", "session", "SessionId"][..],
        &["std", "agent", "session", "SessionConfig"][..],
        &["std", "agent", "session", "continue_or_new"][..],
        &["std", "agent", "session", "ContextPolicy"][..],
        &["std", "agent", "session", "RetentionPolicy"][..],
        &["std", "agent", "session", "CompactionPolicy"][..],
        &["std", "agent", "group", "round_robin"][..],
        &["std", "host", "path", "Path"][..],
        &["std", "host", "url", "Url"][..],
        &["std", "host", "command", "Command"][..],
        &["std", "runtime", "time", "Time"][..],
        &["std", "runtime", "budget", "Money"][..],
        &["std", "runtime", "limits", "Limit"][..],
        &["std", "runtime", "limits", "Cost"][..],
        &["std", "runtime", "limits", "WallTime"][..],
        &["std", "runtime", "approval", "request"][..],
        &["std", "runtime", "error", "raise"][..],
        &["std", "runtime", "checkpoint", "checkpoint"][..],
        &["std", "runtime", "trace", "TraceLabel"][..],
        &["std", "effects", "Console"][..],
        &["std", "effects", "actions", "Console", "stdout_write"][..],
        &["std", "memory", "region"][..],
        &["std", "memory", "get"][..],
        &["std", "memory", "select"][..],
        &["std", "memory", "query"][..],
        &["std", "memory", "scan"][..],
        &["std", "memory", "put"][..],
        &["std", "io", "read_all"][..],
        &["std", "io", "println"][..],
        &["std", "option", "unwrap"][..],
    ] {
        assert!(
            registry.lookup_qualified(path).is_some(),
            "missing std declaration for {path:?}"
        );
    }

    for prelude in [
        "Trusted",
        "Untrusted",
        "Secret",
        "Public",
        "Sanitized",
        "Prompt",
        "PromptEncode",
        "Message",
        "Schema",
        "ResponseDecode",
        "ModelResponse",
        "SessionId",
        "SessionConfig",
        "Conversation",
        "ContextPolicy",
        "RetentionPolicy",
        "CompactionPolicy",
        "LastTurns",
        "SummaryPlusRecent",
        "Days",
        "SummarizeWhen",
        "current_session",
        "Iterations",
        "Tokens",
        "ContextTokens",
        "Cost",
        "WallTime",
        "Attempts",
        "Limit",
        "MemoryRegion",
        "MemorySelection",
        "Store",
        "Index",
        "Console",
        "IOError",
        "approve",
        "Error",
    ] {
        assert!(
            registry.lookup_prelude(prelude).is_some(),
            "missing prelude symbol {prelude}"
        );
    }
    assert!(
        registry.lookup_prelude("unwrap").is_none(),
        "unwrap must remain a qualified std.option helper, not a prelude symbol"
    );

    let cast = registry
        .lookup_qualified(&["std", "agent", "message", "cast"])
        .expect("std.agent.message.cast should exist");
    let StdDecl::Flow(cast) = &cast.decl else {
        panic!("std.agent.message.cast should be a flow");
    };
    assert_eq!(
        cast.params,
        vec![StdType::Message(Box::new(StdType::Var("T".to_owned())))]
    );
    assert_eq!(
        cast.output,
        StdType::Option(Box::new(StdType::Message(Box::new(StdType::Var(
            "T".to_owned()
        )))))
    );

    let continue_or_new = registry
        .lookup_qualified(&["std", "agent", "session", "continue_or_new"])
        .expect("std.agent.session.continue_or_new should exist");
    let StdDecl::Flow(continue_or_new) = &continue_or_new.decl else {
        panic!("std.agent.session.continue_or_new should be a flow");
    };
    assert_eq!(continue_or_new.params, vec![StdType::Var("T".to_owned())]);
    assert_eq!(
        continue_or_new.output,
        StdType::Named("SessionConfig".to_owned())
    );
    assert!(continue_or_new.public_effects.is_empty());
    assert!(continue_or_new.requested_actions.is_empty());
}

#[test]
fn registry_declares_prompt_builder_support_flows() {
    let registry = standard_registry();

    for name in ["new", "system", "user", "assistant", "data"] {
        let symbol = registry
            .lookup_qualified(&["std", "agent", "prompt", name])
            .unwrap_or_else(|| panic!("missing prompt helper `{name}`"));
        assert_eq!(symbol.kind, StdSymbolKind::Flow);
        assert!(matches!(symbol.decl, StdDecl::Flow(_)));
    }
}

#[test]
fn registry_declares_memory_support_types_and_intrinsics() {
    let registry = standard_registry();

    for name in [
        "MemoryRegion",
        "Store",
        "MemorySelection",
        "MemoryTransaction",
        "MemoryVersion",
        "MemoryConflict",
    ] {
        let symbol = registry
            .lookup_prelude(name)
            .unwrap_or_else(|| panic!("missing memory support type `{name}`"));
        assert_eq!(symbol.kind, StdSymbolKind::Type);
        assert!(matches!(symbol.decl, StdDecl::Type(_)));
    }

    for (name, intrinsic_id) in [
        ("region", intrinsic::runtime::MEMORY_REGION),
        ("get", intrinsic::runtime::MEMORY_GET),
        ("contains", intrinsic::runtime::MEMORY_CONTAINS),
        ("keys", intrinsic::runtime::MEMORY_KEYS),
        ("select", intrinsic::runtime::MEMORY_SELECT),
        ("query", intrinsic::runtime::MEMORY_QUERY),
        ("scan", intrinsic::runtime::MEMORY_SCAN),
        ("related_to", intrinsic::runtime::MEMORY_RELATED_TO),
        ("put", intrinsic::runtime::MEMORY_PUT),
        ("insert", intrinsic::runtime::MEMORY_INSERT),
        ("upsert", intrinsic::runtime::MEMORY_UPSERT),
        ("delete", intrinsic::runtime::MEMORY_DELETE),
        ("update", intrinsic::runtime::MEMORY_UPDATE),
        ("clear", intrinsic::runtime::MEMORY_CLEAR),
    ] {
        let symbol = registry
            .lookup_qualified(&["std", "memory", name])
            .unwrap_or_else(|| panic!("missing std.memory.{name}"));
        assert_eq!(symbol.kind, StdSymbolKind::Flow);
        assert!(matches!(symbol.decl, StdDecl::Flow(_)));
        assert_eq!(
            symbol
                .intrinsic
                .as_ref()
                .expect("memory flow should carry an intrinsic")
                .id,
            StdIntrinsicId(intrinsic_id)
        );
    }

    let limit = registry
        .lookup_qualified(&["std", "memory", "limit"])
        .expect("missing std.memory.limit");
    assert_eq!(limit.kind, StdSymbolKind::Flow);
    let StdDecl::Flow(flow) = &limit.decl else {
        panic!("std.memory.limit should be a flow");
    };
    assert_eq!(
        flow.params,
        vec![
            StdType::MemorySelection(Box::new(StdType::Var("V".to_owned()))),
            StdType::Named("std.runtime.limits.Limit".to_owned())
        ]
    );
    assert_eq!(
        flow.output,
        StdType::MemorySelection(Box::new(StdType::Var("V".to_owned())))
    );
    assert!(flow.public_effects.is_empty());
    assert!(flow.requested_actions.is_empty());
    assert!(
        limit.intrinsic.is_none(),
        "memory selection limit is a pure support method, not a runtime intrinsic"
    );
}

#[test]
fn registry_declares_standard_runtime_error_family() {
    let registry = standard_registry();

    for name in [
        "SchemaError",
        "ValidationError",
        "ToolError",
        "ToolTimeout",
        "ToolDenied",
        "PolicyViolation",
        "PolicyDenied",
        "EffectBoundaryViolation",
        "SandboxViolation",
        "PromptInjectionRisk",
        "MissingCitation",
        "BudgetExceeded",
        "ProtocolViolation",
        "HumanRejected",
    ] {
        let symbol = registry
            .lookup_prelude(name)
            .unwrap_or_else(|| panic!("{name} should be a standard error prelude type"));
        assert_eq!(symbol.qualified_path, vec!["std", "runtime", "error", name]);
        assert_eq!(symbol.kind, StdSymbolKind::Type);
        assert!(matches!(symbol.decl, StdDecl::Type(_)));
    }

    let io_error = registry
        .lookup_qualified(&["std", "runtime", "error", "IOError"])
        .expect("IOError should be a qualified standard error type");
    assert_eq!(io_error.kind, StdSymbolKind::Type);

    let raise = registry
        .lookup_qualified(&["std", "runtime", "error", "raise"])
        .expect("Error.raise should be registered");
    assert_eq!(raise.kind, StdSymbolKind::EffectAction);
    let StdDecl::EffectAction(action) = &raise.decl else {
        panic!("Error.raise should be an effect action");
    };
    assert_eq!(action.owner, "Error");
    assert_eq!(action.name, "raise");
    assert_eq!(action.output, StdType::Primitive(StdPrimitiveType::Never));
}

#[test]
fn registry_stores_structured_std_flow_types() {
    let registry = standard_registry();

    let read_all = registry
        .lookup_qualified(&["std", "io", "read_all"])
        .expect("std.io.read_all should exist");
    let StdDecl::Flow(flow) = &read_all.decl else {
        panic!("std.io.read_all should be a flow");
    };
    assert!(flow.params.is_empty());
    assert_eq!(flow.output, StdType::Primitive(StdPrimitiveType::String));
    assert_eq!(flow.public_effects, vec![error_effect("std.io.IOError")]);
    assert_eq!(
        flow.requested_actions,
        vec![StdEffectRef::new(&["Console", "stdin_read_all"])]
    );

    let command_run = registry
        .lookup_qualified(&["std", "host", "command", "run"])
        .expect("std.host.command.run should exist");
    let StdDecl::Flow(flow) = &command_run.decl else {
        panic!("std.host.command.run should be a flow");
    };
    assert!(flow.public_effects.is_empty());
    assert_eq!(
        flow.requested_actions,
        vec![StdEffectRef::wildcard(&["Command", "run"], 1)]
    );

    let prompt_new = registry
        .lookup_qualified(&["std", "agent", "prompt", "new"])
        .expect("std.agent.prompt.new should exist");
    let StdDecl::Flow(flow) = &prompt_new.decl else {
        panic!("std.agent.prompt.new should be a flow");
    };
    assert_eq!(flow.output, StdType::Prompt);

    let memory_region = registry
        .lookup_qualified(&["std", "memory", "region"])
        .expect("std.memory.region should exist");
    let StdDecl::Flow(flow) = &memory_region.decl else {
        panic!("std.memory.region should be a flow");
    };
    assert_eq!(
        flow.output,
        StdType::ResourceHandleMemoryRegion(Box::new(StdType::Var("S".to_owned())))
    );
    assert!(flow.public_effects.is_empty());
    assert!(flow.requested_actions.is_empty());

    let memory_keys = registry
        .lookup_qualified(&["std", "memory", "keys"])
        .expect("std.memory.keys should exist");
    let StdDecl::Flow(flow) = &memory_keys.decl else {
        panic!("std.memory.keys should be a flow");
    };
    assert_eq!(
        flow.params,
        vec![StdType::Store {
            key: Box::new(StdType::Var("K".to_owned())),
            value: Box::new(StdType::Var("V".to_owned())),
        }]
    );
    assert_eq!(
        flow.output,
        StdType::List(Box::new(StdType::Var("K".to_owned())))
    );
    assert!(flow.public_effects.is_empty());
    assert_eq!(flow.requested_actions, vec![store_action("read")]);

    let memory_select = registry
        .lookup_qualified(&["std", "memory", "select"])
        .expect("std.memory.select should exist");
    let StdDecl::Flow(flow) = &memory_select.decl else {
        panic!("std.memory.select should be a flow");
    };
    assert_eq!(
        flow.output,
        StdType::MemorySelection(Box::new(StdType::Var("V".to_owned())))
    );
    assert!(flow.public_effects.is_empty());
    assert_eq!(flow.requested_actions, vec![store_action("read")]);

    let memory_put = registry
        .lookup_qualified(&["std", "memory", "put"])
        .expect("std.memory.put should exist");
    let StdDecl::Flow(flow) = &memory_put.decl else {
        panic!("std.memory.put should be a flow");
    };
    assert!(flow.public_effects.is_empty());
    assert_eq!(flow.requested_actions, vec![store_action("write")]);

    let len = registry
        .lookup_qualified(&["std", "collections", "len"])
        .expect("std.collections.len should exist");
    let StdDecl::Flow(flow) = &len.decl else {
        panic!("std.collections.len should be a flow");
    };
    assert_eq!(
        flow.params,
        vec![StdType::Support(StdSupportConstraint::LengthInput)]
    );
    assert_eq!(flow.output, StdType::Primitive(StdPrimitiveType::USize));

    let is_empty = registry
        .lookup_qualified(&["std", "collections", "is_empty"])
        .expect("std.collections.is_empty should exist");
    let StdDecl::Flow(flow) = &is_empty.decl else {
        panic!("std.collections.is_empty should be a flow");
    };
    assert_eq!(
        flow.params,
        vec![StdType::Support(StdSupportConstraint::EmptinessInput)]
    );
    assert_eq!(flow.output, StdType::Primitive(StdPrimitiveType::Bool));

    let contains_key = registry
        .lookup_qualified(&["std", "collections", "contains_key"])
        .expect("std.collections.contains_key should exist");
    let StdDecl::Flow(flow) = &contains_key.decl else {
        panic!("std.collections.contains_key should be a flow");
    };
    assert_eq!(
        flow.params,
        vec![
            StdType::Map {
                key: Box::new(StdType::Var("K".to_owned())),
                value: Box::new(StdType::Var("V".to_owned())),
            },
            StdType::Var("K".to_owned())
        ]
    );

    let cost = registry
        .lookup_qualified(&["std", "runtime", "limits", "Cost"])
        .expect("std.runtime.limits.Cost should exist");
    let StdDecl::Requirement(requirement) = &cost.decl else {
        panic!("std.runtime.limits.Cost should be a requirement");
    };
    assert_eq!(requirement.params, vec![StdType::Named("Money".to_owned())]);

    let wall_time = registry
        .lookup_qualified(&["std", "runtime", "limits", "WallTime"])
        .expect("std.runtime.limits.WallTime should exist");
    let StdDecl::Requirement(requirement) = &wall_time.decl else {
        panic!("std.runtime.limits.WallTime should be a requirement");
    };
    assert_eq!(
        requirement.params,
        vec![StdType::Named("Duration".to_owned())]
    );
}

#[test]
fn registry_declares_edk_facing_substrate_modules() {
    let registry = standard_registry();

    for path in [
        &["std", "stream", "StreamRead"][..],
        &["std", "stream", "StreamError"][..],
        &["std", "http", "codec", "HttpWireRequest"][..],
        &["std", "http", "codec", "HttpWireResponseHead"][..],
        &["std", "http", "codec", "HttpWireResponse"][..],
        &["std", "crypto", "Digest"][..],
        &["std", "crypto", "CryptoError"][..],
        &["std", "browser", "protocol", "BrowserScreenshot"][..],
    ] {
        let symbol = registry
            .lookup_qualified(path)
            .unwrap_or_else(|| panic!("{} should be registered", path.join(".")));
        assert_eq!(symbol.kind, StdSymbolKind::Type);
    }

    for path in [
        &["std", "stream", "TimedOut"][..],
        &["std", "stream", "Cancelled"][..],
        &["std", "stream", "Closed"][..],
        &["std", "stream", "Interrupted"][..],
        &["std", "stream", "LimitExceeded"][..],
    ] {
        let symbol = registry
            .lookup_qualified(path)
            .unwrap_or_else(|| panic!("{} should be registered", path.join(".")));
        assert_eq!(symbol.kind, StdSymbolKind::Value);
        let StdDecl::Value(value) = &symbol.decl else {
            panic!("{} should be a value", path.join("."));
        };
        assert_eq!(value.ty, StdType::Named("StreamError".to_owned()));
    }

    let host = registry
        .lookup_qualified(&["std", "stream", "Host"])
        .expect("std.stream.Host should be registered");
    assert_eq!(host.kind, StdSymbolKind::Constructor);
    let StdDecl::Flow(flow) = &host.decl else {
        panic!("std.stream.Host should be a constructor flow");
    };
    assert_eq!(
        flow.params,
        vec![StdType::Primitive(StdPrimitiveType::String)]
    );
    assert_eq!(flow.output, StdType::Named("StreamError".to_owned()));

    for (path, action_owner, action_name, selector_arity, error_type) in [
        (
            &["std", "net", "tcp", "connect"][..],
            "Net",
            "tcp_connect",
            2,
            "std.net.tcp.NetworkError",
        ),
        (
            &["std", "stream", "read"][..],
            "Stream",
            "read",
            1,
            "std.stream.StreamError",
        ),
        (
            &["std", "stream", "read_until_limit"][..],
            "Stream",
            "read",
            1,
            "std.stream.StreamError",
        ),
        (
            &["std", "stream", "write_all"][..],
            "Stream",
            "write",
            1,
            "std.stream.StreamError",
        ),
        (
            &["std", "stream", "flush"][..],
            "Stream",
            "flush",
            1,
            "std.stream.StreamError",
        ),
        (
            &["std", "stream", "close"][..],
            "Stream",
            "close",
            1,
            "std.stream.StreamError",
        ),
        (
            &["std", "tls", "connect"][..],
            "Tls",
            "handshake",
            1,
            "std.tls.TlsError",
        ),
        (
            &["std", "fs", "read_bytes"][..],
            "Fs",
            "read",
            1,
            "std.fs.IOError",
        ),
        (
            &["std", "fs", "write_bytes"][..],
            "Fs",
            "write",
            1,
            "std.fs.IOError",
        ),
        (
            &["std", "fs", "list"][..],
            "Fs",
            "list",
            1,
            "std.fs.IOError",
        ),
        (
            &["std", "fs", "stat"][..],
            "Fs",
            "stat",
            1,
            "std.fs.IOError",
        ),
        (
            &["std", "fs", "atomic_replace"][..],
            "Fs",
            "atomic_replace",
            1,
            "std.fs.IOError",
        ),
        (
            &["std", "secret", "read"][..],
            "Secret",
            "read",
            1,
            "SecretError",
        ),
        (
            &["std", "browser", "protocol", "attach"][..],
            "Browser",
            "attach",
            1,
            "BrowserError",
        ),
        (
            &["std", "browser", "protocol", "create"][..],
            "Browser",
            "attach",
            1,
            "BrowserError",
        ),
        (
            &["std", "browser", "protocol", "send"][..],
            "Browser",
            "send",
            1,
            "BrowserError",
        ),
        (
            &["std", "browser", "protocol", "recv"][..],
            "Browser",
            "recv",
            1,
            "BrowserError",
        ),
        (
            &["std", "browser", "protocol", "screenshot"][..],
            "Browser",
            "screenshot",
            1,
            "BrowserError",
        ),
        (
            &["std", "browser", "protocol", "close"][..],
            "Browser",
            "close",
            1,
            "BrowserError",
        ),
    ] {
        let symbol = registry
            .lookup_qualified(path)
            .unwrap_or_else(|| panic!("{} should be registered", path.join(".")));
        assert_eq!(symbol.kind, StdSymbolKind::Flow);
        let StdDecl::Flow(flow) = &symbol.decl else {
            panic!("{} should be a flow", path.join("."));
        };
        assert_eq!(flow.public_effects, vec![error_effect(error_type)]);
        let selector_type_param = match action_owner {
            "Stream" => Some("S"),
            "Fs" => Some("R"),
            "Secret" => Some("K"),
            _ => None,
        };
        let expected_action = selector_type_param.map_or_else(
            || StdEffectRef::wildcard(&[action_owner, action_name], selector_arity),
            |name| StdEffectRef::typed(&[action_owner, action_name], StdType::Var(name.to_owned())),
        );
        assert_eq!(flow.requested_actions, vec![expected_action]);
        let intrinsic = symbol
            .intrinsic
            .as_ref()
            .unwrap_or_else(|| panic!("{} should carry intrinsic metadata", path.join(".")));
        assert_eq!(intrinsic.purity, IntrinsicPurity::Host);
        assert_eq!(intrinsic.dispatch, IntrinsicDispatch::Host);
    }

    for path in [
        &["std", "http", "codec", "encode_request"][..],
        &["std", "http", "codec", "decode_response_head"][..],
        &["std", "http", "codec", "decode_response"][..],
        &["std", "codec", "text", "utf8_decode"][..],
        &["std", "codec", "text", "utf8_encode"][..],
        &["std", "crypto", "sha256"][..],
        &["std", "crypto", "constant_time_eq"][..],
    ] {
        let symbol = registry
            .lookup_qualified(path)
            .unwrap_or_else(|| panic!("{} should be registered", path.join(".")));
        assert_eq!(symbol.kind, StdSymbolKind::Flow);
        let StdDecl::Flow(flow) = &symbol.decl else {
            panic!("{} should be a flow", path.join("."));
        };
        assert!(flow.public_effects.is_empty());
        assert!(flow.requested_actions.is_empty());
        let intrinsic = symbol
            .intrinsic
            .as_ref()
            .unwrap_or_else(|| panic!("{} should carry intrinsic metadata", path.join(".")));
        assert_eq!(intrinsic.purity, IntrinsicPurity::Pure);
        assert_eq!(intrinsic.dispatch, IntrinsicDispatch::PureKernel);
    }

    let hmac = registry
        .lookup_qualified(&["std", "crypto", "hmac_sha256"])
        .expect("std.crypto.hmac_sha256 should be registered");
    let StdDecl::Flow(hmac_flow) = &hmac.decl else {
        panic!("std.crypto.hmac_sha256 should be a flow");
    };
    assert_eq!(hmac_flow.public_effects, vec![error_effect("CryptoError")]);
    assert_eq!(
        hmac_flow.requested_actions,
        vec![StdEffectRef::wildcard(&["Secret", "use"], 1)]
    );
    let hmac_intrinsic = hmac.intrinsic.as_ref().expect("hmac intrinsic metadata");
    assert_eq!(hmac_intrinsic.purity, IntrinsicPurity::Host);
    assert_eq!(hmac_intrinsic.dispatch, IntrinsicDispatch::Host);
}

#[test]
fn registry_declares_substrate_effect_action_metadata() {
    let registry = standard_registry();

    for (owner, action, stable_id) in [
        ("Net", "tcp_connect", 130),
        ("Stream", "read", 131),
        ("Stream", "write", 132),
        ("Stream", "flush", 133),
        ("Stream", "close", 134),
        ("Tls", "handshake", 135),
        ("Fs", "read", 136),
        ("Fs", "write", 137),
        ("Fs", "list", 138),
        ("Fs", "stat", 139),
        ("Fs", "atomic_replace", 140),
        ("Secret", "read", 85),
        ("Secret", "use", 86),
        ("Browser", "attach", 141),
        ("Browser", "send", 142),
        ("Browser", "recv", 143),
        ("Browser", "close", 144),
        ("Browser", "screenshot", 145),
    ] {
        let symbol = registry
            .lookup_qualified(&["std", "effects", "actions", owner, action])
            .unwrap_or_else(|| panic!("std.effects.actions.{owner}.{action} should exist"));
        let StdDecl::EffectAction(decl) = &symbol.decl else {
            panic!("{owner}.{action} should be an effect action");
        };
        assert_eq!(decl.stable_id, Some(stable_id));
        assert!(
            decl.runtime_requirement.is_some(),
            "{owner}.{action} should carry runtime metadata"
        );
    }
}

#[test]
fn registry_rejects_runtime_payload_names_as_static_action_selectors() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let actions = builder.module(&["std", "effects", "actions", "Probe"], "probe actions");
    builder.symbol(
        actions,
        "read",
        StdSymbolKind::EffectAction,
        StdDecl::EffectAction(
            EffectActionDecl::new("Probe", "read", &["string"], "unit")
                .with_effect_args(&[EffectActionArgKind::StringPattern]),
        ),
        "probe action",
    );
    let module = builder.module(&["std", "probe"], "probe module");
    builder.symbol(
        module,
        "read",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_actions(
            "read",
            &["string"],
            "unit",
            &[],
            &[StdEffectRef::with_args(
                &["Probe", "read"],
                vec![StdStaticArg::path(&["path"])],
            )],
        )),
        "invalid probe flow",
    );

    let error = builder
        .try_finish()
        .expect_err("unresolved runtime payload selector must be rejected");
    assert!(error.reason.contains("unknown static resource `path`"));
}

#[test]
fn registry_rejects_undeclared_generic_effect_selectors() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let effects = builder.module(&["std", "effects"], "effects");
    builder.symbol(
        effects,
        "Error",
        StdSymbolKind::Effect,
        StdDecl::Effect(EffectDecl::generic_core("Error", &["E"])),
        "typed error",
    );
    let module = builder.module(&["std", "probe"], "probe module");
    builder.symbol(
        module,
        "bad",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::effectful(
            "bad",
            &[],
            "unit",
            &[StdEffectRef::typed(
                &["Error"],
                StdType::Var("T".to_owned()),
            )],
        )),
        "invalid probe flow",
    );

    let error = builder
        .try_finish()
        .expect_err("undeclared generic selector must be rejected");
    assert!(error.reason.contains("undeclared generic `T`"));
}

#[test]
fn registry_rejects_duplicate_qualified_symbols() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "probe module");
    for _ in 0..2 {
        builder.symbol(
            module,
            "duplicate",
            StdSymbolKind::Flow,
            StdDecl::Flow(FlowDecl::pure("duplicate", &[], "unit")),
            "duplicate flow",
        );
    }

    let error = builder
        .try_finish()
        .expect_err("duplicate qualified symbols must be rejected");
    assert!(error.reason.contains("duplicate qualified"), "{error}");
}

#[test]
fn registry_rejects_duplicate_generic_parameter_names() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "probe module");
    builder.symbol(
        module,
        "duplicate_generics",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            "duplicate_generics",
            &[StdGenericParam::new("T"), StdGenericParam::new("T")],
            &[],
            "unit",
            &[],
            &[],
        )),
        "invalid generic declaration",
    );

    let error = builder
        .try_finish()
        .expect_err("duplicate generic names must be rejected");
    assert!(error.reason.contains("duplicate generic parameter `T`"));
}

#[test]
fn registry_rejects_unknown_generic_bound_specs() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "probe module");
    builder.symbol(
        module,
        "unknown_bound",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            "unknown_bound",
            &[StdGenericParam::bounded(
                "T",
                &[StdSpecRef::new(&["std", "probe", "MissingSpec"])],
            )],
            &["T"],
            "T",
            &[],
            &[],
        )),
        "invalid generic bound",
    );

    let error = builder
        .try_finish()
        .expect_err("unknown bound specs must be rejected");
    assert!(
        error
            .reason
            .contains("unknown spec `std.probe.MissingSpec`")
    );
}

#[test]
fn registry_rejects_generic_bound_spec_arity_mismatch() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "probe module");
    builder.symbol(
        module,
        "Parameterized",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic(
            "Parameterized",
            &["A"],
            TypeDeclKind::Spec,
        )),
        "parameterized spec",
    );
    builder.symbol(
        module,
        "bad_bound_arity",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            "bad_bound_arity",
            &[StdGenericParam::bounded(
                "T",
                &[StdSpecRef::new(&["std", "probe", "Parameterized"])],
            )],
            &["T"],
            "T",
            &[],
            &[],
        )),
        "invalid generic bound arity",
    );

    let error = builder
        .try_finish()
        .expect_err("bound spec arity mismatches must be rejected");
    assert!(
        error
            .reason
            .contains("expects 1 argument(s) but received 0")
    );
}

#[test]
fn registry_rejects_invalid_spec_implementation_self_type() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "probe module");
    builder.symbol(
        module,
        "Marker",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Marker", &[], TypeDeclKind::Spec)),
        "marker spec",
    );
    builder.spec_impl(StdImplFact::new(
        StdType::Named("std.probe.Missing".to_owned()),
        StdSpecRef::new(&["std", "probe", "Marker"]),
    ));

    let error = builder
        .try_finish()
        .expect_err("impl facts with unknown self types must be rejected");
    assert!(
        error
            .reason
            .contains("unknown standard type `std.probe.Missing`")
    );
}

#[test]
fn registry_rejects_unknown_nested_flow_signature_type() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "probe module");
    builder.symbol(
        module,
        "bad_flow",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl {
            name: "bad_flow".to_owned(),
            type_params: Vec::new(),
            params: vec![StdType::Array(Box::new(StdType::Named(
                "std.probe.Missing".to_owned(),
            )))],
            output: StdType::Primitive(StdPrimitiveType::Unit),
            public_effects: Vec::new(),
            requested_actions: Vec::new(),
            source_method: None,
        }),
        "invalid flow signature",
    );

    let error = builder
        .try_finish()
        .expect_err("nested unknown flow parameter type must be rejected");
    assert!(
        error
            .reason
            .contains("unknown standard type `std.probe.Missing`")
    );
}

#[test]
fn registry_rejects_unknown_nominal_representation_type() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "probe module");
    builder.symbol(
        module,
        "Broken",
        StdSymbolKind::Type,
        StdDecl::Type(
            TypeDecl::generic("Broken", &[], TypeDeclKind::Wrapper)
                .with_representation(StdType::Named("std.probe.Missing".to_owned())),
        ),
        "invalid representation",
    );

    let error = builder
        .try_finish()
        .expect_err("unknown nominal representation type must be rejected");
    assert!(
        error
            .reason
            .contains("unknown standard type `std.probe.Missing`")
    );
}

#[test]
fn registry_rejects_unknown_source_method_receiver_type() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "probe module");
    builder.symbol(
        module,
        "bad_method",
        StdSymbolKind::Flow,
        StdDecl::Flow(
            FlowDecl::pure("bad_method", &[], "unit")
                .with_value_method("std.probe.Missing", "bad_method"),
        ),
        "invalid source method",
    );

    let error = builder
        .try_finish()
        .expect_err("unknown source method receiver type must be rejected");
    assert!(
        error
            .reason
            .contains("unknown standard type `std.probe.Missing`")
    );
}

#[test]
fn registry_rejects_unknown_type_nested_in_spec_argument() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "probe"], "probe module");
    builder.symbol(
        module,
        "Parameterized",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic(
            "Parameterized",
            &["A"],
            TypeDeclKind::Spec,
        )),
        "parameterized spec",
    );
    builder.symbol(
        module,
        "bad_bound",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_type_params_actions(
            "bad_bound",
            &[StdGenericParam::bounded(
                "T",
                &[StdSpecRef::with_args(
                    &["std", "probe", "Parameterized"],
                    vec![StdType::Named("std.probe.Missing".to_owned())],
                )],
            )],
            &["T"],
            "T",
            &[],
            &[],
        )),
        "invalid spec argument",
    );

    let error = builder
        .try_finish()
        .expect_err("unknown type nested in a spec argument must be rejected");
    assert!(
        error
            .reason
            .contains("unknown standard type `std.probe.Missing`")
    );
}

fn error_effect(error_type: &str) -> StdEffectRef {
    StdEffectRef::typed(&["Error"], StdType::parse(error_type))
}

fn store_action(action: &str) -> StdEffectRef {
    StdEffectRef::with_args(
        &["Memory", action],
        vec![StdStaticArg::path(&["std", "memory", "Store"])],
    )
}
