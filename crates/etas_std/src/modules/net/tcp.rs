use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRecordField, StdRegistryBuilder, StdSymbolKind, StdType, TypeDecl,
    TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "net", "tcp"],
        "TCP substrate declarations for EDK and low-level network packages.",
    );
    for (name, representation) in [
        ("Host", Some(record(&[("host", "string")]))),
        ("Port", Some(record(&[("port", "i32")]))),
        ("TcpOptions", Some(record(&[]))),
        ("TcpStream", None),
        ("NetworkError", None),
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
            "TCP substrate support type.",
        );
    }
    substrate_flow(
        builder,
        module,
        "connect",
        &[
            "std.net.tcp.Host",
            "std.net.tcp.Port",
            "std.net.tcp.TcpOptions",
        ],
        "std.net.tcp.TcpStream",
        &["Error[std.net.tcp.NetworkError]"],
        &["Net.tcp_connect[host, port]"],
        intrinsic::runtime::NET_TCP_CONNECT,
        "Open a host-mediated TCP connection.",
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

fn substrate_flow(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    params: &[&str],
    output: &str,
    public_effects: &[&str],
    requested_actions: &[&str],
    id: u32,
    summary: &str,
) {
    builder.symbol_with_intrinsic(
        module,
        name,
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_actions(
            name,
            params,
            output,
            public_effects,
            requested_actions,
        )),
        summary,
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(id),
            qualified_path: vec!["std".into(), "net".into(), "tcp".into(), name.into()],
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
