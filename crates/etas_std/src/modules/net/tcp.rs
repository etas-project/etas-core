use crate::{
    IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl, StdRecordField, StdRegistryBuilder,
    StdSymbolKind, StdType, TypeDecl, TypeDeclKind, intrinsic,
};

use crate::modules::registration::{IntrinsicFlowRegistration, register_intrinsic_flow};

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
    register_intrinsic_flow(
        builder,
        module,
        &["std", "net", "tcp"],
        IntrinsicFlowRegistration {
            name: "connect",
            type_params: &[],
            params: &[
                "std.net.tcp.Host",
                "std.net.tcp.Port",
                "std.net.tcp.TcpOptions",
            ],
            output: "std.net.tcp.TcpStream",
            public_effects: &["Error[std.net.tcp.NetworkError]"],
            requested_actions: &["Net.tcp_connect[host, port]"],
            intrinsic_id: intrinsic::runtime::NET_TCP_CONNECT,
            summary: "Open a host-mediated TCP connection.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
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
