use etas_code::{
    CallableId, CodeFingerprint, DefinitionKey, DefinitionKind, ImplementationKey, MethodId,
    PackageKey, descriptor::*,
};
use prost::Message;

use super::*;

mod body_tests;
mod header_tests;
mod snapshot_tests;

#[test]
fn parameter_nonreturn_survives_binary_trace_codec() {
    let call = ParameterCallInfo {
        handlers: Some(vec![]),
        callable: CallableId::declaration(key("visit", DefinitionKind::Flow)).unwrap(),
        signature: TypeInfoRef(4),
        position: 2,
        arguments: Some(vec![
            ParameterArgumentInfo::Bool(true),
            ParameterArgumentInfo::Integer {
                negative: true,
                magnitude: 1_u128 << 127,
            },
        ]),
    };
    let original = TraceInfo::Outcome(CallableOutcomeInfo::ParameterNonReturn(call));
    let bytes = traces::encode(&original).encode_to_vec();
    let decoded = schema::Trace::decode(bytes.as_slice()).unwrap();
    assert_eq!(traces::decode(decoded).unwrap(), original);
}

#[test]
fn parameter_integer_codec_rejects_non_128_bit_magnitude() {
    let original = TraceInfo::ParameterCall(ParameterCallInfo {
        handlers: Some(vec![]),
        callable: CallableId::declaration(key("visit", DefinitionKind::Flow)).unwrap(),
        signature: TypeInfoRef(0),
        position: 0,
        arguments: Some(vec![ParameterArgumentInfo::Integer {
            negative: false,
            magnitude: 42,
        }]),
    });
    let mut encoded = traces::encode(&original);
    let Some(schema::trace::Kind::ParameterCall(call)) = &mut encoded.kind else {
        unreachable!()
    };
    let Some(schema::parameter_argument::Kind::Integer(value)) =
        &mut call.arguments.as_mut().unwrap().values[0].kind
    else {
        unreachable!()
    };
    assert!(value.magnitude.pop().is_some());
    assert!(traces::decode(encoded).is_err());
}

#[test]
fn parameter_handler_scope_codec_rejects_unknown_scope_kind() {
    let original = TraceInfo::ParameterCall(ParameterCallInfo {
        callable: CallableId::declaration(key("visit", DefinitionKind::Flow)).unwrap(),
        signature: TypeInfoRef(0),
        position: 0,
        arguments: Some(vec![]),
        handlers: Some(vec![ParameterHandlerScopeInfo::Handle {
            arms: vec![ParameterHandlerArmInfo {
                action: EffectInfo {
                    target: EffectTargetInfo::Action {
                        effect: key("Console", DefinitionKind::Effect),
                        name: "write".into(),
                    },
                    arguments: vec![],
                },
                trace: TraceInfoRef(0),
                completion: ParameterHandlerCompletionInfo::Resume {
                    result_type: TypeInfoRef(0),
                    value: ReturnObservationInfo::Bool(true),
                },
            }],
        }]),
    });
    let mut encoded = traces::encode(&original);
    let Some(schema::trace::Kind::ParameterCall(call)) = &mut encoded.kind else {
        unreachable!()
    };
    call.handlers.as_mut().unwrap().scopes[0].kind = None;
    assert!(traces::decode(encoded).is_err());
    for control in [None, Some(9)] {
        let mut encoded = traces::encode(&original);
        let Some(schema::trace::Kind::ParameterCall(call)) = &mut encoded.kind else {
            unreachable!()
        };
        let Some(schema::parameter_handler_scope::Kind::Handle(scope)) =
            &mut call.handlers.as_mut().unwrap().scopes[0].kind
        else {
            unreachable!()
        };
        scope.arms[0].control = control;
        assert!(traces::decode(encoded).is_err());
    }
    let mut encoded = traces::encode(&original);
    let Some(schema::trace::Kind::ParameterCall(call)) = &mut encoded.kind else {
        unreachable!()
    };
    let Some(schema::parameter_handler_scope::Kind::Handle(scope)) =
        &mut call.handlers.as_mut().unwrap().scopes[0].kind
    else {
        unreachable!()
    };
    scope.arms[0].trace = None;
    assert!(traces::decode(encoded).is_err());
}

