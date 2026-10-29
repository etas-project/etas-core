mod callable_values;
pub mod container;
pub mod error;
pub mod model;
mod proto;
mod request_templates;
mod type_validation;

pub use callable_values::*;
pub use container::{
    ARTIFACT_SCHEMA_VERSION, COMPRESSION_ZSTD, DecodedMetadataArtifact, EncodedMetadataSection,
    MAGIC, MAX_METADATA_ARTIFACT_SIZE, MAX_METADATA_COMPRESSED_SECTION_SIZE,
    MAX_METADATA_HEADER_SIZE, MAX_METADATA_SECTION_COUNT, MAX_METADATA_TOTAL_UNCOMPRESSED_SIZE,
    MAX_METADATA_UNCOMPRESSED_SECTION_SIZE, MetadataArtifactHeader, MetadataArtifactInfo,
    MetadataSectionKind, PACKAGE_METADATA_FILE, blake3_hash, decode_metadata_artifact,
    encode_metadata_artifact, file_checksum, optional_file_checksum,
    package_metadata_artifact_path, section_from_message, source_payload_checksum,
    validate_artifact_schema, write_metadata_artifact_file,
};
pub use error::MetadataArtifactError;
pub use model::*;

/// Checks graph-local references and resource bounds outside the binary codec.
/// The frontend must also validate root references against its resolved interface.
pub fn validate_callable_value_graph(
    graph: &CallableValueGraph,
) -> Result<usize, MetadataArtifactError> {
    proto::validate_callable_values(graph)
}

pub fn callable_value_graph_fingerprint(
    graph: &CallableValueGraph,
) -> Result<etas_code::CodeFingerprint, MetadataArtifactError> {
    validate_callable_value_graph(graph)?;
    Ok(proto::callable_value_graph_fingerprint(graph))
}
pub use proto::{
    MAX_METADATA_GRAPH_DEPTH, MAX_METADATA_GRAPH_NODES, package_metadata_from_artifact,
    package_metadata_from_package_graph_payload, package_metadata_to_sections,
};

pub fn definition_descriptors_fingerprint(
    set: &etas_code::descriptor::DefinitionDescriptorSet,
) -> etas_code::CodeFingerprint {
    proto::descriptors_fingerprint(set)
}

