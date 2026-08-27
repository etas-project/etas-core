use crate::{
    FlowDecl, StdDecl, StdEffectRef, StdGenericParam, StdRecordField, StdRegistryBuilder,
    StdStaticArg, StdSymbolKind, StdType, TypeDecl, TypeDeclKind,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "agent", "message"], "Typed agent message support.");
    for (name, params) in [
        ("Message", &["T"][..]),
        ("MessageId", &[][..]),
        ("Participant", &[][..]),
        ("AgentId", &[][..]),
        ("TraceId", &[][..]),
        ("Provenance", &[][..]),
        ("Role", &[][..]),
    ] {
        let mut decl = TypeDecl::generic(name, params, TypeDeclKind::Support);
        if name == "Message" {
            decl = decl.with_representation(record(&[
                ("body", "T"),
                ("content", "T"),
                ("id", "std.agent.message.MessageId"),
                ("from", "Option[std.agent.message.Participant]"),
                ("role", "std.agent.message.Role"),
                ("session", "Option[std.agent.session.SessionId]"),
                ("created_at", "std.runtime.time.Time"),
                ("provenance", "Option[std.agent.message.Provenance]"),
            ]));
        }
        let symbol = builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(decl),
            "Agent message support declaration.",
        );
        if name == "Message" {
            builder.prelude(name, symbol);
        }
    }

    for (name, params, output, docs) in [
        (
            "new",
            &["T"][..],
            "Message[T]",
            "Construct a typed message from a payload.",
        ),
        (
            "cast",
            &["Message[T]"][..],
            "Option[Message[T]]",
            "Attempt a checked identity typed message cast.",
        ),
        (
            "with_session",
            &["Message[T]", "SessionConfig"][..],
            "Message[T]",
            "Attach a runtime session configuration to a typed message.",
        ),
    ] {
        let requested_actions = if name == "with_session" {
            vec![StdEffectRef::with_args(
                &["Memory", "write"],
                vec![StdStaticArg::path(&[
                    "std",
                    "agent",
                    "session",
                    "SessionId",
                ])],
            )]
        } else {
            Vec::new()
        };
        let decl = FlowDecl::with_type_params_actions(
            name,
            &[StdGenericParam::new("T")],
            params,
            output,
            &[],
            &requested_actions,
        );
        builder.symbol(module, name, StdSymbolKind::Flow, StdDecl::Flow(decl), docs);
    }
}

fn record(fields: &[(&str, &str)]) -> StdType {
    StdType::Record(
        fields
            .iter()
            .map(|(name, ty)| StdRecordField::new(name, StdType::parse(ty)))
            .collect(),
    )
}