#[test]
fn parameter_nonreturn_survives_full_package_artifact_roundtrip() {
    use crate::*;
    let flow = key("visit", DefinitionKind::Flow);
    let call = ParameterCallInfo {
        handlers: Some(vec![ParameterHandlerScopeInfo::Handle {
            arms: vec![
                ParameterHandlerArmInfo {
                    action: EffectInfo {
                        target: EffectTargetInfo::Action {
                            effect: key("Console", DefinitionKind::Effect),
                            name: "write".into(),
                        },
                        arguments: vec![],
                    },
                    trace: TraceInfoRef(1),
                    completion: ParameterHandlerCompletionInfo::Resume {
                        result_type: TypeInfoRef(2),
                        value: ReturnObservationInfo::Unit,
                    },
                },
                ParameterHandlerArmInfo {
                    action: EffectInfo {
                        target: EffectTargetInfo::Action {
                            effect: key("Choice", DefinitionKind::Effect),
                            name: "decide".into(),
                        },
                        arguments: vec![],
                    },
                    trace: TraceInfoRef(1),
                    completion: ParameterHandlerCompletionInfo::Finish {
                        result_type: TypeInfoRef(3),
                        value: ReturnObservationInfo::Bool(true),
                    },
                },
            ],
        }]),
        callable: CallableId::declaration(flow).unwrap(),
        signature: TypeInfoRef(0),
        position: 0,
        arguments: Some(vec![
            ParameterArgumentInfo::Input { position: 1 },
            ParameterArgumentInfo::Integer {
                negative: true,
                magnitude: 1_u128 << 127,
            },
        ]),
    };
    let graph = DefinitionDescriptorSet {
        headers: None,
        graph: DescriptorGraphInput {
            types: vec![
                TypeInfo::Function(SignatureInfo {
                    generics: vec![],
                    parameters: vec![
                        ParameterInfo {
                            name: Some("f".into()),
                            ty: TypeInfoRef(1),
                        },
                        ParameterInfo {
                            name: Some("value".into()),
                            ty: TypeInfoRef(3),
                        },
                    ],
                    output: TypeInfoRef(2),
                    declared_output: None,
                    effects: Some(EffectRowInfoRef(0)),
                }),
                TypeInfo::Function(SignatureInfo {
                    generics: vec![],
                    parameters: vec![
                        ParameterInfo {
                            name: Some("value".into()),
                            ty: TypeInfoRef(3),
                        },
                        ParameterInfo {
                            name: Some("boundary".into()),
                            ty: TypeInfoRef(4),
                        },
                    ],
                    output: TypeInfoRef(2),
                    declared_output: None,
                    effects: Some(EffectRowInfoRef(0)),
                }),
                TypeInfo::Primitive(PrimitiveInfo::Unit),
                TypeInfo::Primitive(PrimitiveInfo::Bool),
                TypeInfo::Primitive(PrimitiveInfo::I128),
            ],
            rows: vec![
                EffectRowInfo {
                    effects: vec![],
                    tail: None,
                },
                EffectRowInfo {
                    effects: vec![EffectInfo {
                        target: EffectTargetInfo::Action {
                            effect: key("Console", DefinitionKind::Effect),
                            name: "write".into(),
                        },
                        arguments: vec![],
                    }],
                    tail: None,
                },
            ],
            traces: vec![
                TraceInfo::Outcome(CallableOutcomeInfo::ParameterNonReturn(call)),
                TraceInfo::Empty,
            ],
            ..Default::default()
        },
        declarations: vec![],
    };
    let mut metadata = PackageMetadata {
        version: 1,
        package: PackageIdentity {
            name: "descriptor-test".into(),
            version: "0.1.0".into(),
            edition: "2026".into(),
        },
        ..Default::default()
    };
    metadata.public_metadata.definition_descriptors = Some(graph);
    let header = MetadataArtifactHeader {
        artifact_schema_version: ARTIFACT_SCHEMA_VERSION,
        compiler_version: "test".into(),
        package_id: "descriptor-test".into(),
        package_version: "0.1.0".into(),
        source_payload_hash: "source".into(),
        manifest_hash: "manifest".into(),
        dependency_lock_hash: "lock".into(),
        created_target: "test".into(),
    };
    let bytes = encode_metadata_artifact(&header, package_metadata_to_sections(&metadata).unwrap())
        .unwrap();
    assert_eq!(
        package_metadata_from_artifact(std::path::Path::new("descriptor.etasmeta"), &bytes)
            .unwrap()
            .1,
        metadata
    );
}