/// Validates descriptive data and returns its artifact graph budget usage.
pub fn validate_definition_descriptors(
    set: &etas_code::descriptor::DefinitionDescriptorSet,
) -> Result<usize, MetadataArtifactError> {
    proto::validate_descriptors(set)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn metadata_artifact_round_trips_package_metadata() {
        let metadata = sample_metadata();
        let bytes = encode_sample(&metadata);

        let (header, decoded) =
            package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap();

        assert_eq!(header.package_id, "demo");
        assert_eq!(decoded, metadata);
    }

    #[test]
    fn metadata_artifact_round_trips_vendor_definition_origins_without_using_store_paths() {
        let mut metadata = sample_metadata();
        for origin in [
            None,
            Some(etas_code::PackageKey::from_canonical_fingerprint(
                etas_code::CodeFingerprint::of_bytes("etas.test.vendor-origin", b"original-owner"),
            )),
        ] {
            metadata.dependencies[0].source = ResolvedDependencySource::Vendor {
                path: ".etas/vendor/alias".into(),
                checksum: "payload-checksum".into(),
                store: Some(".etas/store/payload".into()),
                definition_origin: origin,
            };
            let bytes = encode_sample(&metadata);
            let (_, decoded) =
                package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap();
            assert_eq!(decoded, metadata);
        }
    }

    #[test]
    fn metadata_artifact_round_trips_conditional_default_effects_and_open_tails() {
        let mut metadata = sample_metadata();
        metadata.public_metadata.flows[0]
            .generic_params
            .push(GenericParam {
                name: "E".into(),
                kind: GenericParamKind::Effect,
                bounds: Vec::new(),
            });
        let summary = &mut metadata.public_metadata.effect_summaries[0];
        let action = EffectRef {
            path: vec![
                "std".into(),
                "effects".into(),
                "actions".into(),
                "Code".into(),
                "check".into(),
            ],
            args: Vec::new(),
        };
        summary.public_effects.tail = Some("E".into());
        summary.requested_actions = EffectRow {
            effects: vec![action.clone()],
            tail: Some("E".into()),
        };
        summary.handled_requested_actions = summary.requested_actions.clone();
        let produced = EffectRow {
            effects: summary.public_effects.effects.clone(),
            tail: None,
        };
        summary.default_handler_effects = Some(DefaultHandlerEffects {
            direct_effects: EffectRow {
                effects: Vec::new(),
                tail: Some("E".into()),
            },
            default_actions: EffectRow {
                effects: vec![action.clone()],
                tail: Some("E".into()),
            },
            handlers: vec![DefaultHandlerContribution { action, produced }],
        });
        let bytes = encode_sample(&metadata);
        let (_, decoded) =
            package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap();
        assert_eq!(decoded, metadata);
    }

    #[test]
    fn metadata_artifact_round_trips_named_type_parameter_scope_and_unused_parameters() {
        let mut metadata = sample_metadata();
        metadata.public_metadata.types.push(NamedSignature {
            path: vec!["demo".into(), "Wrapper".into()],
            generic_params: vec![
                GenericParam {
                    name: "T".into(),
                    kind: GenericParamKind::Type,
                    bounds: vec![SpecBound {
                        spec: vec!["demo".into(), "Allowed".into()],
                        args: vec![],
                    }],
                },
                GenericParam {
                    name: "Unused".into(),
                    ..Default::default()
                },
            ],
            visibility: Visibility::Public,
            ty: Some(Type {
                kind: TypeKind::Nominal,
                path: vec!["demo".into(), "Wrapper".into()],
                children: vec![Type {
                    kind: TypeKind::Var,
                    name: "T".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }),
        });
        let header = sample_header();
        let bytes =
            encode_metadata_artifact(&header, package_metadata_to_sections(&metadata).unwrap())
                .unwrap();
        let (_, decoded) =
            package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap();
        assert_eq!(decoded, metadata);
    }

    #[test]
    fn enum_payload_layouts_round_trip_and_invalid_payload_forms_are_rejected() {
        let mut metadata = sample_metadata();
        let variant = |name: &str, ty| TypeField {
            name: name.into(),
            ty,
        };
        metadata.public_metadata.enums.push(NamedSignature {
            path: vec!["demo".into(), "Response".into()],
            visibility: Visibility::Public,
            ty: Some(Type {
                kind: TypeKind::Enum,
                path: vec!["demo".into(), "Response".into()],
                fields: vec![
                    variant(
                        "Empty",
                        Type {
                            kind: TypeKind::Tuple,
                            ..Default::default()
                        },
                    ),
                    variant(
                        "Success",
                        Type {
                            kind: TypeKind::Tuple,
                            children: vec![Type {
                                kind: TypeKind::Primitive,
                                name: "string".into(),
                                ..Default::default()
                            }],
                            ..Default::default()
                        },
                    ),
                    variant(
                        "Failure",
                        Type {
                            kind: TypeKind::Record,
                            fields: vec![variant(
                                "code",
                                Type {
                                    kind: TypeKind::Primitive,
                                    name: "i32".into(),
                                    ..Default::default()
                                },
                            )],
                            ..Default::default()
                        },
                    ),
                    variant(
                        "NamedEmpty",
                        Type {
                            kind: TypeKind::Record,
                            ..Default::default()
                        },
                    ),
                ],
                ..Default::default()
            }),
            ..Default::default()
        });
        let bytes = encode_sample(&metadata);
        let (_, decoded) =
            package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap();
        assert_eq!(decoded, metadata);
        for invalid in [
            Type {
                kind: TypeKind::Primitive,
                name: "i32".into(),
                ..Default::default()
            },
            Type {
                kind: TypeKind::Record,
                fields: vec![
                    variant(
                        "x",
                        Type {
                            kind: TypeKind::Tuple,
                            ..Default::default()
                        },
                    ),
                    variant(
                        "x",
                        Type {
                            kind: TypeKind::Tuple,
                            ..Default::default()
                        },
                    ),
                ],
                ..Default::default()
            },
        ] {
            let mut broken = metadata.clone();
            broken
                .public_metadata
                .enums
                .last_mut()
                .unwrap()
                .ty
                .as_mut()
                .unwrap()
                .fields[0]
                .ty = invalid;
            assert!(package_metadata_to_sections(&broken).is_err());
        }
    }

    #[test]
    fn metadata_artifact_round_trips_public_handler_value() {
        let mut metadata = sample_metadata();
        metadata.public_metadata.values.push(NamedSignature {
            generic_params: Vec::new(),
            path: vec!["demo".to_owned(), "DefaultHandler".to_owned()],
            visibility: Visibility::Public,
            ty: Some(Type {
                kind: TypeKind::Handler,
                effects: Some(EffectRow {
                    effects: vec![EffectRef {
                        path: vec![
                            "demo".to_owned(),
                            "EdkHttp".to_owned(),
                            "request".to_owned(),
                        ],
                        args: Vec::new(),
                    }],
                    tail: None,
                }),
                produced_effects: Some(EffectRow {
                    effects: vec![EffectRef {
                        path: vec!["Error".to_owned()],
                        args: Vec::new(),
                    }],
                    tail: None,
                }),
                children: vec![Type {
                    kind: TypeKind::Primitive,
                    name: "unit".to_owned(),
                    ..Default::default()
                }],
                ..Default::default()
            }),
        });

        let bytes = encode_sample(&metadata);
        let (_, decoded) =
            package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap();

        assert_eq!(
            decoded.public_metadata.values,
            metadata.public_metadata.values
        );
    }

    #[test]
    fn metadata_artifact_round_trips_generic_action_signature() {
        let mut metadata = sample_metadata();
        let action = &mut metadata.public_metadata.actions[0];
        action.generic_params = vec![ActionGenericParam {
            name: "S".to_owned(),
            kind: GenericParamKind::Type,
            bounds: vec![SpecBound {
                spec: vec![
                    "std".to_owned(),
                    "stream".to_owned(),
                    "ByteStream".to_owned(),
                ],
                args: Vec::new(),
            }],
        }];
        action.params = vec![Type {
            kind: TypeKind::Var,
            name: "S".to_owned(),
            ..Default::default()
        }];
        action.effect_args = vec![ActionArgKind::Type];
        action.selector_param_names = vec!["S".to_owned()];
        action.selector_defaults = vec![None];

        let bytes = encode_sample(&metadata);
        let (_, decoded) =
            package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap();

        assert_eq!(
            decoded.public_metadata.actions,
            metadata.public_metadata.actions
        );
    }

    #[test]
    fn metadata_artifact_round_trips_generic_callable_signature() {
        let mut metadata = sample_metadata();
        let flow = &mut metadata.public_metadata.flows[0];
        flow.generic_params = vec![
            GenericParam {
                name: "R".to_owned(),
                kind: GenericParamKind::Type,
                bounds: vec![SpecBound {
                    spec: vec!["std".to_owned(), "fs".to_owned(), "Region".to_owned()],
                    args: Vec::new(),
                }],
            },
            GenericParam {
                name: "E".to_owned(),
                kind: GenericParamKind::Effect,
                bounds: Vec::new(),
            },
        ];
        flow.input = vec![Type {
            kind: TypeKind::Var,
            name: "R".to_owned(),
            ..Default::default()
        }];
        flow.effects = Some(EffectRow {
            effects: Vec::new(),
            tail: Some("E".to_owned()),
        });

        let bytes = encode_sample(&metadata);
        let (_, decoded) =
            package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap();

        assert_eq!(
            decoded.public_metadata.flows,
            metadata.public_metadata.flows
        );
    }

    #[test]
    fn metadata_artifact_rejects_unknown_section_kind() {
        let mut bytes = encode_sample(&sample_metadata());
        let table = section_table_start(&bytes);
        bytes[table] = 99;
        bytes[table + 1] = 0;

        let error = decode_metadata_artifact(Path::new("package.etasmeta"), &bytes).unwrap_err();

        assert!(error.to_string().contains("unknown section kind"));
    }

    #[test]
    fn metadata_artifact_rejects_duplicate_sections() {
        let metadata = sample_metadata();
        let mut sections = package_metadata_to_sections(&metadata).unwrap();
        sections.push(sections[0].clone());
        let bytes = encode_metadata_artifact(&sample_header(), sections).unwrap();

        let error = decode_metadata_artifact(Path::new("package.etasmeta"), &bytes).unwrap_err();

        assert!(error.to_string().contains("duplicate"));
    }

    #[test]
    fn metadata_artifact_rejects_trailing_bytes() {
        let mut bytes = encode_sample(&sample_metadata());
        bytes.push(0);

        let error = decode_metadata_artifact(Path::new("package.etasmeta"), &bytes).unwrap_err();

        assert!(error.to_string().contains("trailing byte"));
    }

    #[test]
    fn metadata_artifact_rejects_hash_mismatch() {
        let mut bytes = encode_sample(&sample_metadata());
        let table = section_table_start(&bytes);
        let hash = table + 2 + 1 + 8 + 8 + 8;
        bytes[hash] ^= 0xff;

        let error = decode_metadata_artifact(Path::new("package.etasmeta"), &bytes).unwrap_err();

        assert!(error.to_string().contains("hash mismatch"));
    }

    #[test]
    fn metadata_artifact_rejects_zstd_corruption() {
        let mut bytes = encode_sample(&sample_metadata());
        let payload = first_payload_offset(&bytes);
        bytes[payload] ^= 0xff;

        let error = decode_metadata_artifact(Path::new("package.etasmeta"), &bytes).unwrap_err();

        let message = error.to_string();
        assert!(message.contains("decompression failed") || message.contains("hash mismatch"));
    }

    #[test]
    fn metadata_artifact_rejects_schema_version_mismatch() {
        for unsupported in [ARTIFACT_SCHEMA_VERSION - 1, ARTIFACT_SCHEMA_VERSION + 1] {
            let mut header = sample_header();
            header.artifact_schema_version = unsupported;

            let error = validate_artifact_schema(&header).unwrap_err();

            assert!(error.to_string().contains("schema version"));
        }
    }

    #[test]
    fn metadata_artifact_rejects_missing_package_graph() {
        let section = EncodedMetadataSection {
            kind: MetadataSectionKind::Exports,
            payload: Vec::new(),
        };
        let bytes = encode_metadata_artifact(&sample_header(), vec![section]).unwrap();

        let error =
            package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap_err();

        assert!(error.to_string().contains("missing package_graph"));
    }

    #[test]
    fn metadata_artifact_rejects_malformed_action_selector_metadata() {
        let mut metadata = sample_metadata();
        metadata.public_metadata.actions[0].selector_defaults.pop();
        let error = package_metadata_to_sections(&metadata).unwrap_err();

        assert!(
            error.to_string().contains("selector_defaults length"),
            "{error}"
        );
    }

    #[test]
    fn metadata_artifact_round_trips_widened_parameter_calls() {
        let mut metadata = sample_metadata();
        metadata.public_metadata.effect_summaries[0].action_trace = ActionTrace::Widened {
            actions: vec![EffectRef {
                path: vec!["Console".to_owned(), "stdout_write".to_owned()],
                args: Vec::new(),
            }],
            parameter_calls: vec!["f".to_owned()],
        };

        let bytes = encode_sample(&metadata);
        let (_, decoded) =
            package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap();

        assert_eq!(decoded, metadata);
    }

    #[test]
    fn metadata_artifact_rejects_excessive_section_count_before_allocation() {
        let mut bytes = encode_sample(&sample_metadata());
        let count = section_count_offset(&bytes);
        bytes[count..count + 4]
            .copy_from_slice(&((MAX_METADATA_SECTION_COUNT + 1) as u32).to_le_bytes());

        let error = decode_metadata_artifact(Path::new("package.etasmeta"), &bytes).unwrap_err();

        assert!(error.to_string().contains("too many sections"), "{error}");
    }

    #[test]
    fn metadata_artifact_rejects_excessive_declared_uncompressed_size() {
        let mut bytes = encode_sample(&sample_metadata());
        let table = section_table_start(&bytes);
        let uncompressed_len = table + 2 + 1 + 8 + 8;
        bytes[uncompressed_len..uncompressed_len + 8]
            .copy_from_slice(&((MAX_METADATA_UNCOMPRESSED_SECTION_SIZE + 1) as u64).to_le_bytes());

        let error = decode_metadata_artifact(Path::new("package.etasmeta"), &bytes).unwrap_err();

        assert!(error.to_string().contains("size limit"), "{error}");
    }

    #[test]
    fn metadata_artifact_rejects_excessively_deep_action_trace() {
        let mut metadata = sample_metadata();
        let mut trace = ActionTrace::ParameterCall {
            parameter: "f".to_owned(),
        };
        for _ in 0..70 {
            trace = ActionTrace::Repeat(Box::new(trace));
        }
        metadata.public_metadata.effect_summaries[0].action_trace = trace;
        let error = package_metadata_to_sections(&metadata).unwrap_err();

        assert!(error.to_string().contains("maximum depth"), "{error}");
    }

    #[test]
    fn metadata_artifact_rejects_excessively_deep_type_graph() {
        let mut metadata = sample_metadata();
        let mut ty = Type {
            kind: TypeKind::Primitive,
            name: "i32".to_owned(),
            ..Default::default()
        };
        for _ in 0..70 {
            ty = Type {
                kind: TypeKind::Array,
                children: vec![ty],
                ..Default::default()
            };
        }
        metadata.public_metadata.flows[0].input = vec![ty];
        let error = package_metadata_to_sections(&metadata).unwrap_err();

        assert!(error.to_string().contains("maximum depth"), "{error}");
    }

    fn encode_sample(metadata: &PackageMetadata) -> Vec<u8> {
        encode_metadata_artifact(
            &sample_header(),
            package_metadata_to_sections(metadata).unwrap(),
        )
        .unwrap()
    }

    fn sample_header() -> MetadataArtifactHeader {
        MetadataArtifactHeader {
            artifact_schema_version: ARTIFACT_SCHEMA_VERSION,
            compiler_version: "test-compiler".to_owned(),
            package_id: "demo".to_owned(),
            package_version: "0.1.0".to_owned(),
            source_payload_hash: "blake3:source".to_owned(),
            manifest_hash: "blake3:manifest".to_owned(),
            dependency_lock_hash: "blake3:lock".to_owned(),
            created_target: "test".to_owned(),
        }
    }

    fn sample_metadata() -> PackageMetadata {
        PackageMetadata {
            version: 1,
            package: PackageIdentity {
                name: "demo".to_owned(),
                version: "0.1.0".to_owned(),
                edition: "2026".to_owned(),
            },
            dependencies: vec![ResolvedDependency {
                identity: PackageIdentity {
                    name: "dep".to_owned(),
                    version: "1.2.3".to_owned(),
                    edition: "2026".to_owned(),
                },
                import_root: "dep".to_owned(),
                source: ResolvedDependencySource::GitHubRelease {
                    repo: "owner/dep".to_owned(),
                    release: "v1.2.3".to_owned(),
                    asset: "dep.etaspkg".to_owned(),
                    asset_checksum: "blake3:asset".to_owned(),
                    payload_checksum: "blake3:payload".to_owned(),
                    store: Some(".etas/store/packages/blake3/payload".to_owned()),
                },
                dependencies: Vec::new(),
                public_metadata: PublicMetadata {
                    flows: vec![CallableSignature {
                        path: vec!["dep".to_owned(), "run".to_owned()],
                        generic_params: Vec::new(),
                        param_names: vec!["input".to_owned()],
                        input: vec![Type {
                            kind: TypeKind::Primitive,
                            name: "string".to_owned(),
                            ..Default::default()
                        }],
                        output: Some(Type {
                            kind: TypeKind::Primitive,
                            name: "i32".to_owned(),
                            ..Default::default()
                        }),
                        effects: Some(EffectRow {
                            effects: vec![EffectRef {
                                path: vec!["Console".to_owned()],
                                args: Vec::new(),
                            }],
                            tail: None,
                        }),
                        visibility: Visibility::Public,
                    }],
                    ..Default::default()
                },
                effect_metadata: EffectMetadata {
                    tags: vec![EffectTag {
                        path: vec!["Console".to_owned()],
                        runtime_requirement: Some("host.console".to_owned()),
                    }],
                    extensions: Vec::new(),
                },
                tool_bindings: vec![ToolBinding {
                    tool: "dep.tool".to_owned(),
                    kind: "provider".to_owned(),
                    provider: "fixture".to_owned(),
                    effect_row: vec!["Console".to_owned()],
                    action_row: vec!["Console.stdout_write".to_owned()],
                }],
            }],
            external_modules: vec![ExternalModule {
                package: None,
                id: 1,
                path: vec!["demo".to_owned()],
                exports: vec![ExternalExport {
                    id: 1,
                    name: "main".to_owned(),
                    visibility: Visibility::Public,
                }],
            }],
            public_metadata: PublicMetadata {
                types: vec![
                    NamedSignature {
                        generic_params: Vec::new(),
                        path: vec!["demo".to_owned(), "HttpRequest".to_owned()],
                        visibility: Visibility::Public,
                        ty: Some(Type {
                            kind: TypeKind::Record,
                            fields: vec![TypeField {
                                name: "url".to_owned(),
                                ty: Type {
                                    kind: TypeKind::Primitive,
                                    name: "string".to_owned(),
                                    ..Default::default()
                                },
                            }],
                            ..Default::default()
                        }),
                    },
                    NamedSignature {
                        generic_params: Vec::new(),
                        path: vec!["demo".to_owned(), "Path".to_owned()],
                        visibility: Visibility::Public,
                        ty: Some(Type {
                            kind: TypeKind::Alias,
                            path: vec!["demo".to_owned(), "Path".to_owned()],
                            children: vec![Type {
                                kind: TypeKind::Primitive,
                                name: "string".to_owned(),
                                ..Default::default()
                            }],
                            ..Default::default()
                        }),
                    },
                    NamedSignature {
                        generic_params: Vec::new(),
                        path: vec!["demo".to_owned(), "UserId".to_owned()],
                        visibility: Visibility::Public,
                        ty: Some(Type {
                            kind: TypeKind::Nominal,
                            path: vec!["demo".to_owned(), "UserId".to_owned()],
                            children: vec![Type {
                                kind: TypeKind::Primitive,
                                name: "string".to_owned(),
                                ..Default::default()
                            }],
                            ..Default::default()
                        }),
                    },
                ],
                flows: vec![CallableSignature {
                    path: vec!["demo".to_owned(), "main".to_owned()],
                    input: vec![
                        Type {
                            kind: TypeKind::Record,
                            fields: vec![
                                TypeField {
                                    name: "name".to_owned(),
                                    ty: Type {
                                        kind: TypeKind::Primitive,
                                        name: "string".to_owned(),
                                        ..Default::default()
                                    },
                                },
                                TypeField {
                                    name: "count".to_owned(),
                                    ty: Type {
                                        kind: TypeKind::Primitive,
                                        name: "i32".to_owned(),
                                        ..Default::default()
                                    },
                                },
                            ],
                            ..Default::default()
                        },
                        Type {
                            kind: TypeKind::Function,
                            children: vec![
                                Type {
                                    kind: TypeKind::Primitive,
                                    name: "string".to_owned(),
                                    ..Default::default()
                                },
                                Type {
                                    kind: TypeKind::Primitive,
                                    name: "unit".to_owned(),
                                    ..Default::default()
                                },
                            ],
                            effects: Some(EffectRow {
                                effects: vec![EffectRef {
                                    path: vec!["Network".to_owned()],
                                    args: Vec::new(),
                                }],
                                tail: None,
                            }),
                            ..Default::default()
                        },
                    ],
                    output: Some(Type {
                        kind: TypeKind::Primitive,
                        name: "i32".to_owned(),
                        ..Default::default()
                    }),
                    visibility: Visibility::Public,
                    ..Default::default()
                }],
                effect_summaries: vec![EffectSummary {
                    behavior: None,
                    handler_requests: None,
                    returned_callables: None,
                    default_handler_effects: None,
                    item: vec!["demo".to_owned(), "main".to_owned()],
                    public_effects: EffectRow {
                        effects: vec![EffectRef {
                            path: vec!["Error".to_owned()],
                            args: vec![EffectArg {
                                kind: EffectArgKind::Type,
                                ty: Some(Type {
                                    kind: TypeKind::Named,
                                    name: "IOError".to_owned(),
                                    path: vec!["IOError".to_owned()],
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }],
                        }],
                        tail: None,
                    },
                    requested_actions: EffectRow {
                        effects: vec![EffectRef {
                            path: vec!["Console".to_owned(), "stdout_write".to_owned()],
                            args: Vec::new(),
                        }],
                        tail: None,
                    },
                    handled_requested_actions: EffectRow {
                        effects: vec![EffectRef {
                            path: vec!["Console".to_owned(), "stdout_write".to_owned()],
                            args: Vec::new(),
                        }],
                        tail: None,
                    },
                    latent_flows: vec![LatentFlowSummary {
                        declared_bound: EffectRow {
                            effects: vec![EffectRef {
                                path: vec!["Network".to_owned()],
                                args: Vec::new(),
                            }],
                            tail: None,
                        },
                        inferred_effects: EffectRow {
                            effects: vec![EffectRef {
                                path: vec!["Network".to_owned()],
                                args: Vec::new(),
                            }],
                            tail: None,
                        },
                    }],
                    action_trace: ActionTrace::Seq(vec![
                        ActionTrace::Event(ActionTraceEvent {
                            action: EffectRef {
                                path: vec!["Console".to_owned(), "stdout_write".to_owned()],
                                args: Vec::new(),
                            },
                            source: ActionTraceEventSource::Perform,
                        }),
                        ActionTrace::ParameterCall {
                            parameter: "f".to_owned(),
                        },
                    ]),
                }],
                actions: vec![ActionSignature {
                    path: vec![
                        "demo".to_owned(),
                        "EdkHttp".to_owned(),
                        "request".to_owned(),
                    ],
                    generic_params: Vec::new(),
                    params: vec![
                        Type {
                            kind: TypeKind::Primitive,
                            name: "string".to_owned(),
                            ..Default::default()
                        },
                        Type {
                            kind: TypeKind::Primitive,
                            name: "string".to_owned(),
                            ..Default::default()
                        },
                        Type {
                            kind: TypeKind::Named,
                            path: vec!["demo".to_owned(), "HttpRequest".to_owned()],
                            ..Default::default()
                        },
                    ],
                    effect_args: vec![ActionArgKind::StringPattern, ActionArgKind::StringPattern],
                    selector_param_names: vec!["method".to_owned(), "host".to_owned()],
                    selector_defaults: vec![
                        Some(EffectArg {
                            kind: EffectArgKind::String,
                            value: "GET".to_owned(),
                            ..Default::default()
                        }),
                        Some(EffectArg {
                            kind: EffectArgKind::Wildcard,
                            ..Default::default()
                        }),
                    ],
                    output: Some(Type {
                        kind: TypeKind::Named,
                        path: vec!["demo".to_owned(), "HttpResponse".to_owned()],
                        ..Default::default()
                    }),
                    returns_never: false,
                    visibility: Visibility::Public,
                }],
                annotations: vec![AnnotationMetadata {
                    item: vec!["demo".to_owned(), "main".to_owned()],
                    path: vec!["limits".to_owned()],
                    args: vec![AnnotationArgMetadata {
                        name: String::new(),
                        value: AnnotationValueMetadata {
                            kind: AnnotationValueKind::Array,
                            elements: vec![AnnotationValueMetadata {
                                kind: AnnotationValueKind::Limit,
                                value: "Tokens".to_owned(),
                                elements: vec![AnnotationValueMetadata {
                                    kind: AnnotationValueKind::Int,
                                    value: "128".to_owned(),
                                    ..Default::default()
                                }],
                                ..Default::default()
                            }],
                            ..Default::default()
                        },
                    }],
                }],
                ..Default::default()
            },
            effect_metadata: EffectMetadata {
                tags: vec![EffectTag {
                    path: vec!["Console".to_owned()],
                    runtime_requirement: Some("host.console".to_owned()),
                }],
                extensions: Vec::new(),
            },
            tool_bindings: Vec::new(),
            bins: vec![BinTarget {
                name: "demo".to_owned(),
                module: "demo".to_owned(),
                flow: "main".to_owned(),
            }],
        }
    }

    fn section_table_start(bytes: &[u8]) -> usize {
        let header_len =
            u32::from_le_bytes(bytes[MAGIC.len()..MAGIC.len() + 4].try_into().unwrap()) as usize;
        MAGIC.len() + 4 + header_len + 4
    }

    fn section_count_offset(bytes: &[u8]) -> usize {
        let header_len =
            u32::from_le_bytes(bytes[MAGIC.len()..MAGIC.len() + 4].try_into().unwrap()) as usize;
        MAGIC.len() + 4 + header_len
    }

    fn first_payload_offset(bytes: &[u8]) -> usize {
        let table = section_table_start(bytes);
        u64::from_le_bytes(bytes[table + 3..table + 11].try_into().unwrap()) as usize
    }
}
