use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRecordField, StdRegistryBuilder, StdSymbolKind, StdType, TypeDecl,
    TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(&["std", "tls"], "TLS substrate declarations.");
    for (name, representation) in [
        ("TcpStream", None),
        ("TlsStream", None),
        ("Host", Some(record(&[("host", "string")]))),
        ("TlsConfig", Some(record(&[]))),
        ("TlsError", None),
    ] {
        let mut decl = TypeDecl::generic(name, &[], TypeDeclKind::Support);
        if let Some(representation) = representation {
            decl = decl.with_representation(representation);
        }
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(decl),
            "TLS substrate support type.",
        );
    }
    builder.symbol_with_intrinsic(
        module,
        "connect",
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_actions(
            "connect",
            &["std.net.tcp.TcpStream", "std.tls.Host", "std.tls.TlsConfig"],
            "std.tls.TlsStream",
            &["Error[std.tls.TlsError]"],
            &["Tls.handshake[server_name]"],
        )),
        "Open a host-mediated TLS client session over a TCP stream.",
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(intrinsic::runtime::TLS_CONNECT),
            qualified_path: vec!["std".into(), "tls".into(), "connect".into()],
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}

fn record(fields: &[(&str, &str)]) -> StdType {
    StdType::Record(
        fields
            .iter()
            .map(|(name, ty)| StdRecordField::new(name, StdType::parse(ty)))
            .collect(),
    )
}