fn key(name: &str, kind: DefinitionKind) -> DefinitionKey {
    DefinitionKey::new(
        PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([9; 32])),
        vec!["library".into()],
        vec![name.into()],
        kind,
    )
    .unwrap()
}

fn fixture() -> DefinitionDescriptorSet {
    let node = key("Node", DefinitionKind::Type);
    let alias = key("Label", DefinitionKind::Type);
    let reply = key("Reply", DefinitionKind::Type);
    let flow = key("visit", DefinitionKind::Flow);
    let effect = key("Console", DefinitionKind::Effect);
    let spec = key("Readable", DefinitionKind::Spec);
    let method = MethodId::new(
        ImplementationKey::new(
            flow.package(),
            vec!["library".into()],
            key("Worker", DefinitionKind::Agent),
            Some(spec.clone()),
            CodeFingerprint::from_bytes([8; 32]),
        )
        .unwrap(),
        "run".into(),
    )
    .unwrap();
    let call = ParameterCallInfo {
        handlers: Some(vec![]),
        callable: CallableId::method(method.clone()),
        signature: TypeInfoRef(10),
        position: 0,
        arguments: None,
    };
    let event = EffectInfo {
        target: EffectTargetInfo::Action {
            effect: effect.clone(),
            name: "write".into(),
        },
        arguments: vec![
            StaticEffectArgInfo::Type(TypeInfoRef(2)),
            StaticEffectArgInfo::Resource(key("scope", DefinitionKind::Value)),
            StaticEffectArgInfo::String("console".into()),
            StaticEffectArgInfo::Integer("42".into()),
            StaticEffectArgInfo::Wildcard,
            StaticEffectArgInfo::Callable(CallableId::method(method.clone())),
        ],
    };
    DefinitionDescriptorSet {
        headers: None,
        graph: DescriptorGraphInput {
            types: vec![
                TypeInfo::Primitive(PrimitiveInfo::Unit),
                TypeInfo::Primitive(PrimitiveInfo::Bool),
                TypeInfo::Primitive(PrimitiveInfo::String),
                TypeInfo::Parameter(GenericParameterRef(0)),
                TypeInfo::Nominal {
                    definition: node.clone(),
                    parameters: vec![GenericParameterRef(0)],
                    representation: Some(TypeInfoRef(5)),
                },
                TypeInfo::Record(vec![
                    FieldInfo {
                        name: "value".into(),
                        ty: TypeInfoRef(3),
                    },
                    FieldInfo {
                        name: "next".into(),
                        ty: TypeInfoRef(6),
                    },
                ]),
                TypeInfo::Builtin {
                    constructor: BuiltinTypeInfo::Option,
                    arguments: vec![TypeInfoRef(7)],
                },
                TypeInfo::Applied {
                    constructor: TypeInfoRef(4),
                    arguments: vec![TypeInfoRef(3)],
                },
                TypeInfo::Alias {
                    definition: alias.clone(),
                    parameters: vec![],
                    target: TypeInfoRef(2),
                },
                TypeInfo::Enum {
                    definition: reply.clone(),
                    parameters: vec![],
                    variants: vec![
                        VariantInfo {
                            name: "Empty".into(),
                            payload: VariantPayloadInfo::None,
                        },
                        VariantInfo {
                            name: "Text".into(),
                            payload: VariantPayloadInfo::Positional(vec![TypeInfoRef(2)]),
                        },
                        VariantInfo {
                            name: "Status".into(),
                            payload: VariantPayloadInfo::Named(vec![FieldInfo {
                                name: "success".into(),
                                ty: TypeInfoRef(1),
                            }]),
                        },
                    ],
                },
                TypeInfo::Function(SignatureInfo {
                    generics: vec![GenericParameterRef(1), GenericParameterRef(2)],
                    parameters: vec![ParameterInfo {
                        name: Some("value".into()),
                        ty: TypeInfoRef(13),
                    }],
                    output: TypeInfoRef(0),
                    declared_output: Some(TypeInfoRef(0)),
                    effects: Some(EffectRowInfoRef(0)),
                }),
                TypeInfo::Handler {
                    handled: EffectRowInfoRef(1),
                    produced: Some(EffectRowInfoRef(0)),
                    result: Some(TypeInfoRef(0)),
                },
                TypeInfo::Tuple(vec![TypeInfoRef(1), TypeInfoRef(2)]),
                TypeInfo::Function(SignatureInfo {
                    generics: vec![],
                    parameters: vec![],
                    output: TypeInfoRef(0),
                    declared_output: None,
                    effects: None,
                }),
            ],
            rows: vec![
                EffectRowInfo {
                    effects: vec![EffectInfo {
                        target: EffectTargetInfo::Effect(effect),
                        arguments: vec![],
                    }],
                    tail: Some(GenericParameterRef(2)),
                },
                EffectRowInfo {
                    effects: vec![event.clone()],
                    tail: None,
                },
            ],
            binders: vec![
                GenericParameterInfo {
                    owner: GenericOwner::Declaration(node.clone()),
                    position: 0,
                    name: "T".into(),
                    kind: GenericParameterKind::Type,
                    bounds: vec![],
                },
                GenericParameterInfo {
                    owner: GenericOwner::Declaration(flow.clone()),
                    position: 0,
                    name: "U".into(),
                    kind: GenericParameterKind::Type,
                    bounds: vec![SpecInfo {
                        definition: spec,
                        arguments: vec![TypeInfoRef(2)],
                    }],
                },
                GenericParameterInfo {
                    owner: GenericOwner::Declaration(flow.clone()),
                    position: 1,
                    name: "E".into(),
                    kind: GenericParameterKind::EffectRow,
                    bounds: vec![],
                },
                GenericParameterInfo {
                    owner: GenericOwner::Method(method),
                    position: 0,
                    name: "V".into(),
                    kind: GenericParameterKind::Type,
                    bounds: vec![],
                },
            ],
            traces: vec![
                TraceInfo::Empty,
                TraceInfo::Event(event),
                TraceInfo::Outcome(CallableOutcomeInfo::Returned {
                    ty: TypeInfoRef(0),
                    observation: ReturnObservationInfo::Unit,
                }),
                TraceInfo::ParameterCall(call.clone()),
                TraceInfo::Seq(vec![TraceInfoRef(1), TraceInfoRef(3)]),
                TraceInfo::Choice(vec![TraceInfoRef(4), TraceInfoRef(0)]),
                TraceInfo::Repeat(TraceInfoRef(5)),
                TraceInfo::UnknownOrder(EffectRowInfoRef(1)),
                TraceInfo::Widened {
                    actions: EffectRowInfoRef(1),
                    parameter_calls: vec![
                        call,
                        ParameterCallInfo {
                            handlers: Some(vec![]),
                            callable: CallableId::declaration(flow.clone()).unwrap(),
                            signature: TypeInfoRef(10),
                            position: 0,
                            arguments: None,
                        },
                    ],
                },
                TraceInfo::Outcome(CallableOutcomeInfo::Raised {
                    error: TypeInfoRef(2),
                }),
                TraceInfo::Outcome(CallableOutcomeInfo::NonReturn),
                TraceInfo::Outcome(CallableOutcomeInfo::Returned {
                    ty: TypeInfoRef(1),
                    observation: ReturnObservationInfo::Bool(true),
                }),
                TraceInfo::Outcome(CallableOutcomeInfo::Returned {
                    ty: TypeInfoRef(9),
                    observation: ReturnObservationInfo::Variant {
                        owner: reply.clone(),
                        name: "Status".into(),
                    },
                }),
                TraceInfo::Outcome(CallableOutcomeInfo::Returned {
                    ty: TypeInfoRef(12),
                    observation: ReturnObservationInfo::Any,
                }),
            ],
        },
        declarations: vec![(node, 4), (alias, 8), (reply, 9), (flow, 10)]
            .into_iter()
            .map(|(definition, ty)| DefinitionTypeBinding {
                definition,
                ty: TypeInfoRef(ty),
            })
            .collect(),
    }
}

