pub mod container;
pub mod error;
pub mod model;
mod proto;

pub use container::{
    ARTIFACT_SCHEMA_VERSION, COMPRESSION_ZSTD, DecodedMetadataArtifact, EncodedMetadataSection,
    MAGIC, MetadataArtifactHeader, MetadataArtifactInfo, MetadataSectionKind,
    PACKAGE_METADATA_FILE, blake3_hash, decode_metadata_artifact, encode_metadata_artifact,
    file_checksum, optional_file_checksum, package_metadata_artifact_path, section_from_message,
    source_payload_checksum, validate_artifact_schema, write_metadata_artifact_file,
};
pub use error::MetadataArtifactError;
pub use model::*;
pub use proto::{
    package_metadata_from_artifact, package_metadata_from_package_graph_payload,
    package_metadata_to_sections,
};

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
    fn metadata_artifact_round_trips_public_handler_value() {
        let mut metadata = sample_metadata();
        metadata.public_metadata.values.push(NamedSignature {
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
                }),
                produced_effects: Some(EffectRow {
                    effects: vec![EffectRef {
                        path: vec!["Error".to_owned()],
                        args: Vec::new(),
                    }],
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
        let mut sections = package_metadata_to_sections(&metadata);
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
        let mut header = sample_header();
        header.artifact_schema_version = ARTIFACT_SCHEMA_VERSION + 1;

        let error = validate_artifact_schema(&header).unwrap_err();

        assert!(error.to_string().contains("schema version"));
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
        let bytes = encode_sample(&metadata);

        let error =
            package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap_err();

        assert!(
            error.to_string().contains("selector_defaults length"),
            "{error}"
        );
    }

    fn encode_sample(metadata: &PackageMetadata) -> Vec<u8> {
        encode_metadata_artifact(&sample_header(), package_metadata_to_sections(metadata)).unwrap()
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
                    },
                    requested_actions: EffectRow {
                        effects: vec![EffectRef {
                            path: vec!["Console".to_owned(), "stdout_write".to_owned()],
                            args: Vec::new(),
                        }],
                    },
                    handled_requested_actions: EffectRow {
                        effects: vec![EffectRef {
                            path: vec!["Console".to_owned(), "stdout_write".to_owned()],
                            args: Vec::new(),
                        }],
                    },
                    latent_flows: vec![LatentFlowSummary {
                        declared_bound: EffectRow {
                            effects: vec![EffectRef {
                                path: vec!["Network".to_owned()],
                                args: Vec::new(),
                            }],
                        },
                        inferred_effects: EffectRow {
                            effects: vec![EffectRef {
                                path: vec!["Network".to_owned()],
                                args: Vec::new(),
                            }],
                        },
                    }],
                }],
                actions: vec![ActionSignature {
                    path: vec![
                        "demo".to_owned(),
                        "EdkHttp".to_owned(),
                        "request".to_owned(),
                    ],
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

    fn first_payload_offset(bytes: &[u8]) -> usize {
        let table = section_table_start(bytes);
        u64::from_le_bytes(bytes[table + 3..table + 11].try_into().unwrap()) as usize
    }
}
