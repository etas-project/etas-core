use std::collections::BTreeMap;

use crate::{
    EffectActionArgKind, EffectActionDecl, EffectDecl, StdDecl, StdGenericParam, StdModuleId,
    StdRegistryBuilder, StdRuntimeRequirement, StdSpecRef, StdSymbolKind,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "effects"],
        "Source-language effect declarations re-exported for package imports.",
    );

    for effect in STANDARD_EFFECTS {
        let symbol = builder.symbol(
            module,
            effect.name,
            StdSymbolKind::Effect,
            if effect.name == "Error" {
                StdDecl::Effect(EffectDecl::generic_core("Error", &["E"]))
            } else if effect.core {
                StdDecl::Effect(EffectDecl::core(effect.name))
            } else {
                let mut decl = EffectDecl::standard(effect.name, effect.extends)
                    .with_stable_id(effect.stable_id)
                    .with_runtime_requirement(effect.runtime_requirement.clone());
                if effect.high_impact_ack {
                    decl = decl.with_high_impact_ack();
                }
                StdDecl::Effect(decl)
            },
            effect.description,
        );
        if effect.prelude {
            builder.prelude(effect.name, symbol);
        }
    }

    let mut action_modules = BTreeMap::<&str, StdModuleId>::new();
    for action in STANDARD_ACTIONS {
        let action_module = *action_modules.entry(action.owner).or_insert_with(|| {
            builder.module(
                &["std", "effects", "actions", action.owner],
                "Standard effect action declarations for one effect owner.",
            )
        });
        builder.symbol(
            action_module,
            action.name,
            StdSymbolKind::EffectAction,
            StdDecl::EffectAction({
                let decl =
                    EffectActionDecl::new(action.owner, action.name, action.params, action.output)
                        .with_effect_args(action.effect_args)
                        .with_stable_id(action.stable_id)
                        .with_runtime_requirement(action.runtime_requirement.clone());
                let mut decl = declare_standard_action_generics(action, decl);
                if action.high_impact_ack {
                    decl = decl.with_high_impact_ack();
                }
                decl
            }),
            action.description,
        );
    }
}

fn declare_standard_action_generics(
    action: &StandardActionSpec,
    decl: EffectActionDecl,
) -> EffectActionDecl {
    match action.owner {
        "Stream" => decl
            .with_type_params(&[StdGenericParam::bounded(
                "S",
                &[StdSpecRef::new(&["std", "stream", "ByteStream"])],
            )])
            .with_selector_param_names(&["S"]),
        "Fs" => decl
            .with_type_params(&[StdGenericParam::bounded(
                "R",
                &[StdSpecRef::new(&["std", "fs", "Region"])],
            )])
            .with_selector_param_names(&["R"]),
        "Secret" => decl
            .with_type_params(&[StdGenericParam::new("K")])
            .with_selector_param_names(&["K"]),
        "Agentic" => decl
            .with_type_params(&[StdGenericParam::new("O")])
            .with_selector_param_names(&["C", "O"]),
        "Memory" => decl.with_type_params(&[StdGenericParam::new("R")]),
        _ => decl,
    }
}

struct StandardEffectSpec {
    name: &'static str,
    extends: &'static [&'static str],
    core: bool,
    prelude: bool,
    stable_id: u32,
    runtime_requirement: StdRuntimeRequirement,
    high_impact_ack: bool,
    description: &'static str,
}

struct StandardActionSpec {
    owner: &'static str,
    name: &'static str,
    params: &'static [&'static str],
    output: &'static str,
    stable_id: u32,
    runtime_requirement: StdRuntimeRequirement,
    effect_args: &'static [EffectActionArgKind],
    high_impact_ack: bool,
    description: &'static str,
}