#[test]
fn graph_protobuf_preserves_recursive_types_generic_identity_and_trace_placeholders() {
    let input = fixture();
    input.validate(limits()).unwrap();
    let encoded = to_proto(&input).encode_to_vec();
    let output = from_proto(ProtoDescriptorGraph::decode(encoded.as_slice()).unwrap()).unwrap();
    assert_eq!(input, output);
    let original = DescriptorGraph::new(input.graph, limits()).unwrap();
    let decoded = DescriptorGraph::new(output.graph, limits()).unwrap();
    assert_eq!(
        original.type_fingerprint(TypeInfoRef(4)).unwrap(),
        decoded.type_fingerprint(TypeInfoRef(4)).unwrap()
    );
    assert_eq!(
        original.trace_fingerprint(TraceInfoRef(8)).unwrap(),
        decoded.trace_fingerprint(TraceInfoRef(8)).unwrap()
    );
}

#[test]
fn every_primitive_and_compiler_constructor_has_a_stable_wire_mapping() {
    use BuiltinTypeInfo as B;
    use PrimitiveInfo as P;
    let primitives = [
        P::Unit,
        P::Never,
        P::Bool,
        P::I8,
        P::I16,
        P::I32,
        P::I64,
        P::I128,
        P::Isize,
        P::U8,
        P::U16,
        P::U32,
        P::U64,
        P::U128,
        P::Usize,
        P::F32,
        P::F64,
        P::Char,
        P::String,
        P::Bytes,
    ];
    let constructors = [
        B::Array,
        B::List,
        B::Map,
        B::Set,
        B::Range,
        B::Slice,
        B::Option,
        B::Result,
        B::Schema,
        B::Prompt,
        B::PromptPart,
        B::Message,
        B::MemorySelection,
        B::Store,
        B::MemoryRegion,
        B::MemoryRegionHandle,
        B::ExternalToolHandle,
        B::Trusted,
        B::Untrusted,
        B::Secret,
        B::Public,
        B::Sanitized,
    ];
    let set = DefinitionDescriptorSet {
        headers: None,
        graph: DescriptorGraphInput {
            types: primitives
                .into_iter()
                .map(TypeInfo::Primitive)
                .chain(
                    constructors
                        .into_iter()
                        .map(|constructor| TypeInfo::Builtin {
                            constructor,
                            arguments: vec![TypeInfoRef(0); constructor.arity()],
                        }),
                )
                .collect(),
            ..Default::default()
        },
        declarations: vec![],
    };
    assert_eq!(from_proto(to_proto(&set)).unwrap(), set);
}

