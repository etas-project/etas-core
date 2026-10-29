use super::*;

fn payload(target: ProtoCallableRequestTarget) -> Vec<u8> {
    let mut graph = valid_graph();
    graph.public_metadata = Some(ProtoPublicMetadataSection {
        effect_summaries: vec![ProtoEffectSummary {
            item: vec!["demo".into(), "call".into()],
            public_effects: Some(ProtoEffectRow::default()),
            requested_actions: Some(ProtoEffectRow::default()),
            handled_requested_actions: Some(ProtoEffectRow::default()),
            action_trace: Some(ProtoActionTrace {
                kind: ProtoActionTraceKind::ParameterCall as i32,
                parameter: Some(target),
                ..Default::default()
            }),
            ..Default::default()
        }],
        ..Default::default()
    });
    graph.encode_to_vec()
}

#[test]
fn binary_package_graph_rejects_missing_method_target_kind_and_receiver() {
    for target in [
        ProtoCallableRequestTarget { target: None },
        ProtoCallableRequestTarget {
            target: Some(proto_callable_request_target::Target::SpecMethod(Box::new(
                ProtoSpecMethodInvocation {
                    receiver: None,
                    spec: vec!["api".into(), "Encoder".into()],
                    method: "encode".into(),
                    arity: 1,
                    ..Default::default()
                },
            ))),
        },
    ] {
        let bytes = payload(target);
        let error = package_metadata_from_package_graph_payload(&bytes).unwrap_err();
        let reason = error.to_string();
        assert!(
            reason.contains("kind") || reason.contains("receiver"),
            "{reason}"
        );
    }
}

#[test]
fn binary_package_graph_enforces_type_depth_inside_method_evidence() {
    let mut receiver = ProtoType {
        kind: "primitive".into(),
        name: "unit".into(),
        ..Default::default()
    };
    for _ in 0..70 {
        receiver = ProtoType {
            kind: "option".into(),
            children: vec![receiver],
            ..Default::default()
        };
    }
    let target = ProtoCallableRequestTarget {
        target: Some(proto_callable_request_target::Target::SpecMethod(Box::new(
            ProtoSpecMethodInvocation {
                receiver: Some(receiver),
                spec: vec!["api".into(), "Encoder".into()],
                method: "encode".into(),
                arity: 1,
                ..Default::default()
            },
        ))),
    };
    let bytes = payload(target);
    let error = package_metadata_from_package_graph_payload(&bytes).unwrap_err();
    let reason = error.to_string();
    assert!(
        reason.contains("maximum depth") || reason.contains("recursion limit"),
        "{reason}"
    );
}