const STANDARD_EFFECTS: &[StandardEffectSpec] = &[
    StandardEffectSpec {
        name: "Agentic",
        extends: &[],
        core: true,
        prelude: false,
        stable_id: 0,
        runtime_requirement: StdRuntimeRequirement::Agentic,
        high_impact_ack: false,
        description: "Core agent/model inference effect.",
    },
    StandardEffectSpec {
        name: "Network",
        extends: &[],
        core: true,
        prelude: false,
        stable_id: 1,
        runtime_requirement: StdRuntimeRequirement::Network,
        high_impact_ack: false,
        description: "Core network effect.",
    },
    StandardEffectSpec {
        name: "FileIO",
        extends: &[],
        core: true,
        prelude: false,
        stable_id: 2,
        runtime_requirement: StdRuntimeRequirement::FileIO,
        high_impact_ack: false,
        description: "Core filesystem and file-like IO effect.",
    },
    StandardEffectSpec {
        name: "Command",
        extends: &[],
        core: true,
        prelude: false,
        stable_id: 3,
        runtime_requirement: StdRuntimeRequirement::Command,
        high_impact_ack: true,
        description: "Core command execution effect.",
    },
    StandardEffectSpec {
        name: "Memory",
        extends: &[],
        core: true,
        prelude: true,
        stable_id: 4,
        runtime_requirement: StdRuntimeRequirement::DurableMemory,
        high_impact_ack: false,
        description: "Core durable memory effect.",
    },
    StandardEffectSpec {
        name: "Secret",
        extends: &[],
        core: true,
        prelude: true,
        stable_id: 6,
        runtime_requirement: StdRuntimeRequirement::SecretAccess,
        high_impact_ack: true,
        description: "Core secret access effect.",
    },
    StandardEffectSpec {
        name: "Time",
        extends: &[],
        core: true,
        prelude: false,
        stable_id: 7,
        runtime_requirement: StdRuntimeRequirement::Time,
        high_impact_ack: false,
        description: "Core time effect.",
    },
    StandardEffectSpec {
        name: "Human",
        extends: &[],
        core: true,
        prelude: false,
        stable_id: 10,
        runtime_requirement: StdRuntimeRequirement::HostAuthority,
        high_impact_ack: false,
        description: "Core human-interaction effect.",
    },
    StandardEffectSpec {
        name: "Error",
        extends: &[],
        core: true,
        prelude: true,
        stable_id: 8,
        runtime_requirement: StdRuntimeRequirement::RuntimeHandler,
        high_impact_ack: false,
        description: "Core typed error effect.",
    },
    StandardEffectSpec {
        name: "Console",
        extends: &["FileIO"],
        core: false,
        prelude: true,
        stable_id: 9,
        runtime_requirement: StdRuntimeRequirement::Console,
        high_impact_ack: false,
        description: "Console input/output through the runtime console host boundary.",
    },
    StandardEffectSpec {
        name: "Approval",
        extends: &["Human"],
        core: false,
        prelude: true,
        stable_id: 5,
        runtime_requirement: StdRuntimeRequirement::Approval,
        high_impact_ack: true,
        description: "Human approval effect through the runtime approval boundary.",
    },
    StandardEffectSpec {
        name: "Clock",
        extends: &["Time"],
        core: false,
        prelude: false,
        stable_id: 21,
        runtime_requirement: StdRuntimeRequirement::Time,
        high_impact_ack: false,
        description: "Standard clock action boundary.",
    },
    StandardEffectSpec {
        name: "Net",
        extends: &["Network"],
        core: false,
        prelude: false,
        stable_id: 22,
        runtime_requirement: StdRuntimeRequirement::Network,
        high_impact_ack: false,
        description: "TCP connection substrate action boundary.",
    },
    StandardEffectSpec {
        name: "Stream",
        extends: &[],
        core: false,
        prelude: false,
        stable_id: 23,
        runtime_requirement: StdRuntimeRequirement::Network,
        high_impact_ack: false,
        description: "Bounded byte stream substrate action boundary.",
    },
    StandardEffectSpec {
        name: "Tls",
        extends: &["Network"],
        core: false,
        prelude: false,
        stable_id: 24,
        runtime_requirement: StdRuntimeRequirement::Network,
        high_impact_ack: false,
        description: "TLS client session substrate action boundary.",
    },
    StandardEffectSpec {
        name: "Fs",
        extends: &["FileIO"],
        core: false,
        prelude: false,
        stable_id: 25,
        runtime_requirement: StdRuntimeRequirement::FileIO,
        high_impact_ack: false,
        description: "Project-scoped filesystem substrate action boundary.",
    },
    StandardEffectSpec {
        name: "Browser",
        extends: &["Network"],
        core: false,
        prelude: false,
        stable_id: 26,
        runtime_requirement: StdRuntimeRequirement::Network,
        high_impact_ack: false,
        description: "Browser protocol/session substrate action boundary.",
    },
];