#[test]
fn callable_selectors_reject_non_callable_or_incomplete_wire_identities() {
    for kind in [
        None,
        Some(schema::callable::Kind::Declaration(identities::encode_key(
            &key("NotCallable", DefinitionKind::Type),
        ))),
    ] {
        let mut wire = to_proto(&fixture());
        wire.rows[1].effects[0].arguments[5].kind =
            Some(schema::static_arg::Kind::Callable(schema::Callable {
                kind,
            }));
        assert!(from_proto(wire).is_err());
    }
    let mut wire = to_proto(&fixture());
    let Some(schema::static_arg::Kind::Callable(callable)) =
        &mut wire.rows[1].effects[0].arguments[5].kind
    else {
        panic!("fixture must contain a callable selector");
    };
    let Some(schema::callable::Kind::Method(method)) = &mut callable.kind else {
        panic!("fixture must contain a method identity");
    };
    method.header.clear();
    assert!(from_proto(wire).is_err());
}

#[test]
fn malformed_graphs_are_rejected_without_replacing_missing_or_unknown_values() {
    for case in 0..10 {
        let mut wire = to_proto(&fixture());
        match case {
            0 => wire.version = 999,
            1 => wire.types[0].kind = Some(schema::ty::Kind::Primitive(999)),
            2 => wire.types[0].kind = None,
            3 => wire.rows[0].tail = Some(0),
            4 => wire.declarations[0].ty = u32::MAX,
            5 => wire.declarations.push(wire.declarations[0].clone()),
            6 => wire.binders[0].kind = 99,
            7 => wire.traces[0].kind = Some(schema::trace::Kind::Repeat(0)),
            8 => wire.binders[3]
                .owner
                .as_mut()
                .unwrap()
                .kind
                .as_mut()
                .map(|kind| {
                    if let schema::callable::Kind::Method(method) = kind {
                        method.header.clear();
                    }
                })
                .unwrap(),
            9 => {
                wire.binders[1].owner = wire.binders[0].owner.clone();
                wire.binders[1].position = 7;
            }
            _ => unreachable!(),
        }
        assert!(from_proto(wire).is_err(), "case {case}");
    }
}