const STANDARD_ACTIONS: &[StandardActionSpec] = &[
    StandardActionSpec {
        owner: "Console",
        name: "stdin_read_line",
        params: &[],
        output: "string",
        stable_id: 16,
        runtime_requirement: StdRuntimeRequirement::Console,
        effect_args: &[],
        high_impact_ack: false,
        description: "Console standard-input line read action.",
    },
    StandardActionSpec {
        owner: "Console",
        name: "stdin_read_all",
        params: &[],
        output: "string",
        stable_id: 17,
        runtime_requirement: StdRuntimeRequirement::Console,
        effect_args: &[],
        high_impact_ack: false,
        description: "Console standard-input full read action.",
    },
    StandardActionSpec {
        owner: "Console",
        name: "stdout_write",
        params: &["string"],
        output: "unit",
        stable_id: 18,
        runtime_requirement: StdRuntimeRequirement::Console,
        effect_args: &[],
        high_impact_ack: false,
        description: "Console standard-output write action.",
    },
    StandardActionSpec {
        owner: "Console",
        name: "stderr_write",
        params: &["string"],
        output: "unit",
        stable_id: 19,
        runtime_requirement: StdRuntimeRequirement::Console,
        effect_args: &[],
        high_impact_ack: false,
        description: "Console standard-error write action.",
    },
    StandardActionSpec {
        owner: "Command",
        name: "run",
        params: &["Command"],
        output: "CommandResult",
        stable_id: 83,
        runtime_requirement: StdRuntimeRequirement::Command,
        effect_args: &[EffectActionArgKind::StaticResourcePath {
            ty: "std.host.sandbox.SandboxProfile",
        }],
        high_impact_ack: true,
        description: "Command run action.",
    },
    StandardActionSpec {
        owner: "Secret",
        name: "read",
        params: &["SecretKey[K]"],
        output: "std.secret.SecretValue[K]",
        stable_id: 85,
        runtime_requirement: StdRuntimeRequirement::SecretAccess,
        effect_args: &[EffectActionArgKind::Type],
        high_impact_ack: true,
        description: "Secret read action.",
    },
    StandardActionSpec {
        owner: "Secret",
        name: "use",
        params: &["std.secret.SecretValue[K]"],
        output: "unit",
        stable_id: 86,
        runtime_requirement: StdRuntimeRequirement::SecretAccess,
        effect_args: &[EffectActionArgKind::Type],
        high_impact_ack: true,
        description: "Non-revealing use of an opaque secret value.",
    },
    StandardActionSpec {
        owner: "Agentic",
        name: "infer",
        params: &["Prompt", "Schema[O]"],
        output: "O",
        stable_id: 88,
        runtime_requirement: StdRuntimeRequirement::Agentic,
        effect_args: &[
            EffectActionArgKind::StaticResourcePath { ty: "Agent" },
            EffectActionArgKind::Type,
        ],
        high_impact_ack: false,
        description: "Agentic inference action.",
    },
    StandardActionSpec {
        owner: "Memory",
        name: "read",
        params: &["MemoryRegion[R]"],
        output: "unit",
        stable_id: 0,
        runtime_requirement: StdRuntimeRequirement::DurableMemory,
        effect_args: &[EffectActionArgKind::MemoryPlace],
        high_impact_ack: false,
        description: "Memory read action.",
    },
    StandardActionSpec {
        owner: "Memory",
        name: "write",
        params: &["MemoryRegion[R]"],
        output: "unit",
        stable_id: 1,
        runtime_requirement: StdRuntimeRequirement::DurableMemory,
        effect_args: &[EffectActionArgKind::MemoryPlace],
        high_impact_ack: true,
        description: "Memory write action.",
    },
    StandardActionSpec {
        owner: "Approval",
        name: "request",
        params: &["ApprovalRequest"],
        output: "ApprovalDecision",
        stable_id: 32,
        runtime_requirement: StdRuntimeRequirement::Approval,
        effect_args: &[],
        high_impact_ack: true,
        description: "Approval request action.",
    },
    StandardActionSpec {
        owner: "Clock",
        name: "now",
        params: &[],
        output: "Time",
        stable_id: 119,
        runtime_requirement: StdRuntimeRequirement::Time,
        effect_args: &[],
        high_impact_ack: false,
        description: "Clock now action.",
    },
    StandardActionSpec {
        owner: "Clock",
        name: "sleep",
        params: &["std.runtime.time.Duration"],
        output: "unit",
        stable_id: 120,
        runtime_requirement: StdRuntimeRequirement::Time,
        effect_args: &[],
        high_impact_ack: false,
        description: "Clock sleep action.",
    },
    StandardActionSpec {
        owner: "Net",
        name: "tcp_connect",
        params: &["std.net.tcp.Host", "std.net.tcp.Port"],
        output: "std.net.tcp.TcpStream",
        stable_id: 130,
        runtime_requirement: StdRuntimeRequirement::Tcp,
        effect_args: &[
            EffectActionArgKind::StringPattern,
            EffectActionArgKind::StringPattern,
        ],
        high_impact_ack: false,
        description: "TCP connect action.",
    },
    StandardActionSpec {
        owner: "Stream",
        name: "read",
        params: &["S"],
        output: "StreamRead",
        stable_id: 131,
        runtime_requirement: StdRuntimeRequirement::Stream,
        effect_args: &[EffectActionArgKind::Type],
        high_impact_ack: false,
        description: "Byte stream read action.",
    },
    StandardActionSpec {
        owner: "Stream",
        name: "write",
        params: &["S"],
        output: "unit",
        stable_id: 132,
        runtime_requirement: StdRuntimeRequirement::Stream,
        effect_args: &[EffectActionArgKind::Type],
        high_impact_ack: false,
        description: "Byte stream write action.",
    },
    StandardActionSpec {
        owner: "Stream",
        name: "flush",
        params: &["S"],
        output: "unit",
        stable_id: 133,
        runtime_requirement: StdRuntimeRequirement::Stream,
        effect_args: &[EffectActionArgKind::Type],
        high_impact_ack: false,
        description: "Byte stream flush action.",
    },
    StandardActionSpec {
        owner: "Stream",
        name: "close",
        params: &["S"],
        output: "unit",
        stable_id: 134,
        runtime_requirement: StdRuntimeRequirement::Stream,
        effect_args: &[EffectActionArgKind::Type],
        high_impact_ack: false,
        description: "Byte stream close action.",
    },
    StandardActionSpec {
        owner: "Tls",
        name: "handshake",
        params: &["std.tls.Host"],
        output: "std.tls.TlsStream",
        stable_id: 135,
        runtime_requirement: StdRuntimeRequirement::Tls,
        effect_args: &[EffectActionArgKind::StringPattern],
        high_impact_ack: false,
        description: "TLS client handshake action.",
    },
    StandardActionSpec {
        owner: "Fs",
        name: "read",
        params: &["WorkspacePath[R]"],
        output: "bytes",
        stable_id: 136,
        runtime_requirement: StdRuntimeRequirement::FileIO,
        effect_args: &[EffectActionArgKind::Type],
        high_impact_ack: false,
        description: "Workspace filesystem read action.",
    },
    StandardActionSpec {
        owner: "Fs",
        name: "write",
        params: &["WorkspacePath[R]"],
        output: "unit",
        stable_id: 137,
        runtime_requirement: StdRuntimeRequirement::FileIO,
        effect_args: &[EffectActionArgKind::Type],
        high_impact_ack: true,
        description: "Workspace filesystem write action.",
    },
    StandardActionSpec {
        owner: "Fs",
        name: "list",
        params: &["WorkspacePath[R]"],
        output: "List[WorkspacePath[R]]",
        stable_id: 138,
        runtime_requirement: StdRuntimeRequirement::FileIO,
        effect_args: &[EffectActionArgKind::Type],
        high_impact_ack: false,
        description: "Workspace filesystem list action.",
    },
    StandardActionSpec {
        owner: "Fs",
        name: "stat",
        params: &["WorkspacePath[R]"],
        output: "FsStat",
        stable_id: 139,
        runtime_requirement: StdRuntimeRequirement::FileIO,
        effect_args: &[EffectActionArgKind::Type],
        high_impact_ack: false,
        description: "Workspace filesystem stat action.",
    },
    StandardActionSpec {
        owner: "Fs",
        name: "atomic_replace",
        params: &["WorkspacePath[R]"],
        output: "unit",
        stable_id: 140,
        runtime_requirement: StdRuntimeRequirement::FileIO,
        effect_args: &[EffectActionArgKind::Type],
        high_impact_ack: true,
        description: "Workspace filesystem atomic replace action.",
    },
    StandardActionSpec {
        owner: "Browser",
        name: "attach",
        params: &["BrowserProfile"],
        output: "BrowserSession",
        stable_id: 141,
        runtime_requirement: StdRuntimeRequirement::Browser,
        effect_args: &[EffectActionArgKind::StaticResourcePath {
            ty: "BrowserProfile",
        }],
        high_impact_ack: false,
        description: "Browser protocol attach action.",
    },
    StandardActionSpec {
        owner: "Browser",
        name: "send",
        params: &["BrowserSession"],
        output: "unit",
        stable_id: 142,
        runtime_requirement: StdRuntimeRequirement::Browser,
        effect_args: &[EffectActionArgKind::StaticResourcePath {
            ty: "BrowserSession",
        }],
        high_impact_ack: false,
        description: "Browser protocol send action.",
    },
    StandardActionSpec {
        owner: "Browser",
        name: "recv",
        params: &["BrowserSession"],
        output: "BrowserMessage",
        stable_id: 143,
        runtime_requirement: StdRuntimeRequirement::Browser,
        effect_args: &[EffectActionArgKind::StaticResourcePath {
            ty: "BrowserSession",
        }],
        high_impact_ack: false,
        description: "Browser protocol receive action.",
    },
    StandardActionSpec {
        owner: "Browser",
        name: "close",
        params: &["BrowserSession"],
        output: "unit",
        stable_id: 144,
        runtime_requirement: StdRuntimeRequirement::Browser,
        effect_args: &[EffectActionArgKind::StaticResourcePath {
            ty: "BrowserSession",
        }],
        high_impact_ack: false,
        description: "Browser protocol close action.",
    },
    StandardActionSpec {
        owner: "Browser",
        name: "screenshot",
        params: &["BrowserSession"],
        output: "BrowserScreenshot",
        stable_id: 145,
        runtime_requirement: StdRuntimeRequirement::Browser,
        effect_args: &[EffectActionArgKind::StaticResourcePath {
            ty: "BrowserSession",
        }],
        high_impact_ack: false,
        description: "Browser protocol screenshot action.",
    },
];