#[test]
fn binary_artifact_decoder_validates_flat_graph_references_before_accepting_metadata() {
    use crate::*;
    let metadata = PackageMetadata {
        version: 1,
        package: PackageIdentity {
            name: "descriptors".into(),
            version: "0.1.0".into(),
            edition: "2026".into(),
        },
        public_metadata: PublicMetadata {
            definition_descriptors: Some(fixture()),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut sections = package_metadata_to_sections(&metadata).unwrap();
    let section = sections
        .iter_mut()
        .find(|section| section.kind == MetadataSectionKind::PackageGraph)
        .unwrap();
    let mut graph =
        super::super::schema::ProtoPackageGraphSection::decode(section.payload.as_slice()).unwrap();
    graph
        .public_metadata
        .as_mut()
        .unwrap()
        .definition_descriptors
        .as_mut()
        .unwrap()
        .rows[0]
        .tail = Some(u32::MAX);
    *section = section_from_message(MetadataSectionKind::PackageGraph, graph).unwrap();
    let header = MetadataArtifactHeader {
        artifact_schema_version: ARTIFACT_SCHEMA_VERSION,
        compiler_version: "test".into(),
        package_id: "descriptors".into(),
        package_version: "0.1.0".into(),
        source_payload_hash: "source".into(),
        manifest_hash: "manifest".into(),
        dependency_lock_hash: "lock".into(),
        created_target: "test".into(),
    };
    let bytes = encode_metadata_artifact(&header, sections).unwrap();
    let error = package_metadata_from_artifact(std::path::Path::new("tampered.etasmeta"), &bytes)
        .unwrap_err();
    assert!(
        error.to_string().contains("references missing Binder"),
        "{error}"
    );
}

#[test]
fn package_graph_roundtrip_and_shared_descriptor_budget_use_the_production_codec() {
    use crate::*;
    let mut metadata = PackageMetadata {
        version: 1,
        package: PackageIdentity {
            name: "descriptors".into(),
            version: "0.1.0".into(),
            edition: "2026".into(),
        },
        ..Default::default()
    };
    metadata.public_metadata.definition_descriptors = Some(fixture());
    let sections = package_metadata_to_sections(&metadata).unwrap();
    let header = MetadataArtifactHeader {
        artifact_schema_version: ARTIFACT_SCHEMA_VERSION,
        compiler_version: "test".into(),
        package_id: "descriptors".into(),
        package_version: "0.1.0".into(),
        source_payload_hash: "source".into(),
        manifest_hash: "manifest".into(),
        dependency_lock_hash: "lock".into(),
        created_target: "test".into(),
    };
    let bytes = encode_metadata_artifact(&header, sections).unwrap();
    assert_eq!(
        package_metadata_from_artifact(std::path::Path::new("package.etasmeta"), &bytes)
            .unwrap()
            .1,
        metadata
    );
    let large = DefinitionDescriptorSet {
        headers: None,
        graph: DescriptorGraphInput {
            types: vec![TypeInfo::Primitive(PrimitiveInfo::Unit); 50_000],
            ..Default::default()
        },
        declarations: vec![],
    };
    metadata.public_metadata.definition_descriptors = Some(large.clone());
    metadata.dependencies.push(ResolvedDependency {
        identity: PackageIdentity {
            name: "dep".into(),
            version: "0.1.0".into(),
            edition: "2026".into(),
        },
        import_root: "dep".into(),
        source: ResolvedDependencySource::Path {
            path: "dep".into(),
            checksum: "checksum".into(),
        },
        dependencies: vec![],
        public_metadata: PublicMetadata {
            definition_descriptors: Some(large),
            ..Default::default()
        },
        effect_metadata: Default::default(),
        tool_bindings: vec![],
    });
    assert!(
        package_metadata_to_sections(&metadata)
            .unwrap_err()
            .to_string()
            .contains("maximum node count")
    );
}
